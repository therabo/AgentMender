//! Read-only resolution state for one originating Codex conversation.
mod progressive;
pub use progressive::{
    Advance, AdvanceResult, ContextRequest, ResolutionPhase, TerminalOutcome, WINDOW_BATCH,
};

use crate::{ApplicationError, QueryControl, TargetSelector, session_search::SessionSearchService};
pub use agentmender_domain::search_intent::Outcome;
use agentmender_domain::{
    search_intent::{
        CoverageGoal, ResultPolicy, SearchActivity, SearchIntent, ValidatedSearchIntent,
    },
    *,
};
use serde::Serialize;
use std::sync::Arc;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

const RUN_LIMIT: usize = 4;
const RUN_RETENTION: Duration = Duration::from_secs(600);
const SCAN_TIME: Duration = Duration::from_secs(5);
const CANDIDATE_BATCH: usize = COHORT_BATCH;
const CONTEXT_LIMIT: usize = 24;
const CONTEXT_BYTES: usize = 128 * 1024;

#[derive(Clone)]
struct Retrieval {
    hard_filter: Option<QueryExpr>,
    literal: QueryExpr,
    expansion: Option<QueryExpr>,
    semantic_required: bool,
}

impl Retrieval {
    fn compile_parameters(&self) -> Result<SearchParameters, ApplicationError> {
        let lexical = QueryExpr::Or {
            args: std::iter::once(self.literal.clone())
                .chain(self.expansion.clone())
                .collect(),
        };
        let clauses: Vec<_> = std::iter::once(lexical)
            .chain(self.hard_filter.clone())
            .collect();
        let parameters = SearchParameters {
            structured: Some(StructuredQuery {
                // Positions 0 and 1 are the outer AND and the lexical OR. Keep
                // both lexical observations alongside the original hard filter.
                filter: QueryExpr::And { args: clauses },
                mode: SearchMode::Exhaustive,
                sort: SearchSort::DiscoveryPriority,
                coverage: CoverageRequirement::BestEffort,
            }),
            query: String::new(),
            include_archived: true,
            include_content: self.semantic_required,
            project_label: None,
            purpose: SearchPurpose::Discovery,
            page_size: CANDIDATE_BATCH,
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

pub use agentmender_domain::{Assessment, Disposition, EvidenceRelationship};

pub enum ResolutionAction {
    Start(Box<ValidatedSearchIntent>),
    Advance { run: String, step: Box<Advance> },
}

impl ResolutionAction {
    pub fn is_start(&self) -> bool {
        matches!(self, Self::Start(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResolutionError {
    #[error("The run is stale or belongs to another originating conversation. Start a fresh run.")]
    StaleSearchRun,
    #[error(
        "Assess presented candidates with their own evidence handles and the requested activity and outcome."
    )]
    InvalidAssessment,
    #[error("The current batch context budget is exhausted. Assess available evidence.")]
    BudgetExceeded,
    #[error("Use advance with the current run revision and a new operation ID.")]
    InvalidAdvance,
    #[error("Another operation changed the run. Use its latest revision.")]
    RevisionConflict,
    #[error("Assess the outstanding candidate batch before requesting more candidates.")]
    AssessmentRequired,
    #[error("The captured session cohort is no longer available. Start a new run.")]
    CohortUnavailable,
    #[error("The resolver has no free run slot. Finish or cancel an existing run.")]
    RunCapacity,
}

impl ResolutionError {
    pub fn code(self) -> &'static str {
        match self {
            Self::StaleSearchRun => "stale_search_run",
            Self::InvalidAssessment => "invalid_assessment",
            Self::BudgetExceeded => "budget_exceeded",
            Self::InvalidAdvance => "invalid_advance",
            Self::RevisionConflict => "resolution_revision_conflict",
            Self::AssessmentRequired => "assessment_required",
            Self::RunCapacity => "resolution_run_capacity",
            Self::CohortUnavailable => "search_cohort_unavailable",
        }
    }
}

#[derive(Clone, Serialize)]
pub struct MatchBasis {
    literal: Truth,
    expansion: Option<Truth>,
}

#[derive(Clone)]
pub struct Citation {
    pub handle: usize,
    pub reference: ContextReference,
    pub evidence: ConversationEvidence,
}

#[derive(Clone)]
pub struct ResolutionCandidate {
    pub metadata: SessionMetadata,
    pub match_basis: MatchBasis,
    pub source_coverage: Option<SourceCoverage>,
    pub evidence: Vec<Citation>,
}

pub type ResolvedCandidate = CohortReceipt;

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
    pub sessions_examined: u64,
    pub physically_traversed: usize,
    pub unknown: usize,
    pub partial_sources: usize,
    pub unavailable_sources: usize,
    pub suppressed_records: usize,
    pub lexical_candidates: usize,
    pub snapshot_sessions: u64,
    pub candidates_staged: usize,
    pub candidates_evaluated: usize,
    pub remaining_candidates: usize,
    pub confirmed: usize,
    pub unconfirmed: usize,
    pub rejected: usize,
    pub metadata_verified: usize,
    pub published: usize,
    pub not_published: usize,
    pub retrieval_complete: bool,
    pub evaluation_complete: bool,
    pub publication_ready: bool,
    pub literal_candidates: usize,
    pub expansion_only_candidates: usize,
    pub candidates_presented: usize,
    pub assessments: usize,
    pub insufficient: usize,
    pub context_requests: usize,
    pub context_bytes: usize,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub context_decoded_bytes: u64,
    #[serde(skip_serializing_if = "is_zero_u64")]
    pub context_replay_bytes: u64,
    pub invalid_assessments: usize,
    pub freshness_failures: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub publication_checks: usize,
    #[serde(skip_serializing_if = "ProviderWork::is_empty")]
    pub provider_work: ProviderWork,
    pub scan_complete: bool,
    pub more_candidates: bool,
    pub stop_reason: Option<&'static str>,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone)]
pub struct ResolutionWindow {
    pub messages: Vec<Citation>,
    pub source_coverage: SourceCoverage,
    pub work: ReadWork,
    pub next: Option<usize>,
    pub previous: Option<usize>,
    pub partial: bool,
    pub at_start: bool,
    pub at_end: bool,
}

#[derive(Clone)]
pub enum ResolutionData {
    Candidates(Vec<ResolutionCandidate>),
    Advanced(Box<AdvanceResult>),
}

pub struct ResolutionResponse {
    pub run: String,
    pub result_policy: ResultPolicy,
    pub coverage_goal: CoverageGoal,
    pub revision: usize,
    pub progressive_coverage: progressive::Coverage,
    pub execution_state: SearchExecutionState,
    pub phase: ResolutionPhase,
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

#[derive(Clone)]
struct Candidate {
    hit: SessionSearchHit,
    basis: MatchBasis,
    metadata_match: bool,
    handles: Vec<usize>,
    assessment: Option<Assessment>,
    stale_context: bool,
}

#[derive(Clone)]
struct Run {
    binding: String,
    instance: String,
    retrieval: Retrieval,
    search_intent: SearchIntent,
    parameters: SearchParameters,
    next: Option<CohortPosition>,
    cohort_id: String,
    progress: ResolutionProgress,
    candidates: Vec<Candidate>,
    anchors: Vec<Anchor>,
    requested_windows: Vec<(usize, ContextOptions)>,
    revision: usize,
    stream: progressive::Stream,
    last_used: Instant,
}

#[derive(Default)]
pub struct ResolutionStore {
    runs: BTreeMap<String, Run>,
}

impl ResolutionStore {
    pub fn release(&mut self, service: &SessionSearchService<'_>, id: &str) {
        if self.runs.get(id).is_some_and(|run| {
            run.instance == service.instance
                && service
                    .binding(SearchPurpose::Discovery)
                    .is_ok_and(|binding| run.binding == binding)
        }) {
            self.runs.remove(id);
            service.release_cohort(id);
        }
    }

    pub fn execute(
        &mut self,
        service: &SessionSearchService<'_>,
        action: ResolutionAction,
        new_id: Option<String>,
        control: &QueryControl,
    ) -> Result<ResolutionResponse, ApplicationError> {
        control.check()?;
        let binding = service.binding(SearchPurpose::Discovery)?;
        self.runs.retain(|id, run| {
            let live = run.last_used.elapsed() < RUN_RETENTION;
            if !live && !run.can_reclaim() {
                service.release_cohort(id);
            }
            live
        });
        match action {
            ResolutionAction::Start(plan) => {
                let (intent, hard_filter, literal, expansion) = plan.into_parts();
                let retrieval = Retrieval {
                    hard_filter,
                    literal,
                    expansion,
                    semantic_required: intent.requires_semantic_evidence(),
                };
                let parameters = retrieval.compile_parameters()?;
                self.start(
                    service,
                    control,
                    new_id,
                    Run::new(binding, service.instance, retrieval, parameters, intent),
                )
            }
            ResolutionAction::Advance { run, step } => {
                let state = self
                    .runs
                    .get_mut(&run)
                    .ok_or(ResolutionError::StaleSearchRun)?;
                if state.binding != binding || state.instance != service.instance {
                    return Err(ResolutionError::StaleSearchRun.into());
                }
                let was_active = !state.can_reclaim();
                let result = state.advance(service, control, run.clone(), *step);
                if was_active && state.can_reclaim() {
                    service.release_cohort(&run);
                }
                result
            }
        }
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
        let reclaim = if self.runs.len() >= RUN_LIMIT {
            Some(
                self.runs
                    .iter()
                    .filter(|(_, run)| run.can_reclaim())
                    .min_by_key(|(_, run)| run.last_used)
                    .map(|(key, _)| key.clone())
                    .ok_or(ResolutionError::RunCapacity)?,
            )
        } else {
            None
        };
        run.cohort_id.clone_from(&id);
        let acquisition = service
            .capture_cohort(&id, &run.parameters, control)
            .and_then(|work| {
                run.progress.provider_work.add(&work);
                run.scan(service, control)
            });
        let data = match acquisition {
            Ok(()) => {
                let reply = run.retrieval_response();
                if run.can_reclaim() {
                    service.release_cohort(&id);
                }
                reply
            }
            Err(
                error @ (ApplicationError::Provider(_)
                | ApplicationError::StaleSearchEvidence
                | ApplicationError::Resolution(ResolutionError::CohortUnavailable)),
            ) => {
                service.release_cohort(&id);
                run.blocked(error.public_code())
            }
            Err(error) => {
                service.release_cohort(&id);
                return Err(error);
            }
        };
        let result = run.response(id.clone(), data);
        if let Some(reclaim) = reclaim {
            self.runs.remove(&reclaim);
        }
        self.runs.insert(id, run);
        Ok(result)
    }
}

impl Run {
    fn has_source_gaps(&self) -> bool {
        self.progress.unknown != 0
            || self.progress.partial_sources != 0
            || self.progress.unavailable_sources != 0
            || self.progress.suppressed_records != 0
    }

    fn new(
        binding: String,
        instance: &str,
        retrieval: Retrieval,
        parameters: SearchParameters,
        search_intent: SearchIntent,
    ) -> Self {
        Self {
            binding,
            instance: instance.to_owned(),
            retrieval,
            search_intent,
            parameters,
            next: None,
            cohort_id: String::new(),
            progress: ResolutionProgress {
                scan_passes: 1,
                ..Default::default()
            },
            candidates: Vec::new(),
            anchors: Vec::new(),
            requested_windows: Vec::new(),
            revision: 0,
            stream: progressive::Stream::default(),
            last_used: Instant::now(),
        }
    }

    fn response(&self, run: String, data: ResolutionData) -> ResolutionResponse {
        let mut progress = self.progress.clone();
        self.project_progress(&mut progress);
        ResolutionResponse {
            run,
            result_policy: self.search_intent.result_policy,
            coverage_goal: self.search_intent.coverage_goal,
            revision: self.revision,
            progressive_coverage: self.stream_coverage(),
            execution_state: self.stream.execution_state(),
            phase: self.phase(),
            progress,
            data,
        }
    }

    fn scan(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
    ) -> Result<(), ApplicationError> {
        let started = Instant::now();
        for _ in 0..COHORT_SCAN_PAGES {
            if self.progress.scan_complete
                || self.progress.stop_reason.is_some()
                || started.elapsed() >= SCAN_TIME
            {
                break;
            }
            let tick = Instant::now();
            let mut page = service.read_cohort(
                CohortRead {
                    scope: service.cohort_scope(&self.cohort_id)?,
                    parameters: self.parameters.clone(),
                    position: self.next.clone().unwrap_or_default(),
                    refresh: None,
                },
                control,
            )?;
            self.progress.provider_pages += 1;
            self.progress.retrieval_ms += tick.elapsed().as_millis() as u64;
            let coverage = page.coverage;
            self.progress.bytes_read += coverage.bytes_read;
            self.progress.records_inspected += coverage.work.records_inspected;
            self.progress.compressed_bytes += coverage.work.compressed_bytes;
            self.progress.decoded_bytes += coverage.work.decoded_bytes;
            self.progress.replay_bytes += coverage.work.replay_bytes;
            self.progress.provider_work.add(&coverage.work.provider);
            self.progress.sources_inspected += coverage.sessions_inspected;
            self.progress.physically_traversed += coverage.fully_traversed_sessions;
            self.progress.partial_sources += coverage.truncated_sessions;
            self.progress.unavailable_sources += coverage.unreadable_sessions;
            self.progress.unknown += coverage.unresolved_sessions;
            self.progress.suppressed_records += coverage.suppressed_records;
            self.progress.freshness_failures += usize::from(
                coverage
                    .reasons
                    .iter()
                    .any(|reason| reason == "source_revision_changed"),
            );

            if let Some(next) = &page.next
                && !progressive::cursor_advances(self.next.as_ref(), next)
            {
                self.progress.stop_reason = Some("no_search_progress");
                break;
            }
            if !self.observe_batch_order(&mut page.sessions) {
                self.progress.stop_reason = Some("unstable_candidate_order");
                break;
            }
            self.next = page.next;
            self.progress.scan_complete = self.next.is_none();
            for hit in page.sessions {
                self.count_candidate(&hit);
            }
            self.revision += 1;
        }
        Ok(())
    }

    fn candidate_batch(&self, start: usize, end: usize) -> Vec<ResolutionCandidate> {
        self.candidates[start..end]
            .iter()
            .filter(|candidate| {
                !candidate.metadata_match
                    && candidate.assessment.is_none()
                    && !candidate.stale_context
            })
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
                        reference: self
                            .anchor(handle)
                            .expect("active anchor")
                            .reference
                            .clone(),
                        evidence: evidence.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    fn filter(&self) -> &QueryExpr {
        &self
            .parameters
            .structured
            .as_ref()
            .expect("validated filter")
            .filter
    }

    fn count_candidate(&mut self, hit: &SessionSearchHit) {
        let basis = self.retrieval.match_basis(hit);
        self.progress.candidates_staged += 1;
        self.progress.lexical_candidates += usize::from(!hit.matched_terms.is_empty());
        if basis.literal == Truth::True {
            self.progress.literal_candidates += 1;
        } else if basis.expansion == Some(Truth::True) {
            self.progress.expansion_only_candidates += 1;
        }
    }

    fn retain_candidate(&mut self, mut hit: SessionSearchHit, basis: MatchBasis) {
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
        let metadata_match = !self.retrieval.semantic_required
            && self.filter().metadata_truth(&hit.metadata) == Truth::True;
        self.candidates.push(Candidate {
            hit,
            basis,
            metadata_match,
            handles,
            assessment: None,
            stale_context: false,
        });
    }

    fn add_anchor(&mut self, anchor: Anchor) -> usize {
        if let Some(index) = self.anchors.iter().position(|a| {
            a.candidate == anchor.candidate
                && a.reference == anchor.reference
                && a.continuation == anchor.continuation
                && a.citable == anchor.citable
        }) {
            return self.anchor_base() + index;
        }
        let index = self.anchors.len();
        self.anchors.push(anchor);
        self.anchor_base() + index
    }

    fn context(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        handle: usize,
        options: Option<ContextOptions>,
    ) -> Result<ResolutionWindow, ApplicationError> {
        let anchor = self
            .anchor(handle)
            .filter(|a| a.candidate < self.presented())
            .cloned()
            .ok_or(ApplicationError::InvalidSearch)?;
        let options = match (anchor.continuation.clone(), options) {
            (Some(fixed), None) => fixed,
            (None, Some(options)) => options,
            (None, None) => ContextOptions {
                max_bytes: 4096,
                max_messages: 5,
                ..Default::default()
            },
            (Some(_), Some(_)) => return Err(ApplicationError::InvalidSearch),
        };
        if !options.valid() || self.requested_windows.contains(&(handle, options.clone())) {
            return Err(ApplicationError::InvalidSearch);
        }
        if self.requested_windows.len() >= CONTEXT_LIMIT
            || self.active_context_bytes() + options.max_bytes > CONTEXT_BYTES
        {
            return Err(ResolutionError::BudgetExceeded.into());
        }
        self.progress.context_requests += 1;
        self.requested_windows.push((handle, options.clone()));
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
        self.progress.provider_work.add(&window.work.provider);
        self.progress.context_decoded_bytes += window.work.decoded_bytes;
        self.progress.context_replay_bytes += window.work.replay_bytes;
        if window.next.as_ref() == Some(&anchor.reference)
            && next_options.as_ref() == Some(&options)
        {
            return Err(ApplicationError::StaleSearchEvidence);
        }
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
        Ok(ResolutionWindow {
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

    fn assess(&mut self, assessments: Vec<Assessment>) -> Result<(), ApplicationError> {
        let mut seen = BTreeSet::new();
        let updates: Option<Vec<_>> = assessments
            .into_iter()
            .map(|mut assessment| {
                if assessment.disposition == Disposition::Insufficient
                    && assessment.missing_evidence.is_none()
                    && bounded_text(&assessment.explanation, 512)
                {
                    assessment.missing_evidence = Some(assessment.explanation.clone());
                }
                let candidate = self.candidates[..self.presented()].iter().position(|c| {
                    !c.metadata_match
                        && !c.stale_context
                        && c.hit.metadata.native_session_id == assessment.native_session_id
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
                        self.anchor(i)
                            .is_some_and(|a| a.candidate == candidate && a.citable)
                    })
                    && (assessment.disposition != Disposition::VerifiedRelevant
                        || self.supports_intent(&assessment));
                valid.then_some((candidate, assessment))
            })
            .collect();
        let Some(updates) = updates.filter(|a| (1..=CANDIDATE_BATCH).contains(&a.len())) else {
            self.progress.invalid_assessments += 1;
            return Err(ResolutionError::InvalidAssessment.into());
        };
        if updates
            .iter()
            .all(|(i, assessment)| self.candidates[*i].assessment.as_ref() == Some(assessment))
        {
            return Err(ResolutionError::InvalidAdvance.into());
        }
        for (i, assessment) in updates {
            self.candidates[i].assessment = Some(assessment);
        }
        self.refresh_assessment_counts();
        self.revision += 1;
        Ok(())
    }

    fn refresh_assessment_counts(&mut self) {
        self.progress.assessments = self.stream.evaluated()
            + self
                .candidates
                .iter()
                .filter(|c| c.assessment.is_some())
                .count();
        self.progress.insufficient = self.stream.unconfirmed()
            + self
                .candidates
                .iter()
                .filter(|c| {
                    c.assessment
                        .as_ref()
                        .is_some_and(|a| a.disposition == Disposition::Insufficient)
                })
                .count();
    }

    fn supports_intent(&self, assessment: &Assessment) -> bool {
        let intent = &self.search_intent;
        let required_activity = intent.activity.evidence_outcome();
        assessment.relationship == Some(EvidenceRelationship::Direct)
            && intent.required_outcome.into_iter().chain(required_activity)
                .all(|outcome| assessment.outcomes.contains(&outcome))
            && (intent.activity == SearchActivity::Any
                || assessment.activity == Some(intent.activity)
                || (required_activity.is_some() && assessment.activity.is_none()))
            // Bounded passages cannot establish a session-wide semantic exclusion.
            && intent.excluded_outcome.is_none()
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
            _: &SearchReadRequest,
        ) -> Result<SessionSearchPage, ProviderError> {
            panic!("progressive resolution never invokes live inventory")
        }

        fn capture_cohort(
            &self,
            _: &CohortScope,
            _: &SearchParameters,
        ) -> Result<ProviderWork, ProviderError> {
            Ok(ProviderWork::default())
        }

        fn read_cohort(&self, read: &CohortRead) -> Result<CohortPage, ProviderError> {
            self.0.set(self.0.get() + 1);
            assert_eq!(read.parameters.purpose, SearchPurpose::Discovery);
            assert_eq!(read.scope.caller, CALLER);
            assert_eq!(read.scope.excluded.len(), 1);
            Ok(CohortPage {
                unresolved: vec![],
                sessions: vec![],
                coverage: SearchCoverage::default(),
                next: Some(CohortPosition {
                    index: 1,
                    after: Some("00000000-0000-4000-8000-000000000009".into()),
                    stream: None,
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

    pub(super) fn example_intent() -> ValidatedSearchIntent {
        serde_json::from_value::<SearchIntent>(serde_json::json!({
            "schema_version":"1",
            "subject":"Find all Codex conversations outside MISSION1 about a permission denied error",
            "hints":{"terms":["permission denied"]},
            "constraints":{"target_provider":"codex","project_membership":{
                "mode":"exclude","names":["MISSION1"],"basis":"verified"
            }}
        })).unwrap().validate().unwrap()
    }

    fn step(revision: usize) -> Box<Advance> {
        Box::new(Advance {
            expected_revision: revision,
            operation_id: "next".into(),
            assessments: vec![],
            context_requests: vec![],
            publish: false,
            keep_alive: false,
            cancel: false,
            luna_unavailable: false,
        })
    }

    #[test]
    fn expansion_cannot_override_unknown_hard_membership() {
        let (_, hard_filter, literal, _) = example_intent().into_parts();
        let retrieval = Retrieval {
            hard_filter,
            literal,
            semantic_required: false,
            expansion: Some(
                agentmender_domain::search_intent::compile_expansion(&["EPERM".into()]).unwrap(),
            ),
        };
        let params = retrieval.compile_parameters().unwrap();
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
                ResolutionAction::Start(Box::new(spec.validate().unwrap())),
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
                    ResolutionAction::Advance {
                        run: start.run.clone(),
                        step: step(start.revision),
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
    fn repeated_continuation_interrupts_before_evaluation() {
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
        assert!(matches!(first.data, ResolutionData::Advanced(reply)
            if reply.outcome == Some(TerminalOutcome::Blocked) && reply.matches.is_empty()));
        assert_eq!(first.execution_state, SearchExecutionState::Interrupted);
        assert_eq!(first.phase, ResolutionPhase::Interrupted);
        assert!(!first.progress.evaluation_complete);
        assert_eq!(reader.0.get(), 2);
    }
}
