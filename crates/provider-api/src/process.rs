//! Fixed process commands for isolated reads and confirmed native removal.
use crate::ProviderError;
use crate::evidence::EvidenceSandbox;
use agentmender_domain::ProviderKind;
use std::{path::Path, process::Child, time::Duration};

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Instant,
};

#[cfg(target_os = "macos")]
const PROFILE: &str = "(version 1)(allow default)(deny file-write*)(deny network*)(deny process-fork)(deny mach-lookup)";

#[cfg(target_os = "macos")]
fn command(executable: &Path) -> Result<Command, ProviderError> {
    if !executable.is_absolute() {
        return Err(ProviderError::CapabilityUnavailable(
            "provider executable must be an absolute path".into(),
        ));
    }
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command.args(["-p", PROFILE]).arg(executable);
    // Environment secrets and dynamic loader injection are not inherited.
    command.env_clear();
    for name in ["HOME", "PATH", "TMPDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    Ok(command)
}

#[derive(Debug, PartialEq, Eq)]
pub enum NativeExit {
    Refused,
    Success,
    Uncertain,
}

/// Apply the reviewed Codex home and grant rollout writes only at reviewed paths.
pub fn run_confined_codex_delete(
    executable: &Path,
    root: &Path,
    id: uuid::Uuid,
    rollouts: &[&Path],
    timeout: Duration,
    is_current: impl FnOnce() -> bool,
) -> NativeExit {
    #[cfg(target_os = "macos")]
    {
        if !executable.is_absolute() || !root.is_absolute() {
            return NativeExit::Refused;
        }
        let sessions = root.join("sessions");
        let archived = root.join("archived_sessions");
        let Ok(scope) = policy_literal(root) else {
            return NativeExit::Refused;
        };
        let (Ok(sessions_scope), Ok(archived_scope)) =
            (policy_literal(&sessions), policy_literal(&archived))
        else {
            return NativeExit::Refused;
        };
        let mut approved = String::new();
        for path in rollouts {
            if !path
                .strip_prefix(&sessions)
                .or_else(|_| path.strip_prefix(&archived))
                .is_ok_and(|relative| {
                    !relative.as_os_str().is_empty()
                        && relative
                            .components()
                            .all(|part| matches!(part, std::path::Component::Normal(_)))
                })
            {
                return NativeExit::Refused;
            }
            let Ok(path) = policy_literal(path) else {
                return NativeExit::Refused;
            };
            approved.push_str(&format!(" (literal {path})"));
        }
        let mut policy = format!(
            "(version 1)(allow default)(deny file-write* (require-not (subpath {scope})))(deny network*)(deny process-fork)(deny mach-lookup)"
        );
        // An archive mutation after the final snapshot may introduce a new file.
        // The provider can remove only the reviewed rollout files in these trees.
        if approved.is_empty() {
            policy.push_str(&format!(
                "(deny file-write* (subpath {sessions_scope}) (subpath {archived_scope}))"
            ));
        } else {
            policy.push_str(&format!(
                "(deny file-write* (require-all (require-any (subpath {sessions_scope}) (subpath {archived_scope})) (require-not (require-any{approved}))))"
            ));
        }
        for name in [
            "exports",
            "attachments",
            "generated_images",
            "worktrees",
            "visualizations",
            "sqlite",
        ] {
            let Ok(protected) = policy_literal(&root.join(name)) else {
                return NativeExit::Refused;
            };
            policy.push_str(&format!("(deny file-write* (subpath {protected}))"));
        }
        let mut command = Command::new("/usr/bin/sandbox-exec");
        command
            .args(["-p", &policy])
            .arg(executable)
            .arg("-c")
            .arg(format!("sqlite_home={scope}"))
            .arg("delete")
            .arg(id.hyphenated().to_string())
            .arg("--force")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("CODEX_HOME", root)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for name in ["HOME", "TMPDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        if !is_current() {
            return NativeExit::Refused;
        }
        let Ok(mut child) = command.spawn() else {
            return NativeExit::Uncertain;
        };
        let started = Instant::now();
        while started.elapsed() < timeout {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return NativeExit::Success,
                Ok(Some(_)) => return NativeExit::Uncertain,
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        NativeExit::Uncertain
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (executable, root, id, rollouts, timeout, is_current);
        NativeExit::Refused
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleMetadata {
    pub identifier: String,
    pub version: String,
    pub executable: String,
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn small_output(mut command: Command) -> Result<Vec<u8>, ProviderError> {
    let mut child = command.spawn().map_err(|_| {
        ProviderError::CapabilityUnavailable("write-denied metadata process could not start".into())
    })?;
    let mut output = child.stdout.take().ok_or_else(|| {
        ProviderError::CapabilityUnavailable("metadata stdout unavailable".into())
    })?;
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .by_ref()
            .take(4097)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < Duration::from_secs(3) => {
                thread::sleep(Duration::from_millis(5))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProviderError::CapabilityUnavailable(
                    "write-denied metadata process failed or timed out".into(),
                ));
            }
        }
    };
    if !status.success() {
        return Err(ProviderError::CapabilityUnavailable(
            "write-denied metadata process returned failure".into(),
        ));
    }
    let bytes = receiver
        .recv_timeout(Duration::from_millis(500))
        .map_err(|_| ProviderError::CapabilityUnavailable("metadata output timed out".into()))?
        .map_err(|_| ProviderError::CapabilityUnavailable("metadata output unreadable".into()))?;
    if bytes.len() > 4096 {
        return Err(ProviderError::CapabilityUnavailable(
            "metadata output byte budget exceeded".into(),
        ));
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn linux_version_command(executable: &Path) -> Result<Command, ProviderError> {
    if !executable.is_absolute() {
        return Err(ProviderError::InvalidScope(
            "provider probe requires an absolute path".into(),
        ));
    }
    let mut process = Command::new("/usr/bin/bwrap");
    process.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--setenv",
        "HOME",
        "/nonexistent",
        "--setenv",
        "PATH",
        "/usr/bin",
        "--ro-bind",
        "/usr",
        "/usr",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--chdir",
        "/",
    ]);
    for system in ["/lib", "/lib64", "/bin"] {
        if Path::new(system).exists() {
            process.args(["--ro-bind", system, system]);
        }
    }
    process
        .arg("--ro-bind")
        .arg(executable)
        .args(["/provider", "--", "/provider"])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    Ok(process)
}

/// Probe an executable version without a shell or unsandboxed retry.
pub fn probe_version(kind: ProviderKind, executable: &Path) -> Result<String, ProviderError> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        #[cfg(target_os = "macos")]
        let mut process = command(executable)?;
        #[cfg(target_os = "linux")]
        let mut process = linux_version_command(executable)?;
        process.arg("--version");
        let bytes = small_output(process)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            ProviderError::CapabilityUnavailable("unrecognized provider version banner".into())
        })?;
        parse_version(kind, text).ok_or(ProviderError::CapabilityUnavailable(
            "unrecognized provider version banner".into(),
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (kind, executable);
        Err(ProviderError::CapabilityUnavailable(
            "provider process isolation is not yet verified on this platform; metadata audit only"
                .into(),
        ))
    }
}

