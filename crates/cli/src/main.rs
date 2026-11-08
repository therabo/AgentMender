use std::{collections::BTreeMap, env, path::PathBuf, process::ExitCode, sync::Arc};

use agentmender_application::{
    ApplicationError, DiscoveryService, InspectedEntity, QueryControl, ReadOnlyRequest,
    ReadOnlyResponse,
};
use agentmender_domain::{
    ArtifactId, CompatibilityStatus, Completeness, DiscoveryScope, EntityId, IntegrityReport,
    LifecycleIntent, LifecyclePlan, PlanStatus, Presence, Provider, ProviderKind,
    VerificationStatus, observed_path_is_absolute,
};
use agentmender_provider_api::{
    ProviderAdapter, ProviderError, ProviderLocation, process::find_executable,
};
use agentmender_provider_claude::{ClaudeAdapter, DesktopContext};
use agentmender_provider_codex::CodexAdapter;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

mod approvals;
mod conversation;
mod lifecycle;
mod transactions;

#[derive(Clone, Parser)]
#[command(
    version,
    about = "Provider-aware inventory and approved lifecycle management"
)]
struct Args {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, value_enum)]
    provider: Option<SelectedProvider>,
    /// Use a provider fixture directory instead of a live provider.
    #[arg(long, global = true, requires = "provider")]
    fixture: Option<PathBuf>,
    /// Interpret the Claude fixture as an observed Desktop fixture.
    #[arg(long, global = true, requires = "fixture")]
    desktop_fixture: bool,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_desktop_root: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    claude_app_bundle: Vec<PathBuf>,
    #[arg(long, global = true)]
    codex_root: Option<PathBuf>,
    /// Select the exact closed Codex Desktop cleanup profile at this Electron data root.
    #[arg(long, global = true, value_parser = absolute_path)]
    codex_desktop_user_data: Option<PathBuf>,
    #[arg(long, global = true)]
    claude_root: Option<PathBuf>,
    #[arg(long, global = true, value_parser = absolute_path)]
    codex_executable: Option<PathBuf>,
    /// Independently registered management identities in the selected Codex root.
    #[arg(long, global = true, value_parser = conversation::canonical_uuid)]
    protect_codex_session: Vec<String>,
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
#[derive(Clone, Copy, ValueEnum)]
enum PlanIntent {
    Archive,
    Unarchive,
    SoftDelete,
    Purge,
    Repair,
    CleanupSession,
}
impl From<PlanIntent> for LifecycleIntent {
    fn from(value: PlanIntent) -> Self {
        match value {
            PlanIntent::Archive => Self::Archive,
            PlanIntent::Unarchive => Self::Unarchive,
            PlanIntent::SoftDelete => Self::SoftDelete,
            PlanIntent::Purge => Self::Purge,
            PlanIntent::Repair => Self::Repair,
            PlanIntent::CleanupSession => Self::CleanupSession,
        }
    }
}
#[derive(Clone, Subcommand)]
enum Action {
    Providers,
    Mcp {
        /// Opt in to real-session metadata search and non-authorizing assessment.
        #[arg(long, conflicts_with = "session_removal")]
        session_search: bool,
        /// Opt in to metadata disclosure and durable primary-removal preparation.
        #[arg(long)]
        session_removal: bool,
        #[arg(long, value_parser = absolute_path)]
        state_root: Option<PathBuf>,
    },
    Plans {
        #[command(subcommand)]
        action: approvals::PlanAction,
    },
    Approve {
        plan_id: String,
        #[arg(long)]
        transaction_id: String,
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
    Approvals {
        #[command(subcommand)]
        action: approvals::ApprovalAction,
    },
    Scan,
    Inspect {
        id: String,
    },
    Graph,
    Verify,
    Plan {
        intent: PlanIntent,
        id: String,
    },
    Explain {
        id: String,
    },
    /// Inspect existing AgentMender-owned durable knowledge without provider access.
    Transactions {
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
        #[command(subcommand)]
        action: transactions::TransactionAction,
    },
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
    InvalidGraph,
    UnknownEntity,
    AmbiguousEntity,
    InspectionUnavailable,
    OutputError,
    TransactionUnavailable,
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
            ProviderError::Graph(_) => ErrorCode::InvalidGraph,
        };
        Self::new(code, error.to_string())
    }
}
impl From<ApplicationError> for CliError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::ManagementProtectionUnavailable
            | ApplicationError::ProtectedManagementSession
            | ApplicationError::RemovalPlanChanged
            | ApplicationError::ReadOnlyRemovalUnavailable => {
                Self::new(ErrorCode::InvalidScope, error.to_string())
            }
            ApplicationError::Provider(error) => error.into(),
            ApplicationError::Cancelled => Self::new(ErrorCode::DiscoveryFailed, "query cancelled"),
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
fn print_provider(p: &Provider) {
    println!(
        "{}: {:?} (state: {})",
        p.kind,
        p.presence,
        p.state_root.display()
    );
    println!(
        "  compatibility: {:?} — {}",
        p.compatibility.status, p.compatibility.reason
    );
    println!("  evidence: {:?}", p.evidence_status);
    println!(
        "  selected executable: {} (version: {})",
        p.fingerprint
            .executable
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".into()),
        p.fingerprint
            .executable_version
            .as_deref()
            .unwrap_or("unknown")
    );
    for installation in &p.installations {
        println!(
            "  {:?}: {} (version: {}, source: {:?})",
            installation.surface,
            installation.location.display(),
            installation.version.as_deref().unwrap_or("unknown"),
            installation.version_source
        );
    }
    for d in &p.diagnostics {
        eprintln!("  {:?}: {}", d.code, d.message);
    }
}
fn print_id(id: &ArtifactId) -> String {
    format!("{:?} {}", id.class, id.local_id)
}

