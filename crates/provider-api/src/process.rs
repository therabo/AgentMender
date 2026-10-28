//! Provider processes are unavailable without verified OS write denial (ADR-0001).
use crate::ProviderError;
use crate::evidence::EvidenceSandbox;
use agentmender_domain::ProviderKind;
use std::{path::Path, process::Child};

#[cfg(target_os = "macos")]
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleMetadata {
    pub identifier: String,
    pub version: String,
    pub executable: String,
}

#[cfg(target_os = "macos")]
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

/// The only executable probe. No shell, arbitrary arguments or unsandboxed retry.
pub fn probe_version(kind: ProviderKind, executable: &Path) -> Result<String, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        let mut process = command(executable)?;
        process.arg("--version");
        let bytes = small_output(process)?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| ProviderError::ProviderVersionUnknown(kind))?;
        parse_version(kind, text).ok_or(ProviderError::ProviderVersionUnknown(kind))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (kind, executable);
        Err(ProviderError::CapabilityUnavailable(
            "provider process isolation is not yet verified on this platform; metadata audit only"
                .into(),
        ))
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
pub fn parse_version(kind: ProviderKind, banner: &str) -> Option<String> {
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

/// Private adapter transport must send only the fixed read-only request set.
/// Even a provider startup bug cannot write files under the inherited sandbox.
pub fn spawn_codex_read_only(executable: &Path, state_root: &Path) -> Result<Child, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        command(executable)?
            .args(["app-server", "--listen", "stdio://"])
            .env("CODEX_HOME", state_root)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|_| {
                ProviderError::CapabilityUnavailable(
                    "write-denied Codex App Server could not start".into(),
                )
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (executable, state_root);
        Err(ProviderError::CapabilityUnavailable(
            "Codex process isolation is not verified on this platform".into(),
        ))
    }
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
fn evidence_command(
    executable: &Path,
    sandbox: &EvidenceSandbox,
    capture: bool,
) -> Result<Command, ProviderError> {
    sandbox.validate()?;
    if !executable.is_absolute() || !executable.is_file() {
        return Err(ProviderError::CapabilityUnavailable(
            "evidence executable must be an absolute existing file".into(),
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
    let read_rule = if capture {
        let database = policy_literal(&sandbox.source_root().join("state_5.sqlite"))?;
        let wal = policy_literal(&sandbox.source_root().join("state_5.sqlite-wal"))?;
        let shm = policy_literal(&sandbox.source_root().join("state_5.sqlite-shm"))?;
        format!(
            "(deny file-read-data (require-all (subpath {home}) (require-not (require-any (literal {program}) (literal {database}) (literal {wal}) (literal {shm}) (subpath {root})))) )"
        )
    } else {
        format!(
            "(deny file-read-data (require-all (require-any (subpath {home}) (subpath {source})) (require-not (require-any (literal {program}) (subpath {root})))) )"
        )
    };
    let profile = format!(
        "(version 1)(allow default)(deny file-write* (require-not (subpath {root})))(deny network*)(deny process-fork)(deny mach-lookup){read_rule}"
    );
    let mut command = Command::new("/usr/bin/sandbox-exec");
    command
        .args(["-p", &profile])
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

/// A fixed internal query worker; writable permission comes only from an owned token.
pub fn spawn_codex_capture(
    helper: &Path,
    sandbox: &EvidenceSandbox,
) -> Result<Child, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        evidence_command(helper, sandbox, true)?
            .arg("--internal-codex-evidence-capture")
            .spawn()
            .map_err(|_| {
                ProviderError::CapabilityUnavailable(
                    "isolated evidence capture could not start".into(),
                )
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (helper, sandbox);
        Err(ProviderError::CapabilityUnavailable(
            "isolated evidence capture is not verified on this platform".into(),
        ))
    }
}

pub fn spawn_codex_evidence(
    executable: &Path,
    sandbox: &EvidenceSandbox,
) -> Result<Child, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        evidence_command(executable, sandbox, false)?
            .args(["app-server", "--listen", "stdio://"])
            .spawn()
            .map_err(|_| {
                ProviderError::CapabilityUnavailable(
                    "isolated Codex evidence process could not start".into(),
                )
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (executable, sandbox);
        Err(ProviderError::CapabilityUnavailable(
            "isolated Codex evidence runtime is not verified on this platform".into(),
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
    let name = match kind {
        ProviderKind::Codex => "codex",
        ProviderKind::Claude => "claude",
    };
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(name);
        #[cfg(windows)]
        let candidate = candidate.with_extension("exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn evidence_sandbox_allows_owned_writes_and_only_source_metadata_reads() {
        let source = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let private = source.path().join("private-content");
        std::fs::write(&private, b"secret-canary").unwrap();
        let sandbox = EvidenceSandbox::new(source.path(), home.path()).unwrap();
        let inside = sandbox.path().join("allowed");
        let status = evidence_command(Path::new("/usr/bin/touch"), &sandbox, false)
            .unwrap()
            .arg(&inside)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(inside.exists());
        let status = evidence_command(Path::new("/usr/bin/touch"), &sandbox, false)
            .unwrap()
            .arg(sandbox.probe_path())
            .status()
            .unwrap();
        assert!(!status.success());
        assert!(!sandbox.probe_path().exists());
        let status = evidence_command(Path::new("/usr/bin/stat"), &sandbox, false)
            .unwrap()
            .arg(&private)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        let output = evidence_command(Path::new("/bin/cat"), &sandbox, false)
            .unwrap()
            .arg(&private)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read(private).unwrap(), b"secret-canary");
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
