use std::{env, path::PathBuf, process::ExitCode};

use agentmender_application::{ApplicationError, DiscoveryService, InspectedEntity};
use agentmender_domain::{
    CompatibilityStatus, Completeness, DiscoveryScope, EntityId, Presence, ProviderKind,
    observed_path_is_absolute,
};
use agentmender_provider_api::{
    ProviderAdapter, ProviderError, ProviderLocation, process::find_executable,
};
use agentmender_provider_claude::ClaudeAdapter;
use agentmender_provider_codex::CodexAdapter;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

#[derive(Parser)]
#[command(version, about = "Read-only inventory of AI agent state")]
struct Args {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, value_enum)]
    provider: Option<SelectedProvider>,
    /// Use a provider fixture directory instead of a live provider.
    #[arg(long, global = true, requires = "provider")]
    fixture: Option<PathBuf>,
    #[arg(long, global = true)]
    codex_root: Option<PathBuf>,
    #[arg(long, global = true)]
    claude_root: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    codex_executable: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_executable: Option<PathBuf>,
    #[arg(long, global = true, value_parser = observed_project_path)]
    project: Option<PathBuf>,
    #[arg(long, global = true, default_value_t = 10_000, value_parser = entry_limit)]
    max_entries: usize,
    #[arg(long, global = true)]
    active_only: bool,
    #[command(subcommand)]
    command: Action,
}
#[derive(Clone, Copy, ValueEnum)]
enum SelectedProvider {
    Codex,
    Claude,
}
#[derive(Subcommand)]
enum Action {
    Providers,
    Scan,
    Inspect { id: String },
}