fn print_verification(r: &IntegrityReport) {
    const LIMIT: usize = 10;
    let provider = &r.context.provider;
    println!(
        "{}: {:?} (state: {})",
        provider.kind,
        r.verification_status,
        provider.state_root.display()
    );
    println!("  verification: {:?}", r.verification_status);
    println!(
        "  compatibility: {:?} — {}",
        provider.compatibility.status, provider.compatibility.reason
    );
    println!(
        "  evidence: {:?}; discovery evidence: {:?}",
        provider.evidence_status, r.context.evidence_status
    );
    println!(
        "  declared discovery boundary: {:?} — {}",
        r.context.completeness, r.context.coverage
    );
    match &r.proof_context.contract {
        Some(contract) => println!(
            "  proof contract: {} (provider {}, adapter {}, coherent: {})",
            contract.id, contract.provider_version, contract.adapter_version, contract.coherent
        ),
        None => println!("  proof contract: absent"),
    }
    println!(
        "  declared proof coverage: {} node namespaces, {} relationship namespaces; {} required targets, {} required owners",
        r.proof_context.node_coverage.len(),
        r.proof_context.relationship_coverage.len(),
        r.proof_context.required_targets.len(),
        r.proof_context.required_owners.len()
    );
    println!("  findings: {}", r.findings.len());
    for finding in r.findings.iter().take(LIMIT) {
        println!(
            "    {:?}: {:?}, {:?} — {}",
            finding.kind, finding.severity, finding.disposition, finding.explanation
        );
        for id in finding.affected.iter().take(LIMIT) {
            println!("      affected: {}", print_id(id));
        }
        if finding.affected.len() > LIMIT {
            println!(
                "      ... {} more affected entities",
                finding.affected.len() - LIMIT
            );
        }
        for evidence in finding.evidence.iter().take(LIMIT) {
            let e = &evidence.evidence;
            let description = r
                .rules
                .iter()
                .find(|rule| rule.id == e.rule_id)
                .map(|rule| rule.description.as_str())
                .unwrap_or("rule description unavailable");
            println!(
                "      rule {} ({description}); source {:?}; confidence {:?} raw / {:?} effective",
                e.rule_id, e.observation.source, e.observation.confidence, evidence.confidence
            );
        }
        if finding.evidence.len() > LIMIT {
            println!(
                "      ... {} more evidence records",
                finding.evidence.len() - LIMIT
            );
        }
    }
    if r.findings.len() > LIMIT {
        println!("    ... {} more findings", r.findings.len() - LIMIT);
    }
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for blocker in &r.blockers {
        let group = groups.entry(format!("{:?}", blocker.code)).or_default();
        group.0 += 1;
        for sample in blocker.affected.iter().map(print_id).chain(
            blocker
                .evidence
                .iter()
                .filter_map(|e| e.evidence.locator.as_ref().map(|p| p.display().to_string())),
        ) {
            if group.1.len() < 3 && !group.1.contains(&sample) {
                group.1.push(sample);
            }
        }
    }
    println!("  blockers: {}", r.blockers.len());
    for (code, (count, samples)) in groups {
        println!(
            "    {code}: {count}{}",
            if samples.is_empty() {
                String::new()
            } else {
                format!(" (examples: {})", samples.join(", "))
            }
        );
    }
    println!("  residuals: {}", r.residuals.len());
    for residual in r.residuals.iter().take(LIMIT) {
        println!(
            "    {:?}: {} — {}",
            residual.classification,
            print_id(&residual.artifact),
            residual.explanation
        );
    }
    if r.residuals.len() > LIMIT {
        println!("    ... {} more residuals", r.residuals.len() - LIMIT);
    }
    println!(
        "  diagnostics: {} graph, {} discovery, {} provider",
        r.graph_diagnostics.len(),
        r.context.diagnostics.len(),
        provider.diagnostics.len()
    );
    println!("  Use --json for the complete report and evidence.");
}
fn print_desktop_plan_status(plan: &LifecyclePlan) {
    let content = plan.content();
    let Some(manifest) = &content.complete_removal_manifest else {
        return;
    };
    let Some(desktop) = &manifest.desktop else {
        return;
    };
    println!(
        "Desktop session: {:?} (ID: {}), build {}, profile {}",
        desktop.target_title,
        manifest.primary.native_session_id,
        desktop.bundle_build,
        manifest.profile.identity
    );
    if content
        .blockers
        .contains(&agentmender_domain::PlanBlocker::RequiresProviderClosed)
    {
        println!("Codex must remain closed throughout any admitted cleanup.");
    }
    if manifest.profile.admission == agentmender_domain::ProfileAdmission::DesktopUnaccepted {
        println!("Structural implementation: Candidate. Desktop execution is disabled.");
    }
    if desktop.requires_shared_global_publication() {
        println!(
            "Shared global-state publication requires compatible writer coordination. Closing Codex alone cannot resolve this blocker."
        );
    }
    println!("Graphical acceptance: Not tested. Verification is manual.");
}

