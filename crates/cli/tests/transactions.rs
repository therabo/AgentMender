#[path = "../../persistence/tests/common/mod.rs"]
mod common;

use agentmender_domain::{LifecycleIntent, evaluate_integrity};
use agentmender_persistence::{OwnedStateRoot, TransactionStore};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn run(home: &Path, root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home)
        .env("PATH", "")
        .args(["--json", "transactions", "--state-root"])
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}
fn contents(path: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(key, None);
                walk(root, &path, out);
            } else {
                out.insert(key, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(path, path, &mut out);
    out
}
fn fixture(home: &Path) -> (PathBuf, String) {
    let path = home.join("owned");
    let root = OwnedStateRoot::create(&path, &[home.join(".codex"), home.join(".claude")]).unwrap();
    let c = common::case(0);
    let plan = c.plan(LifecycleIntent::Archive);
    let report = evaluate_integrity(&c.graph, &c.proof);
    let mut store = TransactionStore::open(root).unwrap();
    let id = store.create_transaction(&plan).unwrap();
    store.prepare_recovery(&id, 0).unwrap();
    store.prepare(&id, 1).unwrap();
    store
        .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    let _receipt = store
        .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    drop(store);
    (path, id.to_string())
}
fn report(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["mode"], "durable_knowledge_only");
    value["report"].clone()
}
#[test]
fn inspection_is_read_only_and_preserves_unknown_outcome() {
    let home = tempfile::tempdir().unwrap();
    let (root, id) = fixture(home.path());
    let before = contents(&root);
    let list = report(run(home.path(), &root, &["list"]));
    assert_eq!(list.as_array().unwrap().len(), 1);
    let inspect = report(run(home.path(), &root, &["inspect", &id]));
    assert_eq!(inspect["transaction_id"], id);
    assert_eq!(inspect["state"], "outcome_unknown");
    assert_eq!(inspect["recovery"], "verified");
    assert_eq!(inspect["protected_recovery_required"], true);
    assert_eq!(inspect["provider_state_verified"], false);
    assert_eq!(inspect["execution_authorized"], false);
    let journal = report(run(home.path(), &root, &["journal", &id]));
    assert_eq!(journal.as_array().unwrap().len(), 5);
    assert_eq!(contents(&root), before);
}
#[test]
fn missing_empty_and_provider_roots_are_never_initialized() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("missing");
    assert!(!run(home.path(), &missing, &["list"]).status.success());
    assert!(!missing.exists());
    let empty = home.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let before = contents(&empty);
    assert!(!run(home.path(), &empty, &["list"]).status.success());
    assert_eq!(contents(&empty), before);
    let provider = home.path().join(".codex");
    fs::create_dir(&provider).unwrap();
    fs::write(provider.join("canary"), b"provider state").unwrap();
    let before = contents(&provider);
    assert!(!run(home.path(), &provider, &["list"]).status.success());
    assert_eq!(contents(&provider), before);
}
#[test]
fn unsupported_or_incomplete_commands_and_bad_ids_are_rejected() {
    let home = tempfile::tempdir().unwrap();
    let (root, _) = fixture(home.path());
    let before = contents(&root);
    for command in [
        "create",
        "execute",
        "resume",
        "retry-effect",
        "rollback",
        "approve",
    ] {
        assert!(
            !run(home.path(), &root, &[command]).status.success(),
            "{command}"
        );
    }
    assert!(
        !run(home.path(), &root, &["inspect", "not-a-transaction-id"])
            .status
            .success()
    );
    assert_eq!(contents(&root), before);
}

#[test]
fn lifecycle_cannot_execute_or_retry_an_existing_unsupported_attempt() {
    let home = tempfile::tempdir().unwrap();
    let (root, id) = fixture(home.path());
    let before = contents(&root);
    for action in ["execute", "reconcile"] {
        let result = run(home.path(), &root, &[action, &id]);
        assert_eq!(result.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["code"], "transaction_unavailable");
        assert_eq!(contents(&root), before);
    }
    assert_eq!(
        report(run(home.path(), &root, &["inspect", &id]))["state"],
        "outcome_unknown"
    );
}

