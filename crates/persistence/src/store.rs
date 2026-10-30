use crate::{FixedFile, OwnedStateRoot, RecoveryStore};
use agentmender_domain::{
    ArtifactGraph, IntegrityReport, JournalRecord, LifecyclePlan, Observation, PlanValidation,
    PlanningContext, TransactionBinding, TransactionEvent, TransactionId, TransactionSnapshot,
    Verification,
};
use hmac::{Hmac, KeyInit, Mac};
use rusqlite::{Connection, OpenFlags, params};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const APP_ID: i64 = 0x414d3631;
const TABLE: &str = "CREATE TABLE events (position INTEGER PRIMARY KEY, transaction_id TEXT NOT NULL, sequence INTEGER NOT NULL, record TEXT NOT NULL, UNIQUE(transaction_id, sequence))";
const AUTHORITY_TABLE: &str =
    "CREATE TABLE authority_metadata (id INTEGER PRIMARY KEY CHECK(id = 1), key_id TEXT NOT NULL)";
const AUTHORITY_UPDATE: &str = "CREATE TRIGGER authority_no_update BEFORE UPDATE ON authority_metadata BEGIN SELECT RAISE(ABORT, 'immutable authority'); END";
const AUTHORITY_DELETE: &str = "CREATE TRIGGER authority_no_delete BEFORE DELETE ON authority_metadata BEGIN SELECT RAISE(ABORT, 'immutable authority'); END";
const AUTHORITY_INSERT: &str = "CREATE TRIGGER authority_no_insert BEFORE INSERT ON authority_metadata BEGIN SELECT RAISE(ABORT, 'immutable authority'); END";
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
/// It cannot be forged, cloned, or reused after conversion to an observation.
///
/// ```compile_fail
/// use agentmender_persistence::DurableIntent;
/// let _gate = DurableIntent {};
/// ```
/// ```compile_fail
/// use agentmender_persistence::DurableIntent;
/// fn duplicate(gate: DurableIntent) { let _copy = gate.clone(); }
/// ```
/// ```compile_fail
/// use agentmender_persistence::DurableIntent;
/// fn reuse(gate: DurableIntent) { let _receipt = gate.into_observation(); let _again = gate.into_observation(); }
/// ```
#[derive(Debug)]
pub struct DurableIntent {
    id: TransactionId,
    sequence: u64,
    digest: agentmender_domain::JournalDigest,
    approval_id: agentmender_domain::ApprovalId,
    plan: agentmender_domain::PlanDigest,
    state: agentmender_domain::StateFingerprint,
    operation_index: u32,
    expires_at: u64,
}
/// Observation authority cannot be converted back into an execution gate.
///
/// ```compile_fail
/// use agentmender_persistence::DurableObservation;
/// let _receipt = DurableObservation {};
/// ```
/// ```compile_fail
/// use agentmender_persistence::DurableObservation;
/// fn duplicate(receipt: DurableObservation) { let _copy = receipt.clone(); }
/// ```
#[derive(Debug)]
pub struct DurableObservation {
    id: TransactionId,
    sequence: u64,
    digest: agentmender_domain::JournalDigest,
}
impl DurableIntent {
    pub fn into_observation(self) -> DurableObservation {
        DurableObservation {
            id: self.id,
            sequence: self.sequence,
            digest: self.digest,
        }
    }

    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    pub fn approval_id(&self) -> &agentmender_domain::ApprovalId {
        &self.approval_id
    }
    pub fn plan_digest(&self) -> &agentmender_domain::PlanDigest {
        &self.plan
    }
    pub fn state_fingerprint(&self) -> &agentmender_domain::StateFingerprint {
        &self.state
    }
    pub fn operation_index(&self) -> u32 {
        self.operation_index
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.id
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
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
    let mut expected: Vec<(String, String, String)> = vec![
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
    ];
    if version == 2 {
        expected.extend([
            (
                "table".into(),
                "authority_metadata".into(),
                AUTHORITY_TABLE.into(),
            ),
            (
                "trigger".into(),
                "authority_no_delete".into(),
                AUTHORITY_DELETE.into(),
            ),
            (
                "trigger".into(),
                "authority_no_insert".into(),
                AUTHORITY_INSERT.into(),
            ),
            (
                "trigger".into(),
                "authority_no_update".into(),
                AUTHORITY_UPDATE.into(),
            ),
        ]);
        expected.sort_by(|a, b| a.1.cmp(&b.1));
    }
    if !matches!(version, 1 | 2) || app != APP_ID || objects != expected {
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
fn agree(db: &Connection, file: &File, root: &OwnedStateRoot) -> Result<Vec<JournalRecord>> {
    let a = db_records(db)?;
    let b = external(file)?;
    if a != b {
        return Err(StoreError::Invalid);
    }
    authenticate(db, root, &a)?;
    Ok(a)
}
type AuthorityMac = Hmac<Sha256>;
fn key_id(key: &[u8; 32]) -> agentmender_domain::ApprovalKeyId {
    let mut hash = Sha256::new();
    hash.update(b"agentmender.authority-key.v2\0");
    hash.update(key);
    agentmender_domain::ApprovalKeyId::from_bytes(hash.finalize().into())
}
fn read_key(root: &OwnedStateRoot) -> Result<[u8; 32]> {
    let mut f = root.open_existing_fixed(FixedFile::AuthorityKey, false)?;
    let mut bytes = Vec::new();
    (&mut f).take(33).read_to_end(&mut bytes)?;
    bytes.try_into().map_err(|_| StoreError::Invalid)
}
fn preauthority_key(root: &OwnedStateRoot) -> Result<[u8; 32]> {
    match root.open_existing_fixed(FixedFile::AuthorityKey, true) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            (&mut file).take(33).read_to_end(&mut bytes)?;
            let key: [u8; 32] = bytes.try_into().map_err(|_| StoreError::Invalid)?;
            // An orphan may have survived an interrupted initialization before its
            // original file or directory sync completed. Re-establish durability
            // before its key ID can become authoritative.
            #[cfg(test)]
            fault_support::fault("orphan_before_file_sync")?;
            file.sync_all()?;
            #[cfg(test)]
            fault_support::fault("orphan_before_dir_sync")?;
            root.sync_dir()?;
            #[cfg(test)]
            fault_support::fault("orphan_after_dir_sync")?;
            Ok(key)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut key = [0; 32];
            getrandom::fill(&mut key).map_err(|_| StoreError::Random)?;
            #[cfg(test)]
            fault_support::fault("key_before_create")?;
            let mut f = root.open_child("authority.key", true, true)?;
            #[cfg(test)]
            fault_support::fault("key_created")?;
            #[cfg(test)]
            if fault_support::active("key_partial") {
                f.write_all(&key[..16])?;
                fault_support::fault("key_partial")?;
            }
            f.write_all(&key)?;
            #[cfg(test)]
            fault_support::fault("key_written")?;
            f.sync_all()?;
            #[cfg(test)]
            fault_support::fault("key_file_synced")?;
            root.sync_dir()?;
            #[cfg(test)]
            fault_support::fault("key_dir_synced")?;
            Ok(key)
        }
        Err(e) => Err(e.into()),
    }
}
fn migrate_schema(db: &Connection, root: &OwnedStateRoot) -> Result<()> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version != 1 {
        return Err(StoreError::Invalid);
    }
    let key = preauthority_key(root)?;
    #[cfg(test)]
    fault_support::fault("schema_before_begin")?;
    db.execute_batch(&format!("BEGIN IMMEDIATE; {AUTHORITY_TABLE};"))?;
    #[cfg(test)]
    fault_support::fault("schema_table")?;
    let result = (|| -> Result<()> {
        db.execute(
            "INSERT INTO authority_metadata(id,key_id) VALUES (1,?1)",
            [key_id(&key).as_str()],
        )?;
        #[cfg(test)]
        fault_support::fault("schema_metadata")?;
        db.execute_batch(&format!(
            "{AUTHORITY_UPDATE}; {AUTHORITY_DELETE}; {AUTHORITY_INSERT}; PRAGMA user_version=2;"
        ))?;
        #[cfg(test)]
        fault_support::fault("schema_version")?;
        db.execute_batch("COMMIT")?;
        #[cfg(test)]
        fault_support::fault("schema_commit")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = db.execute_batch("ROLLBACK");
    }
    result?;
    schema(db, false)
}
fn authenticate(db: &Connection, root: &OwnedStateRoot, records: &[JournalRecord]) -> Result<()> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version == 1 {
        if records.iter().any(|r| r.schema_version() != 1) {
            return Err(StoreError::Invalid);
        }
        return Ok(());
    }
    let key = read_key(root)?;
    let id = key_id(&key);
    let ids: Vec<String> = db
        .prepare("SELECT key_id FROM authority_metadata WHERE id=1")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let count: i64 = db.query_row("SELECT count(*) FROM authority_metadata", [], |r| r.get(0))?;
    if count != 1 || ids != [id.as_str()] {
        return Err(StoreError::Invalid);
    }
    for record in records {
        if let TransactionEvent::Authorization {
            action,
            authentication,
        } = record.event()
        {
            if authentication.key_id != id {
                return Err(StoreError::Invalid);
            }
            let tag = authentication.tag.as_str().as_bytes();
            let mut decoded = [0u8; 32];
            for (i, pair) in tag.as_chunks::<2>().0.iter().enumerate() {
                if i >= 32 {
                    return Err(StoreError::Invalid);
                }
                let digit = |v: u8| -> Option<u8> {
                    match v {
                        b'0'..=b'9' => Some(v - b'0'),
                        b'a'..=b'f' => Some(v - b'a' + 10),
                        _ => None,
                    }
                };
                decoded[i] = digit(pair[0]).ok_or(StoreError::Invalid)? * 16
                    + digit(pair[1]).ok_or(StoreError::Invalid)?;
            }
            if tag.len() != 64 {
                return Err(StoreError::Invalid);
            }
            let bytes = agentmender_domain::authorization_signing_bytes(
                &id,
                record.transaction_id(),
                record.sequence(),
                record.binding_digest(),
                &record.previous_digest().cloned(),
                action,
            );
            let mut mac = AuthorityMac::new_from_slice(&key).map_err(|_| StoreError::Invalid)?;
            mac.update(&bytes);
            mac.verify_slice(&decoded)
                .map_err(|_| StoreError::Invalid)?;
        }
    }
    Ok(())
}
fn now() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| StoreError::Preconditions)
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
        agree(&db, &journal, &root)?;
        if new_database {
            migrate_schema(&db, &root)?;
        }
        root.sync_dir()?;
        Ok(Self {
            root,
            _lock: lock,
            db,
            journal,
            poisoned: false,
        })
    }
    /// Explicitly migrate a validated legacy store while holding its writer lock.
    pub fn migrate_v1(root: OwnedStateRoot) -> Result<()> {
        let store = Self::open(root)?;
        let version: i64 = store
            .db
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 1 {
            return Err(StoreError::Invalid);
        }
        agree(&store.db, &store.journal, &store.root)?;
        migrate_schema(&store.db, &store.root)
    }
    pub fn list(&self) -> Result<Vec<TransactionSnapshot>> {
        self.ensure()?;
        validate(&agree(&self.db, &self.journal, &self.root)?)
    }
    pub fn records(&self, id: &TransactionId) -> Result<Vec<JournalRecord>> {
        self.ensure()?;
        Ok(agree(&self.db, &self.journal, &self.root)?
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
        let mut records = agree(&self.db, &self.journal, &self.root)?;
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
        let version: i64 = self.db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if record.schema_version() != 2 || version != 2 {
            return Err(StoreError::Preconditions);
        }
        records.push(record.clone());
        validate(&records)?;
        authenticate(&self.db, &self.root, &records)?;
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
        let r = JournalRecord::new_v2(
            id.clone(),
            0,
            binding.digest(),
            None,
            TransactionEvent::Created {
                binding,
                plan: Some(plan.clone()),
            },
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
        if s.schema_version() != 2 {
            return Err(StoreError::Preconditions);
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
    fn authorization(
        &self,
        snapshot: &TransactionSnapshot,
        action: agentmender_domain::ApprovalAction,
    ) -> Result<JournalRecord> {
        let key = read_key(&self.root)?;
        let id = key_id(&key);
        let sequence = snapshot
            .sequence()
            .checked_add(1)
            .ok_or(StoreError::Invalid)?;
        let previous = Some(snapshot.last_digest().clone());
        let bytes = agentmender_domain::authorization_signing_bytes(
            &id,
            snapshot.id(),
            sequence,
            &snapshot.binding().digest(),
            &previous,
            &action,
        );
        let mut mac = AuthorityMac::new_from_slice(&key).map_err(|_| StoreError::Invalid)?;
        mac.update(&bytes);
        let tag =
            agentmender_domain::AuthenticationTag::from_bytes(mac.finalize().into_bytes().into());
        snapshot
            .next_record(TransactionEvent::Authorization {
                action,
                authentication: agentmender_domain::AuthorizationAuthentication { key_id: id, tag },
            })
            .map_err(|_| StoreError::Invalid)
    }
    pub fn grant_approval(
        &mut self,
        id: &TransactionId,
        expected: u64,
        plan: &LifecyclePlan,
        graph: &ArtifactGraph,
        report: &IntegrityReport,
        context: &PlanningContext,
    ) -> Result<agentmender_domain::ApprovalGrant> {
        use agentmender_domain::{
            ApprovalAction, ApprovalDirection, ApprovalGrant, ApprovalId, ApprovalNonce,
            ApprovalPrincipal, ApprovalScope, approval_eligible,
        };
        let snapshot = self.inspect(id)?;
        if snapshot.sequence() != expected {
            return Err(StoreError::Stale);
        }
        let manifest = snapshot.recovery().ok_or(StoreError::Preconditions)?;
        let verified = RecoveryStore::new(self.root.clone())
            .verify_manifest(manifest)
            .is_ok();
        if snapshot.review_plan() != Some(plan)
            || !approval_eligible(&snapshot, graph, report, context, verified)
        {
            return Err(StoreError::Preconditions);
        }
        let mut identity = [0; 32];
        let mut nonce = [0; 32];
        getrandom::fill(&mut identity).map_err(|_| StoreError::Random)?;
        getrandom::fill(&mut nonce).map_err(|_| StoreError::Random)?;
        let issued_at = now()?;
        let grant = ApprovalGrant {
            id: ApprovalId::from_bytes(identity),
            nonce: ApprovalNonce::from_bytes(nonce),
            scope: ApprovalScope {
                transaction_id: id.clone(),
                binding: snapshot.binding().clone(),
                recovery_digest: manifest.digest().clone(),
                operation_index: 0,
                direction: ApprovalDirection::ForwardOnly,
            },
            principal: ApprovalPrincipal::LocalInteractiveUser,
            issued_at,
            expires_at: issued_at
                .checked_add(300)
                .ok_or(StoreError::Preconditions)?,
        };
        let record = self.authorization(
            &snapshot,
            ApprovalAction::Granted {
                grant: grant.clone(),
            },
        )?;
        self.append(record)?;
        Ok(grant)
    }
    pub fn revoke_approval(
        &mut self,
        id: &TransactionId,
        expected: u64,
        approval_id: &agentmender_domain::ApprovalId,
    ) -> Result<()> {
        let snapshot = self.inspect(id)?;
        if snapshot.sequence() != expected {
            return Err(StoreError::Stale);
        }
        let record = self.authorization(
            &snapshot,
            agentmender_domain::ApprovalAction::Revoked {
                approval_id: approval_id.clone(),
                at: now()?,
            },
        )?;
        self.append(record)
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
        let grant = s
            .approval()
            .grant
            .as_ref()
            .ok_or(StoreError::Preconditions)?;
        let at = now()?;
        if !agentmender_domain::approval_valid(&s, grant, at) || s.review_plan() != Some(plan) {
            return Err(StoreError::Preconditions);
        }
        let r = self.authorization(
            &s,
            agentmender_domain::ApprovalAction::Consumed {
                approval_id: grant.id.clone(),
                at,
                plan: plan.digest().clone(),
                state: plan.state_fingerprint().clone(),
                operation_index: 0,
            },
        )?;
        let approval_id = grant.id.clone();
        let expires_at = grant.expires_at;
        let sequence = r.sequence();
        let digest = r.digest().clone();
        self.append(r)?;
        Ok(DurableIntent {
            id: id.clone(),
            sequence,
            digest,
            approval_id,
            plan: plan.digest().clone(),
            state: plan.state_fingerprint().clone(),
            operation_index: 0,
            expires_at,
        })
    }
    pub fn record_observation(
        &mut self,
        id: &TransactionId,
        expected: u64,
        receipt: &DurableObservation,
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
    /// Authenticate consumed v2 history and recovery before observation-only work.
    pub fn inspect_consumed(&self, id: &TransactionId) -> Result<TransactionSnapshot> {
        let snapshot = self.inspect(id)?;
        if snapshot.schema_version() != 2
            || !matches!(
                snapshot.state(),
                agentmender_domain::TransactionState::OutcomeUnknown
                    | agentmender_domain::TransactionState::EffectObserved
                    | agentmender_domain::TransactionState::Verifying
            )
            || snapshot.approval().state != agentmender_domain::ApprovalState::Consumed
            || snapshot.approval().grant.is_none()
        {
            return Err(StoreError::Preconditions);
        }
        let plan = snapshot.review_plan().ok_or(StoreError::Preconditions)?;
        if TransactionBinding::from_plan(plan).map_err(|_| StoreError::Preconditions)?
            != *snapshot.binding()
        {
            return Err(StoreError::Preconditions);
        }
        let manifest = snapshot.recovery().ok_or(StoreError::Preconditions)?;
        RecoveryStore::new(self.root.clone())
            .verify_manifest(manifest)
            .map_err(|_| StoreError::Preconditions)?;
        Ok(snapshot)
    }
    /// Record fresh, decisive read-only evidence without reissuing an effect gate.
    pub fn record_reconciliation(
        &mut self,
        id: &TransactionId,
        expected: u64,
        observation: Observation,
    ) -> Result<()> {
        let snapshot = self.inspect_consumed(id)?;
        if snapshot.sequence() != expected {
            return Err(StoreError::Stale);
        }
        if snapshot.state() != agentmender_domain::TransactionState::OutcomeUnknown
            || snapshot.approval().state != agentmender_domain::ApprovalState::Consumed
            || snapshot.approval().grant.is_none()
            || snapshot.review_plan().is_none()
            || observation != Observation::Applied
        {
            return Err(StoreError::Preconditions);
        }
        let plan = snapshot.review_plan().ok_or(StoreError::Preconditions)?;
        if TransactionBinding::from_plan(plan).map_err(|_| StoreError::Preconditions)?
            != *snapshot.binding()
        {
            return Err(StoreError::Preconditions);
        }
        let manifest = snapshot.recovery().ok_or(StoreError::Preconditions)?;
        RecoveryStore::new(self.root.clone())
            .verify_manifest(manifest)
            .map_err(|_| StoreError::Preconditions)?;
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
        agree(&db, &journal, &root)?;
        Ok(Self {
            root,
            _lock: lock,
            db,
        })
    }
    pub fn list(&self) -> Result<Vec<TransactionSnapshot>> {
        let f = self.root.open_existing_fixed(FixedFile::Journal, false)?;
        validate(&agree(&self.db, &f, &self.root)?)
    }
    pub fn records(&self, id: &TransactionId) -> Result<Vec<JournalRecord>> {
        let f = self.root.open_existing_fixed(FixedFile::Journal, false)?;
        Ok(agree(&self.db, &f, &self.root)?
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
                    && matches!(
                        r.event(),
                        TransactionEvent::Authorization {
                            action: agentmender_domain::ApprovalAction::Consumed { .. },
                            ..
                        }
                    )
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
        if let Ok(action) = std::env::var("AGENTMENDER_TEST_RACE") {
            let mut start = [0; 1];
            std::io::stdin().read_exact(&mut start).unwrap();
            let mut store = match TransactionStore::open(root) {
                Ok(store) => store,
                Err(StoreError::Locked) => std::process::exit(2),
                Err(e) => panic!("race open: {e}"),
            };
            let id: TransactionId = std::env::var("AGENTMENDER_TEST_ID")
                .unwrap()
                .parse()
                .unwrap();
            let c = crate::test_fixture::case(0);
            let plan = c.plan(LifecycleIntent::Archive);
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            let result = match action.as_str() {
                "grant" => store
                    .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
                    .map(|_| ()),
                "consume" => store
                    .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
                    .map(|_| ()),
                "revoke" => {
                    let snapshot = store.inspect(&id).unwrap();
                    let grant = snapshot.approval().grant.as_ref().unwrap();
                    store.revoke_approval(&id, 3, &grant.id).map(|_| ())
                }
                _ => panic!("unknown race action"),
            };
            match result {
                Ok(()) => std::process::exit(0),
                Err(StoreError::Stale) => std::process::exit(2),
                Err(e) => panic!("unexpected race result: {e}"),
            }
        }
        if std::env::var_os("AGENTMENDER_TEST_MIGRATE").is_some() {
            let _ = TransactionStore::migrate_v1(root);
            panic!("migration fault did not abort");
        }
        let mut store = TransactionStore::open(root).unwrap();
        let c = crate::test_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Archive);
        if let Ok(id) = std::env::var("AGENTMENDER_TEST_FAKE_EFFECT") {
            let id: TransactionId = id.parse().unwrap();
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            let receipt = store
                .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
                .unwrap();
            let marker = path.parent().unwrap().join("fake-effect-marker");
            let receipt = synthetic_effect(&store, receipt, &marker);
            store
                .record_observation(&id, 4, &receipt.into_observation(), Observation::Applied)
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
                "grant" => {
                    let _ = store.grant_approval(&id, seq, &plan, &c.graph, &report, &c.planning);
                }
                "revoke" => {
                    let grant = snap.approval().grant.as_ref().unwrap();
                    let _ = store.revoke_approval(&id, seq, &grant.id);
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
                    let _ = store.record_observation(
                        &id,
                        seq + 1,
                        &receipt.into_observation(),
                        Observation::Applied,
                    );
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
            let _ = store.record_intent(&id, 3, &plan, &c.graph, &report, &c.planning);
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
    fn after_child_exit<T>(mut operation: impl FnMut() -> Result<T>) -> Result<T> {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            match operation() {
                Err(StoreError::Locked) => {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(StoreError::Locked);
                    }
                    std::thread::sleep(remaining.min(Duration::from_millis(10)));
                }
                result => return result,
            }
        }
    }
    // A crash can leave a rollback journal whose owner fails the owned-root
    // capability checks. Assert the protected state without opening SQLite.
    #[cfg(windows)]
    fn assert_protected_sidecar(
        root: &OwnedStateRoot,
        prefix_count: usize,
        expected: Option<&[JournalRecord]>,
        key_expected: bool,
        context: &str,
    ) -> bool {
        use std::{collections::BTreeMap, os::windows::fs::OpenOptionsExt};
        use windows_permissions::{
            constants::{SeObjectType, SecurityInformation},
            utilities::current_process_sid,
            wrappers::{
                ConvertSecurityDescriptorToStringSecurityDescriptor, EqualSid, GetNamedSecurityInfo,
            },
        };
        let sidecar = root.fixed_path(FixedFile::SqliteRollbackJournal);
        let metadata = match std::fs::symlink_metadata(&sidecar) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
            Err(e) => panic!("{context}: sidecar metadata: {e}"),
        };
        assert!(
            metadata.is_file(),
            "{context}: sidecar is not a regular file"
        );
        let identity = || {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(0x00200000)
                .open(&sidecar)
                .unwrap_or_else(|e| panic!("{context}: sidecar open: {e}"));
            let info = winapi_util::file::information(&file)
                .unwrap_or_else(|e| panic!("{context}: sidecar identity: {e}"));
            assert_eq!(
                info.file_attributes() & 0x400,
                0,
                "{context}: reparse sidecar"
            );
            assert_eq!(info.number_of_links(), 1, "{context}: linked sidecar");
            (
                info.volume_serial_number(),
                info.file_index(),
                info.number_of_links(),
                info.file_attributes(),
            )
        };
        let _ = identity();
        let security = || {
            let sd = GetNamedSecurityInfo(
                &sidecar,
                SeObjectType::SE_FILE_OBJECT,
                SecurityInformation::Owner | SecurityInformation::Dacl,
            )
            .unwrap_or_else(|e| panic!("{context}: sidecar security: {e}"));
            let owner = sd
                .owner()
                .unwrap_or_else(|| panic!("{context}: missing owner"));
            let current =
                current_process_sid().unwrap_or_else(|e| panic!("{context}: current SID: {e}"));
            let is_current = EqualSid(owner, &current);
            let descriptor = ConvertSecurityDescriptorToStringSecurityDescriptor(
                &sd,
                SecurityInformation::Owner | SecurityInformation::Dacl,
            )
            .unwrap_or_else(|e| panic!("{context}: sidecar descriptor: {e}"));
            (is_current, descriptor)
        };
        if security().0 {
            return false;
        }
        root.revalidate()
            .unwrap_or_else(|e| panic!("{context}: root: {e}"));
        for fixed in [FixedFile::Database, FixedFile::Journal, FixedFile::Lock] {
            root.open_existing_fixed(fixed, false)
                .unwrap_or_else(|e| panic!("{context}: required fixed file: {e}"));
        }
        match root.open_existing_fixed(FixedFile::AuthorityKey, false) {
            Ok(_) if key_expected => {}
            Err(e) if !key_expected && e.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => panic!("{context}: unexpected authority key"),
            Err(e) => panic!("{context}: authority key: {e}"),
        }
        for fixed in [FixedFile::SqliteWal, FixedFile::SqliteShm] {
            match std::fs::symlink_metadata(root.fixed_path(fixed)) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => panic!("{context}: sidecar metadata: {e}"),
                Ok(_) => panic!("{context}: unexpected WAL or SHM"),
            }
        }
        let owned_error = root
            .open_existing_fixed(FixedFile::SqliteRollbackJournal, false)
            .unwrap_err();
        assert_eq!(
            owned_error.kind(),
            std::io::ErrorKind::PermissionDenied,
            "{context}"
        );
        assert_eq!(
            owned_error.to_string(),
            "Windows object owner differs from current user",
            "{context}"
        );
        let snapshot = || {
            let mut entries = BTreeMap::new();
            for entry in std::fs::read_dir(sidecar.parent().unwrap()).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let metadata = std::fs::symlink_metadata(&path).unwrap();
                assert!(metadata.is_file(), "{context}: unexpected entry class");
                entries.insert(entry.file_name(), std::fs::read(path).unwrap());
            }
            (entries, identity(), security().1)
        };
        let before = snapshot();
        let journal_before = std::fs::read(root.fixed_path(FixedFile::Journal)).unwrap();
        let denied = |result: Result<()>| match result {
            Err(StoreError::Io(e))
                if e.kind() == owned_error.kind() && e.to_string() == owned_error.to_string() => {}
            Err(e) => panic!("{context}: expected PermissionDenied, got {e}"),
            Ok(()) => panic!("{context}: expected PermissionDenied"),
        };
        denied(after_child_exit(|| {
            TransactionStore::open(root.clone()).map(|_| ())
        }));
        assert_eq!(snapshot(), before, "{context}: store open changed evidence");
        denied(after_child_exit(|| {
            TransactionReader::open(root.clone()).map(|_| ())
        }));
        assert_eq!(
            snapshot(),
            before,
            "{context}: reader open changed evidence"
        );
        denied(after_child_exit(|| {
            TransactionStore::migrate_v1(root.clone())
        }));
        assert_eq!(snapshot(), before, "{context}: migration changed evidence");
        let sidecar_error = root.validate_sqlite_sidecars().unwrap_err();
        assert_eq!(sidecar_error.kind(), owned_error.kind(), "{context}");
        assert_eq!(
            sidecar_error.to_string(),
            owned_error.to_string(),
            "{context}"
        );
        assert_eq!(
            snapshot(),
            before,
            "{context}: sidecar validation changed evidence"
        );
        let prefix = after_child_exit(|| TransactionReader::external_prefix(root))
            .unwrap_or_else(|e| panic!("{context}: prefix: {e}"));
        assert_eq!(prefix.records.len(), prefix_count, "{context}");
        if let Some(expected) = expected {
            assert_eq!(prefix.records, expected, "{context}");
        }
        assert_eq!(
            std::fs::read(root.fixed_path(FixedFile::Journal)).unwrap(),
            journal_before,
            "{context}"
        );
        assert_eq!(snapshot(), before, "{context}: prefix changed evidence");
        true
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
            ("AGENTMENDER_TEST_ERROR", "before_write", false, 0),
            ("AGENTMENDER_TEST_ERROR", "written", true, 1),
            ("AGENTMENDER_TEST_ERROR", "insert", true, 1),
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
            #[cfg(windows)]
            if assert_protected_sidecar(&root, count, None, true, &format!("{var} {boundary}")) {
                continue;
            }
            let opened = after_child_exit(|| TransactionStore::open(root.clone()));
            if diverged {
                match opened {
                    Err(StoreError::Invalid) => {}
                    Err(e) => panic!("{var} {boundary}: expected invalid divergence, got {e}"),
                    Ok(_) => panic!(
                        "{var} {boundary}: expected invalid divergence, reopened successfully"
                    ),
                }
                let prefix = after_child_exit(|| TransactionReader::external_prefix(&root))
                    .unwrap_or_else(|e| panic!("{var} {boundary}: external prefix: {e}"));
                assert_eq!(prefix.records.len(), count, "{var} {boundary}");
            } else {
                let s = opened.unwrap_or_else(|e| panic!("{var} {boundary}: reopen: {e}"));
                assert_eq!(s.list().unwrap().len(), count, "{var} {boundary}");
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
        let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
        store
            .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
            .unwrap();
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
            "grant",
            "revoke",
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
                if !["recovery", "prepared", "grant"].contains(&stage) {
                    store
                        .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
                        .unwrap();
                }
                if ["verification_started", "verified"].contains(&stage) {
                    let receipt = store
                        .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
                        .unwrap();
                    if ["verification_started", "verified"].contains(&stage) {
                        store
                            .record_observation(
                                &id,
                                4,
                                &receipt.into_observation(),
                                Observation::Applied,
                            )
                            .unwrap();
                    }
                    if stage == "verified" {
                        store.start_verification(&id, 5).unwrap();
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
                let reopened = after_child_exit(|| TransactionStore::open(root.clone()));
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
                    let prefix =
                        after_child_exit(|| TransactionReader::external_prefix(&root)).unwrap();
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
    fn unsigned_compensation_is_rejected_for_v2() {
        let dir = tempfile::tempdir().unwrap();
        let root =
            OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                .unwrap();
        let c = crate::test_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Archive);
        let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
        let mut store = TransactionStore::open(root).unwrap();
        let id = store.create_transaction(&plan).unwrap();
        store.prepare_recovery(&id, 0).unwrap();
        store.prepare(&id, 1).unwrap();
        store
            .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
            .unwrap();
        let receipt = store
            .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
            .unwrap();
        store
            .record_observation(&id, 4, &receipt.into_observation(), Observation::Applied)
            .unwrap();
        let snapshot = store.inspect(&id).unwrap();
        assert!(
            snapshot
                .next_record(TransactionEvent::CompensationIntent {
                    plan: plan.digest().clone(),
                    state: plan.state_fingerprint().clone(),
                    operation_index: 0
                })
                .is_err()
        );
    }
    #[test]
    fn explicit_v1_migration_preserves_history_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let root =
            OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                .unwrap();
        let mut journal = root.open_fixed(FixedFile::Journal).unwrap();
        drop(root.open_fixed(FixedFile::Lock).unwrap());
        drop(root.open_fixed(FixedFile::Database).unwrap());
        let db = Connection::open(database_path(&root).unwrap()).unwrap();
        configure(&db, true).unwrap();
        schema(&db, true).unwrap();
        let plan = crate::test_fixture::case(0).plan(LifecycleIntent::Archive);
        let binding = TransactionBinding::from_plan(&plan).unwrap();
        let id = TransactionId::from_bytes([7; 32]);
        let record = JournalRecord::new(
            id.clone(),
            0,
            binding.digest(),
            None,
            TransactionEvent::Created {
                binding,
                plan: None,
            },
        );
        let text = serde_json::to_string(&record).unwrap();
        journal.write_all(text.as_bytes()).unwrap();
        journal.write_all(b"\n").unwrap();
        journal.sync_all().unwrap();
        db.execute(
            "INSERT INTO events(transaction_id,sequence,record) VALUES (?1,0,?2)",
            params![id.as_str(), text],
        )
        .unwrap();
        drop(db);
        drop(journal);
        let path = root.fixed_path(FixedFile::Journal);
        let before = std::fs::read(&path).unwrap();
        let reader = TransactionReader::open(root.clone()).unwrap();
        assert_eq!(reader.inspect(&id).unwrap().schema_version(), 1);
        drop(reader);
        assert!(!root.fixed_path(FixedFile::AuthorityKey).exists());
        TransactionStore::migrate_v1(root.clone()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut store = TransactionStore::open(root).unwrap();
        assert_eq!(store.inspect(&id).unwrap().schema_version(), 1);
        assert!(store.prepare_recovery(&id, 0).is_err());
    }
    #[test]
    fn competing_processes_serialize_authorization() {
        use std::process::Stdio;
        for actions in [
            ["grant", "grant"],
            ["consume", "consume"],
            ["revoke", "consume"],
        ] {
            let dir = tempfile::tempdir().unwrap();
            let root =
                OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                    .unwrap();
            let c = crate::test_fixture::case(0);
            let plan = c.plan(LifecycleIntent::Archive);
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            let mut store = TransactionStore::open(root.clone()).unwrap();
            let id = store.create_transaction(&plan).unwrap();
            store.prepare_recovery(&id, 0).unwrap();
            store.prepare(&id, 1).unwrap();
            if actions[0] != "grant" {
                store
                    .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
                    .unwrap();
            }
            assert!(matches!(
                TransactionReader::open(root.clone()),
                Err(StoreError::Locked)
            ));
            drop(store);
            let mut children: Vec<_> = actions
                .iter()
                .map(|action| {
                    Command::new(std::env::current_exe().unwrap())
                        .arg("--exact")
                        .arg("store::tests::crash_child")
                        .env("AGENTMENDER_TEST_ROOT", dir.path().join("owned"))
                        .env("AGENTMENDER_TEST_ID", id.as_str())
                        .env("AGENTMENDER_TEST_RACE", action)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .spawn()
                        .unwrap()
                })
                .collect();
            for child in &mut children {
                child.stdin.take().unwrap().write_all(b"x").unwrap();
            }
            let codes: Vec<_> = children
                .iter_mut()
                .map(|child| child.wait().unwrap().code().unwrap())
                .collect();
            assert_eq!(
                codes.iter().filter(|code| **code == 0).count(),
                1,
                "{actions:?}: {codes:?}"
            );
            assert!(codes.iter().all(|code| *code == 0 || *code == 2));
            let reader = TransactionReader::open(root.clone()).unwrap();
            let snapshot = reader.inspect(&id).unwrap();
            assert_eq!(
                snapshot.sequence(),
                if actions[0] == "grant" { 3 } else { 4 }
            );
            if actions[0] == "grant" {
                assert_eq!(
                    snapshot.approval().state,
                    agentmender_domain::ApprovalState::Granted
                );
            } else {
                assert!(matches!(
                    snapshot.approval().state,
                    agentmender_domain::ApprovalState::Consumed
                        | agentmender_domain::ApprovalState::Revoked
                ));
            }
            assert_eq!(
                db_records(&reader.db).unwrap().len(),
                snapshot.sequence() as usize + 1
            );
        }
    }
    #[test]
    fn migration_key_and_schema_crash_cuts_preserve_legacy_history() {
        for boundary in [
            "key_before_create",
            "key_created",
            "key_partial",
            "key_written",
            "key_file_synced",
            "key_dir_synced",
            "orphan_before_file_sync",
            "orphan_before_dir_sync",
            "orphan_after_dir_sync",
            "schema_before_begin",
            "schema_table",
            "schema_metadata",
            "schema_version",
            "schema_commit",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let root =
                OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                    .unwrap();
            let mut journal = root.open_fixed(FixedFile::Journal).unwrap();
            drop(root.open_fixed(FixedFile::Lock).unwrap());
            drop(root.open_fixed(FixedFile::Database).unwrap());
            let db = Connection::open(database_path(&root).unwrap()).unwrap();
            configure(&db, true).unwrap();
            schema(&db, true).unwrap();
            let plan = crate::test_fixture::case(0).plan(LifecycleIntent::Archive);
            let binding = TransactionBinding::from_plan(&plan).unwrap();
            let id = TransactionId::from_bytes([7; 32]);
            let record = JournalRecord::new(
                id.clone(),
                0,
                binding.digest(),
                None,
                TransactionEvent::Created {
                    binding,
                    plan: None,
                },
            );
            let text = serde_json::to_string(&record).unwrap();
            writeln!(journal, "{text}").unwrap();
            journal.sync_all().unwrap();
            db.execute(
                "INSERT INTO events(transaction_id,sequence,record) VALUES (?1,0,?2)",
                params![id.as_str(), &text],
            )
            .unwrap();
            drop(db);
            drop(journal);
            let path = root.fixed_path(FixedFile::Journal);
            let before = std::fs::read(&path).unwrap();
            let key_path = root.fixed_path(FixedFile::AuthorityKey);
            if boundary.starts_with("orphan_") {
                let mut orphan = root.open_child("authority.key", true, true).unwrap();
                orphan.write_all(&[9; 32]).unwrap();
            }
            let status = Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("store::tests::crash_child")
                .env("AGENTMENDER_TEST_ROOT", dir.path().join("owned"))
                .env("AGENTMENDER_TEST_MIGRATE", "1")
                .env("AGENTMENDER_TEST_FAULT", boundary)
                .status()
                .unwrap();
            assert!(!status.success(), "{boundary}");
            assert_ne!(
                status.code(),
                Some(101),
                "fault was not reached: {boundary}"
            );
            #[cfg(unix)]
            assert!(status.code().is_none(), "fault was not reached: {boundary}");
            assert_eq!(std::fs::read(&path).unwrap(), before, "{boundary}");
            #[cfg(windows)]
            if assert_protected_sidecar(
                &root,
                1,
                Some(std::slice::from_ref(&record)),
                boundary != "key_before_create",
                boundary,
            ) {
                continue;
            }
            let db = Connection::open(database_path(&root).unwrap())
                .unwrap_or_else(|e| panic!("{boundary}: raw database open: {e}"));
            let version: i64 = db
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(
                version,
                if boundary == "schema_commit" { 2 } else { 1 },
                "{boundary}"
            );
            assert_eq!(db_records(&db).unwrap(), vec![record], "{boundary}");
            let raw: String = db
                .query_row("SELECT record FROM events", [], |r| r.get(0))
                .unwrap();
            assert_eq!(raw, text, "{boundary}");
            drop(db);
            let key = std::fs::read(&key_path).ok();
            if matches!(boundary, "key_created" | "key_partial") {
                assert!(TransactionStore::migrate_v1(root.clone()).is_err());
                assert_eq!(std::fs::read(&key_path).unwrap(), key.unwrap());
            } else {
                if version == 1 {
                    TransactionStore::migrate_v1(root.clone())
                        .unwrap_or_else(|e| panic!("{boundary}: migration: {e}"));
                }
                let adopted = std::fs::read(&key_path).unwrap();
                assert_eq!(adopted.len(), 32);
                if let Some(key) = key {
                    assert_eq!(adopted, key);
                }
                let store = TransactionStore::open(root.clone())
                    .unwrap_or_else(|e| panic!("{boundary}: reopen: {e}"));
                assert_eq!(store.inspect(&id).unwrap().sequence(), 0);
                let raw: String = store
                    .db
                    .query_row("SELECT record FROM events", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(raw, text, "{boundary}");
                let anchored: String = store
                    .db
                    .query_row(
                        "SELECT key_id FROM authority_metadata WHERE id=1",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                let bytes: [u8; 32] = adopted.try_into().unwrap();
                assert_eq!(anchored, key_id(&bytes).as_str());
            }
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }
    #[test]
    fn migrated_legacy_compensation_unknown_is_preserved() {
        use agentmender_domain::{
            CompensationContract, NativeLifecycleOperation, RecoveryManifest, TransactionState,
        };
        let dir = tempfile::tempdir().unwrap();
        let root =
            OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                .unwrap();
        let mut journal = root.open_fixed(FixedFile::Journal).unwrap();
        drop(root.open_fixed(FixedFile::Lock).unwrap());
        drop(root.open_fixed(FixedFile::Database).unwrap());
        let db = Connection::open(database_path(&root).unwrap()).unwrap();
        configure(&db, true).unwrap();
        schema(&db, true).unwrap();
        let plan = crate::test_fixture::case(0).plan(LifecycleIntent::Archive);
        let binding = TransactionBinding::from_plan(&plan).unwrap();
        let id = TransactionId::from_bytes([9; 32]);
        let first = JournalRecord::new(
            id.clone(),
            0,
            binding.digest(),
            None,
            TransactionEvent::Created {
                binding: binding.clone(),
                plan: None,
            },
        );
        let mut records = vec![first];
        let manifest = RecoveryManifest::new(id.clone(), &binding).with_compensation(
            CompensationContract::ReviewedSemantic {
                operation: NativeLifecycleOperation::UnarchiveSession,
                evidence: "a".repeat(64).parse().unwrap(),
            },
        );
        for event in [
            TransactionEvent::RecoveryPrepared { manifest },
            TransactionEvent::Prepared,
            TransactionEvent::Intent {
                plan: plan.digest().clone(),
                state: plan.state_fingerprint().clone(),
                operation_index: 0,
            },
            TransactionEvent::Observed {
                observation: Observation::Applied,
            },
            TransactionEvent::CompensationIntent {
                plan: plan.digest().clone(),
                state: plan.state_fingerprint().clone(),
                operation_index: 0,
            },
        ] {
            records.push(
                TransactionSnapshot::replay(&records)
                    .unwrap()
                    .next_record(event)
                    .unwrap(),
            );
        }
        for record in &records {
            let text = serde_json::to_string(record).unwrap();
            writeln!(journal, "{text}").unwrap();
            db.execute(
                "INSERT INTO events(transaction_id,sequence,record) VALUES (?1,?2,?3)",
                params![id.as_str(), record.sequence() as i64, text],
            )
            .unwrap();
        }
        journal.sync_all().unwrap();
        drop(journal);
        drop(db);
        TransactionStore::migrate_v1(root.clone()).unwrap();
        assert_eq!(
            TransactionReader::open(root)
                .unwrap()
                .inspect(&id)
                .unwrap()
                .state(),
            TransactionState::CompensationOutcomeUnknown
        );
    }
    #[test]
    fn forged_mac_fails_even_when_hashes_and_stores_agree() {
        let dir = tempfile::tempdir().unwrap();
        let root =
            OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                .unwrap();
        let c = crate::test_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Archive);
        let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
        let mut store = TransactionStore::open(root.clone()).unwrap();
        let id = store.create_transaction(&plan).unwrap();
        store.prepare_recovery(&id, 0).unwrap();
        store.prepare(&id, 1).unwrap();
        store
            .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
            .unwrap();
        let records = store.records(&id).unwrap();
        let old = records.last().unwrap();
        let TransactionEvent::Authorization {
            action,
            authentication,
        } = old.event()
        else {
            panic!("expected authorization");
        };
        let forged = JournalRecord::new_v2(
            id.clone(),
            old.sequence(),
            old.binding_digest().clone(),
            old.previous_digest().cloned(),
            TransactionEvent::Authorization {
                action: action.clone(),
                authentication: agentmender_domain::AuthorizationAuthentication {
                    key_id: authentication.key_id.clone(),
                    tag: agentmender_domain::AuthenticationTag::from_bytes([0; 32]),
                },
            },
        );
        let text = serde_json::to_string(&forged).unwrap();
        drop(store);
        let db = Connection::open(database_path(&root).unwrap()).unwrap();
        db.execute_batch("DROP TRIGGER events_no_update").unwrap();
        db.execute(
            "UPDATE events SET record=?1 WHERE transaction_id=?2 AND sequence=3",
            params![text, id.as_str()],
        )
        .unwrap();
        db.execute_batch(UPDATE_TRIGGER).unwrap();
        drop(db);
        let mut lines: Vec<String> = records[..3]
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();
        lines.push(text);
        std::fs::write(
            root.fixed_path(FixedFile::Journal),
            format!("{}\n", lines.join("\n")),
        )
        .unwrap();
        assert!(TransactionReader::open(root.clone()).is_err());
        assert!(TransactionStore::open(root).is_err());
    }
    #[test]
    fn every_authorization_field_is_mac_bound() {
        use agentmender_domain::{
            ApprovalAction, ApprovalKeyId, AuthorizationAuthentication, BindingDigest,
            JournalDigest,
        };
        let cases = [
            ("grant", "/grant/id"),
            ("grant", "/grant/nonce"),
            ("grant", "/grant/scope/transaction_id"),
            ("grant", "/grant/scope/recovery_digest"),
            ("grant", "/grant/scope/binding/plan_digest"),
            ("grant", "/grant/scope/binding/state_fingerprint"),
            ("grant", "/grant/scope/binding/provider"),
            ("grant", "/grant/scope/binding/intent"),
            ("grant", "/grant/scope/binding/operation"),
            ("grant", "/grant/scope/binding/target"),
            ("grant", "/grant/scope/binding/native_identity"),
            ("grant", "/grant/scope/binding/profile"),
            ("grant", "/grant/scope/binding/capability_effect"),
            ("grant", "/grant/scope/binding/conditions"),
            ("grant", "/grant/scope/binding/operation_commitment"),
            ("grant", "/grant/scope/operation_index"),
            ("grant", "/grant/issued_at"),
            ("grant", "/grant/expires_at"),
            ("grant", "key_id"),
            ("grant", "transaction_id"),
            ("grant", "sequence"),
            ("grant", "previous"),
            ("grant", "binding"),
            ("revoke", "/approval_id"),
            ("revoke", "/at"),
            ("consume", "/approval_id"),
            ("consume", "/at"),
            ("consume", "/plan"),
            ("consume", "/state"),
            ("consume", "/operation_index"),
        ];
        for (stage, field) in cases {
            let dir = tempfile::tempdir().unwrap();
            let root =
                OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")])
                    .unwrap();
            let c = crate::test_fixture::case(0);
            let plan = c.plan(LifecycleIntent::Archive);
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            let mut store = TransactionStore::open(root.clone()).unwrap();
            let id = store.create_transaction(&plan).unwrap();
            store.prepare_recovery(&id, 0).unwrap();
            store.prepare(&id, 1).unwrap();
            let grant = store
                .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
                .unwrap();
            if stage == "revoke" {
                store.revoke_approval(&id, 3, &grant.id).unwrap();
            }
            if stage == "consume" {
                store
                    .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
                    .unwrap();
            }
            let records = store.records(&id).unwrap();
            let old = records.last().unwrap();
            let TransactionEvent::Authorization {
                action,
                authentication,
            } = old.event()
            else {
                panic!("expected authorization");
            };
            let mut value = serde_json::to_value(action).unwrap();
            if field.starts_with('/') {
                let slot = value.pointer_mut(field).unwrap();
                *slot = match slot {
                    serde_json::Value::String(v) if v.len() == 71 && v.starts_with("sha256:") => {
                        serde_json::Value::String(format!("sha256:{}", "f".repeat(64)))
                    }
                    serde_json::Value::String(v) if v.len() == 64 => {
                        serde_json::Value::String("f".repeat(64))
                    }
                    serde_json::Value::String(v) if field.ends_with("provider") => {
                        serde_json::Value::String(
                            if v == "codex" { "claude" } else { "codex" }.into(),
                        )
                    }
                    serde_json::Value::String(_) if field.ends_with("intent") => {
                        serde_json::Value::String("purge".into())
                    }
                    serde_json::Value::String(_) if field.ends_with("operation") => {
                        serde_json::Value::String("delete_session".into())
                    }
                    serde_json::Value::Number(n) => serde_json::json!(n.as_u64().unwrap() + 1),
                    _ => panic!("unexpected field {field}"),
                };
            }
            let changed_action: ApprovalAction = serde_json::from_value(value).unwrap();
            let changed_key = if field == "key_id" {
                ApprovalKeyId::from_bytes([42; 32])
            } else {
                authentication.key_id.clone()
            };
            let changed_id = if field == "transaction_id" {
                TransactionId::from_bytes([42; 32])
            } else {
                id.clone()
            };
            let sequence = old.sequence() + u64::from(field == "sequence");
            let previous = if field == "previous" {
                Some("f".repeat(64).parse::<JournalDigest>().unwrap())
            } else {
                old.previous_digest().cloned()
            };
            let binding = if field == "binding" {
                "f".repeat(64).parse::<BindingDigest>().unwrap()
            } else {
                old.binding_digest().clone()
            };
            let forged = JournalRecord::new_v2(
                changed_id.clone(),
                sequence,
                binding,
                previous,
                TransactionEvent::Authorization {
                    action: changed_action,
                    authentication: AuthorizationAuthentication {
                        key_id: changed_key,
                        tag: authentication.tag.clone(),
                    },
                },
            );
            assert!(
                authenticate(&store.db, &root, std::slice::from_ref(&forged)).is_err(),
                "MAC accepted {stage} {field}"
            );
            let text = serde_json::to_string(&forged).unwrap();
            let mut lines: Vec<String> = records[..records.len() - 1]
                .iter()
                .map(|r| serde_json::to_string(r).unwrap())
                .collect();
            lines.push(text.clone());
            drop(store);
            let db = Connection::open(database_path(&root).unwrap()).unwrap();
            db.execute_batch("DROP TRIGGER events_no_update").unwrap();
            db.execute(
                "UPDATE events SET transaction_id=?1, sequence=?2, record=?3 WHERE position=?4",
                params![
                    changed_id.as_str(),
                    sequence as i64,
                    text,
                    records.len() as i64
                ],
            )
            .unwrap();
            db.execute_batch(UPDATE_TRIGGER).unwrap();
            drop(db);
            std::fs::write(
                root.fixed_path(FixedFile::Journal),
                format!("{}\n", lines.join("\n")),
            )
            .unwrap();
            assert!(
                TransactionReader::open(root.clone()).is_err(),
                "reader accepted {stage} {field}"
            );
            assert!(
                TransactionStore::open(root).is_err(),
                "writer accepted {stage} {field}"
            );
        }
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
            let plan = c.plan(LifecycleIntent::Archive);
            let report = agentmender_domain::evaluate_integrity(&c.graph, &c.proof);
            store
                .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
                .unwrap();
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
                history[4].event(),
                TransactionEvent::Authorization {
                    action: agentmender_domain::ApprovalAction::Consumed { .. },
                    ..
                }
            ));
            if observed {
                assert!(matches!(
                    history[5].event(),
                    TransactionEvent::Observed {
                        observation: Observation::Applied
                    }
                ));
            }
        }
    }
}