fn print_plan(plan: &LifecyclePlan) {
    let c = plan.content();
    println!(
        "Target: {} ({:?}, {})",
        print_id(&c.request.target),
        c.request.target.provider,
        c.request.target.state_root.display()
    );
    println!("Intent: {:?}; status: {:?}", c.request.intent, c.status);
    print_desktop_plan_status(plan);
    println!("Digest: {}", plan.digest().as_str());
    println!("State fingerprint: {}", c.state_fingerprint.as_str());
    println!(
        "Provider version: {}; adapter: {}",
        c.provider
            .executable_version
            .as_deref()
            .unwrap_or("unknown"),
        c.provider.adapter_version
    );
    println!("Native identity: {:?}", c.native_target_id);
    println!("Native capability: {:?}", c.capability);
    println!(
        "Observation scope: {:?}; compatibility: {:?}; evidence: {:?}",
        c.observation_scope, c.compatibility_status, c.evidence_status
    );
    println!("Integrity: {:?}", c.integrity);
    println!("Effects contract: {:?}", c.effect_contract);
    for artifact in &c.artifacts {
        println!(
            "  {:?}: {} ownership {:?}",
            artifact.role,
            print_id(&artifact.id),
            artifact.ownership
        );
    }
    println!("Blockers: {:?}", c.blockers);
    println!("Preconditions: {:?}", c.preconditions);
    println!("Postconditions: {:?}", c.postconditions);
    println!("Space: {:?}", c.space);
    println!(
        "Human approval required: {}; recovery required: {}",
        c.requires_human_approval, c.recovery_required
    );
    println!("Execution: NOT PERFORMED; plan is not authorization; Ready is not approved");
}
struct FactoryEnvironment {
    home: Option<PathBuf>,
    helper: Option<PathBuf>,
}

