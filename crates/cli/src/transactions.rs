use super::{Args, CliError, ErrorCode, emit};
use agentmender_application::transactions::{
    RecoveryAvailability, TransactionInspection, TransactionInspectionService,
    TransactionQueryError, TransactionReadPort,
};
use agentmender_domain::{JournalRecord, TransactionId, TransactionSnapshot};
use agentmender_persistence::{OwnedStateRoot, TransactionReader};
use clap::Subcommand;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Clone, Subcommand)]
pub enum TransactionAction {
    List,
    Inspect { id: String },
    Journal { id: String },
}

struct Reader(TransactionReader);
impl Reader {
    fn view(
        &self,
        snapshot: TransactionSnapshot,
    ) -> Result<TransactionInspection, TransactionQueryError> {
        let available = self.0.recovery_available(snapshot.id()).unwrap_or(false);
        let recovery = if snapshot.recovery().is_none() {
            RecoveryAvailability::NotPrepared
        } else if available {
            RecoveryAvailability::Verified
        } else {
            RecoveryAvailability::Unavailable
        };
        Ok(TransactionInspection::from_snapshot(&snapshot, recovery))
    }
}
impl TransactionReadPort for Reader {
    fn list(&self) -> Result<Vec<TransactionInspection>, TransactionQueryError> {
        self.0
            .list()
            .map_err(|_| TransactionQueryError::Unavailable)?
            .into_iter()
            .map(|s| self.view(s))
            .collect()
    }
    fn inspect(&self, id: &TransactionId) -> Result<TransactionInspection, TransactionQueryError> {
        let found = self
            .0
            .list()
            .map_err(|_| TransactionQueryError::Unavailable)?
            .into_iter()
            .find(|s| s.id() == id)
            .ok_or(TransactionQueryError::NotFound)?;
        self.view(found)
    }
    fn journal(&self, id: &TransactionId) -> Result<Vec<JournalRecord>, TransactionQueryError> {
        self.inspect(id)?;
        self.0
            .records(id)
            .map_err(|_| TransactionQueryError::Unavailable)
    }
}

pub(super) fn protected_roots(args: &Args) -> Result<Vec<PathBuf>, CliError> {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| {
            CliError::new(
                ErrorCode::InvalidConfiguration,
                "absolute HOME or USERPROFILE is required for protected-root validation",
            )
        })?;
    let mut roots = vec![
        home.join(".codex"),
        home.join(".claude"),
        home.join("Library/Application Support/Claude"),
        home.join("Applications/Codex.app"),
        home.join("Applications/Claude.app"),
    ];
    #[cfg(target_os = "macos")]
    roots.extend([
        PathBuf::from("/Applications/Codex.app"),
        PathBuf::from("/Applications/Claude.app"),
    ]);
    for variable in ["CODEX_HOME", "CLAUDE_CONFIG_DIR"] {
        if let Some(value) = env::var_os(variable) {
            roots.push(PathBuf::from(value));
        }
    }
    #[cfg(windows)]
    for variable in ["APPDATA", "LOCALAPPDATA"] {
        if let Some(value) = env::var_os(variable) {
            roots.push(PathBuf::from(value).join("Claude"));
        }
    }
    roots.extend(
        [
            &args.fixture,
            &args.codex_root,
            &args.claude_root,
            &args.claude_desktop_root,
            &args.codex_executable,
            &args.claude_executable,
        ]
        .into_iter()
        .filter_map(|p| p.clone()),
    );
    roots.extend(args.claude_app_bundle.iter().cloned());
    if roots.iter().any(|p| !p.is_absolute()) {
        return Err(CliError::new(
            ErrorCode::InvalidConfiguration,
            "protected provider roots must be absolute",
        ));
    }
    Ok(roots)
}

pub fn run(args: &Args, root: &Path, action: &TransactionAction) -> Result<bool, CliError> {
    if args.fixture.is_some()
        || args.provider.is_some()
        || args.project.is_some()
        || args.active_only
    {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "transaction inspection has no provider or discovery scope",
        ));
    }
    let unavailable = || {
        CliError::new(
            ErrorCode::TransactionUnavailable,
            "durable state is unavailable or requires protected recovery",
        )
    };
    let root = OwnedStateRoot::open(root, &protected_roots(args)?).map_err(|_| unavailable())?;
    let reader = Reader(TransactionReader::open(root).map_err(|_| unavailable())?);
    let service = TransactionInspectionService::new(&reader);
    let id = match action {
        TransactionAction::List => None,
        TransactionAction::Inspect { id } | TransactionAction::Journal { id } => Some(
            id.parse::<TransactionId>()
                .map_err(|_| CliError::new(ErrorCode::InvalidIdentity, "invalid transaction ID"))?,
        ),
    };
    let value = match action {
        TransactionAction::List => serde_json::to_value(service.list().map_err(|_| unavailable())?),
        TransactionAction::Inspect { .. } => serde_json::to_value(
            service
                .inspect(id.as_ref().expect("ID action"))
                .map_err(|_| unavailable())?,
        ),
        TransactionAction::Journal { .. } => serde_json::to_value(
            service
                .journal(id.as_ref().expect("ID action"))
                .map_err(|_| unavailable())?,
        ),
    }
    .map_err(|_| unavailable())?;
    if args.json {
        emit("durable_knowledge_only", value)?;
    } else {
        println!(
            "AgentMender durable knowledge; sequence defines order. Transaction state does not prove current provider state or authorize execution."
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|_| unavailable())?
        );
    }
    Ok(false)
}
