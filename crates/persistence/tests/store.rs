mod common;
use agentmender_domain::*;
use agentmender_persistence::*;
use std::fs;

fn root(temp: &tempfile::TempDir) -> OwnedStateRoot {
    OwnedStateRoot::create(&temp.path().join("owned"), &[temp.path().join("provider")]).unwrap()
}
#[test]
fn durable_reopen_sequence_and_read_only_inspection() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let report = evaluate_integrity(&c.graph, &c.proof);
    let plan = c.plan(LifecycleIntent::Archive);
    let mut store = TransactionStore::open(root.clone()).unwrap();
    let id = store.create_transaction(&plan).unwrap();
    let id2 = store.create_transaction(&plan).unwrap();
    assert_ne!(id, id2);
    assert!(matches!(
        TransactionStore::open(root.clone()),
        Err(StoreError::Locked)
    ));
    store.prepare_recovery(&id, 0).unwrap();
    assert!(store.prepare(&id, 0).is_err());
    store.prepare(&id, 1).unwrap();
    store
        .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    let receipt = store
        .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::OutcomeUnknown
    );
    drop(store);
    let reader = TransactionReader::open(root.clone()).unwrap();
    assert_eq!(reader.list().unwrap().len(), 2);
    assert!(reader.recovery_available(&id).unwrap());
    assert_eq!(
        reader.inspect(&id).unwrap().state(),
        TransactionState::OutcomeUnknown
    );
    drop(reader);
    let mut store = TransactionStore::open(root).unwrap();
    store
        .record_observation(&id, 4, &receipt.into_observation(), Observation::NoEffect)
        .unwrap();
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::Failed
    );
}
#[test]
fn tamper_and_future_schema_fail_closed_and_external_prefix_survives() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let plan = c.plan(LifecycleIntent::Archive);
    let mut s = TransactionStore::open(root.clone()).unwrap();
    s.create_transaction(&plan).unwrap();
    drop(s);
    let db = rusqlite::Connection::open(t.path().join("owned/transactions.sqlite3")).unwrap();
    db.execute_batch("PRAGMA user_version=99").unwrap();
    drop(db);
    assert!(TransactionReader::open(root.clone()).is_err());
    let prefix = TransactionReader::external_prefix(&root).unwrap();
    assert!(prefix.complete);
    assert_eq!(prefix.records.len(), 1);
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(t.path().join("owned/transactions.journal"))
        .unwrap();
    f.write_all(b"{torn").unwrap();
    drop(f);
    let prefix = TransactionReader::external_prefix(&root).unwrap();
    assert!(!prefix.complete);
    assert_eq!(prefix.records.len(), 1);
}
#[test]
fn missing_recovery_blocks_intent_and_later_advancement() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let report = evaluate_integrity(&c.graph, &c.proof);
    let plan = c.plan(LifecycleIntent::Archive);
    let mut s = TransactionStore::open(root.clone()).unwrap();
    let id = s.create_transaction(&plan).unwrap();
    s.prepare_recovery(&id, 0).unwrap();
    s.prepare(&id, 1).unwrap();
    s.grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    let manifest = s.inspect(&id).unwrap().recovery().unwrap().clone();
    for e in fs::read_dir(t.path().join("owned")).unwrap() {
        let p = e.unwrap().path();
        if p.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("recovery-")
        {
            fs::remove_file(p).unwrap();
        }
    }
    assert!(
        RecoveryStore::new(root.clone())
            .verify_manifest(&manifest)
            .is_err()
    );
    assert!(
        s.record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
            .is_err()
    );
    let snapshot = s.inspect(&id).unwrap();
    assert_eq!(snapshot.state(), TransactionState::Prepared);
    assert_eq!(snapshot.approval().state, ApprovalState::Granted);
    assert_eq!(snapshot.sequence(), 3);
}

