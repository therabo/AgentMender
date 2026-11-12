use super::*;
use std::collections::VecDeque;

pub const WINDOW_BATCH: usize = 2;

#[derive(Clone, PartialEq, Eq)]
pub struct ContextRequest {
    pub anchor: usize,
    pub window: Option<ContextOptions>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Advance {
    pub expected_revision: usize,
    pub operation_id: String,
    pub assessments: Vec<Assessment>,
    pub context_requests: Vec<ContextRequest>,
    pub expansion: Vec<String>,
    pub cancel: bool,
    pub luna_unavailable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    CompletedWithMatches,
    CompletedNoMatches,
    Partial,
    Blocked,
    Cancelled,
}

impl TerminalOutcome {
    fn execution_state(self) -> SearchExecutionState {
        match self {
            Self::CompletedWithMatches | Self::CompletedNoMatches => {
                SearchExecutionState::Completed
            }
            Self::Partial | Self::Blocked | Self::Cancelled => SearchExecutionState::Interrupted,
        }
    }
}

#[derive(Clone, Default)]
pub struct AdvanceResult {
    pub matches: Vec<ResolvedCandidate>,
    pub alternatives: Vec<ResolvedCandidate>,
    pub windows: Vec<ResolutionWindow>,
    pub context_failures: Vec<ContextFailure>,
    pub candidates: Vec<ResolutionCandidate>,
    pub outcome: Option<TerminalOutcome>,
    pub replayed: bool,
    pub invalidated: Vec<String>,
    replay_filter: Option<QueryExpr>,
    replay_observations: BTreeMap<String, (QueryObservation, bool)>,
}

#[derive(Clone)]
pub struct ContextFailure {
    pub anchor: usize,
    pub reason: &'static str,
}

#[derive(Serialize)]
pub struct Coverage {
    inventory_complete: bool,
    physical_history_complete: bool,
    logical_history_complete: bool,
    eligible_evaluation_complete: bool,
    candidate_evaluation_complete: bool,
    lexical_coverage_limited: bool,
    confirmed: usize,
    unconfirmed: usize,
    rejected: usize,
    published: usize,
    released: usize,
    inventory_window_offset: usize,
    current_source_decoded_offset: Option<u64>,
}

#[derive(Clone, Default)]
pub(super) struct Stream {
    released: usize,
    refresh: VecDeque<CandidateRefresh>,
    refreshing_batch: bool,
    refresh_coverage_limited: bool,
    verification_gap: Option<&'static str>,
    anchor_base: usize,
    prior_context_bytes: usize,
    confirmed: usize,
    unconfirmed: usize,
    rejected: usize,
    terminal: Option<TerminalOutcome>,
    previous: Option<(Advance, AdvanceResult)>,
    last_candidate: Option<(std::cmp::Reverse<usize>, String)>,
    inventory_boundary: Option<String>,
    reanchored: Option<(Option<InventoryBoundary>, usize)>,
}

#[derive(Clone)]
struct CandidateRefresh {
    id: String,
    identity: ArtifactId,
    next: Option<SearchContinuation>,
}

impl Stream {
    pub(super) fn execution_state(&self) -> SearchExecutionState {
        self.terminal.map_or(
            SearchExecutionState::InProgress,
            TerminalOutcome::execution_state,
        )
    }

    pub(super) fn reset_order(&mut self) {
        self.last_candidate = None;
        self.inventory_boundary = None;
        self.reanchored = None;
    }

    pub(super) fn has_refresh_work(&self) -> bool {
        !self.refresh.is_empty()
    }

    pub(super) fn unconfirmed(&self) -> usize {
        self.unconfirmed
    }

    fn queue_refresh(&mut self, candidate: &Candidate) {
        if self.refreshing_batch {
            self.verification_gap = Some("candidate_evidence_stale");
        } else {
            self.refresh.push_back(CandidateRefresh {
                id: candidate.hit.metadata.native_session_id.clone(),
                identity: candidate.hit.metadata.identity.clone(),
                next: None,
            });
        }
    }

    pub(super) fn evaluated(&self) -> usize {
        self.confirmed + self.unconfirmed + self.rejected
    }
}

impl Run {
    pub(super) fn can_reclaim(&self) -> bool {
        self.stream.terminal.is_some()
    }

