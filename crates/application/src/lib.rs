//! Query use cases shared by CLI and the read-only MCP interface.
pub mod conversation;
pub mod indexed_search;
pub mod lifecycle;
pub mod session_search;
pub mod transactions;
use agentmender_domain::*;
use agentmender_provider_api::{ProviderAdapter, ProviderError, validate_scope};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("independently verified management identity required")]
    CallerIdentityRequired { instance: String },
    #[error("invalid bounded search request or provider response")]
    InvalidSearch,
    #[error("search continuation or conversation evidence changed")]
    StaleSearchEvidence,
    #[error("trusted management-session registration is missing or no longer verified")]
    ManagementProtectionUnavailable,
    #[error("the selected session is a protected management session")]
    ProtectedManagementSession,
    #[error("removal preview changed; obtain and review a new exact plan digest")]
    RemovalPlanChanged,
    #[error(
        "this read-only deployment cannot prepare or execute removal; review the compatibility assessment"
    )]
    ReadOnlyRemovalUnavailable,
    #[error("query cancelled or deadline exceeded")]
    Cancelled,
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("unknown entity: {0}")]
    UnknownEntity(EntityId),
    #[error("ambiguous entity: {0}")]
    AmbiguousEntity(EntityId),
    #[error("inventory incomplete; absence of {0} cannot be established")]
    InspectionUnavailable(EntityId),
}

impl ApplicationError {
    /// Stable, sanitized error catalog for inbound interfaces.
    pub fn public_code(&self) -> &'static str {
        match self {
            Self::CallerIdentityRequired { .. } => "caller_identity_required",
            Self::InvalidSearch => "invalid_scope",
            Self::StaleSearchEvidence => "stale_search_evidence",
            Self::ManagementProtectionUnavailable => "management_protection_required",
            Self::ProtectedManagementSession => "protected_management_session",
            Self::RemovalPlanChanged => "removal_plan_changed",
            Self::ReadOnlyRemovalUnavailable => "read_only_removal_unavailable",
            Self::Cancelled => "cancelled",
            Self::UnknownEntity(_) => "target_not_observed",
            Self::AmbiguousEntity(_) => "ambiguous_target",
            Self::InspectionUnavailable(_) => "target_unavailable",
            Self::Provider(error) => match error {
                ProviderError::ProviderNotInstalled(_) => "provider_not_detected",
                ProviderError::ProviderVersionUnknown(_) => "compatibility_blocked",
                ProviderError::ProviderUnsupported(_) => "provider_unsupported",
                ProviderError::CapabilityUnavailable(_) => "capability_unavailable",
                ProviderError::InvalidScope(_) => "invalid_scope",
                ProviderError::MalformedProviderState { .. }
                | ProviderError::PermissionDenied(_)
                | ProviderError::DiscoveryFailed { .. }
                | ProviderError::Graph(_) => "query_failed",
            },
        }
    }
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
        ProviderError::DiscoveryFailed { .. }
        | ProviderError::InvalidScope(_)
        | ProviderError::Graph(_) => DiagnosticCode::IncompleteDiscovery,
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

#[derive(Debug, Default, Serialize)]
pub struct GraphReport {
    pub graphs: Vec<ArtifactGraph>,
    pub failures: Vec<ProviderFailure>,
}

#[derive(Debug, Default, Serialize)]
pub struct VerificationReport {
    pub reports: Vec<IntegrityReport>,
    pub failures: Vec<ProviderFailure>,
}

/// One fresh observation and its pure planning inputs. Never accepted from a caller as proof.
pub struct ApprovalInputs {
    pub graph: ArtifactGraph,
    pub integrity: IntegrityReport,
    pub context: PlanningContext,
    pub plan: LifecyclePlan,
    pub contexts: Vec<GraphContext>,
    pub failures: Vec<ProviderFailure>,
}

