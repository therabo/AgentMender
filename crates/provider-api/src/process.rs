//! Provider processes are unavailable without verified OS write denial (ADR-0001).
use crate::ProviderError;
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

/// The only executable probe. No shell, arbitrary arguments or unsandboxed retry.
pub fn probe_version(kind: ProviderKind, executable: &Path) -> Result<String, ProviderError> {
    #[cfg(target_os = "macos")]
    {
        let mut child = command(executable)?.arg("--version").spawn().map_err(|_| {
            ProviderError::CapabilityUnavailable(
                "write-denied version probe could not start".into(),
            )
        })?;
        let mut output = child.stdout.take().ok_or_else(|| {
            ProviderError::CapabilityUnavailable("version stdout unavailable".into())
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
                        "version probe failed or timed out under write denial".into(),
                    ));
                }
            }
        };
        if !status.success() {
            return Err(ProviderError::ProviderVersionUnknown(kind));
        }
        let bytes = receiver
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| ProviderError::ProviderVersionUnknown(kind))?
            .map_err(|_| ProviderError::ProviderVersionUnknown(kind))?;
        if bytes.len() > 4096 {
            return Err(ProviderError::ProviderVersionUnknown(kind));
        }
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
