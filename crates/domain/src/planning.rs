//! Immutable, evidence-bound intentions. A plan is neither authorization nor execution.
use crate::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleIntent {
    Archive,
    Unarchive,
    SoftDelete,
    Purge,
    Repair,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeLifecycleOperation {
    ArchiveSession,
    UnarchiveSession,
    DeleteSession,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeEffectScope {
    TargetOnly,
    TargetAndAttemptedDescendants,
    TargetAndDescendants,
    TargetRestoreWithProviderManagedState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleCapability {
    pub provider: ProviderKind,
    pub fingerprint: ProviderFingerprint,
    pub operation: NativeLifecycleOperation,
    pub protocol_evidence_digest: String,
    pub effect_scope: NativeEffectScope,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeEffectBoundary {
    pub contract_id: EntityId,
    pub capability: LifecycleCapability,
    pub effect_rule_ids: Vec<EntityId>,
    pub target: ArtifactId,
    pub operation: NativeLifecycleOperation,
    pub affected: Vec<ArtifactId>,
    pub completeness: Completeness,
    pub coherent: bool,
    pub evidence: Vec<GraphEvidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct PlanningContext {
    pub scope: DiscoveryScope,
    pub capabilities: Vec<LifecycleCapability>,
    pub effects: Option<NativeEffectBoundary>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleRequest {
    pub target: ArtifactId,
    pub intent: LifecycleIntent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Ready,
    Blocked,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedArtifactRole {
    Target,
    PotentialEffect,
    Retained,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedArtifact {
    pub id: ArtifactId,
    pub ownership: OwnershipAssessment,
    pub explicitly_protected: bool,
    pub role: PlannedArtifactRole,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanOperation {
    pub target: ArtifactId,
    pub native_id: EntityId,
    pub operation: NativeLifecycleOperation,
    pub affected: Vec<ArtifactId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanPrecondition {
    BoundStateUnchanged,
    ProviderProfileUnchanged,
    NativeIdentityUnchanged,
    CapabilityUnchanged,
    EffectBoundaryUnchanged,
    CompleteAuthoritativeScope,
    NoAdditionalReferences,
    OwnershipUnchanged,
    ProtectedRootsUnchanged,
    IntegrityUnchanged,
    FreshCoherentObservation,
    ImmediateRevalidation,
    IndependentApproval,
    RecoveryAvailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedPostcondition {
    NativeSessionArchived,
    NativeSessionUnarchived,
    NativeSessionDeleted,
    NoIndependentArtifactOperation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanBlocker {
    UnsupportedIntent,
    CapabilityUnavailable,
    TargetMissing,
    TargetNotNativeSession,
    ProtectedTarget,
    IncompleteDiscovery,
    FilteredScope,
    IntegrityMismatch,
    IntegrityUnverified,
    EffectBoundaryUnavailable,
    EffectBoundaryInvalid,
    EffectEvidenceUnverified,
    NativeIdentityUnverified,
    OwnershipNotExclusive,
    SnapshotStale,
    ConcurrentChange,
    ConflictingEvidence,
    UnknownCompatibility,
    EvidenceStatusUnverified,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateUnknownReason {
    IncompleteBoundary,
    MissingSize,
    ConflictingSizes,
    NotArtifact,
    Overflow,
    PhysicalIdentityUnavailable,
    ProtectedOwnership,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalSpaceEstimate {
    pub known_subtotal_bytes: u64,
    pub total_bytes: Option<u64>,
    pub unknown_reasons: Vec<EstimateUnknownReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalReclaimEstimate {
    pub bytes: Option<u64>,
    pub reason: EstimateUnknownReason,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpaceEstimate {
    pub logical: LogicalSpaceEstimate,
    pub physical: PhysicalReclaimEstimate,
}
macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                if s.len() != 71
                    || !s.starts_with("sha256:")
                    || !s[7..]
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(serde::de::Error::custom("invalid SHA-256 digest"));
                }
                Ok(Self(s))
            }
        }
    };
}
digest_type!(PlanDigest);
digest_type!(StateFingerprint);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanIntegritySummary {
    pub status: VerificationStatus,
    pub blockers: Vec<IntegrityBlockerCode>,
    pub findings: Vec<IntegrityIssueKind>,
    pub contract_id: Option<EntityId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanEffectContract {
    pub id: EntityId,
    pub affected: Vec<ArtifactId>,
    pub rule_ids: Vec<EntityId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanContent {
    pub schema_version: u32,
    pub request: LifecycleRequest,
    pub native_target_id: Option<EntityId>,
    pub observation_scope: DiscoveryScope,
    pub compatibility_status: CompatibilityStatus,
    pub evidence_status: EvidenceStatus,
    pub integrity: PlanIntegritySummary,
    pub effect_contract: Option<PlanEffectContract>,
    pub provider: ProviderFingerprint,
    pub capability: Option<LifecycleCapability>,
    pub status: PlanStatus,
    pub state_fingerprint: StateFingerprint,
    pub artifacts: Vec<PlannedArtifact>,
    pub operations: Vec<PlanOperation>,
    pub preconditions: Vec<PlanPrecondition>,
    pub postconditions: Vec<ExpectedPostcondition>,
    pub blockers: Vec<PlanBlocker>,
    pub space: SpaceEstimate,
    pub requires_human_approval: bool,
    pub recovery_required: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecyclePlan {
    content: PlanContent,
    digest: PlanDigest,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanWire {
    content: PlanContent,
    digest: PlanDigest,
}
impl<'de> Deserialize<'de> for LifecyclePlan {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut input = StrictValue::deserialize(d)?.0;
        let wire: PlanWire =
            serde_json::from_value(input.clone()).map_err(serde::de::Error::custom)?;
        let mut encoded = serde_json::to_value(&wire).map_err(serde::de::Error::custom)?;
        canonical(&mut input);
        canonical(&mut encoded);
        if input != encoded {
            return Err(serde::de::Error::custom(
                "unknown or noncanonical plan fields",
            ));
        }
        let c = &wire.content;
        let valid = c.schema_version == 1
            && c.requires_human_approval
            && c.recovery_required
            && match c.status {
                PlanStatus::Ready => {
                    c.blockers.is_empty()
                        && c.observation_scope.project_path.is_none()
                        && c.observation_scope.include_archived
                        && (1..=100_000).contains(&c.observation_scope.max_entries)
                        && c.compatibility_status == CompatibilityStatus::ReadOnlyKnown
                        && !matches!(
                            c.evidence_status,
                            EvidenceStatus::Unverified | EvidenceStatus::RealPartial
                        )
                        && matches!(
                            c.integrity.status,
                            VerificationStatus::Healthy
                                | VerificationStatus::HealthyWithExpectedResiduals
                        )
                        && c.integrity.blockers.is_empty()
                        && c.integrity.findings.is_empty()
                        && c.integrity.contract_id.is_some()
                        && c.effect_contract.as_ref().is_some_and(|e| {
                            !e.rule_ids.is_empty()
                                && e.rule_ids.iter().collect::<BTreeSet<_>>().len()
                                    == e.rule_ids.len()
                                && c.operations.first().is_some_and(|op| {
                                    projected(&e.affected, "plan")
                                        == projected(&op.affected, "plan")
                                })
                        })
                        && c.capability.is_some()
                        && c.operations.len() == 1
                        && c.operations[0].target == c.request.target
                        && c.native_target_id.as_ref() == Some(&c.operations[0].native_id)
                        && c.request.target.class == GraphNodeKind::Session
                        && c.capability.as_ref().is_some_and(|cap| {
                            cap.fingerprint == c.provider
                                && valid_scope(cap.operation, cap.effect_scope)
                                && valid_digest(&cap.protocol_evidence_digest)
                                && cap.provider == c.request.target.provider
                                && (cap.effect_scope != NativeEffectScope::TargetOnly
                                    || c.operations[0].affected.len() == 1)
                        })
                        && c.operations[0]
                            .affected
                            .iter()
                            .collect::<BTreeSet<_>>()
                            .len()
                            == c.operations[0].affected.len()
                        && c.artifacts
                            .iter()
                            .map(|a| &a.id)
                            .collect::<BTreeSet<_>>()
                            .len()
                            == c.artifacts.len()
                        && c.artifacts.iter().all(|a| {
                            let expected_role = if a.id == c.request.target {
                                PlannedArtifactRole::Target
                            } else if c.operations[0].affected.contains(&a.id) {
                                PlannedArtifactRole::PotentialEffect
                            } else {
                                PlannedArtifactRole::Retained
                            };
                            a.role == expected_role
                        })
                        && c.artifacts
                            .iter()
                            .filter(|a| {
                                a.role == PlannedArtifactRole::Target && a.id == c.request.target
                            })
                            .count()
                            == 1
                        && projected(&c.preconditions, "plan")
                            == projected(&required_preconditions(), "plan")
                        && projected(&c.postconditions, "plan")
                            == projected(&required_postconditions(c.request.intent), "plan")
                        && c.capability
                            .as_ref()
                            .is_some_and(|cap| cap.operation == c.operations[0].operation)
                        && operation(c.request.intent) == Some(c.operations[0].operation)
                        && c.operations[0].affected.contains(&c.request.target)
                        && c.operations[0].affected.iter().all(|id| {
                            c.artifacts.iter().any(|a| {
                                &a.id == id
                                    && !a.explicitly_protected
                                    && matches!(
                                        id.class,
                                        GraphNodeKind::Session | GraphNodeKind::Artifact
                                    )
                                    && id.provider == c.request.target.provider
                                    && id.state_root == c.request.target.state_root
                                    && (id == &c.request.target
                                        || a.role == PlannedArtifactRole::PotentialEffect)
                                    && ((c.request.intent != LifecycleIntent::Purge
                                        && id.class != GraphNodeKind::Artifact)
                                        || a.ownership.classification
                                            == OwnershipClassification::Exclusive)
                            })
                        })
                }
                _ => !c.blockers.is_empty() && c.operations.is_empty(),
            };
        if !valid || plan_hash(c) != wire.digest {
            return Err(serde::de::Error::custom("invalid plan structure or digest"));
        }
        Ok(Self {
            content: wire.content,
            digest: wire.digest,
        })
    }
}

// Reject duplicate JSON keys before serde_json::Value can discard a claim. Plans
// will cross independent inspection/approval parsers; ambiguous input is invalid.
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("unambiguous JSON plan content")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("duplicate plan field"));
                    }
                    values.insert(key, map.next_value::<StrictValue>()?.0);
                }
                Ok(StrictValue(Value::Object(values)))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<StrictValue>()? {
                    values.push(value.0);
                }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value.into())))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value)))
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(value)))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                self.visit_unit()
            }
        }
        d.deserialize_any(Visitor)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanValidation {
    Current,
    Stale,
    Unknown,
}
impl LifecyclePlan {
    pub fn content(&self) -> &PlanContent {
        &self.content
    }
    pub fn digest(&self) -> &PlanDigest {
        &self.digest
    }
    pub fn state_fingerprint(&self) -> &StateFingerprint {
        &self.content.state_fingerprint
    }
    pub fn validate_against(
        &self,
        g: &ArtifactGraph,
        report: &IntegrityReport,
        context: &PlanningContext,
    ) -> PlanValidation {
        let fresh = plan_lifecycle(g, report, context, self.content.request.clone());
        if fresh.content.state_fingerprint != self.content.state_fingerprint
            || fresh.digest != self.digest
        {
            PlanValidation::Stale
        } else if fresh.content.status == PlanStatus::Ready {
            PlanValidation::Current
        } else {
            PlanValidation::Unknown
        }
    }
}

// Every array in these v1 structures is a set. Sorting after projection also
// removes exact semantic duplicates; object keys use serde_json's sorted map.
fn canonical(v: &mut Value) {
    match v {
        Value::Array(a) => {
            for x in a.iter_mut() {
                canonical(x);
            }
            a.sort_by_key(|x| serde_json::to_vec(x).expect("JSON value"));
            a.dedup();
        }
        Value::Object(m) => {
            for x in m.values_mut() {
                canonical(x);
            }
        }
        _ => (),
    }
}
fn value<T: Serialize>(input: &T) -> Value {
    serde_json::to_value(input).expect("serializable domain value")
}
fn remove(v: &mut Value, path: &[&str], key: &str) {
    let mut current = v;
    for part in path {
        let Some(next) = current.get_mut(*part) else {
            return;
        };
        current = next;
    }
    if let Some(map) = current.as_object_mut() {
        map.remove(key);
    }
}
fn remove_in_array(v: &mut Value, path: &[&str], key: &str) {
    let mut current = v;
    for part in path {
        let Some(next) = current.get_mut(*part) else {
            return;
        };
        current = next;
    }
    if let Some(array) = current.as_array_mut() {
        for item in array {
            remove(item, &[], key);
        }
    }
}
fn project_context(v: &mut Value) {
    remove(v, &["provider", "compatibility"], "reason");
    remove(v, &[], "coverage");
    remove_in_array(v, &["diagnostics"], "message");
    remove_in_array(v, &["provider", "diagnostics"], "message");
}
fn projected<T: Serialize>(input: &T, kind: &str) -> Value {
    let mut v = value(input);
    match kind {
        "graph" => {
            if let Some(c) = v.get_mut("context") {
                project_context(c);
            }
            remove_in_array(&mut v, &["rules"], "description");
        }
        "report" => {
            if let Some(c) = v.get_mut("context") {
                project_context(c);
            }
            remove_in_array(&mut v, &["rules"], "description");
            for name in ["findings", "blockers", "residuals"] {
                remove_in_array(&mut v, &[name], "explanation");
            }
        }
        _ => (),
    }
    canonical(&mut v);
    v
}
fn hash(domain: &[u8], mut value: Value) -> String {
    canonical(&mut value);
    let mut h = Sha256::new();
    h.update(domain);
    h.update(serde_json::to_vec(&value).expect("JSON value"));
    format!(
        "sha256:{}",
        h.finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
fn plan_hash(c: &PlanContent) -> PlanDigest {
    PlanDigest(hash(b"agentmender/plan/v1\0", projected(c, "plan")))
}
// Duplicate structural claims are rejected by the effect contract. Preserve that
// distinction before canonical set deduplication, just like original graph conflicts.
fn effect_claim_shape(context: &PlanningContext) -> Value {
    match &context.effects {
        None => Value::Null,
        Some(boundary) => json!({
            "distinct_affected": boundary.affected.iter().collect::<BTreeSet<_>>().len() == boundary.affected.len(),
            "distinct_rule_ids": boundary.effect_rule_ids.iter().collect::<BTreeSet<_>>().len() == boundary.effect_rule_ids.len()
        }),
    }
}
fn authoritative(g: &ArtifactGraph, evidence: &[GraphEvidence]) -> bool {
    !evidence.is_empty()
        && evidence.iter().all(|e| {
            e.nature == EvidenceNature::Observation
                && e.observation.provider_version.as_ref()
                    == g.context().provider.fingerprint.executable_version.as_ref()
                && e.observation.provider_version.is_some()
                && g.effective_confidence(e) == Confidence::Authoritative
        })
}
fn native_id(g: &ArtifactGraph, id: &ArtifactId) -> Option<EntityId> {
    let n = g.node(id)?;
    if id.class != GraphNodeKind::Session
        || n.observations.len() != 1
        || !authoritative(g, &n.evidence)
    {
        return None;
    }
    let NodeObservation::Session(session) = &n.observations[0] else {
        return None;
    };
    if session.state == SessionState::Unknown {
        return None;
    }
    let native = EntityId::new(&session.provider_session_id).ok()?;
    let aliases = g.nodes().iter().filter(|other| other.id != *id && other.observations.iter().any(|o| matches!(o, NodeObservation::Session(s) if s.provider_session_id == session.provider_session_id))).count();
    if aliases != 0 {
        return None;
    }
    Some(native)
}
fn descendants_covered(g: &ArtifactGraph, ids: &BTreeSet<ArtifactId>) -> bool {
    let mut frontier: Vec<_> = ids
        .iter()
        .filter(|id| id.class == GraphNodeKind::Session)
        .cloned()
        .collect();
    let mut visited = BTreeSet::new();
    while let Some(parent) = frontier.pop() {
        if !visited.insert(parent.clone()) {
            continue;
        }
        for edge in g.edges().iter().filter(|e| {
            e.kind == EdgeKind::SpawnedBy
                && e.target == parent
                && e.source.class == GraphNodeKind::Session
        }) {
            if !ids.contains(&edge.source) {
                return false;
            }
            frontier.push(edge.source.clone());
        }
    }
    true
}
fn valid_digest(s: &str) -> bool {
    s.len() == 71
        && s.starts_with("sha256:")
        && s[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_scope(operation: NativeLifecycleOperation, scope: NativeEffectScope) -> bool {
    scope == NativeEffectScope::TargetOnly
        || matches!(
            (operation, scope),
            (
                NativeLifecycleOperation::ArchiveSession,
                NativeEffectScope::TargetAndAttemptedDescendants
            ) | (
                NativeLifecycleOperation::UnarchiveSession,
                NativeEffectScope::TargetRestoreWithProviderManagedState
            ) | (
                NativeLifecycleOperation::DeleteSession,
                NativeEffectScope::TargetAndDescendants
            )
        )
}
fn operation(intent: LifecycleIntent) -> Option<NativeLifecycleOperation> {
    match intent {
        LifecycleIntent::Archive => Some(NativeLifecycleOperation::ArchiveSession),
        LifecycleIntent::Unarchive => Some(NativeLifecycleOperation::UnarchiveSession),
        LifecycleIntent::Purge => Some(NativeLifecycleOperation::DeleteSession),
        LifecycleIntent::SoftDelete | LifecycleIntent::Repair => None,
    }
}

/// Derive an intention using only the supplied evidence. Planning-time validation
/// cannot replace immediate future execution-time revalidation.
pub fn plan_lifecycle(
    g: &ArtifactGraph,
    report: &IntegrityReport,
    context: &PlanningContext,
    request: LifecycleRequest,
) -> LifecyclePlan {
    let recomputed = evaluate_integrity(g, &report.proof_context);
    let fingerprint = StateFingerprint(hash(
        b"agentmender/state/v1\0",
        json!({"graph": projected(g,"graph"), "report": projected(report,"report"), "recomputed": projected(&recomputed,"report"), "context": projected(context,"context"), "request": projected(&request,"request"), "original_diagnostics": g.diagnostics(), "effect_claim_shape": effect_claim_shape(context), "effective_node_evidence": g.nodes().iter().flat_map(|n| n.evidence.iter().map(|e| EvaluatedEvidence { evidence: e.clone(), confidence: g.effective_confidence(e) })).collect::<Vec<_>>(), "effective_edge_evidence": g.edges().iter().flat_map(|edge| edge.evidence.iter().map(|e| EvaluatedEvidence { evidence: e.clone(), confidence: g.effective_confidence(e) })).collect::<Vec<_>>()}),
    ));
    let mut blockers = Vec::new();
    let op = operation(request.intent);
    if op.is_none() {
        blockers.push(PlanBlocker::UnsupportedIntent);
    }
    let mut matches: Vec<_> = context
        .capabilities
        .iter()
        .filter(|c| {
            c.provider == g.context().provider.kind
                && c.fingerprint == g.context().provider.fingerprint
                && Some(c.operation) == op
        })
        .cloned()
        .collect();
    dedup(&mut matches);
    let conflicting_capabilities = matches.len() > 1
        || matches.iter().any(|c| {
            !valid_scope(c.operation, c.effect_scope) || !valid_digest(&c.protocol_evidence_digest)
        });
    if conflicting_capabilities {
        blockers.push(PlanBlocker::ConflictingEvidence);
    }
    let cap = if matches.len() == 1 && !conflicting_capabilities {
        matches.pop()
    } else {
        None
    };
    if op.is_some() && cap.is_none() {
        blockers.push(PlanBlocker::CapabilityUnavailable);
    }
    if matches!(
        g.context().evidence_status,
        EvidenceStatus::Unverified | EvidenceStatus::RealPartial
    ) || matches!(
        g.context().provider.evidence_status,
        EvidenceStatus::Unverified | EvidenceStatus::RealPartial
    ) {
        blockers.push(PlanBlocker::EvidenceStatusUnverified);
    }
    if context.scope.project_path.is_some()
        || !context.scope.include_archived
        || !(1..=100_000).contains(&context.scope.max_entries)
    {
        blockers.push(PlanBlocker::FilteredScope);
    }
    if g.context().completeness != Completeness::Complete {
        blockers.push(PlanBlocker::IncompleteDiscovery);
    }
    if g.context().provider.compatibility.status != CompatibilityStatus::ReadOnlyKnown {
        blockers.push(PlanBlocker::UnknownCompatibility);
    }
    if projected(report, "report") != projected(&recomputed, "report") {
        blockers.push(PlanBlocker::IntegrityMismatch);
    }
    if !matches!(
        recomputed.verification_status,
        VerificationStatus::Healthy | VerificationStatus::HealthyWithExpectedResiduals
    ) {
        blockers.push(PlanBlocker::IntegrityUnverified);
    }
    for d in g
        .context()
        .diagnostics
        .iter()
        .chain(g.context().provider.diagnostics.iter())
    {
        match d.code {
            DiagnosticCode::SnapshotStale => blockers.push(PlanBlocker::SnapshotStale),
            DiagnosticCode::ConcurrentChange => blockers.push(PlanBlocker::ConcurrentChange),
            _ => (),
        }
    }
    if !g.diagnostics().is_empty() {
        blockers.push(PlanBlocker::ConflictingEvidence);
    }
    if g.node(&request.target).is_none() {
        blockers.push(PlanBlocker::TargetMissing);
    }
    if request.target.class != GraphNodeKind::Session {
        blockers.push(PlanBlocker::TargetNotNativeSession);
    }
    if g.roots().iter().any(|r| {
        r.node == request.target
            && matches!(
                r.kind,
                RootKind::ProviderGlobal | RootKind::UnknownState | RootKind::UserProtected
            )
    }) {
        blockers.push(PlanBlocker::ProtectedTarget);
    }
    let mut affected = BTreeSet::new();
    let mut boundary_valid = false;
    if let (Some(boundary), Some(expected)) = (&context.effects, op) {
        let ids: BTreeSet<_> = boundary.affected.iter().cloned().collect();
        let effect_rules: BTreeSet<_> = boundary.effect_rule_ids.iter().collect();
        let rules_valid = !effect_rules.is_empty()
            && effect_rules.len() == boundary.effect_rule_ids.len()
            && effect_rules.iter().all(|id| {
                g.rules().iter().filter(|r| &r.id == *id).count() == 1
                    && boundary.evidence.iter().any(|e| &e.rule_id == *id)
            })
            && boundary
                .evidence
                .iter()
                .all(|e| effect_rules.contains(&e.rule_id));
        boundary_valid = boundary.target == request.target
            && boundary.operation == expected
            && cap.as_ref() == Some(&boundary.capability)
            && boundary.operation == boundary.capability.operation
            && rules_valid
            && boundary.coherent
            && boundary.completeness == Completeness::Complete
            && ids.len() == boundary.affected.len()
            && ids.contains(&request.target)
            && ids.iter().all(|id| {
                g.node(id).is_some()
                    && matches!(id.class, GraphNodeKind::Session | GraphNodeKind::Artifact)
            })
            && cap
                .as_ref()
                .is_none_or(|c| c.effect_scope != NativeEffectScope::TargetOnly || ids.len() == 1)
            && (expected == NativeLifecycleOperation::UnarchiveSession
                || descendants_covered(g, &ids));
        if !boundary_valid {
            blockers.push(PlanBlocker::EffectBoundaryInvalid);
        }
        if !rules_valid || !authoritative(g, &boundary.evidence) {
            blockers.push(PlanBlocker::EffectEvidenceUnverified);
        }
        if boundary_valid {
            affected = ids;
        }
    } else if op.is_some() {
        blockers.push(PlanBlocker::EffectBoundaryUnavailable);
    }
    if g.roots().iter().any(|r| {
        affected.contains(&r.node)
            && matches!(
                r.kind,
                RootKind::ProviderGlobal | RootKind::UnknownState | RootKind::UserProtected
            )
    }) {
        blockers.push(PlanBlocker::ProtectedTarget);
    }
    let native_target_id = native_id(g, &request.target);
    if native_target_id.is_none()
        || affected
            .iter()
            .filter(|id| id.class == GraphNodeKind::Session)
            .any(|id| native_id(g, id).is_none())
    {
        blockers.push(PlanBlocker::NativeIdentityUnverified);
    }
    let artifacts: Vec<_> = g
        .nodes()
        .iter()
        .map(|node| {
            let ownership = g.ownership_assessment(&node.id).expect("graph node");
            let role = if node.id == request.target {
                PlannedArtifactRole::Target
            } else if affected.contains(&node.id) {
                PlannedArtifactRole::PotentialEffect
            } else {
                PlannedArtifactRole::Retained
            };
            if affected.contains(&node.id)
                && (request.intent == LifecycleIntent::Purge
                    || node.id.class == GraphNodeKind::Artifact)
                && ownership.classification != OwnershipClassification::Exclusive
            {
                blockers.push(PlanBlocker::OwnershipNotExclusive);
            }
            PlannedArtifact {
                id: node.id.clone(),
                explicitly_protected: g.roots().iter().any(|r| {
                    r.node == node.id
                        && matches!(
                            r.kind,
                            RootKind::ProviderGlobal
                                | RootKind::UnknownState
                                | RootKind::UserProtected
                        )
                }),
                ownership,
                role,
            }
        })
        .collect();
    let mut subtotal = 0u64;
    let mut reasons = Vec::new();
    if !boundary_valid {
        reasons.push(EstimateUnknownReason::IncompleteBoundary);
    }
    for id in &affected {
        let node = g.node(id).expect("validated boundary");
        if id.class != GraphNodeKind::Artifact {
            reasons.push(EstimateUnknownReason::NotArtifact);
            continue;
        }
        if g.ownership_assessment(id)
            .expect("graph node")
            .classification
            != OwnershipClassification::Exclusive
        {
            reasons.push(EstimateUnknownReason::ProtectedOwnership);
            continue;
        }
        let sizes: BTreeSet<_> = node
            .observations
            .iter()
            .filter_map(|o| match o {
                NodeObservation::Artifact(a) => a.size_bytes,
                _ => None,
            })
            .collect();
        if node
            .observations
            .iter()
            .any(|o| !matches!(o,NodeObservation::Artifact(a) if a.size_bytes.is_some()))
        {
            reasons.push(EstimateUnknownReason::MissingSize);
        }
        if sizes.len() > 1 {
            reasons.push(EstimateUnknownReason::ConflictingSizes);
        } else if let Some(size) = sizes.first() {
            if let Some(sum) = subtotal.checked_add(*size) {
                subtotal = sum;
            } else {
                reasons.push(EstimateUnknownReason::Overflow);
            }
        } else {
            reasons.push(EstimateUnknownReason::MissingSize);
        }
    }
    dedup(&mut reasons);
    dedup(&mut blockers);
    let status = if op.is_none()
        || (cap.is_none()
            && !conflicting_capabilities
            && g.context().provider.compatibility.status == CompatibilityStatus::ReadOnlyKnown)
    {
        PlanStatus::Unsupported
    } else if blockers.is_empty() {
        PlanStatus::Ready
    } else {
        PlanStatus::Blocked
    };
    let operations = if status == PlanStatus::Ready {
        vec![PlanOperation {
            target: request.target.clone(),
            native_id: native_target_id.clone().expect("ready native identity"),
            operation: op.expect("ready operation"),
            affected: affected.into_iter().collect(),
        }]
    } else {
        vec![]
    };
    let postconditions = if status == PlanStatus::Ready {
        required_postconditions(request.intent)
    } else {
        vec![ExpectedPostcondition::NoIndependentArtifactOperation]
    };
    let content = PlanContent {
        schema_version: 1,
        request,
        native_target_id,
        observation_scope: context.scope.clone(),
        compatibility_status: g.context().provider.compatibility.status,
        evidence_status: g.context().evidence_status,
        integrity: PlanIntegritySummary {
            status: recomputed.verification_status,
            blockers: recomputed.blockers.iter().map(|b| b.code).collect(),
            findings: recomputed.findings.iter().map(|f| f.kind).collect(),
            contract_id: recomputed
                .proof_context
                .contract
                .as_ref()
                .map(|c| c.id.clone()),
        },
        effect_contract: context.effects.as_ref().map(|e| PlanEffectContract {
            id: e.contract_id.clone(),
            affected: e.affected.clone(),
            rule_ids: e.effect_rule_ids.clone(),
        }),
        provider: g.context().provider.fingerprint.clone(),
        capability: cap,
        status,
        state_fingerprint: fingerprint,
        artifacts,
        operations,
        preconditions: required_preconditions(),
        postconditions,
        blockers,
        space: SpaceEstimate {
            logical: LogicalSpaceEstimate {
                known_subtotal_bytes: subtotal,
                total_bytes: if reasons.is_empty() {
                    Some(subtotal)
                } else {
                    None
                },
                unknown_reasons: reasons,
            },
            physical: PhysicalReclaimEstimate {
                bytes: None,
                reason: EstimateUnknownReason::PhysicalIdentityUnavailable,
            },
        },
        requires_human_approval: true,
        recovery_required: true,
    };
    let digest = plan_hash(&content);
    LifecyclePlan { content, digest }
}
fn dedup<T: Serialize + Clone>(v: &mut Vec<T>) {
    v.sort_by_key(|x| serde_json::to_vec(x).expect("serializable"));
    v.dedup_by(|a, b| {
        serde_json::to_vec(a).expect("serializable") == serde_json::to_vec(b).expect("serializable")
    });
}
impl std::fmt::Display for PlanDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::fmt::Display for StateFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn required_preconditions() -> Vec<PlanPrecondition> {
    vec![
        PlanPrecondition::BoundStateUnchanged,
        PlanPrecondition::ProviderProfileUnchanged,
        PlanPrecondition::NativeIdentityUnchanged,
        PlanPrecondition::CapabilityUnchanged,
        PlanPrecondition::EffectBoundaryUnchanged,
        PlanPrecondition::CompleteAuthoritativeScope,
        PlanPrecondition::NoAdditionalReferences,
        PlanPrecondition::OwnershipUnchanged,
        PlanPrecondition::ProtectedRootsUnchanged,
        PlanPrecondition::IntegrityUnchanged,
        PlanPrecondition::FreshCoherentObservation,
        PlanPrecondition::ImmediateRevalidation,
        PlanPrecondition::IndependentApproval,
        PlanPrecondition::RecoveryAvailable,
    ]
}
fn required_postconditions(intent: LifecycleIntent) -> Vec<ExpectedPostcondition> {
    let native = match intent {
        LifecycleIntent::Archive => ExpectedPostcondition::NativeSessionArchived,
        LifecycleIntent::Unarchive => ExpectedPostcondition::NativeSessionUnarchived,
        LifecycleIntent::Purge => ExpectedPostcondition::NativeSessionDeleted,
        LifecycleIntent::SoftDelete | LifecycleIntent::Repair => return vec![],
    };
    vec![
        native,
        ExpectedPostcondition::NoIndependentArtifactOperation,
    ]
}
