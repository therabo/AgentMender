//! Read-only resolution state for one originating Codex conversation.
use crate::{ApplicationError, QueryControl, TargetSelector, session_search::SessionSearchService};
pub use agentmender_domain::search_intent::Outcome;
use agentmender_domain::{
    search_intent::{
        CoverageGoal, LexicalExpansion, ResultPolicy, SearchActivity, SearchIntent,
        ValidatedSearchIntent,
    },
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};

const RUN_LIMIT: usize = 4;
const WORK_TIME: Duration = Duration::from_secs(480);
const RUN_RETENTION: Duration = Duration::from_secs(600);
const SCAN_TIME: Duration = Duration::from_secs(5);
const SCAN_BATCH: usize = 12;
const SCAN_LIMIT: usize = 256;
const CANDIDATE_LIMIT: usize = 96;
const CANDIDATE_BATCH: usize = 5;
const CONTEXT_LIMIT: usize = 24;
const CONTEXT_BYTES: usize = 128 * 1024;
const ASSESSMENT_LIMIT: usize = 48;
const RESULT_BATCH: usize = 5;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnswerCardinality {
    Single,
    Multiple,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnswerFocus {
    #[default]
    Conversation,
    Decision,
    Event,
    Chronology,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub question: String,
    pub cardinality: AnswerCardinality,
    #[serde(default)]
    pub focus: AnswerFocus,
    #[serde(default)]
    pub required_outcomes: Vec<Outcome>,
    #[serde(default)]
    pub hard_filter: Option<QueryExpr>,
    pub literal: QueryExpr,
    #[serde(default)]
    pub expansion: Option<QueryExpr>,
}

impl Intent {
    fn parameters(&self) -> Result<SearchParameters, ApplicationError> {
        fn hint_nodes(query: &QueryExpr) -> bool {
            match query {
                QueryExpr::Text { alternatives, .. } => alternatives.is_empty(),
                QueryExpr::And { args } | QueryExpr::Or { args } => args.iter().all(hint_nodes),
                _ => false,
            }
        }

        let bounded_hint = |query: &QueryExpr| {
            query.valid()
                && query.node_count() <= 18
                && hint_nodes(query)
                && query.text_terms().iter().map(String::len).sum::<usize>() <= 384
        };
        if !bounded_text(&self.question, 1024)
            || self.required_outcomes.len() > 4
            || !bounded_hint(&self.literal)
            || self
                .expansion
                .as_ref()
                .is_some_and(|q| !bounded_hint(q) || q == &self.literal)
        {
            return Err(ApplicationError::InvalidSearch);
        }
        self.compile_parameters()
    }

    fn compile_parameters(&self) -> Result<SearchParameters, ApplicationError> {
        let lexical = QueryExpr::Or {
            args: std::iter::once(self.literal.clone())
                .chain(self.expansion.clone())
                .collect(),
        };
        let parameters = SearchParameters {
            structured: Some(StructuredQuery {
                // Positions 0 and 1 are the outer AND and the lexical OR. Keep
                // both lexical observations alongside the original hard filter.
                filter: QueryExpr::And {
                    args: std::iter::once(lexical)
                        .chain(self.hard_filter.clone())
                        .collect(),
                },
                mode: SearchMode::Exhaustive,
                sort: SearchSort::DiscoveryPriority,
                coverage: CoverageRequirement::BestEffort,
            }),
            query: String::new(),
            include_archived: true,
            include_content: true,
            project_label: None,
            purpose: SearchPurpose::Discovery,
            page_size: 12,
            excerpts: true,
        };
        if !parameters.valid() {
            return Err(ApplicationError::InvalidSearch);
        }
        Ok(parameters)
    }

    fn match_basis(&self, hit: &SessionSearchHit) -> MatchBasis {
        let observed = |query: &QueryExpr, at| {
            query.evaluate_observed_branch(
                &hit.metadata,
                &hit.query_observation,
                hit.content_complete,
                at,
            )
        };
        MatchBasis {
            literal: observed(&self.literal, 2),
            expansion: self
                .expansion
                .as_ref()
                .map(|query| observed(query, 2 + self.literal.node_count())),
        }
    }
}

fn bounded_text(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit && !text.chars().any(char::is_control)
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    VerifiedRelevant,
    Related,
    Contradiction,
    Insufficient,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub native_session_id: String,
    pub disposition: Disposition,
    pub evidence: Vec<usize>,
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<SearchActivity>,
    pub explanation: String,
    #[serde(default)]
    pub missing_evidence: Option<String>,
    #[serde(default)]
    pub plausible_alternative: bool,
}

pub enum ResolutionAction {
    Start(Box<Intent>),
    StartV1(Box<ValidatedSearchIntent>),
    Next {
        run: String,
    },
    Expand {
        run: String,
        expansion: Box<QueryExpr>,
    },
    ExpandTerms {
        run: String,
        expansion: LexicalExpansion,
    },
    Context {
        run: String,
        anchor: usize,
        window: Option<ContextOptions>,
    },
    Assess {
        run: String,
        assessments: Vec<Assessment>,
    },
    Finish {
        run: String,
        offset: usize,
        revision: Option<usize>,
        luna_unavailable: bool,
        preferred_session_id: Option<String>,
    },
}

impl ResolutionAction {
    pub fn is_start(&self) -> bool {
        matches!(self, Self::Start(_) | Self::StartV1(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResolutionError {
    #[error("The run is stale or belongs to another originating conversation. Start a fresh run.")]
    StaleSearchRun,
    #[error(
        "Expansion requires a finished first scan and distinct new terms. Only one expansion is allowed."
    )]
    InvalidQueryExpansion,
    #[error(
        "Assess presented candidates with their own evidence handles and the requested activity and outcome."
    )]
    InvalidAssessment,
    #[error("The preferred session must be a verified match in a single_best run.")]
    InvalidPreference,
    #[error(
        "The run's context, assessment or duration budget is exhausted. Finish with the available evidence."
    )]
    BudgetExceeded,
}

