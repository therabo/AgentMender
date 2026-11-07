//! Fixed Desktop sequencing, entered only with a consumed durable intent.
use crate::{
    desktop_catalog::CatalogTarget,
    desktop_catalog_capture::{acquire_catalog, observe_catalog},
    desktop_legacy,
    desktop_shared::{self as shared, GlobalKind, SharedFence},
    scoped_cleanup::{self, CodexScopedCleanupAdapter, PreparedPrimaryFence},
    scoped_files,
};
use agentmender_domain::{
    CleanupActionOutcome, CleanupBoundaryObservation as Boundary, CleanupFileIdentity,
    CleanupOutcome, CompleteRemovalAction as Action, CompleteRemovalActionProgress,
    CompleteRemovalManifest, CompleteRemovalProfile, CompleteRemovalReport,
    DesktopBoundaryEvidence, DesktopBundlePin, DesktopFileCommitment, DesktopRemovalManifest,
    EntityId, LifecycleIntent, LifecyclePlan, NativeLifecycleOperation, PersistentObservation,
    PersistentSurface, PersistentSurfaceObservation, PlanStatus, ProfileAdmission,
    WriterRequirement, validate_complete_removal_manifest, validate_complete_removal_report,
};
use agentmender_persistence::{
    CompleteRemovalAttempt, CompleteRemovalProgress, DurableIntent, DurableObservation,
    EffectError, NativeOutcome,
};
use agentmender_provider_api::{
    ProviderError, ScopedCleanupObservation,
    desktop_process::{
        DesktopWriterInventory, desktop_writer_inventory, desktop_writer_scope_matches_owner,
    },
};
use rustix::fs::{self, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, Metadata},
    io::{BufReader, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const BUNDLE: &str = DesktopBundlePin::PATH;
const PLIST_23880: &str = "7a3920d56d052cdf4bf803a92e2098be015eaf9263589d51e04ac4121f16eed7";
const PLIST_24141: &str = "fb72c75a6cabb64f05420aef782b2d6ac86269a6c5557a75dabfa796905ee6ef";
const LOCAL: &str = "local";
const SOURCE: &str = "cli";
const SURFACES: [&str; 10] = [
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
];

/// Only this module constructs the sequencing context after validating the
/// one-shot handoff. Sibling adapters cannot mint a second execution grant.
pub(super) struct DesktopExecution<'a> {
    manifest: &'a CompleteRemovalManifest,
    adapter: &'a CodexScopedCleanupAdapter,
}
impl DesktopExecution<'_> {
    pub(super) fn manifest(&self) -> &CompleteRemovalManifest {
        self.manifest
    }

    pub(super) fn require_closed(&self) -> Result<(), EffectError> {
        require_closed(self.adapter)
    }
}

fn invalid() -> EffectError {
    EffectError::InvalidEvidence
}

fn directory(path: &Path) -> Result<Metadata, EffectError> {
    let mut fd = fs::open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| invalid())?;
    if !path.is_absolute() {
        return Err(invalid());
    }
    for part in path.components() {
        match part {
            std::path::Component::RootDir => {}
            std::path::Component::Normal(name) => {
                fd = fs::openat(
                    &fd,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|_| invalid())?;
            }
            _ => return Err(invalid()),
        }
    }
    File::from(fd).metadata().map_err(|_| invalid())
}

fn identity(path: &Path, meta: &Metadata) -> CleanupFileIdentity {
    CleanupFileIdentity {
        path: path.to_path_buf(),
        device: meta.dev(),
        inode: meta.ino(),
        links: meta.nlink(),
        size_bytes: meta.len(),
        mtime_seconds: meta.mtime(),
        mtime_nanoseconds: meta.mtime_nsec() as u32,
    }
}

fn same_inode(a: &CleanupFileIdentity, b: &CleanupFileIdentity) -> bool {
    a.path == b.path && a.device == b.device && a.inode == b.inode
}

