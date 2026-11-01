//! Exact-session cleanup evidence and immutable review manifest. No I/O occurs here.
use crate::{ArtifactId, EntityId};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stable physical metadata observed without following links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupFileIdentity {
    #[serde(with = "crate::path_serde")]
    pub path: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub links: u64,
    pub size_bytes: u64,
    pub mtime_seconds: i64,
    pub mtime_nanoseconds: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupProfile {
    pub version: String,
    pub executable_sha256: String,
    pub schema_sha256: String,
    pub adapter: String,
    pub platform: String,
    pub journal_mode: String,
    pub history_layout: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupBoundaryObservation {
    CompleteAndUnshared,
    Incomplete,
    Shared,
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupRows {
    pub table: String,
    pub expected_count: u64,
    pub preimage_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupPayload {
    pub artifact: ArtifactId,
    pub identity: CleanupFileIdentity,
    pub header_session_id: EntityId,
    pub header_sha256: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupResidualKind {
    Shared,
    Referenced,
    Global,
    Unknown,
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupResidual {
    pub artifact: ArtifactId,
    pub kind: CleanupResidualKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupIndexAction {
    pub identity: CleanupFileIdentity,
    pub preimage_sha256: String,
    pub record_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupDatabasePath {
    #[serde(with = "crate::path_serde")]
    pub path: PathBuf,
}
/// Frozen review content; no transcript bodies or general mutation commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupManifest {
    pub target: ArtifactId,
    pub native_session_id: EntityId,
    pub root: CleanupFileIdentity,
    pub profile: CleanupProfile,
    pub database: CleanupFileIdentity,
    pub rows: Vec<CleanupRows>,
    /// Exact rollout paths read from the canonical target row.
    pub database_payload_paths: Vec<CleanupDatabasePath>,
    pub payloads: Vec<CleanupPayload>,
    pub optional_index: Option<CleanupIndexAction>,
    pub retained_database_sha256: String,
    pub retained_files: Vec<CleanupFileIdentity>,
    pub residuals: Vec<CleanupResidual>,
}
/// Raw adapter observations are deliberately not deserializable or imported as proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScopedCleanupEvidence {
    pub manifest: CleanupManifest,
    pub root_boundary: CleanupBoundaryObservation,
    pub references: CleanupBoundaryObservation,
    pub writer_fence: CleanupBoundaryObservation,
    pub schema: CleanupBoundaryObservation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupOutcome {
    Removed,
    RemovedWithResiduals,
    Blocked,
    OutcomeUnknown,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupActionOutcome {
    Removed,
    AlreadyAbsent,
    Retained,
    Unknown,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupActionReport {
    pub artifact: ArtifactId,
    pub outcome: CleanupActionOutcome,
    pub logical_unlinked_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReport {
    pub outcome: CleanupOutcome,
    pub actions: Vec<CleanupActionReport>,
    /// Absence at the fenced verification cutoff, not a provider return code.
    pub canonical_row_absent: bool,
    pub retained_primary_unchanged: bool,
    pub optional_index: Option<CleanupActionOutcome>,
    pub post_cutoff_residue: bool,
    pub logical_planned_bytes: u64,
    pub logical_unlinked_bytes: u64,
    pub physical_reclaimed_bytes: Option<u64>,
}

/// Verify all reported actions are exactly the approved payloads, and that
/// success is supported by required absence and retained-primary observations.
pub fn validate_cleanup_report(manifest: &CleanupManifest, report: &CleanupReport) -> bool {
    use std::collections::BTreeSet;
    if !crate::validate_cleanup_manifest(manifest)
        || report.physical_reclaimed_bytes.is_some()
        || report.actions.len() != manifest.payloads.len()
        || report.optional_index.is_some() != manifest.optional_index.is_some()
    {
        return false;
    }
    let Some(planned) = manifest
        .payloads
        .iter()
        .try_fold(0u64, |sum, p| sum.checked_add(p.identity.size_bytes))
    else {
        return false;
    };
    let mut seen = BTreeSet::new();
    let mut actual = 0u64;
    for action in &report.actions {
        let Some(payload) = manifest
            .payloads
            .iter()
            .find(|p| p.artifact == action.artifact)
        else {
            return false;
        };
        if !seen.insert(&action.artifact) {
            return false;
        }
        let expected = if action.outcome == CleanupActionOutcome::Removed {
            payload.identity.size_bytes
        } else {
            0
        };
        if action.logical_unlinked_bytes != expected {
            return false;
        }
        let Some(sum) = actual.checked_add(expected) else {
            return false;
        };
        actual = sum;
    }
    if report.logical_planned_bytes != planned
        || report.logical_unlinked_bytes != actual
        || actual > planned
    {
        return false;
    }
    let primary = report.canonical_row_absent
        && report.retained_primary_unchanged
        && report.actions.iter().all(|a| {
            matches!(
                a.outcome,
                CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
            )
        });
    let residual = !manifest.residuals.is_empty()
        || report.post_cutoff_residue
        || report.optional_index.is_some_and(|o| {
            !matches!(
                o,
                CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
            )
        });
    match report.outcome {
        CleanupOutcome::Removed => primary && !residual,
        CleanupOutcome::RemovedWithResiduals => primary && residual,
        CleanupOutcome::Blocked | CleanupOutcome::OutcomeUnknown | CleanupOutcome::Failed => {
            !primary
        }
    }
}