/// Fresh scoped provider evidence is interpreted into an immutable plan.
pub fn cleanup_inputs(
    port: &dyn agentmender_provider_api::ScopedCleanupReadPort,
    target: &str,
    scope: &DiscoveryScope,
) -> Result<ApprovalInputs, ApplicationError> {
    validate_scope(scope)?;
    let id = EntityId::new(target)
        .map_err(|_| ProviderError::InvalidScope("invalid cleanup target".into()))?;
    let observed = port.observe_cleanup(target, scope)?;
    if observed.context.scope != *scope {
        return Err(ProviderError::InvalidScope(
            "planning context does not match discovery scope".into(),
        )
        .into());
    }
    let manifest = match (
        observed.context.cleanup_evidence.as_ref(),
        observed.context.complete_removal_evidence.as_ref(),
    ) {
        (Some(evidence), None) => &evidence.manifest,
        (None, Some(evidence)) => &evidence.manifest.primary,
        _ => return Err(ApplicationError::InspectionUnavailable(id)),
    };
    if manifest.native_session_id.as_str() != target {
        return Err(ApplicationError::InspectionUnavailable(id));
    }
    let target_id = manifest.target.clone();
    if observed
        .graph
        .nodes()
        .iter()
        .filter(|n| {
            n.id == target_id
                && n.observations.iter().any(
                    |o| matches!(o, NodeObservation::Session(s) if s.provider_session_id == target),
                )
        })
        .count()
        != 1
    {
        return Err(ApplicationError::InspectionUnavailable(id));
    }
    let integrity = evaluate_integrity(&observed.graph, &IntegrityContext::default());
    let plan = plan_lifecycle(
        &observed.graph,
        &integrity,
        &observed.context,
        LifecycleRequest {
            target: target_id,
            intent: LifecycleIntent::CleanupSession,
        },
    );
    let contexts = vec![observed.graph.context().clone()];
    Ok(ApprovalInputs {
        graph: observed.graph,
        integrity,
        context: observed.context,
        plan,
        contexts,
        failures: Vec::new(),
    })
}

#[derive(Debug, Serialize)]
pub struct PlanningReport {
    pub plan: LifecyclePlan,
    pub contexts: Vec<GraphContext>,
    pub failures: Vec<ProviderFailure>,
}

