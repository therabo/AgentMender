//! Exact provider profile admission. Pure policy; no filesystem or database I/O.
use agentmender_domain::{CleanupManifest, ProviderKind, validate_cleanup_manifest};
pub(super) const SCOPED_CODEX_VERSION: &str = "0.166.0";
pub(super) const SCOPED_CODEX_EXECUTABLE_SHA256: &str =
    "39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1";
pub(super) const SCOPED_CODEX_SCHEMA_SHA256: &str =
    "1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647";
pub(super) const SCOPED_ADAPTER: &str = "scoped-cleanup-v1";

pub(super) fn validate_manifest_profile(m: &CleanupManifest) -> bool {
    let root = &m.root.path;
    let p = &m.profile;
    validate_cleanup_manifest(m)
        && m.target.provider == ProviderKind::Codex
        && p.version == SCOPED_CODEX_VERSION
        && p.executable_sha256 == SCOPED_CODEX_EXECUTABLE_SHA256
        && p.schema_sha256 == SCOPED_CODEX_SCHEMA_SHA256
        && p.adapter == SCOPED_ADAPTER
        && p.platform == "darwin-arm64"
        && p.journal_mode == "wal"
        && p.history_layout == "legacy-plain"
        && m.database.path == root.join("state_5.sqlite")
        && m.rows.len() == 6
        && m.rows
            .iter()
            .any(|r| r.table == "threads" && r.expected_count == 1)
        && m.rows.iter().all(|r| {
            matches!(
                r.table.as_str(),
                "threads"
                    | "thread_dynamic_tools"
                    | "thread_read_receipts"
                    | "thread_attachments"
                    | "guardian_review_feedback"
                    | "thread_spawn_edges"
            ) && (!matches!(
                r.table.as_str(),
                "thread_attachments" | "guardian_review_feedback" | "thread_spawn_edges"
            ) || r.expected_count == 0)
        })
        && m.payloads.iter().all(|p| {
            (p.identity.path.starts_with(root.join("sessions"))
                || p.identity.path.starts_with(root.join("archived_sessions")))
                && p.identity
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| {
                        n.starts_with("rollout-")
                            && n.ends_with(&format!("-{}.jsonl", m.native_session_id.as_str()))
                    })
        })
        && m.optional_index
            .as_ref()
            .is_none_or(|i| i.identity.path == root.join("session_index.jsonl"))
}
