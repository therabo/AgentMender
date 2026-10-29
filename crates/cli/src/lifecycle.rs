//! Trusted composition only; application owns the approved lifecycle workflow.
use super::{
    Args, CliError, ErrorCode, SelectedProvider, root,
    transactions::{TransactionAction, protected_roots},
};
use agentmender_application::{DiscoveryService, lifecycle};
use agentmender_domain::{ProviderKind, TransactionId, TransactionState};
use agentmender_persistence::{OwnedStateRoot, TransactionStore};
use agentmender_provider_api::{ProviderLocation, process::find_executable};
use agentmender_provider_codex::{CodexAdapter, CodexLifecycleAdapter};
use std::{env, path::Path};

pub fn run(args: &Args, state_root: &Path, action: &TransactionAction) -> Result<bool, CliError> {
    if args.fixture.is_some()
        || args.desktop_fixture
        || args.project.is_some()
        || args.active_only
        || matches!(args.provider, Some(SelectedProvider::Claude))
    {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "lifecycle requires trusted live Codex configuration and the transaction's original scope",
        ));
    }
    let (id, reconcile) = match action {
        TransactionAction::Execute { id } => (id, false),
        TransactionAction::Reconcile { id } => (id, true),
        _ => {
            return Err(CliError::new(
                ErrorCode::InvalidArguments,
                "lifecycle action required",
            ));
        }
    };
    let id: TransactionId = id
        .parse()
        .map_err(|_| CliError::new(ErrorCode::InvalidIdentity, "invalid transaction ID"))?;
    let home = env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| {
            CliError::new(ErrorCode::InvalidConfiguration, "absolute HOME is required")
        })?;
    let helper = env::current_exe().map_err(|_| {
        CliError::new(
            ErrorCode::InvalidConfiguration,
            "current executable unavailable",
        )
    })?;
    let location = ProviderLocation {
        state_root: root(args.codex_root.clone(), "CODEX_HOME", ".codex")?,
        executable: args
            .codex_executable
            .clone()
            .or_else(|| find_executable(ProviderKind::Codex)),
    };
    let reader =
        CodexAdapter::new(location.clone()).with_evidence_helper(helper.clone(), home.clone());
    let adapter =
        CodexLifecycleAdapter::new(CodexAdapter::new(location).with_evidence_helper(helper, home));
    let discovery = DiscoveryService::new(vec![&reader]);
    let unavailable = || {
        CliError::new(
            ErrorCode::TransactionUnavailable,
            "durable transaction unavailable or protected",
        )
    };
    let owned =
        OwnedStateRoot::open(state_root, &protected_roots(args)?).map_err(|_| unavailable())?;
    let mut store = TransactionStore::open(owned).map_err(|_| unavailable())?;
    let result = if reconcile {
        lifecycle::reconcile(&mut store, &id, &discovery, &adapter)
    } else {
        lifecycle::execute(&mut store, &id, &discovery, &adapter)
    };
    let state = result
        .map_err(|error| CliError::new(ErrorCode::TransactionUnavailable, format!("{error}")))?;
    if args.json {
        super::emit(
            "approved_codex_lifecycle",
            serde_json::json!({"transaction_id":id,"state":state,"reconciliation_only":reconcile,"provider_state_verified":state == TransactionState::Succeeded,"compensation":"not_compensatable","recovery":"reconciliation_metadata_only"}),
        )?;
    } else {
        println!("Transaction {id}: {state:?}");
        println!(
            "Recovery stores reconciliation metadata; it does not restore provider state. Compensation is unavailable."
        );
    }
    Ok(!matches!(
        state,
        TransactionState::Succeeded | TransactionState::Failed
    ))
}
