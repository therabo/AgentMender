//! Pure integrity evaluation over explicit closed observation contracts.
use crate::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityContract {
    pub id: EntityId,
    pub provider_version: String,
    pub adapter_version: String,
    pub interface_id: Option<String>,
    pub layout_id: Option<String>,
    pub coherent: bool,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeCoverage {
    pub class: GraphNodeKind,
    pub completeness: Completeness,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelationshipCoverage {
    pub source_class: GraphNodeKind,
    pub target_class: GraphNodeKind,
    pub kind: EdgeKind,
    pub completeness: Completeness,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredTargetKind {
    Reference,
    Transcript,
    Child,
    DatabaseReference,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequiredTarget {
    pub source: ArtifactId,
    pub target: ArtifactId,
    pub edge_kind: EdgeKind,
    pub kind: RequiredTargetKind,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerDirection {
    OwnerToArtifact,
    ArtifactToOwner,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnerRelation {
    pub owner_class: GraphNodeKind,
    pub edge_kind: EdgeKind,
    pub direction: OwnerDirection,
    pub rule_ids: Vec<EntityId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequiredOwner {
    pub artifact: ArtifactId,
    pub relations: Vec<OwnerRelation>,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResidualObservation {
    pub artifact: ArtifactId,
    pub expectation_evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct IntegrityContext {
    pub contract: Option<IntegrityContract>,
    pub node_coverage: Vec<NodeCoverage>,
    pub relationship_coverage: Vec<RelationshipCoverage>,
    pub required_targets: Vec<RequiredTarget>,
    pub required_owners: Vec<RequiredOwner>,
    pub residuals: Vec<ResidualObservation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Healthy,
    HealthyWithExpectedResiduals,
    Degraded,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidualClassification {
    ExpectedProviderResidual,
    SafeResidual,
    UnknownResidual,
    DangerousResidual,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityIssueKind {
    DanglingReference,
    MissingTranscript,
    MissingChild,
    BrokenDatabaseReference,
    OrphanTranscript,
    OrphanChild,
    OrphanAttachment,
    OrphanTodo,
    OrphanPlan,
    DuplicateSessionIdentifier,
    ConflictingIdentityEvidence,
    ConflictingRelationshipEvidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegritySeverity {
    Info,
    Warning,
    Error,
    Critical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueDisposition {
    EstablishedViolation,
    SuspiciousInconsistency,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityIssueIdentity {
    pub provider: ProviderKind,
    #[serde(with = "crate::path_serde")]
    pub state_root: PathBuf,
    pub kind: IntegrityIssueKind,
    pub subject: ArtifactId,
    pub affected: Vec<ArtifactId>,
    pub rule_ids: Vec<EntityId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityIssue {
    pub identity: IntegrityIssueIdentity,
    pub kind: IntegrityIssueKind,
    pub severity: IntegritySeverity,
    pub disposition: IssueDisposition,
    pub affected: Vec<ArtifactId>,
    pub evidence: Vec<EvaluatedEvidence>,
    pub explanation: String,
    pub observations: Vec<NodeObservation>,
    pub relationships: Vec<ReferenceEdge>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityBlockerCode {
    MissingIntegrityContract,
    ContractMismatch,
    IncoherentObservation,
    UnknownCompatibility,
    PartialCoverage,
    MissingCoverage,
    ConflictingCoverage,
    SnapshotStale,
    ConcurrentChange,
    LinkSkipped,
    MalformedObservation,
    UnreadableObservation,
    UnknownRule,
    UnknownProducer,
    UnknownArtifact,
    UnresolvedIdentity,
    ConflictingEvidence,
    UnmetPrerequisite,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityBlocker {
    pub code: IntegrityBlockerCode,
    pub affected: Vec<ArtifactId>,
    pub evidence: Vec<EvaluatedEvidence>,
    pub explanation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityResidual {
    pub artifact: ArtifactId,
    pub classification: ResidualClassification,
    pub evidence: Vec<EvaluatedEvidence>,
    pub explanation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityReport {
    pub context: GraphContext,
    pub proof_context: IntegrityContext,
    pub verification_status: VerificationStatus,
    pub findings: Vec<IntegrityIssue>,
    pub blockers: Vec<IntegrityBlocker>,
    pub residuals: Vec<IntegrityResidual>,
    pub graph_diagnostics: Vec<GraphDiagnostic>,
    pub rules: Vec<EvidenceRule>,
}

fn canonical<T: Serialize>(v: &mut Vec<T>) {
    let mut keyed: Vec<_> = std::mem::take(v)
        .into_iter()
        .map(|x| {
            (
                serde_json::to_string(&x).expect("serializable domain fact"),
                x,
            )
        })
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.dedup_by(|a, b| a.0 == b.0);
    *v = keyed.into_iter().map(|(_, x)| x).collect();
}
fn normalize(c: &mut IntegrityContext) {
    if let Some(x) = &mut c.contract {
        canonical(&mut x.evidence);
    }
    for x in &mut c.node_coverage {
        canonical(&mut x.evidence);
    }
    for x in &mut c.relationship_coverage {
        canonical(&mut x.evidence);
    }
    for x in &mut c.required_targets {
        canonical(&mut x.evidence);
    }
    for x in &mut c.required_owners {
        canonical(&mut x.evidence);
        for r in &mut x.relations {
            canonical(&mut r.rule_ids);
        }
        canonical(&mut x.relations);
    }
    for x in &mut c.residuals {
        canonical(&mut x.expectation_evidence);
    }
    canonical(&mut c.node_coverage);
    canonical(&mut c.relationship_coverage);
    canonical(&mut c.required_targets);
    canonical(&mut c.required_owners);
    canonical(&mut c.residuals);
}
fn evaluated(g: &ArtifactGraph, e: &[GraphEvidence]) -> Vec<EvaluatedEvidence> {
    let mut v: Vec<_> = e
        .iter()
        .map(|x| EvaluatedEvidence {
            evidence: x.clone(),
            confidence: g.effective_confidence(x),
        })
        .collect();
    canonical(&mut v);
    v
}
fn authoritative(g: &ArtifactGraph, c: &IntegrityContract, e: &[GraphEvidence]) -> bool {
    !e.is_empty()
        && e.iter().all(|x| {
            x.nature == EvidenceNature::Observation
                && x.observation.provider_version.as_deref() == Some(c.provider_version.as_str())
                && g.effective_confidence(x) == Confidence::Authoritative
        })
}
fn blocker(
    code: IntegrityBlockerCode,
    affected: Vec<ArtifactId>,
    evidence: Vec<EvaluatedEvidence>,
    explanation: &str,
) -> IntegrityBlocker {
    IntegrityBlocker {
        code,
        affected,
        evidence,
        explanation: explanation.into(),
    }
}
fn issue(
    g: &ArtifactGraph,
    kind: IntegrityIssueKind,
    disposition: IssueDisposition,
    subject: ArtifactId,
    mut affected: Vec<ArtifactId>,
    evidence: Vec<GraphEvidence>,
    explanation: &str,
) -> IntegrityIssue {
    canonical(&mut affected);
    let mut rule_ids: Vec<_> = evidence.iter().map(|x| x.rule_id.clone()).collect();
    canonical(&mut rule_ids);
    IntegrityIssue {
        identity: IntegrityIssueIdentity {
            provider: subject.provider,
            state_root: subject.state_root.clone(),
            kind,
            subject,
            affected: affected.clone(),
            rule_ids,
        },
        kind,
        severity: match kind {
            IntegrityIssueKind::MissingChild
            | IntegrityIssueKind::BrokenDatabaseReference
            | IntegrityIssueKind::DuplicateSessionIdentifier => IntegritySeverity::Error,
            _ => IntegritySeverity::Warning,
        },
        disposition,
        affected,
        evidence: evaluated(g, &evidence),
        explanation: explanation.into(),
        observations: Vec::new(),
        relationships: Vec::new(),
    }
}
fn same_local(a: &ArtifactId, b: &ArtifactId) -> bool {
    a.provider == b.provider && a.state_root == b.state_root && a.local_id == b.local_id
}
fn normalized_observation(o: &NodeObservation) -> serde_json::Value {
    let mut v = serde_json::to_value(o).expect("observation serializes");
    if let Some(value) = v.get_mut("value").and_then(|x| x.as_object_mut()) {
        value.remove("evidence");
    }
    v
}
fn known_node(g: &ArtifactGraph, id: &ArtifactId) -> bool {
    g.node(id).is_some_and(|n| {
        id.class != GraphNodeKind::Unresolved
            && !n.observations.iter().any(
                |o| matches!(o, NodeObservation::Artifact(a) if a.kind == ArtifactKind::Unknown),
            )
    })
}
fn node_complete(
    g: &ArtifactGraph,
    c: &IntegrityContext,
    contract: &IntegrityContract,
    class: GraphNodeKind,
) -> bool {
    let records: Vec<_> = c
        .node_coverage
        .iter()
        .filter(|x| x.class == class)
        .collect();
    !records.is_empty()
        && records.iter().all(|x| {
            x.completeness == Completeness::Complete && authoritative(g, contract, &x.evidence)
        })
}
fn relation_complete(
    g: &ArtifactGraph,
    c: &IntegrityContext,
    contract: &IntegrityContract,
    source: GraphNodeKind,
    target: GraphNodeKind,
    kind: EdgeKind,
) -> bool {
    let records: Vec<_> = c
        .relationship_coverage
        .iter()
        .filter(|x| x.source_class == source && x.target_class == target && x.kind == kind)
        .collect();
    !records.is_empty()
        && records.iter().all(|x| {
            x.completeness == Completeness::Complete && authoritative(g, contract, &x.evidence)
        })
}

/// Evaluate only the supplied graph and explicit adapter contract; no I/O or clock is used.
pub fn evaluate_integrity(g: &ArtifactGraph, input: &IntegrityContext) -> IntegrityReport {
    let mut c = input.clone();
    normalize(&mut c);
    let mut blockers = Vec::new();
    let mut findings = Vec::new();
    let fp = &g.context().provider.fingerprint;
    if g.context().provider.compatibility.status != CompatibilityStatus::ReadOnlyKnown {
        blockers.push(blocker(
            IntegrityBlockerCode::UnknownCompatibility,
            vec![],
            vec![],
            "Compatibility is not an exact known interface.",
        ));
    }
    if g.context().completeness != Completeness::Complete {
        blockers.push(blocker(
            IntegrityBlockerCode::PartialCoverage,
            vec![],
            vec![],
            "Graph discovery is incomplete.",
        ));
    }
    let contract = c.contract.as_ref();
    if let Some(x) = contract {
        if fp.executable_version.as_deref() != Some(x.provider_version.as_str())
            || fp.adapter_version != x.adapter_version
            || fp.interface_id != x.interface_id
            || fp.layout_id != x.layout_id
            || !authoritative(g, x, &x.evidence)
        {
            blockers.push(blocker(
                IntegrityBlockerCode::ContractMismatch,
                vec![],
                evaluated(g, &x.evidence),
                "Contract fingerprint or evidence is not verified.",
            ));
        }
        if !x.coherent {
            blockers.push(blocker(
                IntegrityBlockerCode::IncoherentObservation,
                vec![],
                evaluated(g, &x.evidence),
                "Observation coherence is not asserted.",
            ));
        }
    } else {
        blockers.push(blocker(
            IntegrityBlockerCode::MissingIntegrityContract,
            vec![],
            vec![],
            "No closed integrity contract was supplied.",
        ));
    }
    for d in g
        .context()
        .diagnostics
        .iter()
        .chain(g.context().provider.diagnostics.iter())
    {
        let code = match d.code {
            DiagnosticCode::SnapshotStale => Some(IntegrityBlockerCode::SnapshotStale),
            DiagnosticCode::ConcurrentChange => Some(IntegrityBlockerCode::ConcurrentChange),
            DiagnosticCode::LinkSkipped => Some(IntegrityBlockerCode::LinkSkipped),
            DiagnosticCode::MalformedProviderState => {
                Some(IntegrityBlockerCode::MalformedObservation)
            }
            DiagnosticCode::PermissionDenied | DiagnosticCode::SnapshotUnavailable => {
                Some(IntegrityBlockerCode::UnreadableObservation)
            }
            DiagnosticCode::UnknownArtifact => Some(IntegrityBlockerCode::UnknownArtifact),
            DiagnosticCode::ProviderVersionUnknown => Some(IntegrityBlockerCode::UnknownProducer),
            DiagnosticCode::ProviderUnsupported
            | DiagnosticCode::CapabilityUnavailable
            | DiagnosticCode::LimitReached
            | DiagnosticCode::IncompleteDiscovery
            | DiagnosticCode::ProviderNotInstalled
            | DiagnosticCode::ProviderNotDetected => Some(IntegrityBlockerCode::UnmetPrerequisite),
        };
        if let Some(code) = code {
            blockers.push(blocker(
                code,
                vec![],
                vec![],
                "Discovery reported an integrity limitation.",
            ));
        }
    }
    for n in g.nodes() {
        if n.id.class == GraphNodeKind::Unresolved {
            blockers.push(blocker(
                IntegrityBlockerCode::UnresolvedIdentity,
                vec![n.id.clone()],
                evaluated(g, &n.evidence),
                "Endpoint identity is unresolved.",
            ));
        }
        if !known_node(g, &n.id) && n.id.class != GraphNodeKind::Unresolved {
            blockers.push(blocker(
                IntegrityBlockerCode::UnknownArtifact,
                vec![n.id.clone()],
                evaluated(g, &n.evidence),
                "Artifact kind is unknown.",
            ));
        }
        let mut variants: Vec<_> = n.observations.iter().map(normalized_observation).collect();
        canonical(&mut variants);
        if variants.len() > 1 {
            findings.push(issue(
                g,
                if n.id.class == GraphNodeKind::Session {
                    IntegrityIssueKind::DuplicateSessionIdentifier
                } else {
                    IntegrityIssueKind::ConflictingIdentityEvidence
                },
                IssueDisposition::SuspiciousInconsistency,
                n.id.clone(),
                vec![n.id.clone()],
                n.evidence.clone(),
                "Distinct observations disagree about the same identity.",
            ));
            blockers.push(blocker(
                IntegrityBlockerCode::ConflictingEvidence,
                vec![n.id.clone()],
                evaluated(g, &n.evidence),
                "Conflicting identity observations prevent negative proof.",
            ));
        }
    }
    for d in g.diagnostics() {
        match d {
            GraphDiagnostic::ConflictingParentage { child } => {
                let edges: Vec<_> = g
                    .edges()
                    .iter()
                    .filter(|e| e.kind == EdgeKind::SpawnedBy && &e.source == child)
                    .collect();
                let mut affected = vec![child.clone()];
                let mut ev = vec![];
                for edge in edges {
                    affected.push(edge.target.clone());
                    ev.extend(edge.evidence.clone());
                }
                findings.push(issue(
                    g,
                    IntegrityIssueKind::ConflictingRelationshipEvidence,
                    IssueDisposition::SuspiciousInconsistency,
                    child.clone(),
                    affected.clone(),
                    ev.clone(),
                    "Observed parent claims disagree.",
                ));
                blockers.push(blocker(
                    IntegrityBlockerCode::ConflictingEvidence,
                    affected,
                    evaluated(g, &ev),
                    "Conflicting parent claims prevent negative proof.",
                ));
            }
            GraphDiagnostic::ConflictingRuleDefinitions { .. } => blockers.push(blocker(
                IntegrityBlockerCode::ConflictingEvidence,
                vec![],
                vec![],
                "Rule definitions conflict.",
            )),
            GraphDiagnostic::UnrecognizedRule { .. }
            | GraphDiagnostic::InadmissibleSource { .. } => blockers.push(blocker(
                IntegrityBlockerCode::UnknownRule,
                vec![],
                vec![],
                "Evidence rule is unknown or inadmissible.",
            )),
            _ => (),
        }
    }
    for e in g
        .nodes()
        .iter()
        .flat_map(|n| n.evidence.iter())
        .chain(g.edges().iter().flat_map(|x| x.evidence.iter()))
    {
        if contract.is_some_and(|c| {
            e.observation.provider_version.as_deref() != Some(c.provider_version.as_str())
        }) || e.observation.provider_version.is_none()
        {
            blockers.push(blocker(
                IntegrityBlockerCode::UnknownProducer,
                vec![],
                evaluated(g, std::slice::from_ref(e)),
                "Evidence producer is unknown.",
            ));
        }
    }
    if let Some(contract) = contract {
        for node in g.nodes().iter().filter(|n| {
            n.id.class != GraphNodeKind::Unresolved && n.id.class != GraphNodeKind::ProviderState
        }) {
            if !authoritative(g, contract, &node.evidence) {
                blockers.push(blocker(
                    IntegrityBlockerCode::UnmetPrerequisite,
                    vec![node.id.clone()],
                    evaluated(g, &node.evidence),
                    "Entity evidence is not authoritative within the contract.",
                ));
            }
        }
    }
    if let Some(contract) = contract {
        for x in &c.node_coverage {
            if !node_complete(g, &c, contract, x.class) {
                blockers.push(blocker(
                    IntegrityBlockerCode::ConflictingCoverage,
                    vec![],
                    evaluated(g, &x.evidence),
                    "Node coverage is incomplete or unverified.",
                ));
            }
        }
        for x in &c.relationship_coverage {
            if !relation_complete(g, &c, contract, x.source_class, x.target_class, x.kind) {
                blockers.push(blocker(
                    IntegrityBlockerCode::ConflictingCoverage,
                    vec![],
                    evaluated(g, &x.evidence),
                    "Relationship coverage is incomplete or unverified.",
                ));
            }
        }
    }
    for (i, left) in c.required_owners.iter().enumerate() {
        for right in c
            .required_owners
            .iter()
            .skip(i + 1)
            .filter(|r| r.artifact == left.artifact)
        {
            if left.relations != right.relations {
                let mut evidence = left.evidence.clone();
                evidence.extend(right.evidence.clone());
                blockers.push(blocker(
                    IntegrityBlockerCode::ConflictingEvidence,
                    vec![left.artifact.clone()],
                    evaluated(g, &evidence),
                    "Exhaustive owner declarations disagree.",
                ));
            }
        }
    }
    for (i, left) in c.required_targets.iter().enumerate() {
        for right in c.required_targets.iter().skip(i + 1).filter(|r| {
            r.source == left.source
                && r.edge_kind == left.edge_kind
                && same_local(&r.target, &left.target)
        }) {
            if left.kind != right.kind || left.target.class != right.target.class {
                let mut evidence = left.evidence.clone();
                evidence.extend(right.evidence.clone());
                blockers.push(blocker(
                    IntegrityBlockerCode::ConflictingEvidence,
                    vec![
                        left.source.clone(),
                        left.target.clone(),
                        right.target.clone(),
                    ],
                    evaluated(g, &evidence),
                    "Required target declarations disagree.",
                ));
            }
        }
    }
    // An unresolved placeholder is expected for a missing reference; it blocks overall health,
    // but not an otherwise closed proof of that specific target.
    let base_ok = blockers
        .iter()
        .all(|b| b.code == IntegrityBlockerCode::UnresolvedIdentity);
    if let Some(contract) = contract {
        for t in &c.required_targets {
            let present = g.node(&t.target).is_some();
            let ambiguous = g.nodes().iter().any(|n| {
                n.id.class != GraphNodeKind::Unresolved
                    && n.id.class != t.target.class
                    && same_local(&n.id, &t.target)
            });
            let edges: Vec<_> = g
                .edges()
                .iter()
                .filter(|e| {
                    e.source == t.source
                        && e.kind == t.edge_kind
                        && same_local(&e.target, &t.target)
                        && (e.target.class == t.target.class
                            || e.target.class == GraphNodeKind::Unresolved)
                })
                .collect();
            let compatible = t.source.class != GraphNodeKind::Unresolved
                && t.target.class != GraphNodeKind::Unresolved
                && match t.kind {
                    RequiredTargetKind::Transcript => t.target.class == GraphNodeKind::Artifact && g.node(&t.target).is_none_or(|n| n.observations.iter().all(|o| matches!(o, NodeObservation::Artifact(a) if a.kind == ArtifactKind::Transcript))),
                    RequiredTargetKind::Child => {
                        t.source.class == GraphNodeKind::Session
                            && t.target.class == GraphNodeKind::Session
                            && t.edge_kind != EdgeKind::SpawnedBy
                            && t.source != t.target
                    }
                    _ => true,
                };
            let valid = compatible
                && base_ok
                && known_node(g, &t.source)
                && t.target.class != GraphNodeKind::Unresolved
                && !ambiguous
                && authoritative(g, contract, &t.evidence)
                && node_complete(g, &c, contract, t.target.class)
                && edges
                    .iter()
                    .any(|e| authoritative(g, contract, &e.evidence));
            if !present && valid {
                let kind = match t.kind {
                    RequiredTargetKind::Reference => IntegrityIssueKind::DanglingReference,
                    RequiredTargetKind::Transcript => IntegrityIssueKind::MissingTranscript,
                    RequiredTargetKind::Child => IntegrityIssueKind::MissingChild,
                    RequiredTargetKind::DatabaseReference => {
                        IntegrityIssueKind::BrokenDatabaseReference
                    }
                };
                let mut ev = t.evidence.clone();
                ev.extend(contract.evidence.clone());
                for coverage in c.node_coverage.iter().filter(|x| x.class == t.target.class) {
                    ev.extend(coverage.evidence.clone());
                }
                for e in edges {
                    ev.extend(e.evidence.clone());
                }
                findings.push(issue(
                    g,
                    kind,
                    IssueDisposition::EstablishedViolation,
                    t.source.clone(),
                    vec![t.source.clone(), t.target.clone()],
                    ev,
                    "An authoritative required target is absent from its complete namespace.",
                ));
            } else if !valid {
                blockers.push(blocker(
                    IntegrityBlockerCode::UnmetPrerequisite,
                    vec![t.source.clone(), t.target.clone()],
                    evaluated(g, &t.evidence),
                    "Required target absence cannot be verified.",
                ));
            }
        }
        for owner in &c.required_owners {
            let orphan_kind = g
                .node(&owner.artifact)
                .and_then(|n| n.observations.first())
                .and_then(|o| match o {
                    NodeObservation::Session(_) => Some(IntegrityIssueKind::OrphanChild),
                    NodeObservation::Artifact(a) => match a.kind {
                        ArtifactKind::Transcript => Some(IntegrityIssueKind::OrphanTranscript),
                        ArtifactKind::Attachment => Some(IntegrityIssueKind::OrphanAttachment),
                        ArtifactKind::Todo => Some(IntegrityIssueKind::OrphanTodo),
                        ArtifactKind::Plan => Some(IntegrityIssueKind::OrphanPlan),
                        _ => None,
                    },
                    _ => None,
                });
            let roles_valid = !owner.relations.is_empty()
                && owner.relations.iter().all(|r| {
                    let (source, target) = match r.direction {
                        OwnerDirection::OwnerToArtifact => (r.owner_class, owner.artifact.class),
                        OwnerDirection::ArtifactToOwner => (owner.artifact.class, r.owner_class),
                    };
                    !r.rule_ids.is_empty()
                        && r.owner_class != GraphNodeKind::Unresolved
                        && r.rule_ids.iter().all(|id| {
                            let matching: Vec<_> =
                                g.rules().iter().filter(|rule| &rule.id == id).collect();
                            matching.len() == 1
                                && matching[0].adapter_version == contract.adapter_version
                                && matching[0].interface_id == contract.interface_id
                                && matching[0].layout_id == contract.layout_id
                        })
                        && node_complete(g, &c, contract, r.owner_class)
                        && relation_complete(g, &c, contract, source, target, r.edge_kind)
                });
            let mut candidate = false;
            let mut uncertain = false;
            for r in &owner.relations {
                for e in g.edges().iter().filter(|e| e.kind == r.edge_kind) {
                    let other = match r.direction {
                        OwnerDirection::OwnerToArtifact if e.target == owner.artifact => {
                            Some(&e.source)
                        }
                        OwnerDirection::ArtifactToOwner if e.source == owner.artifact => {
                            Some(&e.target)
                        }
                        _ => None,
                    };
                    if let Some(id) = other {
                        for ev in &e.evidence {
                            if r.rule_ids.contains(&ev.rule_id) {
                                if id.class == r.owner_class
                                    && known_node(g, id)
                                    && !g.nodes().iter().any(|n| {
                                        n.id.class != id.class
                                            && n.id.class != GraphNodeKind::Unresolved
                                            && same_local(&n.id, id)
                                    })
                                    && authoritative(g, contract, std::slice::from_ref(ev))
                                {
                                    candidate = true;
                                } else {
                                    uncertain = true;
                                }
                            } else if !g.rules().iter().any(|rule| rule.id == ev.rule_id) {
                                uncertain = true;
                            }
                        }
                    }
                }
            }
            let owner_valid = base_ok
                && known_node(g, &owner.artifact)
                && roles_valid
                && authoritative(g, contract, &owner.evidence)
                && node_complete(g, &c, contract, owner.artifact.class);
            if !candidate
                && !uncertain
                && owner_valid
                && !g.roots().iter().any(|r| r.node == owner.artifact)
                && let Some(kind) = orphan_kind
            {
                let mut ev = owner.evidence.clone();
                ev.extend(contract.evidence.clone());
                for coverage in &c.node_coverage {
                    if coverage.class == owner.artifact.class
                        || owner
                            .relations
                            .iter()
                            .any(|r| r.owner_class == coverage.class)
                    {
                        ev.extend(coverage.evidence.clone());
                    }
                }
                for role in &owner.relations {
                    let (source, target) = match role.direction {
                        OwnerDirection::OwnerToArtifact => (role.owner_class, owner.artifact.class),
                        OwnerDirection::ArtifactToOwner => (owner.artifact.class, role.owner_class),
                    };
                    for coverage in c.relationship_coverage.iter().filter(|x| {
                        x.source_class == source
                            && x.target_class == target
                            && x.kind == role.edge_kind
                    }) {
                        ev.extend(coverage.evidence.clone());
                    }
                }
                findings.push(issue(
                    g,
                    kind,
                    IssueDisposition::EstablishedViolation,
                    owner.artifact.clone(),
                    vec![owner.artifact.clone()],
                    ev,
                    "No owner exists in the explicitly closed owner namespaces.",
                ));
            } else if !candidate || !owner_valid || uncertain {
                blockers.push(blocker(
                    IntegrityBlockerCode::UnmetPrerequisite,
                    vec![owner.artifact.clone()],
                    evaluated(g, &owner.evidence),
                    "Owner absence cannot be verified.",
                ));
            }
        }
        for n in g
            .nodes()
            .iter()
            .filter(|n| n.id.class != GraphNodeKind::Unresolved)
        {
            if !node_complete(g, &c, contract, n.id.class) {
                blockers.push(blocker(
                    IntegrityBlockerCode::MissingCoverage,
                    vec![n.id.clone()],
                    vec![],
                    "Observed node class lacks complete authoritative coverage.",
                ));
            }
        }
        for edge in g.edges() {
            if edge.source.class != GraphNodeKind::Unresolved
                && edge.target.class != GraphNodeKind::Unresolved
                && !relation_complete(
                    g,
                    &c,
                    contract,
                    edge.source.class,
                    edge.target.class,
                    edge.kind,
                )
            {
                blockers.push(blocker(
                    IntegrityBlockerCode::MissingCoverage,
                    vec![edge.source.clone(), edge.target.clone()],
                    vec![],
                    "Observed relationship namespace lacks complete authoritative coverage.",
                ));
            }
        }
        if c.node_coverage.is_empty() {
            blockers.push(blocker(
                IntegrityBlockerCode::MissingCoverage,
                vec![],
                vec![],
                "No closed node namespace was declared.",
            ));
        }
    }
    if c.required_targets.is_empty() && c.required_owners.is_empty() && c.residuals.is_empty() {
        blockers.push(blocker(
            IntegrityBlockerCode::UnmetPrerequisite,
            vec![],
            vec![],
            "No integrity checks were declared.",
        ));
    }
    let mut residual_groups: std::collections::BTreeMap<ArtifactId, Vec<&ResidualObservation>> =
        std::collections::BTreeMap::new();
    for residual in &c.residuals {
        residual_groups
            .entry(residual.artifact.clone())
            .or_default()
            .push(residual);
    }
    let residuals: Vec<_> = residual_groups
        .into_iter()
        .map(|(artifact, records)| {
            let expected = blockers
                .iter()
                .all(|b| b.code == IntegrityBlockerCode::UnresolvedIdentity)
                && !findings.iter().any(|f| {
                    f.disposition == IssueDisposition::EstablishedViolation
                        && f.affected.contains(&artifact)
                })
                && contract.is_some_and(|x| {
                    records
                        .iter()
                        .all(|r| authoritative(g, x, &r.expectation_evidence))
                })
                && known_node(g, &artifact);
            let evidence = records
                .iter()
                .flat_map(|r| r.expectation_evidence.clone())
                .collect::<Vec<_>>();
            IntegrityResidual {
                artifact,
                classification: if expected {
                    ResidualClassification::ExpectedProviderResidual
                } else {
                    ResidualClassification::UnknownResidual
                },
                evidence: evaluated(g, &evidence),
                explanation: if expected {
                    "Explicit contract evidence identifies an expected provider residual."
                } else {
                    "Residual expectation is unverified and remains protected."
                }
                .into(),
            }
        })
        .collect();
    if residuals
        .iter()
        .any(|r| r.classification == ResidualClassification::UnknownResidual)
    {
        blockers.push(blocker(
            IntegrityBlockerCode::UnmetPrerequisite,
            vec![],
            vec![],
            "At least one residual expectation is unverified.",
        ));
    }
    for finding in &mut findings {
        finding.observations = finding
            .affected
            .iter()
            .filter_map(|id| g.node(id))
            .flat_map(|n| n.observations.clone())
            .collect();
        finding.relationships = g
            .edges()
            .iter()
            .filter(|e| {
                finding.affected.contains(&e.source) && finding.affected.contains(&e.target)
            })
            .cloned()
            .collect();
        canonical(&mut finding.observations);
        canonical(&mut finding.relationships);
    }
    for b in &mut blockers {
        canonical(&mut b.affected);
        canonical(&mut b.evidence);
    }
    canonical(&mut blockers);
    let mut merged: std::collections::BTreeMap<String, IntegrityIssue> =
        std::collections::BTreeMap::new();
    for finding in findings {
        let key = serde_json::to_string(&finding.identity).expect("issue identity serializes");
        if let Some(existing) = merged.get_mut(&key) {
            existing.evidence.extend(finding.evidence);
            existing.observations.extend(finding.observations);
            existing.relationships.extend(finding.relationships);
            canonical(&mut existing.evidence);
            canonical(&mut existing.observations);
            canonical(&mut existing.relationships);
        } else {
            merged.insert(key, finding);
        }
    }
    let mut findings: Vec<_> = merged.into_values().collect();
    canonical(&mut findings);
    let mut residuals = residuals;
    canonical(&mut residuals);
    let status = if !findings.is_empty() {
        VerificationStatus::Degraded
    } else if !blockers.is_empty() {
        VerificationStatus::Unknown
    } else if !residuals.is_empty() {
        VerificationStatus::HealthyWithExpectedResiduals
    } else {
        VerificationStatus::Healthy
    };
    IntegrityReport {
        context: g.context().clone(),
        proof_context: c,
        verification_status: status,
        findings,
        blockers,
        residuals,
        graph_diagnostics: g.diagnostics().to_vec(),
        rules: g.rules().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (ArtifactGraph, IntegrityContext, ArtifactId, GraphEvidence) {
        let ev = GraphEvidence {
            observation: Evidence {
                source: EvidenceSource::Fixture,
                confidence: Confidence::Authoritative,
                provider_version: Some("1".into()),
                rule: "rule".into(),
            },
            rule_id: EntityId::new("rule").unwrap(),
            nature: EvidenceNature::Observation,
            locator: None,
            observed_at: None,
        };
        let context = GraphContext {
            provider: Provider {
                kind: ProviderKind::Codex,
                presence: Presence::StateOnly,
                state_root: "/synthetic".into(),
                fingerprint: ProviderFingerprint {
                    executable: None,
                    executable_version: Some("1".into()),
                    adapter_version: "adapter".into(),
                    interface_id: Some("interface".into()),
                    layout_id: Some("layout".into()),
                },
                capabilities: ProviderCapabilities::new(),
                compatibility: CompatibilityProfile {
                    status: CompatibilityStatus::ReadOnlyKnown,
                    interface_stability: InterfaceStability::OfficialStable,
                    reason: String::new(),
                },
                installations: vec![],
                evidence_status: EvidenceStatus::SyntheticOnly,
                diagnostics: vec![],
            },
            evidence_status: EvidenceStatus::SyntheticOnly,
            completeness: Completeness::Complete,
            coverage: "synthetic complete".into(),
            diagnostics: vec![],
        };
        let mut b = ArtifactGraph::builder(context);
        b.add_rule(EvidenceRule {
            id: ev.rule_id.clone(),
            provider: ProviderKind::Codex,
            description: "synthetic".into(),
            admissible_sources: vec![EvidenceSource::Fixture],
            max_confidence: Confidence::Authoritative,
            adapter_version: "adapter".into(),
            interface_id: Some("interface".into()),
            layout_id: Some("layout".into()),
        })
        .unwrap();
        let id = b.observe(NodeObservation::Artifact(Artifact {
            id: EntityId::new("a").unwrap(),
            kind: ArtifactKind::Plan,
            locator: "/synthetic/a".into(),
            size_bytes: None,
            evidence: ev.observation.clone(),
        }));
        b.add_node_evidence(&id, ev.clone()).unwrap();
        b.add_edge(ReferenceEdge {
            source: id.clone(),
            target: id.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev.clone()],
        })
        .unwrap();
        let c = IntegrityContext {
            contract: Some(IntegrityContract {
                id: EntityId::new("contract").unwrap(),
                provider_version: "1".into(),
                adapter_version: "adapter".into(),
                interface_id: Some("interface".into()),
                layout_id: Some("layout".into()),
                coherent: true,
                evidence: vec![ev.clone()],
            }),
            node_coverage: vec![NodeCoverage {
                class: GraphNodeKind::Artifact,
                completeness: Completeness::Complete,
                evidence: vec![ev.clone()],
            }],
            relationship_coverage: vec![RelationshipCoverage {
                source_class: GraphNodeKind::Artifact,
                target_class: GraphNodeKind::Artifact,
                kind: EdgeKind::References,
                completeness: Completeness::Complete,
                evidence: vec![ev.clone()],
            }],
            required_targets: vec![RequiredTarget {
                source: id.clone(),
                target: id.clone(),
                edge_kind: EdgeKind::References,
                kind: RequiredTargetKind::Reference,
                evidence: vec![ev.clone()],
            }],
            ..Default::default()
        };
        (b.finish().unwrap(), c, id, ev)
    }
    #[test]
    fn healthy_and_explicit_residuals_require_closed_contract() {
        let (g, mut c, id, ev) = setup();
        assert_eq!(
            evaluate_integrity(&g, &c).verification_status,
            VerificationStatus::Healthy
        );
        c.residuals.push(ResidualObservation {
            artifact: id.clone(),
            expectation_evidence: vec![ev],
        });
        assert_eq!(
            evaluate_integrity(&g, &c).verification_status,
            VerificationStatus::HealthyWithExpectedResiduals
        );
        c.residuals[0].expectation_evidence.clear();
        let report = evaluate_integrity(&g, &c);
        assert_eq!(report.verification_status, VerificationStatus::Unknown);
        assert_eq!(
            report.residuals[0].classification,
            ResidualClassification::UnknownResidual
        );
        assert_eq!(
            evaluate_integrity(&g, &IntegrityContext::default()).verification_status,
            VerificationStatus::Unknown
        );
    }
    #[test]
    fn coverage_without_checks_and_unverified_positive_are_unknown() {
        let (g, mut c, _, _) = setup();
        c.required_targets.clear();
        assert_eq!(
            evaluate_integrity(&g, &c).verification_status,
            VerificationStatus::Unknown
        );
        let (_, mut c, _, _) = setup();
        c.required_targets[0].evidence.clear();
        let report = evaluate_integrity(&g, &c);
        assert!(report.findings.is_empty());
        assert_eq!(report.verification_status, VerificationStatus::Unknown);
    }
    #[test]
    fn session_aliases_do_not_create_cross_identity_conflicts() {
        let (g, _, _, ev) = setup();
        let mut b = ArtifactGraph::builder(g.context().clone());
        b.add_rule(g.rules()[0].clone()).unwrap();
        for local in ["namespace-a", "namespace-b"] {
            let id = b.observe(NodeObservation::Session(Session {
                id: EntityId::new(local).unwrap(),
                provider_session_id: "same-local-id".into(),
                project_id: None,
                state: SessionState::Archived,
                last_active: None,
                evidence: ev.observation.clone(),
            }));
            b.add_node_evidence(&id, ev.clone()).unwrap();
        }
        let report = evaluate_integrity(&b.finish().unwrap(), &IntegrityContext::default());
        assert!(report.findings.is_empty());
    }
    #[test]
    fn incompatible_target_expectation_blocks_even_when_present() {
        let (g, mut c, id, ev) = setup();
        c.required_targets.push(RequiredTarget {
            source: id.clone(),
            target: id,
            edge_kind: EdgeKind::References,
            kind: RequiredTargetKind::Child,
            evidence: vec![ev],
        });
        let report = evaluate_integrity(&g, &c);
        assert!(report.findings.is_empty());
        assert_eq!(report.verification_status, VerificationStatus::Unknown);
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.code == IntegrityBlockerCode::UnmetPrerequisite)
        );
    }
    #[test]
    fn present_wrong_artifact_kind_cannot_satisfy_transcript() {
        let (g, mut c, _, _) = setup();
        c.required_targets[0].kind = RequiredTargetKind::Transcript;
        let report = evaluate_integrity(&g, &c);
        assert!(report.findings.is_empty());
        assert_eq!(report.verification_status, VerificationStatus::Unknown);
    }
    #[test]
    fn residual_declarations_merge_conservatively() {
        let (g, mut c, id, ev) = setup();
        c.residuals.push(ResidualObservation {
            artifact: id.clone(),
            expectation_evidence: vec![ev.clone()],
        });
        let mut second = ev.clone();
        second.locator = Some("/second".into());
        c.residuals.push(ResidualObservation {
            artifact: id.clone(),
            expectation_evidence: vec![second],
        });
        let report = evaluate_integrity(&g, &c);
        assert_eq!(report.residuals.len(), 1);
        assert_eq!(
            report.residuals[0].classification,
            ResidualClassification::ExpectedProviderResidual
        );
        assert_eq!(report.residuals[0].evidence.len(), 2);
        c.residuals.push(ResidualObservation {
            artifact: id,
            expectation_evidence: vec![],
        });
        let report = evaluate_integrity(&g, &c);
        assert_eq!(report.residuals.len(), 1);
        assert_eq!(
            report.residuals[0].classification,
            ResidualClassification::UnknownResidual
        );
        assert_eq!(report.residuals[0].evidence.len(), 2);
    }
    #[test]
    fn suspicious_session_conflict_has_error_severity() {
        let (g, _, _, ev) = setup();
        let mut b = ArtifactGraph::builder(g.context().clone());
        b.add_rule(g.rules()[0].clone()).unwrap();
        for provider_id in ["one", "two"] {
            let id = b.observe(NodeObservation::Session(Session {
                id: EntityId::new("session").unwrap(),
                provider_session_id: provider_id.into(),
                project_id: None,
                state: SessionState::Archived,
                last_active: None,
                evidence: ev.observation.clone(),
            }));
            b.add_node_evidence(&id, ev.clone()).unwrap();
        }
        let report = evaluate_integrity(&b.finish().unwrap(), &IntegrityContext::default());
        assert_eq!(
            report.findings[0].kind,
            IntegrityIssueKind::DuplicateSessionIdentifier
        );
        assert_eq!(
            report.findings[0].disposition,
            IssueDisposition::SuspiciousInconsistency
        );
        assert_eq!(report.findings[0].severity, IntegritySeverity::Error);
    }
    #[test]
    fn owner_identity_collision_blocks_absence_and_owner_confirmation() {
        let (g, mut c, plan, ev) = setup();
        let mut b = ArtifactGraph::builder(g.context().clone());
        b.add_rule(g.rules()[0].clone()).unwrap();
        let observed_plan = b.observe(g.node(&plan).unwrap().observations[0].clone());
        b.add_node_evidence(&observed_plan, ev.clone()).unwrap();
        let owner = b.observe(NodeObservation::Session(Session {
            id: EntityId::new("owner").unwrap(),
            provider_session_id: "owner".into(),
            project_id: None,
            state: SessionState::Archived,
            last_active: None,
            evidence: ev.observation.clone(),
        }));
        b.add_node_evidence(&owner, ev.clone()).unwrap();
        let collision = b.observe(NodeObservation::Artifact(Artifact {
            id: EntityId::new("owner").unwrap(),
            kind: ArtifactKind::Plan,
            locator: "/owner".into(),
            size_bytes: None,
            evidence: ev.observation.clone(),
        }));
        b.add_node_evidence(&collision, ev.clone()).unwrap();
        b.add_edge(ReferenceEdge {
            source: owner,
            target: plan.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev.clone()],
        })
        .unwrap();
        c.required_targets.clear();
        c.node_coverage.push(NodeCoverage {
            class: GraphNodeKind::Session,
            completeness: Completeness::Complete,
            evidence: vec![ev.clone()],
        });
        c.relationship_coverage.push(RelationshipCoverage {
            source_class: GraphNodeKind::Session,
            target_class: GraphNodeKind::Artifact,
            kind: EdgeKind::References,
            completeness: Completeness::Complete,
            evidence: vec![ev.clone()],
        });
        c.required_owners.push(RequiredOwner {
            artifact: plan,
            relations: vec![OwnerRelation {
                owner_class: GraphNodeKind::Session,
                edge_kind: EdgeKind::References,
                direction: OwnerDirection::OwnerToArtifact,
                rule_ids: vec![ev.rule_id.clone()],
            }],
            evidence: vec![ev],
        });
        let report = evaluate_integrity(&b.finish().unwrap(), &c);
        assert!(report.findings.is_empty());
        assert_eq!(report.verification_status, VerificationStatus::Unknown);
    }
    #[test]
    fn incomplete_owner_namespace_cannot_prove_orphan() {
        let (g, mut c, id, ev) = setup();
        c.node_coverage.push(NodeCoverage {
            class: GraphNodeKind::Session,
            completeness: Completeness::Complete,
            evidence: vec![ev.clone()],
        });
        c.relationship_coverage.push(RelationshipCoverage {
            source_class: GraphNodeKind::Session,
            target_class: GraphNodeKind::Artifact,
            kind: EdgeKind::References,
            completeness: Completeness::Complete,
            evidence: vec![ev.clone()],
        });
        c.required_owners.push(RequiredOwner {
            artifact: id,
            relations: vec![OwnerRelation {
                owner_class: GraphNodeKind::Session,
                edge_kind: EdgeKind::References,
                direction: OwnerDirection::OwnerToArtifact,
                rule_ids: vec![ev.rule_id.clone()],
            }],
            evidence: vec![ev],
        });
        // The admitted rule on a wrong-class endpoint blocks absence.
        assert!(evaluate_integrity(&g, &c).findings.is_empty());
        let mut wire = serde_json::to_value(&g).unwrap();
        wire["edges"] = serde_json::json!([]);
        let g: ArtifactGraph = serde_json::from_value(wire).unwrap();
        c.required_targets.clear();
        let report = evaluate_integrity(&g, &c);
        assert_eq!(report.findings[0].kind, IntegrityIssueKind::OrphanPlan);
        assert_eq!(report.findings[0].severity, IntegritySeverity::Warning);
        let mut duplicate = c.required_owners[0].clone();
        duplicate.evidence[0].locator = Some("/another-observation".into());
        c.required_owners.push(duplicate);
        let merged = evaluate_integrity(&g, &c);
        assert_eq!(merged.findings.len(), 1);
        assert!(
            merged.findings[0]
                .evidence
                .iter()
                .any(|e| e.evidence.locator.is_some())
        );
        c.required_owners[1].relations[0].rule_ids = vec![EntityId::new("different-rule").unwrap()];
        let conflicting = evaluate_integrity(&g, &c);
        assert!(conflicting.findings.is_empty());
        assert!(
            conflicting
                .blockers
                .iter()
                .any(|b| b.code == IntegrityBlockerCode::ConflictingEvidence)
        );
        c.required_owners.pop();
        c.residuals.push(ResidualObservation {
            artifact: c.required_owners[0].artifact.clone(),
            expectation_evidence: c.required_owners[0].evidence.clone(),
        });
        assert_eq!(
            evaluate_integrity(&g, &c).residuals[0].classification,
            ResidualClassification::UnknownResidual
        );
        c.relationship_coverage.last_mut().unwrap().completeness = Completeness::Partial;
        assert!(evaluate_integrity(&g, &c).findings.is_empty());
    }
}
