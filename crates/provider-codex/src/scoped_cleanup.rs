//! Receipt-gated orchestration for the exact local scoped-cleanup profile.
//! The filesystem and database modules contain the fixed provider operations;
//! this module never accepts arbitrary paths or SQL from a caller.
use crate::{
    CodexAdapter, scoped_files,
    scoped_profile::{
        self, SCOPED_ADAPTER, SCOPED_CODEX_EXECUTABLE_SHA256, SCOPED_CODEX_SCHEMA_SHA256,
        SCOPED_CODEX_VERSION, SCOPED_EXPANDED_ADAPTER,
    },
    scoped_storage,
};
use agentmender_domain::{
    CleanupActionOutcome, CleanupActionReport, CleanupManifest, CleanupOutcome, CleanupReport,
    GraphNodeKind, LifecycleIntent, LifecyclePlan, NativeLifecycleOperation, PlanStatus,
    ProviderKind, validate_cleanup_report,
};
use agentmender_persistence::{
    CleanupAttempt, DurableCleanupPort, DurableIntent, EffectError, NativeOutcome,
};
use sha2::{Digest, Sha256};
use std::io::Read;

pub struct CodexScopedCleanupAdapter {
    reader: CodexAdapter,
    expanded: bool,
    protected_sessions: std::collections::BTreeSet<String>,
    #[cfg(unix)]
    desktop_user_data: Option<std::path::PathBuf>,
    #[cfg(test)]
    synthetic_pin: bool,
    #[cfg(test)]
    fault: Option<TestFault>,
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TestFault {
    AfterUnlink,
    AfterCommit,
    CrashAfterUnlink,
    CrashAfterCommit,
    RejectProgress,
}

impl CodexScopedCleanupAdapter {
    pub fn new(reader: CodexAdapter) -> Self {
        Self {
            reader,
            expanded: false,
            protected_sessions: Default::default(),
            #[cfg(unix)]
            desktop_user_data: None,
            #[cfg(test)]
            synthetic_pin: false,
            #[cfg(test)]
            fault: None,
        }
    }

    /// Explicitly select the separately versioned expanded primary profile.
    pub fn new_expanded(reader: CodexAdapter) -> Self {
        let mut adapter = Self::new(reader);
        adapter.expanded = true;
        adapter
    }

    /// Trusted host registration, fixed before MCP requests; not a caller assertion.
    pub fn with_protected_sessions(mut self, sessions: Vec<String>) -> Self {
        self.protected_sessions = sessions.into_iter().collect();
        self
    }

    #[cfg(test)]
    pub(super) fn for_owned_test(reader: CodexAdapter) -> Self {
        Self {
            reader,
            expanded: false,
            protected_sessions: Default::default(),
            #[cfg(unix)]
            desktop_user_data: None,
            synthetic_pin: true,
            fault: None,
        }
    }

    #[cfg(unix)]
    pub fn new_desktop(reader: CodexAdapter, user_data: std::path::PathBuf) -> Self {
        let mut adapter = Self::new_expanded(reader);
        adapter.desktop_user_data = Some(user_data);
        adapter
    }

    pub(super) fn reader(&self) -> &CodexAdapter {
        &self.reader
    }

    pub(super) fn capture_target(
        &self,
        target: &str,
    ) -> Result<crate::ScopedCaptureResult, agentmender_provider_api::ProviderError> {
        self.capture(target, false)
    }

    fn capture(
        &self,
        target: &str,
        metadata: bool,
    ) -> Result<crate::ScopedCaptureResult, agentmender_provider_api::ProviderError> {
        #[cfg(test)]
        if self.synthetic_pin {
            return crate::scoped_capture::snapshot_inner(
                &self.reader.location.state_root,
                target,
                self.desktop_user_data.is_some(),
                metadata,
            );
        }
        use agentmender_provider_api::{ProviderError, evidence::EvidenceSandbox, process};
        let (helper, real_home) = self.reader.evidence_helper.as_ref().ok_or_else(|| {
            ProviderError::CapabilityUnavailable(
                "protected scoped evidence helper unavailable".into(),
            )
        })?;
        let sandbox = EvidenceSandbox::new(&self.reader.location.state_root, real_home)?;
        if sandbox.source_root() != self.reader.location.state_root {
            return Err(ProviderError::CapabilityUnavailable(
                "source root identity rejected".into(),
            ));
        }
        let request = crate::ScopedCaptureRequest {
            source_root: sandbox.source_root().into(),
            mirror_root: sandbox.path().into(),
            probe_path: sandbox.probe_path().into(),
            target_id: target.into(),
            desktop_closed_snapshot: self.desktop_user_data.is_some(),
            include_session_metadata: metadata,
        };
        crate::scoped_capture::read_worker_result(
            process::spawn_codex_scoped_capture(helper, &sandbox)?,
            &request,
        )
    }