fn roots(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
) -> Result<(CleanupFileIdentity, CleanupFileIdentity), EffectError> {
    let reader = adapter.reader();
    let (_, home) = reader
        .evidence_helper
        .as_ref()
        .ok_or(EffectError::Unavailable)?;
    let root = &reader.location.state_root;
    // The first profile fixes the default pair under a single explicitly bound
    // home. Owned acceptance can use the same layout under its isolated HOME.
    if root != &home.join(".codex") || user_data != home.join("Library/Application Support/Codex") {
        return Err(EffectError::ProfileMismatch);
    }
    let home_meta = directory(home)?;
    let root_meta = directory(root)?;
    let user_meta = directory(user_data)?;
    if !desktop_writer_scope_matches_owner(home_meta.uid())
        || [&home_meta, &root_meta, &user_meta]
            .iter()
            .any(|m| m.uid() != home_meta.uid() || m.mode() & 0o022 != 0)
        || (root_meta.dev(), root_meta.ino()) == (user_meta.dev(), user_meta.ino())
    {
        return Err(invalid());
    }
    Ok((identity(root, &root_meta), identity(user_data, &user_meta)))
}

fn hash_bundle_member(path: &Path, size: u64) -> Result<String, EffectError> {
    let mismatch = EffectError::ProfileMismatch;
    if path.canonicalize().ok().as_deref() != Some(path) {
        return Err(mismatch);
    }
    let file = File::from(
        fs::open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| mismatch)?,
    );
    let before = file.metadata().map_err(|_| mismatch)?;
    if !before.is_file() || before.nlink() != 1 || before.len() != size {
        return Err(mismatch);
    }
    let mut input = BufReader::new(&file);
    let mut hasher = Sha256::new();
    let mut buf = [0; 64 * 1024];
    let mut len = 0;
    loop {
        let n = input.read(&mut buf).map_err(|_| mismatch)?;
        len += n as u64;
        if len > size {
            return Err(mismatch);
        }
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let after = std::fs::symlink_metadata(path).map_err(|_| mismatch)?;
    if len != size
        || !after.is_file()
        || path.canonicalize().ok().as_deref() != Some(path)
        || identity(path, &before) != identity(path, &after)
        || (before.ctime(), before.ctime_nsec()) != (after.ctime(), after.ctime_nsec())
    {
        return Err(mismatch);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Verify the exact plist, executable, archive and CLI bytes without launching
/// the provider. The plist selects one build, so mixed bundles cannot match.
fn bundle() -> Result<DesktopBundlePin, EffectError> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err(EffectError::UnsupportedVersion);
    }
    bundle_at(Path::new(BUNDLE))
}

fn bundle_at(root: &Path) -> Result<DesktopBundlePin, EffectError> {
    let mismatch = EffectError::ProfileMismatch;
    let plist = root.join("Contents/Info.plist");
    let digest = hash_bundle_member(&plist, 20_787)?;
    let (pin, asar_size) = match digest.as_str() {
        PLIST_23880 => (DesktopBundlePin::BUILD_23880, 674_614_757),
        PLIST_24141 => (DesktopBundlePin::BUILD_24141, 674_615_301),
        _ => return Err(mismatch),
    };
    for (relative, size, expected) in [
        ("Contents/MacOS/ChatGPT", 71_008, pin.main_sha256),
        ("Contents/Resources/app.asar", asar_size, pin.asar_sha256),
        (
            "Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
            267_754_128,
            DesktopBundlePin::CLI_SHA256,
        ),
        (
            "Contents/Resources/codex-cli/bin/codex",
            328,
            "50ab38ba21d0d9f8346f32f41848382f15b556190f3c7a07e885a4fb73e379c8",
        ),
    ] {
        if hash_bundle_member(&root.join(relative), size)? != expected {
            return Err(mismatch);
        }
    }
    if hash_bundle_member(&plist, 20_787)? != digest {
        return Err(mismatch);
    }
    Ok(pin)
}

fn writers(adapter: &CodexScopedCleanupAdapter) -> DesktopWriterInventory {
    let Some(cli) = adapter.reader().location.executable.as_deref() else {
        return DesktopWriterInventory::Unknown;
    };
    desktop_writer_inventory(Path::new(BUNDLE), cli)
}

fn require_closed(adapter: &CodexScopedCleanupAdapter) -> Result<(), EffectError> {
    match writers(adapter) {
        DesktopWriterInventory::Closed => Ok(()),
        DesktopWriterInventory::Active => Err(EffectError::RequiresProviderClosed),
        DesktopWriterInventory::Unknown => Err(EffectError::WriterInventoryUnknown),
    }
}