    pub(super) fn blocked(&mut self, reason: &'static str) -> ResolutionData {
        self.progress.stop_reason = Some(reason);
        self.stream.terminal = Some(TerminalOutcome::Blocked);
        ResolutionData::Advanced(Box::new(AdvanceResult {
            outcome: Some(TerminalOutcome::Blocked),
            ..Default::default()
        }))
    }

    pub(super) fn presented(&self) -> usize {
        self.candidates.len()
    }

    pub(super) fn anchor_base(&self) -> usize {
        self.stream.anchor_base
    }

    pub(super) fn anchor(&self, handle: usize) -> Option<&Anchor> {
        self.anchors.get(handle.checked_sub(self.anchor_base())?)
    }

    pub(super) fn active_context_bytes(&self) -> usize {
        self.progress.context_bytes - self.stream.prior_context_bytes
    }

    pub(super) fn observe_batch_order(&mut self, hits: &mut Vec<SessionSearchHit>) -> bool {
        let boundary = self
            .next
            .as_ref()
            .and_then(|cursor| cursor.inventory_after.as_ref())
            .map(|key| key.native_session_id.clone());
        let stream = &mut self.stream;
        let mut previous = if stream.inventory_boundary == boundary {
            stream.last_candidate.clone()
        } else {
            None
        };
        let terms = self.parameters.ranking_terms();
        let mut valid = true;
        hits.retain(|hit| {
            let id = &hit.metadata.native_session_id;
            let key = (
                std::cmp::Reverse(discovery_title_weight(&hit.metadata, &terms)),
                id.clone(),
            );
            if boundary.as_ref().is_some_and(|boundary| id <= boundary)
                || previous.as_ref().is_some_and(|prior| key < *prior)
            {
                valid = false;
                return false;
            }
            if previous.as_ref() == Some(&key) {
                return false;
            }
            previous = Some(key);
            true
        });
        if !valid {
            return false;
        }
        stream.inventory_boundary = boundary;
        stream.last_candidate = previous;
        true
    }

    pub(super) fn stream_coverage(&self) -> Coverage {
        let stream = &self.stream;
        let p = &self.progress;
        let single_pass = p.scan_passes == 1;
        let physical = p.scan_complete && p.physically_traversed == p.sources_inspected;
        let lexical_coverage_limited = self.has_source_gaps() || stream.refresh_coverage_limited;
        let logical = physical && !lexical_coverage_limited;
        Coverage {
            inventory_complete: p.scan_complete,
            physical_history_complete: physical,
            logical_history_complete: logical,
            eligible_evaluation_complete: logical
                && single_pass
                && p.assessments == p.sources_inspected
                && p.insufficient == 0
                && p.freshness_failures == 0,
            candidate_evaluation_complete: p.scan_complete
                && p.unavailable_sources == 0
                && p.stop_reason.is_none()
                && stream.evaluated() == p.lexical_candidates,
            lexical_coverage_limited,
            confirmed: stream.confirmed,
            unconfirmed: stream.unconfirmed,
            rejected: stream.rejected,
            published: stream.confirmed,
            released: stream.released,
            inventory_window_offset: self.next.as_ref().map_or(0, |cursor| cursor.start),
            current_source_decoded_offset: self
                .next
                .as_ref()
                .and_then(|cursor| cursor.stream.as_ref())
                .map(|cursor| cursor.byte_offset),
        }
    }

    pub(super) fn reanchor_source(&mut self) -> Option<SearchContinuation> {
        let stream = &mut self.stream;
        let cursor = self.next.as_ref()?;
        cursor.stream.as_ref()?;
        let position = (cursor.inventory_after.clone(), cursor.start);
        if stream.reanchored.as_ref() == Some(&position) {
            return None;
        }
        stream.reanchored = Some(position);
        Some(SearchContinuation {
            stream: None,
            ..cursor.clone()
        })
    }

    fn release_batch(&mut self) {
        let stream = &mut self.stream;
        stream.anchor_base += self.anchors.len();
        stream.prior_context_bytes = self.progress.context_bytes;
        self.candidates.clear();
        self.anchors.clear();
        self.requested_windows.clear();
        self.refresh_assessment_counts();
    }