    fn manifest<'a>(&self, plan: &'a LifecyclePlan) -> Result<&'a CleanupManifest, EffectError> {
        self.manifest_for(plan, false)
    }

    fn manifest_for<'a>(
        &self,
        plan: &'a LifecyclePlan,
        expanded: bool,
    ) -> Result<&'a CleanupManifest, EffectError> {
        let p = plan.content();
        let frozen: std::collections::BTreeSet<_> = p
            .artifacts
            .iter()
            .filter(|a| a.explicitly_protected && a.id.class == GraphNodeKind::Session)
            .filter_map(|a| {
                a.id.local_id
                    .as_str()
                    .strip_prefix("codex:session:")
                    .map(str::to_owned)
            })
            .collect();
        if frozen != self.protected_sessions
            || self
                .protected_sessions
                .contains(p.native_target_id.as_ref().map_or("", |id| id.as_str()))
        {
            return Err(EffectError::InvalidEvidence);
        }
        let m = if expanded {
            let outer = p
                .complete_removal_manifest
                .as_ref()
                .ok_or(EffectError::InvalidEvidence)?;
            if p.schema_version != 3
                || p.cleanup_manifest.is_some()
                || !agentmender_domain::validate_complete_removal_manifest(outer)
            {
                return Err(EffectError::InvalidEvidence);
            }
            &outer.primary
        } else {
            if p.schema_version != 2 || p.complete_removal_manifest.is_some() {
                return Err(EffectError::InvalidEvidence);
            }
            p.cleanup_manifest
                .as_ref()
                .ok_or(EffectError::InvalidEvidence)?
        };
        if scoped_profile::is_expanded(m) != expanded
            || self.expanded != expanded
            || p.status != PlanStatus::Ready
            || p.request.intent != LifecycleIntent::CleanupSession
            || p.request.target != m.target
            || p.native_target_id.as_ref() != Some(&m.native_session_id)
            || m.target.provider != ProviderKind::Codex
            || m.target.class != GraphNodeKind::Session
            || m.root.path != self.reader.location.state_root
            || self.reader.fixture.is_some()
            || p.operations.len() != 1
            || p.operations[0].operation != NativeLifecycleOperation::ScopedCleanupSession
            || p.operations[0].target != m.target
            || p.operations[0].native_id != m.native_session_id
            || p.provider.executable.as_ref() != self.reader.location.executable.as_ref()
            || p.observation_scope.project_path.is_some()
            || !p.observation_scope.include_archived
            || m.payloads.len() != 1
            || !scoped_profile::validate_manifest_profile(m)
            || !scoped_files::valid_rollout_name(
                &m.payloads[0].identity.path,
                m.native_session_id.as_str(),
            )
        {
            return Err(EffectError::InvalidEvidence);
        }
        Ok(m)
    }

    pub(super) fn validate_executable(&self) -> Result<(), EffectError> {
        #[cfg(test)]
        if self.synthetic_pin {
            return Ok(());
        }
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            return Err(EffectError::UnsupportedVersion);
        }
        let path = self
            .reader
            .location
            .executable
            .as_deref()
            .ok_or(EffectError::Unavailable)?;
        if !path.is_absolute() || path.canonicalize().map_err(|_| EffectError::Unavailable)? != path
        {
            return Err(EffectError::ProfileMismatch);
        }
        let metadata = std::fs::symlink_metadata(path).map_err(|_| EffectError::Unavailable)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != 268_972_064
        {
            return Err(EffectError::ProfileMismatch);
        }
        let file = std::fs::File::open(path).map_err(|_| EffectError::Unavailable)?;
        let before = file.metadata().map_err(|_| EffectError::Unavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.dev() != before.dev()
                || metadata.ino() != before.ino()
                || before.nlink() != 1
            {
                return Err(EffectError::ProfileMismatch);
            }
        }
        let mut input = std::io::BufReader::new(&file);
        let mut hash = Sha256::new();
        let mut bytes = [0u8; 65536];
        loop {
            let n = input
                .read(&mut bytes)
                .map_err(|_| EffectError::Unavailable)?;
            if n == 0 {
                break;
            }
            hash.update(&bytes[..n]);
        }
        let after = file.metadata().map_err(|_| EffectError::Unavailable)?;
        if before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
            || hash
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                != SCOPED_CODEX_EXECUTABLE_SHA256
        {
            return Err(EffectError::ProfileMismatch);
        }
        Ok(())
    }
}

fn not_expired(expires_at: u64) -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .is_ok_and(|now| now.as_secs() < expires_at)
}