impl ResolutionError {
    pub fn code(self) -> &'static str {
        match self {
            Self::StaleSearchRun => "stale_search_run",
            Self::InvalidQueryExpansion => "invalid_query_expansion",
            Self::InvalidAssessment => "invalid_assessment",
            Self::InvalidPreference => "invalid_preference",
            Self::BudgetExceeded => "budget_exceeded",
        }
    }
}

#[derive(Clone, Serialize)]
pub struct MatchBasis {
    literal: Truth,
    expansion: Option<Truth>,
}

pub struct Citation {
    pub handle: usize,
    pub reference: ContextReference,
    pub evidence: ConversationEvidence,
}

pub struct ResolutionCandidate {
    pub metadata: SessionMetadata,
    pub match_basis: MatchBasis,
    pub source_coverage: Option<SourceCoverage>,
    pub evidence: Vec<Citation>,
}

pub struct ResolvedCandidate {
    pub metadata: SessionMetadata,
    pub assessment: Assessment,
    pub citations: Vec<ContextReference>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionOutcome {
    SingleVerified,
    MultipleVerified,
    Ambiguous,
    NoVerifiedMatch,
    IncompleteSearch,
}

#[derive(Clone, Default, Serialize)]
pub struct ResolutionProgress {
    pub scan_passes: usize,
    pub provider_pages: usize,
    pub retrieval_ms: u64,
    pub records_inspected: u64,
    pub bytes_read: usize,
    pub compressed_bytes: u64,
    pub decoded_bytes: u64,
    pub replay_bytes: u64,
    pub sources_inspected: usize,
    pub physically_traversed: usize,
    pub unknown: usize,
    pub partial_sources: usize,
    pub unavailable_sources: usize,
    pub suppressed_records: usize,
    pub lexical_candidates: usize,
    pub literal_candidates: usize,
    pub expansion_only_candidates: usize,
    pub candidates_presented: usize,
    pub assessments: usize,
    pub insufficient: usize,
    pub context_requests: usize,
    pub context_bytes: usize,
    pub invalid_assessments: usize,
    pub freshness_failures: usize,
    pub scan_complete: bool,
    pub more_candidates: bool,
    pub stop_reason: Option<&'static str>,
}

pub enum ResolutionData {
    Candidates {
        candidates: Vec<ResolutionCandidate>,
        query: Option<Box<QueryExpr>>,
    },
    Context {
        messages: Vec<Citation>,
        source_coverage: SourceCoverage,
        work: ReadWork,
        next: Option<usize>,
        previous: Option<usize>,
        partial: bool,
        at_start: bool,
        at_end: bool,
    },
    Assessed,
    Finished {
        outcome: ResolutionOutcome,
        matches: Vec<ResolvedCandidate>,
        alternatives: Vec<ResolvedCandidate>,
        next_offset: Option<usize>,
        revision: usize,
    },
}

pub struct ResolutionResponse {
    pub run: String,
    pub focus: AnswerFocus,
    pub coverage_goal: Option<CoverageGoal>,
    pub progress: ResolutionProgress,
    pub data: ResolutionData,
}

#[derive(Clone)]
struct Anchor {
    candidate: usize,
    reference: ContextReference,
    /// Continuation handles carry the exact navigation options from the reader.
    continuation: Option<ContextOptions>,
    citable: bool,
}

struct Candidate {
    hit: SessionSearchHit,
    basis: MatchBasis,
    handles: Vec<usize>,
    assessment: Option<Assessment>,
    scan_pass: usize,
}

struct Run {
    binding: String,
    instance: String,
    started: Instant,
    intent: Intent,
    search_intent: Option<Box<SearchIntent>>,
    parameters: SearchParameters,
    original_filter: Option<QueryExpr>,
    next: Option<SearchContinuation>,
    progress: ResolutionProgress,
    candidates: Vec<Candidate>,
    anchors: Vec<Anchor>,
    requested_windows: Vec<(usize, ContextOptions)>,
    assessment_batches: usize,
    revision: usize,
    publication: Option<Publication>,
}

struct Publication {
    luna_unavailable: bool,
    preferred_session_id: Option<String>,
    outcome: ResolutionOutcome,
    choices: Vec<usize>,
}

#[derive(Default)]
pub struct ResolutionStore {
    runs: BTreeMap<String, Run>,
}

impl ResolutionStore {
    pub fn execute(
        &mut self,
        service: &SessionSearchService<'_>,
        action: ResolutionAction,
        new_id: Option<String>,
        control: &QueryControl,
    ) -> Result<ResolutionResponse, ApplicationError> {
        control.check()?;
        let binding = service.binding(SearchPurpose::Discovery)?;
        self.runs
            .retain(|_, run| run.started.elapsed() < RUN_RETENTION);
        let (id, action) = match action {
            ResolutionAction::Start(intent) => {
                let parameters = intent.parameters()?;
                return self.start(
                    service,
                    control,
                    new_id,
                    Run::new(binding, service.instance, *intent, parameters, None),
                );
            }
            ResolutionAction::StartV1(plan) => {
                let (spec, hard_filter, literal) = plan.into_parts();
                let intent = Intent {
                    question: String::new(),
                    cardinality: match spec.result_policy {
                        ResultPolicy::AllRelevant => AnswerCardinality::Multiple,
                        ResultPolicy::SingleBest => AnswerCardinality::Single,
                    },
                    focus: if spec.activity == SearchActivity::Decision {
                        AnswerFocus::Decision
                    } else {
                        AnswerFocus::Conversation
                    },
                    required_outcomes: spec
                        .required_outcome
                        .into_iter()
                        .chain(spec.activity.evidence_outcome())
                        .collect(),
                    hard_filter,
                    literal,
                    expansion: None,
                };
                let parameters = intent.compile_parameters()?;
                return self.start(
                    service,
                    control,
                    new_id,
                    Run::new(
                        binding,
                        service.instance,
                        intent,
                        parameters,
                        Some(Box::new(spec)),
                    ),
                );
            }
            action => {
                let id = match &action {
                    ResolutionAction::Next { run }
                    | ResolutionAction::Expand { run, .. }
                    | ResolutionAction::ExpandTerms { run, .. }
                    | ResolutionAction::Context { run, .. }
                    | ResolutionAction::Assess { run, .. }
                    | ResolutionAction::Finish { run, .. } => run.clone(),
                    _ => unreachable!("start returned"),
                };
                (id, action)
            }
        };
        let run = self
            .runs
            .get_mut(&id)
            .ok_or(ResolutionError::StaleSearchRun)?;
        if run.binding != binding || run.instance != service.instance {
            return Err(run.contract_error(ResolutionError::StaleSearchRun));
        }
        if run.started.elapsed() >= WORK_TIME && run.progress.stop_reason.is_none() {
            run.progress.stop_reason = Some("resolution_duration_limit");
            run.publication = None;
            run.revision += 1;
        }
        let data = match action {
            ResolutionAction::Start(_) | ResolutionAction::StartV1(_) => {
                unreachable!("start returned")
            }
            ResolutionAction::Next { .. } => run.scan(service, control, false)?,
            ResolutionAction::Expand { expansion, .. } => {
                if run.search_intent.is_some() {
                    return Err(ResolutionError::InvalidQueryExpansion.into());
                }
                run.expand(service, control, *expansion)?
            }
            ResolutionAction::ExpandTerms { expansion, .. } => {
                if run.search_intent.is_none() {
                    return Err(ResolutionError::InvalidQueryExpansion.into());
                }
                let expansion = expansion.into_query();
                let original = run.intent.literal.text_terms();
                if expansion
                    .text_terms()
                    .iter()
                    .any(|term| original.contains(term))
                {
                    return Err(ResolutionError::InvalidQueryExpansion.into());
                }
                run.expand(service, control, expansion)?
            }
            ResolutionAction::Context { anchor, window, .. } => {
                run.context(service, control, anchor, window)?
            }
            ResolutionAction::Assess { assessments, .. } => run.assess(assessments)?,
            ResolutionAction::Finish {
                offset,
                revision,
                luna_unavailable,
                preferred_session_id,
                ..
            } => run.finish(
                service,
                control,
                offset,
                revision,
                luna_unavailable,
                preferred_session_id,
            )?,
        };
        Ok(run.response(id, data))
    }
    fn start(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        new_id: Option<String>,
        mut run: Run,
    ) -> Result<ResolutionResponse, ApplicationError> {
        let id = new_id.ok_or(ApplicationError::InvalidSearch)?;
        if self.runs.contains_key(&id) {
            return Err(ApplicationError::InvalidSearch);
        }
        let data = run.scan(service, control, true)?;
        let result = run.response(id.clone(), data);
        if self.runs.len() >= RUN_LIMIT
            && let Some(oldest) = self
                .runs
                .iter()
                .min_by_key(|(_, run)| run.started)
                .map(|(key, _)| key.clone())
        {
            self.runs.remove(&oldest);
        }
        self.runs.insert(id, run);
        Ok(result)
    }
}

impl Run {
    fn new(
        binding: String,
        instance: &str,
        intent: Intent,
        parameters: SearchParameters,
        search_intent: Option<Box<SearchIntent>>,
    ) -> Self {
        Self {
            binding,
            instance: instance.to_owned(),
            started: Instant::now(),
            intent,
            search_intent,
            parameters,
            original_filter: None,
            next: None,
            progress: ResolutionProgress {
                scan_passes: 1,
                ..Default::default()
            },
            candidates: Vec::new(),
            anchors: Vec::new(),
            requested_windows: Vec::new(),
            assessment_batches: 0,
            revision: 0,
            publication: None,
        }
    }

