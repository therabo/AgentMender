//! Exact, receipt-gated Codex lifecycle integration. The read-only adapter remains separate.
use super::*;
use agentmender_persistence::{
    DurableEffectPort, DurableIntent, DurableObservation, EffectAttempt, EffectError,
    EffectObservation, NativeObservation, NativeOutcome,
};
use sha2::{Digest, Sha256};

const VERSION: &str = "0.165.0-alpha.1.2";
const HASH: &str = "d84a54b031085f01e3ad915ea7c72ebea4dcf69e90f0d1e0e4a637e8280991d2";
const LAYOUT: &str =
    "codex-state-5:1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647";
const PROTOCOL: &str = "sha256:14a6e75b4bae9ec7b4bb41f3ed9840296f4d671295f300532efc7a7ca9b3d686";
const CONTRACT: &str = "codex-unarchive-target-only-v165-sha256-d84a54b0-v1";

/// Trusted composition: executable, root and read-only evidence helper come from the reader.
pub struct CodexLifecycleAdapter {
    reader: CodexAdapter,
}
impl CodexLifecycleAdapter {
    pub fn new(reader: CodexAdapter) -> Self {
        Self { reader }
    }
    fn validate_plan(&self, plan: &LifecyclePlan) -> Result<(), EffectError> {
        self.validate_plan_content(plan.content())
    }
    fn validate_plan_content(&self, p: &PlanContent) -> Result<(), EffectError> {
        if p.request.intent != LifecycleIntent::Unarchive
            || p.operations.len() != 1
            || p.operations[0].operation != NativeLifecycleOperation::UnarchiveSession
        {
            return Err(EffectError::UnsupportedOperation);
        }
        let op = &p.operations[0];
        let target = &p.request.target;
        if target.provider != ProviderKind::Codex
            || target.class != GraphNodeKind::Session
            || target.state_root != self.reader.location.state_root
            || op.target != *target
            || op.affected.as_slice() != [target.clone()]
            || p.native_target_id.as_ref() != Some(&op.native_id)
        {
            return Err(EffectError::NativeIdentityMismatch);
        }
        if p.status != PlanStatus::Ready
            || p.evidence_status != EvidenceStatus::SnapshotVerified
            || p.compatibility_status != CompatibilityStatus::ReadOnlyKnown
            || self.reader.fixture.is_some()
        {
            return Err(EffectError::InvalidEvidence);
        }
        let cap = p.capability.as_ref().ok_or(EffectError::ProfileMismatch)?;
        let contract = p
            .effect_contract
            .as_ref()
            .ok_or(EffectError::ProfileMismatch)?;
        if cap.provider != ProviderKind::Codex
            || cap.operation != NativeLifecycleOperation::UnarchiveSession
            || cap.effect_scope != NativeEffectScope::TargetRestoreWithProviderManagedState
            || cap.protocol_evidence_digest != PROTOCOL
            || cap.fingerprint != p.provider
            || contract.id.as_str() != CONTRACT
            || contract.affected.as_slice() != [target.clone()]
        {
            return Err(EffectError::ProfileMismatch);
        }
        let fp = &p.provider;
        if fp.executable.as_ref() != self.reader.location.executable.as_ref()
            || fp.executable_version.as_deref() != Some(VERSION)
            || fp.interface_id.as_deref() != Some("codex-app-server-0.165.0-alpha.1.2")
            || fp.layout_id.as_deref() != Some(LAYOUT)
            || fp.adapter_version != env!("CARGO_PKG_VERSION")
        {
            return Err(EffectError::ProfileMismatch);
        }
        if p.observation_scope.project_path.is_some()
            || !p.observation_scope.include_archived
            || self.reader.evidence_helper.is_none()
        {
            return Err(EffectError::InvalidEvidence);
        }
        Ok(())
    }
    fn validate_executable(&self) -> Result<(), EffectError> {
        if !cfg!(target_os = "macos") {
            return Err(EffectError::UnsupportedVersion);
        }
        let executable = self
            .reader
            .location
            .executable
            .as_deref()
            .ok_or(EffectError::Unavailable)?;
        if !executable.is_absolute() || !executable.is_file() {
            return Err(EffectError::Unavailable);
        }
        let file = std::fs::File::open(executable).map_err(|_| EffectError::Unavailable)?;
        let before = file.metadata().map_err(|_| EffectError::Unavailable)?;
        if before.len() > 256 * 1024 * 1024 {
            return Err(EffectError::ProfileMismatch);
        }
        let mut hash = Sha256::new();
        let mut input = std::io::BufReader::new(&file);
        let mut buffer = [0u8; 65536];
        loop {
            let n = input
                .read(&mut buffer)
                .map_err(|_| EffectError::Unavailable)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        let after = file.metadata().map_err(|_| EffectError::Unavailable)?;
        if before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
            || hash
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                != HASH
        {
            return Err(EffectError::ProfileMismatch);
        }
        #[cfg(target_os = "macos")]
        {
            let bundle = bundle_root(executable).ok_or(EffectError::ProfileMismatch)?;
            for (name, size, hash) in RESOURCES {
                read_pinned_source(&bundle.join(name), size, hash, Some(size))?;
            }
        }
        if process::probe_version(ProviderKind::Codex, executable)
            .map_err(|_| EffectError::Unavailable)?
            != VERSION
        {
            return Err(EffectError::UnsupportedVersion);
        }
        Ok(())
    }
}
impl DurableEffectPort for CodexLifecycleAdapter {
    fn preflight(&self, plan: &LifecyclePlan) -> Result<(), EffectError> {
        self.validate_plan(plan)?;
        self.validate_executable()
    }
    fn dispatch(&self, intent: DurableIntent, plan: &LifecyclePlan) -> EffectAttempt {
        let matches = intent.plan_digest() == plan.digest()
            && intent.state_fingerprint() == plan.state_fingerprint()
            && intent.operation_index() == 0;
        let expires_at = intent.expires_at();
        let observation = intent.into_observation();
        let outcome = if !matches {
            NativeOutcome::NeverDispatched(EffectError::InvalidEvidence)
        } else if let Err(e) = self.preflight(plan) {
            NativeOutcome::NeverDispatched(e)
        } else if !grant_valid(expires_at) {
            NativeOutcome::NeverDispatched(EffectError::InvalidEvidence)
        } else {
            self.launch(&observation, plan, expires_at)
        };
        EffectAttempt {
            observation,
            outcome,
        }
    }
    fn observe(&self, plan: &LifecyclePlan) -> Result<EffectObservation, EffectError> {
        self.preflight(plan)?;
        let snapshot = self
            .reader
            .discover(&plan.content().observation_scope)
            .map_err(|_| EffectError::Unavailable)?;
        let graph = self
            .reader
            .build_graph(&snapshot)
            .map_err(|_| EffectError::InvalidEvidence)?;
        let context = self
            .reader
            .integrity_context(&graph)
            .map_err(|_| EffectError::InvalidEvidence)?;
        let integrity = evaluate_integrity(&graph, &context);
        let coherent = snapshot.evidence_status == EvidenceStatus::SnapshotVerified
            && snapshot.provider.fingerprint == plan.content().provider
            && !snapshot.diagnostics.iter().any(|d| {
                matches!(
                    d.code,
                    DiagnosticCode::SnapshotStale
                        | DiagnosticCode::ConcurrentChange
                        | DiagnosticCode::MalformedProviderState
                        | DiagnosticCode::LimitReached
                )
            });
        let id = plan
            .content()
            .native_target_id
            .as_ref()
            .ok_or(EffectError::NativeIdentityMismatch)?
            .as_str();
        let found: Vec<_> = snapshot
            .sessions
            .iter()
            .filter(|s| s.provider_session_id == id)
            .collect();
        let native = if !coherent
            || found.len() != 1
            || found[0].id != plan.content().request.target.local_id
        {
            NativeObservation::Unknown
        } else {
            match found[0].state {
                SessionState::Active => NativeObservation::Active,
                SessionState::Archived => NativeObservation::Archived,
                SessionState::Unknown => NativeObservation::Unknown,
            }
        };
        Ok(EffectObservation {
            native,
            graph,
            integrity,
        })
    }
}

impl CodexLifecycleAdapter {
    #[cfg(not(target_os = "macos"))]
    fn launch(&self, _: &DurableObservation, _: &LifecyclePlan, _: u64) -> NativeOutcome {
        NativeOutcome::NeverDispatched(EffectError::UnsupportedVersion)
    }