pub(super) fn report(
    manifest: &CleanupManifest,
    outcome: CleanupOutcome,
    action: CleanupActionOutcome,
    row_absent: bool,
    retained: bool,
) -> CleanupReport {
    let bytes = manifest.payloads[0].identity.size_bytes;
    CleanupReport {
        outcome,
        actions: vec![CleanupActionReport {
            artifact: manifest.payloads[0].artifact.clone(),
            outcome: action,
            logical_unlinked_bytes: if action == CleanupActionOutcome::Removed {
                bytes
            } else {
                0
            },
        }],
        canonical_row_absent: row_absent,
        retained_primary_unchanged: retained,
        optional_index: manifest
            .optional_index
            .as_ref()
            .map(|_| CleanupActionOutcome::Retained),
        post_cutoff_residue: false,
        logical_planned_bytes: bytes,
        logical_unlinked_bytes: if action == CleanupActionOutcome::Removed {
            bytes
        } else {
            0
        },
        physical_reclaimed_bytes: None,
    }
}

pub(super) fn report_blocked(manifest: &CleanupManifest) -> CleanupReport {
    report(
        manifest,
        CleanupOutcome::Blocked,
        CleanupActionOutcome::Retained,
        false,
        false,
    )
}

impl DurableCleanupPort for CodexScopedCleanupAdapter {
    fn preflight(&self, plan: &LifecyclePlan) -> Result<(), EffectError> {
        self.manifest(plan)?;
        self.validate_executable()?;
        let manifest = self.manifest(plan)?;
        let snapshot = self
            .capture_target(manifest.native_session_id.as_str())
            .map_err(|_| EffectError::Unavailable)?;
        let cwd = snapshot
            .cwd
            .as_deref()
            .ok_or(EffectError::InvalidEvidence)?;
        crate::scoped_config::validate(&manifest.root.path, cwd)
    }

    fn dispatch(&self, intent: DurableIntent, plan: &LifecyclePlan) -> CleanupAttempt {
        let matches = intent.plan_digest() == plan.digest()
            && intent.state_fingerprint() == plan.state_fingerprint()
            && intent.operation_index() == 0;
        let expires_at = intent.expires_at();
        let observation = intent.into_observation();
        let (outcome, mut report) = if !matches || !not_expired(expires_at) {
            (
                NativeOutcome::NeverDispatched(EffectError::InvalidEvidence),
                None,
            )
        } else if let Err(error) = DurableCleanupPort::preflight(self, plan) {
            (NativeOutcome::NeverDispatched(error), None)
        } else {
            self.execute(plan, expires_at)
        };
        if matches!(outcome, NativeOutcome::NeverDispatched(_))
            && report.is_none()
            && let Ok(manifest) = self.manifest(plan)
        {
            report = Some(report_blocked(manifest));
        }
        CleanupAttempt {
            observation,
            outcome,
            report,
        }
    }

    fn observe(&self, plan: &LifecyclePlan) -> Result<CleanupReport, EffectError> {
        let manifest = self.manifest(plan)?;
        let (action, retained_files) = scoped_files::reconcile_files(manifest)?;
        let db = self
            .capture_target(manifest.native_session_id.as_str())
            .map_err(|_| EffectError::Unavailable)?;
        let row_absent = db
            .rows
            .iter()
            .find(|r| r.table == "threads")
            .is_some_and(|r| r.count == 0);
        let observed_retained = if scoped_profile::is_expanded(manifest) {
            &db.expanded_retained_database_sha256
        } else {
            &db.retained_database_sha256
        };
        let retained_database = observed_retained == &manifest.retained_database_sha256;
        // Independent read-only observations cannot establish one simultaneous
        // fenced cutoff, so preserve the overall unknown state even if individual
        // frozen actions are observed absent. No destructive authority is recreated.
        let retained_at_cutoff = false;
        let _ = (retained_files, retained_database);
        Ok(report(
            manifest,
            CleanupOutcome::OutcomeUnknown,
            action,
            row_absent,
            retained_at_cutoff,
        ))
    }
}

impl agentmender_persistence::DurableCompleteRemovalPort for CodexScopedCleanupAdapter {
    fn preflight(&self, plan: &LifecyclePlan) -> Result<(), EffectError> {
        #[cfg(unix)]
        if let Some(path) = &self.desktop_user_data {
            return crate::desktop_cleanup::preflight(self, path, plan);
        }
        let manifest = self.manifest_for(plan, true)?;
        self.validate_executable()?;
        let snapshot = self
            .capture_target(manifest.native_session_id.as_str())
            .map_err(|_| EffectError::Unavailable)?;
        let cwd = snapshot
            .cwd
            .as_deref()
            .ok_or(EffectError::InvalidEvidence)?;
        crate::scoped_config::validate(&manifest.root.path, cwd)
    }