    fn contract_error(&self, error: ResolutionError) -> ApplicationError {
        if self.search_intent.is_some() {
            error.into()
        } else if error == ResolutionError::StaleSearchRun {
            ApplicationError::StaleSearchEvidence
        } else {
            ApplicationError::InvalidSearch
        }
    }

    fn response(&self, run: String, data: ResolutionData) -> ResolutionResponse {
        let mut progress = self.progress.clone();
        progress.more_candidates = progress.candidates_presented < self.candidates.len()
            || (!progress.scan_complete && progress.stop_reason.is_none());
        ResolutionResponse {
            run,
            focus: self.intent.focus,
            coverage_goal: self
                .search_intent
                .as_ref()
                .map(|intent| intent.coverage_goal),
            progress,
            data,
        }
    }

    fn scan(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        first: bool,
    ) -> Result<ResolutionData, ApplicationError> {
        let started = Instant::now();
        let first_index = self.progress.candidates_presented;
        let want = first_index + CANDIDATE_BATCH;
        for _ in 0..SCAN_BATCH {
            if self.progress.scan_complete
                || self.progress.stop_reason.is_some()
                || self.candidates.len() >= want
                || started.elapsed() >= SCAN_TIME
            {
                break;
            }
            if self.progress.provider_pages >= SCAN_LIMIT {
                self.progress.stop_reason = Some("search_work_limit");
                break;
            }
            let tick = Instant::now();
            let result =
                service.search(self.parameters.clone(), self.next.clone(), control.clone());
            self.progress.provider_pages += 1;
            self.progress.retrieval_ms += tick.elapsed().as_millis() as u64;
            let result = match result {
                Ok(result) => result,
                Err(ApplicationError::StaleSearchEvidence)
                    if !first || self.progress.provider_pages > 1 =>
                {
                    self.progress.freshness_failures += 1;
                    self.progress.stop_reason = Some("search_revision_changed");
                    break;
                }
                Err(error) => return Err(error),
            };
            if result.binding_revision != self.binding {
                return Err(ApplicationError::StaleSearchEvidence);
            }
            let page = result.page;
            let coverage = page.coverage;
            self.progress.bytes_read += coverage.bytes_read;
            self.progress.records_inspected += coverage.work.records_inspected;
            self.progress.compressed_bytes += coverage.work.compressed_bytes;
            self.progress.decoded_bytes += coverage.work.decoded_bytes;
            self.progress.replay_bytes += coverage.work.replay_bytes;
            self.progress.sources_inspected += coverage.sessions_inspected;
            self.progress.physically_traversed += coverage.fully_traversed_sessions;
            self.progress.partial_sources += coverage.truncated_sessions;
            self.progress.unavailable_sources += coverage.unreadable_sessions;
            self.progress.unknown += coverage.unresolved_sessions;
            self.progress.suppressed_records += coverage.suppressed_records;

            if page.next.is_some() && page.next == self.next {
                self.progress.stop_reason = Some("no_search_progress");
                break;
            }
            self.next = page.next;
            self.progress.scan_complete = self.next.is_none();
            for hit in page.sessions {
                self.add_candidate(hit);
            }
            self.revision += 1;
            self.publication = None;
        }
        let end = want.min(self.candidates.len());
        let candidates = self.candidates[first_index..end]
            .iter()
            .map(|candidate| ResolutionCandidate {
                metadata: candidate.hit.metadata.clone(),
                match_basis: candidate.basis.clone(),
                source_coverage: candidate.hit.source_coverage.clone(),
                evidence: candidate
                    .handles
                    .iter()
                    .zip(&candidate.hit.evidence)
                    .map(|(&handle, evidence)| Citation {
                        handle,
                        reference: self.anchors[handle].reference.clone(),
                        evidence: evidence.clone(),
                    })
                    .collect(),
            })
            .collect();
        self.progress.candidates_presented = end;
        Ok(ResolutionData::Candidates {
            candidates,
            query: (first && self.search_intent.is_none()).then(|| Box::new(self.filter().clone())),
        })
    }

