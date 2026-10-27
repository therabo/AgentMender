//! Query use cases shared by CLI and a future read-only MCP interface.
use agentmender_domain::*;
use agentmender_provider_api::{ProviderAdapter, ProviderError, validate_scope};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("unknown entity: {0}")]
    UnknownEntity(EntityId),
    #[error("ambiguous entity: {0}")]
    AmbiguousEntity(EntityId),
    #[error("inventory incomplete; absence of {0} cannot be established")]
    InspectionUnavailable(EntityId),
}

#[derive(Debug, Serialize)]
pub struct ProviderFailure {
    pub provider: ProviderKind,
    pub code: DiagnosticCode,
    pub message: String,
}

fn failure(provider: ProviderKind, error: ProviderError) -> ProviderFailure {
    let code = match &error {
        ProviderError::ProviderNotInstalled(_) => DiagnosticCode::ProviderNotInstalled,
        ProviderError::ProviderVersionUnknown(_) => DiagnosticCode::ProviderVersionUnknown,
        ProviderError::ProviderUnsupported(_) => DiagnosticCode::ProviderUnsupported,
        ProviderError::CapabilityUnavailable(_) => DiagnosticCode::CapabilityUnavailable,
        ProviderError::MalformedProviderState { .. } => DiagnosticCode::MalformedProviderState,
        ProviderError::PermissionDenied(_) => DiagnosticCode::PermissionDenied,
        ProviderError::DiscoveryFailed { .. } | ProviderError::InvalidScope(_) => {
            DiagnosticCode::IncompleteDiscovery
        }
    };
    ProviderFailure {
        provider,
        code,
        message: error.to_string(),
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ProviderReport {
    pub providers: Vec<Provider>,
    pub failures: Vec<ProviderFailure>,
}
#[derive(Debug, Default, Serialize)]
pub struct ScanReport {
    pub snapshots: Vec<DiscoverySnapshot>,
    pub failures: Vec<ProviderFailure>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "entity", rename_all = "snake_case")]
pub enum InspectedEntity {
    Workspace(Workspace),
    Project(Project),
    Session(Session),
    Artifact(Artifact),
}
#[derive(Debug, Serialize)]
pub struct InspectReport {
    pub entity: InspectedEntity,
    pub provider: ProviderKind,
    pub completeness: Completeness,
    pub diagnostics: Vec<Diagnostic>,
    pub failures: Vec<ProviderFailure>,
}

/// Composition supplies adapters; application never imports their concrete crates.
pub struct DiscoveryService<'a> {
    adapters: Vec<&'a dyn ProviderAdapter>,
}
impl<'a> DiscoveryService<'a> {
    pub fn new(adapters: Vec<&'a dyn ProviderAdapter>) -> Self {
        Self { adapters }
    }
    pub fn providers(&self) -> ProviderReport {
        let mut report = ProviderReport::default();
        for adapter in &self.adapters {
            match adapter.detect() {
                Ok(provider) => report.providers.push(provider),
                Err(error) => report.failures.push(failure(adapter.kind(), error)),
            }
        }
        report.providers.sort_by_key(|provider| provider.kind);
        report.failures.sort_by_key(|failure| failure.provider);
        report
    }
    pub fn scan(&self, scope: &DiscoveryScope) -> Result<ScanReport, ApplicationError> {
        validate_scope(scope)?;
        let mut report = ScanReport::default();
        for adapter in &self.adapters {
            match adapter.discover(scope) {
                Ok(mut snapshot) => {
                    snapshot.normalize();
                    report.snapshots.push(snapshot);
                }
                Err(error) => report.failures.push(failure(adapter.kind(), error)),
            }
        }
        report
            .snapshots
            .sort_by_key(|snapshot| snapshot.provider.kind);
        report.failures.sort_by_key(|failure| failure.provider);
        Ok(report)
    }
    pub fn inspect(
        &self,
        id: &EntityId,
        scope: &DiscoveryScope,
    ) -> Result<InspectReport, ApplicationError> {
        let report = self.scan(scope)?;
        let unavailable = !report.failures.is_empty()
            || report
                .snapshots
                .iter()
                .any(|s| s.completeness != Completeness::Complete);
        let mut matches = Vec::new();
        for snapshot in report.snapshots {
            for entity in snapshot
                .workspaces
                .iter()
                .filter(|e| &e.id == id)
                .cloned()
                .map(InspectedEntity::Workspace)
                .chain(
                    snapshot
                        .projects
                        .iter()
                        .filter(|e| &e.id == id)
                        .cloned()
                        .map(InspectedEntity::Project),
                )
                .chain(
                    snapshot
                        .sessions
                        .iter()
                        .filter(|e| &e.id == id)
                        .cloned()
                        .map(InspectedEntity::Session),
                )
                .chain(
                    snapshot
                        .artifacts
                        .iter()
                        .filter(|e| &e.id == id)
                        .cloned()
                        .map(InspectedEntity::Artifact),
                )
            {
                matches.push(InspectReport {
                    entity,
                    provider: snapshot.provider.kind,
                    completeness: snapshot.completeness,
                    diagnostics: snapshot.diagnostics.clone(),
                    failures: Vec::new(),
                });
            }
        }
        if matches.len() > 1 {
            return Err(ApplicationError::AmbiguousEntity(id.clone()));
        }
        if let Some(mut found) = matches.pop() {
            found.failures = report.failures;
            Ok(found)
        } else if unavailable {
            Err(ApplicationError::InspectionUnavailable(id.clone()))
        } else {
            Err(ApplicationError::UnknownEntity(id.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FailingAdapter;
    impl ProviderAdapter for FailingAdapter {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Codex
        }
        fn detect(&self) -> Result<Provider, ProviderError> {
            Err(ProviderError::ProviderVersionUnknown(self.kind()))
        }
        fn discover(&self, _: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
            Err(ProviderError::CapabilityUnavailable(
                "fixture failure".into(),
            ))
        }
    }
    #[test]
    fn failures_remain_structured_and_visible() {
        let adapter = FailingAdapter;
        let service = DiscoveryService::new(vec![&adapter]);
        assert_eq!(
            service.providers().failures[0].code,
            DiagnosticCode::ProviderVersionUnknown
        );
        let report = service.scan(&DiscoveryScope::default()).unwrap();
        assert!(report.snapshots.is_empty());
        assert_eq!(
            report.failures[0].code,
            DiagnosticCode::CapabilityUnavailable
        );
    }
    #[test]
    fn invalid_budgets_are_rejected_before_adapter_calls() {
        let service = DiscoveryService::new(vec![]);
        assert!(
            service
                .scan(&DiscoveryScope {
                    max_entries: 0,
                    ..Default::default()
                })
                .is_err()
        );
    }
    #[test]
    fn unavailable_inventory_does_not_prove_absence() {
        let adapter = FailingAdapter;
        let service = DiscoveryService::new(vec![&adapter]);
        let error = service
            .inspect(
                &EntityId::new("codex:session:missing").unwrap(),
                &DiscoveryScope::default(),
            )
            .unwrap_err();
        assert!(matches!(error, ApplicationError::InspectionUnavailable(_)));
    }
}