    fn dispatch(
        &self,
        intent: DurableIntent,
        plan: &LifecyclePlan,
        progress: &mut dyn agentmender_persistence::CompleteRemovalProgress,
    ) -> agentmender_persistence::CompleteRemovalAttempt {
        #[cfg(unix)]
        if let Some(path) = &self.desktop_user_data {
            return crate::desktop_cleanup::dispatch(self, path, intent, plan, progress);
        }
        use agentmender_domain::{CompleteRemovalAction, CompleteRemovalActionProgress};
        let matches = intent.plan_digest() == plan.digest()
            && intent.state_fingerprint() == plan.state_fingerprint()
            && intent.operation_index() == 0;
        let expires_at = intent.expires_at();
        let observation = intent.into_observation();
        let mut completed = [false; 2];
        let (outcome, mut primary) = if !matches || !not_expired(expires_at) {
            (
                NativeOutcome::NeverDispatched(EffectError::InvalidEvidence),
                None,
            )
        } else if let Err(e) =
            <Self as agentmender_persistence::DurableCompleteRemovalPort>::preflight(self, plan)
        {
            (NativeOutcome::NeverDispatched(e), None)
        } else if let Ok(m) = self.manifest_for(plan, true) {
            self.execute_manifest(m, expires_at, &mut |action| {
                #[cfg(test)]
                if self.fault == Some(TestFault::RejectProgress) {
                    return Err(EffectError::Transport);
                }
                progress.record(
                    &observation,
                    CompleteRemovalActionProgress {
                        action,
                        outcome: CleanupActionOutcome::Removed,
                    },
                )?;
                completed[match action {
                    CompleteRemovalAction::PrimaryPayload => 0,
                    CompleteRemovalAction::StateThreadRecords => 1,
                    _ => return Err(EffectError::InvalidEvidence),
                }] = true;
                Ok(())
            })
        } else {
            (
                NativeOutcome::NeverDispatched(EffectError::InvalidEvidence),
                None,
            )
        };
        if matches!(outcome, NativeOutcome::NeverDispatched(_))
            && primary.is_none()
            && let Ok(manifest) = self.manifest_for(plan, true)
        {
            primary = Some(report_blocked(manifest));
        }
        let report = primary.and_then(|primary| {
            plan.content()
                .complete_removal_manifest
                .as_ref()
                .map(|m| complete_report(m, primary, completed))
        });
        agentmender_persistence::CompleteRemovalAttempt {
            observation,
            outcome,
            report,
        }
    }

    fn observe(
        &self,
        plan: &LifecyclePlan,
    ) -> Result<agentmender_domain::CompleteRemovalReport, EffectError> {
        #[cfg(unix)]
        if let Some(path) = &self.desktop_user_data {
            return crate::desktop_cleanup::reconcile(self, path, plan);
        }
        let manifest = self.manifest_for(plan, true)?;
        let (action, _) = scoped_files::reconcile_files(manifest)?;
        let db = self
            .capture_target(manifest.native_session_id.as_str())
            .map_err(|_| EffectError::Unavailable)?;
        let row_absent = db
            .rows
            .iter()
            .find(|r| r.table == "threads")
            .is_some_and(|r| r.count == 0);
        let primary = report(
            manifest,
            CleanupOutcome::OutcomeUnknown,
            action,
            row_absent,
            false,
        );
        Ok(complete_report(
            plan.content()
                .complete_removal_manifest
                .as_ref()
                .ok_or(EffectError::InvalidEvidence)?,
            primary,
            [false; 2],
        ))
    }
}

fn complete_report(
    m: &agentmender_domain::CompleteRemovalManifest,
    mut primary: CleanupReport,
    completed: [bool; 2],
) -> agentmender_domain::CompleteRemovalReport {
    if !completed[0]
        && primary
            .actions
            .iter()
            .any(|action| action.outcome == CleanupActionOutcome::Removed)
    {
        for action in &mut primary.actions {
            action.outcome = CleanupActionOutcome::Unknown;
            action.logical_unlinked_bytes = 0;
        }
        primary.logical_unlinked_bytes = 0;
    }
    if !completed[1] {
        primary.canonical_row_absent = false;
    }
    use agentmender_domain::{
        CompleteRemovalActionProgress, CompleteRemovalReport, PersistentObservation,
        PersistentSurfaceObservation,
    };
    CompleteRemovalReport {
        desktop_collateral_unchanged: None,
        primary,
        actions: m
            .actions
            .iter()
            .enumerate()
            .take_while(|(i, _)| completed[*i])
            .map(|(_, action)| CompleteRemovalActionProgress {
                action: *action,
                outcome: CleanupActionOutcome::Removed,
            })
            .collect(),
        surfaces: m
            .required_surfaces
            .iter()
            .map(|s| PersistentSurfaceObservation {
                id: s.id.clone(),
                observation: PersistentObservation::Unknown,
            })
            .collect(),
    }
}

/// Holds all primary writer and database boundaries across the complete transaction.
/// The Desktop entry is sealed by the consumed, validated execution context.
pub(super) struct PreparedPrimaryFence<'a> {
    manifest: &'a CleanupManifest,
    cwd: std::path::PathBuf,
    // Fields drop in declaration order: SQLite must close before sidecars.
    database: scoped_storage::DatabaseFence,
    postimage_held: bool,
    sidecars: scoped_files::DatabaseSidecars,
    writers: scoped_files::WriterFence,
}

