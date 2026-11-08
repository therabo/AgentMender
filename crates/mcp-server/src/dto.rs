//! Strict public input and privacy-preserving presentation types.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Codex,
    Claude,
}
impl From<Provider> for agentmender_domain::ProviderKind {
    fn from(v: Provider) -> Self {
        match v {
            Provider::Codex => Self::Codex,
            Provider::Claude => Self::Claude,
        }
    }
}
impl From<agentmender_domain::ProviderKind> for Provider {
    fn from(v: agentmender_domain::ProviderKind) -> Self {
        match v {
            agentmender_domain::ProviderKind::Codex => Self::Codex,
            agentmender_domain::ProviderKind::Claude => Self::Claude,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityClass {
    ProviderState,
    Workspace,
    Project,
    Session,
    Artifact,
    Unresolved,
}
impl From<agentmender_domain::GraphNodeKind> for EntityClass {
    fn from(v: agentmender_domain::GraphNodeKind) -> Self {
        use agentmender_domain::GraphNodeKind as K;
        match v {
            K::ProviderState => Self::ProviderState,
            K::Workspace => Self::Workspace,
            K::Project => Self::Project,
            K::Session => Self::Session,
            K::Artifact => Self::Artifact,
            K::Unresolved => Self::Unresolved,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    pub provider: Provider,
    pub class: EntityClass,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub token: String,
}
impl EntityRef {
    pub fn valid(&self) -> bool {
        self.token.len() == 64
            && self
                .token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}
fn default_max_entries() -> usize {
    1000
}
fn default_include_archived() -> bool {
    true
}
fn default_page_size() -> usize {
    50
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    #[serde(default = "default_max_entries")]
    #[schemars(range(min = 1, max = 10000))]
    pub max_entries: usize,
    #[serde(default = "default_include_archived")]
    pub include_archived: bool,
}
impl Default for Scope {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            include_archived: true,
        }
    }
}
impl Scope {
    pub fn valid(&self) -> bool {
        (1..=10000).contains(&self.max_entries)
    }
}
impl From<Scope> for agentmender_domain::DiscoveryScope {
    fn from(s: Scope) -> Self {
        Self {
            project_path: None,
            include_archived: s.include_archived,
            max_entries: s.max_entries,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageInput {
    #[serde(default = "default_page_size")]
    #[schemars(range(min = 1, max = 100))]
    pub page_size: usize,
    #[serde(default)]
    pub cursor: Option<String>,
}
impl Default for PageInput {
    fn default() -> Self {
        Self {
            page_size: 50,
            cursor: None,
        }
    }
}
impl PageInput {
    pub fn valid(&self) -> bool {
        (1..=100).contains(&self.page_size) && self.cursor.as_ref().is_none_or(|s| s.len() <= 256)
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProvidersInput {}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryInput {
    #[serde(default)]
    pub provider: Option<Provider>,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub page: PageInput,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetInput {
    #[serde(default)]
    pub provider: Option<Provider>,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub page: PageInput,
    pub target: EntityRef,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Archive,
    Unarchive,
    SoftDelete,
    Purge,
}
impl From<Intent> for agentmender_domain::LifecycleIntent {
    fn from(v: Intent) -> Self {
        match v {
            Intent::Archive => Self::Archive,
            Intent::Unarchive => Self::Unarchive,
            Intent::SoftDelete => Self::SoftDelete,
            Intent::Purge => Self::Purge,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanInput {
    #[serde(default)]
    pub provider: Option<Provider>,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub page: PageInput,
    pub target: EntityRef,
    pub intent: Intent,
}

/// Closed vocabulary of source domain categories.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Active,
    ActiveSession,
    AmbiguousIdentity,
    Archive,
    ArchiveSession,
    Archived,
    ArchivedRetainedSession,
    ArchivedSessionDiscovery,
    Artifact,
    ArtifactDiscovery,
    ArtifactToOwner,
    AssociatedWith,
    Attachment,
    AuditOnlyUnknown,
    Authoritative,
    Blocked,
    BoundStateUnchanged,
    BrokenDatabaseReference,
    BundleMetadata,
    BundledCli,
    CapabilityUnavailable,
    CapabilityUnchanged,
    Child,
    Claude,
    Cli,
    Codex,
    Complete,
    CompleteAuthoritativeScope,
    ConcurrentChange,
    ConflictingCoverage,
    ConflictingEvidence,
    ConflictingIdentityEvidence,
    ConflictingRelationshipEvidence,
    ConflictingSizes,
    ContractMismatch,
    Critical,
    Current,
    DangerousResidual,
    DanglingReference,
    DatabaseReference,
    Degraded,
    DeleteSession,
    Derived,
    Desktop,
    DuplicateSessionIdentifier,
    EffectBoundaryInvalid,
    EffectBoundaryUnavailable,
    EffectBoundaryUnchanged,
    EffectEvidenceUnverified,
    EmptyEdgeEvidence,
    Error,
    EstablishedViolation,
    EvidenceStatusUnverified,
    Exclusive,
    ExecutableProbe,
    ExpectedProviderResidual,
    Failed,
    FilesystemMetadata,
    FilteredScope,
    Fixture,
    FixtureVerified,
    ForeignRule,
    ForeignScope,
    FreshCoherentObservation,
    Global,
    Healthy,
    HealthyWithExpectedResiduals,
    Heuristic,
    ImmediateRevalidation,
    InadmissibleSource,
    IncoherentObservation,
    IncompleteBoundary,
    IncompleteDiscovery,
    IndependentApproval,
    Index,
    Inference,
    Info,
    Installed,
    IntegrityMismatch,
    IntegrityUnchanged,
    IntegrityUnverified,
    InternalImplementationDetail,
    LimitReached,
    LinkSkipped,
    MalformedObservation,
    MalformedProviderState,
    MapsTo,
    MetadataAudit,
    MismatchedObservation,
    MissingChild,
    MissingCoverage,
    MissingEvidence,
    MissingIntegrityContract,
    MissingNode,
    MissingSize,
    MissingTranscript,
    NativeIdentityUnchanged,
    NativeIdentityUnverified,
    NativeSessionArchived,
    NativeSessionDeleted,
    NativeSessionUnarchived,
    NoAdditionalReferences,
    NoIndependentArtifactOperation,
    NoOwnershipContract,
    NodeCoverage,
    NotArtifact,
    NotDetected,
    Observation,
    OfficialExperimental,
    OfficialStable,
    OrphanAttachment,
    OrphanChild,
    OrphanPlan,
    OrphanTodo,
    OrphanTranscript,
    Orphaned,
    Overflow,
    OwnerToArtifact,
    OwnershipNotExclusive,
    OwnershipUnchanged,
    ParentSessionDiscovery,
    Partial,
    PartialCoverage,
    PermissionDenied,
    PhysicalIdentityUnavailable,
    Plan,
    PotentialEffect,
    Project,
    ProjectDiscovery,
    ProtectedOwnership,
    ProtectedRoot,
    ProtectedRootsUnchanged,
    ProtectedTarget,
    ProviderApi,
    ProviderGlobal,
    ProviderMetadata,
    ProviderNotDetected,
    ProviderNotInstalled,
    ProviderProfileUnchanged,
    ProviderSnapshot,
    ProviderState,
    ProviderUnsupported,
    ProviderVersionUnknown,
    Purge,
    ReadOnlyKnown,
    Ready,
    RealPartial,
    RealVerified,
    RecoveryAvailable,
    Reference,
    Referenced,
    References,
    RelationshipCoverage,
    Repair,
    RequiredOwner,
    ResidualObservation,
    Retained,
    RuleContextMismatch,
    SafeResidual,
    Session,
    SessionDiscovery,
    SessionMetadata,
    Shared,
    SnapshotStale,
    SnapshotUnavailable,
    SnapshotVerified,
    SoftDelete,
    SpawnedBy,
    Stale,
    StateOnly,
    Strong,
    SuspiciousInconsistency,
    SyntheticOnly,
    Target,
    TargetAndAttemptedDescendants,
    TargetAndDescendants,
    TargetMissing,
    TargetNotNativeSession,
    TargetOnly,
    TargetRestoreWithProviderManagedState,
    Todo,
    Transcript,
    Unarchive,
    UnarchiveSession,
    Unavailable,
    UnavailableCoverage,
    Unknown,
    UnknownArtifact,
    UnknownCompatibility,
    UnknownProducer,
    UnknownResidual,
    UnknownRule,
    UnknownState,
    UnmetPrerequisite,
    UnreadableObservation,
    Unresolved,
    UnresolvedEndpoint,
    UnresolvedIdentity,
    Unsupported,
    UnsupportedIntent,
    Unverified,
    UnverifiedProducerVersion,
    UserProtected,
    Warning,
    Workspace,
    Worktree,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FingerprintView {
    pub known_version: Option<String>,
    pub known_interface: Option<String>,
    pub known_layout: Option<String>,
    pub reference: String,
    pub executable_present: bool,
    pub version_ref: Option<String>,
    pub adapter_ref: String,
    pub interface_ref: Option<String>,
    pub layout_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ContextView {
    pub namespace_ref: String,
    pub coverage_ref: Option<String>,
    pub fingerprint: FingerprintView,
    pub provider_evidence_status: Category,
    pub interface_stability: Category,
    pub capabilities: Vec<Category>,
    pub installations: Vec<InstallationView>,
    pub provider: Provider,
    pub presence: Category,
    pub compatibility: Category,
    pub evidence_status: Category,
    pub completeness: Option<Category>,
    pub diagnostics: Vec<Category>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InstallationView {
    pub fact_ref: String,
    pub known_version: Option<String>,
    pub surface: Category,
    pub executable_present: bool,
    pub version_ref: Option<String>,
    pub version_source: Category,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct EvidenceView {
    pub known_version: Option<String>,
    pub version_ref: Option<String>,
    pub raw_rule_ref: String,
    pub effective_confidence: Option<Category>,
    pub fact_ref: String,
    pub source: Category,
    pub confidence: Category,
    pub nature: Category,
    pub rule_ref: String,
    pub locator_present: bool,
    pub timestamp_present: bool,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RawEvidenceView {
    pub known_version: Option<String>,
    pub fact_ref: String,
    pub source: Category,
    pub confidence: Category,
    pub version_ref: Option<String>,
    pub rule_ref: String,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NodeView {
    pub namespace_ref: String,
    pub ownership: Option<Category>,
    pub ownership_blockers: Vec<Category>,
    pub ownership_evidence: Vec<EvidenceView>,
    pub local_ref: String,
    pub id: EntityRef,
    pub observation_count: usize,
    pub conflicting: bool,
    pub protected: bool,
    pub observations: Vec<ObservationView>,
    pub evidence: Vec<EvidenceView>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ObservationView {
    pub path_present: bool,
    pub parent_ref: Option<String>,
    pub timestamp_present: bool,
    pub raw_evidence: Option<RawEvidenceView>,
    pub fact_ref: String,
    pub kind: String,
    pub state: Option<Category>,
    pub artifact_kind: Option<Category>,
    pub size_bytes: Option<u64>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct EdgeView {
    pub namespace_ref: String,
    pub source: EntityRef,
    pub target: EntityRef,
    pub kind: Category,
    pub evidence: Vec<EvidenceView>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RuleView {
    pub known_id: Option<String>,
    pub descriptor_ref: String,
    pub reference: String,
    pub recognized: bool,
    pub sources: Vec<Category>,
    pub max_confidence: Category,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Record {
    Contract {
        descriptor_ref: String,
        known_version: Option<String>,
        reference: String,
        coherent: bool,
        evidence: Vec<EvidenceView>,
    },
    RelationshipCoverage {
        source_class: EntityClass,
        target_class: EntityClass,
        edge_kind: Category,
        completeness: Category,
        evidence: Vec<EvidenceView>,
    },
    OwnerRelation {
        artifact: EntityRef,
        owner_class: EntityClass,
        edge_kind: Category,
        direction: Category,
        rule_refs: Vec<String>,
    },
    ScanReference {
        evidence: RawEvidenceView,
        source_ref: String,
        target_ref: String,
        reference_kind: Category,
        evidence_source: Category,
        confidence: Category,
    },
    Diagnostic {
        code: String,
        node: Option<EntityRef>,
        rule_ref: Option<String>,
    },
    Proof {
        proof_kind: Category,
        subject: Option<EntityRef>,
        target: Option<EntityRef>,
        category: Option<Category>,
        completeness: Option<Category>,
        evidence: Vec<EvidenceView>,
    },
    Node {
        node: NodeView,
    },
    Edge {
        edge: EdgeView,
    },
    Root {
        node: EntityRef,
        root_kind: Category,
    },
    Rule {
        rule: RuleView,
    },
    Finding {
        identity_ref: String,
        subject: EntityRef,
        rule_refs: Vec<String>,
        observations: Vec<ObservationView>,
        relationships: Vec<EdgeView>,
        issue_kind: Category,
        severity: Category,
        disposition: Category,
        affected: Vec<EntityRef>,
        evidence: Vec<EvidenceView>,
    },
    Residual {
        artifact: EntityRef,
        classification: Category,
        evidence: Vec<EvidenceView>,
    },
    PlannedArtifact {
        id: EntityRef,
        role: Category,
        ownership: Category,
        protected: bool,
        blockers: Vec<Category>,
        evidence: Vec<EvidenceView>,
    },
    Operation {
        native_id_ref: String,
        target: EntityRef,
        operation: Category,
        affected: Vec<EntityRef>,
    },
    Provider {
        context: ContextView,
    },
    ScanEntity {
        node: NodeView,
    },
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BlockerView {
    pub namespace_ref: Option<String>,
    pub provider: Option<Provider>,
    pub evidence: Vec<EvidenceView>,
    pub code: Category,
    pub affected: Vec<EntityRef>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CapabilityView {
    pub provider: Provider,
    pub fingerprint: FingerprintView,
    pub operation: Category,
    pub effect_scope: Category,
    pub protocol_evidence_ref: String,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PlanView {
    pub schema_version: u32,
    pub fingerprint: FingerprintView,
    pub compatibility: Category,
    pub evidence_status: Category,
    pub scope_max_entries: usize,
    pub scope_include_archived: bool,
    pub scope_project_path_present: bool,
    pub native_target_ref: Option<String>,
    pub integrity_contract_ref: Option<String>,
    pub capability: Option<CapabilityView>,
    pub logical_known_subtotal_bytes: u64,
    pub logical_total_bytes: Option<u64>,
    pub logical_unknown_reasons: Vec<Category>,
    pub physical_bytes: Option<u64>,
    pub physical_reason: Category,
    pub integrity_status: Category,
    pub integrity_blockers: Vec<Category>,
    pub integrity_findings: Vec<Category>,
    pub effect_contract_ref: Option<String>,
    pub effect_affected: Vec<EntityRef>,
    pub effect_rules: Vec<String>,
    pub digest: String,
    pub state_fingerprint: String,
    pub digest_basis: String,
    pub status: Category,
    pub intent: Category,
    pub target: EntityRef,
    pub blockers: Vec<Category>,
    pub preconditions: Vec<Category>,
    pub postconditions: Vec<Category>,
    pub planning_only: bool,
    pub executed: bool,
    pub requires_human_approval: bool,
    pub recovery_required: bool,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RecordCounts {
    pub nodes: usize,
    pub edges: usize,
    pub roots: usize,
    pub rules: usize,
    pub diagnostics: usize,
    pub proofs: usize,
    pub findings: usize,
    pub residuals: usize,
    pub planned_artifacts: usize,
    pub operations: usize,
    pub providers: usize,
    pub scan_entities: usize,
    pub scan_references: usize,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct VerificationSummary {
    pub namespace_ref: String,
    pub status: Category,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RecordEntry {
    pub namespace_ref: String,
    #[serde(flatten)]
    pub record: Record,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PageView {
    pub verification: Vec<VerificationSummary>,
    pub counts: RecordCounts,
    pub contexts: Vec<ContextView>,
    pub failures: Vec<BlockerView>,
    pub statuses: Vec<Category>,
    pub blockers: Vec<BlockerView>,
    pub plan: Option<PlanView>,
    pub total_records: usize,
    pub records: Vec<RecordEntry>,
    pub next_cursor: Option<String>,
    pub presentation_complete: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ManagementProtectionRequired,
    ProtectedManagementSession,
    RemovalPlanChanged,
    ReadOnlyRemovalUnavailable,
    Busy,
    Cancelled,
    DeadlineExceeded,
    QueryFailed,
    TargetNotObserved,
    AmbiguousTarget,
    TargetUnavailable,
    ProviderNotDetected,
    ProviderUnsupported,
    CompatibilityBlocked,
    CapabilityUnavailable,
    InvalidScope,
    StaleCursor,
    InvalidCursor,
    ResponseTooLarge,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ErrorView {
    pub error: ErrorCode,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ToolOutput {
    Success(Box<PageView>),
    Error(ErrorView),
}
