use crate::{FixedFile, OwnedStateRoot, RecoveryStore};
use agentmender_domain::{
    ArtifactGraph, IntegrityReport, JournalRecord, LifecyclePlan, Observation, PlanValidation,
    PlanningContext, TransactionBinding, TransactionEvent, TransactionId, TransactionSnapshot,
    Verification,
};
use rusqlite::{Connection, OpenFlags, params};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    time::Duration,
};
use thiserror::Error;

const APP_ID: i64 = 0x414d3631;
const TABLE: &str = "CREATE TABLE events (position INTEGER PRIMARY KEY, transaction_id TEXT NOT NULL, sequence INTEGER NOT NULL, record TEXT NOT NULL, UNIQUE(transaction_id, sequence))";
const UPDATE_TRIGGER: &str = "CREATE TRIGGER events_no_update BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT, 'append only'); END";
const DELETE_TRIGGER: &str = "CREATE TRIGGER events_no_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT, 'append only'); END";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("owned state error: {0}")]
    Owned(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("invalid or inconsistent durable transaction knowledge")]
    Invalid,
    #[error("transaction is missing or sequence is stale")]
    Stale,
    #[error("writer is already locked")]
    Locked,
    #[error("store is poisoned after an ambiguous write")]
    Poisoned,
    #[error("fresh validation or recovery verification failed")]
    Preconditions,
    #[error("random transaction identity unavailable")]
    Random,
}
type Result<T> = std::result::Result<T, StoreError>;

/// A durable intent is evidence of ordering, never permission to execute.
#[derive(Debug)]
pub struct DurableIntent {
    id: TransactionId,
    sequence: u64,
    digest: agentmender_domain::JournalDigest,
}

pub struct TransactionStore {
    root: OwnedStateRoot,
    _lock: File,
    db: Connection,
    journal: File,
    poisoned: bool,
}
pub struct TransactionReader {
    root: OwnedStateRoot,
    _lock: File,
    db: Connection,
}