impl<'a> PreparedPrimaryFence<'a> {
    pub(super) fn acquire(
        execution: &'a crate::desktop_cleanup::DesktopExecution<'_>,
        expires_at: u64,
    ) -> Result<Self, NativeOutcome> {
        Self::prepare_inner(&execution.manifest().primary, expires_at, true)
    }

    fn prepare(m: &'a CleanupManifest, expires_at: u64) -> Result<Self, NativeOutcome> {
        Self::prepare_inner(m, expires_at, false)
    }

    fn prepare_inner(
        m: &'a CleanupManifest,
        expires_at: u64,
        desktop: bool,
    ) -> Result<Self, NativeOutcome> {
        let writers =
            scoped_files::WriterFence::acquire(m).map_err(NativeOutcome::NeverDispatched)?;
        scoped_files::validate_before_unlink(m).map_err(NativeOutcome::NeverDispatched)?;
        // Sidecars precede SQLite and outlive it to preserve POSIX locks.
        let mut sidecars = if desktop {
            scoped_files::DatabaseSidecars::acquire_desktop(m)
        } else {
            scoped_files::DatabaseSidecars::acquire(m)
        }
        .map_err(NativeOutcome::NeverDispatched)?;
        if desktop {
            sidecars
                .validate_before_database(m)
                .map_err(NativeOutcome::NeverDispatched)?;
        }
        let mut database =
            scoped_storage::DatabaseFence::acquire(m).map_err(NativeOutcome::Unknown)?;
        sidecars.bind_created(m).map_err(NativeOutcome::Unknown)?;
        let cwd = database.target_cwd(m).map_err(NativeOutcome::Unknown)?;
        crate::scoped_config::validate(&m.root.path, &cwd).map_err(NativeOutcome::Unknown)?;
        database.prepare_delete(m).map_err(NativeOutcome::Unknown)?;
        writers.validate(m).map_err(NativeOutcome::Unknown)?;
        sidecars.validate(m).map_err(NativeOutcome::Unknown)?;
        crate::scoped_config::validate(&m.root.path, &cwd).map_err(NativeOutcome::Unknown)?;
        if !not_expired(expires_at) {
            return Err(NativeOutcome::Unknown(EffectError::InvalidEvidence));
        }
        Ok(Self {
            manifest: m,
            cwd,
            postimage_held: false,
            writers,
            sidecars,
            database,
        })
    }

    pub(super) fn unlink_and_verify(&self) -> Result<(), EffectError> {
        self.writers.validate(self.manifest)?;
        self.sidecars.validate(self.manifest)?;
        crate::scoped_config::validate(&self.manifest.root.path, &self.cwd)?;
        scoped_files::validate_and_unlink(self.manifest)?;
        if !scoped_files::verify_retained_and_absence(self.manifest)? {
            return Err(EffectError::InvalidEvidence);
        }
        self.sidecars.validate(self.manifest)
    }

    pub(super) fn commit_and_verify(&mut self) -> Result<bool, EffectError> {
        self.sidecars.validate(self.manifest)?;
        if !self.database.commit_and_verify(self.manifest)? {
            return Ok(false);
        }
        let verified = self.database.hold_postimage(self.manifest)?;
        self.postimage_held = true;
        Ok(verified)
    }

    pub(super) fn verify_retained(&self) -> Result<bool, EffectError> {
        if !self.postimage_held {
            return Err(EffectError::InvalidEvidence);
        }
        Ok(self.database.verify_postimage(self.manifest)?
            && crate::scoped_config::validate(&self.manifest.root.path, &self.cwd).is_ok()
            && scoped_files::verify_retained_and_absence(self.manifest)?
            && self.writers.validate(self.manifest).is_ok()
            && self.sidecars.validate(self.manifest).is_ok())
    }
}

impl CodexScopedCleanupAdapter {
    fn execute(
        &self,
        plan: &LifecyclePlan,
        expires_at: u64,
    ) -> (NativeOutcome, Option<CleanupReport>) {
        let m = match self.manifest(plan) {
            Ok(m) => m,
            Err(e) => return (NativeOutcome::NeverDispatched(e), None),
        };
        self.execute_manifest(m, expires_at, &mut |_| Ok(()))
    }

