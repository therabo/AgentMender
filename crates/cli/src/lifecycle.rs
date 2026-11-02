//! Trusted composition only; application owns the approved lifecycle workflow.
use super::{
    Args, CliError, ErrorCode, SelectedProvider, root,
    transactions::{TransactionAction, protected_roots},
};
use agentmender_application::{DiscoveryService, lifecycle};
use agentmender_domain::{ProviderKind, TransactionId, TransactionState};
use agentmender_persistence::{OwnedStateRoot, TransactionStore};
use agentmender_provider_api::{ProviderLocation, process::find_executable};
use agentmender_provider_codex::{CodexAdapter, CodexLifecycleAdapter, CodexScopedCleanupAdapter};
use std::{env, path::Path};

pub(super) fn scoped_adapter(args: &Args) -> Result<CodexScopedCleanupAdapter, CliError> {
    if args.fixture.is_some()
        || args.desktop_fixture
        || matches!(args.provider, Some(SelectedProvider::Claude))
    {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "scoped cleanup requires live Codex configuration",
        ));
    }
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
    Ok(CodexScopedCleanupAdapter::new(
        CodexAdapter::new(location).with_evidence_helper(helper, home),
    ))
}

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
    let scoped = store
        .inspect(&id)
        .ok()
        .and_then(|s| {
            s.review_plan().map(|p| {
                p.content().request.intent == agentmender_domain::LifecycleIntent::CleanupSession
            })
        })
        .unwrap_or(false);
    let result = if scoped {
        let cleanup = scoped_adapter(args)?;
        if reconcile {
            lifecycle::reconcile_cleanup(&mut store, &id, &cleanup)
        } else {
            lifecycle::execute_scoped_cleanup(&mut store, &id, &cleanup, &cleanup)
        }
    } else if reconcile {
        lifecycle::reconcile(&mut store, &id, &discovery, &adapter)
    } else {
        lifecycle::execute(&mut store, &id, &discovery, &adapter)
    };
    let state = match result {
        Ok(state) => state,
        Err(error) if scoped => {
            let snapshot = store.inspect(&id).map_err(|_| unavailable())?;
            if !matches!(
                snapshot.state(),
                TransactionState::OutcomeUnknown | TransactionState::RecoveryRequired
            ) {
                return Err(CliError::new(
                    ErrorCode::TransactionUnavailable,
                    format!("{error}"),
                ));
            }
            snapshot.state()
        }
        Err(error) => {
            return Err(CliError::new(
                ErrorCode::TransactionUnavailable,
                format!("{error}"),
            ));
        }
    };
    if scoped {
        return report_cleanup(args, &store, &id, state, reconcile);
    }
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

fn report_cleanup(
    args: &Args,
    store: &TransactionStore,
    id: &TransactionId,
    state: TransactionState,
    reconcile: bool,
) -> Result<bool, CliError> {
    use agentmender_domain::{CleanupActionOutcome, TransactionEvent};
    let unavailable = || {
        CliError::new(
            ErrorCode::TransactionUnavailable,
            "cleanup report unavailable",
        )
    };
    let snapshot = store.inspect(id).map_err(|_| unavailable())?;
    let manifest = snapshot
        .review_plan()
        .and_then(|p| p.content().cleanup_manifest.as_ref())
        .ok_or_else(unavailable)?;
    let records = store.records(id).map_err(|_| unavailable())?;
    let report = records
        .iter()
        .rev()
        .find_map(|record| match record.event() {
            TransactionEvent::CleanupObserved { report } => Some(report),
            _ => None,
        });
    let assessment =
        report.map(|report| agentmender_domain::assess_legacy_session_removal(manifest, report));
    let product_complete = assessment
        .as_ref()
        .is_some_and(|assessment| assessment.product_complete());
    let retained_known_bytes = manifest
        .retained_files
        .iter()
        .try_fold(0u64, |n, f| n.checked_add(f.size_bytes));
    let planned_bytes = manifest
        .payloads
        .iter()
        .try_fold(0u64, |n, p| n.checked_add(p.identity.size_bytes))
        .ok_or_else(unavailable)?;
    let count = |outcome| report.map(|r| r.actions.iter().filter(|a| a.outcome == outcome).count());
    let required = serde_json::json!({"planned": manifest.payloads.len(), "removed": count(CleanupActionOutcome::Removed), "already_absent": count(CleanupActionOutcome::AlreadyAbsent), "retained": count(CleanupActionOutcome::Retained), "unknown": count(CleanupActionOutcome::Unknown), "failed": count(CleanupActionOutcome::Failed)});
    if args.json {
        super::emit(
            "approved_scoped_cleanup",
            serde_json::json!({"transaction_id":id,"state":state,"reconciliation_only":reconcile,"outcome":assessment.as_ref().map(|a|a.outcome),"outcome_contract":"complete_session_removal_v1","primary_cleanup_outcome":report.map(|r|r.outcome),"primary_cleanup_contract":"scoped_primary_cleanup_v1","session_removal":assessment,"product_complete":product_complete,"required_actions":required,"optional_index_planned":manifest.optional_index.is_some(),"optional_index":report.and_then(|r|r.optional_index),"residual_count":manifest.residuals.len(),"canonical_row_absent":report.map(|r|r.canonical_row_absent),"retained_primary_unchanged":report.map(|r|r.retained_primary_unchanged),"logical_planned_bytes":planned_bytes,"logical_unlinked_bytes":report.map(|r|r.logical_unlinked_bytes),"retained_known_logical_bytes":retained_known_bytes,"physical_reclaimed_bytes":null,"residuals":manifest.residuals,"post_cutoff_residue":report.map(|r|r.post_cutoff_residue),"recovery":"reconciliation_metadata_only"}),
        )?;
    } else {
        if state == TransactionState::Succeeded {
            println!(
                "Primary state for session {} removed at the fenced cutoff; logical payload bytes unlinked: {} of {} planned.",
                manifest.native_session_id,
                report.map_or(0, |r| r.logical_unlinked_bytes),
                planned_bytes
            );
        } else {
            println!("Transaction {id}: {state:?}. Cleanup completion is not established.");
        }
        println!(
            "Complete session removal: unverified. Supported provider/UI projections, restart and maximum safe cleanup are not established by this primary-cleanup profile."
        );
        println!(
            "Known retained rollout bytes: {}. Physical allocation reclaimed: unknown.",
            retained_known_bytes.map_or_else(|| "unknown (sum overflow)".into(), |n| n.to_string())
        );
        println!(
            "Required actions: {}",
            serde_json::to_string(&required).map_err(|_| unavailable())?
        );
        println!(
            "Optional index outcome: {}",
            serde_json::to_string(&report.and_then(|r| r.optional_index))
                .map_err(|_| unavailable())?
        );
        println!(
            "Retained residuals: {}",
            super::approvals::safe(&manifest.residuals)?
        );
        println!("Recovery stores reconciliation metadata and cannot restore the payload.");
    }
    // A historical successful primary transaction is not product-level removal.
    Ok(!product_complete)
}
