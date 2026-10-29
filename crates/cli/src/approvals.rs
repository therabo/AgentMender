//! Local terminal approval broker; the MCP surface never calls this module.
use super::{Args, CliError, ErrorCode, PlanIntent, absolute_path, transactions::protected_roots};
use agentmender_application::DiscoveryService;
use agentmender_domain::{ApprovalId, DiscoveryScope, EntityId, PlanStatus, TransactionId};
use agentmender_persistence::{OwnedStateRoot, TransactionReader, TransactionStore};
use clap::Subcommand;
use std::{
    io::{self, BufRead, IsTerminal, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Subcommand)]
pub enum PlanAction {
    Prepare {
        intent: PlanIntent,
        id: String,
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
    Inspect {
        plan_id: String,
        #[arg(long)]
        transaction_id: String,
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
}
#[derive(Clone, Subcommand)]
pub enum ApprovalAction {
    List {
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
    Inspect {
        transaction_id: String,
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
    History {
        transaction_id: String,
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
    Revoke {
        approval_id: String,
        #[arg(long)]
        transaction_id: String,
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
    Migrate {
        #[arg(long, value_parser = absolute_path)]
        state_root: PathBuf,
    },
}
fn unavailable() -> CliError {
    CliError::new(
        ErrorCode::TransactionUnavailable,
        "durable state unavailable, stale, or ineligible",
    )
}
fn tx(s: &str) -> Result<TransactionId, CliError> {
    s.parse()
        .map_err(|_| CliError::new(ErrorCode::InvalidIdentity, "invalid transaction ID"))
}
fn root(args: &Args, path: &Path, create: bool) -> Result<OwnedStateRoot, CliError> {
    let protected = protected_roots(args)?;
    if create && !path.exists() {
        OwnedStateRoot::create(path, &protected)
    } else {
        OwnedStateRoot::open(path, &protected)
    }
    .map_err(|_| unavailable())
}
fn reader(args: &Args, path: &Path) -> Result<TransactionReader, CliError> {
    TransactionReader::open(root(args, path, false)?).map_err(|_| unavailable())
}
/// JSON escapes all C0 controls; escape directional controls and other format controls too.
fn safe<T: serde::Serialize>(value: &T) -> Result<String, CliError> {
    let json = serde_json::to_string_pretty(value).map_err(|_| unavailable())?;
    let escaped: String = json.chars().map(|c| if (c.is_control() && c != '\n') || matches!(c, '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{206f}') { format!("\\u{:04x}", c as u32) } else { c.to_string() }).collect();
    if escaped.len() > 512 * 1024 {
        return Err(CliError::new(
            ErrorCode::OutputError,
            "review exceeds the 512 KiB output limit",
        ));
    }
    Ok(escaped)
}
fn terminal() -> Result<(), CliError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "local interactive terminal input and output required",
        ));
    }
    Ok(())
}
fn challenge(action: &str, id: &TransactionId, digest: &str) -> Result<(), CliError> {
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|_| unavailable())?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let expected = format!("{action} {id} {digest} {nonce}");
    println!("Type exactly: {expected}");
    io::stdout().flush().map_err(|_| unavailable())?;
    let mut answer = String::new();
    io::BufReader::new(io::stdin().lock().take(4096))
        .read_line(&mut answer)
        .map_err(|_| unavailable())?;
    if answer.trim_end_matches(['\r', '\n']) != expected {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "challenge did not match",
        ));
    }
    Ok(())
}
fn review_value(
    snapshot: &agentmender_domain::TransactionSnapshot,
) -> Result<serde_json::Value, CliError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| unavailable())?
        .as_secs();
    Ok(
        serde_json::json!({"transaction_id":snapshot.id(),"sequence":snapshot.sequence(),"schema_version":snapshot.schema_version(),"state":snapshot.state(),"binding":snapshot.binding(),"plan":snapshot.review_plan(),"approval_state":format!("{:?}", snapshot.approval_status_at(now)),"grant":snapshot.approval().grant,"recovery":snapshot.recovery(),"recovery_limitations":"ReconciliationOnly: recovery records do not restore provider state or authorize execution","terminal_scope":"This interaction proves only a current-user process used a local terminal; it does not establish human identity against a same-user shell or PTY"}),
    )
}
fn print_review(
    snapshot: &agentmender_domain::TransactionSnapshot,
    json: bool,
) -> Result<(), CliError> {
    let value = review_value(snapshot)?;
    if json {
        println!(
            "{}",
            safe(&serde_json::json!({"mode":"durable_knowledge_only", "report":value}))?
        );
    } else {
        let mut output = String::new();
        if let Some(plan) = snapshot.review_plan() {
            let content = plan.content();
            let header = serde_json::json!({"provider":content.request.target.provider,"target":content.request.target,"intent":content.request.intent,"native_operation":content.operations,"plan_digest":plan.digest(),"transaction_id":snapshot.id(),"status":snapshot.state(),"recovery":snapshot.recovery().map(|r| r.digest()),"blockers":content.blockers});
            output.push_str(&format!("Review summary:\n{}\n", safe(&header)?));
        }
        output.push_str(&format!("Complete durable review:\n{}", safe(&value)?));
        if output.len() > 512 * 1024 {
            return Err(CliError::new(
                ErrorCode::OutputError,
                "review exceeds the 512 KiB output limit",
            ));
        }
        println!("{output}");
    }
    Ok(())
}
pub fn prepare(
    args: &Args,
    service: &DiscoveryService<'_>,
    scope: &DiscoveryScope,
    intent: PlanIntent,
    id: &str,
    path: &Path,
) -> Result<bool, CliError> {
    if args.json {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "plan preparation has no JSON mode",
        ));
    }
    let id = EntityId::new(id)
        .map_err(|_| CliError::new(ErrorCode::InvalidIdentity, "invalid target"))?;
    let inputs = service.approval_inputs(&|target| target.local_id == id, intent.into(), scope)?;
    println!("{}", safe(&inputs.plan)?);
    if inputs.plan.content().status != PlanStatus::Ready || !inputs.failures.is_empty() {
        return Ok(true);
    }
    let mut store = TransactionStore::open(root(args, path, true)?).map_err(|_| unavailable())?;
    let id = store
        .create_transaction(&inputs.plan)
        .map_err(|_| unavailable())?;
    store.prepare_recovery(&id, 0).map_err(|_| unavailable())?;
    store.prepare(&id, 1).map_err(|_| unavailable())?;
    println!("transaction_id: {id}");
    Ok(false)
}
pub fn inspect_plan(args: &Args, action: &PlanAction) -> Result<bool, CliError> {
    let PlanAction::Inspect {
        plan_id,
        transaction_id,
        state_root,
    } = action
    else {
        return Err(unavailable());
    };
    let s = reader(args, state_root)?
        .inspect(&tx(transaction_id)?)
        .map_err(|_| unavailable())?;
    let plan = s.review_plan().ok_or_else(unavailable)?;
    if plan.digest().to_string() != *plan_id {
        return Err(unavailable());
    }
    print_review(&s, args.json)?;
    Ok(false)
}
pub fn approve(
    args: &Args,
    service: &DiscoveryService<'_>,
    plan_id: &str,
    transaction_id: &str,
    path: &Path,
) -> Result<bool, CliError> {
    terminal()?;
    if args.json {
        return Err(CliError::new(
            ErrorCode::InvalidArguments,
            "approval has no JSON mode",
        ));
    }
    let id = tx(transaction_id)?;
    let r = reader(args, path)?;
    let s = r.inspect(&id).map_err(|_| unavailable())?;
    let plan = s.review_plan().ok_or_else(unavailable)?.clone();
    if plan.digest().to_string() != plan_id {
        return Err(unavailable());
    }
    let recovery_verified = r.recovery_available(&id).map_err(|_| unavailable())?;
    print_review(&s, false)?;
    let content = plan.content();
    let target = content.request.target.clone();
    let fresh = service.approval_inputs(
        &|candidate| candidate == &target,
        content.request.intent,
        &content.observation_scope,
    )?;
    println!(
        "Fresh plan blockers:\n{}",
        safe(&fresh.plan.content().blockers)?
    );
    if !agentmender_domain::approval_eligible(
        &s,
        &fresh.graph,
        &fresh.integrity,
        &fresh.context,
        recovery_verified,
    ) {
        return Err(unavailable());
    }
    let sequence = s.sequence();
    drop(r);
    challenge("approve", &id, plan_id)?;
    let mut store = TransactionStore::open(root(args, path, false)?).map_err(|_| unavailable())?;
    let content = plan.content();
    let target = content.request.target.clone();
    let inputs = service.approval_inputs(
        &|candidate| candidate == &target,
        content.request.intent,
        &content.observation_scope,
    )?;
    let grant = store
        .grant_approval(
            &id,
            sequence,
            &plan,
            &inputs.graph,
            &inputs.integrity,
            &inputs.context,
        )
        .map_err(|_| unavailable())?;
    println!("approval_id: {}", grant.id);
    Ok(false)
}
pub fn administration(args: &Args, action: &ApprovalAction) -> Result<bool, CliError> {
    match action {
        ApprovalAction::Migrate { state_root } => {
            if args.json {
                return Err(CliError::new(
                    ErrorCode::InvalidArguments,
                    "migration has no JSON mode",
                ));
            }
            TransactionStore::migrate_v1(root(args, state_root, false)?)
                .map_err(|_| unavailable())?;
            println!("migration complete");
        }
        ApprovalAction::List { state_root } => {
            let snapshots = reader(args, state_root)?
                .list()
                .map_err(|_| unavailable())?;
            if args.json {
                let views: Vec<_> = snapshots
                    .iter()
                    .map(review_value)
                    .collect::<Result<_, _>>()?;
                println!(
                    "{}",
                    safe(&serde_json::json!({"mode":"durable_knowledge_only", "report":views}))?
                );
            } else {
                let views: Vec<_> = snapshots
                    .iter()
                    .map(review_value)
                    .collect::<Result<_, _>>()?;
                println!("{}", safe(&views)?);
            }
        }
        ApprovalAction::Inspect {
            transaction_id,
            state_root,
        } => {
            let s = reader(args, state_root)?
                .inspect(&tx(transaction_id)?)
                .map_err(|_| unavailable())?;
            print_review(&s, args.json)?;
        }
        ApprovalAction::History {
            transaction_id,
            state_root,
        } => {
            let r = reader(args, state_root)?;
            let records = r.records(&tx(transaction_id)?).map_err(|_| unavailable())?;
            if args.json {
                println!(
                    "{}",
                    safe(&serde_json::json!({"mode":"durable_knowledge_only", "report":records}))?
                );
            } else {
                println!("{}", safe(&records)?);
            }
        }
        ApprovalAction::Revoke {
            approval_id,
            transaction_id,
            state_root,
        } => {
            terminal()?;
            if args.json {
                return Err(CliError::new(
                    ErrorCode::InvalidArguments,
                    "revocation has no JSON mode",
                ));
            }
            let id = tx(transaction_id)?;
            let approval: ApprovalId = approval_id.parse().map_err(|_| unavailable())?;
            let r = reader(args, state_root)?;
            let s = r.inspect(&id).map_err(|_| unavailable())?;
            if s.approval().grant.as_ref().map(|g| &g.id) != Some(&approval) {
                return Err(unavailable());
            }
            print_review(&s, false)?;
            let sequence = s.sequence();
            let digest = s
                .review_plan()
                .ok_or_else(unavailable)?
                .digest()
                .to_string();
            drop(r);
            challenge("revoke", &id, &digest)?;
            TransactionStore::open(root(args, state_root, false)?)
                .map_err(|_| unavailable())?
                .revoke_approval(&id, sequence, &approval)
                .map_err(|_| unavailable())?;
            println!("approval revoked");
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::safe;
    #[test]
    fn oversized_review_fails_instead_of_truncating() {
        assert!(safe(&"x".repeat(512 * 1024)).is_err());
    }
    #[test]
    fn review_escapes_terminal_metadata_losslessly() {
        let metadata = "name\n\u{1b}[31m\u{202e}text\u{0085}";
        let Ok(encoded) = safe(&metadata) else {
            panic!("serialization failed")
        };
        assert!(
            !encoded
                .chars()
                .any(|c| (c.is_control() && c != '\n') || c == '\u{202e}')
        );
        assert_eq!(serde_json::from_str::<String>(&encoded).unwrap(), metadata);
    }
}