    fn execute_manifest(
        &self,
        m: &CleanupManifest,
        expires_at: u64,
        progress: &mut dyn FnMut(
            agentmender_domain::CompleteRemovalAction,
        ) -> Result<(), EffectError>,
    ) -> (NativeOutcome, Option<CleanupReport>) {
        let mut fence = match PreparedPrimaryFence::prepare(m, expires_at) {
            Ok(v) => v,
            Err(outcome) => return (outcome, None),
        };
        // From this point any error may follow a successful unlink and is unknown.
        if let Err(e) = scoped_files::validate_and_unlink(m) {
            return (
                NativeOutcome::Unknown(e),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Unknown,
                    false,
                    false,
                )),
            );
        }
        if let Err(e) = progress(agentmender_domain::CompleteRemovalAction::PrimaryPayload) {
            return (
                NativeOutcome::Unknown(e),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Removed,
                    false,
                    false,
                )),
            );
        }
        if let Err(e) = fence.sidecars.validate(m) {
            return (
                NativeOutcome::Unknown(e),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Removed,
                    false,
                    false,
                )),
            );
        }
        #[cfg(test)]
        if self.fault == Some(TestFault::CrashAfterUnlink) {
            std::process::exit(86);
        }
        #[cfg(test)]
        if self.fault == Some(TestFault::AfterUnlink) {
            return (
                NativeOutcome::Unknown(EffectError::Transport),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Removed,
                    false,
                    false,
                )),
            );
        }
        let database_ok = match fence.database.commit_and_verify(m) {
            Ok(v) => v,
            Err(e) => {
                return (
                    NativeOutcome::Unknown(e),
                    Some(report(
                        m,
                        CleanupOutcome::OutcomeUnknown,
                        CleanupActionOutcome::Removed,
                        false,
                        false,
                    )),
                );
            }
        };
        if database_ok
            && let Err(e) = progress(agentmender_domain::CompleteRemovalAction::StateThreadRecords)
        {
            return (
                NativeOutcome::Unknown(e),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Removed,
                    true,
                    false,
                )),
            );
        }
        #[cfg(test)]
        if self.fault == Some(TestFault::CrashAfterCommit) {
            std::process::exit(87);
        }
        #[cfg(test)]
        if self.fault == Some(TestFault::AfterCommit) {
            return (
                NativeOutcome::Unknown(EffectError::Transport),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Removed,
                    false,
                    false,
                )),
            );
        }
        let files_ok = match scoped_files::verify_retained_and_absence(m) {
            Ok(v) => v,
            Err(e) => {
                return (
                    NativeOutcome::Unknown(e),
                    Some(report(
                        m,
                        CleanupOutcome::OutcomeUnknown,
                        CleanupActionOutcome::Removed,
                        false,
                        false,
                    )),
                );
            }
        };
        if !database_ok
            || !files_ok
            || fence.writers.validate(m).is_err()
            || fence.sidecars.validate(m).is_err()
        {
            return (
                NativeOutcome::Unknown(EffectError::InvalidEvidence),
                Some(report(
                    m,
                    CleanupOutcome::OutcomeUnknown,
                    CleanupActionOutcome::Removed,
                    false,
                    false,
                )),
            );
        }
        let outcome = if m.residuals.is_empty() && m.optional_index.is_none() {
            CleanupOutcome::Removed
        } else {
            CleanupOutcome::RemovedWithResiduals
        };
        let result = report(m, outcome, CleanupActionOutcome::Removed, true, true);
        if !validate_cleanup_report(m, &result) {
            return (NativeOutcome::Unknown(EffectError::InvalidEvidence), None);
        }
        (NativeOutcome::Accepted, Some(result))
    }
}

impl agentmender_provider_api::SessionMetadataReadPort for CodexScopedCleanupAdapter {
    fn session_metadata(
        &self,
    ) -> Result<agentmender_domain::SessionInventory, agentmender_provider_api::ProviderError> {
        self.validate_executable().map_err(|_| {
            agentmender_provider_api::ProviderError::CapabilityUnavailable(
                "exact stable 0.166.0 Darwin arm64 metadata profile required".into(),
            )
        })?;
        self.capture("00000000-0000-0000-0000-000000000000", true)?
            .session_metadata
            .ok_or_else(|| {
                agentmender_provider_api::ProviderError::CapabilityUnavailable(
                    "session metadata unavailable".into(),
                )
            })
    }
}