/// Verify the fixed CLI command surface in a write-denied process. The provider
/// adapter separately admits only releases with an exercised behavioral contract.
pub fn probe_codex_delete(executable: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        let Ok(mut process) = command(executable) else {
            return false;
        };
        process.args(["delete", "--help"]);
        small_output(process).is_ok_and(|bytes| {
            std::str::from_utf8(&bytes).is_ok_and(|help| {
                help.lines()
                    .any(|line| line == "Usage: codex delete [OPTIONS] <SESSION>")
                    && help.lines().any(|line| line.trim() == "--force")
                    && help.lines().any(|line| {
                        line.trim() == "Delete without prompting. SESSION must be a UUID"
                    })
            })
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = executable;
        false
    }
}

/// Read three fixed app-bundle keys using the system's write-denied plist reader.
pub fn read_bundle_metadata(bundle: &Path) -> Result<BundleMetadata, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        if !bundle.is_absolute() {
            return Err(ProviderError::InvalidScope(
                "app bundle must be absolute".into(),
            ));
        }
        let info = bundle.join("Contents/Info.plist");
        let metadata = std::fs::symlink_metadata(&info).map_err(|e| crate::io_error(&info, e))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 65536 {
            return Err(ProviderError::InvalidScope(
                "bundle metadata must be a bounded regular file".into(),
            ));
        }
        let mut values = Vec::new();
        for key in [
            "CFBundleIdentifier",
            "CFBundleShortVersionString",
            "CFBundleExecutable",
        ] {
            let mut process = command(Path::new("/usr/bin/plutil"))?;
            process.args(["-extract", key, "raw", "-o", "-"]).arg(&info);
            let bytes = small_output(process)?;
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| {
                    ProviderError::CapabilityUnavailable("invalid bundle metadata encoding".into())
                })?
                .trim();
            if text.is_empty() || text.chars().any(char::is_control) {
                return Err(ProviderError::CapabilityUnavailable(
                    "invalid bundle metadata value".into(),
                ));
            }
            values.push(text.to_owned());
        }
        Ok(BundleMetadata {
            identifier: values[0].clone(),
            version: values[1].clone(),
            executable: values[2].clone(),
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bundle;
        Err(ProviderError::CapabilityUnavailable(
            "app bundle metadata discovery is only verified on macOS".into(),
        ))
    }
}