    pub(super) fn advance(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        id: String,
        step: Advance,
    ) -> Result<ResolutionResponse, ApplicationError> {
        let stream = &self.stream;
        if !bounded_text(&step.operation_id, 64)
            || step.assessments.len() > CANDIDATE_BATCH
            || step.context_requests.len() > WINDOW_BATCH
            || step.context_requests.iter().any(|request| {
                request
                    .window
                    .as_ref()
                    .is_some_and(|window| window.max_messages > 5 || window.max_bytes > 4096)
            })
            || (step.cancel || step.luna_unavailable)
                && (!step.assessments.is_empty()
                    || !step.context_requests.is_empty()
                    || !step.expansion.is_empty())
            || step.cancel && step.luna_unavailable
            || !step.context_requests.is_empty() && !step.expansion.is_empty()
        {
            return Err(ResolutionError::InvalidAdvance.into());
        }
        if let Some((previous, reply)) = &stream.previous
            && step.operation_id == previous.operation_id
        {
            if &step != previous {
                return Err(ResolutionError::RevisionConflict.into());
            }
            let mut data = reply.clone();
            data.replayed = true;
            self.verify_replayed_results(service, control, &mut data)?;
            return Ok(self.response(id, ResolutionData::Advanced(Box::new(data))));
        }
        if step.expected_revision != self.revision || stream.terminal.is_some() {
            return Err(ResolutionError::RevisionConflict.into());
        }
        // A rejected request or cancelled transport never commits half a batch.
        // Only the active batch and one preceding operation are resident.
        let mut staged = self.clone();
        let reply = staged.apply_step(service, control, &step)?;
        control.check()?;
        staged.revision += 1;
        staged.last_used = Instant::now();
        let stream = &mut staged.stream;
        stream.terminal = reply.outcome;
        stream.previous = Some((step, reply.clone()));
        *self = staged;
        Ok(self.response(id, ResolutionData::Advanced(Box::new(reply))))
    }

    fn apply_step(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        step: &Advance,
    ) -> Result<AdvanceResult, ApplicationError> {
        let mut reply = AdvanceResult::default();
        if step.cancel {
            self.progress.stop_reason = Some("cancelled_by_caller");
            self.stream.refresh.clear();
            self.release_batch();
            reply.outcome = Some(TerminalOutcome::Cancelled);
            return Ok(reply);
        }
        if step.luna_unavailable {
            self.progress.stop_reason = Some("luna_unavailable");
            self.stream.refresh.clear();
            self.release_batch();
            reply.outcome = Some(TerminalOutcome::Blocked);
            return Ok(reply);
        }
        if !step.assessments.is_empty() {
            self.assess(step.assessments.clone())?;
        }
        if !step.context_requests.is_empty() {
            self.context_step(service, control, &step.context_requests, &mut reply)?;
            return Ok(reply);
        }
        let needs_assessment = self
            .candidates
            .iter()
            .any(|candidate| candidate.assessment.is_none() && !candidate.stale_context);
        if needs_assessment {
            if !step.expansion.is_empty() {
                return Err(ResolutionError::InvalidQueryExpansion.into());
            }
            if step.assessments.is_empty() {
                return Err(ResolutionError::AssessmentRequired.into());
            }
            reply.candidates = self
                .candidate_batch(0, self.presented())
                .into_iter()
                .zip(&self.candidates)
                .filter_map(|(view, candidate)| {
                    (candidate.assessment.is_none() && !candidate.stale_context).then_some(view)
                })
                .collect();
            return Ok(reply);
        }
        if !self.candidates.is_empty() {
            self.publish_batch(service, control, &mut reply)?;
            self.release_batch();
        }
        if self.stream.has_refresh_work() {
            // The caller can propose expansion again after stale candidates are settled.
            reply.candidates = self.refresh_candidates(service, control)?;
            if !reply.candidates.is_empty() || self.stream.has_refresh_work() {
                return Ok(reply);
            }
        }
        if !step.expansion.is_empty() {
            let expansion = compile_expansion(&step.expansion)
                .map_err(|_| ResolutionError::InvalidQueryExpansion)?;
            if expansion
                .text_terms()
                .iter()
                .any(|term| self.retrieval.literal.text_terms().contains(term))
            {
                return Err(ResolutionError::InvalidQueryExpansion.into());
            }
            self.expand(expansion)?;
        }
        let published = self.stream.confirmed;
        if !self.progress.scan_complete && self.progress.stop_reason.is_none() {
            match self.scan(service, control, false) {
                Ok(candidates) => reply.candidates = candidates,
                Err(
                    error @ (ApplicationError::Cancelled
                    | ApplicationError::DiscoveryCallerUnavailable),
                ) => {
                    return Err(error);
                }
                Err(error) => {
                    self.progress.stop_reason = Some(error.public_code());
                    reply.outcome = Some(if published > 0 {
                        TerminalOutcome::Partial
                    } else {
                        TerminalOutcome::Blocked
                    });
                }
            }
        }
        if (self.progress.stop_reason.is_some() || self.progress.scan_complete)
            && reply.candidates.is_empty()
            && reply.outcome.is_none()
        {
            reply.outcome = Some(self.terminal_outcome(published));
        }
        Ok(reply)
    }