    #[cfg(target_os = "macos")]
    fn launch(
        &self,
        _: &DurableObservation,
        plan: &LifecyclePlan,
        expires_at: u64,
    ) -> NativeOutcome {
        use agentmender_provider_api::evidence::EvidenceSandbox;
        use std::process::{Command, Stdio};
        let Some((_, home)) = &self.reader.evidence_helper else {
            return NativeOutcome::NeverDispatched(EffectError::Unavailable);
        };
        let root = &self.reader.location.state_root;
        let scratch = match EvidenceSandbox::new(root, home) {
            Ok(v) => v,
            Err(_) => return NativeOutcome::NeverDispatched(EffectError::Unavailable),
        };
        if scratch.validate().is_err()
            || scratch.source_root() != root
            || scratch.source_root() == scratch.real_home()
        {
            return NativeOutcome::NeverDispatched(EffectError::InvalidEvidence);
        }
        let Some(exe) = self.reader.location.executable.as_deref() else {
            return NativeOutcome::NeverDispatched(EffectError::Unavailable);
        };
        fn literal(p: &Path) -> Option<String> {
            let s = p.to_str()?;
            if s.chars().any(char::is_control) {
                return None;
            }
            Some(format!(
                "\"{}\"",
                s.replace('\\', "\\\\").replace('"', "\\\"")
            ))
        }
        let pinned = match pin_image(exe, &scratch) {
            Ok(pinned) => pinned,
            Err(e) => return NativeOutcome::NeverDispatched(e),
        };
        let original_bundle = match bundle_root(exe) {
            Some(path) => path,
            None => return NativeOutcome::NeverDispatched(EffectError::ProfileMismatch),
        };
        let (
            Some(root_rule),
            Some(scratch_rule),
            Some(home_rule),
            Some(exe_rule),
            Some(bundle_rule),
            Some(source_bundle_rule),
        ) = (
            literal(root),
            literal(scratch.path()),
            literal(scratch.real_home()),
            literal(&pinned),
            literal(&scratch.path().join("PinnedCodex.app")),
            literal(&original_bundle),
        )
        else {
            return NativeOutcome::NeverDispatched(EffectError::InvalidEvidence);
        };
        let profile = format!(
            "(version 1)(allow default)(deny file-write* (require-not (require-any (subpath {root_rule}) (subpath {scratch_rule}))))(deny file-write* (subpath {bundle_rule}))(deny file-read-data (subpath {source_bundle_rule}))(deny network*)(deny process-fork)(deny mach-lookup)(deny file-read-data (require-all (subpath {home_rule}) (require-not (require-any (subpath {root_rule}) (subpath {scratch_rule}) (literal {exe_rule})))) )"
        );
        if let Err(e) = self.validate_executable() {
            return NativeOutcome::NeverDispatched(e);
        }
        if !grant_valid(expires_at) {
            return NativeOutcome::NeverDispatched(EffectError::InvalidEvidence);
        }
        let child = Command::new("/usr/bin/sandbox-exec")
            .args(["-p", &profile])
            .arg(&pinned)
            .args(["app-server", "--listen", "stdio://"])
            .env_clear()
            .env("HOME", scratch.path())
            .env("TMPDIR", scratch.path())
            .env("CODEX_HOME", root)
            .current_dir(scratch.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let child = match child {
            Ok(child) => child,
            Err(_) => return NativeOutcome::NeverDispatched(EffectError::Unavailable),
        };
        let result = (|| -> Result<(), EffectError> {
            let mut transport = Transport::from_child(child, Duration::from_secs(10))
                .map_err(|_| EffectError::Transport)?;
            transport.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"agentmender","title":"AgentMender","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}}})).map_err(|_| EffectError::Transport)?;
            validate_initialize(&lifecycle_response(&mut transport, 1)?, root)
                .map_err(|_| EffectError::ProtocolMismatch)?;
            transport
                .send(json!({"method":"initialized","params":{}}))
                .map_err(|_| EffectError::Transport)?;
            if !grant_valid(expires_at) {
                return Err(EffectError::InvalidEvidence);
            }
            transport.send(json!({"id":2,"method":"thread/unarchive","params":{"threadId":plan.content().operations[0].native_id.as_str()}})).map_err(|_| EffectError::Transport)?;
            let response = lifecycle_response(&mut transport, 2)?;
            validate_unarchive_response(&response, plan.content().operations[0].native_id.as_str())
        })();
        match result {
            Ok(()) => NativeOutcome::Accepted,
            Err(e) => NativeOutcome::Unknown(e),
        }
    }
}

#[cfg(any(target_os = "macos", all(test, unix)))]
fn lifecycle_response(transport: &mut Transport, request_id: u64) -> Result<Value, EffectError> {
    for _ in 0..256 {
        let remaining = transport.deadline.saturating_duration_since(Instant::now());
        let value = match transport.incoming.recv_timeout(remaining) {
            Ok(Ok(value)) => value,
            Ok(Err(())) => return Err(EffectError::ProtocolMismatch),
            Err(mpsc::RecvTimeoutError::Timeout) => return Err(EffectError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(EffectError::ProcessTerminated);
            }
        };
        if value.get("id").and_then(Value::as_u64) == Some(request_id) {
            if value.get("error").is_some() {
                return Err(EffectError::ProviderRejected);
            }
            return Ok(value);
        }
        if value.get("id").is_some() {
            return Err(EffectError::ProtocolMismatch);
        }
    }
    Err(EffectError::ProtocolMismatch)
}

#[cfg(any(target_os = "macos", test))]
fn validate_unarchive_response(response: &Value, id: &str) -> Result<(), EffectError> {
    let thread = response
        .get("result")
        .and_then(|r| r.get("thread"))
        .and_then(Value::as_object)
        .ok_or(EffectError::ProtocolMismatch)?;
    let required = [
        "cliVersion",
        "createdAt",
        "cwd",
        "ephemeral",
        "id",
        "modelProvider",
        "preview",
        "projectId",
        "sessionId",
        "source",
        "status",
        "turns",
        "updatedAt",
    ];
    if required.iter().any(|key| !thread.contains_key(*key))
        || thread.get("id").and_then(Value::as_str) != Some(id)
    {
        return Err(EffectError::ProtocolMismatch);
    }
    if !["cliVersion", "modelProvider", "preview", "sessionId"]
        .iter()
        .all(|key| thread.get(*key).is_some_and(Value::is_string))
        || thread.get("createdAt").and_then(Value::as_i64).is_none()
        || thread.get("updatedAt").and_then(Value::as_i64).is_none()
        || thread.get("ephemeral").and_then(Value::as_bool).is_none()
        || !thread
            .get("projectId")
            .is_some_and(|v| v.is_null() || v.is_string())
        || !thread
            .get("turns")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        || !thread
            .get("source")
            .is_some_and(|v| v.is_string() || v.is_object())
    {
        return Err(EffectError::ProtocolMismatch);
    }
    let parsed: WireThread = serde_json::from_value(Value::Object(thread.clone()))
        .map_err(|_| EffectError::ProtocolMismatch)?;
    if !parsed.cwd.is_absolute() {
        return Err(EffectError::ProtocolMismatch);
    }
    if parsed.id != id {
        return Err(EffectError::NativeIdentityMismatch);
    }
    Ok(())
}

fn grant_valid(expires_at: u64) -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .is_ok_and(|now| now.as_secs() < expires_at)
}

#[cfg(target_os = "macos")]
fn bundle_root(exe: &Path) -> Option<PathBuf> {
    if exe.file_name()? != "codex"
        || exe.parent()?.file_name()? != "MacOS"
        || exe.parent()?.parent()?.file_name()? != "Contents"
    {
        return None;
    }
    let root = exe.parent()?.parent()?.parent()?;
    if root.extension()? != "app" || !root.is_absolute() {
        return None;
    }
    Some(root.to_path_buf())
}

#[cfg(target_os = "macos")]
const RESOURCES: [(&str, usize, &str); 3] = [
    (
        "Contents/Info.plist",
        525,
        "c40b0e6b87b94ef5a232ab8312678c5db2156428f57c3f3d6c32525373be7f59",
    ),
    (
        "Contents/_CodeSignature/CodeResources",
        2355,
        "a768efff8337bb3aa892c1ae2527cca49ef878abdba59ad6674e685ab3c4a288",
    ),
    (
        "Contents/embedded.provisionprofile",
        12329,
        "3e61e790c2b949afab19c726a6b6291adfe6518df678fd2eb28e88269a19ecac",
    ),
];

#[cfg(target_os = "macos")]
fn read_pinned_source(
    path: &Path,
    limit: usize,
    expected_hash: &str,
    exact_size: Option<usize>,
) -> Result<Vec<u8>, EffectError> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| EffectError::Unavailable)?;
    if !meta.file_type().is_file()
        || meta.len() > limit as u64
        || exact_size.is_some_and(|size| meta.len() != size as u64)
    {
        return Err(EffectError::ProfileMismatch);
    }
    let file = std::fs::File::open(path).map_err(|_| EffectError::Unavailable)?;
    let before = file.metadata().map_err(|_| EffectError::Unavailable)?;
    let mut data = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| EffectError::Unavailable)?;
    if data.len() > limit
        || data.len() as u64 != before.len()
        || Sha256::digest(&data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            != expected_hash
    {
        return Err(EffectError::ProfileMismatch);
    }
    Ok(data)
}