#[test]
fn lifecycle_rejects_fixture_and_claude_configuration_before_state_access() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("must-not-be-created");
    let id = "a".repeat(64);
    for action in ["execute", "reconcile"] {
        for options in [
            vec!["--provider", "claude"],
            vec!["--provider", "codex", "--fixture", "unused"],
        ] {
            let output = Command::new(env!("CARGO_BIN_EXE_agentmender"))
                .env_clear()
                .env("HOME", home.path())
                .env("PATH", "")
                .args(options)
                .args(["--json", "transactions", "--state-root"])
                .arg(&missing)
                .args([action, &id])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(!missing.exists());
        }
    }
    let (root, id) = fixture(home.path());
    let before = contents(&root);
    for action in ["execute", "reconcile"] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmender"))
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "")
            .args([
                "--provider",
                "claude",
                "--json",
                "transactions",
                "--state-root",
            ])
            .arg(&root)
            .args([action, &id])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(contents(&root), before);
    }
}
#[test]
fn corrupt_recovery_is_reported_without_changing_unknown_state() {
    let home = tempfile::tempdir().unwrap();
    let (root, id) = fixture(home.path());
    let path = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("recovery-")
        })
        .unwrap();
    fs::write(path, b"corrupt recovery").unwrap();
    let before = contents(&root);
    let inspect = report(run(home.path(), &root, &["inspect", &id]));
    assert_eq!(inspect["state"], "outcome_unknown");
    assert_eq!(inspect["recovery"], "unavailable");
    assert_eq!(inspect["protected_recovery_required"], true);
    assert_eq!(inspect["provider_state_verified"], false);
    assert_eq!(inspect["execution_authorized"], false);
    assert_eq!(contents(&root), before);
}
#[test]
fn torn_journal_and_future_database_schema_fail_without_writes() {
    let home = tempfile::tempdir().unwrap();
    let (root, id) = fixture(home.path());
    let journal = root.join("transactions.journal");
    let mut bytes = fs::read(&journal).unwrap();
    bytes.extend_from_slice(b"{torn");
    fs::write(&journal, bytes).unwrap();
    let before = contents(&root);
    assert!(!run(home.path(), &root, &["inspect", &id]).status.success());
    assert_eq!(contents(&root), before);
    let second = tempfile::tempdir().unwrap();
    let (root, id) = fixture(second.path());
    let db = root.join("transactions.sqlite3");
    let mut bytes = fs::read(&db).unwrap();
    bytes[60..64].copy_from_slice(&99_u32.to_be_bytes());
    fs::write(&db, bytes).unwrap();
    let before = contents(&root);
    assert!(
        !run(second.path(), &root, &["inspect", &id])
            .status
            .success()
    );
    assert_eq!(contents(&root), before);
}

#[test]
fn approval_and_revocation_reject_non_terminal_without_writes() {
    let home = tempfile::tempdir().unwrap();
    let (root, id) = fixture(home.path());
    let before = contents(&root);
    let binary = env!("CARGO_BIN_EXE_agentmender");
    let approval = "0".repeat(64);
    for args in [
        vec![
            "approve",
            "invalid",
            "--transaction-id",
            &id,
            "--state-root",
            root.to_str().unwrap(),
        ],
        vec![
            "approvals",
            "revoke",
            &approval,
            "--transaction-id",
            &id,
            "--state-root",
            root.to_str().unwrap(),
        ],
    ] {
        let output = Command::new(binary)
            .env("HOME", home.path())
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("terminal"));
        assert_eq!(contents(&root), before);
    }
}

#[test]
fn plan_inspection_returns_the_exact_durable_plan_without_writes() {
    let home = tempfile::tempdir().unwrap();
    let (root, id) = fixture(home.path());
    let owned = OwnedStateRoot::open(
        &root,
        &[home.path().join(".codex"), home.path().join(".claude")],
    )
    .unwrap();
    let reader = agentmender_persistence::TransactionReader::open(owned).unwrap();
    let snapshot = reader.inspect(&id.parse().unwrap()).unwrap();
    let plan = snapshot.review_plan().unwrap();
    let digest = plan.digest().to_string();
    let expected = serde_json::to_value(plan).unwrap();
    drop(reader);
    let before = contents(&root);
    let output = Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env("HOME", home.path())
        .args([
            "--json",
            "plans",
            "inspect",
            &digest,
            "--transaction-id",
            &id,
            "--state-root",
        ])
        .arg(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let review: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(review["report"]["plan"], expected);
    assert_eq!(review["report"]["transaction_id"], id);
    assert_eq!(contents(&root), before);
}

#[test]
fn claude_unsupported_prepare_never_creates_state_or_grants() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/claude")
        .join("2.1.311-synthetic");
    let scan = Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "")
        .args(["--json", "--provider", "claude", "--fixture"])
        .arg(&fixture)
        .arg("scan")
        .output()
        .unwrap();
    assert!(
        scan.status.success(),
        "{}",
        String::from_utf8_lossy(&scan.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&scan.stdout).unwrap();
    let id = result["report"]["snapshots"][0]["sessions"][0]["id"]
        .as_str()
        .unwrap();
    let root = home.path().join("missing-owned-root");
    let prepare = Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "")
        .args(["--provider", "claude", "--fixture"])
        .arg(&fixture)
        .args(["plans", "prepare", "purge", id, "--state-root"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(prepare.status.code(), Some(2));
    let plan: serde_json::Value = serde_json::from_slice(&prepare.stdout).unwrap();
    assert_eq!(plan["content"]["status"], "unsupported");
    assert_eq!(plan["content"]["request"]["target"]["local_id"], id);
    assert_eq!(plan["content"]["request"]["intent"], "purge");
    assert!(plan["content"]["operations"].as_array().unwrap().is_empty());
    assert!(plan["content"]["capability"].is_null());
    assert!(plan["content"]["effect_contract"].is_null());
    assert!(plan.get("transaction_id").is_none());
    assert!(!root.exists());
}