#[test]
fn interruption_with_two_recovery_links_blocks_intent_without_cleanup() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let plan = c.plan(LifecycleIntent::Archive);
    let mut store = TransactionStore::open(root.clone()).unwrap();
    let id = store.create_transaction(&plan).unwrap();
    store.prepare_recovery(&id, 0).unwrap();
    store.prepare(&id, 1).unwrap();
    store
        .grant_approval(
            &id,
            2,
            &plan,
            &c.graph,
            &evaluate_integrity(&c.graph, &c.proof),
            &c.planning,
        )
        .unwrap();
    let manifest = store.inspect(&id).unwrap().recovery().unwrap().clone();
    let final_path = t
        .path()
        .join("owned")
        .join(format!("recovery-{}.json", manifest.digest()));
    let stage_path = t.path().join("owned/recovery-stage-interrupted");
    let before = fs::read(&final_path).unwrap();
    // Exact filesystem state of an interruption after no-replace publication,
    // before unlinking the second temporary name.
    fs::hard_link(&final_path, &stage_path).unwrap();
    assert!(
        RecoveryStore::new(root.clone())
            .verify_manifest(&manifest)
            .is_err()
    );
    assert!(
        store
            .record_intent(
                &id,
                3,
                &plan,
                &c.graph,
                &evaluate_integrity(&c.graph, &c.proof),
                &c.planning
            )
            .is_err()
    );
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::Prepared
    );
    assert_eq!(
        store.inspect(&id).unwrap().approval().state,
        ApprovalState::Granted
    );
    assert_eq!(store.inspect(&id).unwrap().sequence(), 3);
    assert_eq!(fs::read(&final_path).unwrap(), before);
    assert_eq!(fs::read(&stage_path).unwrap(), before);
}
#[test]
fn inspection_does_not_initialize() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    assert!(TransactionReader::open(root).is_err());
    assert_eq!(fs::read_dir(t.path().join("owned")).unwrap().count(), 0);
}

#[test]
fn sqlite_is_append_only_and_schema_tampering_fails_closed() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let mut store = TransactionStore::open(root.clone()).unwrap();
    store
        .create_transaction(&c.plan(LifecycleIntent::Archive))
        .unwrap();
    drop(store);
    let db = rusqlite::Connection::open(t.path().join("owned/transactions.sqlite3")).unwrap();
    assert!(db.execute("UPDATE events SET sequence = 7", []).is_err());
    assert!(db.execute("DELETE FROM events", []).is_err());
    assert!(db.execute("INSERT INTO events(transaction_id, sequence, record) SELECT transaction_id, sequence, record FROM events", []).is_err());
    db.execute_batch("DROP TRIGGER events_no_update").unwrap();
    drop(db);
    assert!(TransactionStore::open(root).is_err());
}
#[test]
fn stale_plan_and_wrong_receipt_are_rejected() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let report = evaluate_integrity(&c.graph, &c.proof);
    let plan = c.plan(LifecycleIntent::Archive);
    let mut store = TransactionStore::open(root).unwrap();
    let a = store.create_transaction(&plan).unwrap();
    let b = store.create_transaction(&plan).unwrap();
    for id in [&a, &b] {
        store.prepare_recovery(id, 0).unwrap();
        store.prepare(id, 1).unwrap();
    }
    let changed = common::case(1);
    let changed_report = evaluate_integrity(&changed.graph, &changed.proof);
    assert!(
        store
            .record_intent(
                &a,
                2,
                &plan,
                &changed.graph,
                &changed_report,
                &changed.planning
            )
            .is_err()
    );
    for id in [&a, &b] {
        store
            .grant_approval(id, 2, &plan, &c.graph, &report, &c.planning)
            .unwrap();
    }
    let receipt = store
        .record_intent(&a, 3, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    let _other_receipt = store
        .record_intent(&b, 3, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    assert!(
        store
            .record_observation(&b, 4, &receipt.into_observation(), Observation::Applied)
            .is_err()
    );
    assert_eq!(
        store.inspect(&b).unwrap().state(),
        TransactionState::OutcomeUnknown
    );
}

#[test]
fn unversioned_nonempty_and_wrong_journal_mode_are_never_migrated() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let db_path = t.path().join("owned/transactions.sqlite3");
    let db = rusqlite::Connection::open(&db_path).unwrap();
    db.execute_batch("CREATE TABLE unrelated (data TEXT);")
        .unwrap();
    drop(db);
    let before = fs::read(&db_path).unwrap();
    assert!(TransactionStore::open(root.clone()).is_err());
    assert_eq!(before, fs::read(&db_path).unwrap());

    fs::remove_file(db_path).unwrap();
    drop(TransactionStore::open(root.clone()).unwrap());
    let db_path = t.path().join("owned/transactions.sqlite3");
    let db = rusqlite::Connection::open(&db_path).unwrap();
    let mode: String = db
        .query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    drop(db);
    let before = fs::read(&db_path).unwrap();
    assert!(TransactionStore::open(root).is_err());
    assert_eq!(before, fs::read(&db_path).unwrap());
}