    fn context_step(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        requests: &[ContextRequest],
        reply: &mut AdvanceResult,
    ) -> Result<(), ApplicationError> {
        let merged = match requests {
            [first, second] => merge_windows(first, second),
            _ => None,
        };
        let requests = match requests {
            [first, second] if first == second => &requests[..1],
            _ => merged.as_ref().map_or(requests, std::slice::from_ref),
        };
        for request in requests {
            let candidate = self
                .anchor(request.anchor)
                .ok_or(ApplicationError::InvalidSearch)?
                .candidate;
            let result = if self.candidates[candidate].stale_context {
                Err(ApplicationError::StaleSearchEvidence)
            } else {
                self.context(service, control, request.anchor, request.window.clone())
            };
            match result {
                Ok(window) => {
                    reply.windows.push(window);
                    self.candidates[candidate].assessment = None;
                }
                Err(
                    error @ (ApplicationError::StaleSearchEvidence
                    | ApplicationError::Provider(_)
                    | ApplicationError::Resolution(ResolutionError::BudgetExceeded)),
                ) => {
                    if matches!(error, ApplicationError::StaleSearchEvidence) {
                        self.candidates[candidate].stale_context = true;
                        self.candidates[candidate].assessment = None;
                    }
                    reply.context_failures.push(ContextFailure {
                        anchor: request.anchor,
                        reason: error.public_code(),
                    });
                }
                Err(error) => return Err(error),
            }
        }
        self.refresh_assessment_counts();
        Ok(())
    }

    fn refresh_candidates(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
    ) -> Result<Vec<ResolutionCandidate>, ApplicationError> {
        self.stream.refreshing_batch = true;
        let started = Instant::now();
        for _ in 0..CANDIDATE_BATCH {
            if started.elapsed() >= SCAN_TIME {
                break;
            }
            let stream = &mut self.stream;
            let Some(mut refresh) = stream.refresh.pop_front() else {
                break;
            };
            let mut parameters = self.parameters.clone();
            let QueryExpr::And { args } = &mut parameters
                .structured
                .as_mut()
                .expect("resolution query")
                .filter
            else {
                unreachable!("compiled resolution conjunction")
            };
            args.push(QueryExpr::NativeId {
                values: vec![refresh.id.clone()],
            });
            let result = service.search(parameters, refresh.next.clone(), control.clone());
            let page = match result {
                Ok(result) if result.binding_revision == self.binding => result.page,
                Err(
                    error @ (ApplicationError::Cancelled
                    | ApplicationError::DiscoveryCallerUnavailable),
                ) => {
                    return Err(error);
                }
                _ => {
                    stream.verification_gap = Some("candidate_evidence_stale");
                    continue;
                }
            };
            self.progress
                .provider_work
                .add(&page.coverage.work.provider);
            stream.refresh_coverage_limited |= page.coverage.truncated_sessions != 0
                || page.coverage.unreadable_sessions != 0
                || page.coverage.unresolved_sessions != 0
                || page.coverage.suppressed_records != 0;
            if let Some(hit) = page
                .sessions
                .into_iter()
                .find(|hit| hit.metadata.identity == refresh.identity)
            {
                let basis = self.retrieval.match_basis(&hit);
                self.retain_candidate(hit, basis);
            } else if let Some(next) = page.next
                && cursor_advances(refresh.next.as_ref(), &next)
            {
                refresh.next = Some(next);
                stream.refresh.push_back(refresh);
            } else {
                stream.verification_gap = Some("candidate_evidence_stale");
            }
        }
        let candidates = self.candidate_batch(0, self.presented());
        if candidates.is_empty() && !self.stream.has_refresh_work() {
            self.stream.refreshing_batch = false;
        }
        Ok(candidates)
    }

