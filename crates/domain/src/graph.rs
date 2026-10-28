//! Pure, evidence-preserving graph reconstructed from a discovery snapshot.
use crate::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphContext {
    pub provider: Provider,
    pub evidence_status: EvidenceStatus,
    pub completeness: Completeness,
    pub coverage: String,
    pub diagnostics: Vec<Diagnostic>,
}
impl From<&DiscoverySnapshot> for GraphContext {
    fn from(s: &DiscoverySnapshot) -> Self {
        Self {
            provider: s.provider.clone(),
            evidence_status: s.evidence_status,
            completeness: s.completeness,
            coverage: s.coverage.clone(),
            diagnostics: s.diagnostics.clone(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeKind {
    ProviderState,
    Workspace,
    Project,
    Session,
    Artifact,
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactId {
    pub provider: ProviderKind,
    #[serde(with = "crate::path_serde")]
    pub state_root: PathBuf,
    pub class: GraphNodeKind,
    pub local_id: EntityId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NodeObservation {
    ProviderState,
    Workspace(Workspace),
    Project(Project),
    Session(Session),
    Artifact(Artifact),
    Unresolved { id: EntityId },
}
impl NodeObservation {
    fn identity(&self) -> (GraphNodeKind, EntityId) {
        match self {
            Self::ProviderState => (
                GraphNodeKind::ProviderState,
                EntityId::new("provider-state").expect("constant identity"),
            ),
            Self::Workspace(x) => (GraphNodeKind::Workspace, x.id.clone()),
            Self::Project(x) => (GraphNodeKind::Project, x.id.clone()),
            Self::Session(x) => (GraphNodeKind::Session, x.id.clone()),
            Self::Artifact(x) => (GraphNodeKind::Artifact, x.id.clone()),
            Self::Unresolved { id } => (GraphNodeKind::Unresolved, id.clone()),
        }
    }
    fn raw_evidence(&self) -> Option<&Evidence> {
        match self {
            Self::Workspace(x) => Some(&x.evidence),
            Self::Project(x) => Some(&x.evidence),
            Self::Session(x) => Some(&x.evidence),
            Self::Artifact(x) => Some(&x.evidence),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalIdentity {
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactNode {
    pub id: ArtifactId,
    pub observations: Vec<NodeObservation>,
    pub physical_identity: PhysicalIdentity,
    pub evidence: Vec<GraphEvidence>,
}
impl ArtifactNode {
    pub const fn is_protected(&self) -> bool {
        true
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceNature {
    Observation,
    Inference,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEvidence {
    pub observation: Evidence,
    pub rule_id: EntityId,
    pub nature: EvidenceNature,
    #[serde(with = "crate::path_serde::optional")]
    pub locator: Option<PathBuf>,
    pub observed_at: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRule {
    pub id: EntityId,
    pub provider: ProviderKind,
    pub description: String,
    pub admissible_sources: Vec<EvidenceSource>,
    pub max_confidence: Confidence,
    pub adapter_version: String,
    pub interface_id: Option<String>,
    pub layout_id: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    References,
    SpawnedBy,
    MapsTo,
    AssociatedWith,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEdge {
    pub source: ArtifactId,
    pub target: ArtifactId,
    pub kind: EdgeKind,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootKind {
    ProviderGlobal,
    ActiveSession,
    ArchivedRetainedSession,
    UnknownState,
    UserProtected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedRoot {
    pub node: ArtifactId,
    pub kind: RootKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipClassification {
    Exclusive,
    Shared,
    Referenced,
    Derived,
    Orphaned,
    Global,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionReason {
    NoOwnershipContract,
    PartialCoverage,
    UnavailableCoverage,
    UnknownCompatibility,
    UnknownArtifact,
    UnresolvedEndpoint,
    AmbiguousIdentity,
    ConflictingEvidence,
    UnknownRule,
    InadmissibleSource,
    ProtectedRoot,
    SnapshotStale,
    ConcurrentChange,
    UnverifiedProducerVersion,
    RuleContextMismatch,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipAssessment {
    pub classification: OwnershipClassification,
    pub supporting_evidence: Vec<GraphEvidence>,
    pub blockers: Vec<ProtectionReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphDiagnostic {
    ConflictingNodeObservations { node: ArtifactId },
    ConflictingRuleDefinitions { rule: EntityId },
    ConflictingParentage { child: ArtifactId },
    UnrecognizedRule { rule: EntityId },
    InadmissibleSource { rule: EntityId },
    UnresolvedEndpoint { node: ArtifactId },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatedEvidence {
    pub evidence: GraphEvidence,
    pub confidence: Confidence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExplanation {
    pub node: ArtifactNode,
    pub incoming: Vec<ReferenceEdge>,
    pub outgoing: Vec<ReferenceEdge>,
    pub roots: Vec<ProtectedRoot>,
    pub ownership: OwnershipAssessment,
    pub evidence: Vec<EvaluatedEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    #[error("graph identity belongs to another provider or state root")]
    ForeignScope,
    #[error("observation does not match its graph identity")]
    MismatchedObservation,
    #[error("graph endpoint or root does not exist")]
    MissingNode,
    #[error("observation lacks matching raw evidence")]
    MissingEvidence,
    #[error("edge requires evidence")]
    EmptyEdgeEvidence,
    #[error("rule belongs to another provider")]
    ForeignRule,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtifactGraph {
    context: GraphContext,
    nodes: Vec<ArtifactNode>,
    edges: Vec<ReferenceEdge>,
    roots: Vec<ProtectedRoot>,
    rules: Vec<EvidenceRule>,
    diagnostics: Vec<GraphDiagnostic>,
}
#[derive(Deserialize)]
struct GraphWire {
    context: GraphContext,
    nodes: Vec<ArtifactNode>,
    edges: Vec<ReferenceEdge>,
    roots: Vec<ProtectedRoot>,
    rules: Vec<EvidenceRule>,
}
impl<'de> Deserialize<'de> for ArtifactGraph {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = GraphWire::deserialize(d)?;
        let mut b = Self::builder(w.context);
        for n in w.nodes {
            if n.observations.is_empty() {
                return Err(serde::de::Error::custom(GraphError::MissingNode));
            }
            b.check_scope(&n.id).map_err(serde::de::Error::custom)?;
            for o in n.observations {
                if b.id_for(&o) != n.id {
                    return Err(serde::de::Error::custom(GraphError::MismatchedObservation));
                }
                b.observe(o);
            }
            for e in n.evidence {
                b.add_node_evidence(&n.id, e)
                    .map_err(serde::de::Error::custom)?;
            }
        }
        for r in w.rules {
            b.add_rule(r).map_err(serde::de::Error::custom)?;
        }
        for e in w.edges {
            b.add_edge(e).map_err(serde::de::Error::custom)?;
        }
        for r in w.roots {
            b.add_root(r).map_err(serde::de::Error::custom)?;
        }
        b.finish().map_err(serde::de::Error::custom)
    }
}
fn canonical<T: Serialize>(v: &mut Vec<T>) {
    // Use the full lossless fact key for equality as well as ordering. PathBuf's
    // equality can normalize distinct raw locator spellings such as /a/./b.
    let mut keyed: Vec<_> = std::mem::take(v)
        .into_iter()
        .map(|value| {
            (
                serde_json::to_string(&value).expect("domain values serialize"),
                value,
            )
        })
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.dedup_by(|a, b| a.0 == b.0);
    *v = keyed.into_iter().map(|(_, value)| value).collect();
}
#[derive(Debug)]
pub struct GraphBuilder {
    context: GraphContext,
    nodes: BTreeMap<ArtifactId, ArtifactNode>,
    edges: Vec<ReferenceEdge>,
    roots: Vec<ProtectedRoot>,
    rules: Vec<EvidenceRule>,
}
impl GraphBuilder {
    fn id_for(&self, o: &NodeObservation) -> ArtifactId {
        let (class, local_id) = o.identity();
        ArtifactId {
            provider: self.context.provider.kind,
            state_root: self.context.provider.state_root.clone(),
            class,
            local_id,
        }
    }
    fn check_scope(&self, id: &ArtifactId) -> Result<(), GraphError> {
        if id.provider != self.context.provider.kind
            || id.state_root != self.context.provider.state_root
        {
            Err(GraphError::ForeignScope)
        } else {
            Ok(())
        }
    }
    pub fn observe(&mut self, o: NodeObservation) -> ArtifactId {
        let id = self.id_for(&o);
        self.nodes
            .entry(id.clone())
            .or_insert_with(|| ArtifactNode {
                id: id.clone(),
                observations: Vec::new(),
                physical_identity: PhysicalIdentity::Unavailable,
                evidence: Vec::new(),
            })
            .observations
            .push(o);
        id
    }
    pub fn add_node_evidence(
        &mut self,
        id: &ArtifactId,
        e: GraphEvidence,
    ) -> Result<(), GraphError> {
        self.check_scope(id)?;
        self.nodes
            .entry(id.clone())
            .or_insert_with(|| ArtifactNode {
                id: id.clone(),
                observations: Vec::new(),
                physical_identity: PhysicalIdentity::Unavailable,
                evidence: Vec::new(),
            })
            .evidence
            .push(e);
        Ok(())
    }
    pub fn add_rule(&mut self, r: EvidenceRule) -> Result<(), GraphError> {
        if r.provider != self.context.provider.kind {
            return Err(GraphError::ForeignRule);
        }
        self.rules.push(r);
        Ok(())
    }
    pub fn add_edge(&mut self, e: ReferenceEdge) -> Result<(), GraphError> {
        self.check_scope(&e.source)?;
        self.check_scope(&e.target)?;
        self.edges.push(e);
        Ok(())
    }
    pub fn add_root(&mut self, r: ProtectedRoot) -> Result<(), GraphError> {
        self.check_scope(&r.node)?;
        self.roots.push(r);
        Ok(())
    }
    pub fn finish(mut self) -> Result<ArtifactGraph, GraphError> {
        canonical(&mut self.context.diagnostics);
        canonical(&mut self.context.provider.diagnostics);
        canonical(&mut self.context.provider.installations);
        for n in self.nodes.values_mut() {
            if n.observations.is_empty() {
                return Err(GraphError::MissingNode);
            }
            canonical(&mut n.observations);
            canonical(&mut n.evidence);
            for o in &n.observations {
                if let NodeObservation::Workspace(w) = o
                    && w.provider != n.id.provider
                {
                    return Err(GraphError::ForeignScope);
                }
                if let Some(e) = o.raw_evidence()
                    && !n.evidence.iter().any(|g| &g.observation == e)
                {
                    return Err(GraphError::MissingEvidence);
                }
            }
        }
        for e in &self.edges {
            if e.evidence.is_empty() {
                return Err(GraphError::EmptyEdgeEvidence);
            }
            if !self.nodes.contains_key(&e.source) || !self.nodes.contains_key(&e.target) {
                return Err(GraphError::MissingNode);
            }
        }
        for r in &self.roots {
            if !self.nodes.contains_key(&r.node) {
                return Err(GraphError::MissingNode);
            }
        }
        for r in &mut self.rules {
            canonical(&mut r.admissible_sources);
        }
        canonical(&mut self.rules);
        canonical(&mut self.roots);
        let mut edges: BTreeMap<(ArtifactId, ArtifactId, EdgeKind), Vec<GraphEvidence>> =
            BTreeMap::new();
        for e in self.edges {
            edges
                .entry((e.source, e.target, e.kind))
                .or_default()
                .extend(e.evidence);
        }
        let edges = edges
            .into_iter()
            .map(|((source, target, kind), mut evidence)| {
                canonical(&mut evidence);
                ReferenceEdge {
                    source,
                    target,
                    kind,
                    evidence,
                }
            })
            .collect();
        let mut graph = ArtifactGraph {
            context: self.context,
            nodes: self.nodes.into_values().collect(),
            edges,
            roots: self.roots,
            rules: self.rules,
            diagnostics: Vec::new(),
        };
        for n in &graph.nodes {
            if n.observations.len() > 1 {
                graph
                    .diagnostics
                    .push(GraphDiagnostic::ConflictingNodeObservations { node: n.id.clone() });
            }
            if n.id.class == GraphNodeKind::Unresolved {
                graph
                    .diagnostics
                    .push(GraphDiagnostic::UnresolvedEndpoint { node: n.id.clone() });
            }
        }
        let mut counts = BTreeMap::new();
        for r in &graph.rules {
            *counts.entry(r.id.clone()).or_insert(0usize) += 1;
        }
        for (id, count) in counts {
            if count > 1 {
                graph
                    .diagnostics
                    .push(GraphDiagnostic::ConflictingRuleDefinitions { rule: id });
            }
        }
        // SpawnedBy describes one immediate spawning parent. Keep every claim
        // when different targets disagree; none becomes a resolved conclusion.
        let mut parents: BTreeMap<ArtifactId, BTreeSet<ArtifactId>> = BTreeMap::new();
        for edge in &graph.edges {
            if edge.kind == EdgeKind::SpawnedBy {
                parents
                    .entry(edge.source.clone())
                    .or_default()
                    .insert(edge.target.clone());
            }
        }
        for (child, targets) in parents {
            if targets.len() > 1 {
                graph
                    .diagnostics
                    .push(GraphDiagnostic::ConflictingParentage { child });
            }
        }
        for e in graph
            .nodes
            .iter()
            .flat_map(|n| n.evidence.iter())
            .chain(graph.edges.iter().flat_map(|e| e.evidence.iter()))
        {
            let matching: Vec<_> = graph.rules.iter().filter(|r| r.id == e.rule_id).collect();
            if matching.is_empty() {
                graph.diagnostics.push(GraphDiagnostic::UnrecognizedRule {
                    rule: e.rule_id.clone(),
                });
            } else if matching
                .iter()
                .any(|r| !r.admissible_sources.contains(&e.observation.source))
            {
                graph.diagnostics.push(GraphDiagnostic::InadmissibleSource {
                    rule: e.rule_id.clone(),
                });
            }
        }
        canonical(&mut graph.diagnostics);
        Ok(graph)
    }
}
impl ArtifactGraph {
    pub fn builder(context: GraphContext) -> GraphBuilder {
        GraphBuilder {
            context,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            roots: Vec::new(),
            rules: Vec::new(),
        }
    }
    pub fn context(&self) -> &GraphContext {
        &self.context
    }
    pub fn nodes(&self) -> &[ArtifactNode] {
        &self.nodes
    }
    pub fn edges(&self) -> &[ReferenceEdge] {
        &self.edges
    }
    pub fn roots(&self) -> &[ProtectedRoot] {
        &self.roots
    }
    pub fn rules(&self) -> &[EvidenceRule] {
        &self.rules
    }
    pub fn diagnostics(&self) -> &[GraphDiagnostic] {
        &self.diagnostics
    }
    pub fn node(&self, id: &ArtifactId) -> Option<&ArtifactNode> {
        self.nodes
            .binary_search_by(|n| n.id.cmp(id))
            .ok()
            .map(|i| &self.nodes[i])
    }
    pub fn effective_confidence(&self, e: &GraphEvidence) -> Confidence {
        if self.context.provider.compatibility.status != CompatibilityStatus::ReadOnlyKnown {
            return Confidence::Unknown;
        }
        let mut rules = self.rules.iter().filter(|r| r.id == e.rule_id);
        let Some(r) = rules.next() else {
            return Confidence::Unknown;
        };
        if rules.next().is_some()
            || !r.admissible_sources.contains(&e.observation.source)
            || r.adapter_version != self.context.provider.fingerprint.adapter_version
            || r.interface_id != self.context.provider.fingerprint.interface_id
            || r.layout_id != self.context.provider.fingerprint.layout_id
        {
            return Confidence::Unknown;
        }
        use Confidence::{Authoritative, Heuristic, Strong, Unknown};
        let bounded = match (e.observation.confidence, r.max_confidence) {
            (Unknown, _) | (_, Unknown) => Unknown,
            (Heuristic, _) | (_, Heuristic) => Heuristic,
            (Strong, _) | (_, Strong) => Strong,
            (Authoritative, Authoritative) => Authoritative,
        };
        match (bounded, e.nature) {
            (Strong | Authoritative, EvidenceNature::Inference) => Heuristic,
            _ => bounded,
        }
    }
    pub fn ownership_assessment(&self, id: &ArtifactId) -> Result<OwnershipAssessment, GraphError> {
        let node = self.node(id).ok_or(GraphError::MissingNode)?;
        let mut blockers = vec![ProtectionReason::NoOwnershipContract];
        let mut uncertain = false;
        match self.context.completeness {
            Completeness::Complete => (),
            Completeness::Partial => blockers.push(ProtectionReason::PartialCoverage),
            Completeness::Unavailable => blockers.push(ProtectionReason::UnavailableCoverage),
        }
        if self.context.provider.compatibility.status != CompatibilityStatus::ReadOnlyKnown {
            blockers.push(ProtectionReason::UnknownCompatibility);
            uncertain = true;
        }
        for d in self
            .context
            .diagnostics
            .iter()
            .chain(self.context.provider.diagnostics.iter())
        {
            match d.code {
                DiagnosticCode::SnapshotStale => {
                    blockers.push(ProtectionReason::SnapshotStale);
                    uncertain = true;
                }
                DiagnosticCode::ConcurrentChange => {
                    blockers.push(ProtectionReason::ConcurrentChange);
                    uncertain = true;
                }
                _ => (),
            }
        }
        if node.observations.len() > 1 {
            blockers.push(ProtectionReason::ConflictingEvidence);
            uncertain = true;
        }
        for diagnostic in &self.diagnostics {
            if let GraphDiagnostic::ConflictingParentage { child } = diagnostic
                && (child == id
                    || self.edges.iter().any(|edge| {
                        edge.kind == EdgeKind::SpawnedBy
                            && &edge.source == child
                            && &edge.target == id
                    }))
            {
                blockers.push(ProtectionReason::ConflictingEvidence);
                uncertain = true;
            }
        }
        if id.class == GraphNodeKind::Unresolved {
            blockers.push(ProtectionReason::UnresolvedEndpoint);
            uncertain = true;
        }
        if node
            .observations
            .iter()
            .any(|o| matches!(o,NodeObservation::Artifact(a) if a.kind==ArtifactKind::Unknown))
        {
            blockers.push(ProtectionReason::UnknownArtifact);
            uncertain = true;
        }
        let own_roots: Vec<_> = self.roots.iter().filter(|r| &r.node == id).collect();
        if !own_roots.is_empty() {
            blockers.push(ProtectionReason::ProtectedRoot);
        }
        let mut supporting = Vec::new();
        let mut referrers = BTreeSet::new();
        for edge in self
            .edges
            .iter()
            .filter(|e| &e.target == id && e.kind != EdgeKind::AssociatedWith)
        {
            if edge.source.class == GraphNodeKind::Unresolved {
                blockers.push(ProtectionReason::UnresolvedEndpoint);
                uncertain = true;
            }
            let source_valid = self.node(&edge.source).is_some_and(|n| {
                n.observations.len() == 1 && n.id.class != GraphNodeKind::Unresolved
                    && !n.observations.iter().any(|o| matches!(o, NodeObservation::Artifact(a) if a.kind == ArtifactKind::Unknown))
            });
            if self
                .node(&edge.source)
                .is_some_and(|source| source.observations.len() > 1)
            {
                blockers.push(ProtectionReason::ConflictingEvidence);
                uncertain = true;
            }
            if source_valid && self.roots.iter().any(|r| r.node == edge.source) {
                for e in &edge.evidence {
                    if self.effective_confidence(e) != Confidence::Unknown {
                        referrers.insert(edge.source.clone());
                        supporting.push(e.clone());
                    }
                }
            }
        }
        for e in node.evidence.iter().chain(
            self.edges
                .iter()
                .filter(|x| &x.source == id || &x.target == id)
                .flat_map(|x| x.evidence.iter()),
        ) {
            if e.observation.provider_version.is_none() {
                blockers.push(ProtectionReason::UnverifiedProducerVersion);
            }
            let matching: Vec<_> = self.rules.iter().filter(|r| r.id == e.rule_id).collect();
            if matching.is_empty() {
                blockers.push(ProtectionReason::UnknownRule);
            } else if matching.len() > 1 {
                blockers.push(ProtectionReason::ConflictingEvidence);
                uncertain = true;
            } else if !matching[0]
                .admissible_sources
                .contains(&e.observation.source)
            {
                blockers.push(ProtectionReason::InadmissibleSource);
            } else if matching[0].adapter_version
                != self.context.provider.fingerprint.adapter_version
                || matching[0].interface_id != self.context.provider.fingerprint.interface_id
                || matching[0].layout_id != self.context.provider.fingerprint.layout_id
            {
                blockers.push(ProtectionReason::RuleContextMismatch);
            }
        }
        canonical(&mut blockers);
        canonical(&mut supporting);
        let classification = if own_roots.iter().any(|r| r.kind == RootKind::ProviderGlobal) {
            OwnershipClassification::Global
        } else if uncertain {
            OwnershipClassification::Unknown
        } else if referrers.len() >= 2 {
            OwnershipClassification::Shared
        } else if referrers.len() == 1 {
            OwnershipClassification::Referenced
        } else {
            OwnershipClassification::Unknown
        };
        Ok(OwnershipAssessment {
            classification,
            supporting_evidence: supporting,
            blockers,
        })
    }
    pub fn explain(&self, id: &ArtifactId) -> Result<NodeExplanation, GraphError> {
        let node = self.node(id).ok_or(GraphError::MissingNode)?.clone();
        let incoming: Vec<_> = self
            .edges
            .iter()
            .filter(|e| &e.target == id)
            .cloned()
            .collect();
        let outgoing: Vec<_> = self
            .edges
            .iter()
            .filter(|e| &e.source == id)
            .cloned()
            .collect();
        let roots = self
            .roots
            .iter()
            .filter(|r| &r.node == id)
            .cloned()
            .collect();
        let mut evidence: Vec<_> = node
            .evidence
            .iter()
            .chain(
                incoming
                    .iter()
                    .chain(outgoing.iter())
                    .flat_map(|e| e.evidence.iter()),
            )
            .map(|e| EvaluatedEvidence {
                evidence: e.clone(),
                confidence: self.effective_confidence(e),
            })
            .collect();
        canonical(&mut evidence);
        Ok(NodeExplanation {
            node,
            incoming,
            outgoing,
            roots,
            ownership: self.ownership_assessment(id)?,
            evidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> GraphContext {
        GraphContext {
            provider: Provider {
                kind: ProviderKind::Codex,
                presence: Presence::StateOnly,
                state_root: "/state".into(),
                fingerprint: ProviderFingerprint {
                    executable: None,
                    executable_version: None,
                    adapter_version: "1".into(),
                    interface_id: None,
                    layout_id: None,
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
            coverage: "test".into(),
            diagnostics: vec![],
        }
    }
    fn evidence() -> GraphEvidence {
        GraphEvidence {
            observation: Evidence {
                source: EvidenceSource::Fixture,
                confidence: Confidence::Authoritative,
                provider_version: None,
                rule: "r".into(),
            },
            rule_id: EntityId::new("r").unwrap(),
            nature: EvidenceNature::Observation,
            locator: None,
            observed_at: None,
        }
    }
    fn rule() -> EvidenceRule {
        EvidenceRule {
            id: EntityId::new("r").unwrap(),
            provider: ProviderKind::Codex,
            description: "fixture".into(),
            admissible_sources: vec![EvidenceSource::Fixture],
            max_confidence: Confidence::Heuristic,
            adapter_version: "1".into(),
            interface_id: None,
            layout_id: None,
        }
    }
    #[test]
    fn retained_referrers_produce_descriptive_classifications_and_round_trip() {
        let mut b = ArtifactGraph::builder(context());
        let ev = evidence();
        b.add_rule(rule()).unwrap();
        let target = b.observe(NodeObservation::Artifact(Artifact {
            id: EntityId::new("a").unwrap(),
            kind: ArtifactKind::Plan,
            locator: "/a".into(),
            size_bytes: None,
            evidence: ev.observation.clone(),
        }));
        b.add_node_evidence(&target, ev.clone()).unwrap();
        let roots: Vec<_> = (0..2)
            .map(|i| {
                let id = b.observe(NodeObservation::Session(Session {
                    id: EntityId::new(format!("s{i}")).unwrap(),
                    provider_session_id: format!("s{i}"),
                    project_id: None,
                    state: SessionState::Archived,
                    last_active: None,
                    evidence: ev.observation.clone(),
                }));
                b.add_node_evidence(&id, ev.clone()).unwrap();
                b.add_root(ProtectedRoot {
                    node: id.clone(),
                    kind: RootKind::ArchivedRetainedSession,
                })
                .unwrap();
                id
            })
            .collect();
        b.add_edge(ReferenceEdge {
            source: roots[0].clone(),
            target: target.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev.clone()],
        })
        .unwrap();
        let single = b.finish().unwrap();
        assert_eq!(
            single.ownership_assessment(&target).unwrap().classification,
            OwnershipClassification::Referenced
        );
        assert_eq!(single.effective_confidence(&ev), Confidence::Heuristic);
        assert!(single.node(&target).unwrap().is_protected());
        let mut wire = serde_json::to_value(&single).unwrap();
        let extra = ReferenceEdge {
            source: roots[1].clone(),
            target: target.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev],
        };
        wire["edges"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(extra).unwrap());
        wire["diagnostics"] = serde_json::json!([{"kind":"unrecognized_rule","rule":"fake"}]);
        let shared: ArtifactGraph = serde_json::from_value(wire).unwrap();
        assert_eq!(
            shared.ownership_assessment(&target).unwrap().classification,
            OwnershipClassification::Shared
        );
        assert!(shared.diagnostics().is_empty());
        let round: ArtifactGraph =
            serde_json::from_str(&serde_json::to_string(&shared).unwrap()).unwrap();
        assert_eq!(shared, round);
    }
    #[test]
    fn missing_raw_evidence_and_malformed_import_are_rejected() {
        let mut b = ArtifactGraph::builder(context());
        let id = b.observe(NodeObservation::Artifact(Artifact {
            id: EntityId::new("a").unwrap(),
            kind: ArtifactKind::Plan,
            locator: "/a".into(),
            size_bytes: None,
            evidence: evidence().observation,
        }));
        assert_eq!(b.finish().unwrap_err(), GraphError::MissingEvidence);
        let mut b = ArtifactGraph::builder(context());
        b.observe(NodeObservation::ProviderState);
        let g = b.finish().unwrap();
        let mut wire = serde_json::to_value(g).unwrap();
        wire["nodes"][0]["id"] = serde_json::to_value(id).unwrap();
        assert!(serde_json::from_value::<ArtifactGraph>(wire).is_err());
    }
}