/// Strictly recognize official version banners; proximity never proves compatibility.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn parse_version(kind: ProviderKind, banner: &str) -> Option<String> {
    let line = banner.trim();
    let value = match kind {
        ProviderKind::Codex => line.strip_prefix("codex-cli ")?,
        ProviderKind::Claude => line.strip_suffix(" (Claude Code)")?,
    };
    if value.len() > 128
        || value
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, '.' | '-' | '+'))
    {
        return None;
    }
    let core = value.split(['-', '+']).next()?;
    let parts: Vec<_> = core.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some(value.into())
}

#[cfg(target_os = "macos")]
fn policy_literal(path: &Path) -> Result<String, ProviderError> {
    let text = path
        .to_str()
        .filter(|s| !s.chars().any(char::is_control))
        .ok_or_else(|| {
            ProviderError::InvalidScope("sandbox policy path must be UTF-8 without controls".into())
        })?;
    Ok(format!(
        "\"{}\"",
        text.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

#[cfg(target_os = "macos")]
fn content_command(executable: &Path, sandbox: &EvidenceSandbox) -> Result<Command, ProviderError> {
    sandbox.validate()?;
    if !executable.is_absolute() || !executable.is_file() {
        return Err(ProviderError::InvalidScope(
            "invalid worker executable".into(),
        ));
    }
    let root = policy_literal(sandbox.path())?;
    let source = policy_literal(sandbox.source_root())?;
    let home = policy_literal(sandbox.real_home())?;
    let program = policy_literal(
        &executable
            .canonicalize()
            .map_err(|e| crate::io_error(executable, e))?,
    )?;
    let database = policy_literal(&sandbox.source_root().join("state_5.sqlite"))?;
    let wal = policy_literal(&sandbox.source_root().join("state_5.sqlite-wal"))?;
    let shm = policy_literal(&sandbox.source_root().join("state_5.sqlite-shm"))?;
    let desktop_projects = policy_literal(&sandbox.source_root().join(".codex-global-state.json"))?;
    let sessions = policy_literal(&sandbox.source_root().join("sessions"))?;
    let archived = policy_literal(&sandbox.source_root().join("archived_sessions"))?;
    let policy = format!(
        "(version 1)(allow default)(deny file-write* (require-not (subpath {root})))(deny network*)(deny process-fork)(deny mach-lookup)(deny file-read-data (require-all (require-any (subpath {home}) (subpath {source})) (require-not (require-any (literal {program}) (literal {database}) (literal {wal}) (literal {shm}) (literal {desktop_projects}) (literal {source}) (subpath {sessions}) (subpath {archived}) (subpath {root})))) )"
    );
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command
        .args(["-p", &policy])
        .arg(executable)
        .env_clear()
        .env("HOME", sandbox.path())
        .env("CODEX_HOME", sandbox.path())
        .env("TMPDIR", sandbox.path())
        .current_dir(sandbox.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    Ok(command)
}

pub fn spawn_codex_content_capture(
    helper: &Path,
    sandbox: &EvidenceSandbox,
) -> Result<Child, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        content_command(helper, sandbox)?
            .arg("--internal-codex-content-capture")
            .spawn()
            .map_err(|_| ProviderError::CapabilityUnavailable("content worker unavailable".into()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (helper, sandbox);
        Err(ProviderError::CapabilityUnavailable(
            "content isolation unsupported on this platform".into(),
        ))
    }
}

pub fn verify_source_write_denial(root: &Path) -> Result<(), ProviderError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos_sandbox::verify_source_write_denial(root)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = root;
        Err(ProviderError::CapabilityUnavailable(
            "kernel sandbox attestation is only verified on macOS".into(),
        ))
    }
}

/// Resolve executable paths without invoking the shell or probing unrelated commands.
pub fn find_executable(kind: ProviderKind) -> Option<std::path::PathBuf> {
    find_executables(kind).into_iter().next()
}

pub fn find_executables(kind: ProviderKind) -> Vec<std::path::PathBuf> {
    let name = match kind {
        ProviderKind::Codex => "codex",
        ProviderKind::Claude => "claude",
    };
    std::env::var_os("PATH")
        .iter()
        .flat_map(|p| std::env::split_paths(p))
        .take(128)
        .filter(|directory| {
            #[cfg(windows)]
            {
                use std::path::{Component, Prefix};
                directory.is_absolute()
                    && matches!(directory.components().next(), Some(Component::Prefix(prefix))
                    if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
            }
            #[cfg(not(windows))]
            directory.is_absolute()
        })
        .map(|directory| {
            directory.join(if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.into()
            })
        })
        .filter(|candidate| candidate.is_file())
        .collect()
}

/// Read Microsoft's registered package metadata. Only the OS-supplied
/// PowerShell executable and a fixed, parameter-free query are used.
#[cfg(windows)]
pub fn codex_windows_packages() -> Result<Vec<u8>, ProviderError> {
    let root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or_else(|| {
            ProviderError::CapabilityUnavailable("Windows system root unavailable".into())
        })?;
    let mut command = Command::new(root.join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$env:PSModulePath=Join-Path $PSHOME Modules; [Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-AppxPackage -Name OpenAI.Codex* | Select-Object Name,PublisherId,InstallLocation,PackageFamilyName,@{n='Architecture';e={$_.Architecture.ToString()}},@{n='Version';e={$_.Version.ToString()}} | ConvertTo-Json -Compress"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    small_output(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn content_policy_reads_only_declared_transcripts_and_denies_source_writes_auth_and_links() {
        use std::os::unix::fs::symlink;
        let source = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("sessions")).unwrap();
        let transcript = source.path().join("sessions/visible.jsonl");
        std::fs::write(&transcript, b"VISIBLE FIXTURE").unwrap();
        let auth = source.path().join("auth.json");
        std::fs::write(&auth, b"SECRET CANARY").unwrap();
        let link = source.path().join("sessions/linked.jsonl");
        symlink(&auth, &link).unwrap();
        let sandbox = EvidenceSandbox::new(source.path(), home.path()).unwrap();
        let output = content_command(Path::new("/bin/cat"), &sandbox)
            .unwrap()
            .arg(&transcript)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"VISIBLE FIXTURE");
        for path in [&auth, &link] {
            let output = content_command(Path::new("/bin/cat"), &sandbox)
                .unwrap()
                .arg(path)
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
        }
        assert!(
            !content_command(Path::new("/usr/bin/touch"), &sandbox)
                .unwrap()
                .arg(source.path().join("sessions/must-remain-absent"))
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(std::fs::read(auth).unwrap(), b"SECRET CANARY");
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn native_delete_timeout_stops_the_fixture_process() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap();
        let executable = path.join("fake-codex");
        std::fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' \"$$\" > pid\nexec /bin/sleep 15\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(
            run_confined_codex_delete(
                &executable,
                &path,
                uuid::Uuid::nil(),
                &[],
                Duration::from_secs(1),
                || true,
            ),
            NativeExit::Uncertain
        );
        let pid = std::fs::read_to_string(path.join("pid")).unwrap();
        let status = Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(
            !status.success(),
            "timed out fixture should have been reaped"
        );
    }

    #[test]
    fn banner_parser_keeps_exact_prereleases_and_rejects_ambiguous_output() {
        assert_eq!(
            parse_version(ProviderKind::Codex, "codex-cli 0.164.0-alpha.11.1\n"),
            Some("0.164.0-alpha.11.1".into())
        );
        assert_eq!(
            parse_version(ProviderKind::Claude, "2.1.311 (Claude Code)"),
            Some("2.1.311".into())
        );
        for value in [
            "codex-cli latest",
            "codex-cli 0.164",
            "codex-cli 0.164.0\nextra",
            "0.164.0",
        ] {
            assert!(parse_version(ProviderKind::Codex, value).is_none());
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn sandbox_denies_file_writes_before_execution() {
        // Canary touches a fresh fixture only. This is not a provider operation.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("must-not-exist");
        let status = Command::new("/usr/bin/sandbox-exec")
            .args(["-p", PROFILE, "/usr/bin/touch"])
            .arg(&target)
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success());
        assert!(!target.exists());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn sandbox_preserves_existing_files_against_mutation_operations() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("valuable");
        let renamed = dir.path().join("renamed");
        std::fs::write(&source, b"original").unwrap();
        let forbidden = [
            vec!["/bin/rm".into(), source.as_os_str().to_owned()],
            vec![
                "/bin/mv".into(),
                source.as_os_str().to_owned(),
                renamed.as_os_str().to_owned(),
            ],
            vec![
                "/bin/mkdir".into(),
                dir.path().join("new-directory").into_os_string(),
            ],
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "printf changed > \"$1\"".into(),
                "sandbox-canary".into(),
                source.as_os_str().to_owned(),
            ],
        ];
        for args in forbidden {
            let status = Command::new("/usr/bin/sandbox-exec")
                .args(["-p", PROFILE])
                .args(args)
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(!status.success());
            assert_eq!(std::fs::read(&source).unwrap(), b"original");
            assert!(!renamed.exists());
        }
    }
}