    fn publish_batch(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        reply: &mut AdvanceResult,
    ) -> Result<(), ApplicationError> {
        let mut checks = Vec::new();
        let mut choices = Vec::new();
        for candidate in &self.candidates {
            let Some(assessment) = candidate
                .assessment
                .as_ref()
                .filter(|_| !candidate.stale_context)
            else {
                continue;
            };
            let citations: Vec<_> = assessment
                .evidence
                .iter()
                .map(|&handle| {
                    self.anchor(handle)
                        .expect("validated assessment anchor")
                        .reference
                        .clone()
                })
                .collect();
            let check = citations
                .first()
                .or_else(|| {
                    candidate.handles.first().map(|&handle| {
                        &self
                            .anchor(handle)
                            .expect("active candidate anchor")
                            .reference
                    })
                })
                .filter(|first| {
                    citations.iter().all(|r| {
                        r.source_bytes == first.source_bytes
                            && r.source_fingerprint == first.source_fingerprint
                    })
                })
                .map(|first| {
                    let index = checks.len();
                    checks.push(first.clone());
                    index
                });
            choices.push((candidate, assessment, citations, check));
        }
        let fresh = match self.verify_publication(service, control, &checks) {
            Ok(verified) => {
                self.progress.provider_work.add(&verified.work);
                verified.sessions
            }
            Err(
                error
                @ (ApplicationError::Cancelled | ApplicationError::DiscoveryCallerUnavailable),
            ) => {
                return Err(error);
            }
            Err(_) => vec![None; checks.len()],
        };
        self.progress.publication_checks += usize::from(!checks.is_empty());
        let filter = self.filter().clone();
        let stream = &mut self.stream;
        for candidate in self
            .candidates
            .iter()
            .filter(|candidate| candidate.stale_context)
        {
            stream.queue_refresh(candidate);
        }
        for (candidate, assessment, citations, check) in choices {
            let current = check.and_then(|index| fresh[index].as_ref()).filter(|m| {
                m.identity == candidate.hit.metadata.identity
                    && filter.evaluate_observed(
                        m,
                        &candidate.hit.query_observation,
                        candidate.hit.content_complete,
                    ) == Truth::True
            });
            if current.is_none() && (check.is_some() || !citations.is_empty()) {
                self.progress.freshness_failures += 1;
                stream.queue_refresh(candidate);
                continue;
            }
            stream.released += 1;
            let state = assessment.disposition.verification_state();
            match state {
                CandidateVerificationState::Confirmed => stream.confirmed += 1,
                CandidateVerificationState::Unconfirmed => stream.unconfirmed += 1,
                CandidateVerificationState::Rejected => stream.rejected += 1,
            }
            let rows = match state {
                CandidateVerificationState::Confirmed => &mut reply.matches,
                CandidateVerificationState::Unconfirmed if assessment.plausible_alternative => {
                    &mut reply.alternatives
                }
                _ => continue,
            };
            let metadata = current.expect("a published judgment requires current source evidence");
            reply.replay_observations.insert(
                metadata.native_session_id.clone(),
                (
                    candidate.hit.query_observation.clone(),
                    candidate.hit.content_complete,
                ),
            );
            rows.push(ResolvedCandidate {
                metadata: metadata.clone(),
                assessment: assessment.clone(),
                citations,
            });
        }
        if !reply.replay_observations.is_empty() {
            reply.replay_filter = Some(filter);
        }
        stream.refreshing_batch = false;
        Ok(())
    }

