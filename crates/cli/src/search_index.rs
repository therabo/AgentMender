//! Explicit private search cache maintenance; never lifecycle execution.
use super::{Args, CliError, ErrorCode, SelectedProvider};
use agentmender_application::{
    QueryControl,
    indexed_search::{SharedSearchIndex, build_search_index},
};
use agentmender_domain::*;
use agentmender_persistence::open_search_index;
use agentmender_provider_api::ProviderLocation;
use agentmender_provider_codex::CodexAdapter;
use clap::Subcommand;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Subcommand)]
pub(super) enum IndexAction {
    /// Read supported messages into explicitly selected private storage; resume checkpoints.
    Build {
        #[arg(long,default_value_t=1000,value_parser=page_limit)]
        max_pages: usize,
    },
    Status,
    /// Recreate a corrupt derived database; requires no active index connections.
    Rebuild,
    /// Remove all derived postings/text. Does not promise physical secure erasure.
    Reset,
}
fn page_limit(value: &str) -> Result<usize, String> {
    value
        .parse()
        .ok()
        .filter(|n| (1..=100_000).contains(n))
        .ok_or_else(|| "max-pages must be 1..=100000".into())
}
fn failure() -> CliError {
    CliError::new(
        ErrorCode::InvalidConfiguration,
        "private search index unavailable; use a dedicated owner-only directory outside provider/project state",
    )
}
pub(super) fn open(args: &Args, path: &Path) -> Result<SharedSearchIndex, CliError> {
    let protected = super::transactions::protected_roots(args)?;
    Ok(Arc::new(Mutex::new(
        open_search_index(path, &protected).map_err(|_| failure())?,
    )))
}
pub(super) fn run(args: &Args, path: &Path, action: &IndexAction) -> Result<bool, CliError> {
    if !matches!(args.provider, Some(SelectedProvider::Codex)) || args.fixture.is_some() {
        return Err(failure());
    }
    let mut config = args.clone();
    config.codex_root = Some(super::root(
        config.codex_root.clone(),
        "CODEX_HOME",
        ".codex",
    )?);
    if matches!(action, IndexAction::Rebuild) {
        let protected = super::transactions::protected_roots(&config)?;
        let index = agentmender_persistence::rebuild_search_index(path, &protected)
            .map_err(|_| failure())?;
        super::emit("search_index", index.status().map_err(|_| failure())?)?;
        return Ok(false);
    }
    let shared = open(&config, path)?;
    let mut index = shared.lock().map_err(|_| failure())?;
    let build = match action {
        IndexAction::Build { max_pages } => {
            let root = config.codex_root.as_deref().ok_or_else(failure)?;
            let helper = std::env::current_exe().map_err(|_| failure())?;
            let home = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .ok_or_else(failure)?;
            let adapter = CodexAdapter::new(ProviderLocation {
                state_root: root.into(),
                executable: config.codex_executable.clone(),
            })
            .with_evidence_helper(helper, home);
            let excluded = config
                .protect_codex_session
                .iter()
                .map(|id| ArtifactId {
                    provider: ProviderKind::Codex,
                    state_root: root.into(),
                    class: GraphNodeKind::Session,
                    local_id: EntityId::new(format!("codex:session:{id}")).expect("validated UUID"),
                })
                .collect::<Vec<_>>();
            let control = QueryControl::new(Some(Instant::now() + Duration::from_secs(300)));
            Some(
                build_search_index(&adapter, &mut index, root, &excluded, *max_pages, control)
                    .map_err(|_| failure())?,
            )
        }
        IndexAction::Reset => {
            index.reset().map_err(|_| failure())?;
            None
        }
        IndexAction::Status => None,
        IndexAction::Rebuild => unreachable!("handled before opening"),
    };
    let partial = build.as_ref().is_some_and(|b| {
        !b.inventory_exhausted
            || b.pending_sources > 0
            || b.unsupported_sessions > 0
            || b.partial_sources > 0
    });
    let status = index.status().map_err(|_| failure())?;
    super::emit(
        "search_index",
        serde_json::json!({"status":status,"build":build,"provider_sources_modified":false,"absence_proof":false}),
    )?;
    Ok(partial)
}