impl agentmender_provider_api::ScopedCleanupReadPort for CodexScopedCleanupAdapter {
    fn observe_cleanup(
        &self,
        target: &str,
        scope: &agentmender_domain::DiscoveryScope,
    ) -> Result<
        agentmender_provider_api::ScopedCleanupObservation,
        agentmender_provider_api::ProviderError,
    > {
        use agentmender_domain::*;
        use agentmender_provider_api::{ProviderError, ScopedCleanupObservation, validate_scope};
        let unavailable = || {
            ProviderError::CapabilityUnavailable(
                "scoped cleanup evidence unavailable or incomplete".into(),
            )
        };
        validate_scope(scope)?;
        if scope.project_path.is_some()
            || !scope.include_archived
            || self.reader.fixture.is_some()
            || self
                .protected_sessions
                .iter()
                .any(|protected| protected == target)
        {
            return Err(unavailable());
        }
        let desktop_pin = self
            .desktop_user_data
            .as_deref()
            .map(|path| crate::desktop_cleanup::validate_read_profile(self, path))
            .transpose()?;
        self.validate_executable().map_err(|_| unavailable())?;
        let db = self.capture_target(target)?;
        if self
            .protected_sessions
            .iter()
            .any(|id| !db.known_threads.iter().any(|r| &r.id == id))
        {
            return Err(unavailable());
        }
        let cwd = db.cwd.as_deref().ok_or_else(unavailable)?;
        crate::scoped_config::validate(&self.reader.location.state_root, cwd)
            .map_err(|_| unavailable())?;
        if db.history_mode.as_deref() != Some("legacy")
            || db.dependent_rows != 0
            || db.rows.iter().any(|r| {
                matches!(r.table.as_str(), "thread_spawn_edges") && r.count != 0
                    || (!self.expanded
                        && matches!(
                            r.table.as_str(),
                            "thread_attachments" | "guardian_review_feedback"
                        )
                        && r.count != 0)
            })
        {
            return Err(unavailable());
        }
        let path = db.target_path.as_ref().ok_or_else(unavailable)?;
        let files = if self.desktop_user_data.is_some() {
            scoped_files::observe_desktop(&self.reader.location.state_root, target, path)
        } else {
            scoped_files::observe(&self.reader.location.state_root, target, path)
        }
        .map_err(|_| unavailable())?;
        let inventory = scoped_files::inspect_rollouts(&self.reader.location.state_root)
            .map_err(|_| unavailable())?;
        let inventory_keys: std::collections::BTreeSet<_> = inventory
            .iter()
            .map(|(file, id, _, _)| (&file.path, id.as_str()))
            .collect();
        if db
            .known_threads
            .iter()
            .any(|thread| !inventory_keys.contains(&(&thread.rollout_path, thread.id.as_str())))
        {
            return Err(unavailable());
        }
        let native = EntityId::new(target).map_err(|_| unavailable())?;
        let target_local = crate::scoped_id("session", target)?;
        let relative = path
            .strip_prefix(&self.reader.location.state_root)
            .map_err(|_| unavailable())?;
        let payload_local = crate::artifact_identity(relative).ok_or_else(unavailable)?;
        let provider = Provider {
            kind: ProviderKind::Codex,
            presence: Presence::Installed,
            state_root: self.reader.location.state_root.clone(),
            fingerprint: ProviderFingerprint { executable: self.reader.location.executable.clone(), executable_version: Some(SCOPED_CODEX_VERSION.into()), adapter_version: env!("CARGO_PKG_VERSION").into(), interface_id: Some(if self.expanded { SCOPED_EXPANDED_ADAPTER } else { SCOPED_ADAPTER }.into()), layout_id: Some(format!("codex-state-5:{SCOPED_CODEX_SCHEMA_SHA256}")) },
            capabilities: ProviderCapabilities::new(),
            compatibility: CompatibilityProfile { status: CompatibilityStatus::ReadOnlyKnown, interface_stability: InterfaceStability::InternalImplementationDetail, reason: "Exact scoped cleanup evidence only; native lifecycle and general provider compatibility remain separate.".into() },
            installations: vec![], evidence_status: EvidenceStatus::SnapshotVerified, diagnostics: vec![],
        };
        let fingerprint = provider.fingerprint.clone();
        let mut graph = ArtifactGraph::builder(GraphContext {
            provider,
            evidence_status: EvidenceStatus::SnapshotVerified,
            completeness: Completeness::Partial,
            coverage: "Exact scoped cleanup evidence; other provider state remains unverified"
                .into(),
            diagnostics: vec![],
        });
        let rule_id =
            EntityId::new("codex-scoped-cleanup-exact-evidence").map_err(|_| unavailable())?;
        graph.add_rule(EvidenceRule {
            id: rule_id.clone(),
            provider: ProviderKind::Codex,
            description: "Fixed schema and bounded rollout metadata observations".into(),
            admissible_sources: vec![EvidenceSource::ProviderSnapshot],
            max_confidence: Confidence::Authoritative,
            adapter_version: fingerprint.adapter_version,
            interface_id: fingerprint.interface_id,
            layout_id: fingerprint.layout_id,
        })?;
        let evidence = Evidence {
            source: EvidenceSource::ProviderSnapshot,
            confidence: Confidence::Authoritative,
            provider_version: Some(SCOPED_CODEX_VERSION.into()),
            rule: "scoped exact identity".into(),
        };
        let graph_evidence = GraphEvidence {
            observation: evidence.clone(),
            rule_id,
            nature: EvidenceNature::Observation,
            locator: None,
            observed_at: None,
        };
        let global = graph.observe(NodeObservation::ProviderState);
        graph.add_root(ProtectedRoot {
            node: global,
            kind: RootKind::ProviderGlobal,
        })?;
        for id in &self.protected_sessions {
            let node = graph.observe(NodeObservation::Session(Session {
                id: crate::scoped_id("session", id)?,
                provider_session_id: id.clone(),
                project_id: None,
                state: SessionState::Unknown,
                last_active: None,
                evidence: evidence.clone(),
            }));
            graph.add_root(ProtectedRoot {
                node: node.clone(),
                kind: RootKind::UserProtected,
            })?;
            graph.add_node_evidence(&node, graph_evidence.clone())?;
        }
        let session = graph.observe(NodeObservation::Session(Session {
            id: target_local,
            provider_session_id: target.into(),
            project_id: None,
            state: if db.archived == Some(true) {
                SessionState::Archived
            } else {
                SessionState::Active
            },
            last_active: None,
            evidence: evidence.clone(),
        }));
        graph.add_node_evidence(&session, graph_evidence.clone())?;
        let artifact = graph.observe(NodeObservation::Artifact(Artifact {
            id: payload_local,
            kind: ArtifactKind::Transcript,
            locator: path.clone(),
            size_bytes: Some(files.payload.size_bytes),
            evidence,
        }));
        graph.add_node_evidence(&artifact, graph_evidence)?;
        let manifest = CleanupManifest {
            target: session,
            native_session_id: native.clone(),
            root: files.root,
            profile: CleanupProfile {
                version: SCOPED_CODEX_VERSION.into(),
                executable_sha256: SCOPED_CODEX_EXECUTABLE_SHA256.into(),
                schema_sha256: SCOPED_CODEX_SCHEMA_SHA256.into(),
                adapter: if self.expanded {
                    SCOPED_EXPANDED_ADAPTER
                } else {
                    SCOPED_ADAPTER
                }
                .into(),
                platform: "darwin-arm64".into(),
                journal_mode: "wal".into(),
                history_layout: "legacy-plain".into(),
            },
            database: files.database,
            rows: db
                .rows
                .into_iter()
                .map(|r| CleanupRows {
                    table: r.table,
                    expected_count: r.count,
                    preimage_sha256: r.sha256,
                })
                .collect(),
            database_payload_paths: vec![CleanupDatabasePath { path: path.clone() }],
            payloads: vec![CleanupPayload {
                artifact,
                identity: files.payload,
                header_session_id: native,
                header_sha256: files.header_sha256,
            }],
            optional_index: None,
            retained_database_sha256: if self.expanded {
                db.expanded_retained_database_sha256
            } else {
                db.retained_database_sha256
            },
            retained_files: files.retained_files,
            residuals: vec![CleanupResidual {
                artifact: ArtifactId {
                    provider: ProviderKind::Codex,
                    state_root: self.reader.location.state_root.clone(),
                    class: GraphNodeKind::Artifact,
                    local_id: EntityId::new("codex:residual:unscanned-stores")
                        .map_err(|_| unavailable())?,
                },
                kind: CleanupResidualKind::Unknown,
            }],
        };
        if !scoped_profile::validate_manifest_profile(&manifest) {
            return Err(unavailable());
        }
        let primary_evidence = ScopedCleanupEvidence {
            manifest: manifest.clone(),
            root_boundary: CleanupBoundaryObservation::CompleteAndUnshared,
            references: CleanupBoundaryObservation::CompleteAndUnshared,
            writer_fence: CleanupBoundaryObservation::CompleteAndUnshared,
            schema: CleanupBoundaryObservation::CompleteAndUnshared,
        };
        let complete_removal_evidence = if self.expanded {
            Some(CompleteRemovalEvidence {
                desktop_boundary: None,
                manifest: CompleteRemovalManifest {
                    desktop: None,
                    primary: manifest.clone(),
                    actions: vec![
                        CompleteRemovalAction::PrimaryPayload,
                        CompleteRemovalAction::StateThreadRecords,
                    ],
                    profile: CompleteRemovalProfile {
                        identity: EntityId::new("codex-scoped-cleanup-v2")
                            .map_err(|_| unavailable())?,
                        admission: ProfileAdmission::PrimaryOnly,
                        writer_requirement: WriterRequirement::CompatibleLifetimeFence,
                    },
                    required_surfaces: vec![PersistentSurface {
                        id: EntityId::new("codex-desktop-reconstruction-unverified")
                            .map_err(|_| unavailable())?,
                    }],
                    technical_residuals: manifest.residuals.clone(),
                },
                primary_evidence: primary_evidence.clone(),
            })
        } else {
            None
        };
        let context = PlanningContext {
            scope: scope.clone(),
            capabilities: vec![],
            effects: None,
            cleanup_evidence: if self.expanded {
                None
            } else {
                Some(primary_evidence)
            },
            complete_removal_evidence,
        };
        let mut observation = ScopedCleanupObservation {
            graph: graph.finish()?,
            context,
        };
        if let (Some(path), Some(pin)) = (&self.desktop_user_data, desktop_pin) {
            crate::desktop_cleanup::observe(self, path, target, pin, &mut observation)?;
        }
        Ok(observation)
    }
}

#[cfg(test)]
#[path = "scoped_cleanup_acceptance.rs"]
mod acceptance;
