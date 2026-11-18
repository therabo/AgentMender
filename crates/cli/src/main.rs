use std::{env, path::PathBuf, process::ExitCode};

use agentmender_domain::{ProviderKind, canonical_uuid};
use agentmender_provider_api::process;
use agentmender_provider_claude::{ClaudeAdapter, DesktopContext, InstallationView, RootSource};
use agentmender_provider_codex::{CodexAdapter, discovery::Discovery};
use clap::{Parser, Subcommand};

mod ui;

#[derive(Parser)]
#[command(version, about = "Local session manager for Codex and Claude Code")]
struct Args {
    /// Override CODEX_HOME or ~/.codex.
    #[arg(long, global = true, value_parser = absolute_path)]
    codex_root: Option<PathBuf>,
    /// Override CLAUDE_CONFIG_DIR or ~/.claude.
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_root: Option<PathBuf>,
    /// Also inspect a separate Claude Code configuration root.
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_additional_root: Vec<PathBuf>,
    /// Inspect installations under this Claude Desktop data root.
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_desktop_root: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_app_bundle: Vec<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    codex_executable: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_executable: Option<PathBuf>,
    /// Block direct deletion of this Codex UUID. May be supplied multiple times.
    #[arg(long, global = true, value_parser = protected_uuid)]
    protect_codex_session: Vec<String>,
    #[arg(long, global = true, default_value_t = 10_000, value_parser = clap::value_parser!(u32).range(1..=100_000))]
    max_entries: u32,
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Open the local session manager in the default browser.
    Ui {
        /// Print the launch URL for manual opening.
        #[arg(long)]
        no_open: bool,
    },
}

fn absolute_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    path.is_absolute()
        .then_some(path)
        .ok_or_else(|| "path must be absolute".into())
}

fn protected_uuid(value: &str) -> Result<String, String> {
    if canonical_uuid(value) {
        Ok(value.into())
    } else {
        Err("expected a canonical lowercase Codex UUID".into())
    }
}

fn root(
    explicit: Option<&PathBuf>,
    variable: &str,
    suffix: &str,
    home: &std::path::Path,
) -> Result<PathBuf, String> {
    let path = explicit
        .cloned()
        .or_else(|| env::var_os(variable).map(PathBuf::from))
        .unwrap_or_else(|| home.join(suffix));
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(format!(
            "provider root must be absolute; specify it at launch or set {variable}"
        ))
    }
}

fn claude_adapter(args: &Args, home: &std::path::Path) -> Result<ClaudeAdapter, String> {
    let config = root(
        args.claude_root.as_ref(),
        "CLAUDE_CONFIG_DIR",
        ".claude",
        home,
    )?;
    let source = if args.claude_root.is_some() {
        RootSource::Explicit
    } else if env::var_os("CLAUDE_CONFIG_DIR").is_some() {
        RootSource::Environment
    } else {
        RootSource::Default
    };
    let mut roots = vec![(config.clone(), source)];
    if source == RootSource::Environment && config != home.join(".claude") {
        roots.push((home.join(".claude"), RootSource::Default));
    }
    roots.extend(
        args.claude_additional_root
            .iter()
            .cloned()
            .map(|path| (path, RootSource::Additional)),
    );

    let mut cli_candidates = args.claude_executable.iter().cloned().collect::<Vec<_>>();
    if cli_candidates.is_empty() && source != RootSource::Explicit {
        cli_candidates = process::find_executables(ProviderKind::Claude);
        cli_candidates.push(home.join(".local/bin").join(if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        }));
    }
    let (mut desktop, installations) = if source == RootSource::Explicit {
        (Vec::new(), Vec::new())
    } else {
        native_claude_desktop(home)
    };
    if let Some(data_root) = &args.claude_desktop_root {
        desktop.retain(|item| &item.data_root != data_root);
        desktop.insert(
            0,
            DesktopContext {
                data_root: data_root.clone(),
                app_bundles: args.claude_app_bundle.clone(),
            },
        );
    } else if !args.claude_app_bundle.is_empty() {
        desktop
            .first_mut()
            .ok_or("--claude-app-bundle requires a Desktop root")?
            .app_bundles
            .extend(args.claude_app_bundle.iter().cloned());
    }
    ClaudeAdapter::new(roots, desktop, cli_candidates, installations).map_err(|err| err.to_string())
}

