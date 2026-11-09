//! Opt-in bounded handoff DTOs. No persistence or provider access in this crate.
use crate::{dto::EntityRef, projection::Projector, protocol::schema};
use agentmender_application::{
    ApplicationError, QueryControl,
    conversation::{ConversationRequest, ConversationResponse},
};
use rmcp::model::{ErrorData, Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type ConversationExecutor = Arc<
    dyn Fn(ConversationRequest, QueryControl) -> Result<ConversationResponse, ApplicationError>
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
    #[schemars(length(max = 4096))]
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
    #[default]
    Diagnostic,
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
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ContextInput {
    target: EntityRef,
    #[schemars(length(max = 4096))]
    evidence_ref: String,
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
    #[serde(flatten)]
    session: SessionLabelView,
    score: u32,
    matched_terms: Vec<String>,
    match_sources: Vec<String>,
    content_status: String,
    hard_filters_verified: bool,
    evidence: Vec<EvidenceView>,
}
#[derive(Serialize, JsonSchema)]
struct SearchCoverageView {
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

pub(super) fn tools() -> Vec<Tool> {
    [
        ("session_search", "Search typed Boolean/date/project/provider filters and streamed text evidence. UNKNOWN candidates are separate. Management requires verified host binding and fails closed without it; diagnostic has no caller guarantee", schema::<SearchInput>(), schema::<SearchOutput>(), true),
        ("session_context", "Retrieve source-bound untrusted text. Optional window supports before/after, forward/backward/turn navigation and occurrences; signed references recheck identity, source freshness and exclusions", schema::<ContextInput>(), schema::<ContextOutput>(), true),
        ("preview_removal", "Fresh removal review when admitted; read-only deployments return a compatibility assessment without a plan or execution authority", schema::<PreviewInput>(), schema::<PreviewOutput>(), true),
        ("prepare_removal", "Persist the exact reviewed plan and recovery metadata; never grant human approval or delete", schema::<PrepareInput>(), schema::<TransactionsView>(), false),
        ("removal_status", "Recover durable primary-removal results across client disconnect or restart; no redispatch", schema::<StatusInput>(), schema::<TransactionsView>(), true),
        ("request_removal_execution", "Return transaction status and trusted external-terminal handoff; this deployment cannot execute via MCP", schema::<HandoffInput>(), schema::<TransactionsView>(), true),
    ].into_iter().map(|(name, description, input, output, read)| {
        let mut tool = Tool::new(name, description, input);
        tool.output_schema = Some(output);
        tool.annotations = Some(ToolAnnotations::new().read_only(read).destructive(false).idempotent(true).open_world(true));
        tool
    }).collect()
}
fn hex(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn request(
    name: &str,
    args: &serde_json::Value,
    projector: &Projector,
) -> Result<ConversationRequest, ErrorData> {
    let invalid = || ErrorData::invalid_params("invalid input", None);
    let target = |v: EntityRef| {
        if !v.valid()
            || v.provider != crate::dto::Provider::Codex
            || v.class != crate::dto::EntityClass::Session
        {
            Err(invalid())
        } else {
            Ok(projector.selector(&v))
        }
    };
    match name {
        "session_search" => {
            let v: SearchInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            if v.query.len() > 256 || v.query.chars().any(char::is_control) {
                return Err(invalid());
            }
            if v.include_content
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
                    purpose: v.purpose.into(),
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
                    .map(|c| projector.open_search(b"cursor", &c).ok_or_else(invalid))
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
            let reference = projector
                .open_search(b"evidence", &v.evidence_ref)
                .ok_or_else(invalid)?;
            let target = target(v.target)?;
            if let Some(window) = v.window {
                let options: agentmender_domain::ContextOptions = serde_json::to_value(window)
                    .and_then(serde_json::from_value)
                    .map_err(|_| invalid())?;
                if !options.valid() {
                    return Err(invalid());
                }
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
            Ok(ConversationRequest::Preview {
                target: target(v.target)?,
            })
        }
        "prepare_removal" => {
            let v: PrepareInput = serde_json::from_value(args.clone()).map_err(|_| invalid())?;
            if !v
                .expected_plan_digest
                .strip_prefix("sha256:")
                .is_some_and(hex)
            {
                return Err(invalid());
            }
            Ok(ConversationRequest::Prepare {
                target: target(v.target)?,
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
        ConversationResponse::EvidenceSearch(result) => {
            let management = result.parameters.purpose == agentmender_domain::SearchPurpose::Management;
            let project_hit = |hit: agentmender_domain::SessionSearchHit, verified: bool| {
                let evidence = hit.evidence.into_iter().map(|e| {
                    let reference = agentmender_domain::ContextReference {
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
                EvidenceHitView {
                    session: label(projector, &hit.metadata, verified && hit.metadata.state != agentmender_domain::SessionState::Unknown, true),
                    score: hit.score,
                    matched_terms: hit.matched_terms,
                    match_sources: hit.match_sources,
                    content_status: hit.content_status,
                    hard_filters_verified: verified,
                    evidence,
                }
            };
            let sessions: Vec<_> = result.page.sessions.into_iter().map(|hit| project_hit(hit,true)).collect();
            let unresolved_candidates: Vec<_> = result.page.unresolved.into_iter().map(|hit| project_hit(hit,false)).collect();
            let coverage = result.page.coverage;
            serde_json::to_value(SearchOutput::Evidence(EvidenceSearchView {
                query_interpretation: result.parameters.structured.as_ref().map(query_interpretation).transpose().map_err(|_| ProjectionError::ResponseTooLarge)?,
                presentation_complete: result.page.next.is_none(),
                coverage_requirement_met: result.coverage_requirement_met,
                result_count: sessions.len(), sessions, unresolved_candidates,
                purpose: if management { "management" } else { "diagnostic" }.into(),
                management_exclusion: if management { "trusted_composition_binding" } else { "not_guaranteed_diagnostic" }.into(),
                authenticated_caller_binding: false,
                registration_instance: result.registration_instance,
                completeness: "partial",
                search_basis: if result.parameters.reads_content() { "streamed_supported_user_assistant_content" } else { "structured_or_legacy_metadata" },
                ranking: "deterministic structured order or bounded lexical candidate ranking; not global semantic ranking; see coverage",
                coverage: SearchCoverageView {
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
                next_cursor: result.page.next.map(|next| projector.seal_search(b"cursor", &next)),
            }))
        }
        ConversationResponse::ContextWindow(result) => {
            let management = result.reference.purpose == agentmender_domain::SearchPurpose::Management;
            let suggested_window = |direction: agentmender_domain::ContextDirection| {
                let options = agentmender_domain::ContextOptions { direction, message_offset: None, ..result.options.clone() };
                serde_json::to_value(options).and_then(serde_json::from_value::<ContextOptionsInput>)
                    .map_err(|_| ProjectionError::ResponseTooLarge)
            };
            let next_window = result.window.next.as_ref().map(|next| {
                let direction = if next.position.as_ref().is_some_and(|p| p.chunk_offset > 0) {
                    agentmender_domain::ContextDirection::Around
                } else if matches!(result.options.direction, agentmender_domain::ContextDirection::Occurrences) {
                    agentmender_domain::ContextDirection::Occurrences
                } else { agentmender_domain::ContextDirection::Forward };
                suggested_window(direction).map(|mut options| {
                    if next.position.as_ref().is_some_and(|p| p.chunk_offset > 0) {
                        options.before = 0;
                        options.after = 0;
                    }
                    options
                })
            }).transpose()?;
            let previous_window = result.window.previous.as_ref().map(|_|
                suggested_window(agentmender_domain::ContextDirection::Backward)).transpose()?;
            let messages = result.window.messages.into_iter().map(|e| {
                let mut reference = result.reference.clone();
                reference.record_index = e.record_index;
                reference.source_fingerprint = e.source_fingerprint.clone();
                reference.source_bytes = e.source_bytes;
                reference.position = e.position.clone();
                evidence_view(e, projector.seal_search(b"evidence", &reference))
            }).collect();
            serde_json::to_value(ContextOutput::Window(ContextWindowView {
                messages,
                previous_window, next_window,
                previous_ref: result.window.previous.map(|r| projector.seal_search(b"evidence", &r)),
                next_ref: result.window.next.map(|r| projector.seal_search(b"evidence", &r)),
                at_start: result.window.at_start, at_end: result.window.at_end,
                partial: result.window.partial, bytes_read: result.window.bytes_read,
                purpose: if management { "management" } else { "diagnostic" }.into(),
                management_exclusion: if management { "trusted_composition_binding" } else { "not_guaranteed_diagnostic" },
            }))
        }
        ConversationResponse::Context(result) => {
            let management = result.purpose == agentmender_domain::SearchPurpose::Management;
            serde_json::to_value(ContextOutput::Message(ContextView {
                purpose: if management { "management" } else { "diagnostic" }.into(),
                management_exclusion: if management { "trusted_composition_binding" } else { "not_guaranteed_diagnostic" },
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
                reference: reference(),
                options: agentmender_domain::ContextOptions::default(),
                window: agentmender_domain::ContextWindow {
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
        let response = ConversationResponse::ContextWindow(
            agentmender_application::session_search::ContextWindowResult {
                reference: reference(),
                options: agentmender_domain::ContextOptions {
                    before: 5,
                    after: 10,
                    ..Default::default()
                },
                window: agentmender_domain::ContextWindow {
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
}