fn database_path(root: &OwnedStateRoot) -> Result<std::path::PathBuf> {
    let path = root.fixed_path(FixedFile::Database);
    let parent = path.parent().ok_or(StoreError::Invalid)?;
    Ok(std::fs::canonicalize(parent)?.join(path.file_name().ok_or(StoreError::Invalid)?))
}
#[cfg(test)]
mod fault_support {
    use super::Result;
    thread_local! { static SUPPRESSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
    pub(super) fn suspended<T>(f: impl FnOnce() -> T) -> T {
        SUPPRESSED.with(|s| {
            s.set(true);
            let result = f();
            s.set(false);
            result
        })
    }
    pub(super) fn active(boundary: &str) -> bool {
        !SUPPRESSED.with(std::cell::Cell::get)
            && std::env::var("AGENTMENDER_TEST_FAULT").ok().as_deref() == Some(boundary)
    }
    pub(super) fn fault(boundary: &str) -> Result<()> {
        if SUPPRESSED.with(std::cell::Cell::get) {
            return Ok(());
        }
        if std::env::var("AGENTMENDER_TEST_FAULT").ok().as_deref() == Some(boundary) {
            std::process::abort();
        }
        if std::env::var("AGENTMENDER_TEST_ERROR").ok().as_deref() == Some(boundary) {
            return Err(std::io::Error::other("injected fault").into());
        }
        Ok(())
    }
}
fn configure(db: &Connection, writable: bool) -> Result<()> {
    db.busy_timeout(Duration::from_secs(2))?;
    if writable {
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=EXTRA;")?;
    } else {
        db.execute_batch("PRAGMA query_only=ON;")?;
        if db.query_row("PRAGMA query_only", [], |r| r.get::<_, i64>(0))? != 1 {
            return Err(StoreError::Invalid);
        }
    }
    db.execute_batch(
        "PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY;",
    )?;
    #[cfg(target_os = "macos")]
    if writable {
        db.execute_batch("PRAGMA fullfsync=ON;")?;
    }
    let journal: String = db.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
    let fk: i64 = db.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
    let trusted: i64 = db.query_row("PRAGMA trusted_schema", [], |r| r.get(0))?;
    let temp: i64 = db.query_row("PRAGMA temp_store", [], |r| r.get(0))?;
    if journal.to_lowercase() != "delete" || fk != 1 || trusted != 0 || temp != 2 {
        return Err(StoreError::Invalid);
    }
    if writable {
        let sync: i64 = db.query_row("PRAGMA synchronous", [], |r| r.get(0))?;
        if sync != 3 {
            return Err(StoreError::Invalid);
        }
        #[cfg(target_os = "macos")]
        if db.query_row("PRAGMA fullfsync", [], |r| r.get::<_, i64>(0))? != 1 {
            return Err(StoreError::Invalid);
        }
    }
    Ok(())
}
fn schema(db: &Connection, initialize: bool) -> Result<()> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let app: i64 = db.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let objects: Vec<(String, String, String)> = {
        let mut q = db.prepare(
            "SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<_, _>>()?
    };
    if version == 0 && app == 0 && objects.is_empty() && initialize {
        db.execute_batch(&format!("BEGIN IMMEDIATE; {TABLE}; {UPDATE_TRIGGER}; {DELETE_TRIGGER}; PRAGMA application_id={APP_ID}; PRAGMA user_version=1; COMMIT;"))?;
        return schema(db, false);
    }
    if version != 1
        || app != APP_ID
        || objects
            != vec![
                ("table".into(), "events".into(), TABLE.into()),
                (
                    "trigger".into(),
                    "events_no_delete".into(),
                    DELETE_TRIGGER.into(),
                ),
                (
                    "trigger".into(),
                    "events_no_update".into(),
                    UPDATE_TRIGGER.into(),
                ),
            ]
    {
        return Err(StoreError::Invalid);
    }
    let mut check = db.prepare("PRAGMA integrity_check")?;
    let checks: Vec<String> = check
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    if checks != ["ok"]
        || db
            .prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some()
    {
        return Err(StoreError::Invalid);
    }
    Ok(())
}
fn validate(records: &[JournalRecord]) -> Result<Vec<TransactionSnapshot>> {
    let mut histories: BTreeMap<String, Vec<JournalRecord>> = BTreeMap::new();
    for r in records {
        histories
            .entry(r.transaction_id().as_str().to_owned())
            .or_default()
            .push(r.clone());
    }
    histories
        .values()
        .map(|r| TransactionSnapshot::replay(r).map_err(|_| StoreError::Invalid))
        .collect()
}
fn db_records(db: &Connection) -> Result<Vec<JournalRecord>> {
    let mut q = db.prepare(
        "SELECT position, transaction_id, sequence, record FROM events ORDER BY position",
    )?;
    let rows = q.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (position, id, seq, text) = row?;
        if position != out.len() as i64 + 1 || out.len() >= 100_000 || text.len() > 131_072 {
            return Err(StoreError::Invalid);
        }
        let record: JournalRecord = serde_json::from_str(&text).map_err(|_| StoreError::Invalid)?;
        if record.transaction_id().as_str() != id
            || i64::try_from(record.sequence()).ok() != Some(seq)
        {
            return Err(StoreError::Invalid);
        }
        out.push(record);
    }
    validate(&out)?;
    Ok(out)
}
fn external(mut file: &File) -> Result<Vec<JournalRecord>> {
    file.seek(SeekFrom::Start(0))?;
    if file.metadata()?.len() > 67_108_864 {
        return Err(StoreError::Invalid);
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if !bytes.is_empty() && bytes.last() != Some(&b'\n') {
        return Err(StoreError::Invalid);
    }
    let mut records = Vec::new();
    for line in bytes
        .strip_suffix(b"\n")
        .unwrap_or(&bytes)
        .split(|b| *b == b'\n')
    {
        if line.is_empty() {
            if bytes.is_empty() {
                break;
            }
            return Err(StoreError::Invalid);
        }
        if line.len() > 131_072 || records.len() >= 100_000 {
            return Err(StoreError::Invalid);
        }
        records.push(serde_json::from_slice(line).map_err(|_| StoreError::Invalid)?);
    }
    validate(&records)?;
    Ok(records)
}
fn agree(db: &Connection, file: &File) -> Result<Vec<JournalRecord>> {
    let a = db_records(db)?;
    let b = external(file)?;
    if a != b {
        return Err(StoreError::Invalid);
    }
    Ok(a)
}
impl TransactionStore {
    pub fn open(root: OwnedStateRoot) -> Result<Self> {
        root.revalidate()?;
        let lock = root.open_fixed(FixedFile::Lock)?;
        lock.try_lock().map_err(|_| StoreError::Locked)?;
        let journal = root.open_fixed(FixedFile::Journal)?;
        let db_file = root.open_fixed(FixedFile::Database)?;
        let new_database = db_file.metadata()?.len() == 0;
        drop(db_file);
        root.revalidate()?;
        root.validate_sqlite_sidecars()?;
        let db = Connection::open_with_flags(
            database_path(&root)?,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        if !new_database {
            schema(&db, false)?;
            let mode: String = db.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
            if !mode.eq_ignore_ascii_case("delete") {
                return Err(StoreError::Invalid);
            }
        }
        configure(&db, true)?;
        schema(&db, new_database)?;
        agree(&db, &journal)?;
        root.sync_dir()?;
        Ok(Self {
            root,
            _lock: lock,
            db,
            journal,
            poisoned: false,
        })
    }
    pub fn list(&self) -> Result<Vec<TransactionSnapshot>> {
        self.ensure()?;
        validate(&agree(&self.db, &self.journal)?)
    }
    pub fn records(&self, id: &TransactionId) -> Result<Vec<JournalRecord>> {
        self.ensure()?;
        Ok(agree(&self.db, &self.journal)?
            .into_iter()
            .filter(|r| r.transaction_id() == id)
            .collect())
    }
    pub fn inspect(&self, id: &TransactionId) -> Result<TransactionSnapshot> {
        TransactionSnapshot::replay(&self.records(id)?).map_err(|_| StoreError::Stale)
    }
    fn ensure(&self) -> Result<()> {
        if self.poisoned {
            Err(StoreError::Poisoned)
        } else {
            Ok(())
        }
    }
    fn append(&mut self, record: JournalRecord) -> Result<()> {
        self.ensure()?;
        self.root.revalidate()?;
        self.root.validate_sqlite_sidecars()?;
        let mut records = agree(&self.db, &self.journal)?;
        if let Ok(snapshot) = TransactionSnapshot::replay(
            &records
                .iter()
                .filter(|r| r.transaction_id() == record.transaction_id())
                .cloned()
                .collect::<Vec<_>>(),
        ) && let Some(manifest) = snapshot.recovery()
        {
            RecoveryStore::new(self.root.clone())
                .verify_manifest(manifest)
                .map_err(|_| StoreError::Preconditions)?;
        }
        records.push(record.clone());
        validate(&records)?;
        let encoded = serde_json::to_vec(&record).map_err(|_| StoreError::Invalid)?;
        if encoded.len() > 131_072
            || records.len() > 100_000
            || self
                .journal
                .metadata()?
                .len()
                .saturating_add(encoded.len() as u64 + 1)
                > 67_108_864
        {
            return Err(StoreError::Invalid);
        }
        let seq = i64::try_from(record.sequence()).map_err(|_| StoreError::Invalid)?;
        // From the first external write onward any failure has ambiguous durable state.
        self.poisoned = true;
        #[cfg(test)]
        fault_support::fault("before_write")?;
        #[cfg(test)]
        if fault_support::active("partial") {
            self.journal.write_all(&encoded[..encoded.len() / 2])?;
            self.journal.sync_all()?;
            fault_support::fault("partial")?;
        }
        self.journal.write_all(&encoded)?;
        self.journal.write_all(b"\n")?;
        #[cfg(test)]
        fault_support::fault("written")?;
        self.journal.sync_all()?;
        #[cfg(test)]
        fault_support::fault("external")?;
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO events(transaction_id,sequence,record) VALUES (?1,?2,?3)",
            params![
                record.transaction_id().as_str(),
                seq,
                std::str::from_utf8(&encoded).map_err(|_| StoreError::Invalid)?
            ],
        )?;
        #[cfg(test)]
        fault_support::fault("insert")?;
        tx.commit()?;
        #[cfg(test)]
        fault_support::fault("commit")?;
        self.poisoned = false;
        Ok(())
    }
    pub fn create_transaction(&mut self, plan: &LifecyclePlan) -> Result<TransactionId> {
        self.ensure()?;
        let binding = TransactionBinding::from_plan(plan).map_err(|_| StoreError::Preconditions)?;
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| StoreError::Random)?;
        let id = TransactionId::from_bytes(bytes);
        if !self.records(&id)?.is_empty() {
            return Err(StoreError::Stale);
        }
        let r = JournalRecord::new(
            id.clone(),
            0,
            binding.digest(),
            None,
            TransactionEvent::Created { binding },
        );
        self.append(r)?;
        Ok(id)
    }
    fn next(
        &self,
        id: &TransactionId,
        expected: u64,
        event: TransactionEvent,
    ) -> Result<JournalRecord> {
        let snapshot = self.inspect(id)?;
        if snapshot.sequence() != expected {
            return Err(StoreError::Stale);
        }
        snapshot.next_record(event).map_err(|_| StoreError::Invalid)
    }
    pub fn prepare_recovery(&mut self, id: &TransactionId, expected: u64) -> Result<()> {
        let s = self.inspect(id)?;
        if s.sequence() != expected {
            return Err(StoreError::Stale);
        }
        let manifest = agentmender_domain::RecoveryManifest::new(id.clone(), s.binding());
        let r = s
            .next_record(TransactionEvent::RecoveryPrepared {
                manifest: manifest.clone(),
            })
            .map_err(|_| StoreError::Invalid)?;
        let recovery = RecoveryStore::new(self.root.clone());
        self.poisoned = true;
        #[cfg(test)]
        fault_support::fault("before_recovery")?;
        let point = recovery.prepare(&manifest)?;
        #[cfg(test)]
        fault_support::fault("after_recovery")?;
        recovery.verify(&point, &manifest)?;
        self.poisoned = false;
        self.append(r)
    }
    pub fn prepare(&mut self, id: &TransactionId, expected: u64) -> Result<()> {
        let s = self.inspect(id)?;
        if s.sequence() != expected {
            return Err(StoreError::Stale);
        }
        let manifest = s.recovery().ok_or(StoreError::Preconditions)?;
        RecoveryStore::new(self.root.clone())
            .verify_manifest(manifest)
            .map_err(|_| StoreError::Preconditions)?;
        let r = s
            .next_record(TransactionEvent::Prepared)
            .map_err(|_| StoreError::Invalid)?;
        self.append(r)
    }
    pub fn record_intent(
        &mut self,
        id: &TransactionId,
        expected: u64,
        plan: &LifecyclePlan,
        graph: &ArtifactGraph,
        report: &IntegrityReport,
        context: &PlanningContext,
    ) -> Result<DurableIntent> {
        let s = self.inspect(id)?;
        if s.sequence() != expected {
            return Err(StoreError::Stale);
        }
        if plan.validate_against(graph, report, context) != PlanValidation::Current
            || TransactionBinding::from_plan(plan).map_err(|_| StoreError::Preconditions)?
                != *s.binding()
        {
            return Err(StoreError::Preconditions);
        }
        let manifest = s.recovery().ok_or(StoreError::Preconditions)?;
        RecoveryStore::new(self.root.clone())
            .verify_manifest(manifest)
            .map_err(|_| StoreError::Preconditions)?;
        let r = s
            .next_record(TransactionEvent::Intent {
                plan: plan.digest().clone(),
                state: plan.state_fingerprint().clone(),
                operation_index: 0,
            })
            .map_err(|_| StoreError::Invalid)?;
        let sequence = r.sequence();
        let digest = r.digest().clone();
        self.append(r)?;
        Ok(DurableIntent {
            id: id.clone(),
            sequence,
            digest,
        })
    }
    pub fn record_observation(
        &mut self,
        id: &TransactionId,
        expected: u64,
        receipt: &DurableIntent,
        observation: Observation,
    ) -> Result<()> {
        let snapshot = self.inspect(id)?;
        if receipt.id != *id
            || receipt.sequence != expected
            || receipt.digest != *snapshot.last_digest()
        {
            return Err(StoreError::Preconditions);
        }
        let r = self.next(id, expected, TransactionEvent::Observed { observation })?;
        self.append(r)
    }
    pub fn start_verification(&mut self, id: &TransactionId, expected: u64) -> Result<()> {
        let r = self.next(id, expected, TransactionEvent::VerificationStarted)?;
        self.append(r)
    }
    pub fn record_verification(
        &mut self,
        id: &TransactionId,
        expected: u64,
        verification: Verification,
    ) -> Result<()> {
        let r = self.next(id, expected, TransactionEvent::Verified { verification })?;
        self.append(r)
    }
}
impl TransactionReader {
    pub fn open(root: OwnedStateRoot) -> Result<Self> {
        root.revalidate()?;
        let lock = root.open_existing_fixed(FixedFile::Lock, false)?;
        lock.try_lock_shared().map_err(|_| StoreError::Locked)?;
        let journal = root.open_existing_fixed(FixedFile::Journal, false)?;
        let _db_file = root.open_existing_fixed(FixedFile::Database, false)?;
        root.validate_sqlite_sidecars()?;
        let db = Connection::open_with_flags(
            database_path(&root)?,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        configure(&db, false)?;
        schema(&db, false)?;
        agree(&db, &journal)?;
        Ok(Self {
            root,
            _lock: lock,
            db,
        })
    }
    pub fn list(&self) -> Result<Vec<TransactionSnapshot>> {
        let f = self.root.open_existing_fixed(FixedFile::Journal, false)?;
        validate(&agree(&self.db, &f)?)
    }
    pub fn records(&self, id: &TransactionId) -> Result<Vec<JournalRecord>> {
        let f = self.root.open_existing_fixed(FixedFile::Journal, false)?;
        Ok(agree(&self.db, &f)?
            .into_iter()
            .filter(|r| r.transaction_id() == id)
            .collect())
    }
    pub fn inspect(&self, id: &TransactionId) -> Result<TransactionSnapshot> {
        TransactionSnapshot::replay(&self.records(id)?).map_err(|_| StoreError::Stale)
    }
    pub fn recovery_available(&self, id: &TransactionId) -> Result<bool> {
        let s = self.inspect(id)?;
        Ok(s.recovery().is_some_and(|m| {
            RecoveryStore::new(self.root.clone())
                .verify_manifest(m)
                .is_ok()
        }))
    }
}
/// Verified external prefix is diagnostic evidence only, not a committed snapshot.
#[derive(Debug)]
pub struct ExternalPrefix {
    pub records: Vec<JournalRecord>,
    pub complete: bool,
}
impl TransactionReader {
    pub fn external_prefix(root: &OwnedStateRoot) -> Result<ExternalPrefix> {
        root.revalidate()?;
        let lock = root.open_existing_fixed(FixedFile::Lock, false)?;
        lock.try_lock_shared().map_err(|_| StoreError::Locked)?;
        let mut file = root.open_existing_fixed(FixedFile::Journal, false)?;
        if file.metadata()?.len() > 67_108_864 {
            return Err(StoreError::Invalid);
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let mut records = Vec::new();
        let mut snapshots: BTreeMap<String, TransactionSnapshot> = BTreeMap::new();
        let mut complete = true;
        for line in bytes.split_inclusive(|b| *b == b'\n') {
            let Some(body) = line.strip_suffix(b"\n") else {
                complete = false;
                break;
            };
            if body.len() > 131_072 || records.len() >= 100_000 {
                complete = false;
                break;
            }
            let Ok(record) = serde_json::from_slice::<JournalRecord>(body) else {
                complete = false;
                break;
            };
            let key = record.transaction_id().as_str().to_owned();
            let next = match snapshots.get(&key) {
                Some(snapshot) => snapshot.advanced(&record),
                None => TransactionSnapshot::replay(std::slice::from_ref(&record)),
            };
            let Ok(next) = next else {
                complete = false;
                break;
            };
            snapshots.insert(key, next);
            records.push(record);
        }
        Ok(ExternalPrefix { records, complete })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use agentmender_domain::LifecycleIntent;
    use std::{path::PathBuf, process::Command};

    // This stands in for a future effect; it exists only in this test module.
    // The by-value receipt proves the effect cannot be entered by this harness
    // until the real durable-intent method has returned.
    fn synthetic_effect(
        store: &TransactionStore,
        receipt: DurableIntent,
        marker: &std::path::Path,
    ) -> DurableIntent {
        let sql = db_records(&store.db).unwrap();
        let journal = external(&store.journal).unwrap();
        let matches_receipt = |records: &[JournalRecord]| {
            records.iter().any(|r| {
                r.transaction_id() == &receipt.id
                    && r.sequence() == receipt.sequence
                    && r.digest() == &receipt.digest
                    && matches!(r.event(), TransactionEvent::Intent { .. })
            })
        };
        assert!(matches_receipt(&sql));
        assert!(matches_receipt(&journal));
        fault_support::fault("during_fake_effect").unwrap();
        std::fs::write(marker, b"synthetic effect observed").unwrap();
        fault_support::fault("after_fake_effect").unwrap();
        receipt
    }
    #[test]
    fn crash_child() {
        let Ok(path) = std::env::var("AGENTMENDER_TEST_ROOT") else {
            return;
        };
        let path = PathBuf::from(path);
        let root = OwnedStateRoot::open(&path, &[path.parent().unwrap().join("provider")]).unwrap();
        if let Ok(expect) = std::env::var("AGENTMENDER_TEST_LOCK") {
            if expect == "locked" {
                assert!(matches!(
                    TransactionStore::open(root.clone()),
                    Err(StoreError::Locked)
                ));
                assert!(matches!(
                    TransactionReader::open(root),
                    Err(StoreError::Locked)
                ));
            } else {
                drop(TransactionStore::open(root.clone()).unwrap());
                drop(TransactionReader::open(root).unwrap());
            }
            return;
        }
        let mut store = TransactionStore::open(root).unwrap();
        let c = crate::test_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Archive);
        if let Ok(id) = std::env::var("AGENTMENDER_TEST_FAKE_EFFECT") {
            let id: TransactionId = id.parse().unwrap();
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            let receipt = store
                .record_intent(&id, 2, &plan, &c.graph, &report, &c.planning)
                .unwrap();
            let marker = path.parent().unwrap().join("fake-effect-marker");
            let receipt = synthetic_effect(&store, receipt, &marker);
            store
                .record_observation(&id, 3, &receipt, Observation::Applied)
                .unwrap();
            return;
        }
        if let Ok(stage) = std::env::var("AGENTMENDER_TEST_STAGE") {
            let id: TransactionId = std::env::var("AGENTMENDER_TEST_ID")
                .unwrap()
                .parse()
                .unwrap();
            let snap = store.inspect(&id).unwrap();
            let seq = snap.sequence();
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            match stage.as_str() {
                "recovery" => {
                    let _ = store.prepare_recovery(&id, seq);
                }
                "prepared" => {
                    let _ = store.prepare(&id, seq);
                }
                "intent" => {
                    let _ = store.record_intent(&id, seq, &plan, &c.graph, &report, &c.planning);
                }
                "observed" => {
                    let receipt = fault_support::suspended(|| {
                        store
                            .record_intent(&id, seq, &plan, &c.graph, &report, &c.planning)
                            .unwrap()
                    });
                    let _ = store.record_observation(&id, seq + 1, &receipt, Observation::Applied);
                }
                "verification_started" => {
                    let _ = store.start_verification(&id, seq);
                }
                "verified" => {
                    let _ = store.record_verification(&id, seq, Verification::Passed);
                }
                "compensation" => {
                    let r = snap
                        .next_record(TransactionEvent::CompensationIntent {
                            plan: plan.digest().clone(),
                            state: plan.state_fingerprint().clone(),
                            operation_index: 0,
                        })
                        .unwrap();
                    let _ = store.append(r);
                }
                _ => panic!("unknown stage"),
            }
            panic!("stage fault did not abort");
        }
        if let Ok(id) = std::env::var("AGENTMENDER_TEST_INTENT") {
            let id = id.parse().unwrap();
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            let _ = store.record_intent(&id, 2, &plan, &c.graph, &report, &c.planning);
            panic!("intent fault did not abort");
        }
        if std::env::var_os("AGENTMENDER_TEST_ERROR").is_some() {
            assert!(store.create_transaction(&plan).is_err());
            assert!(matches!(store.list(), Err(StoreError::Poisoned)));
        } else {
            let _ = store.create_transaction(&plan);
            panic!("fault did not abort");
        }
    }
    #[test]
    fn crash_and_error_boundaries() {
        for (var, boundary, diverged, count) in [
            ("AGENTMENDER_TEST_FAULT", "before_write", false, 0),
            ("AGENTMENDER_TEST_FAULT", "partial", true, 0),
            ("AGENTMENDER_TEST_FAULT", "written", true, 1),
            ("AGENTMENDER_TEST_FAULT", "external", true, 1),
            ("AGENTMENDER_TEST_FAULT", "insert", true, 1),
            ("AGENTMENDER_TEST_FAULT", "commit", false, 1),
            ("AGENTMENDER_TEST_ERROR", "external", true, 1),
            ("AGENTMENDER_TEST_ERROR", "commit", false, 1),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("owned");
            let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
            drop(TransactionStore::open(root.clone()).unwrap());
            let status = Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("store::tests::crash_child")
                .arg("--nocapture")
                .env("AGENTMENDER_TEST_ROOT", &path)
                .env(var, boundary)
                .status()
                .unwrap();
            if var == "AGENTMENDER_TEST_FAULT" {
                assert!(!status.success());
            } else {
                assert!(status.success());
            }
            let opened = TransactionStore::open(root.clone());
            if diverged {
                assert!(opened.is_err());
                let prefix = TransactionReader::external_prefix(&root).unwrap();
                assert_eq!(prefix.records.len(), count);
            } else {
                let s = opened.unwrap();
                assert_eq!(s.list().unwrap().len(), count);
            }
        }
    }
    #[test]
    fn crash_after_intent_commit_remains_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("owned");
        let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
        let c = crate::test_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Archive);
        let mut store = TransactionStore::open(root.clone()).unwrap();
        let id = store.create_transaction(&plan).unwrap();
        store.prepare_recovery(&id, 0).unwrap();
        store.prepare(&id, 1).unwrap();
        drop(store);
        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("store::tests::crash_child")
            .env("AGENTMENDER_TEST_ROOT", &path)
            .env("AGENTMENDER_TEST_INTENT", id.as_str())
            .env("AGENTMENDER_TEST_FAULT", "commit")
            .status()
            .unwrap();
        assert!(!status.success());
        let reopened = TransactionStore::open(root).unwrap();
        assert_eq!(
            reopened.inspect(&id).unwrap().state(),
            agentmender_domain::TransactionState::OutcomeUnknown
        );
    }
    #[test]
    fn crash_at_each_transition_boundary() {
        for stage in [
            "recovery",
            "prepared",
            "intent",
            "observed",
            "verification_started",
            "verified",
        ] {
            for boundary in [
                "before_write",
                "partial",
                "written",
                "external",
                "insert",
                "commit",
            ] {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("owned");
                let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
                let c = crate::test_fixture::case(0);
                let plan = c.plan(LifecycleIntent::Archive);
                let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
                let mut store = TransactionStore::open(root.clone()).unwrap();
                let id = store.create_transaction(&plan).unwrap();
                if stage != "recovery" {
                    store.prepare_recovery(&id, 0).unwrap();
                }
                if !["recovery", "prepared"].contains(&stage) {
                    store.prepare(&id, 1).unwrap();
                }
                if ["verification_started", "verified"].contains(&stage) {
                    let receipt = store
                        .record_intent(&id, 2, &plan, &c.graph, &report, &c.planning)
                        .unwrap();
                    if ["verification_started", "verified"].contains(&stage) {
                        store
                            .record_observation(&id, 3, &receipt, Observation::Applied)
                            .unwrap();
                    }
                    if stage == "verified" {
                        store.start_verification(&id, 4).unwrap();
                    }
                }
                let before = store.records(&id).unwrap().len() + usize::from(stage == "observed");
                drop(store);
                let status = Command::new(std::env::current_exe().unwrap())
                    .arg("--exact")
                    .arg("store::tests::crash_child")
                    .env("AGENTMENDER_TEST_ROOT", &path)
                    .env("AGENTMENDER_TEST_ID", id.as_str())
                    .env("AGENTMENDER_TEST_STAGE", stage)
                    .env("AGENTMENDER_TEST_FAULT", boundary)
                    .status()
                    .unwrap();
                assert!(!status.success());
                let reopened = TransactionStore::open(root.clone());
                if ["before_write", "commit"].contains(&boundary) {
                    let store = reopened.unwrap();
                    assert_eq!(
                        store.records(&id).unwrap().len(),
                        before + usize::from(boundary == "commit")
                    );
                    if stage == "intent" && boundary == "commit" {
                        assert_eq!(
                            store.inspect(&id).unwrap().state(),
                            agentmender_domain::TransactionState::OutcomeUnknown
                        );
                    }
                } else {
                    assert!(reopened.is_err());
                    let prefix = TransactionReader::external_prefix(&root).unwrap();
                    assert_eq!(
                        prefix.records.len(),
                        before + usize::from(boundary != "partial")
                    );
                }
            }
        }
    }
    #[test]
    fn recovery_publication_interruption_retains_prior_history() {
        for boundary in ["before_recovery", "after_recovery"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("owned");
            let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
            let c = crate::test_fixture::case(0);
            let mut store = TransactionStore::open(root.clone()).unwrap();
            let id = store
                .create_transaction(&c.plan(LifecycleIntent::Archive))
                .unwrap();
            drop(store);
            let status = Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("store::tests::crash_child")
                .env("AGENTMENDER_TEST_ROOT", &path)
                .env("AGENTMENDER_TEST_ID", id.as_str())
                .env("AGENTMENDER_TEST_STAGE", "recovery")
                .env("AGENTMENDER_TEST_FAULT", boundary)
                .status()
                .unwrap();
            assert!(!status.success());
            let mut store = TransactionStore::open(root).unwrap();
            assert_eq!(store.records(&id).unwrap().len(), 1);
            store.prepare_recovery(&id, 0).unwrap();
        }
    }
    #[test]
    fn synthetic_compensation_crash_remains_unknown() {
        use agentmender_domain::{
            CompensationContract, NativeLifecycleOperation, RecoveryManifest, TransactionState,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("owned");
        let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
        let c = crate::test_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Archive);
        let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
        let mut store = TransactionStore::open(root.clone()).unwrap();
        let id = store.create_transaction(&plan).unwrap();
        let snapshot = store.inspect(&id).unwrap();
        let manifest = RecoveryManifest::new(id.clone(), snapshot.binding()).with_compensation(
            CompensationContract::ReviewedSemantic {
                operation: NativeLifecycleOperation::UnarchiveSession,
                evidence: "a".repeat(64).parse().unwrap(),
            },
        );
        let recovery = RecoveryStore::new(root.clone());
        let point = recovery.prepare(&manifest).unwrap();
        recovery.verify(&point, &manifest).unwrap();
        let r = snapshot
            .next_record(TransactionEvent::RecoveryPrepared { manifest })
            .unwrap();
        store.append(r).unwrap();
        store.prepare(&id, 1).unwrap();
        let receipt = store
            .record_intent(&id, 2, &plan, &c.graph, &report, &c.planning)
            .unwrap();
        store
            .record_observation(&id, 3, &receipt, Observation::Applied)
            .unwrap();
        drop(store);
        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("store::tests::crash_child")
            .env("AGENTMENDER_TEST_ROOT", &path)
            .env("AGENTMENDER_TEST_ID", id.as_str())
            .env("AGENTMENDER_TEST_STAGE", "compensation")
            .env("AGENTMENDER_TEST_FAULT", "commit")
            .status()
            .unwrap();
        assert!(!status.success());
        let reopened = TransactionStore::open(root).unwrap();
        assert_eq!(
            reopened.inspect(&id).unwrap().state(),
            TransactionState::CompensationOutcomeUnknown
        );
    }
    #[test]
    fn process_lock_blocks_writer_and_reader_until_release() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("owned");
        let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
        let store = TransactionStore::open(root).unwrap();
        let child = |expect| {
            Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("store::tests::crash_child")
                .env("AGENTMENDER_TEST_ROOT", &path)
                .env("AGENTMENDER_TEST_LOCK", expect)
                .status()
                .unwrap()
        };
        assert!(child("locked").success());
        drop(store);
        assert!(child("available").success());
    }
    #[test]
    fn synthetic_effect_requires_durable_intent_and_crashes_remain_unknown() {
        for (boundary, marker_expected, observed) in [
            ("during_fake_effect", false, false),
            ("after_fake_effect", true, false),
            ("none", true, true),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("owned");
            let root = OwnedStateRoot::create(&path, &[dir.path().join("provider")]).unwrap();
            let c = crate::test_fixture::case(0);
            let mut store = TransactionStore::open(root.clone()).unwrap();
            let id = store
                .create_transaction(&c.plan(LifecycleIntent::Archive))
                .unwrap();
            store.prepare_recovery(&id, 0).unwrap();
            store.prepare(&id, 1).unwrap();
            drop(store);
            let status = Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("store::tests::crash_child")
                .env("AGENTMENDER_TEST_ROOT", &path)
                .env("AGENTMENDER_TEST_FAKE_EFFECT", id.as_str())
                .env("AGENTMENDER_TEST_FAULT", boundary)
                .status()
                .unwrap();
            assert_eq!(status.success(), observed);
            assert_eq!(
                dir.path().join("fake-effect-marker").exists(),
                marker_expected
            );
            let reopened = TransactionStore::open(root).unwrap();
            let expected = if observed {
                agentmender_domain::TransactionState::EffectObserved
            } else {
                agentmender_domain::TransactionState::OutcomeUnknown
            };
            assert_eq!(reopened.inspect(&id).unwrap().state(), expected);
            let history = reopened.records(&id).unwrap();
            assert!(matches!(
                history[3].event(),
                TransactionEvent::Intent { .. }
            ));
            if observed {
                assert!(matches!(
                    history[4].event(),
                    TransactionEvent::Observed {
                        observation: Observation::Applied
                    }
                ));
            }
        }
    }
}