pub(super) fn validate_read_profile(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
) -> Result<DesktopBundlePin, ProviderError> {
    let pin = bundle().map_err(|_| {
        ProviderError::CapabilityUnavailable(
            "installed Codex Desktop bundle does not match a reviewed candidate".into(),
        )
    })?;
    roots(adapter, user_data).map_err(|_| {
        ProviderError::CapabilityUnavailable(
            "exact Codex Desktop storage roots are unavailable".into(),
        )
    })?;
    Ok(pin)
}

fn shared_images(
    desktop_root: &CleanupFileIdentity,
    target: &str,
    ownership: bool,
) -> Result<[DesktopFileCommitment; 3], EffectError> {
    let host = shared::local_host_scope(desktop_root, LOCAL)?;
    let [backup, main] = shared::observe_globals(desktop_root, target, &host, ownership)?;
    Ok([backup, main, shared::observe_index(desktop_root, target)?])
}

pub(super) fn observe(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
    target: &str,
    pin: DesktopBundlePin,
    observation: &mut ScopedCleanupObservation,
) -> Result<(), ProviderError> {
    fn unavailable(reason: EffectError) -> ProviderError {
        ProviderError::CapabilityUnavailable(match reason {
            EffectError::ProfileMismatch | EffectError::UnsupportedVersion => {
                "exact Codex Desktop build or storage roots are unsupported".into()
            }
            EffectError::WriterInventoryUnknown => "Codex writer inventory is incomplete".into(),
            _ => "required Desktop projection evidence is incomplete".into(),
        })
    }
    let before = writers(adapter);
    let (desktop_root, user_data) = roots(adapter, user_data).map_err(unavailable)?;
    let evidence = observation
        .context
        .complete_removal_evidence
        .as_mut()
        .ok_or_else(|| unavailable(invalid()))?;
    let manifest = &mut evidence.manifest;
    if manifest.primary.root != desktop_root
        || manifest.primary.native_session_id.as_str() != target
    {
        return Err(unavailable(invalid()));
    }
    let catalog = observe_catalog(adapter.reader(), &desktop_root, target, SOURCE)?;
    if !catalog.bare_uuid_qualified
        || catalog.title.is_empty()
        || catalog.title.len() > 4096
        || catalog.title.chars().any(char::is_control)
    {
        return Err(unavailable(invalid()));
    }
    let [global_backup, global_main, session_index] =
        shared_images(&desktop_root, target, catalog.bare_uuid_qualified).map_err(unavailable)?;
    let legacy = desktop_legacy::observe(&user_data).map_err(unavailable)?;
    if bundle().map_err(unavailable)? != pin {
        return Err(unavailable(EffectError::ProfileMismatch));
    }
    let after = writers(adapter);
    let inventory = if before == after {
        after
    } else {
        DesktopWriterInventory::Unknown
    };
    evidence.desktop_boundary = Some(DesktopBoundaryEvidence {
        legacy: if legacy.logical_legacy_entries == 0 {
            Boundary::CompleteAndUnshared
        } else {
            Boundary::Incomplete
        },
        roots: Boundary::CompleteAndUnshared,
        profile: Boundary::CompleteAndUnshared,
        writer_inventory: if inventory == DesktopWriterInventory::Unknown {
            Boundary::Incomplete
        } else {
            Boundary::CompleteAndUnshared
        },
        writers_closed: inventory == DesktopWriterInventory::Closed,
    });
    manifest.profile = CompleteRemovalProfile {
        identity: EntityId::new(pin.profile_id).expect("fixed profile id"),
        admission: ProfileAdmission::DesktopUnaccepted,
        writer_requirement: WriterRequirement::RequiresProviderClosed,
    };
    manifest.actions = vec![
        Action::DesktopCatalog,
        Action::GlobalBackup,
        Action::GlobalMain,
        Action::SessionIndex,
        Action::PrimaryPayload,
        Action::StateThreadRecords,
    ];
    manifest.required_surfaces = SURFACES
        .iter()
        .map(|id| PersistentSurface {
            id: EntityId::new(*id).expect("fixed persistent surface id"),
        })
        .collect();
    manifest.desktop = Some(DesktopRemovalManifest {
        desktop_root,
        user_data,
        bundle_path: PathBuf::from(BUNDLE),
        bundle_id: DesktopBundlePin::ID.into(),
        bundle_version: pin.version.into(),
        bundle_build: pin.build.into(),
        bundle_sha256: pin.main_sha256.into(),
        asar_sha256: pin.asar_sha256.into(),
        cli_sha256: DesktopBundlePin::CLI_SHA256.into(),
        stable_worker_sha256: manifest.primary.profile.executable_sha256.clone(),
        local_host: LOCAL.into(),
        source_kind: SOURCE.into(),
        target_title: catalog.title,
        legacy,
        catalog: catalog.commitment,
        global_backup,
        global_main,
        session_index,
    });
    Ok(())
}