#[derive(Debug, Serialize)]
pub struct GraphExplainReport {
    pub explanation: NodeExplanation,
    /// Canonical rule descriptors, including conflicting definitions if present.
    pub rules: Vec<EvidenceRule>,
    pub context: GraphContext,
    pub graph_diagnostics: Vec<GraphDiagnostic>,
    /// Context for every discovered graph, including other partial providers.
    pub contexts: Vec<GraphContext>,
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

/// A pure matcher over full graph identities. It must not acquire provider state.
pub type TargetSelector = Arc<dyn Fn(&ArtifactId) -> bool + Send + Sync>;

pub struct ReadOnlyRequest {
    pub provider: Option<ProviderKind>,
    pub scope: DiscoveryScope,
    pub operation: ReadOnlyOperation,
}
pub enum ReadOnlyOperation {
    Providers,
    Scan,
    Graph,
    Inspect(TargetSelector),
    Explain(TargetSelector),
    Verify,
    Plan {
        target: TargetSelector,
        intent: LifecycleIntent,
    },
}
// Keep the public response shape stable for read-only consumers; the response
// is transient and never accumulated in bulk.
#[allow(clippy::large_enum_variant)]
pub enum ReadOnlyResponse {
    Providers(ProviderReport),
    Scan(ScanReport),
    Graph(GraphReport),
    Inspect(GraphExplainReport),
    Explain(GraphExplainReport),
    Verification(VerificationReport),
    Planning(PlanningReport),
}

/// Cooperative cancellation at synchronous provider operation boundaries.
#[derive(Clone, Default)]
pub struct QueryControl {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
}
impl QueryControl {
    pub fn new(deadline: Option<Instant>) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline,
        }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn check(&self) -> Result<(), ApplicationError> {
        if self.cancelled.load(Ordering::Acquire)
            || self.deadline.is_some_and(|d| Instant::now() >= d)
        {
            Err(ApplicationError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn selector_error_id() -> EntityId {
    EntityId::new("opaque-target").expect("static valid ID")
}

/// Composition supplies adapters; application never imports their concrete crates.
pub struct DiscoveryService<'a> {
    adapters: Vec<&'a dyn ProviderAdapter>,
}
impl<'a> DiscoveryService<'a> {
    pub fn new(adapters: Vec<&'a dyn ProviderAdapter>) -> Self {
        Self { adapters }
    }
    pub fn dispatch(
        &self,
        request: ReadOnlyRequest,
        control: QueryControl,
    ) -> Result<ReadOnlyResponse, ApplicationError> {
        control.check()?;
        if let Some(provider) = request.provider
            && !self
                .adapters
                .iter()
                .any(|adapter| adapter.kind() == provider)
        {
            return Err(ProviderError::ProviderUnsupported(provider).into());
        }
        let service = Self::new(
            self.adapters
                .iter()
                .copied()
                .filter(|a| request.provider.is_none_or(|p| a.kind() == p))
                .collect(),
        );
        let result = match request.operation {
            ReadOnlyOperation::Providers => {
                ReadOnlyResponse::Providers(service.providers_controlled(&control)?)
            }
            ReadOnlyOperation::Scan => {
                ReadOnlyResponse::Scan(service.scan_controlled(&request.scope, &control)?)
            }
            ReadOnlyOperation::Graph => {
                ReadOnlyResponse::Graph(service.graph_controlled(&request.scope, &control)?)
            }
            ReadOnlyOperation::Verify => {
                ReadOnlyResponse::Verification(service.verify_controlled(&request.scope, &control)?)
            }
            ReadOnlyOperation::Inspect(target) => {
                ReadOnlyResponse::Inspect(service.explain_matching(
                    target.as_ref(),
                    &request.scope,
                    &control,
                    &selector_error_id(),
                )?)
            }
            ReadOnlyOperation::Explain(target) => {
                ReadOnlyResponse::Explain(service.explain_matching(
                    target.as_ref(),
                    &request.scope,
                    &control,
                    &selector_error_id(),
                )?)
            }
            ReadOnlyOperation::Plan { target, intent } => {
                ReadOnlyResponse::Planning(service.plan_matching(
                    target.as_ref(),
                    intent,
                    &request.scope,
                    &control,
                    &selector_error_id(),
                )?)
            }
        };
        control.check()?;
        Ok(result)
    }
    pub fn providers(&self) -> ProviderReport {
        self.providers_controlled(&QueryControl::default())
            .expect("uncancelled query")
    }
    fn providers_controlled(
        &self,
        control: &QueryControl,
    ) -> Result<ProviderReport, ApplicationError> {
        let mut report = ProviderReport::default();
        for adapter in &self.adapters {
            control.check()?;
            match adapter.detect() {
                Ok(provider) => report.providers.push(provider),
                Err(error) => report.failures.push(failure(adapter.kind(), error)),
            }
        }
        report.providers.sort_by_key(|provider| provider.kind);
        report.failures.sort_by_key(|failure| failure.provider);
        control.check()?;
        Ok(report)
    }
    pub fn scan(&self, scope: &DiscoveryScope) -> Result<ScanReport, ApplicationError> {
        self.scan_controlled(scope, &QueryControl::default())
    }
    fn scan_controlled(
        &self,
        scope: &DiscoveryScope,
        control: &QueryControl,
    ) -> Result<ScanReport, ApplicationError> {
        validate_scope(scope)?;
        let mut report = ScanReport::default();
        for adapter in &self.adapters {
            control.check()?;
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
        control.check()?;
        Ok(report)
    }
    pub fn graph(&self, scope: &DiscoveryScope) -> Result<GraphReport, ApplicationError> {
        self.graph_controlled(scope, &QueryControl::default())
    }
    fn graph_controlled(
        &self,
        scope: &DiscoveryScope,
        control: &QueryControl,
    ) -> Result<GraphReport, ApplicationError> {
        validate_scope(scope)?;
        let mut report = GraphReport::default();
        for adapter in &self.adapters {
            control.check()?;
            let snapshot = adapter.discover(scope);
            control.check()?;
            let result = snapshot.and_then(|snapshot| adapter.build_graph(&snapshot));
            control.check()?;
            match result {
                Ok(graph) => report.graphs.push(graph),
                Err(error) => report.failures.push(failure(adapter.kind(), error)),
            }
        }
        report.graphs.sort_by_cached_key(|graph| {
            (
                graph.context().provider.kind,
                graph.context().provider.state_root.clone(),
                serde_json::to_string(graph).expect("validated graph serializes"),
            )
        });
        report.failures.sort_by(|a, b| {
            (a.provider, format!("{:?}", a.code), &a.message).cmp(&(
                b.provider,
                format!("{:?}", b.code),
                &b.message,
            ))
        });
        control.check()?;
        Ok(report)
    }

    pub fn verify(&self, scope: &DiscoveryScope) -> Result<VerificationReport, ApplicationError> {
        self.verify_controlled(scope, &QueryControl::default())
    }
    fn verify_controlled(
        &self,
        scope: &DiscoveryScope,
        control: &QueryControl,
    ) -> Result<VerificationReport, ApplicationError> {
        validate_scope(scope)?;
        let mut report = VerificationReport::default();
        for adapter in &self.adapters {
            control.check()?;
            let snapshot = adapter.discover(scope);
            control.check()?;
            let graph = snapshot.and_then(|snapshot| adapter.build_graph(&snapshot));
            control.check()?;
            let result = graph.and_then(|graph| {
                let context = adapter.integrity_context(&graph)?;
                Ok(evaluate_integrity(&graph, &context))
            });
            control.check()?;
            match result {
                Ok(integrity) => report.reports.push(integrity),
                Err(error) => report.failures.push(failure(adapter.kind(), error)),
            }
        }
        report.reports.sort_by_cached_key(|r| {
            (
                r.context.provider.kind,
                r.context.provider.state_root.clone(),
                serde_json::to_string(r).expect("integrity report serializes"),
            )
        });
        report.failures.sort_by(|a, b| {
            (a.provider, format!("{:?}", a.code), &a.message).cmp(&(
                b.provider,
                format!("{:?}", b.code),
                &b.message,
            ))
        });
        control.check()?;
        Ok(report)
    }

    /// Plan from exactly one discovery per adapter, preserving all graph contexts.
    pub fn plan(
        &self,
        id: &EntityId,
        intent: LifecycleIntent,
        scope: &DiscoveryScope,
    ) -> Result<PlanningReport, ApplicationError> {
        let selector = |target: &ArtifactId| &target.local_id == id;
        self.plan_matching(&selector, intent, scope, &QueryControl::default(), id)
    }
    fn plan_matching(
        &self,
        selector: &dyn Fn(&ArtifactId) -> bool,
        intent: LifecycleIntent,
        scope: &DiscoveryScope,
        control: &QueryControl,
        id: &EntityId,
    ) -> Result<PlanningReport, ApplicationError> {
        let inputs = self.approval_inputs_matching(selector, intent, scope, control, id)?;
        Ok(PlanningReport {
            plan: inputs.plan,
            contexts: inputs.contexts,
            failures: inputs.failures,
        })
    }

    /// Acquire fresh provider inputs in one discovery pass, resolving the complete target identity.
    pub fn approval_inputs(
        &self,
        selector: &dyn Fn(&ArtifactId) -> bool,
        intent: LifecycleIntent,
        scope: &DiscoveryScope,
    ) -> Result<ApprovalInputs, ApplicationError> {
        self.approval_inputs_matching(
            selector,
            intent,
            scope,
            &QueryControl::default(),
            &selector_error_id(),
        )
    }

    fn approval_inputs_matching(
        &self,
        selector: &dyn Fn(&ArtifactId) -> bool,
        intent: LifecycleIntent,
        scope: &DiscoveryScope,
        control: &QueryControl,
        id: &EntityId,
    ) -> Result<ApprovalInputs, ApplicationError> {
        validate_scope(scope)?;
        let mut graphs = Vec::new();
        let mut failures = Vec::new();
        for adapter in &self.adapters {
            control.check()?;
            let snapshot = adapter.discover(scope);
            control.check()?;
            let result = snapshot.and_then(|snapshot| adapter.build_graph(&snapshot));
            control.check()?;
            let graph = match result {
                Ok(graph) => graph,
                Err(error) => {
                    failures.push(failure(adapter.kind(), error));
                    continue;
                }
            };
            let integrity_context = adapter.integrity_context(&graph);
            control.check()?;
            let planning_context = if integrity_context.is_ok() {
                Some(adapter.planning_context(&graph, scope))
            } else {
                None
            };
            control.check()?;
            let inputs = integrity_context.and_then(|context| {
                let integrity = evaluate_integrity(&graph, &context);
                let planning = planning_context.expect("integrity succeeded")?;
                if planning.scope != *scope {
                    return Err(ProviderError::InvalidScope(
                        "planning context does not match discovery scope".into(),
                    ));
                }
                Ok((integrity, planning))
            });
            let inputs = match inputs {
                Ok(inputs) => Some(inputs),
                Err(ProviderError::InvalidScope(reason)) => {
                    return Err(ProviderError::InvalidScope(reason).into());
                }
                Err(error) => {
                    failures.push(failure(adapter.kind(), error));
                    None
                }
            };
            graphs.push((graph, inputs));
        }
        graphs.sort_by_cached_key(|(graph, _)| {
            (
                graph.context().provider.kind,
                graph.context().provider.state_root.clone(),
                serde_json::to_string(graph).expect("validated graph serializes"),
            )
        });
        failures.sort_by(|a, b| {
            (a.provider, format!("{:?}", a.code), &a.message).cmp(&(
                b.provider,
                format!("{:?}", b.code),
                &b.message,
            ))
        });
        let unavailable = !failures.is_empty()
            || graphs
                .iter()
                .any(|(g, _)| g.context().completeness != Completeness::Complete);
        let matches: Vec<_> = graphs
            .iter()
            .flat_map(|(g, inputs)| {
                g.nodes()
                    .iter()
                    .filter(move |node| selector(&node.id))
                    .map(move |node| (g, inputs, &node.id))
            })
            .collect();
        if matches.len() > 1 {
            return Err(ApplicationError::AmbiguousEntity(id.clone()));
        }
        if let Some((graph, inputs, target)) = matches.first() {
            if !failures.is_empty()
                || graphs.iter().any(|(other, _)| {
                    !std::ptr::eq(other, *graph)
                        && other.context().completeness != Completeness::Complete
                })
            {
                return Err(ApplicationError::InspectionUnavailable(id.clone()));
            }
            let Some((integrity, planning)) = inputs else {
                return Err(ApplicationError::InspectionUnavailable(id.clone()));
            };
            let plan = plan_lifecycle(
                graph,
                integrity,
                planning,
                LifecycleRequest {
                    target: (*target).clone(),
                    intent,
                },
            );
            Ok(ApprovalInputs {
                graph: (*graph).clone(),
                integrity: integrity.clone(),
                context: planning.clone(),
                plan,
                contexts: graphs.iter().map(|(g, _)| g.context().clone()).collect(),
                failures,
            })
        } else if unavailable {
            Err(ApplicationError::InspectionUnavailable(id.clone()))
        } else {
            Err(ApplicationError::UnknownEntity(id.clone()))
        }
    }

    pub fn explain(
        &self,
        id: &EntityId,
        scope: &DiscoveryScope,
    ) -> Result<GraphExplainReport, ApplicationError> {
        let selector = |target: &ArtifactId| &target.local_id == id;
        self.explain_matching(&selector, scope, &QueryControl::default(), id)
    }
    fn explain_matching(
        &self,
        selector: &dyn Fn(&ArtifactId) -> bool,
        scope: &DiscoveryScope,
        control: &QueryControl,
        id: &EntityId,
    ) -> Result<GraphExplainReport, ApplicationError> {
        let report = self.graph_controlled(scope, control)?;
        let unavailable = !report.failures.is_empty()
            || report
                .graphs
                .iter()
                .any(|g| g.context().completeness != Completeness::Complete);
        let matches: Vec<_> = report
            .graphs
            .iter()
            .flat_map(|g| {
                g.nodes()
                    .iter()
                    .filter(move |node| selector(&node.id))
                    .map(move |node| (g, node))
            })
            .collect();
        if matches.len() > 1 {
            return Err(ApplicationError::AmbiguousEntity(id.clone()));
        }
        if let Some((graph, node)) = matches.first() {
            Ok(GraphExplainReport {
                explanation: graph.explain(&node.id).map_err(ProviderError::from)?,
                rules: graph.rules().to_vec(),
                context: graph.context().clone(),
                graph_diagnostics: graph.diagnostics().to_vec(),
                contexts: report.graphs.iter().map(|g| g.context().clone()).collect(),
                failures: report.failures,
            })
        } else if unavailable {
            Err(ApplicationError::InspectionUnavailable(id.clone()))
        } else {
            Err(ApplicationError::UnknownEntity(id.clone()))
        }
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
    struct GraphAdapter {
        calls: std::cell::Cell<usize>,
        completeness: Completeness,
        fail_build: bool,
    }
    impl ProviderAdapter for GraphAdapter {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Codex
        }
        fn detect(&self) -> Result<Provider, ProviderError> {
            panic!("graph must not detect")
        }
        fn discover(&self, _: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
            self.calls.set(self.calls.get() + 1);
            let provider = Provider {
                kind: self.kind(),
                presence: Presence::StateOnly,
                state_root: "/fixture".into(),
                fingerprint: ProviderFingerprint {
                    executable: None,
                    executable_version: None,
                    adapter_version: "test".into(),
                    interface_id: None,
                    layout_id: None,
                },
                capabilities: ProviderCapabilities::new(),
                compatibility: CompatibilityProfile {
                    status: CompatibilityStatus::ReadOnlyKnown,
                    interface_stability: InterfaceStability::OfficialStable,
                    reason: "test".into(),
                },
                installations: vec![],
                evidence_status: EvidenceStatus::SyntheticOnly,
                diagnostics: vec![],
            };
            let mut snapshot = DiscoverySnapshot::empty(provider);
            snapshot.completeness = self.completeness;
            snapshot.coverage = format!("discovery {}", self.calls.get());
            Ok(snapshot)
        }
        fn build_graph(
            &self,
            snapshot: &DiscoverySnapshot,
        ) -> Result<ArtifactGraph, ProviderError> {
            assert_eq!(snapshot.coverage, format!("discovery {}", self.calls.get()));
            if self.fail_build {
                return Err(GraphError::MissingNode.into());
            }
            let mut builder = ArtifactGraph::builder(snapshot.into());
            builder.observe(NodeObservation::ProviderState);
            Ok(builder.finish()?)
        }
    }
    #[test]
    fn unconfigured_provider_is_rejected_before_adapter_calls() {
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let request = ReadOnlyRequest {
            provider: Some(ProviderKind::Claude),
            scope: DiscoveryScope::default(),
            operation: ReadOnlyOperation::Scan,
        };
        assert!(matches!(
            DiscoveryService::new(vec![&adapter]).dispatch(request, QueryControl::default()),
            Err(ApplicationError::Provider(
                ProviderError::ProviderUnsupported(ProviderKind::Claude)
            ))
        ));
        assert_eq!(adapter.calls.get(), 0);
    }

    #[test]
    fn selector_uses_fresh_graph_and_partial_absence_remains_unavailable() {
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Partial,
            fail_build: false,
        };
        let service = DiscoveryService::new(vec![&adapter]);
        let request = |target| ReadOnlyRequest {
            provider: Some(ProviderKind::Codex),
            scope: DiscoveryScope::default(),
            operation: ReadOnlyOperation::Explain(target),
        };
        let result = service
            .dispatch(
                request(Arc::new(|id| {
                    id.local_id == EntityId::new("provider-state").unwrap()
                })),
                QueryControl::default(),
            )
            .unwrap();
        assert!(matches!(result, ReadOnlyResponse::Explain(_)));
        assert_eq!(adapter.calls.get(), 1);
        assert!(matches!(
            service.dispatch(request(Arc::new(|_| false)), QueryControl::default()),
            Err(ApplicationError::InspectionUnavailable(_))
        ));
    }

    #[test]
    fn plan_selector_matches_full_identity_on_the_same_observation() {
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let service = DiscoveryService::new(vec![&adapter]);
        let selector: TargetSelector = Arc::new(|id| {
            id.state_root == std::path::Path::new("/fixture")
                && id.local_id == EntityId::new("provider-state").unwrap()
        });
        let result = service
            .dispatch(
                ReadOnlyRequest {
                    provider: Some(ProviderKind::Codex),
                    scope: DiscoveryScope::default(),
                    operation: ReadOnlyOperation::Plan {
                        target: selector,
                        intent: LifecycleIntent::Archive,
                    },
                },
                QueryControl::default(),
            )
            .unwrap();
        let ReadOnlyResponse::Planning(report) = result else {
            panic!("expected plan")
        };
        assert_eq!(
            report.plan.content().request.target.state_root,
            std::path::Path::new("/fixture")
        );
        assert_eq!(adapter.calls.get(), 1);
    }

    #[test]
    fn selector_ambiguity_and_pre_cancel_are_conservative() {
        let first = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let second = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let service = DiscoveryService::new(vec![&first, &second]);
        let request = || ReadOnlyRequest {
            provider: None,
            scope: DiscoveryScope::default(),
            operation: ReadOnlyOperation::Explain(Arc::new(|id| {
                id.local_id == EntityId::new("provider-state").unwrap()
            })),
        };
        let control = QueryControl::default();
        control.cancel();
        assert!(matches!(
            service.dispatch(request(), control),
            Err(ApplicationError::Cancelled)
        ));
        assert_eq!(first.calls.get(), 0);
        assert_eq!(second.calls.get(), 0);
        assert!(matches!(
            service.dispatch(request(), QueryControl::default()),
            Err(ApplicationError::AmbiguousEntity(_))
        ));
    }

    #[test]
    fn cancellation_after_discovery_stops_graph_building() {
        struct Cancelling {
            control: QueryControl,
            builds: std::cell::Cell<usize>,
            inner: GraphAdapter,
        }
        impl ProviderAdapter for Cancelling {
            fn kind(&self) -> ProviderKind {
                self.inner.kind()
            }
            fn detect(&self) -> Result<Provider, ProviderError> {
                self.inner.detect()
            }
            fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
                let result = self.inner.discover(scope);
                self.control.cancel();
                result
            }
            fn build_graph(
                &self,
                snapshot: &DiscoverySnapshot,
            ) -> Result<ArtifactGraph, ProviderError> {
                self.builds.set(self.builds.get() + 1);
                self.inner.build_graph(snapshot)
            }
        }
        let control = QueryControl::default();
        let adapter = Cancelling {
            control: control.clone(),
            builds: std::cell::Cell::new(0),
            inner: GraphAdapter {
                calls: std::cell::Cell::new(0),
                completeness: Completeness::Complete,
                fail_build: false,
            },
        };
        let request = ReadOnlyRequest {
            provider: None,
            scope: DiscoveryScope::default(),
            operation: ReadOnlyOperation::Graph,
        };
        assert!(matches!(
            DiscoveryService::new(vec![&adapter]).dispatch(request, control),
            Err(ApplicationError::Cancelled)
        ));
        assert_eq!(adapter.builds.get(), 0);
    }
    #[test]
    fn graph_discovers_once_and_reports_builder_failure() {
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let failing = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: true,
        };
        let report = DiscoveryService::new(vec![&adapter, &failing])
            .graph(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(adapter.calls.get(), 1);
        assert_eq!(failing.calls.get(), 1);
        assert_eq!(report.graphs[0].context().coverage, "discovery 1");
        assert_eq!(report.failures[0].code, DiagnosticCode::IncompleteDiscovery);
    }
    #[test]
    fn verify_discovers_once_and_preserves_context_failures() {
        struct ContextFailure(GraphAdapter);
        impl ProviderAdapter for ContextFailure {
            fn kind(&self) -> ProviderKind {
                self.0.kind()
            }
            fn detect(&self) -> Result<Provider, ProviderError> {
                self.0.detect()
            }
            fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
                self.0.discover(scope)
            }
            fn build_graph(
                &self,
                snapshot: &DiscoverySnapshot,
            ) -> Result<ArtifactGraph, ProviderError> {
                self.0.build_graph(snapshot)
            }
            fn integrity_context(
                &self,
                _: &ArtifactGraph,
            ) -> Result<IntegrityContext, ProviderError> {
                Err(ProviderError::CapabilityUnavailable(
                    "context failure".into(),
                ))
            }
        }
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let failing = ContextFailure(GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        });
        let report = DiscoveryService::new(vec![&adapter, &failing, &FailingAdapter])
            .verify(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(adapter.calls.get(), 1);
        assert_eq!(failing.0.calls.get(), 1);
        assert_eq!(report.reports.len(), 1);
        assert_eq!(
            report.reports[0].verification_status,
            VerificationStatus::Unknown
        );
        assert!(
            report.reports[0]
                .blockers
                .iter()
                .any(|b| b.code == IntegrityBlockerCode::MissingIntegrityContract)
        );
        assert_eq!(report.failures.len(), 2);
        assert!(
            report
                .failures
                .iter()
                .any(|f| f.message.contains("context failure"))
        );
    }
    #[test]
    fn explain_rejects_ambiguity_and_partial_absence() {
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Partial,
            fail_build: false,
        };
        let service = DiscoveryService::new(vec![&adapter]);
        assert!(matches!(
            service.explain(
                &EntityId::new("missing").unwrap(),
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::InspectionUnavailable(_))
        ));
        let second = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let service = DiscoveryService::new(vec![&adapter, &second]);
        assert!(matches!(
            service.explain(
                &EntityId::new("provider-state").unwrap(),
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::AmbiguousEntity(_))
        ));
    }
    #[test]
    fn known_native_capability_stays_blocked_without_integrity_proof() {
        struct KnownAdapter(GraphAdapter);
        impl ProviderAdapter for KnownAdapter {
            fn kind(&self) -> ProviderKind {
                self.0.kind()
            }
            fn detect(&self) -> Result<Provider, ProviderError> {
                self.0.detect()
            }
            fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
                self.0.discover(scope)
            }
            fn build_graph(
                &self,
                snapshot: &DiscoverySnapshot,
            ) -> Result<ArtifactGraph, ProviderError> {
                self.0.build_graph(snapshot)
            }
            fn planning_context(
                &self,
                graph: &ArtifactGraph,
                scope: &DiscoveryScope,
            ) -> Result<PlanningContext, ProviderError> {
                Ok(PlanningContext {
                    scope: scope.clone(),
                    capabilities: vec![LifecycleCapability {
                        provider: self.kind(),
                        fingerprint: graph.context().provider.fingerprint.clone(),
                        operation: NativeLifecycleOperation::ArchiveSession,
                        protocol_evidence_digest: format!("sha256:{}", "0".repeat(64)),
                        effect_scope: NativeEffectScope::TargetOnly,
                    }],
                    complete_removal_evidence: None,
                    cleanup_evidence: None,
                    effects: None,
                })
            }
        }
        let adapter = KnownAdapter(GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        });
        let report = DiscoveryService::new(vec![&adapter])
            .plan(
                &EntityId::new("provider-state").unwrap(),
                LifecycleIntent::Archive,
                &DiscoveryScope::default(),
            )
            .unwrap();
        assert_eq!(adapter.0.calls.get(), 1);
        assert_eq!(report.plan.content().status, PlanStatus::Blocked);
        assert!(
            report
                .plan
                .content()
                .blockers
                .contains(&PlanBlocker::IntegrityUnverified)
        );
        assert!(report.plan.content().operations.is_empty());
    }
    #[test]
    fn planning_discovers_once_and_resolves_identity_conservatively() {
        let adapter = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        let id = EntityId::new("provider-state").unwrap();
        assert!(matches!(
            DiscoveryService::new(vec![&adapter, &FailingAdapter]).plan(
                &id,
                LifecycleIntent::Archive,
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::InspectionUnavailable(_))
        ));
        assert_eq!(adapter.calls.get(), 1);
        let report = DiscoveryService::new(vec![&adapter])
            .plan(&id, LifecycleIntent::Archive, &DiscoveryScope::default())
            .unwrap();
        assert_eq!(report.plan.content().request.target.local_id, id);
        struct EmptyPartial(GraphAdapter);
        impl ProviderAdapter for EmptyPartial {
            fn kind(&self) -> ProviderKind {
                self.0.kind()
            }
            fn detect(&self) -> Result<Provider, ProviderError> {
                self.0.detect()
            }
            fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
                self.0.discover(scope)
            }
            fn build_graph(
                &self,
                snapshot: &DiscoverySnapshot,
            ) -> Result<ArtifactGraph, ProviderError> {
                Ok(ArtifactGraph::builder(snapshot.into()).finish()?)
            }
        }
        let partial = EmptyPartial(GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Partial,
            fail_build: false,
        });
        assert!(matches!(
            DiscoveryService::new(vec![&adapter, &partial]).plan(
                &id,
                LifecycleIntent::Archive,
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::InspectionUnavailable(_))
        ));
        assert_eq!(partial.0.calls.get(), 1);
        let selected_partial = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Partial,
            fail_build: false,
        };
        assert!(
            DiscoveryService::new(vec![&selected_partial])
                .plan(&id, LifecycleIntent::Archive, &DiscoveryScope::default())
                .is_ok()
        );
        let missing = EntityId::new("missing").unwrap();
        assert!(matches!(
            DiscoveryService::new(vec![&adapter]).plan(
                &missing,
                LifecycleIntent::Archive,
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::UnknownEntity(_))
        ));
        assert!(matches!(
            DiscoveryService::new(vec![&adapter, &FailingAdapter]).plan(
                &missing,
                LifecycleIntent::Archive,
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::InspectionUnavailable(_))
        ));
        let second = GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        };
        assert!(matches!(
            DiscoveryService::new(vec![&adapter, &second]).plan(
                &id,
                LifecycleIntent::Archive,
                &DiscoveryScope::default()
            ),
            Err(ApplicationError::AmbiguousEntity(_))
        ));
    }
    #[test]
    fn planning_rejects_adapter_context_with_different_scope() {
        struct WrongScope(GraphAdapter);
        impl ProviderAdapter for WrongScope {
            fn kind(&self) -> ProviderKind {
                self.0.kind()
            }
            fn detect(&self) -> Result<Provider, ProviderError> {
                self.0.detect()
            }
            fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
                self.0.discover(scope)
            }
            fn build_graph(
                &self,
                snapshot: &DiscoverySnapshot,
            ) -> Result<ArtifactGraph, ProviderError> {
                self.0.build_graph(snapshot)
            }
            fn planning_context(
                &self,
                _: &ArtifactGraph,
                _: &DiscoveryScope,
            ) -> Result<PlanningContext, ProviderError> {
                Ok(PlanningContext::default())
            }
        }
        let adapter = WrongScope(GraphAdapter {
            calls: std::cell::Cell::new(0),
            completeness: Completeness::Complete,
            fail_build: false,
        });
        let scope = DiscoveryScope {
            include_archived: false,
            ..Default::default()
        };
        assert!(matches!(
            DiscoveryService::new(vec![&adapter]).plan(
                &EntityId::new("provider-state").unwrap(),
                LifecycleIntent::Archive,
                &scope
            ),
            Err(ApplicationError::Provider(ProviderError::InvalidScope(_)))
        ));
        assert_eq!(adapter.0.calls.get(), 1);
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