    fn verify_publication(
        &self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        references: &[ContextReference],
    ) -> Result<EvidenceVerification, ApplicationError> {
        if references.is_empty() {
            return Ok(EvidenceVerification {
                sessions: Vec::new(),
                work: ProviderWork::default(),
            });
        }
        if let Some(verified) = service.verify_search_evidence(references, control)? {
            return Ok(verified);
        }
        // A read port without batch verification can still verify one reference
        // against an exact metadata observation and a bound source probe.
        let result = service.search(
            SearchParameters {
                structured: Some(StructuredQuery {
                    filter: QueryExpr::NativeId {
                        values: references
                            .iter()
                            .map(|r| r.native_session_id.clone())
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
        let sessions = references
            .iter()
            .map(|reference| {
                let Some(hit) = result.page.sessions.iter().find(|hit| {
                    hit.metadata.native_session_id == reference.native_session_id
                        && hit.metadata.identity.local_id == reference.local_id
                }) else {
                    return Ok(None);
                };
                match service.context_window(
                    reference.clone(),
                    selector(&hit.metadata),
                    ContextOptions {
                        direction: ContextDirection::Start,
                        max_bytes: 64,
                        max_messages: 1,
                        ..Default::default()
                    },
                    control.clone(),
                ) {
                    Ok(_) => Ok(Some(hit.metadata.clone())),
                    Err(
                        error @ (ApplicationError::DiscoveryCallerUnavailable
                        | ApplicationError::Cancelled),
                    ) => Err(error),
                    Err(_) => Ok(None),
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(EvidenceVerification {
            sessions,
            work: result.page.coverage.work.provider,
        })
    }

    fn verify_replayed_results(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        reply: &mut AdvanceResult,
    ) -> Result<(), ApplicationError> {
        let anchors: Vec<_> = reply
            .matches
            .iter()
            .chain(&reply.alternatives)
            .map(|r| r.citations[0].clone())
            .collect();
        if anchors.is_empty() {
            return Ok(());
        }
        let fresh = self.verify_publication(service, control, &anchors)?;
        let original_matches = reply.matches.len();
        let original_alternatives = reply.alternatives.len();
        let mut fresh = fresh.sessions.into_iter();
        let filter = reply.replay_filter.as_ref().expect("published query");
        let mut retain = |row: &mut ResolvedCandidate| {
            let id = &row.metadata.native_session_id;
            let (observation, content_complete) = &reply.replay_observations[id];
            let current = fresh.next().flatten().filter(|metadata| {
                metadata.identity == row.metadata.identity
                    && filter.evaluate_observed(metadata, observation, *content_complete)
                        == Truth::True
            });
            if let Some(metadata) = current {
                row.metadata = metadata;
                true
            } else {
                reply.invalidated.push(id.clone());
                false
            }
        };
        reply.matches.retain_mut(&mut retain);
        reply.alternatives.retain_mut(retain);
        let lost = anchors.len() - reply.matches.len() - reply.alternatives.len();
        if lost > 0 {
            self.progress.freshness_failures += lost;
            let stream = &mut self.stream;
            stream.confirmed -= original_matches - reply.matches.len();
            stream.unconfirmed -= original_alternatives - reply.alternatives.len();
            stream.verification_gap = Some("candidate_evidence_stale");
            if reply.outcome.is_some() {
                reply.outcome = Some(TerminalOutcome::Partial);
                stream.terminal = reply.outcome;
                self.progress.stop_reason = stream.verification_gap;
            }
            stream.previous.as_mut().expect("replayed operation").1 = reply.clone();
            self.refresh_assessment_counts();
        }
        Ok(())
    }

    fn terminal_outcome(&mut self, published: usize) -> TerminalOutcome {
        let stream = &mut self.stream;
        self.progress.stop_reason = self
            .progress
            .stop_reason
            .or(stream.verification_gap)
            .or((self.progress.unavailable_sources > 0).then_some("source_unavailable"));
        if !self.stream_coverage().candidate_evaluation_complete {
            TerminalOutcome::Partial
        } else if published > 0 {
            TerminalOutcome::CompletedWithMatches
        } else {
            TerminalOutcome::CompletedNoMatches
        }
    }
}

fn merge_windows(first: &ContextRequest, second: &ContextRequest) -> Option<ContextRequest> {
    let a = first.window.as_ref()?;
    let b = second.window.as_ref()?;
    if first.anchor != second.anchor
        || a.direction != ContextDirection::Around
        || b.direction != ContextDirection::Around
        || a.message_offset.is_some()
        || b.message_offset.is_some()
        || a.query.is_some()
        || b.query.is_some()
        || a.max_bytes + b.max_bytes > 4096
        || a.max_messages + b.max_messages > 5
    {
        return None;
    }
    Some(ContextRequest {
        anchor: first.anchor,
        window: Some(ContextOptions {
            before: a.before.max(b.before),
            after: a.after.max(b.after),
            max_bytes: a.max_bytes + b.max_bytes,
            max_messages: a.max_messages + b.max_messages,
            ..a.clone()
        }),
    })
}

/// DiscoveryPriority pages use native-ID keysets across inventory windows
/// and increasing offsets within each window. The active source cursor may
/// advance through decoder replay before the logical byte offset changes.
pub(super) fn cursor_advances(
    prior: Option<&SearchContinuation>,
    next: &SearchContinuation,
) -> bool {
    let boundary = |cursor: &SearchContinuation| {
        cursor
            .inventory_after
            .as_ref()
            .map(|key| key.native_session_id.clone())
    };
    let before_boundary = prior.and_then(boundary);
    let after_boundary = boundary(next);
    if after_boundary != before_boundary {
        return after_boundary > before_boundary;
    }
    let start = prior.map_or(0, |cursor| cursor.start);
    if next.start != start {
        return next.start > start;
    }
    let Some(after) = next.stream.as_ref() else {
        return false;
    };
    let position = |c: &TranscriptCheckpoint| {
        let replay = c.decoder_progress.as_ref();
        (
            c.byte_offset,
            c.chunk_offset,
            replay.map_or(0, |r| r.decoded),
            replay.map_or(0, |r| r.compressed),
        )
    };
    if let Some(before) = prior.and_then(|cursor| cursor.stream.as_ref()) {
        before.native_session_id == after.native_session_id
            && before.source_fingerprint == after.source_fingerprint
            && position(after) > position(before)
    } else {
        position(after) > (0, 0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint(offset: u64, replay: u64) -> TranscriptCheckpoint {
        TranscriptCheckpoint {
            native_session_id: "session".into(),
            source_fingerprint: "revision".into(),
            byte_offset: offset,
            decoder_progress: Some(DecoderProgress {
                decoded: replay,
                compressed: replay / 2,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn cursor_requires_forward_work_and_accepts_decoder_replay() {
        let previous = SearchContinuation {
            start: 4,
            stream: Some(checkpoint(150, 50)),
            ..Default::default()
        };
        assert!(!cursor_advances(Some(&previous), &previous));
        let advanced = SearchContinuation {
            stream: Some(checkpoint(150, 80)),
            ..previous.clone()
        };
        assert!(cursor_advances(Some(&previous), &advanced));
        assert!(!cursor_advances(Some(&advanced), &previous));
        let rolled = SearchContinuation {
            inventory_after: Some(InventoryBoundary {
                native_session_id: "later".into(),
                sort_value: None,
            }),
            ..Default::default()
        };
        assert!(cursor_advances(Some(&advanced), &rolled));
        assert!(!cursor_advances(Some(&rolled), &advanced));
    }

    #[test]
    fn a_reanchor_preserves_scope_and_only_restarts_the_unfinished_source() {
        let (intent, hard_filter, literal) = super::super::tests::example_intent().into_parts();
        let retrieval = Retrieval {
            hard_filter,
            literal,
            expansion: None,
            exclude_literal_on_expansion: false,
        };
        let parameters = retrieval.compile_parameters().unwrap();
        let mut run = Run::new("caller".into(), "instance", retrieval, parameters, intent);
        let cursor = SearchContinuation {
            scope: "frozen scope".into(),
            inventory_fingerprint: "window".into(),
            corpus_fingerprint: "cohort".into(),
            start: 4,
            stream: Some(checkpoint(150, 50)),
            ..Default::default()
        };
        run.next = Some(cursor.clone());
        let retry = run.reanchor_source().unwrap();
        assert_eq!(
            retry,
            SearchContinuation {
                stream: None,
                ..cursor.clone()
            }
        );
        assert!(run.reanchor_source().is_none());
        run.next.as_mut().unwrap().start += 1;
        assert!(run.reanchor_source().is_some());
    }

    #[test]
    fn overlapping_context_is_merged_only_within_the_same_bounded_anchor() {
        let request = |before, after| ContextRequest {
            anchor: 3,
            window: Some(ContextOptions {
                before,
                after,
                max_bytes: 1024,
                max_messages: 2,
                ..Default::default()
            }),
        };
        let merged = merge_windows(&request(2, 0), &request(0, 1)).unwrap();
        let window = merged.window.unwrap();
        assert_eq!(
            (
                window.before,
                window.after,
                window.max_messages,
                window.max_bytes
            ),
            (2, 1, 4, 2048)
        );
        let mut different = request(2, 0);
        different.anchor += 1;
        assert!(merge_windows(&different, &request(0, 1)).is_none());
    }
}