fn manifest<'a>(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
    plan: &'a LifecyclePlan,
) -> Result<(&'a CompleteRemovalManifest, &'a DesktopRemovalManifest), EffectError> {
    let p = plan.content();
    let m = p.complete_removal_manifest.as_ref().ok_or_else(invalid)?;
    let desktop = m.desktop.as_ref().ok_or_else(invalid)?;
    let primary = &m.primary;
    let reader = adapter.reader();
    if p.schema_version != 4
        || p.status != PlanStatus::Ready
        || p.cleanup_manifest.is_some()
        || p.request.intent != LifecycleIntent::CleanupSession
        || p.request.target != primary.target
        || p.native_target_id.as_ref() != Some(&primary.native_session_id)
        || p.observation_scope.project_path.is_some()
        || !p.observation_scope.include_archived
        || p.operations.len() != 1
        || p.operations[0].operation != NativeLifecycleOperation::ScopedCleanupSession
        || p.operations[0].target != primary.target
        || p.operations[0].native_id != primary.native_session_id
        || p.provider.executable != reader.location.executable
        || reader.fixture.is_some()
        || primary.root.path != reader.location.state_root
        || desktop.user_data.path != user_data
        || !crate::scoped_profile::validate_manifest_profile(primary)
        || !validate_complete_removal_manifest(m)
    {
        return Err(invalid());
    }
    Ok((m, desktop))
}

