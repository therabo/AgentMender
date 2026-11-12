//! Opt-in bounded handoff DTOs. No persistence or provider access in this crate.
use crate::{
    dto::EntityRef,
    projection::Projector,
    protocol::{schema, schema_described},
};
use agentmender_application::{
    ApplicationError, QueryControl,
    conversation::{ConversationRequest, ConversationResponse},
    session_resolution::{self, ResolutionAction},
};
use agentmender_domain::{
    ApparentCodexCaller, ContextReference, SearchPurpose,
    search_intent::{
        self, EXPANSION_LIMIT, PROJECT_LIMIT, PROJECT_NAME_LIMIT, SEARCH_INTENT_VERSION,
        SUBJECT_LIMIT, SearchIntent, SearchIntentError, TERM_LIMIT, TERMS_LIMIT,
    },
};
use rmcp::model::{ErrorData, Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type ConversationExecutor = Arc<
    dyn Fn(
            ConversationRequest,
            QueryControl,
            Option<ApparentCodexCaller>,
        ) -> Result<ConversationResponse, ApplicationError>
        + Send
        + Sync,
>;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchInput {
    #[serde(default)]
    #[schemars(length(max = 256))]
    query: String,
    #[serde(default = "include_archived")]
    include_archived: bool,
    #[serde(default)]
    include_content: bool,
    #[serde(default)]
    purpose: SearchPurposeInput,
    #[serde(default)]
    #[schemars(length(max = 256))]
    project_label: Option<String>,
    #[serde(default)]
    #[schemars(range(min = 1, max = 20))]
    page_size: Option<usize>,
    #[serde(default)]
    #[schemars(length(max = 32768))]
    cursor: Option<String>,
    #[serde(default = "include_archived")]
    excerpts: bool,
    #[serde(default)]
    structured: Option<StructuredQueryInput>,
}
// These wire DTOs mirror the pure domain query contract without adding a
// presentation/schema dependency to domain. Domain validation runs after decoding.
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StructuredQueryInput {
    filter: QueryExprInput,
    #[serde(default)]
    mode: SearchModeInput,
    #[serde(default)]
    sort: SearchSortInput,
    #[serde(default)]
    coverage: CoverageInput,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SearchModeInput {
    #[default]
    Exhaustive,
    Ranked,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SearchSortInput {
    #[default]
    Identity,
    Created,
    LastActive,
    Relevance,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum CoverageInput {
    #[default]
    BestEffort,
    Complete,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ProjectSourceInput {
    #[default]
    ObservedLabel,
    VerifiedLabel,
    VerifiedRoot,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum TextMatchInput {
    #[default]
    Contains,
    Phrase,
    Exact,
}
#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SessionStateInput {
    Active,
    Archived,
    Unknown,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
enum QueryRangeInput {
    Utc(UtcRangeInput),
    Calendar(CalendarRangeInput),
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UtcRangeInput {
    #[serde(default)]
    start: Option<i64>,
    #[serde(default)]
    end: Option<i64>,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CalendarRangeInput {
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date: Option<String>,
    timezone: String,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum QueryExprInput {
    And {
        args: Vec<Self>,
    },
    Or {
        args: Vec<Self>,
    },
    Not {
        arg: Box<Self>,
    },
    Provider {
        values: Vec<crate::dto::Provider>,
    },
    Project {
        values: Vec<String>,
        #[serde(default)]
        source: ProjectSourceInput,
    },
    ProjectUnknown {
        #[serde(default)]
        source: ProjectSourceInput,
    },
    State {
        values: Vec<SessionStateInput>,
    },
    NativeId {
        values: Vec<String>,
    },
    Created {
        range: QueryRangeInput,
    },
    LastActive {
        range: QueryRangeInput,
    },
    Text {
        value: String,
        #[serde(default)]
        kind: TextMatchInput,
        #[serde(default)]
        alternatives: Vec<String>,
    },
    MessageTime {
        range: QueryRangeInput,
    },
}
#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SearchPurposeInput {
    Diagnostic,
    #[default]
    Management,
}
impl From<SearchPurposeInput> for agentmender_domain::SearchPurpose {
    fn from(p: SearchPurposeInput) -> Self {
        match p {
            SearchPurposeInput::Diagnostic => Self::Diagnostic,
            SearchPurposeInput::Management => Self::Management,
        }
    }
}
#[derive(JsonSchema)]
#[schemars(rename = "ContextInput")]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // Legacy catalog shape. Both routes share ContextInput decoding.
struct SignedContextInput {
    target: EntityRef,
    #[schemars(length(max = 4096))]
    evidence_ref: String,
    window: Option<ContextOptionsInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ContextInput {
    /// Signed target for a previously returned evidence reference.
    target: Option<EntityRef>,
    #[schemars(length(max = 32768))]
    evidence_ref: Option<String>,
    /// Open a new caller-scoped view of this exact Codex UUID.
    native_session_id: Option<String>,
    #[serde(default)]
    window: Option<ContextOptionsInput>,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ContextOptionsInput {
    /// Optional byte offset within the same validated message; zero requests its beginning.
    #[serde(default)]
    #[schemars(range(max = 8388608))]
    message_offset: Option<usize>,
    #[serde(default)]
    #[schemars(range(max = 100))]
    before: usize,
    #[serde(default)]
    #[schemars(range(max = 100))]
    after: usize,
    #[serde(default = "context_messages")]
    #[schemars(range(min = 1, max = 20))]
    max_messages: usize,
    #[serde(default = "context_bytes")]
    #[schemars(range(min = 1, max = 16384))]
    max_bytes: usize,
    #[serde(default)]
    #[schemars(length(max = 256))]
    query: Option<String>,
    #[serde(default)]
    direction: ContextDirectionInput,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ContextDirectionInput {
    Start,
    #[default]
    Around,
    Forward,
    Backward,
    Turn,
    Occurrences,
}
fn context_messages() -> usize {
    20
}
fn context_bytes() -> usize {
    16 * 1024
}
fn include_archived() -> bool {
    true
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PreviewInput {
    target: EntityRef,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PrepareInput {
    target: EntityRef,
    #[schemars(regex(pattern = "^sha256:[0-9a-f]{64}$"))]
    expected_plan_digest: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StatusInput {
    #[serde(default)]
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    transaction_id: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HandoffInput {
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    transaction_id: String,
}
#[derive(Serialize, JsonSchema)]
struct SessionLabelView {
    target: EntityRef,
    namespace_ref: String,
    native_session_id: String,
    display_label: String,
    title: Option<String>,
    project_label: Option<String>,
    created_at: Option<i64>,
    last_active_at: Option<i64>,
    state: String,
    exact_profile: &'static str,
    producer_version_matches: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    producer_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_native_session_id: Option<String>,
    protected: bool,
    can_request_preview: bool,
    removal_eligibility: &'static str,
}
#[derive(Serialize, JsonSchema)]
struct SearchView {
    sessions: Vec<SessionLabelView>,
    completeness: String,
    search_basis: &'static str,
    purpose: &'static str,
    management_exclusion: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_compatibility: Option<ReadCompatibilityView>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
enum SearchOutput {
    Legacy(SearchView),
    Evidence(EvidenceSearchView),
    Error(crate::dto::ErrorView),
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
enum DiscoveryOutput {
    Evidence(Box<EvidenceSearchView>),
    Refusal(DiscoveryRefusal),
}
#[derive(Serialize, JsonSchema)]
struct DiscoveryRefusal {
    error: DiscoveryError,
    message: &'static str,
    result_count: usize,
    sessions: [SessionLabelView; 0],
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum DiscoveryError {
    DiscoveryCallerUnavailable,
}
pub(super) fn discovery_caller_unavailable() -> serde_json::Value {
    serde_json::to_value(DiscoveryOutput::Refusal(DiscoveryRefusal {
        error: DiscoveryError::DiscoveryCallerUnavailable,
        message: "Implicit caller exclusion could not be established for the configured Codex root.",
        result_count: 0,
        sessions: [],
    }))
    .expect("discovery refusal serializes")
}
pub(super) fn caller_identity_required(instance: String) -> serde_json::Value {
    let mut value = serde_json::to_value(SearchOutput::Error(crate::dto::ErrorView {
        error: crate::dto::ErrorCode::CallerIdentityRequired,
    }))
    .expect("error serializes");
    value.as_object_mut().expect("error object").extend(serde_json::json!({"sessions":[],"result_count":0,"registration_instance":instance,"authenticated_caller_binding":false}).as_object().expect("fixed object").clone());
    value
}
#[derive(Serialize, JsonSchema)]
struct EvidenceView {
    source: String,
    capture: &'static str,
    record_index: usize,
    excerpt: Option<String>,
    truncated: bool,
    format: String,
    evidence_ref: String,
    untrusted_data: bool,
    position: Option<MessagePositionView>,
}
#[derive(Serialize, JsonSchema)]
struct MessagePositionView {
    ordinal: usize,
    message_id: Option<String>,
    turn_id: Option<String>,
    timestamp: Option<String>,
    chunk_offset: usize,
    chunk_total: usize,
}
#[derive(Serialize, JsonSchema)]
struct EvidenceHitView {
    source_coverage: Option<SourceCoverageView>,
    #[serde(flatten)]
    session: SessionLabelView,
    score: u32,
    matched_terms: Vec<String>,
    match_sources: Vec<String>,
    content_status: String,
    content_coverage: &'static str,
    hard_filters_verified: bool,
    evidence: Vec<EvidenceView>,
}
#[derive(Serialize, JsonSchema)]
struct ReadWorkView {
    compressed_bytes: u64,
    decoded_bytes: u64,
    replay_bytes: u64,
    records_inspected: u64,
}
#[derive(Serialize, JsonSchema)]
struct SourceCoverageView {
    decoded_bytes: u64,
    physical_eof: bool,
    logical_complete: bool,
    skipped_records: usize,
    unsupported_records: usize,
    suppressed_records: usize,
    external_history_unaccounted: bool,
}
impl From<agentmender_domain::SourceCoverage> for SourceCoverageView {
    fn from(c: agentmender_domain::SourceCoverage) -> Self {
        Self {
            decoded_bytes: c.decoded_bytes,
            physical_eof: c.physical_eof,
            logical_complete: c.logical_complete,
            skipped_records: c.skipped_records,
            unsupported_records: c.unsupported_records,
            suppressed_records: c.suppressed_records,
            external_history_unaccounted: c.external_history_unaccounted,
        }
    }
}
impl From<agentmender_domain::ReadWork> for ReadWorkView {
    fn from(work: agentmender_domain::ReadWork) -> Self {
        Self {
            compressed_bytes: work.compressed_bytes,
            decoded_bytes: work.decoded_bytes,
            replay_bytes: work.replay_bytes,
            records_inspected: work.records_inspected,
        }
    }
}
#[derive(Serialize, JsonSchema)]
struct SearchCoverageView {
    work: ReadWorkView,
    eligible_observed: usize,
    sessions_inspected: usize,
    bytes_read: usize,
    matched_in_batch: usize,
    unsupported_sessions: usize,
    truncated_sessions: usize,
    suppressed_records: usize,
    unreadable_sessions: usize,
    unresolved_sessions: usize,
    fully_traversed_sessions: usize,
    reasons: Vec<String>,
}
#[derive(Serialize, JsonSchema)]
struct QueryInterpretationView {
    /// Canonical JSON of the validated input filter. Serialized separately to keep
    /// the finite MCP catalog within its existing transport budget.
    filter_json: String,
    mode: SearchModeInput,
    sort: SearchSortInput,
    coverage: CoverageInput,
}
fn query_interpretation(
    query: &agentmender_domain::StructuredQuery,
) -> Result<QueryInterpretationView, serde_json::Error> {
    let input: StructuredQueryInput =
        serde_json::to_value(query).and_then(serde_json::from_value)?;
    Ok(QueryInterpretationView {
        filter_json: serde_json::to_string(&input.filter)?,
        mode: input.mode,
        sort: input.sort,
        coverage: input.coverage,
    })
}
#[derive(Serialize, JsonSchema)]
struct EvidenceSearchView {
    sessions: Vec<EvidenceHitView>,
    unresolved_candidates: Vec<EvidenceHitView>,
    purpose: String,
    management_exclusion: String,
    authenticated_caller_binding: bool,
    registration_instance: String,
    completeness: &'static str,
    search_basis: &'static str,
    ranking: &'static str,
    result_count: usize,
    coverage: SearchCoverageView,
    next_cursor: Option<String>,
    query_interpretation: Option<QueryInterpretationView>,
    presentation_complete: bool,
    coverage_requirement_met: bool,
}
#[derive(Serialize, JsonSchema)]
struct ContextView {
    evidence: EvidenceView,
    purpose: String,
    management_exclusion: &'static str,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
enum ContextOutput {
    Message(ContextView),
    Window(ContextWindowView),
}
#[derive(Serialize, JsonSchema)]
struct ContextWindowView {
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<EntityRef>,
    source_coverage: SourceCoverageView,
    work: ReadWorkView,
    messages: Vec<EvidenceView>,
    previous_ref: Option<String>,
    next_ref: Option<String>,
    previous_window: Option<ContextOptionsInput>,
    next_window: Option<ContextOptionsInput>,
    at_start: bool,
    at_end: bool,
    partial: bool,
    bytes_read: usize,
    purpose: String,
    management_exclusion: &'static str,
}
#[derive(Serialize, JsonSchema)]
struct ReadCompatibilityView {
    metadata_contract: String,
    provider_version: Option<String>,
    app_server_methods: Vec<String>,
    native_interface_status: String,
    native_delete_admission: String,
    scoped_fallback_admission: String,
    additional_cleanup_admission: String,
    coverage: String,
    relationship_coverage: String,
    source_changed_during_capture: bool,
    authenticated_caller_binding: bool,
}
impl From<&agentmender_domain::SessionReadCompatibility> for ReadCompatibilityView {
    fn from(c: &agentmender_domain::SessionReadCompatibility) -> Self {
        Self {
            metadata_contract: c.metadata_contract.clone(),
            provider_version: c.provider_version.clone(),
            app_server_methods: c.app_server_methods.clone(),
            native_interface_status: c.native_interface_status.clone(),
            native_delete_admission: c.native_delete_admission.clone(),
            scoped_fallback_admission: c.scoped_fallback_admission.clone(),
            additional_cleanup_admission: c.additional_cleanup_admission.clone(),
            coverage: c.coverage.clone(),
            relationship_coverage: serde_json::to_value(c.relationship_coverage)
                .expect("coverage")
                .as_str()
                .expect("enum")
                .into(),
            source_changed_during_capture: c.source_changed_during_capture,
            authenticated_caller_binding: false,
        }
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
enum PreviewOutput {
    Supported(PreviewView),
    ReadOnly(ReadOnlyPreviewView),
}
#[derive(Serialize, JsonSchema)]
struct ReadOnlyPreviewView {
    session: SessionLabelView,
    compatibility: ReadCompatibilityView,
    operation: &'static str,
    status: &'static str,
    execution_permitted: bool,
    affected_session_ids: Vec<String>,
    affected_set_complete: bool,
    descendant_effects: &'static str,
    irreversible_effects: &'static str,
    residuals: &'static str,
    writer_prerequisites: &'static str,
    verification_postconditions: &'static str,
    independent_human_approval_required: bool,
    blockers: Vec<&'static str>,
}
#[derive(Serialize, JsonSchema)]
struct PreviewView {
    session: SessionLabelView,
    review: PrimaryReviewView,
    operation: &'static str,
    irreversible: bool,
    recovery: &'static str,
    execution_channel: &'static str,
}
#[derive(Serialize, JsonSchema)]
struct PrimaryReviewView {
    plan_digest: String,
    state_fingerprint: String,
    digest_basis: &'static str,
    status: String,
    version: String,
    adapter: String,
    executable_sha256: String,
    schema_sha256: String,
    affected_session_ids: Vec<String>,
    required_payload_count: usize,
    target_row_sets: Vec<RowSetView>,
    retained_primary_file_count: usize,
    protected_sessions: Vec<EntityRef>,
    logical_planned_bytes: Option<u64>,
    known_residuals: bool,
    postconditions: Vec<String>,
    blockers: Vec<String>,
}
#[derive(Serialize, JsonSchema)]
struct RowSetView {
    table: String,
    expected_count: u64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
struct RemovalStatusView {
    transaction_id: String,
    plan_digest: String,
    session_label: String,
    state: String,
    approval_status: String,
    recovery_available: bool,
    primary_verified: bool,
    other_sessions_preserved: bool,
    additional_artifacts_fully_cleaned: bool,
    graphical_confirmation: String,
    verification_basis: String,
    execution_channel: String,
    retry_permitted: bool,
    logical_unlinked_bytes: Option<u64>,
    physical_reclaimed_bytes: Option<u64>,
}
#[derive(Serialize, JsonSchema)]
struct TransactionsView {
    transactions: Vec<RemovalStatusView>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum ResolutionInput {
    Start {
        search_intent: Box<SearchIntentInput>,
    },
    Advance {
        run: String,
        expected_revision: usize,
        #[schemars(length(min = 1, max = 64))]
        operation_id: String,
        #[serde(default)]
        #[schemars(length(max = 5))]
        assessments: Vec<AssessmentInput>,
        #[serde(default)]
        #[schemars(length(max = session_resolution::WINDOW_BATCH))]
        context_requests: Vec<ResolutionWindowInput>,
        /// Publish the next verified result page after closing the evaluator.
        #[serde(default)]
        publish: bool,
        /// Renew an active evaluation while the evaluator is still working.
        #[serde(default)]
        keep_alive: bool,
        #[serde(default)]
        cancel: bool,
        #[serde(default)]
        luna_unavailable: bool,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ResolutionWindowInput {
    anchor: usize,
    window: Option<ContextOptionsInput>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchIntentInput {
    schema_version: SearchIntentVersion,
    /// The information need, including material scope and qualifications.
    #[schemars(length(min = 1, max = SUBJECT_LIMIT))]
    subject: String,
    /// Use the activity requested by the user. Omit for broad work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    activity: Option<ActivityInput>,
    /// A requested result such as implemented, validated, resolved or failed. Omit or use null when unspecified.
    required_outcome: Option<OutcomeInput>,
    /// Explicit session-wide outcome exclusion. Verification requires complete logical history.
    excluded_outcome: Option<OutcomeInput>,
    /// Defaults to all_relevant. Use single_best only when explicitly asked to choose one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result_policy: Option<ResultPolicyInput>,
    /// Use exhaustive for requests for complete coverage. The server reports coverage gaps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coverage_goal: Option<CoverageGoalInput>,
    hints: SearchHintsInput,
    /// Only restrictions the user actually requested. An omitted or null field leaves scope unrestricted.
    constraints: Option<SearchConstraintsInput>,
    /// Explicit Boolean predicate for advanced retrieval. Replaces lexical hints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    filter: Option<Box<QueryExprInput>>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
enum SearchIntentVersion {
    #[serde(rename = "1")]
    V1,
}

#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ActivityInput {
    Any,
    Discussion,
    Investigation,
    Implementation,
    Testing,
    Validation,
    Decision,
    Troubleshooting,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ResultPolicyInput {
    AllRelevant,
    SingleBest,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum CoverageGoalInput {
    Standard,
    Exhaustive,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
struct LexicalTerm(#[schemars(length(min = 1, max = TERM_LIMIT))] String);

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
struct ProjectName(#[schemars(length(min = 1, max = PROJECT_NAME_LIMIT))] String);

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchHintsInput {
    /// One or a few distinctive content terms or phrases. Each term can independently retrieve a candidate.
    #[schemars(length(max = TERMS_LIMIT))]
    terms: Vec<LexicalTerm>,
    /// Optional distinct alternatives planned before retrieval. Boolean filters carry their own aliases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = EXPANSION_LIMIT))]
    expansion_terms: Vec<LexicalTerm>,
    /// Topical project names for assessment. They do not constrain stored membership.
    #[serde(default)]
    #[schemars(length(max = PROJECT_LIMIT))]
    project_names: Vec<ProjectName>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchConstraintsInput {
    project_membership: Option<ProjectMembershipInput>,
    /// Target provider within the registered scope.
    target_provider: Option<crate::dto::Provider>,
    /// UTC seconds [start,end) or inclusive calendar dates with an IANA timezone.
    created_range: Option<QueryRangeInput>,
    /// UTC seconds [start,end) or inclusive calendar dates with an IANA timezone.
    last_active_range: Option<QueryRangeInput>,
    archive_state: Option<ArchiveStateInput>,
    /// Explicit search target. The server derives the excluded caller separately.
    #[schemars(regex(pattern = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"))]
    native_session_id: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ArchiveStateInput {
    Active,
    Archived,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProjectMembershipInput {
    mode: MembershipModeInput,
    #[schemars(length(min = 1, max = PROJECT_LIMIT))]
    names: Vec<ProjectName>,
    /// Observed means the recorded project label. Verified requires verified membership evidence.
    basis: MembershipBasisInput,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum MembershipModeInput {
    Include,
    Exclude,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum MembershipBasisInput {
    Observed,
    Verified,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum OutcomeInput {
    Investigated,
    Proposed,
    Attempted,
    Implemented,
    Tested,
    Validated,
    Resolved,
    Reverted,
    Failed,
    Unresolved,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum DispositionInput {
    VerifiedRelevant,
    Related,
    Contradiction,
    Insufficient,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum EvidenceRelationshipInput {
    Direct,
    Discussion,
    Retrospective,
    Contradiction,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AssessmentInput {
    native_session_id: String,
    disposition: DispositionInput,
    /// Cite handles from this candidate's visible evidence. Continuation handles are navigation only.
    #[schemars(length(max = 4))]
    evidence: Vec<usize>,
    #[serde(default)]
    #[schemars(length(max = 8))]
    outcomes: Vec<OutcomeInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    activity: Option<ActivityInput>,
    /// Progressive verified_relevant judgments require direct evidence of work in this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relationship: Option<EvidenceRelationshipInput>,
    #[schemars(length(min = 1, max = 1024))]
    explanation: String,
    /// For insufficient judgments, describe the gap in explanation or supply a short text or list here.
    missing_evidence: Option<EvidenceGapInput>,
    #[serde(default)]
    plausible_alternative: bool,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
enum EvidenceGapInput {
    Text(#[schemars(length(min = 1, max = 512))] String),
    Details(#[schemars(length(max = 4))] Vec<EvidenceGapDetail>),
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
struct EvidenceGapDetail(#[schemars(length(min = 1, max = 120))] String);

impl EvidenceGapInput {
    fn normalize(self) -> Result<Option<Self>, ErrorData> {
        let invalid = || {
            resolution_invalid(
                "invalid_assessment",
                "Use one short missing_evidence string or up to four short details.",
            )
        };
        let text = match self {
            Self::Text(text) => text,
            Self::Details(details) => {
                if details.is_empty() {
                    return Ok(None);
                }
                if details.len() > 4
                    || details
                        .iter()
                        .any(|d| d.0.trim().is_empty() || d.0.len() > 120)
                {
                    return Err(invalid());
                }
                details
                    .into_iter()
                    .map(|d| d.0)
                    .collect::<Vec<_>>()
                    .join(" / ")
            }
        };
        if text.trim().is_empty() || text.len() > 512 || text.chars().any(char::is_control) {
            return Err(invalid());
        }
        Ok(Some(Self::Text(text)))
    }
}

impl ResolutionInput {
    fn action(self) -> Result<ResolutionAction, ErrorData> {
        use ResolutionAction as Action;
        fn decode<I: Serialize, O: serde::de::DeserializeOwned>(input: I) -> Result<O, ErrorData> {
            serde_json::to_value(input)
                .and_then(serde_json::from_value)
                .map_err(|_| {
                    resolution_invalid(
                        "invalid_resolution_request",
                        "Use the documented fields for this action.",
                    )
                })
        }
        Ok(match self {
            Self::Start { search_intent } => {
                let intent: SearchIntent = decode(search_intent)?;
                Action::Start(Box::new(intent.validate().map_err(intent_error)?))
            }
            Self::Advance {
                run,
                expected_revision,
                operation_id,
                assessments,
                context_requests,
                publish,
                keep_alive,
                cancel,
                luna_unavailable,
            } => Action::Advance {
                run,
                step: Box::new(session_resolution::Advance {
                    expected_revision,
                    operation_id,
                    assessments: decode(normalize_assessments(assessments)?)?,
                    context_requests: context_requests
                        .into_iter()
                        .map(|request| {
                            Ok(session_resolution::ContextRequest {
                                anchor: request.anchor,
                                window: request.window.map(decode).transpose()?,
                            })
                        })
                        .collect::<Result<_, ErrorData>>()?,
                    publish,
                    keep_alive,
                    cancel,
                    luna_unavailable,
                }),
            },
        })
    }
}

fn normalize_assessments(
    mut assessments: Vec<AssessmentInput>,
) -> Result<Vec<AssessmentInput>, ErrorData> {
    for assessment in &mut assessments {
        if let Some(gap) = assessment.missing_evidence.take() {
            assessment.missing_evidence = gap.normalize()?;
        }
    }
    Ok(assessments)
}

fn intent_error(error: SearchIntentError) -> ErrorData {
    resolution_invalid(error.code(), &error.to_string())
}

fn resolution_invalid(code: &'static str, correction: &str) -> ErrorData {
    ErrorData::invalid_params(
        correction.to_owned(),
        Some(serde_json::json!({"error": code})),
    )
}

fn resolution_request(args: &serde_json::Value) -> Result<ResolutionAction, ErrorData> {
    if let Some(version) = args
        .get("search_intent")
        .and_then(|intent| intent.get("schema_version"))
        && version
            .as_str()
            .is_some_and(|value| value != SEARCH_INTENT_VERSION)
    {
        return Err(intent_error(SearchIntentError::UnsupportedSchemaVersion));
    }
    let input: ResolutionInput = serde_json::from_value(args.clone()).map_err(|error| {
        // Only return sanitized categories. Serde's error can include private
        // argument content in its unknown-field, variant and type diagnostics.
        let message = error.to_string();
        if message.starts_with("unknown field") {
            resolution_invalid("unknown_field", r#"Remove unknown fields. Minimal start: {"action":"start","search_intent":{"schema_version":"1","subject":"topic","hints":{"terms":["term"]}}}."#)
        } else if message.starts_with("unknown variant") {
            resolution_invalid("invalid_enum", "Use an enum value from the advertised schema.")
        } else {
            resolution_invalid("invalid_resolution_request",
                "Use the required fields and types for this action. Start takes search_intent with schema_version, subject and hints. Follow-up actions take the returned run.")
        }
    })?;
    input.action()
}

#[derive(Serialize, JsonSchema)]
struct ResolutionCitationView {
    handle: usize,
    #[serde(flatten)]
    evidence: EvidenceView,
}

#[derive(Serialize, JsonSchema)]
struct ResolutionCandidateView {
    native_session_id: String,
    title: Option<String>,
    observed_project: Option<String>,
    provider: &'static str,
    match_basis: serde_json::Value,
    source_coverage: Option<SourceCoverageView>,
    evidence: Vec<ResolutionCitationView>,
    untrusted_data: bool,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum VerificationStateView {
    Confirmed,
    Unconfirmed,
    Rejected,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SearchExecutionStateView {
    InProgress,
    Completed,
    Interrupted,
}

#[derive(Serialize, JsonSchema)]
struct ResolutionMatchView {
    #[serde(flatten)]
    session: SessionLabelView,
    assessment: serde_json::Value,
    verification_state: VerificationStateView,
    evidence_refs: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
struct ResolutionView {
    run: String,
    purpose: &'static str,
    management_exclusion: &'static str,
    logical_history_coverage: &'static str,
    authenticated_caller_binding: bool,
    result_policy: ResultPolicyInput,
    coverage: SemanticCoverageView,
    progress: serde_json::Value,
    step_revision: usize,
    progressive_coverage: serde_json::Value,
    execution_state: SearchExecutionStateView,
    phase: ResolutionPhaseView,
    #[serde(flatten)]
    data: ResolutionDataView,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ResolutionPhaseView {
    Retrieving,
    Evaluating,
    Publishing,
    Completed,
    Interrupted,
}

#[derive(Serialize, JsonSchema)]
struct PreparedChoiceView {
    native_session_id: String,
    explanation: String,
}

#[derive(Serialize, JsonSchema)]
struct ResolutionWindowView {
    messages: Vec<ResolutionCitationView>,
    next: Option<usize>,
    previous: Option<usize>,
    source_coverage: SourceCoverageView,
    work: ReadWorkView,
    partial: bool,
    at_start: bool,
    at_end: bool,
}

#[derive(Serialize, JsonSchema)]
struct ResolutionContextFailureView {
    anchor: usize,
    reason: &'static str,
}

#[derive(Serialize, JsonSchema)]
struct SemanticCoverageView {
    goal: CoverageGoalInput,
    eligible_evaluation_complete: bool,
    limitation: &'static str,
}

#[derive(Serialize, JsonSchema)]
#[serde(tag = "stage", rename_all = "snake_case")]
enum ResolutionDataView {
    Candidates {
        candidates: Vec<ResolutionCandidateView>,
        ranking: &'static str,
    },
    Advanced {
        matches: Vec<ResolutionMatchView>,
        alternatives: Vec<ResolutionMatchView>,
        windows: Vec<ResolutionWindowView>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        context_failures: Vec<ResolutionContextFailureView>,
        candidates: Vec<ResolutionCandidateView>,
        outcome: Option<&'static str>,
        replayed: bool,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        invalidated: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        prepared_choices: Vec<PreparedChoiceView>,
    },
}

// Present field names to hosts that read the object-shaped part of the
// catalog. Keep exact action branches for JSON Schema-aware clients.
fn resolution_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    use serde_json::{Map, Value, json};
    let mut schema = schema_described::<ResolutionInput>();
    let root = Arc::make_mut(&mut schema);
    let Some(Value::Array(variants)) = root.remove("oneOf") else {
        return schema;
    };
    let mut properties = Map::new();
    let mut actions = Vec::new();
    for variant in &variants {
        let Some(fields) = variant["properties"].as_object() else {
            continue;
        };
        actions.push(fields["action"]["const"].clone());
        for (name, field) in fields {
            if name != "action" {
                properties.insert(name.clone(), field.clone());
            }
        }
    }
    properties.insert("action".into(), json!({"type":"string", "enum":actions}));
    root.insert("type".into(), json!("object"));
    root.insert("required".into(), json!(["action"]));
    root.insert("additionalProperties".into(), json!(false));
    root.insert("properties".into(), Value::Object(properties));
    root.insert("oneOf".into(), Value::Array(variants));
    schema
}

fn resolution_view(
    result: session_resolution::ResolutionResponse,
    projector: &Projector,
) -> ResolutionView {
    use session_resolution::ResolutionData;
    let display = |value: &str| {
        let mut text = safe(value);
        if text.len() > 256 {
            let mut end = 256;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        text
    };
    let cite = |citation: session_resolution::Citation| ResolutionCitationView {
        handle: citation.handle,
        evidence: evidence_view(
            citation.evidence,
            projector.seal_search(b"evidence", &citation.reference),
        ),
    };
    let resolved = |mut r: session_resolution::ResolvedCandidate| {
        r.metadata.title = r.metadata.title.as_deref().map(display);
        r.metadata.project_label = r.metadata.project_label.as_deref().map(display);
        let mut session = label(projector, &r.metadata, false, true);
        let first = r
            .citations
            .first()
            .expect("fresh resolution requires a citation");
        session.target = projector.discovery_ref(&r.metadata.identity, &first.binding_revision);
        session.removal_eligibility = "discovery_only";
        ResolutionMatchView {
            session,
            verification_state: match r.assessment.disposition.verification_state() {
                agentmender_domain::CandidateVerificationState::Confirmed => {
                    VerificationStateView::Confirmed
                }
                agentmender_domain::CandidateVerificationState::Unconfirmed => {
                    VerificationStateView::Unconfirmed
                }
                agentmender_domain::CandidateVerificationState::Rejected => {
                    VerificationStateView::Rejected
                }
            },
            assessment: serde_json::to_value(r.assessment).expect("typed assessment"),
            evidence_refs: r
                .citations
                .iter()
                .map(|r| projector.seal_search(b"evidence", r))
                .collect(),
        }
    };
    let candidate = |c: session_resolution::ResolutionCandidate| ResolutionCandidateView {
        native_session_id: c.metadata.native_session_id,
        title: c.metadata.title.as_deref().map(display),
        observed_project: c.metadata.project_label.as_deref().map(display),
        provider: "codex",
        match_basis: serde_json::to_value(c.match_basis).expect("typed match basis"),
        source_coverage: c.source_coverage.map(Into::into),
        evidence: c.evidence.into_iter().map(cite).collect(),
        untrusted_data: true,
    };
    let window = |w: session_resolution::ResolutionWindow| ResolutionWindowView {
        messages: w.messages.into_iter().map(cite).collect(),
        next: w.next,
        previous: w.previous,
        source_coverage: w.source_coverage.into(),
        work: w.work.into(),
        partial: w.partial,
        at_start: w.at_start,
        at_end: w.at_end,
    };
    let data = match result.data {
        ResolutionData::Candidates(candidates) => ResolutionDataView::Candidates {
            ranking: "frozen native UUID order; FTS5/BM25 passage selection within source slices",
            candidates: candidates.into_iter().map(candidate).collect(),
        },
        ResolutionData::Advanced(reply) => ResolutionDataView::Advanced {
            matches: reply.matches.into_iter().map(resolved).collect(),
            alternatives: reply.alternatives.into_iter().map(resolved).collect(),
            windows: reply.windows.into_iter().map(window).collect(),
            context_failures: reply
                .context_failures
                .into_iter()
                .map(|failure| ResolutionContextFailureView {
                    anchor: failure.anchor,
                    reason: failure.reason,
                })
                .collect(),
            candidates: reply.candidates.into_iter().map(candidate).collect(),
            outcome: reply.outcome.map(|outcome| match outcome {
                session_resolution::TerminalOutcome::CompletedWithMatches => {
                    "completed_with_matches"
                }
                session_resolution::TerminalOutcome::CompletedNoMatches => "completed_no_matches",
                session_resolution::TerminalOutcome::Partial => "partial",
                session_resolution::TerminalOutcome::Blocked => "blocked",
                session_resolution::TerminalOutcome::Cancelled => "cancelled",
            }),
            replayed: reply.replayed,
            invalidated: reply.invalidated,
            prepared_choices: reply
                .prepared_choices
                .into_iter()
                .map(|choice| PreparedChoiceView {
                    native_session_id: choice.native_session_id,
                    explanation: safe(&choice.explanation),
                })
                .collect(),
        },
    };
    ResolutionView {
        run: result.run,
        purpose: "discovery",
        management_exclusion: "metadata_derived_not_host_authenticated",
        logical_history_coverage: "partial",
        authenticated_caller_binding: false,
        result_policy: match result.result_policy {
            search_intent::ResultPolicy::AllRelevant => ResultPolicyInput::AllRelevant,
            search_intent::ResultPolicy::SingleBest => ResultPolicyInput::SingleBest,
        },
        coverage: SemanticCoverageView {
            goal: match result.coverage_goal {
                search_intent::CoverageGoal::Standard => CoverageGoalInput::Standard,
                search_intent::CoverageGoal::Exhaustive => CoverageGoalInput::Exhaustive,
            },
            eligible_evaluation_complete: false,
            limitation: "lexical_candidates_only",
        },
        progress: serde_json::to_value(result.progress).expect("typed progress"),
        step_revision: result.revision,
        phase: match result.phase {
            session_resolution::ResolutionPhase::Retrieving => ResolutionPhaseView::Retrieving,
            session_resolution::ResolutionPhase::Evaluating => ResolutionPhaseView::Evaluating,
            session_resolution::ResolutionPhase::Publishing => ResolutionPhaseView::Publishing,
            session_resolution::ResolutionPhase::Completed => ResolutionPhaseView::Completed,
            session_resolution::ResolutionPhase::Interrupted => ResolutionPhaseView::Interrupted,
        },
        execution_state: match result.execution_state {
            agentmender_domain::SearchExecutionState::InProgress => {
                SearchExecutionStateView::InProgress
            }
            agentmender_domain::SearchExecutionState::Completed => {
                SearchExecutionStateView::Completed
            }
            agentmender_domain::SearchExecutionState::Interrupted => {
                SearchExecutionStateView::Interrupted
            }
        },
        progressive_coverage: serde_json::to_value(result.progressive_coverage)
            .expect("typed progressive coverage"),
        data,
    }
}

pub(super) fn tools(discovery: bool) -> Vec<Tool> {
    let mut context_output = schema::<ContextOutput>();
    if !discovery {
        // Only the UUID bootstrap in discovery mode issues a new signed target.
        Arc::make_mut(&mut context_output)
            .get_mut("$defs")
            .and_then(|defs| defs.get_mut("ContextWindowView"))
            .and_then(|window| window.get_mut("properties"))
            .and_then(serde_json::Value::as_object_mut)
            .expect("window properties")
            .remove("target");
    }
    let mut registry = vec![
        (
            "session_search",
            "Search typed filters and text evidence. UNKNOWN stays separate. Management requires verified caller binding.",
            schema::<SearchInput>(),
            schema::<SearchOutput>(),
            true,
        ),
        (
            "session_context",
            if discovery {
                "Open the current Codex source by native_session_id or navigate using signed target and evidence_ref. Read-only with caller exclusion."
            } else {
                "Retrieve source-bound untrusted text using a signed target and evidence reference."
            },
            if discovery {
                schema::<ContextInput>()
            } else {
                schema::<SignedContextInput>()
            },
            context_output,
            true,
        ),
        (
            "preview_removal",
            "Fresh removal review when admitted; read-only deployments return a compatibility assessment without a plan or execution authority",
            schema::<PreviewInput>(),
            schema::<PreviewOutput>(),
            true,
        ),
        (
            "prepare_removal",
            "Persist the exact reviewed plan and recovery metadata; never grant human approval or delete",
            schema::<PrepareInput>(),
            schema::<TransactionsView>(),
            false,
        ),
        (
            "removal_status",
            "Recover durable primary-removal results across client disconnect or restart; no redispatch",
            schema::<StatusInput>(),
            schema::<TransactionsView>(),
            true,
        ),
        (
            "request_removal_execution",
            "Return transaction status and trusted external-terminal handoff; this deployment cannot execute via MCP",
            schema::<HandoffInput>(),
            schema::<TransactionsView>(),
            true,
        ),
    ];
    if discovery {
        let mut input = schema::<SearchInput>();
        let input_map = Arc::make_mut(&mut input);
        input_map
            .get_mut("properties")
            .and_then(serde_json::Value::as_object_mut)
            .expect("search properties")
            .remove("purpose");
        input_map
            .get_mut("$defs")
            .and_then(serde_json::Value::as_object_mut)
            .expect("search definitions")
            .remove("SearchPurposeInput");
        registry.insert(0, (
            "session_discover",
            "Read-only inventory for deterministic listing, exact identity and potential deletion candidates. Excludes the apparent caller. Results carry no removal authority. Use session_resolve for conversational intent.",
            input,
            schema::<DiscoveryOutput>(),
            true,
        ));
        registry.insert(0, (
            "session_resolve",
            "Use one Luna Medium Planner and close it before start. Advance the frozen UUID retrieval without Luna. When retrieval completes, use one distinct Luna Medium Evaluator across all candidate batches and any single_best selection. Close it before advance with publish:true. Pass the latest step_revision and collect signed rows in two tables. If a named role fails, attempt the explicit gpt-6-luna Medium host route or the controlled App Server described in the server instructions. Report the observed failure.",
            resolution_schema(),
            schema::<ResolutionView>(),
            true,
        ));
    }
    registry
        .into_iter()
        .map(|(name, description, input, output, read)| {
            let mut tool = Tool::new(name, description, input);
            tool.output_schema = Some(output);
            tool.annotations = Some(
                ToolAnnotations::new()
                    .read_only(read)
                    .destructive(false)
                    .idempotent(name != "session_resolve")
                    .open_world(true),
            );
            tool
        })
        .collect()
}
fn hex(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn cursor_kind(purpose: SearchPurpose) -> &'static [u8] {
    match purpose {
        SearchPurpose::Discovery => b"discovery-cursor",
        _ => b"cursor",
    }
}
fn purpose_labels(purpose: SearchPurpose) -> (&'static str, &'static str) {
    match purpose {
        SearchPurpose::Discovery => ("discovery", "metadata_derived_not_host_authenticated"),
        SearchPurpose::Diagnostic => ("diagnostic", "not_guaranteed_diagnostic"),
        SearchPurpose::Management => ("management", "trusted_composition_binding"),
    }
}
pub(super) fn request(
    name: &str,
    args: &serde_json::Value,
    projector: &Projector,
) -> Result<ConversationRequest, ErrorData> {
    let invalid = || ErrorData::invalid_params("invalid input", None);
    let valid_target = |v: &EntityRef| {
        if !v.valid()
            || v.provider != crate::dto::Provider::Codex
            || v.class != crate::dto::EntityClass::Session
        {
            Err(invalid())
        } else {
            Ok(())
        }
    };
    match name {
        "session_resolve" => Ok(ConversationRequest::Resolve(resolution_request(args)?)),
        "session_search" | "session_discover" => {
            let discovery = name == "session_discover";
            if discovery && args.get("purpose").is_some() {
                return Err(invalid());
            }
            let v: SearchInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            if discovery && v.page_size.is_some_and(|size| !(1..=20).contains(&size)) {
                return Err(resolution_invalid(
                    "invalid_page_size",
                    "Set page_size to an integer from 1 to 20, or omit it.",
                ));
            }
            if v.query.len() > 256 || v.query.chars().any(char::is_control) {
                return Err(invalid());
            }
            if discovery
                || v.include_content
                || matches!(v.purpose, SearchPurposeInput::Management)
                || v.project_label.is_some()
                || v.page_size.is_some()
                || v.cursor.is_some()
                || !v.excerpts
                || v.structured.is_some()
            {
                let parameters = agentmender_domain::SearchParameters {
                    query: v.query,
                    include_archived: v.include_archived,
                    include_content: v.include_content,
                    project_label: v.project_label,
                    purpose: if discovery {
                        SearchPurpose::Discovery
                    } else {
                        v.purpose.into()
                    },
                    page_size: v.page_size.unwrap_or(20),
                    excerpts: v.excerpts,
                    structured: v
                        .structured
                        .map(|s| serde_json::to_value(s).and_then(serde_json::from_value))
                        .transpose()
                        .map_err(|_| invalid())?,
                };
                if !parameters.valid() {
                    return Err(invalid());
                }
                let continuation = v
                    .cursor
                    .map(|c| {
                        projector
                            .open_search(cursor_kind(parameters.purpose), &c)
                            .ok_or_else(invalid)
                    })
                    .transpose()?;
                return Ok(ConversationRequest::EvidenceSearch {
                    parameters,
                    continuation: continuation.map(Box::new),
                });
            }
            Ok(ConversationRequest::Search {
                query: v.query,
                include_archived: v.include_archived,
            })
        }
        "session_context" => {
            let v: ContextInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            let options = v
                .window
                .map(|window| {
                    serde_json::to_value(window)
                        .and_then(serde_json::from_value::<agentmender_domain::ContextOptions>)
                        .map_err(|_| invalid())
                        .and_then(|options| options.valid().then_some(options).ok_or_else(invalid))
                })
                .transpose()?;
            let (reference, target) = match (v.native_session_id, v.target, v.evidence_ref) {
                (Some(native_session_id), None, None)
                    if agentmender_domain::ApparentCodexCaller::from_metadata(
                        &native_session_id,
                        None,
                    )
                    .is_some() =>
                {
                    let mut options = options.unwrap_or_default();
                    if args
                        .get("window")
                        .and_then(|window| window.get("direction"))
                        .is_none()
                    {
                        options.direction = agentmender_domain::ContextDirection::Start;
                    }
                    if !matches!(
                        options.direction,
                        agentmender_domain::ContextDirection::Start
                            | agentmender_domain::ContextDirection::Occurrences
                    ) || options.message_offset.is_some()
                    {
                        return Err(invalid());
                    }
                    return Ok(ConversationRequest::ContextById {
                        native_session_id,
                        options,
                    });
                }
                (None, Some(target), Some(evidence_ref)) => {
                    valid_target(&target)?;
                    let reference: ContextReference = projector
                        .open_search(b"evidence", &evidence_ref)
                        .ok_or_else(invalid)?;
                    let selector = if reference.purpose == SearchPurpose::Discovery {
                        projector.discovery_selector(&target, &reference.binding_revision)
                    } else {
                        projector.selector(&target)
                    };
                    (reference, selector)
                }
                _ => return Err(invalid()),
            };
            if let Some(options) = options {
                return Ok(ConversationRequest::ContextWindow {
                    reference,
                    target,
                    options,
                });
            }
            Ok(ConversationRequest::Context { reference, target })
        }
        "preview_removal" => {
            let v: PreviewInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            valid_target(&v.target)?;
            Ok(ConversationRequest::Preview {
                target: projector.selector(&v.target),
            })
        }
        "prepare_removal" => {
            let v: PrepareInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            valid_target(&v.target)?;
            if !v
                .expected_plan_digest
                .strip_prefix("sha256:")
                .is_some_and(hex)
            {
                return Err(invalid());
            }
            Ok(ConversationRequest::Prepare {
                target: projector.selector(&v.target),
                expected_digest: v.expected_plan_digest,
            })
        }
        "removal_status" => {
            let v: StatusInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            if v.transaction_id.as_ref().is_some_and(|s| !hex(s)) {
                return Err(invalid());
            }
            Ok(ConversationRequest::Status {
                transaction_id: v.transaction_id,
            })
        }
        "request_removal_execution" => {
            let v: HandoffInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            if !hex(&v.transaction_id) {
                return Err(invalid());
            }
            Ok(ConversationRequest::RequestExecution {
                transaction_id: v.transaction_id,
            })
        }
        _ => Err(invalid()),
    }
}
fn safe(v: &str) -> String {
    v.chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}
fn label(
    projector: &Projector,
    s: &agentmender_domain::SessionMetadata,
    can_preview: bool,
    assessment: bool,
) -> SessionLabelView {
    let title = s.title.as_deref().map(safe);
    SessionLabelView {
        target: projector.entity_ref(&s.identity),
        namespace_ref: projector.namespace_ref(&s.identity),
        native_session_id: s.native_session_id.clone(),
        display_label: title
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("Codex session {}", s.native_session_id)),
        title,
        project_label: s.project_label.as_deref().map(safe),
        created_at: s.created_at,
        last_active_at: s.last_active_at,
        state: serde_json::to_value(s.state)
            .expect("state")
            .as_str()
            .expect("state enum")
            .into(),
        exact_profile: if assessment {
            "no_mutation_profile_selected"
        } else {
            "codex-0.166.0-darwin-arm64-scoped-cleanup-v1"
        },
        producer_version_matches: s.producer_version_matches,
        producer_version: s.producer_version.clone(),
        parent_native_session_id: s.parent_native_session_id.clone(),
        // Protected identities are refused before reaching this projection.
        protected: false,
        can_request_preview: can_preview,
        removal_eligibility: if assessment {
            "read_only_assessment_no_execution"
        } else if !can_preview {
            "unavailable"
        } else {
            "requires_fresh_exact_preview"
        },
    }
}
fn evidence_view(
    e: agentmender_domain::ConversationEvidence,
    evidence_ref: String,
) -> EvidenceView {
    EvidenceView {
        source: e.source,
        capture: if e.position.is_some() {
            "source_bound_streamed_message_chunk"
        } else {
            "bounded_legacy_capture_ordinal"
        },
        record_index: e.record_index,
        excerpt: e.excerpt,
        truncated: e.truncated,
        format: e.format,
        evidence_ref,
        untrusted_data: true,
        position: e.position.map(|p| MessagePositionView {
            ordinal: p.ordinal,
            message_id: p.message_id.map(|v| safe(&v)),
            turn_id: p.turn_id.map(|v| safe(&v)),
            timestamp: p.timestamp.map(|v| safe(&v)),
            chunk_offset: p.chunk_offset,
            chunk_total: p.chunk_total,
        }),
    }
}
pub(super) fn project(
    response: ConversationResponse,
    projector: &Projector,
) -> Result<serde_json::Value, crate::projection::ProjectionError> {
    use crate::projection::ProjectionError;
    let value = match response {
        ConversationResponse::Resolution(result) => serde_json::to_value(resolution_view(*result, projector)),
        ConversationResponse::EvidenceSearch(result) => {
            let search_purpose = result.parameters.purpose;
            let discovery = search_purpose == SearchPurpose::Discovery;
            let (purpose, exclusion) = purpose_labels(search_purpose);
            let project_hit = |hit: agentmender_domain::SessionSearchHit, verified: bool| {
                let content_coverage = if hit.content_complete { "complete" } else if !result.parameters.reads_content() { "not_requested" } else if hit.evidence.is_empty() { "unavailable_or_partial" } else { "partial" };
                let evidence = hit.evidence.into_iter().map(|e| {
                    let reference = agentmender_domain::ContextReference {
                        decoder_progress: None,
                        native_session_id: hit.metadata.native_session_id.clone(),
                        local_id: hit.metadata.identity.local_id.clone(),
                        record_index: e.record_index,
                        source_fingerprint: e.source_fingerprint.clone(),
                        source_bytes: e.source_bytes,
                        purpose: result.parameters.purpose,
                        binding_revision: result.binding_revision.clone(),
                        position: e.position.clone(),
                    };
                    evidence_view(e, projector.seal_search(b"evidence", &reference))
                }).collect();
                let mut session = label(projector, &hit.metadata, !discovery && verified && hit.metadata.state != agentmender_domain::SessionState::Unknown, true);
                if discovery {
                    session.target = projector.discovery_ref(&hit.metadata.identity, &result.binding_revision);
                    session.removal_eligibility = "discovery_only";
                }
                EvidenceHitView {
                    source_coverage: hit.source_coverage.map(Into::into),
                    session,
                    score: hit.score,
                    matched_terms: hit.matched_terms,
                    match_sources: hit.match_sources,
                    content_status: hit.content_status,
                    content_coverage,
                    hard_filters_verified: verified,
                    evidence,
                }
            };
            let sessions: Vec<_> = result.page.sessions.into_iter().map(|hit| project_hit(hit,true)).collect();
            let unresolved_candidates: Vec<_> = result.page.unresolved.into_iter().map(|hit| project_hit(hit,false)).collect();
            let coverage = result.page.coverage;
            let view = EvidenceSearchView {
                query_interpretation: result.parameters.structured.as_ref().map(query_interpretation).transpose().map_err(|_| ProjectionError::ResponseTooLarge)?,
                presentation_complete: result.page.next.is_none(),
                coverage_requirement_met: result.coverage_requirement_met,
                result_count: sessions.len(), sessions, unresolved_candidates,
                purpose: purpose.into(),
                management_exclusion: exclusion.into(),
                authenticated_caller_binding: false,
                registration_instance: result.registration_instance,
                completeness: "partial",
                search_basis: if result.parameters.reads_content() { "streamed_supported_user_assistant_content" } else { "structured_or_legacy_metadata" },
                ranking: "deterministic structured order or bounded lexical candidate ranking; not global semantic ranking; see coverage",
                coverage: SearchCoverageView {
                    work: coverage.work.into(),
                    eligible_observed: coverage.eligible_observed,
                    sessions_inspected: coverage.sessions_inspected,
                    bytes_read: coverage.bytes_read,
                    matched_in_batch: coverage.matched_in_batch,
                    unsupported_sessions: coverage.unsupported_sessions,
                    truncated_sessions: coverage.truncated_sessions,
                    suppressed_records: coverage.suppressed_records,
                    unreadable_sessions: coverage.unreadable_sessions,
                    unresolved_sessions: coverage.unresolved_sessions,
                    fully_traversed_sessions: coverage.fully_traversed_sessions,
                    reasons: coverage.reasons,
                },
                next_cursor: result.page.next.map(|next| projector.seal_search(cursor_kind(search_purpose), &next)),
            };
            if discovery {
                serde_json::to_value(DiscoveryOutput::Evidence(Box::new(view)))
            } else {
                serde_json::to_value(SearchOutput::Evidence(view))
            }
        }
        ConversationResponse::ContextWindow(result) => {
            let (purpose, exclusion) = purpose_labels(result.reference.purpose);
            let suggested_window = |direction: agentmender_domain::ContextDirection| {
                let options = agentmender_domain::ContextOptions { direction, message_offset: None, ..result.options.clone() };
                serde_json::to_value(options).and_then(serde_json::from_value::<ContextOptionsInput>)
                    .map_err(|_| ProjectionError::ResponseTooLarge)
            };
            let next_window = result.next_options().map(|options| {
                serde_json::to_value(options).and_then(serde_json::from_value::<ContextOptionsInput>)
                    .map_err(|_| ProjectionError::ResponseTooLarge)
            }).transpose()?;
            if result.window.next.as_ref() == Some(&result.reference)
                && next_window.as_ref().is_some_and(|next|
                    serde_json::to_value(next).ok() == serde_json::to_value(&result.options).ok())
            {
                return Err(ProjectionError::StaleCursor);
            }
            let previous_window = result.window.previous.as_ref().map(|_|
                suggested_window(agentmender_domain::ContextDirection::Backward)).transpose()?;
            let messages = result.window.messages.into_iter().map(|e| {
                let mut reference = result.reference.clone();
                reference.record_index = e.record_index;
                reference.source_fingerprint = e.source_fingerprint.clone();
                reference.source_bytes = e.source_bytes;
                reference.position = e.position.clone();
                reference.decoder_progress = None;
                evidence_view(e, projector.seal_search(b"evidence", &reference))
            }).collect();
            serde_json::to_value(ContextOutput::Window(ContextWindowView {
                target: result.opened_identity.as_ref().map(|identity|
                    projector.discovery_ref(identity, &result.reference.binding_revision)),
                source_coverage: result.window.source_coverage.into(),
                work: result.window.work.into(),
                messages,
                previous_window, next_window,
                previous_ref: result.window.previous.map(|r| projector.seal_search(b"evidence", &r)),
                next_ref: result.window.next.map(|r| projector.seal_search(b"evidence", &r)),
                at_start: result.window.at_start, at_end: result.window.at_end,
                partial: result.window.partial, bytes_read: result.window.bytes_read,
                purpose: purpose.into(),
                management_exclusion: exclusion,
            }))
        }
        ConversationResponse::Context(result) => {
            let (purpose, exclusion) = purpose_labels(result.purpose);
            serde_json::to_value(ContextOutput::Message(ContextView {
                purpose: purpose.into(),
                management_exclusion: exclusion,
                evidence: {
                    let mut reference = result.reference;
                    reference.position = result.evidence.position.clone();
                    evidence_view(result.evidence, projector.seal_search(b"evidence", &reference))
                },
            }))
        }
        ConversationResponse::Search {
            sessions,
            completeness,
            read_compatibility,
        } => serde_json::to_value(SearchOutput::Legacy(SearchView {
            sessions: sessions
                .into_iter()
                .map(|s| label(projector, &s.metadata, s.can_request_preview, read_compatibility.is_some()))
                .collect(),
            completeness: serde_json::to_value(completeness)
                .expect("coverage")
                .as_str()
                .expect("coverage enum")
                .into(),
            search_basis: "metadata_only_all_terms_no_transcript_search",
            purpose:"diagnostic",
            management_exclusion:"registered_only_no_caller_guarantee",
            read_compatibility: read_compatibility.as_ref().map(ReadCompatibilityView::from),
        })),
        ConversationResponse::ReadOnlyPreview {session, compatibility} => serde_json::to_value(PreviewOutput::ReadOnly(ReadOnlyPreviewView {
            session:label(projector,&session,true,true),compatibility:ReadCompatibilityView::from(&compatibility),
            operation:"not_admitted",status:"blocked",execution_permitted:false,
            affected_session_ids:vec![session.native_session_id],affected_set_complete:false,
            descendant_effects:"native_delete_may_include_spawned_descendants; complete effect closure is unverified",
            irreversible_effects:"a future admitted delete would remove provider history; this assessment changes nothing",
            residuals:"not_assessed; no artifact erasure or physical reclaim claim",
            writer_prerequisites:"an admitted operation must prove its own concurrency and active-session contract",
            verification_postconditions:"a future operation must prove target primary absence and unrelated-session preservation",
            independent_human_approval_required:true,
            blockers:vec!["no_admitted_operation_selected","native_effect_contract_unverified","management_caller_not_authenticated","independent_approval_deployment_not_verified"],
        })),
        ConversationResponse::Preview { session, inputs } => {
            let c = inputs.plan.content();
            let m = c
                .cleanup_manifest
                .as_ref()
                .ok_or(ProjectionError::ResponseTooLarge)?;
            let enum_name = |v: serde_json::Value| v.as_str().unwrap_or("unknown").to_owned();
            let review = PrimaryReviewView {
                plan_digest: inputs.plan.digest().to_string(),
                state_fingerprint: inputs.plan.state_fingerprint().to_string(),
                digest_basis: "original_unredacted_scoped_plan_v2",
                status: enum_name(serde_json::to_value(c.status).expect("status")),
                version: m.profile.version.clone(),
                adapter: m.profile.adapter.clone(),
                executable_sha256: m.profile.executable_sha256.clone(),
                schema_sha256: m.profile.schema_sha256.clone(),
                affected_session_ids: vec![m.native_session_id.to_string()],
                required_payload_count: m.payloads.len(),
                target_row_sets: m
                    .rows
                    .iter()
                    .map(|r| RowSetView {
                        table: r.table.clone(),
                        expected_count: r.expected_count,
                    })
                    .collect(),
                retained_primary_file_count: m.retained_files.len(),
                protected_sessions: c
                    .artifacts
                    .iter()
                    .filter(|a| {
                        a.explicitly_protected
                            && a.id.class == agentmender_domain::GraphNodeKind::Session
                    })
                    .map(|a| projector.entity_ref(&a.id))
                    .collect(),
                logical_planned_bytes: c.space.logical.total_bytes,
                known_residuals: !m.residuals.is_empty(),
                postconditions: c
                    .postconditions
                    .iter()
                    .map(|v| enum_name(serde_json::to_value(v).expect("postcondition")))
                    .collect(),
                blockers: c
                    .blockers
                    .iter()
                    .map(|v| enum_name(serde_json::to_value(v).expect("blocker")))
                    .collect(),
            };
            serde_json::to_value(PreviewOutput::Supported(PreviewView {
                session: label(projector, &session, true, false),
                review,
                operation: "scoped_primary_cleanup_v1",
                irreversible: true,
                recovery: "reconciliation_metadata_only_no_payload_restore",
                execution_channel: "trusted_external_terminal",
            }))
        }
        ConversationResponse::Transactions(statuses) => {
            let statuses =
                serde_json::to_value(statuses).map_err(|_| ProjectionError::ResponseTooLarge)?;
            let transactions =
                serde_json::from_value(statuses).map_err(|_| ProjectionError::ResponseTooLarge)?;
            serde_json::to_value(TransactionsView { transactions })
        }
    }
    .map_err(|_| ProjectionError::ResponseTooLarge)?;
    if serde_json::to_vec(&value).map_or(true, |v| v.len() > 80 * 1024) {
        return Err(ProjectionError::ResponseTooLarge);
    }
    Ok(value)
}

#[cfg(test)]
mod search_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn structured_wire_contract_validates_boolean_dates_and_unknown_fields() {
        let projector = Projector::new([9; 32]);
        let query = json!({"structured":{"filter":{"op":"and","args":[
            {"op":"created","range":{"start_date":"2026-10-10","end_date":"2026-10-20","timezone":"Europe/Rome"}},
            {"op":"not","arg":{"op":"project","values":["MISSION1"]}},
            {"op":"or","args":[{"op":"text","value":"PostgreSQL"},{"op":"text","value":"MongoDB"}]}
        ]}},"include_content":true});
        let ConversationRequest::EvidenceSearch { parameters, .. } =
            request("session_search", &query, &projector).unwrap()
        else {
            panic!("structured search must use evidence path")
        };
        assert!(parameters.structured.as_ref().unwrap().valid());
        for invalid in [
            json!({"structured":{"filter":{"op":"created","range":{"start":20,"end":10}}}}),
            json!({"structured":{"filter":{"op":"created","range":{"start_date":"2026-10-10","timezone":"invalid/timezone"}}}}),
            json!({"structured":{"filter":{"op":"project","values":["MISSION1"],"ownership":true}}}),
            json!({"structured":{"filter":{"op":"sql","value":"SELECT *"}}}),
            json!({"structured":{"filter":{"op":"provider","values":["codex"]},"sort":"relevance","mode":"exhaustive"}}),
        ] {
            assert!(
                request("session_search", &invalid, &projector).is_err(),
                "{invalid}"
            );
        }
    }

    fn reference() -> agentmender_domain::ContextReference {
        agentmender_domain::ContextReference {
            decoder_progress: None,
            native_session_id: "candidate".into(),
            local_id: agentmender_domain::EntityId::new("codex:session:candidate").unwrap(),
            record_index: 42,
            source_fingerprint: "source-revision".into(),
            source_bytes: 1000,
            purpose: agentmender_domain::SearchPurpose::Diagnostic,
            binding_revision: "diagnostic_no_caller_guarantee".into(),
            position: Some(agentmender_domain::MessagePosition {
                ordinal: 42,
                ..Default::default()
            }),
        }
    }
    fn target(projector: &Projector) -> serde_json::Value {
        serde_json::to_value(projector.entity_ref(&agentmender_domain::ArtifactId {
            provider: agentmender_domain::ProviderKind::Codex,
            state_root: "/provider".into(),
            class: agentmender_domain::GraphNodeKind::Session,
            local_id: reference().local_id,
        }))
        .unwrap()
    }
    #[test]
    fn context_window_wire_rejects_unbounded_or_unsigned_navigation() {
        let projector = Projector::new([9; 32]);
        let token = projector.seal_search(b"evidence", &reference());
        let args = json!({"target":target(&projector),"evidence_ref":token,"window":{"before":5,"after":10,"max_bytes":8192}});
        assert!(matches!(
            request("session_context", &args, &projector).unwrap(),
            ConversationRequest::ContextWindow { .. }
        ));
        let legacy = json!({"target":target(&projector),"evidence_ref":token});
        assert!(matches!(
            request("session_context", &legacy, &projector).unwrap(),
            ConversationRequest::Context { .. }
        ));
        for window in [
            json!({"before":101}),
            json!({"max_bytes":16385}),
            json!({"max_messages":21}),
            json!({"direction":"occurrences"}),
            json!({"path":"/arbitrary"}),
        ] {
            let args = json!({"target":target(&projector),"evidence_ref":token,"window":window});
            assert!(request("session_context", &args, &projector).is_err());
        }
        let args = json!({"target":target(&projector),"evidence_ref":projector.seal_search(b"cursor",&reference()),"window":{}});
        assert!(request("session_context", &args, &projector).is_err());
    }
    #[test]
    fn context_window_projection_seals_each_message_and_navigation_reference() {
        let projector = Projector::new([9; 32]);
        let response = ConversationResponse::ContextWindow(
            agentmender_application::session_search::ContextWindowResult {
                opened_identity: None,
                reference: reference(),
                options: agentmender_domain::ContextOptions::default(),
                window: agentmender_domain::ContextWindow {
                    source_coverage: Default::default(),
                    work: Default::default(),
                    messages: vec![agentmender_domain::ConversationEvidence {
                        source: "assistant_message".into(),
                        record_index: 42,
                        position: reference().position,
                        excerpt: Some("The final fix follows the rejected proposal.".into()),
                        truncated: false,
                        source_fingerprint: "source-revision".into(),
                        source_bytes: 1000,
                        format: "fixture".into(),
                    }],
                    previous: Some(reference()),
                    next: Some(reference()),
                    at_start: false,
                    at_end: false,
                    partial: false,
                    bytes_read: 1000,
                },
            },
        );
        let view = project(response, &projector).unwrap();
        assert_eq!(view["messages"][0]["untrusted_data"], true);
        assert_eq!(view["messages"][0]["position"]["ordinal"], 42);
        for token in [
            &view["messages"][0]["evidence_ref"],
            &view["previous_ref"],
            &view["next_ref"],
        ] {
            let opened: agentmender_domain::ContextReference = projector
                .open_search(b"evidence", token.as_str().unwrap())
                .unwrap();
            assert_eq!(opened.native_session_id, "candidate");
            assert_eq!(opened.record_index, 42);
        }
        assert!(!view.to_string().contains("/provider"));
    }
    #[test]
    fn huge_message_continuation_does_not_reconsume_preceding_context() {
        let projector = Projector::new([9; 32]);
        let mut next = reference();
        next.position.as_mut().unwrap().chunk_offset = 8192;
        next.position.as_mut().unwrap().chunk_total = 16384;
        let response = ConversationResponse::ContextWindow(
            agentmender_application::session_search::ContextWindowResult {
                opened_identity: None,
                reference: reference(),
                options: agentmender_domain::ContextOptions {
                    before: 5,
                    after: 10,
                    ..Default::default()
                },
                window: agentmender_domain::ContextWindow {
                    source_coverage: Default::default(),
                    work: Default::default(),
                    messages: vec![],
                    previous: None,
                    next: Some(next),
                    at_start: false,
                    at_end: false,
                    partial: false,
                    bytes_read: 1000,
                },
            },
        );
        let view = project(response, &projector).unwrap();
        assert_eq!(view["next_window"]["direction"], "around");
        assert_eq!(view["next_window"]["before"], 0);
        assert_eq!(view["next_window"]["after"], 0);
    }
    #[test]
    fn identical_forward_continuation_pair_is_refused() {
        let projector = Projector::new([9; 32]);
        let result = agentmender_application::session_search::ContextWindowResult {
            opened_identity: None,
            reference: reference(),
            options: agentmender_domain::ContextOptions {
                direction: agentmender_domain::ContextDirection::Forward,
                ..Default::default()
            },
            window: agentmender_domain::ContextWindow {
                source_coverage: Default::default(),
                work: Default::default(),
                messages: vec![],
                previous: None,
                next: Some(reference()),
                at_start: false,
                at_end: false,
                partial: true,
                bytes_read: 0,
            },
        };
        assert!(matches!(
            project(ConversationResponse::ContextWindow(result), &projector),
            Err(crate::projection::ProjectionError::StaleCursor)
        ));
    }
}
