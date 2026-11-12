//! Read-only ports and shared bounded observation helpers. No mutation port exists.
pub mod desktop_process;
pub mod evidence;
pub mod filesystem;
pub mod graph;
#[cfg(target_os = "macos")]
mod macos_sandbox;
pub mod process;

use agentmender_domain::{
    ArtifactGraph, CohortPage, CohortQueue, CohortQueuePage, CohortRead, CohortScope,
    DiscoveryScope, DiscoverySnapshot, GraphError, IntegrityContext, PlanningContext, Provider,
    ProviderKind, observed_path_is_absolute,
};
use std::path::PathBuf;
use thiserror::Error;

/// Separate, read-only scoped cleanup observations; this port carries no authority.
pub struct ScopedCleanupObservation {
    pub graph: ArtifactGraph,
    pub context: PlanningContext,
}
pub trait ScopedCleanupReadPort {
    fn observe_cleanup(
        &self,
        target: &str,
        scope: &DiscoveryScope,
    ) -> Result<ScopedCleanupObservation, ProviderError>;
}

/// Opt-in labels from a bounded metadata source, never transcript contents.
pub trait SessionMetadataReadPort {
    fn session_metadata(&self) -> Result<agentmender_domain::SessionInventory, ProviderError>;
}

/// Bounded conversation evidence, not transcript dumps or mutation authority.
pub trait SessionSearchReadPort {
    fn search_sessions(
        &self,
        request: &agentmender_domain::SearchReadRequest,
    ) -> Result<agentmender_domain::SessionSearchPage, ProviderError>;
    fn capture_cohort(
        &self,
        _scope: &CohortScope,
        _parameters: &agentmender_domain::SearchParameters,
    ) -> Result<agentmender_domain::ProviderWork, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "conversation cohort unavailable".into(),
        ))
    }
    fn read_cohort(&self, _read: &CohortRead) -> Result<CohortPage, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "conversation cohort unavailable".into(),
        ))
    }
    fn cohort_queue(
        &self,
        _scope: &CohortScope,
        _queue: CohortQueue,
    ) -> Result<CohortQueuePage, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "conversation cohort unavailable".into(),
        ))
    }
    fn release_cohort(&self, _id: &str) {}
    fn open_session_window(
        &self,
        _reference: &agentmender_domain::ContextReference,
        _options: &agentmender_domain::ContextOptions,
        _excluded: &[agentmender_domain::ArtifactId],
    ) -> Result<
        (
            agentmender_domain::ContextReference,
            agentmender_domain::ContextWindow,
        ),
        ProviderError,
    > {
        Err(ProviderError::CapabilityUnavailable(
            "UUID context unavailable".into(),
        ))
    }
    /// Fresh metadata and source revision for each anchor in a bounded batch.
    /// A missing entry denotes a stale or unreadable source. None for the
    /// entire batch means the adapter uses the existing single-source path.
    fn verify_search_evidence(
        &self,
        _references: &[agentmender_domain::ContextReference],
        _excluded: &[agentmender_domain::ArtifactId],
    ) -> Result<Option<agentmender_domain::EvidenceVerification>, ProviderError> {
        Ok(None)
    }
    fn transcript_page(
        &self,
        _reference: &agentmender_domain::ContextReference,
        _checkpoint: Option<agentmender_domain::TranscriptCheckpoint>,
        _excluded: &[agentmender_domain::ArtifactId],
    ) -> Result<agentmender_domain::TranscriptPage, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "transcript paging unavailable".into(),
        ))
    }
    fn session_context_window(
        &self,
        _reference: &agentmender_domain::ContextReference,
        _options: &agentmender_domain::ContextOptions,
        _excluded: &[agentmender_domain::ArtifactId],
    ) -> Result<agentmender_domain::ContextWindow, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "context navigation unavailable".into(),
        ))
    }
    fn session_context(
        &self,
        reference: &agentmender_domain::ContextReference,
        excluded: &[agentmender_domain::ArtifactId],
    ) -> Result<agentmender_domain::ConversationEvidence, ProviderError>;
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error(transparent)]
    Graph(#[from] GraphError),
    #[error("provider not installed: {0}")]
    ProviderNotInstalled(ProviderKind),
    #[error("provider version unknown: {0}")]
    ProviderVersionUnknown(ProviderKind),
    #[error("unsupported provider: {0}")]
    ProviderUnsupported(ProviderKind),
    #[error("capability unavailable: {0}")]
    CapabilityUnavailable(String),
    #[error("malformed provider state at {path}: {detail}")]
    MalformedProviderState { path: PathBuf, detail: String },
    #[error("permission denied at {0}")]
    PermissionDenied(PathBuf),
    #[error("discovery failed at {path}: {source}")]
    DiscoveryFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid discovery scope: {0}")]
    InvalidScope(String),
}