fn check_roots(
    adapter: &CodexScopedCleanupAdapter,
    desktop: &DesktopRemovalManifest,
) -> Result<(), EffectError> {
    let (root, data) = roots(adapter, &desktop.user_data.path)?;
    if !same_inode(&root, &desktop.desktop_root) || data != desktop.user_data {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn preflight(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
    plan: &LifecyclePlan,
) -> Result<(), EffectError> {
    let (manifest, desktop) = manifest(adapter, user_data, plan)?;
    if !bundle()?.matches_manifest(&manifest.profile, desktop) {
        return Err(EffectError::ProfileMismatch);
    }
    require_closed(adapter)?;
    check_roots(adapter, desktop)
}

fn result_report(
    manifest: &CompleteRemovalManifest,
    actions: Vec<CompleteRemovalActionProgress>,
    outcome: CleanupOutcome,
    canonical_absent: bool,
    verified_cutoff: bool,
) -> CompleteRemovalReport {
    let payload = if actions.iter().any(|a| a.action == Action::PrimaryPayload) {
        CleanupActionOutcome::Removed
    } else if outcome == CleanupOutcome::Blocked {
        CleanupActionOutcome::Retained
    } else {
        CleanupActionOutcome::Unknown
    };
    CompleteRemovalReport {
        desktop_collateral_unchanged: verified_cutoff.then_some(true),
        primary: scoped_cleanup::report(
            &manifest.primary,
            outcome,
            payload,
            canonical_absent,
            false,
        ),
        actions,
        surfaces: manifest
            .required_surfaces
            .iter()
            .map(|surface| PersistentSurfaceObservation {
                id: surface.id.clone(),
                observation: if verified_cutoff
                    && matches!(
                        surface.id.as_str(),
                        "canonical-active-archived"
                            | "desktop-local-catalog"
                            | "desktop-scan-reconstruction"
                            | "session-index"
                            | "renderer-legacy-import"
                    ) {
                    PersistentObservation::Absent
                } else {
                    PersistentObservation::Unknown
                },
            })
            .collect(),
    }
}

fn record(
    journal: &mut dyn CompleteRemovalProgress,
    observation: &DurableObservation,
    actions: &mut Vec<CompleteRemovalActionProgress>,
    action: Action,
    outcome: CleanupActionOutcome,
) -> Result<(), NativeOutcome> {
    let step = CompleteRemovalActionProgress { action, outcome };
    journal
        .record(observation, step.clone())
        .map_err(NativeOutcome::Unknown)?;
    actions.push(step);
    Ok(())
}

fn run(
    adapter: &CodexScopedCleanupAdapter,
    desktop: &DesktopRemovalManifest,
    execution: &DesktopExecution<'_>,
    observation: &DurableObservation,
    expires_at: u64,
    journal: &mut dyn CompleteRemovalProgress,
    actions: &mut Vec<CompleteRemovalActionProgress>,
) -> Result<(), NativeOutcome> {
    let never = NativeOutcome::NeverDispatched;
    let unknown = NativeOutcome::Unknown;
    let thread = execution.manifest.primary.native_session_id.as_str();
    adapter.validate_executable().map_err(never)?;
    check_roots(adapter, desktop).map_err(never)?;
    require_closed(adapter).map_err(never)?;
    // Obtain all reviewed locks and compare every complete preimage before the
    // first catalog write. The primary fence keeps native locks and prepared
    // canonical DELETEs uncommitted while the projection actions advance.
    let mut primary = PreparedPrimaryFence::acquire(execution, expires_at)?;
    let mut catalog = acquire_catalog(execution).map_err(unknown)?;
    let shared = SharedFence::acquire(execution).map_err(unknown)?;
    if shared_images(&desktop.desktop_root, thread, true).map_err(unknown)?
        != [
            desktop.global_backup.clone(),
            desktop.global_main.clone(),
            desktop.session_index.clone(),
        ]
        || desktop_legacy::observe(&desktop.user_data).map_err(unknown)? != desktop.legacy
    {
        return Err(unknown(invalid()));
    }
    // Acquiring the fences can leave a permanent lock or SQLite sidecars.
    require_closed(adapter).map_err(unknown)?;
    if !not_expired(expires_at) {
        return Err(unknown(invalid()));
    }
    let target = CatalogTarget {
        thread_id: thread.into(),
        source_kind: desktop.source_kind.clone(),
    };
    let post_catalog = catalog.apply(&target, &desktop.catalog).map_err(unknown)?;
    record(
        journal,
        observation,
        actions,
        Action::DesktopCatalog,
        CleanupActionOutcome::Removed,
    )?;
    let mut post_images = Vec::with_capacity(3);
    for (action, old) in [
        (Action::GlobalBackup, &desktop.global_backup),
        (Action::GlobalMain, &desktop.global_main),
        (Action::SessionIndex, &desktop.session_index),
    ] {
        require_closed(adapter).map_err(unknown)?;
        let post = match action {
            Action::GlobalBackup => {
                shared::publish_global(execution, &shared, GlobalKind::Backup, None)
            }
            Action::GlobalMain => {
                shared::publish_global(execution, &shared, GlobalKind::Main, post_images.first())
            }
            Action::SessionIndex => shared::publish_index(execution, &shared),
            _ => return Err(unknown(invalid())),
        }
        .map_err(unknown)?;
        let outcome = if old.target_units == 0 {
            CleanupActionOutcome::AlreadyAbsent
        } else {
            CleanupActionOutcome::Removed
        };
        record(journal, observation, actions, action, outcome)?;
        post_images.push(post);
    }
    require_closed(adapter).map_err(unknown)?;
    primary.unlink_and_verify().map_err(unknown)?;
    record(
        journal,
        observation,
        actions,
        Action::PrimaryPayload,
        CleanupActionOutcome::Removed,
    )?;
    require_closed(adapter).map_err(unknown)?;
    if !primary.commit_and_verify().map_err(unknown)? {
        return Err(unknown(invalid()));
    }
    record(
        journal,
        observation,
        actions,
        Action::StateThreadRecords,
        CleanupActionOutcome::Removed,
    )?;
    catalog.verify(&post_catalog).map_err(unknown)?;
    shared.validate().map_err(unknown)?;
    if !primary.verify_retained().map_err(unknown)?
        || shared_images(&desktop.desktop_root, thread, true)
            .map_err(unknown)?
            .as_slice()
            != post_images.as_slice()
        || desktop_legacy::observe(&desktop.user_data).map_err(unknown)? != desktop.legacy
    {
        return Err(unknown(invalid()));
    }
    check_roots(adapter, desktop).map_err(unknown)?;
    require_closed(adapter).map_err(unknown)
}

fn not_expired(expires_at: u64) -> bool {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .is_ok_and(|now| now.as_secs() < expires_at)
}

pub(super) fn dispatch(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
    intent: DurableIntent,
    plan: &LifecyclePlan,
    journal: &mut dyn CompleteRemovalProgress,
) -> CompleteRemovalAttempt {
    let bound = intent.plan_digest() == plan.digest()
        && intent.state_fingerprint() == plan.state_fingerprint()
        && intent.operation_index() == 0;
    let expires_at = intent.expires_at();
    let observation = intent.into_observation();
    let Ok((manifest, desktop)) = manifest(adapter, user_data, plan) else {
        return CompleteRemovalAttempt {
            observation,
            outcome: NativeOutcome::NeverDispatched(invalid()),
            report: None,
        };
    };
    let mut actions = Vec::with_capacity(manifest.actions.len());
    let result = if !bound || !not_expired(expires_at) {
        Err(NativeOutcome::NeverDispatched(invalid()))
    } else {
        preflight(adapter, user_data, plan)
            .map_err(NativeOutcome::NeverDispatched)
            .and_then(|()| {
                run(
                    adapter,
                    desktop,
                    &DesktopExecution { manifest, adapter },
                    &observation,
                    expires_at,
                    journal,
                    &mut actions,
                )
            })
    };
    let verified_cutoff = result.is_ok();
    let (outcome, cleanup) = match result {
        // Reviewed target actions reached a common cutoff. Remaining roles
        // require independent exact-build evidence before structural success.
        Ok(()) => (
            NativeOutcome::Unknown(invalid()),
            CleanupOutcome::OutcomeUnknown,
        ),
        Err(e @ NativeOutcome::NeverDispatched(_)) => (e, CleanupOutcome::Blocked),
        Err(e) => (e, CleanupOutcome::OutcomeUnknown),
    };
    let canonical_absent = actions
        .last()
        .is_some_and(|a| a.action == Action::StateThreadRecords);
    let report = result_report(
        manifest,
        actions,
        cleanup,
        canonical_absent,
        verified_cutoff,
    );
    if !validate_complete_removal_report(manifest, &report) {
        return CompleteRemovalAttempt {
            observation,
            outcome: NativeOutcome::Unknown(invalid()),
            report: None,
        };
    }
    CompleteRemovalAttempt {
        observation,
        outcome,
        report: Some(report),
    }
}

/// Reobserve only approved primary evidence. A missing receipt or a separate
/// read-only observation never certifies one common persistent cutoff.
pub(super) fn reconcile(
    adapter: &CodexScopedCleanupAdapter,
    user_data: &Path,
    plan: &LifecyclePlan,
) -> Result<CompleteRemovalReport, EffectError> {
    let (manifest, _) = manifest(adapter, user_data, plan)?;
    let (payload, _) = scoped_files::reconcile_files(&manifest.primary)?;
    let db = adapter
        .capture_target(manifest.primary.native_session_id.as_str())
        .map_err(|_| EffectError::Unavailable)?;
    let absent = db.rows.iter().any(|r| r.table == "threads" && r.count == 0);
    let mut report = result_report(
        manifest,
        Vec::new(),
        CleanupOutcome::OutcomeUnknown,
        absent,
        false,
    );
    report.primary = scoped_cleanup::report(
        &manifest.primary,
        CleanupOutcome::OutcomeUnknown,
        payload,
        absent,
        false,
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_member_requires_exact_length_and_unaliased_file() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let member = root.join("member");
        std::fs::write(&member, b"abc").unwrap();
        assert_eq!(
            hash_bundle_member(&member, 3).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        for length in [2, 4] {
            assert!(matches!(
                hash_bundle_member(&member, length),
                Err(EffectError::ProfileMismatch)
            ));
        }
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&member, &alias).unwrap();
        assert!(matches!(
            hash_bundle_member(&alias, 3),
            Err(EffectError::ProfileMismatch)
        ));
        std::fs::remove_file(&alias).unwrap();
        std::fs::hard_link(&member, &alias).unwrap();
        assert!(matches!(
            hash_bundle_member(&member, 3),
            Err(EffectError::ProfileMismatch)
        ));
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    #[ignore = "requires both exact signed bundles; reads bundle files only"]
    fn exact_installed_and_recovered_bundle_pins() {
        let recovered =
            PathBuf::from(std::env::var_os("AGENTMENDER_RECOVERED_DESKTOP_BUNDLE").unwrap());
        assert_eq!(
            bundle_at(&recovered).unwrap(),
            DesktopBundlePin::BUILD_23880
        );
        assert_eq!(bundle().unwrap(), DesktopBundlePin::BUILD_24141);
    }
}
