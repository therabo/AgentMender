use super::*;

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
    pub finish_if_sufficient: bool,
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

#[derive(Clone, Default)]
pub struct AdvanceResult {
    pub matches: Vec<ResolvedCandidate>,
    pub alternatives: Vec<ResolvedCandidate>,
    pub windows: Vec<ResolutionWindow>,
    pub candidates: Vec<ResolutionCandidate>,
    pub outcome: Option<TerminalOutcome>,
    pub replayed: bool,
    pub invalidated: Vec<String>,
}

#[derive(Serialize)]
pub struct Coverage {
    inventory_complete: bool,
    physical_history_complete: bool,
    logical_history_complete: bool,
    eligible_evaluation_complete: bool,
    published: usize,
    released: usize,
    inventory_window_offset: usize,
    current_source_decoded_offset: Option<u64>,
}

#[derive(Clone, Default)]
pub(super) struct Stream {
    released: usize,
    pub(super) assessed: usize,
    pub(super) insufficient: usize,
    anchor_base: usize,
    prior_context_bytes: usize,
    published: usize,
    terminal: Option<TerminalOutcome>,
    previous: Option<(Advance, AdvanceResult)>,
    last_candidate: Option<(std::cmp::Reverse<usize>, String)>,
    inventory_boundary: Option<String>,
    reanchored: Option<(Option<InventoryBoundary>, usize)>,
}

impl Run {
    pub(super) fn enable_stream(&mut self) -> Result<(), ApplicationError> {
        // The legacy selector supports comparing several verified sessions and
        // passing an explicit preferred identity to finish.
        if self.intent.cardinality != AnswerCardinality::Multiple {
            return Err(ResolutionError::InvalidPreference.into());
        }
        self.parameters.page_size = CANDIDATE_BATCH;
        self.stream = Some(Stream::default());
        Ok(())
    }

    pub(super) fn can_reclaim(&self) -> bool {
        self.stream
            .as_ref()
            .is_none_or(|stream| stream.terminal.is_some())
    }

    pub(super) fn blocked(&mut self, reason: &'static str) -> ResolutionData {
        self.progress.stop_reason = Some(reason);
        self.stream.as_mut().expect("progressive run").terminal = Some(TerminalOutcome::Blocked);
        ResolutionData::Advanced(Box::new(AdvanceResult {
            outcome: Some(TerminalOutcome::Blocked),
            ..Default::default()
        }))
    }

    pub(super) fn presented(&self) -> usize {
        self.progress.candidates_presented - self.stream.as_ref().map_or(0, |s| s.released)
    }

    pub(super) fn anchor_base(&self) -> usize {
        self.stream.as_ref().map_or(0, |s| s.anchor_base)
    }

    pub(super) fn anchor(&self, handle: usize) -> Option<&Anchor> {
        self.anchors.get(handle.checked_sub(self.anchor_base())?)
    }

    pub(super) fn active_context_bytes(&self) -> usize {
        self.progress.context_bytes - self.stream.as_ref().map_or(0, |s| s.prior_context_bytes)
    }