#[cfg(not(windows))]
fn native_claude_desktop(home: &std::path::Path) -> (Vec<DesktopContext>, Vec<InstallationView>) {
    let desktop = cfg!(target_os = "macos").then(|| DesktopContext {
        data_root: home.join("Library/Application Support/Claude"),
        app_bundles: vec![
            PathBuf::from("/Applications/Claude.app"),
            home.join("Applications/Claude.app"),
        ],
    });
    (desktop.into_iter().collect(), Vec::new())
}

#[cfg(windows)]
fn native_claude_desktop(home: &std::path::Path) -> (Vec<DesktopContext>, Vec<InstallationView>) {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Package {
        name: String,
        install_location: PathBuf,
        package_family_name: String,
        signature_kind: String,
        version: String,
    }
    let absolute_env = |key| {
        env::var_os(key)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let roaming = absolute_env("APPDATA").unwrap_or_else(|| home.join("AppData/Roaming"));
    let local = absolute_env("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData/Local"));
    let mut contexts = vec![DesktopContext {
        data_root: roaming.join("Claude"),
        app_bundles: Vec::new(),
    }];
    let mut installations = Vec::new();
    if let Ok(bytes) = process::claude_windows_packages() {
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
        let packages = serde_json::from_slice::<Vec<Package>>(bytes)
            .or_else(|_| serde_json::from_slice::<Package>(bytes).map(|item| vec![item]))
            .unwrap_or_default();
        for package in packages.into_iter().take(4) {
            if package.name == "Claude"
                && package.package_family_name == "Claude_pzs8sxrjxfjjc"
                && package.install_location.is_absolute()
                && matches!(
                    package.signature_kind.as_str(),
                    "Store" | "Developer" | "Enterprise"
                )
            {
                installations.push(InstallationView {
                    path: package.install_location.join("app/Claude.exe"),
                    source: if package.signature_kind == "Store" {
                        "desktop_store"
                    } else {
                        "desktop_msix"
                    },
                    version: Some(package.version),
                    confirmed: true,
                });
                contexts.push(DesktopContext {
                    data_root: local
                        .join("Packages")
                        .join(package.package_family_name)
                        .join("LocalCache/Roaming/Claude"),
                    app_bundles: Vec::new(),
                });
            }
        }
    }
    // A conventional Win32 install is a candidate. Package evidence is stronger.
    let win32 = local.join("AnthropicClaude");
    if let Ok(entries) = std::fs::read_dir(win32) {
        for entry in entries.take(32).flatten() {
            let Some(version) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("app-"))
                .filter(|version| {
                    version.len() < 32 && version.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                })
                .map(str::to_owned)
            else {
                continue;
            };
            let path = entry.path().join("Claude.exe");
            if path.is_file() {
                installations.push(InstallationView {
                    path,
                    source: "desktop_win32_candidate",
                    version: Some(version),
                    confirmed: false,
                });
            }
        }
    }
    (contexts, installations)
}

fn run(args: Args) -> Result<(), String> {
    let Action::Ui { no_open } = args.command;
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .filter(|path| path.is_absolute())
        .ok_or("an absolute home directory is required")?;
    let helper = env::current_exe().map_err(|_| "could not resolve the reader executable")?;
    let discovery = Discovery::detect(
        &home,
        args.codex_root.clone(),
        args.codex_executable.clone(),
    )?;
    let codex_location = discovery.location.clone();
    let codex = CodexAdapter::new(codex_location.clone(), helper, home.clone());
    let claude = claude_adapter(&args, &home)?;
    ui::run(ui::Configuration {
        codex_root: codex_location.state_root,
        codex,
        discovery,
        claude,
        limit: args.max_entries as usize,
        protected: args.protect_codex_session,
        no_open,
    })
}

fn main() -> ExitCode {
    let mut raw_args = env::args_os();
    raw_args.next();
    if raw_args.next().as_deref() == Some(std::ffi::OsStr::new("--internal-codex-content-capture"))
        && raw_args.next().is_none()
    {
        return match agentmender_provider_codex::run_worker() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => {
                eprintln!("internal Codex reader refused the request");
                ExitCode::FAILURE
            }
        };
    }
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
