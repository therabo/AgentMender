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
    #[serde(skip_serializing_if = "Option::is_none")]
    read_compatibility: Option<ReadCompatibilityView>,
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
        ("session_search", "Search opt-in title/project metadata; return all possible matches, never select by title", schema::<SearchInput>(), schema::<SearchView>(), true),
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
            Ok(ConversationRequest::Search {
                query: v.query,
                include_archived: v.include_archived,
            })
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
    protected: bool,
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
        protected,
        can_request_preview: can_preview,
        removal_eligibility: if protected {
            "protected"
        } else if assessment {
            "read_only_assessment_no_execution"
        } else if !can_preview {
            "unavailable"
        } else {
            "requires_fresh_exact_preview"
        },
    }
}
pub(super) fn project(
    response: ConversationResponse,
    projector: &Projector,
) -> Result<serde_json::Value, crate::projection::ProjectionError> {
    use crate::projection::ProjectionError;
    let value = match response {
        ConversationResponse::Search {
            sessions,
            completeness,
            read_compatibility,
        } => serde_json::to_value(SearchView {
            sessions: sessions
                .into_iter()
                .map(|s| label(projector, &s.metadata, s.protected, s.can_request_preview, read_compatibility.is_some()))
                .collect(),
            completeness: serde_json::to_value(completeness)
                .expect("coverage")
                .as_str()
                .expect("coverage enum")
                .into(),
            search_basis: "metadata_only_all_terms_no_transcript_search",
            read_compatibility: read_compatibility.as_ref().map(ReadCompatibilityView::from),
        }),
        ConversationResponse::ReadOnlyPreview {session, compatibility, protected} => serde_json::to_value(PreviewOutput::ReadOnly(ReadOnlyPreviewView {
            session:label(projector,&session,protected,!protected,true),compatibility:ReadCompatibilityView::from(&compatibility),
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
                session: label(projector, &session, false, true, false),
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