fn entry_limit(value: &str) -> Result<usize, String> {
    let count: usize = value
        .parse()
        .map_err(|_| "max-entries must be an integer from 1 to 100000")?;
    if (1..=100_000).contains(&count) {
        Ok(count)
    } else {
        Err("max-entries must be from 1 to 100000".into())
    }
}
fn absolute_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("path must be absolute".into())
    }
}
fn observed_project_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if observed_path_is_absolute(&path) {
        Ok(path)
    } else {
        Err("project scope must be an absolute provider path".into())
    }
}
fn root(explicit: Option<PathBuf>, variable: &str, suffix: &str) -> Result<PathBuf, CliError> {
    let path = explicit
        .or_else(|| env::var_os(variable).map(PathBuf::from))
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(suffix)))
        .ok_or_else(|| {
            CliError::new(
                ErrorCode::InvalidConfiguration,
                format!("{variable} and HOME are unset; supply an explicit root"),
            )
        })?;
    if !path.is_absolute() {
        return Err(CliError::new(
            ErrorCode::InvalidConfiguration,
            "provider state root must be absolute",
        ));
    }
    Ok(path)
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    InvalidArguments,
    InvalidConfiguration,
    InvalidIdentity,
    ProviderNotInstalled,
    ProviderVersionUnknown,
    ProviderUnsupported,
    CapabilityUnavailable,
    MalformedProviderState,
    PermissionDenied,
    DiscoveryFailed,
    InvalidScope,
    UnknownEntity,
    AmbiguousEntity,
    InspectionUnavailable,
    OutputError,
}
#[derive(Serialize)]
struct CliError {
    code: ErrorCode,
    message: String,
}
impl CliError {
    fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl From<ProviderError> for CliError {
    fn from(error: ProviderError) -> Self {
        let code = match &error {
            ProviderError::ProviderNotInstalled(_) => ErrorCode::ProviderNotInstalled,
            ProviderError::ProviderVersionUnknown(_) => ErrorCode::ProviderVersionUnknown,
            ProviderError::ProviderUnsupported(_) => ErrorCode::ProviderUnsupported,
            ProviderError::CapabilityUnavailable(_) => ErrorCode::CapabilityUnavailable,
            ProviderError::MalformedProviderState { .. } => ErrorCode::MalformedProviderState,
            ProviderError::PermissionDenied(_) => ErrorCode::PermissionDenied,
            ProviderError::DiscoveryFailed { .. } => ErrorCode::DiscoveryFailed,
            ProviderError::InvalidScope(_) => ErrorCode::InvalidScope,
        };
        Self::new(code, error.to_string())
    }
}
impl From<ApplicationError> for CliError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::Provider(error) => error.into(),
            ApplicationError::InspectionUnavailable(id) => Self::new(
                ErrorCode::InspectionUnavailable,
                format!("inspection unavailable: {id}"),
            ),
            ApplicationError::UnknownEntity(id) => {
                Self::new(ErrorCode::UnknownEntity, format!("unknown entity: {id}"))
            }
            ApplicationError::AmbiguousEntity(id) => Self::new(
                ErrorCode::AmbiguousEntity,
                format!("ambiguous entity: {id}"),
            ),
        }
    }
}
#[derive(Serialize)]
struct Report<T: Serialize> {
    mode: &'static str,
    report: T,
}
fn emit<T: Serialize>(mode: &'static str, report: T) -> Result<(), CliError> {
    println!(
        "{}",
        serde_json::to_string_pretty(&Report { mode, report })
            .map_err(|e| CliError::new(ErrorCode::OutputError, e.to_string()))?
    );
    Ok(())
}
fn run(args: Args) -> Result<bool, CliError> {
    let mode = if args.fixture.is_some() {
        "fixture"
    } else {
        "live"
    };
    let mut adapters: Vec<Box<dyn ProviderAdapter>> = Vec::new();
    if !matches!(args.provider, Some(SelectedProvider::Claude)) {
        let adapter = if let Some(path) = &args.fixture {
            CodexAdapter::from_fixture(path.clone())?
        } else {
            CodexAdapter::new(ProviderLocation {
                state_root: root(args.codex_root, "CODEX_HOME", ".codex")?,
                executable: args
                    .codex_executable
                    .or_else(|| find_executable(ProviderKind::Codex)),
            })
        };
        adapters.push(Box::new(adapter));
    }
    if !matches!(args.provider, Some(SelectedProvider::Codex)) {
        let adapter = if let Some(path) = &args.fixture {
            ClaudeAdapter::from_fixture(path.clone())?
        } else {
            ClaudeAdapter::new(ProviderLocation {
                state_root: root(args.claude_root, "CLAUDE_CONFIG_DIR", ".claude")?,
                executable: args
                    .claude_executable
                    .or_else(|| find_executable(ProviderKind::Claude)),
            })
        };
        adapters.push(Box::new(adapter));
    }
    let service = DiscoveryService::new(adapters.iter().map(|a| a.as_ref()).collect());
    let scope = DiscoveryScope {
        project_path: args.project,
        include_archived: !args.active_only,
        max_entries: args.max_entries,
    };
    match args.command {
        Action::Providers => {
            let report = service.providers();
            let incomplete = !report.failures.is_empty()
                || report.providers.iter().any(|p| {
                    p.presence == Presence::Missing
                        || p.compatibility.status == CompatibilityStatus::AuditOnlyUnknown
                });
            if args.json {
                emit(mode, &report)?;
            } else {
                println!("Mode: {mode}");
                for p in &report.providers {
                    println!(
                        "{}: {:?}, {:?} ({})",
                        p.kind,
                        p.presence,
                        p.compatibility.status,
                        p.state_root.display()
                    );
                }
                for f in &report.failures {
                    eprintln!("{}: {:?}: {}", f.provider, f.code, f.message);
                }
            }
            Ok(incomplete)
        }
        Action::Scan => {
            let report = service.scan(&scope)?;
            let incomplete = !report.failures.is_empty()
                || report
                    .snapshots
                    .iter()
                    .any(|s| s.completeness != Completeness::Complete);
            if args.json {
                emit(mode, &report)?;
            } else {
                println!("Mode: {mode}");
                for s in &report.snapshots {
                    println!("{}: {:?} — {}", s.provider.kind, s.completeness, s.coverage);
                    println!(
                        "  {} workspaces, {} projects, {} sessions, {} artifacts",
                        s.workspaces.len(),
                        s.projects.len(),
                        s.sessions.len(),
                        s.artifacts.len()
                    );
                    for session in &s.sessions {
                        println!("  session {} ({:?})", session.id, session.state);
                    }
                    for d in &s.diagnostics {
                        eprintln!("  {:?}: {}", d.code, d.message);
                    }
                }
                for f in &report.failures {
                    eprintln!("{}: {:?}: {}", f.provider, f.code, f.message);
                }
            }
            Ok(incomplete)
        }
        Action::Inspect { id } => {
            let id = EntityId::new(id)
                .map_err(|e| CliError::new(ErrorCode::InvalidIdentity, e.to_string()))?;
            let report = service.inspect(&id, &scope)?;
            let incomplete =
                report.completeness != Completeness::Complete || !report.failures.is_empty();
            if args.json {
                emit(mode, &report)?;
            } else {
                let kind = match &report.entity {
                    InspectedEntity::Workspace(_) => "workspace",
                    InspectedEntity::Project(_) => "project",
                    InspectedEntity::Session(_) => "session",
                    InspectedEntity::Artifact(_) => "artifact",
                };
                println!(
                    "Mode: {mode}\n{} {kind} {id} ({:?})",
                    report.provider, report.completeness
                );
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report.entity)
                        .map_err(|e| CliError::new(ErrorCode::OutputError, e.to_string()))?
                );
                for failure in &report.failures {
                    eprintln!(
                        "{}: {:?}: {}",
                        failure.provider, failure.code, failure.message
                    );
                }
                for d in &report.diagnostics {
                    eprintln!("  {:?}: {}", d.code, d.message);
                }
            }
            Ok(incomplete)
        }
    }
}
fn main() -> ExitCode {
    let json_requested = env::args_os().any(|arg| arg == "--json");
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return ExitCode::SUCCESS;
            }
            if json_requested {
                eprintln!(
                    "{}",
                    serde_json::to_string(&CliError::new(
                        ErrorCode::InvalidArguments,
                        error.to_string()
                    ))
                    .expect("serializable error")
                );
            } else {
                let _ = error.print();
            }
            return ExitCode::from(2);
        }
    };
    let json = args.json;
    match run(args) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(2),
        Err(error) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::to_string(&error).expect("serializable error")
                );
            } else {
                eprintln!("error: {}", error.message);
            }
            ExitCode::from(2)
        }
    }
}