    fn expand(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        expansion: QueryExpr,
    ) -> Result<ResolutionData, ApplicationError> {
        if self.intent.expansion.is_some()
            || !self.progress.scan_complete
            || self.progress.stop_reason.is_some()
            || self.progress.provider_pages >= SCAN_LIMIT
        {
            return Err(self.contract_error(ResolutionError::InvalidQueryExpansion));
        }
        self.intent.expansion = Some(expansion);
        let compiled = if self.search_intent.is_some() {
            self.intent.compile_parameters()
        } else {
            self.intent.parameters()
        };
        let parameters = match compiled {
            Ok(parameters) => parameters,
            Err(_) => {
                self.intent.expansion = None;
                return Err(self.contract_error(ResolutionError::InvalidQueryExpansion));
            }
        };
        self.original_filter = std::mem::replace(&mut self.parameters, parameters)
            .structured
            .map(|query| query.filter);
        self.progress.scan_passes += 1;
        self.progress.scan_complete = false;
        self.revision += 1;
        self.publication = None;
        self.scan(service, control, true)
    }

    fn filter(&self) -> &QueryExpr {
        &self
            .parameters
            .structured
            .as_ref()
            .expect("validated filter")
            .filter
    }

    fn add_candidate(&mut self, mut hit: SessionSearchHit) {
        let basis = self.intent.match_basis(&hit);
        if let Some(previous) = self
            .candidates
            .iter_mut()
            .find(|c| c.hit.metadata.identity == hit.metadata.identity)
        {
            if previous.scan_pass != self.progress.scan_passes
                && previous
                    .hit
                    .evidence
                    .first()
                    .zip(hit.evidence.first())
                    .is_some_and(|(a, b)| {
                        a.source_fingerprint == b.source_fingerprint
                            && a.source_bytes == b.source_bytes
                    })
            {
                previous.basis.expansion = basis.expansion;
            }
            return;
        }
        self.progress.lexical_candidates += 1;
        if basis.literal == Truth::True {
            self.progress.literal_candidates += 1;
        } else if basis.expansion == Some(Truth::True) {
            self.progress.expansion_only_candidates += 1;
        }
        if self.candidates.len() >= CANDIDATE_LIMIT {
            self.progress.stop_reason = Some("candidate_limit");
            return;
        }
        let candidate = self.candidates.len();
        // Citable evidence must contain a visible passage and a bound position.
        hit.evidence.retain(|e| {
            e.position.is_some()
                && e.excerpt
                    .as_ref()
                    .is_some_and(|t| !t.trim().is_empty() && safe_conversation_text(t))
        });
        let handles = hit
            .evidence
            .iter()
            .map(|e| {
                self.add_anchor(Anchor {
                    candidate,
                    reference: reference(&self.binding, &hit.metadata, e),
                    continuation: None,
                    citable: true,
                })
            })
            .collect();
        self.candidates.push(Candidate {
            hit,
            basis,
            handles,
            assessment: None,
            scan_pass: self.progress.scan_passes,
        });
    }

