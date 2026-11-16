use std::{env, path::PathBuf, process::ExitCode};

use agentmender_domain::{ProviderKind, canonical_uuid};
use agentmender_provider_api::{ProviderLocation, process};
use agentmender_provider_claude::{ClaudeAdapter, DesktopContext};
use agentmender_provider_codex::CodexAdapter;
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
    /// Inspect installations under this Claude Desktop data root.
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_desktop_root: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_app_bundle: Vec<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    codex_executable: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_executable: Option<PathBuf>,
    /// Protect this Codex UUID in the configured root. May be supplied multiple times.
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

fn claude_adapter(
    args: &Args,
    location: ProviderLocation,
    home: &std::path::Path,
) -> Result<ClaudeAdapter, String> {
    let default_desktop = cfg!(target_os = "macos")
        && args.claude_root.is_none()
        && env::var_os("CLAUDE_CONFIG_DIR").is_none();
    let default_root = home.join("Library/Application Support/Claude");
    let desktop = args
        .claude_desktop_root
        .clone()
        .or_else(|| default_desktop.then(|| default_root.clone()));
    let adapter = ClaudeAdapter::new(location);
    let Some(data_root) = desktop else {
        return if args.claude_app_bundle.is_empty() {
            Ok(adapter)
        } else {
            Err("--claude-app-bundle requires a Desktop root".into())
        };
    };
    let mut app_bundles = args.claude_app_bundle.clone();
    if default_desktop && data_root == default_root {
        app_bundles.push(PathBuf::from("/Applications/Claude.app"));
        app_bundles.push(home.join("Applications/Claude.app"));
    }
    Ok(adapter.with_desktop_context(DesktopContext {
        data_root,
        app_bundles,
    }))
}

fn run(args: Args) -> Result<(), String> {
    let Action::Ui { no_open } = args.command;
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .filter(|path| path.is_absolute())
        .ok_or("an absolute home directory is required")?;
    let helper = env::current_exe().map_err(|_| "could not resolve the reader executable")?;
    let codex_location = ProviderLocation {
        state_root: root(args.codex_root.as_ref(), "CODEX_HOME", ".codex", &home)?,
        executable: args
            .codex_executable
            .clone()
            .or_else(|| process::find_executable(ProviderKind::Codex))
            .or_else(process::find_codex_desktop_executable),
    };
    let claude_location = ProviderLocation {
        state_root: root(
            args.claude_root.as_ref(),
            "CLAUDE_CONFIG_DIR",
            ".claude",
            &home,
        )?,
        executable: args
            .claude_executable
            .clone()
            .or_else(|| process::find_executable(ProviderKind::Claude)),
    };
    let native = agentmender_provider_codex::NativeRemoval::detect(
        codex_location.executable.as_deref(),
        &codex_location.state_root,
        args.protect_codex_session.clone(),
    );
    let codex = CodexAdapter::new(codex_location.clone(), helper, home.clone());
    let claude = claude_adapter(&args, claude_location.clone(), &home)?;
    ui::run(ui::Configuration {
        codex_root: codex_location.state_root,
        claude_root: claude_location.state_root,
        codex,
        claude,
        native,
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
