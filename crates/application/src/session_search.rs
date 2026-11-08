//! Read-only search orchestration. Caller identity is fixed by trusted composition.
use crate::{ApplicationError, QueryControl, TargetSelector};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::path::Path;
fn reader_error(error: ProviderError) -> ApplicationError {
    match error {
        ProviderError::InvalidScope(ref reason) if reason == "stale_search_evidence" => {
            ApplicationError::StaleSearchEvidence
        }
        _ => error.into(),
    }
}

pub struct SearchResult {
    pub parameters: SearchParameters,
    pub page: SessionSearchPage,
    pub binding_revision: String,
    pub registration_instance: String,
}
pub struct ContextResult {
    pub evidence: ConversationEvidence,
    pub purpose: SearchPurpose,
}
pub struct SessionSearchService<'a> {
    pub reader: &'a dyn SessionSearchReadPort,
    pub provider_root: &'a Path,
    pub provider: ProviderKind,
    pub management: Option<&'a ManagementIdentity>,
    pub registered: &'a [ArtifactId],
    pub instance: &'a str,
}
impl SessionSearchService<'_> {
    fn binding(&self, purpose: SearchPurpose) -> Result<String, ApplicationError> {
        if purpose == SearchPurpose::Diagnostic {
            return Ok("diagnostic_no_caller_guarantee".into());
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
    fn excluded(&self) -> Result<Vec<ArtifactId>, ApplicationError> {
        let mut ids = self.registered.to_vec();
        if let Some(binding) = self.management {
            ids.push(binding.identity.clone());
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
        let excluded = self.excluded()?;
        let scope = search_scope(&parameters, self.provider_root, &excluded, &revision);
        let mut continuation = continuation.unwrap_or_default();
        if !continuation.scope.is_empty() && continuation.scope != scope {
            return Err(ApplicationError::StaleSearchEvidence);
        }
        continuation.scope = scope.clone();
        let request = SearchReadRequest {
            parameters: parameters.clone(),
            excluded: excluded.clone(),
            management_session: (parameters.purpose == SearchPurpose::Management)
                .then(|| self.management.expect("checked").native_session_id.clone()),
            continuation,
        };
        let mut page = self
            .reader
            .search_sessions(&request)
            .map_err(reader_error)?;
        control.check()?;
        // A faulty adapter cannot silently return excluded or foreign candidates.
        if page.sessions.len() > parameters.page_size
            || page.sessions.iter().any(|h| {
                excluded.contains(&h.metadata.identity)
                    || h.metadata.identity.provider != self.provider
                    || h.metadata.identity.state_root != self.provider_root
                    || h.metadata.identity.class != GraphNodeKind::Session
            })
        {
            return Err(ApplicationError::InvalidSearch);
        }
        if let Some(next) = page.next.as_mut() {
            next.scope = scope;
        }
        Ok(SearchResult {
            parameters,
            page,
            binding_revision: revision,
            registration_instance: self.instance.into(),
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
        let excluded = self.excluded()?;
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
        Ok(ContextResult {
            evidence,
            purpose: reference.purpose,
        })
    }
}