    fn add_anchor(&mut self, anchor: Anchor) -> usize {
        if let Some(index) = self.anchors.iter().position(|a| {
            a.candidate == anchor.candidate
                && a.reference == anchor.reference
                && a.continuation == anchor.continuation
                && a.citable == anchor.citable
        }) {
            return index;
        }
        let index = self.anchors.len();
        self.anchors.push(anchor);
        index
    }

    fn context(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        handle: usize,
        options: Option<ContextOptions>,
    ) -> Result<ResolutionData, ApplicationError> {
        let anchor = self
            .anchors
            .get(handle)
            .filter(|a| a.candidate < self.progress.candidates_presented)
            .cloned()
            .ok_or(ApplicationError::InvalidSearch)?;
        let options = match (anchor.continuation.clone(), options) {
            (Some(fixed), None) => fixed,
            (None, Some(options)) => options,
            (None, None) => ContextOptions {
                max_bytes: 4096,
                ..Default::default()
            },
            (Some(_), Some(_)) => return Err(ApplicationError::InvalidSearch),
        };
        if !options.valid() || self.requested_windows.contains(&(handle, options.clone())) {
            return Err(ApplicationError::InvalidSearch);
        }
        if self.started.elapsed() >= WORK_TIME
            || self.progress.context_requests >= CONTEXT_LIMIT
            || self.progress.context_bytes + options.max_bytes > CONTEXT_BYTES
        {
            return Err(self.contract_error(ResolutionError::BudgetExceeded));
        }
        self.progress.context_requests += 1;
        let result = service.context_window(
            anchor.reference.clone(),
            selector(&self.candidates[anchor.candidate].hit.metadata),
            options.clone(),
            control.clone(),
        );
        if matches!(result, Err(ApplicationError::StaleSearchEvidence)) {
            self.progress.freshness_failures += 1;
        }
        let result = result?;
        let next_options = result.next_options();
        let window = result.window;
        if window.next.as_ref() == Some(&anchor.reference)
            && next_options.as_ref() == Some(&options)
        {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        self.requested_windows.push((handle, options.clone()));
        let candidate = anchor.candidate;
        let messages = window
            .messages
            .into_iter()
            .filter_map(|evidence| {
                let text = evidence
                    .excerpt
                    .as_ref()
                    .filter(|text| !text.trim().is_empty())?;
                self.progress.context_bytes += text.len();
                let reference = reference(
                    &self.binding,
                    &self.candidates[candidate].hit.metadata,
                    &evidence,
                );
                let handle = self.add_anchor(Anchor {
                    candidate,
                    reference: reference.clone(),
                    continuation: None,
                    citable: true,
                });
                Some(Citation {
                    handle,
                    reference,
                    evidence,
                })
            })
            .collect();
        let next = window.next.zip(next_options).map(|(reference, options)| {
            self.add_anchor(Anchor {
                candidate,
                reference,
                continuation: Some(options),
                citable: false,
            })
        });
        let previous = window.previous.map(|reference| {
            self.add_anchor(Anchor {
                candidate,
                reference,
                continuation: Some(ContextOptions {
                    direction: ContextDirection::Backward,
                    message_offset: None,
                    ..options
                }),
                citable: false,
            })
        });
        Ok(ResolutionData::Context {
            messages,
            next,
            previous,
            partial: window.partial,
            at_start: window.at_start,
            at_end: window.at_end,
            source_coverage: window.source_coverage,
            work: window.work,
        })
    }

    fn assess(&mut self, assessments: Vec<Assessment>) -> Result<ResolutionData, ApplicationError> {
        if self.started.elapsed() >= WORK_TIME || self.assessment_batches >= ASSESSMENT_LIMIT {
            return Err(self.contract_error(ResolutionError::BudgetExceeded));
        }
        self.assessment_batches += 1;
        let mut seen = BTreeSet::new();
        let updates: Option<Vec<_>> = assessments
            .into_iter()
            .map(|mut assessment| {
                if self.search_intent.is_some()
                    && assessment.disposition == Disposition::Insufficient
                    && assessment.missing_evidence.is_none()
                    && bounded_text(&assessment.explanation, 512)
                {
                    assessment.missing_evidence = Some(assessment.explanation.clone());
                }
                let candidate = self.candidates[..self.progress.candidates_presented]
                    .iter()
                    .position(|c| {
                        c.hit.metadata.native_session_id == assessment.native_session_id
                    })?;
                let refs: BTreeSet<_> = assessment.evidence.iter().copied().collect();
                let insufficient = assessment.disposition == Disposition::Insufficient;
                let valid = seen.insert(candidate)
                    && bounded_text(&assessment.explanation, 1024)
                    && assessment
                        .missing_evidence
                        .as_ref()
                        .is_none_or(|v| bounded_text(v, 512))
                    && (!insufficient || assessment.missing_evidence.is_some())
                    && (!assessment.plausible_alternative || insufficient && !refs.is_empty())
                    && assessment.outcomes.len() <= 8
                    && refs.len() == assessment.evidence.len()
                    && refs.len() <= 4
                    && (insufficient || !refs.is_empty())
                    && refs.iter().all(|&i| {
                        self.anchors
                            .get(i)
                            .is_some_and(|a| a.candidate == candidate && a.citable)
                    })
                    && (assessment.disposition != Disposition::VerifiedRelevant
                        || self.supports_intent(&assessment, &self.candidates[candidate]));
                valid.then_some((candidate, assessment))
            })
            .collect();
        let Some(updates) = updates.filter(|a| (1..=CANDIDATE_BATCH).contains(&a.len())) else {
            self.progress.invalid_assessments += 1;
            return Err(self.contract_error(ResolutionError::InvalidAssessment));
        };
        for (i, assessment) in updates {
            self.candidates[i].assessment = Some(assessment);
        }
        self.progress.assessments = self
            .candidates
            .iter()
            .filter(|c| c.assessment.is_some())
            .count();
        self.progress.insufficient = self
            .candidates
            .iter()
            .filter(|c| {
                c.assessment
                    .as_ref()
                    .is_some_and(|a| a.disposition == Disposition::Insufficient)
            })
            .count();
        self.revision += 1;
        self.publication = None;
        Ok(ResolutionData::Assessed)
    }

    fn supports_intent(&self, assessment: &Assessment, candidate: &Candidate) -> bool {
        self.intent
            .required_outcomes
            .iter()
            .all(|o| assessment.outcomes.contains(o))
            && self.search_intent.as_ref().is_none_or(|intent| {
                (intent.activity == SearchActivity::Any
                    || assessment.activity == Some(intent.activity)
                    || (intent.activity.evidence_outcome().is_some()
                        && assessment.activity.is_none()))
                    && intent.excluded_outcome.is_none_or(|excluded| {
                        candidate.hit.content_complete && !assessment.outcomes.contains(&excluded)
                    })
            })
    }

    fn finish(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        offset: usize,
        revision: Option<usize>,
        luna_unavailable: bool,
        preferred_session_id: Option<String>,
    ) -> Result<ResolutionData, ApplicationError> {
        if (offset > 0 || revision.is_some()) && revision != Some(self.revision) {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        let v1 = self.search_intent.as_deref();
        if let Some(preferred) = &preferred_session_id
            && (!v1.is_some_and(|spec| spec.result_policy == ResultPolicy::SingleBest)
                || !self.candidates.iter().any(|candidate| {
                    &candidate.hit.metadata.native_session_id == preferred
                        && candidate.assessment.as_ref().is_some_and(|assessment| {
                            assessment.disposition == Disposition::VerifiedRelevant
                        })
                }))
        {
            return Err(ResolutionError::InvalidPreference.into());
        }
        if self.publication.as_ref().is_none_or(|publication| {
            publication.luna_unavailable != luna_unavailable
                || publication.preferred_session_id != preferred_session_id
        }) {
            if offset != 0 {
                return Err(ApplicationError::StaleSearchEvidence);
            }
            let (positives, plausible) = self
                .candidates
                .iter()
                .filter_map(|candidate| candidate.assessment.as_ref())
                .fold((0, 0), |(positives, plausible), assessment| {
                    (
                        positives
                            + usize::from(assessment.disposition == Disposition::VerifiedRelevant),
                        plausible + usize::from(assessment.plausible_alternative),
                    )
                });
            let complete = self.progress.scan_complete
                && self.progress.stop_reason.is_none()
                && self.progress.assessments == self.progress.lexical_candidates
                && !luna_unavailable
                && v1.is_none_or(|intent| {
                    intent.coverage_goal == CoverageGoal::Standard
                        && self.progress.insufficient == 0
                });
            let ambiguous = self.intent.cardinality == AnswerCardinality::Single
                && preferred_session_id.is_none()
                && positives + plausible > 1;
            let outcome = if !complete && (v1.is_some() || !ambiguous) {
                ResolutionOutcome::IncompleteSearch
            } else if ambiguous {
                ResolutionOutcome::Ambiguous
            } else if positives == 0 {
                ResolutionOutcome::NoVerifiedMatch
            } else if self.intent.cardinality == AnswerCardinality::Multiple
                && (v1.is_none() || positives > 1)
            {
                ResolutionOutcome::MultipleVerified
            } else {
                ResolutionOutcome::SingleVerified
            };
            let choices = self
                .candidates
                .iter()
                .enumerate()
                .filter_map(|(index, c)| {
                    let assessment = c.assessment.as_ref()?;
                    let relevant = assessment.disposition == Disposition::VerifiedRelevant;
                    let competing = self.intent.cardinality == AnswerCardinality::Single
                        && assessment.plausible_alternative;
                    if preferred_session_id
                        .as_ref()
                        .is_some_and(|id| id != &c.hit.metadata.native_session_id)
                    {
                        return None;
                    }
                    let related = positives == 0
                        && matches!(
                            assessment.disposition,
                            Disposition::Related | Disposition::Contradiction
                        );
                    (relevant || competing || related).then_some(index)
                })
                .collect();
            self.publication = Some(Publication {
                luna_unavailable,
                preferred_session_id,
                outcome,
                choices,
            });
        }
        let Publication {
            outcome, choices, ..
        } = self.publication.as_ref().expect("built");
        let mut outcome = *outcome;
        if offset > choices.len() {
            return Err(ApplicationError::InvalidSearch);
        }
        let end = (offset + RESULT_BATCH).min(choices.len());
        let single_best = v1.is_some_and(|spec| spec.result_policy == ResultPolicy::SingleBest);
        let preferred = self
            .publication
            .as_ref()
            .is_some_and(|p| p.preferred_session_id.is_some());
        let selected = &choices[offset..end];
        let mut matches = vec![];
        let mut alternatives = vec![];
        let mut failures = 0;
        if !selected.is_empty() {
            // Refresh metadata without re-decoding the scanned source. The
            // frozen content facts can be reused only at the same revision.
            let fresh = service.search(
                SearchParameters {
                    structured: Some(StructuredQuery {
                        filter: QueryExpr::NativeId {
                            values: selected
                                .iter()
                                .map(|&index| {
                                    self.candidates[index]
                                        .hit
                                        .metadata
                                        .native_session_id
                                        .clone()
                                })
                                .collect(),
                        },
                        mode: SearchMode::Exhaustive,
                        sort: SearchSort::Identity,
                        coverage: CoverageRequirement::BestEffort,
                    }),
                    include_content: false,
                    excerpts: false,
                    ..self.parameters.clone()
                },
                None,
                control.clone(),
            )?;
            for &index in selected {
                let candidate = &self.candidates[index];
                let assessment = candidate.assessment.as_ref().expect("selected assessment");
                let valid = fresh
                    .page
                    .sessions
                    .iter()
                    .find(|h| h.metadata.identity == candidate.hit.metadata.identity)
                    .filter(|h| {
                        let filter = if candidate.scan_pass == self.progress.scan_passes {
                            self.filter()
                        } else {
                            self.original_filter.as_ref().expect("first-pass filter")
                        };
                        filter.evaluate_observed(
                            &h.metadata,
                            &candidate.hit.query_observation,
                            candidate.hit.content_complete,
                        ) == Truth::True
                    });
                let Some(current) = valid else {
                    failures += 1;
                    continue;
                };
                let citations = assessment
                    .evidence
                    .iter()
                    .map(|&i| &self.anchors[i].reference);
                let Some(anchor) = citations.clone().next() else {
                    failures += 1;
                    continue;
                };
                if citations.clone().any(|r| {
                    r.source_fingerprint != anchor.source_fingerprint
                        || r.source_bytes != anchor.source_bytes
                }) {
                    failures += 1;
                    continue;
                }
                match service.context_window(
                    anchor.clone(),
                    selector(&current.metadata),
                    ContextOptions {
                        direction: ContextDirection::Start,
                        max_bytes: 64,
                        max_messages: 1,
                        ..Default::default()
                    },
                    control.clone(),
                ) {
                    Ok(_) => {}
                    Err(
                        error @ (ApplicationError::DiscoveryCallerUnavailable
                        | ApplicationError::Cancelled),
                    ) => {
                        return Err(error);
                    }
                    Err(_) => {
                        failures += 1;
                        continue;
                    }
                }
                let choice = ResolvedCandidate {
                    metadata: current.metadata.clone(),
                    assessment: assessment.clone(),
                    citations: citations.cloned().collect(),
                };
                if choice.assessment.disposition == Disposition::VerifiedRelevant
                    && (!single_best || preferred || choices.len() == 1)
                {
                    matches.push(choice);
                } else {
                    alternatives.push(choice);
                }
            }
        }
        if failures > 0 {
            self.progress.freshness_failures += failures;
        }
        if self.progress.freshness_failures > 0 {
            outcome = ResolutionOutcome::IncompleteSearch;
        }
        Ok(ResolutionData::Finished {
            outcome,
            matches,
            alternatives,
            next_offset: (!single_best && end < choices.len()).then_some(end),
            revision: self.revision,
        })
    }
}

fn selector(metadata: &SessionMetadata) -> TargetSelector {
    let identity = metadata.identity.clone();
    Arc::new(move |id| id == &identity)
}

fn reference(
    binding: &str,
    metadata: &SessionMetadata,
    evidence: &ConversationEvidence,
) -> ContextReference {
    ContextReference {
        decoder_progress: None,
        native_session_id: metadata.native_session_id.clone(),
        local_id: metadata.identity.local_id.clone(),
        record_index: evidence.record_index,
        position: evidence.position.clone(),
        source_fingerprint: evidence.source_fingerprint.clone(),
        source_bytes: evidence.source_bytes,
        purpose: SearchPurpose::Discovery,
        binding_revision: binding.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
    use std::{cell::Cell, path::Path};

    const CALLER: &str = "00000000-0000-4000-8000-000000000001";

    struct RepeatingReader(Cell<usize>);

    impl SessionSearchReadPort for RepeatingReader {
        fn search_sessions(
            &self,
            request: &SearchReadRequest,
        ) -> Result<SessionSearchPage, ProviderError> {
            self.0.set(self.0.get() + 1);
            assert_eq!(request.parameters.purpose, SearchPurpose::Discovery);
            assert_eq!(request.discovery_session.as_deref(), Some(CALLER));
            assert_eq!(request.excluded.len(), 1);
            Ok(SessionSearchPage {
                unresolved: vec![],
                sessions: vec![],
                coverage: SearchCoverage::default(),
                inventory_fingerprint: "fixture".into(),
                batch_fingerprint: "fixture".into(),
                next: Some(SearchContinuation {
                    inventory_fingerprint: "fixture".into(),
                    start: 1,
                    ..Default::default()
                }),
            })
        }

        fn session_context(
            &self,
            _: &ContextReference,
            _: &[ArtifactId],
        ) -> Result<ConversationEvidence, ProviderError> {
            panic!("no context for metadata or failed searches")
        }
    }

    fn example_intent() -> Intent {
        serde_json::from_value(serde_json::json!({
            "question":"Find all Codex conversations outside MISSION1 about a permission denied error.",
            "cardinality":"multiple",
            "focus":"event",
            "hard_filter":{"op":"and","args":[
                {"op":"provider","values":["codex"]},
                {"op":"not","arg":{"op":"project","values":["MISSION1"],"source":"verified_label"}}
            ]},
            "literal":{"op":"text","value":"permission denied"},
            "expansion":{"op":"text","value":"EPERM"}
        })).unwrap()
    }

    #[test]
    fn expansion_cannot_override_unknown_hard_membership() {
        let mut intent = example_intent();
        let params = intent.parameters().unwrap();
        let metadata = SessionMetadata {
            identity: ApparentCodexCaller::from_metadata(CALLER, None)
                .unwrap()
                .identity(Path::new("/provider")),
            native_session_id: CALLER.into(),
            title: Some("EPERM".into()),
            project_label: Some("Not the requested project".into()),
            created_at: None,
            last_active_at: None,
            state: SessionState::Active,
            producer_version: None,
            producer_version_matches: false,
            parent_native_session_id: None,
        };
        let filter = params.structured.unwrap().filter;
        let mut observation = QueryObservation::default();
        filter.observe(
            &mut observation,
            &QueryText {
                text: "EPERM",
                timestamp: None,
            },
        );
        assert_eq!(
            filter.evaluate_observed(&metadata, &observation, true),
            Truth::Unknown
        );
        intent.expansion = Some(QueryExpr::Not {
            arg: Box::new(intent.literal.clone()),
        });
        assert!(matches!(
            intent.parameters(),
            Err(ApplicationError::InvalidSearch)
        ));
    }

    #[test]
    fn v1_run_rejects_other_roots_and_registrations_before_reading() {
        let caller = ApparentCodexCaller::from_metadata(CALLER, None).unwrap();
        let reader = RepeatingReader(Cell::new(0));
        let service = SessionSearchService {
            reader: &reader,
            provider_root: Path::new("/provider"),
            provider: ProviderKind::Codex,
            management: None,
            apparent_caller: Some(&caller),
            registered: &[],
            instance: "fixture",
        };
        let spec: agentmender_domain::search_intent::SearchIntent =
            serde_json::from_value(serde_json::json!({
                "schema_version":"1", "subject":"Inspect the permission error",
                "hints":{"terms":["EPERM"]}
            }))
            .unwrap();
        let control = QueryControl::default();
        let mut store = ResolutionStore::default();
        let start = store
            .execute(
                &service,
                ResolutionAction::StartV1(Box::new(spec.validate().unwrap())),
                Some("run".into()),
                &control,
            )
            .unwrap();
        let count = reader.0.get();
        for other in [
            SessionSearchService {
                provider_root: Path::new("/other"),
                ..service
            },
            SessionSearchService {
                instance: "new registration",
                ..service
            },
        ] {
            assert!(matches!(
                store.execute(
                    &other,
                    ResolutionAction::Next {
                        run: start.run.clone()
                    },
                    None,
                    &control
                ),
                Err(ApplicationError::Resolution(
                    ResolutionError::StaleSearchRun
                ))
            ));
        }
        assert_eq!(reader.0.get(), count);
    }

    #[test]
    fn repeated_continuation_stops_and_unavailable_model_preserves_incomplete_outcome() {
        let caller = ApparentCodexCaller::from_metadata(CALLER, None).unwrap();
        let reader = RepeatingReader(Cell::new(0));
        let service = SessionSearchService {
            reader: &reader,
            provider_root: Path::new("/provider"),
            provider: ProviderKind::Codex,
            management: None,
            apparent_caller: Some(&caller),
            registered: &[],
            instance: "fixture",
        };
        let mut store = ResolutionStore::default();
        let control = QueryControl::default();
        let first = store
            .execute(
                &service,
                ResolutionAction::Start(Box::new(example_intent())),
                Some("run".into()),
                &control,
            )
            .unwrap();
        assert_eq!(reader.0.get(), 2);
        assert_eq!(first.progress.stop_reason, Some("no_search_progress"));
        assert!(!first.progress.scan_complete);
        assert!(!first.progress.more_candidates);
        let finish = store
            .execute(
                &service,
                ResolutionAction::Finish {
                    run: first.run,
                    offset: 0,
                    revision: None,
                    luna_unavailable: true,
                    preferred_session_id: None,
                },
                None,
                &control,
            )
            .unwrap();
        assert!(matches!(finish.data, ResolutionData::Finished {
            outcome: ResolutionOutcome::IncompleteSearch,
            matches, .. } if matches.is_empty()));
        assert_eq!(reader.0.get(), 2);
    }
}