/// Copy only the four exact attested bundle files into a fresh owned sandbox.
#[cfg(target_os = "macos")]
fn pin_image(
    exe: &Path,
    sandbox: &agentmender_provider_api::evidence::EvidenceSandbox,
) -> Result<PathBuf, EffectError> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    sandbox
        .validate()
        .map_err(|_| EffectError::InvalidEvidence)?;
    let scratch = sandbox.path();
    let source = bundle_root(exe).ok_or(EffectError::ProfileMismatch)?;
    let bundle = scratch.join("PinnedCodex.app");
    for dir in [
        bundle.clone(),
        bundle.join("Contents"),
        bundle.join("Contents/MacOS"),
        bundle.join("Contents/_CodeSignature"),
    ] {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(|_| EffectError::Unavailable)?;
    }
    let mut files = vec![("Contents/MacOS/codex", 256 * 1024 * 1024, HASH, None)];
    files.extend(
        RESOURCES
            .iter()
            .map(|(name, size, hash)| (*name, *size, *hash, Some(*size))),
    );
    for (name, limit, hash, size) in files {
        let bytes = read_pinned_source(&source.join(name), limit, hash, size)?;
        let destination = bundle.join(name);
        let mode = if name == "Contents/MacOS/codex" {
            0o500
        } else {
            0o400
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&destination)
            .map_err(|_| EffectError::Unavailable)?;
        file.write_all(&bytes)
            .map_err(|_| EffectError::Unavailable)?;
        file.sync_all().map_err(|_| EffectError::Unavailable)?;
        drop(file);
        let metadata =
            std::fs::symlink_metadata(&destination).map_err(|_| EffectError::Unavailable)?;
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.mode() & 0o777 != mode {
            return Err(EffectError::InvalidEvidence);
        }
        read_pinned_source(&destination, limit, hash, size)?;
    }
    Ok(bundle.join("Contents/MacOS/codex"))
}