#[test]
fn approval_is_single_use_and_revocation_survives_reopen() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let c = common::case(0);
    let plan = c.plan(LifecycleIntent::Archive);
    let report = evaluate_integrity(&c.graph, &c.proof);
    let mut store = TransactionStore::open(root.clone()).unwrap();
    let id = store.create_transaction(&plan).unwrap();
    store.prepare_recovery(&id, 0).unwrap();
    store.prepare(&id, 1).unwrap();
    assert!(
        store
            .record_intent(&id, 2, &plan, &c.graph, &report, &c.planning)
            .is_err()
    );
    let grant = store
        .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    assert!(
        store
            .grant_approval(&id, 3, &plan, &c.graph, &report, &c.planning)
            .is_err()
    );
    store.revoke_approval(&id, 3, &grant.id).unwrap();
    assert!(
        store
            .record_intent(&id, 4, &plan, &c.graph, &report, &c.planning)
            .is_err()
    );
    drop(store);
    let reader = TransactionReader::open(root).unwrap();
    assert_eq!(
        reader.inspect(&id).unwrap().approval().state,
        ApprovalState::Revoked
    );
}

#[test]
fn lost_or_replaced_key_fails_closed_without_regeneration() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
    let store = TransactionStore::open(root.clone()).unwrap();
    drop(store);
    let path = t.path().join("owned/authority.key");
    let key = fs::read(&path).unwrap();
    fs::write(&path, [0u8; 32]).unwrap();
    assert!(TransactionReader::open(root.clone()).is_err());
    assert!(TransactionStore::open(root.clone()).is_err());
    fs::write(&path, &key[..12]).unwrap();
    assert!(TransactionStore::open(root.clone()).is_err());
    fs::remove_file(&path).unwrap();
    assert!(TransactionStore::open(root).is_err());
    assert!(!path.exists());
}

#[test]
fn reconciliation_requires_consumed_exact_history_and_never_reissues_intent() {
    let t = tempfile::tempdir().unwrap();
    let root = root(&t);
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
    assert!(
        store
            .record_reconciliation(&id, 3, Observation::Applied)
            .is_err()
    );
    let gate = store
        .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    drop(gate);
    assert!(
        store
            .record_reconciliation(&id, 3, Observation::Applied)
            .is_err()
    );
    assert!(
        store
            .record_reconciliation(&id, 4, Observation::Unknown)
            .is_err()
    );
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::OutcomeUnknown
    );
    store
        .record_reconciliation(&id, 4, Observation::Applied)
        .unwrap();
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::EffectObserved
    );
    assert!(
        store
            .record_reconciliation(&id, 5, Observation::Applied)
            .is_err()
    );
    assert!(
        store
            .record_intent(&id, 5, &plan, &c.graph, &report, &c.planning)
            .is_err()
    );
}
