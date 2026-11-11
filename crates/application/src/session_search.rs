//! Read-only search orchestration. Caller identity is fixed by trusted composition.
use crate::{ApplicationError, QueryControl, TargetSelector};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::{collections::BTreeSet, path::Path};
fn reader_error(error: ProviderError) -> ApplicationError {
    match error {
        ProviderError::InvalidScope(ref reason) if reason == "stale_search_evidence" => {
            ApplicationError::StaleSearchEvidence
        }
        ProviderError::InvalidScope(ref reason) if reason == "discovery_caller_unavailable" => {
            ApplicationError::DiscoveryCallerUnavailable
        }
        _ => error.into(),
    }
}

pub struct SearchResult {
    pub parameters: SearchParameters,
    pub page: SessionSearchPage,
    pub binding_revision: String,
    pub registration_instance: String,
    /// Whether the requested coverage policy is established by this observation.
    /// Best-effort acceptance is not a claim of global corpus completeness.
    pub coverage_requirement_met: bool,
}
pub struct ContextWindowResult {
    pub window: ContextWindow,
    pub reference: ContextReference,
    pub options: ContextOptions,
}
impl ContextWindowResult {
    pub fn next_options(&self) -> Option<ContextOptions> {
        let next = self.window.next.as_ref()?;
        let chunk = next
            .position
            .as_ref()
            .is_some_and(|p| p.chunk_offset > 0 && p.chunk_offset < p.chunk_total);
        let direction = if next.decoder_progress.is_some() {
            self.options.direction
        } else if chunk {
            ContextDirection::Around
        } else if self.options.direction == ContextDirection::Occurrences {
            ContextDirection::Occurrences
        } else {
            ContextDirection::Forward
        };
        let mut options = ContextOptions {
            direction,
            message_offset: None,
            ..self.options.clone()
        };
        if next.position.as_ref().is_some_and(|p| p.chunk_offset > 0) {
            options.before = 0;
            options.after = 0;
        }
        Some(options)
    }
}
pub struct ContextResult {
    pub evidence: ConversationEvidence,
    pub purpose: SearchPurpose,
    pub reference: ContextReference,
}
pub struct SessionSearchService<'a> {
    pub reader: &'a dyn SessionSearchReadPort,
    pub provider_root: &'a Path,
    pub provider: ProviderKind,
    pub management: Option<&'a ManagementIdentity>,
    pub apparent_caller: Option<&'a ApparentCodexCaller>,
    pub registered: &'a [ArtifactId],
    pub instance: &'a str,
}
impl SessionSearchService<'_> {
    pub(crate) fn binding(&self, purpose: SearchPurpose) -> Result<String, ApplicationError> {
        match purpose {
            SearchPurpose::Diagnostic => return Ok("diagnostic_no_caller_guarantee".into()),
            SearchPurpose::Discovery => {
                return Ok(discovery_binding(
                    &self.discovery_caller()?.identity(self.provider_root),
                ));
            }
            SearchPurpose::Management => {}
        }
        let required = || ApplicationError::CallerIdentityRequired {
            instance: self.instance.into(),
        };
        let identity = self.management.ok_or_else(required)?;
        if identity.identity.provider != self.provider
            || identity.identity.class != GraphNodeKind::Session
            || identity.identity.state_root != self.provider_root
            || identity.native_session_id.is_empty()
            || identity.revision.is_empty()
        {
            return Err(required());
        }
        Ok(identity.revision.clone())
    }
    fn discovery_caller(&self) -> Result<&ApparentCodexCaller, ApplicationError> {
        self.apparent_caller
            .filter(|_| self.provider == ProviderKind::Codex)
            .ok_or(ApplicationError::DiscoveryCallerUnavailable)
    }
    fn excluded(&self, purpose: SearchPurpose) -> Result<Vec<ArtifactId>, ApplicationError> {
        let mut ids = self.registered.to_vec();
        if let Some(binding) = self.management {
            ids.push(binding.identity.clone());
        }
        if purpose == SearchPurpose::Discovery {
            ids.push(self.discovery_caller()?.identity(self.provider_root));
        }
        ids.sort();
        ids.dedup();
        if ids.len() > 64 {
            return Err(ApplicationError::InvalidSearch);
        }
        Ok(ids)
    }
    pub fn search(
        &self,
        parameters: SearchParameters,
        continuation: Option<SearchContinuation>,
        control: QueryControl,
    ) -> Result<SearchResult, ApplicationError> {
        control.check()?;
        // No provider access, candidate counts or excerpts before this gate.
        let revision = self.binding(parameters.purpose)?;
        if !parameters.valid() {
            return Err(ApplicationError::InvalidSearch);
        }
        let excluded = self.excluded(parameters.purpose)?;
        let scope = search_scope(&parameters, self.provider_root, &excluded, &revision);
        let mut continuation = continuation.unwrap_or_default();
        if !continuation.scope.is_empty() && continuation.scope != scope {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        continuation.scope = scope.clone();
        let request = SearchReadRequest {
            cached: vec![],
            parameters: parameters.clone(),
            excluded: excluded.clone(),
            management_session: (parameters.purpose == SearchPurpose::Management)
                .then(|| self.management.expect("checked").native_session_id.clone()),
            discovery_session: (parameters.purpose == SearchPurpose::Discovery).then(|| {
                self.apparent_caller
                    .expect("checked")
                    .native_id()
                    .to_owned()
            }),
            continuation,
        };
        let mut page = self
            .reader
            .search_sessions(&request)
            .map_err(reader_error)?;
        control.check()?;
        // Validate deterministic conditions after retrieval/ranking. A model or
        // faulty adapter cannot reintroduce a filtered, foreign, unknown or
        // duplicate identity. Text proof comes from the trusted read adapter's
        // bounded aggregate, not from an LLM's interpretation of excerpts.
        let mut returned = BTreeSet::new();
        let terms = parameters.terms();
        let mut valid_hit = |h: &SessionSearchHit, expected: Truth| {
            let structured_truth = parameters.structured.as_ref().map_or(Truth::True, |query| {
                if !query.filter.observation_valid(&h.query_observation) {
                    return Truth::False;
                }
                query.filter.evaluate_observed(
                    &h.metadata,
                    &h.query_observation,
                    h.content_complete,
                )
            });
            let legacy_truth = if terms.iter().all(|term| h.matched_terms.contains(term)) {
                Truth::True
            } else if parameters.reads_content() && !h.content_complete {
                Truth::Unknown
            } else {
                Truth::False
            };
            returned.insert(h.metadata.identity.clone())
                && !excluded.contains(&h.metadata.identity)
                && h.metadata.identity.provider == self.provider
                && h.metadata.identity.state_root == self.provider_root
                && h.metadata.identity.class == GraphNodeKind::Session
                && (parameters.include_archived || h.metadata.state == SessionState::Active)
                && parameters
                    .project_label
                    .as_ref()
                    .is_none_or(|label| h.metadata.project_label.as_ref() == Some(label))
                && legacy_truth.and(structured_truth) == expected
        };
        if page.sessions.len().saturating_add(page.unresolved.len()) > parameters.page_size
            || page.sessions.iter().any(|h| !valid_hit(h, Truth::True))
            || page
                .unresolved
                .iter()
                .any(|h| !valid_hit(h, Truth::Unknown))
            || page.coverage.sessions_inspected > page.coverage.eligible_observed
            || page.coverage.unresolved_sessions > page.coverage.sessions_inspected
            || page.unresolved.len() > page.coverage.unresolved_sessions
            || page.coverage.fully_traversed_sessions > page.coverage.sessions_inspected
        {
            return Err(ApplicationError::InvalidSearch);
        }
        let requires_complete = parameters
            .structured
            .as_ref()
            .is_some_and(|q| q.coverage == CoverageRequirement::Complete);
        let coverage_requirement_met = !requires_complete
            || (page.next.is_none()
                && page.coverage.sessions_inspected == page.coverage.eligible_observed
                && page.coverage.unresolved_sessions == 0
                && page.coverage.unsupported_sessions == 0
                && page.coverage.unreadable_sessions == 0
                && page.coverage.truncated_sessions == 0
                && page.coverage.suppressed_records == 0
                && (!parameters.reads_content()
                    || page.coverage.fully_traversed_sessions == page.coverage.eligible_observed)
                && !page
                    .coverage
                    .reasons
                    .iter()
                    .any(|reason| reason == "metadata_inventory_limit"));
        if let Some(next) = page.next.as_mut() {
            next.scope = scope;
        }
        Ok(SearchResult {
            parameters,
            page,
            binding_revision: revision,
            registration_instance: self.instance.into(),
            coverage_requirement_met,
        })
    }
    pub fn context_window(
        &self,
        reference: ContextReference,
        target: TargetSelector,
        options: ContextOptions,
        control: QueryControl,
    ) -> Result<ContextWindowResult, ApplicationError> {
        control.check()?;
        if self.binding(reference.purpose)? != reference.binding_revision {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        if !options.valid() {
            return Err(ApplicationError::InvalidSearch);
        }
        let id = ArtifactId {
            provider: self.provider,
            state_root: self.provider_root.into(),
            class: GraphNodeKind::Session,
            local_id: reference.local_id.clone(),
        };
        let excluded = self.excluded(reference.purpose)?;
        if excluded.contains(&id) {
            return Err(ApplicationError::ProtectedManagementSession);
        }
        if !target(&id) {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        let window = self
            .reader
            .session_context_window(&reference, &options, &excluded)
            .map_err(reader_error)?;
        control.check()?;
        let same_source = |r: &ContextReference| {
            r.local_id == reference.local_id
                && r.native_session_id == reference.native_session_id
                && r.source_fingerprint == reference.source_fingerprint
                && r.source_bytes == reference.source_bytes
                && r.purpose == reference.purpose
                && r.binding_revision == reference.binding_revision
        };
        if window.messages.len() > options.max_messages
            || window.messages.iter().any(|e| {
                e.source_fingerprint != reference.source_fingerprint
                    || e.source_bytes != reference.source_bytes
                    || e.excerpt
                        .as_ref()
                        .is_some_and(|text| !safe_conversation_text(text))
            })
            || window
                .messages
                .iter()
                .filter_map(|e| e.excerpt.as_ref())
                .map(String::len)
                .sum::<usize>()
                > options.max_bytes
            || window.previous.as_ref().is_some_and(|r| !same_source(r))
            || window.next.as_ref().is_some_and(|r| !same_source(r))
        {
            return Err(ApplicationError::InvalidSearch);
        }
        Ok(ContextWindowResult {
            window,
            reference,
            options,
        })
    }
    pub fn context(
        &self,
        reference: ContextReference,
        target: TargetSelector,
        control: QueryControl,
    ) -> Result<ContextResult, ApplicationError> {
        control.check()?;
        if self.binding(reference.purpose)? != reference.binding_revision {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        let id = ArtifactId {
            provider: self.provider,
            state_root: self.provider_root.into(),
            class: GraphNodeKind::Session,
            local_id: reference.local_id.clone(),
        };
        let excluded = self.excluded(reference.purpose)?;
        if excluded.contains(&id) {
            return Err(ApplicationError::ProtectedManagementSession);
        }
        if !target(&id) {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        let evidence = self
            .reader
            .session_context(&reference, &excluded)
            .map_err(reader_error)?;
        control.check()?;
        if evidence.source_fingerprint != reference.source_fingerprint
            || evidence.source_bytes != reference.source_bytes
            || evidence
                .excerpt
                .as_ref()
                .is_some_and(|text| text.chars().count() > 512 || !safe_conversation_text(text))
        {
            return Err(ApplicationError::InvalidSearch);
        }
        Ok(ContextResult {
            evidence,
            purpose: reference.purpose,
            reference,
        })
    }
}