#[cfg(all(test, target_os = "macos"))]
#[path = "lifecycle_live_tests.rs"]
mod lifecycle_live_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(dead_code)]
    mod fixtures {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../domain/tests/common/mod.rs"
        ));
    }

    #[test]
    fn only_unarchive_is_eligible_and_synthetic_plans_are_rejected() {
        let reader = CodexAdapter::new(ProviderLocation {
            state_root: "/synthetic-planning".into(),
            executable: None,
        });
        let adapter = CodexLifecycleAdapter::new(reader);
        let case = fixtures::case(0);
        for operation in [
            LifecycleIntent::Archive,
            LifecycleIntent::SoftDelete,
            LifecycleIntent::Purge,
        ] {
            assert_eq!(
                adapter.preflight(&case.plan(operation)),
                Err(EffectError::UnsupportedOperation)
            );
        }
        assert!(
            adapter
                .preflight(&case.plan(LifecycleIntent::Unarchive))
                .is_err()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn pin_source_checks_digest_size_and_symlinks() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source");
        std::fs::write(&path, b"known").unwrap();
        let hash = Sha256::digest(b"known")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            read_pinned_source(&path, 5, &hash, Some(5)).unwrap(),
            b"known"
        );
        assert!(read_pinned_source(&path, 4, &hash, Some(5)).is_err());
        assert!(read_pinned_source(&path, 5, HASH, Some(5)).is_err());
        let link = dir.path().join("link");
        symlink(&path, &link).unwrap();
        assert!(read_pinned_source(&link, 5, &hash, Some(5)).is_err());
        assert!(bundle_root(Path::new("/tmp/Fake.app/Contents/MacOS/codex")).is_some());
        assert!(bundle_root(Path::new("/tmp/Fake.app/Contents/Other/codex")).is_none());
    }

    #[test]
    fn expired_grant_never_starts() {
        assert!(!grant_valid(0));
    }
    #[cfg(unix)]
    #[test]
    fn bounded_transport_classifies_fake_server_failures() {
        use std::process::{Command, Stdio};
        fn fake(script: &str, timeout: Duration) -> Transport {
            let child = Command::new("/bin/sh")
                .args(["-c", script])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            Transport::from_child(child, timeout).unwrap()
        }
        let mut rejected = fake(
            "printf '%s\\n' '{\"id\":2,\"error\":{}}'",
            Duration::from_secs(1),
        );
        assert_eq!(
            lifecycle_response(&mut rejected, 2),
            Err(EffectError::ProviderRejected)
        );
        let mut malformed = fake("printf '%s\\n' 'not-json'", Duration::from_secs(1));
        assert_eq!(
            lifecycle_response(&mut malformed, 2),
            Err(EffectError::ProtocolMismatch)
        );
        let mut exited = fake("exit 0", Duration::from_secs(1));
        assert_eq!(
            lifecycle_response(&mut exited, 2),
            Err(EffectError::ProcessTerminated)
        );
        let mut stalled = fake("exec /bin/sleep 3", Duration::from_millis(20));
        assert_eq!(
            lifecycle_response(&mut stalled, 2),
            Err(EffectError::Timeout)
        );
        let mut wrong_id = fake(
            "printf '%s\\n' '{\"id\":3,\"result\":{}}'",
            Duration::from_secs(1),
        );
        assert_eq!(
            lifecycle_response(&mut wrong_id, 2),
            Err(EffectError::ProtocolMismatch)
        );
    }

    #[test]
    fn response_requires_correlated_thread_and_schema() {
        let cwd = if cfg!(windows) {
            r"C:\synthetic-response"
        } else {
            "/synthetic-response"
        };
        let thread = json!({"cliVersion":"x","createdAt":0,"cwd":cwd,"ephemeral":false,"id":"target","modelProvider":"x","preview":"","projectId":null,"sessionId":"target","source":"cli","status":{"type":"idle"},"turns":[],"updatedAt":0});
        assert!(
            validate_unarchive_response(&json!({"result":{"thread":thread}}), "target").is_ok()
        );
        assert_eq!(
            validate_unarchive_response(&json!({"result":{"thread":thread}}), "other"),
            Err(EffectError::ProtocolMismatch)
        );
        assert_eq!(
            validate_unarchive_response(&json!({"result":{"thread":{"id":"target"}}}), "target"),
            Err(EffectError::ProtocolMismatch)
        );
    }
}
