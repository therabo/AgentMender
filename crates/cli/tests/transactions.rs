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
    let _receipt = store
        .record_intent(&id, 2, &plan, &c.graph, &report, &c.planning)
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
    assert_eq!(journal.as_array().unwrap().len(), 4);
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
fn no_effect_or_approval_subcommands_are_available_and_bad_id_is_rejected() {
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
    bytes[60..64].copy_from_slice(&2_u32.to_be_bytes());
    fs::write(&db, bytes).unwrap();
    let before = contents(&root);
    assert!(
        !run(second.path(), &root, &["inspect", &id])
            .status
            .success()
    );
    assert_eq!(contents(&root), before);
}