    pub(super) fn observe_batch_order(&mut self, hits: &mut Vec<SessionSearchHit>) -> bool {
        let boundary = self
            .next
            .as_ref()
            .and_then(|cursor| cursor.inventory_after.as_ref())
            .map(|key| key.native_session_id.clone());
        let stream = self.stream.as_mut().expect("progressive run");
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
        let stream = self.stream.as_ref().expect("progressive run");
        let p = &self.progress;
        let single_pass = p.scan_passes == 1;
        let physical =
            p.scan_complete && single_pass && p.physically_traversed == p.sources_inspected;
        let logical = physical && !self.has_source_gaps();
        Coverage {
            inventory_complete: p.scan_complete,
            physical_history_complete: physical,
            logical_history_complete: logical,
            eligible_evaluation_complete: logical
                && p.assessments == p.sources_inspected
                && p.insufficient == 0
                && p.freshness_failures == 0,
            published: stream.published,
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
        let stream = self.stream.as_mut()?;
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
        let stream = self.stream.as_mut().expect("progressive run");
        stream.released = self.progress.candidates_presented;
        stream.assessed = self.progress.assessments;
        stream.insufficient = self.progress.insufficient;
        stream.anchor_base += self.anchors.len();
        stream.prior_context_bytes = self.progress.context_bytes;
        self.candidates.clear();
        self.anchors.clear();
        self.requested_windows.clear();
        self.publication = None;
    }

    pub(super) fn advance(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        id: String,
        step: Advance,
    ) -> Result<ResolutionResponse, ApplicationError> {
        let stream = self
            .stream
            .as_ref()
            .ok_or(ResolutionError::InvalidAdvance)?;
        if !bounded_text(&step.operation_id, 64)
            || step.assessments.len() > CANDIDATE_BATCH
            || step.context_requests.len() > WINDOW_BATCH
            || step.context_requests.iter().any(|request| {
                request
                    .window
                    .as_ref()
                    .is_some_and(|window| window.max_messages > 5 || window.max_bytes > 4096)
            })
            || step.cancel
                && (!step.assessments.is_empty()
                    || !step.context_requests.is_empty()
                    || !step.expansion.is_empty()
                    || step.finish_if_sufficient
                    || step.luna_unavailable)
            || step.luna_unavailable
                && (!step.context_requests.is_empty()
                    || !step.expansion.is_empty()
                    || step.finish_if_sufficient)
            || !step.context_requests.is_empty()
                && (!step.expansion.is_empty() || step.finish_if_sufficient)
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
        let stream = staged.stream.as_mut().expect("progressive run");
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
            self.release_batch();
            reply.outcome = Some(TerminalOutcome::Cancelled);
            return Ok(reply);
        }
        if !step.assessments.is_empty() {
            self.assess(step.assessments.clone())?;
        }
        if !step.context_requests.is_empty() {
            let mut requests: Vec<&ContextRequest> = Vec::new();
            for request in &step.context_requests {
                if !requests.contains(&request) {
                    requests.push(request);
                }
            }
            let merged = match requests.as_slice() {
                [first, second] => merge_windows(first, second),
                _ => None,
            };
            for request in merged.as_ref().map_or(requests, |merged| vec![merged]) {
                let candidate = self
                    .anchor(request.anchor)
                    .ok_or(ApplicationError::InvalidSearch)?
                    .candidate;
                reply.windows.push(self.context(
                    service,
                    control,
                    request.anchor,
                    request.window.clone(),
                )?);
                self.candidates[candidate].assessment = None;
            }
            self.refresh_assessment_counts();
            return Ok(reply);
        }
        if !step.expansion.is_empty() {
            // An empty first pass can safely be broadened without retaining
            // a corpus-sized seen set or publishing an identity twice.
            if self.progress.candidates_presented != 0 || step.finish_if_sufficient {
                return Err(ResolutionError::InvalidQueryExpansion.into());
            }
            let expansion = compile_expansion(&step.expansion)
                .map_err(|_| ResolutionError::InvalidQueryExpansion)?;
            if expansion
                .text_terms()
                .iter()
                .any(|term| self.intent.literal.text_terms().contains(term))
            {
                return Err(ResolutionError::InvalidQueryExpansion.into());
            }
            let data = self.expand(service, control, expansion)?;
            let ResolutionData::Candidates { candidates, .. } = data else {
                unreachable!()
            };
            reply.candidates = candidates;
            return Ok(reply);
        }
        let needs_assessment = self
            .candidates
            .iter()
            .any(|candidate| candidate.assessment.is_none());
        let early_finish = step.finish_if_sufficient
            && self
                .search_intent
                .as_ref()
                .is_some_and(|plan| plan.coverage_goal == CoverageGoal::Standard);
        if needs_assessment && !step.luna_unavailable {
            if step.assessments.is_empty() {
                return Err(ResolutionError::AssessmentRequired.into());
            }
            reply.candidates = self
                .candidate_batch(0, self.presented())
                .into_iter()
                .zip(&self.candidates)
                .filter_map(|(view, candidate)| candidate.assessment.is_none().then_some(view))
                .collect();
            return Ok(reply);
        }
        if !self.candidates.is_empty() {
            self.publish_batch(service, control, &mut reply)?;
            let stream = self.stream.as_mut().expect("progressive run");
            stream.published += reply.matches.len();
            self.release_batch();
        }
        let published = self.stream.as_ref().expect("progressive run").published;
        if step.luna_unavailable {
            self.progress.stop_reason = Some("luna_unavailable");
        }
        if early_finish && published > 0 && !self.progress.scan_complete {
            self.progress.stop_reason = Some("sufficient_evidence");
        }
        if self.progress.scan_complete || self.progress.stop_reason.is_some() {
            reply.outcome = Some(self.terminal_outcome(published));
            return Ok(reply);
        }
        match self.scan(service, control, false) {
            Ok(ResolutionData::Candidates { candidates, .. }) => reply.candidates = candidates,
            Ok(_) => unreachable!(),
            Err(ApplicationError::Cancelled) => return Err(ApplicationError::Cancelled),
            Err(error) => {
                self.progress.stop_reason = Some(error.public_code());
                reply.outcome = Some(if published > 0 {
                    TerminalOutcome::Partial
                } else {
                    TerminalOutcome::Blocked
                });
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

    fn publish_batch(
        &mut self,
        service: &SessionSearchService<'_>,
        control: &QueryControl,
        reply: &mut AdvanceResult,
    ) -> Result<(), ApplicationError> {
        let choices: Vec<_> = self
            .candidates
            .iter()
            .filter_map(|candidate| {
                let assessment = candidate.assessment.as_ref()?;
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
                let first = citations.first()?;
                citations
                    .iter()
                    .all(|r| {
                        r.source_bytes == first.source_bytes
                            && r.source_fingerprint == first.source_fingerprint
                    })
                    .then_some((candidate, assessment, citations))
            })
            .collect();
        if choices.is_empty() {
            return Ok(());
        }
        let anchors: Vec<_> = choices.iter().map(|(_, _, refs)| refs[0].clone()).collect();
        self.progress.publication_checks += 1;
        let Some(verified) = service.verify_search_evidence(&anchors, control)? else {
            // Ports without a batch verifier retain the original guarded route.
            let ResolutionData::Finished {
                matches,
                alternatives,
                ..
            } = self.finish(service, control, 0, None, false, None)?
            else {
                unreachable!()
            };
            reply.matches = matches;
            reply.alternatives = alternatives;
            return Ok(());
        };
        self.progress.provider_work.add(&verified.work);
        for ((candidate, assessment, citations), metadata) in
            choices.into_iter().zip(verified.sessions)
        {
            let current = metadata.filter(|m| {
                m.identity == candidate.hit.metadata.identity
                    && self.filter().evaluate_observed(
                        m,
                        &candidate.hit.query_observation,
                        candidate.hit.content_complete,
                    ) == Truth::True
            });
            let Some(metadata) = current else {
                self.progress.freshness_failures += 1;
                continue;
            };
            let row = ResolvedCandidate {
                metadata,
                assessment: assessment.clone(),
                citations,
            };
            match assessment.disposition {
                Disposition::VerifiedRelevant => reply.matches.push(row),
                Disposition::Related | Disposition::Contradiction => reply.alternatives.push(row),
                Disposition::Insufficient => {}
            }
        }
        Ok(())
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
        let Some(fresh) = service.verify_search_evidence(&anchors, control)? else {
            return Err(ApplicationError::StaleSearchEvidence);
        };
        let original_matches = reply.matches.len();
        let mut fresh = fresh.sessions.into_iter();
        let mut retain = |row: &ResolvedCandidate| {
            let valid = fresh.next().flatten().as_ref() == Some(&row.metadata);
            if !valid {
                reply
                    .invalidated
                    .push(row.metadata.native_session_id.clone());
            }
            valid
        };
        reply.matches.retain(&mut retain);
        reply.alternatives.retain(retain);
        let lost = anchors.len() - reply.matches.len() - reply.alternatives.len();
        if lost > 0 {
            self.progress.freshness_failures += lost;
            let stream = self.stream.as_mut().expect("progressive run");
            stream.published -= original_matches - reply.matches.len();
            if reply.outcome.is_some() {
                reply.outcome = Some(TerminalOutcome::Partial);
                stream.terminal = reply.outcome;
            }
            stream.previous.as_mut().expect("replayed operation").1 = reply.clone();
        }
        Ok(())
    }

    fn terminal_outcome(&self, published: usize) -> TerminalOutcome {
        let p = &self.progress;
        let assessed_inventory = self.stream_coverage().eligible_evaluation_complete;
        let exhaustive = self
            .search_intent
            .as_ref()
            .is_some_and(|p| p.coverage_goal == CoverageGoal::Exhaustive);
        if self.has_source_gaps()
            || p.freshness_failures > 0
            || p.insufficient > 0
            || p.assessments < p.lexical_candidates
            || p.stop_reason
                .is_some_and(|reason| reason != "sufficient_evidence")
            || exhaustive && !assessed_inventory
            || published == 0 && !assessed_inventory
        {
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
        let intent: Intent = serde_json::from_value(serde_json::json!({
            "question":"Find matching discussions","cardinality":"multiple",
            "literal":{"op":"text","value":"Cesium"}
        }))
        .unwrap();
        let parameters = intent.parameters().unwrap();
        let mut run = Run::new("caller".into(), "instance", intent, parameters, None);
        run.enable_stream().unwrap();
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