fn adapters(
    args: &Args,
    frozen: Option<&FactoryEnvironment>,
) -> Result<Vec<Box<dyn ProviderAdapter>>, CliError> {
    let mut adapters: Vec<Box<dyn ProviderAdapter>> = Vec::new();
    if !matches!(args.provider, Some(SelectedProvider::Claude)) {
        let adapter = if let Some(path) = &args.fixture {
            CodexAdapter::from_fixture(path.clone())?
        } else {
            let mut adapter = CodexAdapter::new(ProviderLocation {
                state_root: root(args.codex_root.clone(), "CODEX_HOME", ".codex")?,
                executable: args.codex_executable.clone().or_else(|| {
                    if frozen.is_none() {
                        find_executable(ProviderKind::Codex)
                    } else {
                        None
                    }
                }),
            });
            if let (Some(executable), Some(home)) = (
                frozen.map_or_else(|| env::current_exe().ok(), |f| f.helper.clone()),
                frozen.map_or_else(
                    || env::var_os("HOME").map(PathBuf::from),
                    |f| f.home.clone(),
                ),
            ) && home.is_absolute()
            {
                adapter = adapter.with_evidence_helper(executable, home);
            }
            adapter
        };
        adapters.push(Box::new(adapter));
    }
    if !matches!(args.provider, Some(SelectedProvider::Codex)) {
        let adapter = if let Some(path) = &args.fixture {
            if args.desktop_fixture {
                ClaudeAdapter::from_desktop_fixture(path.clone())?
            } else {
                ClaudeAdapter::from_fixture(path.clone())?
            }
        } else {
            let default_desktop = cfg!(target_os = "macos")
                && args.claude_root.is_none()
                && env::var_os("CLAUDE_CONFIG_DIR").is_none();
            let home = frozen.map_or_else(
                || env::var_os("HOME").map(PathBuf::from),
                |f| f.home.clone(),
            );
            let default_root = home
                .as_ref()
                .filter(|p| p.is_absolute())
                .map(|p| p.join("Library/Application Support/Claude"));
            let desktop_root = args.claude_desktop_root.clone().or_else(|| {
                if default_desktop {
                    default_root.clone()
                } else {
                    None
                }
            });
            let mut adapter = ClaudeAdapter::new(ProviderLocation {
                state_root: root(args.claude_root.clone(), "CLAUDE_CONFIG_DIR", ".claude")?,
                executable: args.claude_executable.clone().or_else(|| {
                    if frozen.is_none() {
                        find_executable(ProviderKind::Claude)
                    } else {
                        None
                    }
                }),
            });
            if let Some(data_root) = desktop_root {
                let mut app_bundles = args.claude_app_bundle.clone();
                if default_desktop && default_root.as_ref() == Some(&data_root) {
                    app_bundles.push(PathBuf::from("/Applications/Claude.app"));
                    if let Some(home) = home {
                        app_bundles.push(home.join("Applications/Claude.app"));
                    }
                }
                adapter = adapter.with_desktop_context(DesktopContext {
                    data_root,
                    app_bundles,
                });
            } else if !args.claude_app_bundle.is_empty() {
                return Err(CliError::new(
                    ErrorCode::InvalidArguments,
                    "--claude-app-bundle requires a desktop root",
                ));
            }
            adapter
        };
        adapters.push(Box::new(adapter));
    }
    Ok(adapters)
}
fn run(args: Args) -> Result<bool, CliError> {
    if let Action::Transactions { state_root, action } = &args.command {
        if matches!(
            action,
            transactions::TransactionAction::Execute { .. }
                | transactions::TransactionAction::Reconcile { .. }
        ) {
            return lifecycle::run(&args, state_root, action);
        }
        return transactions::run(&args, state_root, action);
    }
    if let Action::Approvals { action } = &args.command {
        return approvals::administration(&args, action);
    }
    if let Action::Plans {
        action: action @ approvals::PlanAction::Inspect { .. },
    } = &args.command
    {
        return approvals::inspect_plan(&args, action);
    }
    if args.desktop_fixture && !matches!(args.provider, Some(SelectedProvider::Claude)) {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "--desktop-fixture requires --provider claude",
        ));
    }
    if let Action::Mcp {
        session_search,
        session_removal,
        state_root,
    } = &args.command
    {
        if *session_removal && (args.codex_root.is_none() || args.codex_executable.is_none()) {
            return Err(CliError::new(
                ErrorCode::InvalidConfiguration,
                "session-removal requires explicit --codex-root and --codex-executable",
            ));
        }
        let mut config = args.clone();
        if config.fixture.is_none() {
            if !matches!(config.provider, Some(SelectedProvider::Claude)) {
                config.codex_root = Some(root(config.codex_root.clone(), "CODEX_HOME", ".codex")?);
                config.codex_executable = config
                    .codex_executable
                    .or_else(|| find_executable(ProviderKind::Codex))
                    .or_else(|| {
                        if *session_search {
                            agentmender_provider_api::process::find_codex_desktop_executable()
                        } else {
                            None
                        }
                    });
            }
            if !matches!(config.provider, Some(SelectedProvider::Codex)) {
                let default_desktop = cfg!(target_os = "macos")
                    && config.claude_root.is_none()
                    && env::var_os("CLAUDE_CONFIG_DIR").is_none();
                if default_desktop
                    && let Some(home) = env::var_os("HOME")
                        .map(PathBuf::from)
                        .filter(|p| p.is_absolute())
                {
                    let default_root = home.join("Library/Application Support/Claude");
                    let desktop_root = config
                        .claude_desktop_root
                        .get_or_insert_with(|| default_root.clone());
                    if *desktop_root == default_root {
                        config
                            .claude_app_bundle
                            .push(PathBuf::from("/Applications/Claude.app"));
                        config
                            .claude_app_bundle
                            .push(home.join("Applications/Claude.app"));
                    }
                }
                config.claude_root = Some(root(
                    config.claude_root.clone(),
                    "CLAUDE_CONFIG_DIR",
                    ".claude",
                )?);
                config.claude_executable = config
                    .claude_executable
                    .or_else(|| find_executable(ProviderKind::Claude));
            }
        }
        let factory_environment = FactoryEnvironment {
            home: env::var_os("HOME").map(PathBuf::from),
            helper: env::current_exe().ok(),
        };
        let conversation = if *session_search {
            if state_root.is_some() {
                return Err(CliError::new(
                    ErrorCode::InvalidArguments,
                    "read-only session search does not use a transaction state root",
                ));
            }
            Some(conversation::read_only_executor(&config)?)
        } else if *session_removal {
            Some(conversation::executor(&config, state_root.as_deref())?)
        } else {
            if state_root.is_some() {
                return Err(CliError::new(
                    ErrorCode::InvalidArguments,
                    "--state-root requires --session-removal",
                ));
            }
            None
        };
        let executor = Arc::new(
            move |request: ReadOnlyRequest,
                  control: QueryControl|
                  -> Result<ReadOnlyResponse, ApplicationError> {
                control.check()?;
                let mut selected = config.clone();
                if let Some(provider) = request.provider {
                    let requested = match provider {
                        ProviderKind::Codex => SelectedProvider::Codex,
                        ProviderKind::Claude => SelectedProvider::Claude,
                    };
                    if config.provider.is_some_and(|configured| {
                        !matches!(
                            (configured, requested),
                            (SelectedProvider::Codex, SelectedProvider::Codex)
                                | (SelectedProvider::Claude, SelectedProvider::Claude)
                        )
                    }) {
                        return Err(ProviderError::ProviderUnsupported(provider).into());
                    }
                    selected.provider = Some(requested);
                }
                let adapters = adapters(&selected, Some(&factory_environment)).map_err(|_| {
                    ProviderError::CapabilityUnavailable(
                        "provider configuration unavailable".into(),
                    )
                })?;
                control.check()?;
                DiscoveryService::new(adapters.iter().map(|a| a.as_ref()).collect())
                    .dispatch(request, control)
            },
        );
        agentmender_mcp_server::run_stdio_with_conversation(executor, conversation)
            .map_err(|_| CliError::new(ErrorCode::OutputError, "MCP transport failed"))?;
        return Ok(false);
    }
    let mode = if args.fixture.is_some() {
        "fixture"
    } else {
        "live"
    };
    let adapters = adapters(&args, None)?;
    let service = DiscoveryService::new(adapters.iter().map(|a| a.as_ref()).collect());
    let scope = DiscoveryScope {
        project_path: args.project.clone(),
        include_archived: !args.active_only,
        max_entries: args.max_entries,
    };
    match &args.command {
        Action::Plans {
            action:
                approvals::PlanAction::Prepare {
                    intent,
                    id,
                    state_root,
                },
        } => return approvals::prepare(&args, &service, &scope, *intent, id, state_root),
        Action::Approve {
            plan_id,
            transaction_id,
            state_root,
        } => return approvals::approve(&args, &service, plan_id, transaction_id, state_root),
        _ => {}
    }
    match args.command {
        Action::Mcp { .. }
        | Action::Transactions { .. }
        | Action::Plans { .. }
        | Action::Approve { .. }
        | Action::Approvals { .. } => unreachable!("handled above"),
        Action::Providers => {
            let report = service.providers();
            let incomplete = !report.failures.is_empty()
                || report.providers.iter().any(|p| {
                    p.presence == Presence::NotDetected
                        || p.compatibility.status == CompatibilityStatus::AuditOnlyUnknown
                });
            if args.json {
                emit(mode, &report)?;
            } else {
                println!("Mode: {mode}");
                for p in &report.providers {
                    print_provider(p);
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
                    print_provider(&s.provider);
                    println!("  discovery: {:?} — {}", s.completeness, s.coverage);
                    println!("  discovery evidence: {:?}", s.evidence_status);
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
        Action::Graph => {
            let report = service.graph(&scope)?;
            let incomplete = !report.failures.is_empty()
                || report
                    .graphs
                    .iter()
                    .any(|g| g.context().completeness != Completeness::Complete);
            if args.json {
                emit(mode, &report)?;
            } else {
                println!("Mode: {mode}");
                for graph in &report.graphs {
                    print_provider(&graph.context().provider);
                    println!(
                        "  discovery: {:?} — {}",
                        graph.context().completeness,
                        graph.context().coverage
                    );
                    println!(
                        "  {} nodes, {} relationships, {} roots",
                        graph.nodes().len(),
                        graph.edges().len(),
                        graph.roots().len()
                    );
                    for diagnostic in graph.diagnostics() {
                        eprintln!("  {diagnostic:?}");
                    }
                    for diagnostic in &graph.context().diagnostics {
                        eprintln!("  {:?}: {}", diagnostic.code, diagnostic.message);
                    }
                }
                for failure in &report.failures {
                    eprintln!(
                        "{}: {:?}: {}",
                        failure.provider, failure.code, failure.message
                    );
                }
            }
            Ok(incomplete)
        }
        Action::Verify => {
            let report = service.verify(&scope)?;
            let incomplete = !report.failures.is_empty()
                || report.reports.is_empty()
                || report.reports.iter().any(|r| {
                    !matches!(
                        r.verification_status,
                        VerificationStatus::Healthy
                            | VerificationStatus::HealthyWithExpectedResiduals
                    )
                });
            if args.json {
                emit(mode, &report)?;
            } else {
                println!("Mode: {mode}");
                for r in &report.reports {
                    print_verification(r);
                }
                for failure in &report.failures {
                    eprintln!(
                        "{}: {:?}: {}",
                        failure.provider, failure.code, failure.message
                    );
                }
            }
            Ok(incomplete)
        }
        Action::Plan { intent, ref id } => {
            if matches!(intent, PlanIntent::CleanupSession) {
                let report = agentmender_application::cleanup_inputs(
                    &lifecycle::scoped_adapter(&args)?,
                    id,
                    &scope,
                )?;
                let incomplete = report.plan.content().status != PlanStatus::Ready;
                if args.json {
                    emit(
                        "planning_only",
                        serde_json::json!({"plan":report.plan,"contexts":report.contexts,"failures":report.failures}),
                    )?;
                } else {
                    println!("Mode: {mode}; planning only");
                    print_plan(&report.plan);
                }
                return Ok(incomplete);
            }
            let id = EntityId::new(id)
                .map_err(|e| CliError::new(ErrorCode::InvalidIdentity, e.to_string()))?;
            let report = service.plan(&id, intent.into(), &scope)?;
            let incomplete =
                report.plan.content().status != PlanStatus::Ready || !report.failures.is_empty();
            if args.json {
                emit("planning_only", &report)?;
            } else {
                println!("Mode: {mode}; planning only");
                print_plan(&report.plan);
                for failure in &report.failures {
                    eprintln!(
                        "{}: {:?}: {}",
                        failure.provider, failure.code, failure.message
                    );
                }
            }
            Ok(incomplete)
        }
        Action::Explain { id } => {
            let id = EntityId::new(id)
                .map_err(|e| CliError::new(ErrorCode::InvalidIdentity, e.to_string()))?;
            let report = service.explain(&id, &scope)?;
            let incomplete = !report.failures.is_empty()
                || report
                    .contexts
                    .iter()
                    .any(|c| c.completeness != Completeness::Complete);
            if args.json {
                emit(mode, &report)?;
            } else {
                println!("Mode: {mode}\nExplanation for {id}");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .map_err(|e| CliError::new(ErrorCode::OutputError, e.to_string()))?
                );
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
                let context = service.scan(&scope)?;
                if let Some(snapshot) = context
                    .snapshots
                    .iter()
                    .find(|s| s.provider.kind == report.provider)
                {
                    print_provider(&snapshot.provider);
                    println!(
                        "  discovery: {:?} — {}",
                        snapshot.completeness, snapshot.coverage
                    );
                    println!("  discovery evidence: {:?}", snapshot.evidence_status);
                }
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
    let raw_args: Vec<_> = env::args_os().collect();
    if raw_args.len() == 2 && raw_args[1] == "--internal-codex-metadata-capture" {
        return match agentmender_provider_codex::run_metadata_capture_worker() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => {
                eprintln!("internal metadata evidence capture failed");
                ExitCode::from(2)
            }
        };
    }
    #[cfg(unix)]
    if raw_args.len() == 2 && raw_args[1] == "--internal-codex-catalog-evidence-capture" {
        return match agentmender_provider_codex::run_catalog_evidence_worker() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => {
                eprintln!("internal Desktop catalog evidence capture failed");
                ExitCode::from(2)
            }
        };
    }
    if raw_args.len() == 2 && raw_args[1] == "--internal-codex-scoped-evidence-capture" {
        return match agentmender_provider_codex::run_scoped_evidence_worker() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => {
                eprintln!("internal scoped evidence capture failed");
                ExitCode::from(2)
            }
        };
    }
    if raw_args.len() == 2 && raw_args[1] == "--internal-codex-evidence-capture" {
        return match agentmender_provider_codex::run_evidence_capture_worker() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => {
                eprintln!("internal evidence capture failed");
                ExitCode::from(2)
            }
        };
    }
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
