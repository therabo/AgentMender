//! Frozen semantic scope and observations for separately approved complete removal.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompleteRemovalAction {
    DesktopCatalog,
    GlobalBackup,
    GlobalMain,
    SessionIndex,
    PrimaryPayload,
    StateThreadRecords,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterRequirement {
    RequiresProviderClosed,
    CompatibleLifetimeFence,
    CodexClosedWithLifetimeFence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAdmission {
    PrimaryOnly,
    DesktopUnaccepted,
    CompleteAccepted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalProfile {
    pub identity: EntityId,
    pub admission: ProfileAdmission,
    pub writer_requirement: WriterRequirement,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentSurface {
    pub id: EntityId,
}
/// Fixed, reviewed shared-file commitment; absence is frozen as `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopFileCommitment {
    pub identity: Option<CleanupFileIdentity>,
    pub preimage_sha256: Option<String>,
    pub retained_sha256: Option<String>,
    pub metadata_sha256: Option<String>,
    pub target_units: u64,
}
/// The catalog's row, observation sequence, scan suppression and revision form one action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopCatalogCommitment {
    pub identity: CleanupFileIdentity,
    pub schema_sha256: String,
    pub migration_sha256: String,
    pub preimage_sha256: String,
    pub retained_sha256: String,
    pub target_rows: u64,
    pub target_sequence_sha256: String,
    pub revision_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopLegacyFile {
    pub identity: CleanupFileIdentity,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopLegacyCommitment {
    pub files: Vec<DesktopLegacyFile>,
    pub layout_id: String,
    pub origin: String,
    pub logical_legacy_entries: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopRemovalManifest {
    pub desktop_root: CleanupFileIdentity,
    pub user_data: CleanupFileIdentity,
    #[serde(with = "crate::path_serde")]
    pub bundle_path: std::path::PathBuf,
    pub bundle_id: String,
    pub bundle_version: String,
    pub bundle_build: String,
    pub asar_sha256: String,
    pub stable_worker_sha256: String,
    pub bundle_sha256: String,
    pub cli_sha256: String,
    pub local_host: String,
    pub source_kind: String,
    pub target_title: String,
    pub legacy: DesktopLegacyCommitment,
    pub catalog: DesktopCatalogCommitment,
    pub global_backup: DesktopFileCommitment,
    pub global_main: DesktopFileCommitment,
    pub session_index: DesktopFileCommitment,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalManifest {
    pub primary: CleanupManifest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<DesktopRemovalManifest>,
    pub actions: Vec<CompleteRemovalAction>,
    pub profile: CompleteRemovalProfile,
    pub required_surfaces: Vec<PersistentSurface>,
    pub technical_residuals: Vec<CleanupResidual>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompleteRemovalEvidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_boundary: Option<DesktopBoundaryEvidence>,
    pub manifest: CompleteRemovalManifest,
    pub primary_evidence: ScopedCleanupEvidence,
}
/// Fresh adapter observations, never deserialized from an external proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopBoundaryEvidence {
    pub legacy: CleanupBoundaryObservation,
    pub roots: CleanupBoundaryObservation,
    pub profile: CleanupBoundaryObservation,
    pub writer_inventory: CleanupBoundaryObservation,
    pub writers_closed: bool,
}
/// Only the reviewed expanded primary contract is executable; admission is never
/// obtained from serialized claims or a provider observation.
pub fn validate_complete_removal_manifest(m: &CompleteRemovalManifest) -> bool {
    if let Some(d) = &m.desktop {
        return compiled_desktop_mutation_profile() && valid_desktop_contract(m, d);
    }
    validate_cleanup_manifest(&m.primary)
        && m.primary.optional_index.is_none()
        && m.primary.profile.adapter == "scoped-cleanup-v2"
        && m.actions
            == [
                CompleteRemovalAction::PrimaryPayload,
                CompleteRemovalAction::StateThreadRecords,
            ]
        && m.profile.identity.as_str() == "codex-scoped-cleanup-v2"
        && m.profile.admission == ProfileAdmission::PrimaryOnly
        && m.profile.writer_requirement == WriterRequirement::CompatibleLifetimeFence
        && m.required_surfaces.len() == 1
        && m.required_surfaces[0].id.as_str() == "codex-desktop-reconstruction-unverified"
        && !m.required_surfaces.is_empty()
        && m.required_surfaces
            .iter()
            .map(|s| &s.id)
            .collect::<BTreeSet<_>>()
            .len()
            == m.required_surfaces.len()
        && m.technical_residuals == m.primary.residuals
}
fn desktop_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn safe_absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
        && !path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}
fn desktop_identity(i: &CleanupFileIdentity) -> bool {
    safe_absolute(&i.path) && i.inode != 0 && i.mtime_nanoseconds < 1_000_000_000
}
fn desktop_file(f: &DesktopFileCommitment, path: &std::path::Path, device: u64) -> bool {
    match (
        &f.identity,
        &f.preimage_sha256,
        &f.retained_sha256,
        &f.metadata_sha256,
    ) {
        (None, None, None, None) => f.target_units == 0,
        (Some(i), Some(p), Some(r), Some(metadata)) => {
            desktop_identity(i)
                && i.path == path
                && i.device == device
                && i.links == 1
                && desktop_digest(p)
                && desktop_digest(r)
                && desktop_digest(metadata)
        }
        _ => false,
    }
}
fn desktop_legacy(user_data: &CleanupFileIdentity, legacy: &DesktopLegacyCommitment) -> bool {
    let dir = user_data.path.join("Local Storage/leveldb");
    legacy.origin == "app://-"
        && legacy.logical_legacy_entries == 0
        && legacy.files.len() <= 4096
        && match legacy.layout_id.as_str() {
            "absent" => legacy.files.is_empty(),
            "chromium-local-storage-leveldb-v1" => !legacy.files.is_empty(),
            _ => false,
        }
        && legacy.files.iter().all(|f| {
            desktop_identity(&f.identity)
                && f.identity.path.parent() == Some(dir.as_path())
                && f.identity.device == user_data.device
                && f.identity.links == 1
                && desktop_digest(&f.sha256)
        })
        && legacy
            .files
            .windows(2)
            .all(|w| w[0].identity.path < w[1].identity.path)
        && legacy
            .files
            .iter()
            .map(|f| (f.identity.device, f.identity.inode))
            .collect::<BTreeSet<_>>()
            .len()
            == legacy.files.len()
}
fn desktop_distinct_files(m: &CompleteRemovalManifest, d: &DesktopRemovalManifest) -> bool {
    let mut physical = BTreeSet::new();
    std::iter::once(&m.primary.database)
        .chain(m.primary.payloads.iter().map(|p| &p.identity))
        .chain(std::iter::once(&d.catalog.identity))
        .chain(
            [&d.global_backup, &d.global_main, &d.session_index]
                .into_iter()
                .filter_map(|f| f.identity.as_ref()),
        )
        .chain(d.legacy.files.iter().map(|f| &f.identity))
        .all(|i| physical.insert((i.device, i.inode)))
}
fn compiled_desktop_mutation_profile() -> bool {
    // No exact profile has passed the owned two-cycle acceptance.
    false
}
pub(crate) fn valid_desktop_contract(
    m: &CompleteRemovalManifest,
    d: &DesktopRemovalManifest,
) -> bool {
    validate_cleanup_manifest(&m.primary)
        && m.primary.optional_index.is_none()
        && m.primary.profile.adapter == "scoped-cleanup-v2"
        && m.primary.target.provider == ProviderKind::Codex
        && m.primary.profile.version == "0.166.0"
        && m.primary.profile.schema_sha256
            == "1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647"
        && m.primary.profile.platform == "darwin-arm64"
        && m.primary.profile.journal_mode == "wal"
        && m.primary.profile.history_layout == "legacy-plain"
        && m.primary.profile.executable_sha256
            == "39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1"
        && m.profile.identity.as_str() == "codex-desktop-complete-removal-v1"
        && m.profile.admission == ProfileAdmission::DesktopUnaccepted
        && m.profile.writer_requirement == WriterRequirement::RequiresProviderClosed
        && m.actions
            == [
                CompleteRemovalAction::DesktopCatalog,
                CompleteRemovalAction::GlobalBackup,
                CompleteRemovalAction::GlobalMain,
                CompleteRemovalAction::SessionIndex,
                CompleteRemovalAction::PrimaryPayload,
                CompleteRemovalAction::StateThreadRecords,
            ]
        && d.desktop_root == m.primary.root
        && desktop_identity(&d.desktop_root)
        && desktop_identity(&d.user_data)
        && !d.user_data.path.starts_with(&d.desktop_root.path)
        && !d.desktop_root.path.starts_with(&d.user_data.path)
        && (d.user_data.device, d.user_data.inode) != (d.desktop_root.device, d.desktop_root.inode)
        && d.bundle_path == std::path::Path::new("/Applications/ChatGPT.app")
        && d.bundle_id == "com.openai.codex"
        && d.bundle_version == "26.1027.51404"
        && d.bundle_build == "23880"
        && d.bundle_sha256 == "0c1cb804f8858c8ab059a26a1640b9a66c9a5071c48f39d6618e8698da370b56"
        && d.asar_sha256 == "52e0a65f9fea51f4287bbd98c63343761f9424517bb793652bb554764bc0bcc8"
        && d.cli_sha256 == "58a91f6ea19bea477ae3a2ec48d619f0039afe75d3476260747e11d201fa7cea"
        && d.stable_worker_sha256 == m.primary.profile.executable_sha256
        && d.local_host == "local"
        && d.source_kind == "cli"
        && !d.target_title.is_empty()
        && d.target_title.len() <= 4096
        && !d.target_title.chars().any(char::is_control)
        && desktop_legacy(&d.user_data, &d.legacy)
        && desktop_distinct_files(m, d)
        && desktop_identity(&d.catalog.identity)
        && d.catalog.identity.links == 1
        && d.catalog.identity.device == m.primary.root.device
        && d.catalog.identity.path == m.primary.root.path.join("sqlite/codex.db")
        && d.catalog.schema_sha256
            == "a275a36b59fa4895e0793eb1011d7c2b2b7ee25691c2f2bf206ff2f4342cc40a"
        && d.catalog.migration_sha256
            == "5e56f348093c7c1f056a0f2afbd55d03681efa242a5ae63ad3dc3e0a6bedac45"
        && [
            &d.catalog.preimage_sha256,
            &d.catalog.retained_sha256,
            &d.catalog.target_sequence_sha256,
            &d.catalog.revision_sha256,
        ]
        .iter()
        .all(|h| desktop_digest(h))
        && d.catalog.target_rows == 1
        && desktop_file(
            &d.global_backup,
            &m.primary.root.path.join(".codex-global-state.json.bak"),
            m.primary.root.device,
        )
        && desktop_file(
            &d.global_main,
            &m.primary.root.path.join(".codex-global-state.json"),
            m.primary.root.device,
        )
        && desktop_file(
            &d.session_index,
            &m.primary.root.path.join("session_index.jsonl"),
            m.primary.root.device,
        )
        && m.required_surfaces.iter().map(|s| s.id.as_str()).eq([
            "canonical-active-archived",
            "desktop-local-catalog",
            "desktop-scan-reconstruction",
            "session-index",
            "global-main",
            "global-backup",
            "renderer-legacy-import",
            "derived-search-project-pin-recent",
            "command-history",
            "renderer-browser-storage",
        ])
        && m.technical_residuals == m.primary.residuals
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistentObservation {
    Absent,
    Present,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentSurfaceObservation {
    pub id: EntityId,
    pub observation: PersistentObservation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalActionProgress {
    pub action: CompleteRemovalAction,
    pub outcome: CleanupActionOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalReport {
    /// Independent verification of retained Desktop catalog and file content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_collateral_unchanged: Option<bool>,
    pub primary: CleanupReport,
    pub actions: Vec<CompleteRemovalActionProgress>,
    pub surfaces: Vec<PersistentSurfaceObservation>,
}
pub fn validate_complete_removal_report(
    m: &CompleteRemovalManifest,
    r: &CompleteRemovalReport,
) -> bool {
    validate_complete_removal_manifest(m)
        && (m.desktop.is_some() || r.desktop_collateral_unchanged.is_none())
        && validate_cleanup_report(&m.primary, &r.primary)
        && (!matches!(
            r.primary.outcome,
            CleanupOutcome::Blocked | CleanupOutcome::Failed
        ) || (r.actions.is_empty()
            && r.primary.actions.iter().all(|a| {
                matches!(
                    a.outcome,
                    CleanupActionOutcome::Retained | CleanupActionOutcome::Failed
                )
            })))
        && r.actions.iter().all(|a| {
            matches!(
                a.outcome,
                CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
            )
        })
        && r.actions.len() <= m.actions.len()
        && r.actions
            .iter()
            .zip(&m.actions)
            .all(|(a, b)| a.action == *b)
        && (!matches!(
            r.primary.outcome,
            CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
        ) || (r.actions.len() == m.actions.len()
            && r.actions.iter().all(|a| {
                matches!(
                    a.outcome,
                    CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
                )
            })))
        && (!matches!(
            r.primary.outcome,
            CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
        ) || (r.primary.canonical_row_absent
            && r.primary.retained_primary_unchanged
            && r.primary.actions.iter().all(|a| {
                matches!(
                    a.outcome,
                    CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
                )
            })))
        && (m.desktop.is_some()
            || r.actions.first().is_none_or(|a| {
                a.outcome != CleanupActionOutcome::AlreadyAbsent
                    || r.primary
                        .actions
                        .iter()
                        .all(|p| p.outcome != CleanupActionOutcome::Removed)
            }))
        && (m.desktop.is_none()
            || !matches!(
                r.primary.outcome,
                CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
            )
            || (r.desktop_collateral_unchanged == Some(true)
                && r.surfaces
                    .iter()
                    .all(|s| s.observation == PersistentObservation::Absent)))
        && r.surfaces.len() == m.required_surfaces.len()
        && r.surfaces
            .iter()
            .zip(&m.required_surfaces)
            .all(|(a, b)| a.id == b.id)
}
/// No currently compiled Desktop profile has isolated restart acceptance.
pub fn assess_complete_removal(
    m: &CompleteRemovalManifest,
    r: &CompleteRemovalReport,
) -> SessionRemovalAssessment {
    if !validate_complete_removal_report(m, r) {
        return SessionRemovalAssessment {
            outcome: SessionRemovalOutcome::InvalidEvidence,
            maximum_safe_cleanup: MaximumSafeCleanup::Unknown,
            coverage: None,
            primary_removed: false,
            projection_clean: false,
        };
    }
    assess_legacy_session_removal(&m.primary, &r.primary)
}

#[cfg(test)]
mod desktop_contract_tests {
    use super::*;

    fn identity(path: &str, inode: u64) -> CleanupFileIdentity {
        CleanupFileIdentity {
            path: path.into(),
            device: 7,
            inode,
            links: 1,
            size_bytes: 123,
            mtime_seconds: 0,
            mtime_nanoseconds: 0,
        }
    }

    #[test]
    fn legacy_inventory_rejects_unfrozen_or_ambiguous_sources() {
        let user_data = identity("/owned/user-data", 1);
        let mut seed = 0x4d3832315f763431_u64;
        for _ in 0..256 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let path = format!("/owned/user-data/Local Storage/leveldb/{seed:016x}.ldb");
            let valid = DesktopLegacyCommitment {
                files: vec![DesktopLegacyFile {
                    identity: identity(&path, seed | 1),
                    sha256: "a".repeat(64),
                }],
                layout_id: "chromium-local-storage-leveldb-v1".into(),
                origin: "app://-".into(),
                logical_legacy_entries: 0,
            };
            assert!(desktop_legacy(&user_data, &valid));
            let mut bad = valid.clone();
            bad.logical_legacy_entries = 1;
            assert!(!desktop_legacy(&user_data, &bad));
            let mut bad = valid.clone();
            bad.origin = "https://example.invalid".into();
            assert!(!desktop_legacy(&user_data, &bad));
            let mut bad = valid.clone();
            bad.files[0].identity.path = "/other/000001.ldb".into();
            assert!(!desktop_legacy(&user_data, &bad));
            let mut bad = valid.clone();
            bad.files[0].identity.links = 2;
            assert!(!desktop_legacy(&user_data, &bad));
            let mut bad = valid.clone();
            bad.files.push(bad.files[0].clone());
            assert!(!desktop_legacy(&user_data, &bad));
            let mut bad = valid;
            bad.files[0].sha256 = "0".repeat(63);
            assert!(!desktop_legacy(&user_data, &bad));
        }
        let absent = DesktopLegacyCommitment {
            files: vec![],
            layout_id: "absent".into(),
            origin: "app://-".into(),
            logical_legacy_entries: 0,
        };
        assert!(desktop_legacy(&user_data, &absent));
    }

    #[test]
    fn frozen_file_presence_and_absence_are_not_interchangeable() {
        let path = std::path::Path::new("/owned/index");
        let mut f = DesktopFileCommitment {
            identity: None,
            preimage_sha256: None,
            retained_sha256: None,
            metadata_sha256: None,
            target_units: 0,
        };
        assert!(desktop_file(&f, path, 7));
        f.target_units = 1;
        assert!(!desktop_file(&f, path, 7));
        f.identity = Some(identity("/owned/index", 2));
        f.preimage_sha256 = Some("b".repeat(64));
        f.retained_sha256 = Some("c".repeat(64));
        f.metadata_sha256 = Some("d".repeat(64));
        assert!(desktop_file(&f, path, 7));
        assert!(!desktop_file(&f, std::path::Path::new("/owned/other"), 7));
        f.identity.as_mut().unwrap().links = 2;
        assert!(!desktop_file(&f, path, 7));
    }
}
