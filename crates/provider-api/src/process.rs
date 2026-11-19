//! Fixed process commands for isolated reads and confirmed native removal.
use crate::ProviderError;
use crate::evidence::EvidenceSandbox;
use agentmender_domain::ProviderKind;
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
use std::{io::Read, sync::mpsc, thread, time::Instant};

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

/// Invoke Codex's own deletion command in the selected local archive.
pub fn codex_delete(
    executable: &Path,
    root: &Path,
    sqlite_override: &str,
    id: uuid::Uuid,
) -> Result<String, String> {
    let output = Command::new(executable)
        .args(["-c", sqlite_override, "delete"])
        .arg(id.to_string())
        .arg("--force")
        .env("CODEX_HOME", root)
        .env("CODEX_SQLITE_HOME", root)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Avvio del comando Codex: {error}"))?;
    let message = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter(|line| agentmender_domain::safe_conversation_text(line))
            .take(12)
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(1200)
            .collect::<String>()
    };
    if output.status.success() {
        let detail = message(&output.stdout);
        return Ok(if detail.is_empty() {
            "Sessione eliminata da Codex.".into()
        } else {
            detail
        });
    }
    let detail = message(&output.stderr);
    let detail = if detail.is_empty() {
        message(&output.stdout)
    } else {
        detail
    };
    Err(if detail.is_empty() {
        format!("Comando Codex fallito ({})", output.status)
    } else {
        detail
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleMetadata {
    pub identifier: String,
    pub version: String,
    pub executable: String,
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn small_output(command: Command) -> Result<Vec<u8>, ProviderError> {
    bounded_output(command, 4096)
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn bounded_output(mut command: Command, limit: u64) -> Result<Vec<u8>, ProviderError> {
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
            .take(limit + 1)
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
    if bytes.len() as u64 > limit {
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
#[cfg(any(target_os = "macos", target_os = "linux"))]
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

pub struct ClaudeProcessObservation {
    pub status: &'static str,
    pub executables: Vec<std::path::PathBuf>,
}

/// A process-name observation has no authority over provider writers.
pub fn claude_writer_observation() -> ClaudeProcessObservation {
    let unavailable = ClaudeProcessObservation {
        status: "unavailable",
        executables: Vec::new(),
    };
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let mut command = {
        let mut command = Command::new("/bin/ps");
        command.env_clear().args(["-axo", "comm="]);
        command
    };
    #[cfg(windows)]
    let mut command = {
        let Some(root) = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .filter(|root| root.is_absolute())
        else {
            return unavailable;
        };
        let mut command = Command::new(root.join("System32/tasklist.exe"));
        command.env_clear().env("SystemRoot", &root).args([
            "/FI",
            "IMAGENAME eq claude.exe",
            "/FO",
            "CSV",
            "/NH",
        ]);
        command
    };
    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let Ok(bytes) = bounded_output(command, 64 * 1024) else {
            return unavailable;
        };
        let output = String::from_utf8_lossy(&bytes);
        let mut executables = Vec::new();
        let mut found = false;
        for line in output.lines() {
            #[cfg(unix)]
            let executable = Path::new(line.trim());
            #[cfg(windows)]
            let executable = Path::new(
                line.trim()
                    .strip_prefix('"')
                    .and_then(|value| value.split('"').next())
                    .unwrap_or(""),
            );
            if executable
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.eq_ignore_ascii_case("claude") || name.eq_ignore_ascii_case("claude.exe")
                })
            {
                found = true;
                if executable.is_absolute() && executables.len() < 64 {
                    executables.push(executable.into());
                }
            }
        }
        ClaudeProcessObservation {
            status: if found {
                "claude_process_observed"
            } else {
                "no_named_process_observed"
            },
            executables,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    unavailable
}

/// Read Microsoft's registered package metadata. Only the OS-supplied
/// PowerShell executable and a fixed, parameter-free query are used.
#[cfg(windows)]
pub fn codex_windows_packages() -> Result<Vec<u8>, ProviderError> {
    windows_packages(
        "$env:PSModulePath=Join-Path $PSHOME Modules; [Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-AppxPackage -Name OpenAI.Codex* | Select-Object Name,PublisherId,InstallLocation,PackageFamilyName,@{n='Architecture';e={$_.Architecture.ToString()}},@{n='Version';e={$_.Version.ToString()}} | ConvertTo-Json -Compress",
    )
}

#[cfg(windows)]
pub fn claude_windows_packages() -> Result<Vec<u8>, ProviderError> {
    windows_packages(
        "$env:PSModulePath=Join-Path $PSHOME Modules; [Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-AppxPackage -Name Claude | Select-Object Name,InstallLocation,PackageFamilyName,@{n='SignatureKind';e={$_.SignatureKind.ToString()}},@{n='Version';e={$_.Version.ToString()}} | ConvertTo-Json -Compress",
    )
}

#[cfg(windows)]
fn windows_packages(script: &'static str) -> Result<Vec<u8>, ProviderError> {
    let root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or_else(|| {
            ProviderError::CapabilityUnavailable("Windows system root unavailable".into())
        })?;
    let mut command = Command::new(root.join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    small_output(command)
}