pub trait ProviderAdapter {
    fn kind(&self) -> ProviderKind;
    fn detect(&self) -> Result<Provider, ProviderError>;
    fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError>;
    /// Pure interpretation of this adapter's already collected snapshot.
    /// Implementations must not perform I/O or sample the current time.
    fn build_graph(&self, _: &DiscoverySnapshot) -> Result<ArtifactGraph, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "evidence graph interpretation unavailable".into(),
        ))
    }
    /// Pure interpretation of lifecycle protocol evidence; no I/O or clock sampling.
    fn planning_context(
        &self,
        _: &ArtifactGraph,
        scope: &DiscoveryScope,
    ) -> Result<PlanningContext, ProviderError> {
        Ok(PlanningContext {
            scope: scope.clone(),
            capabilities: Vec::new(),
            complete_removal_evidence: None,
            cleanup_evidence: None,
            effects: None,
        })
    }
    /// Pure interpretation of the existing graph; no I/O or clock sampling is permitted.
    /// Current production adapters have no closed integrity proof contracts.
    fn integrity_context(&self, _: &ArtifactGraph) -> Result<IntegrityContext, ProviderError> {
        Ok(IntegrityContext::default())
    }
}

#[derive(Debug, Clone)]
pub struct ProviderLocation {
    pub state_root: PathBuf,
    pub executable: Option<PathBuf>,
}

pub fn validate_scope(scope: &DiscoveryScope) -> Result<(), ProviderError> {
    if !(1..=100_000).contains(&scope.max_entries) {
        return Err(ProviderError::InvalidScope(
            "max_entries must be 1..=100000".into(),
        ));
    }
    if let Some(path) = &scope.project_path
        && (!observed_path_is_absolute(path)
            || path.as_os_str().as_encoded_bytes().len() > 4096
            || path
                .to_str()
                .is_none_or(|s| s.chars().any(char::is_control)))
    {
        return Err(ProviderError::InvalidScope(
            "project path must be absolute UTF-8, without controls, at most 4096 bytes".into(),
        ));
    }
    Ok(())
}

pub fn validate_location(location: &ProviderLocation) -> Result<(), ProviderError> {
    if !location.state_root.is_absolute()
        || location
            .executable
            .as_ref()
            .is_some_and(|p| !p.is_absolute())
    {
        return Err(ProviderError::InvalidScope(
            "provider state root and executable must be absolute".into(),
        ));
    }
    Ok(())
}

pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> ProviderError {
    let path = path.into();
    if source.kind() == std::io::ErrorKind::PermissionDenied {
        ProviderError::PermissionDenied(path)
    } else {
        ProviderError::DiscoveryFailed { path, source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_and_oversized_project_scopes_are_rejected() {
        for project in [
            PathBuf::from("relative"),
            PathBuf::from(format!("/{}", "x".repeat(4096))),
        ] {
            assert!(
                validate_scope(&DiscoveryScope {
                    project_path: Some(project),
                    ..Default::default()
                })
                .is_err()
            );
        }
    }
    #[test]
    fn relative_live_provider_locations_are_rejected() {
        assert!(
            validate_location(&ProviderLocation {
                state_root: "relative".into(),
                executable: None
            })
            .is_err()
        );
    }
}
