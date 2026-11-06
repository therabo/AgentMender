//! Exact Desktop catalog transaction. The caller must hold the reviewed physical
//! boundary and consumed durable intent before opening the connection or fencing it.
use agentmender_domain::{CleanupFileIdentity, DesktopCatalogCommitment};
use agentmender_persistence::EffectError;
use rusqlite::{Connection, OptionalExtension, params, types::Value};
use sha2::{Digest, Sha256};

const SCHEMA: &str = include_str!(
    "../../../compatibility/codex/desktop-complete-removal-v1/catalog/complete-schema.canonical.json"
);
const MIGRATIONS: &str = include_str!(
    "../../../compatibility/codex/desktop-complete-removal-v1/catalog/ordered-migrations.json"
);
const SCHEMA_HASH: &str = "a275a36b59fa4895e0793eb1011d7c2b2b7ee25691c2f2bf206ff2f4342cc40a";
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_ROWS: usize = 100_000;
const MAX_SAFE: i64 = 9_007_199_254_740_991;
const TABLES: [(&str, &str); 14] = [
    ("active_chat_turns", "SELECT * FROM active_chat_turns"),
    ("automation_runs", "SELECT * FROM automation_runs"),
    ("automations", "SELECT * FROM automations"),
    (
        "codex_schema_migrations",
        "SELECT * FROM codex_schema_migrations",
    ),
    ("inbox_items", "SELECT * FROM inbox_items"),
    (
        "live_visualization_suggestions",
        "SELECT * FROM live_visualization_suggestions",
    ),
    (
        "local_app_server_feature_enablement",
        "SELECT * FROM local_app_server_feature_enablement",
    ),
    ("local_thread_catalog", "SELECT * FROM local_thread_catalog"),
    (
        "local_thread_catalog_hosts",
        "SELECT * FROM local_thread_catalog_hosts",
    ),
    (
        "local_thread_catalog_metadata",
        "SELECT * FROM local_thread_catalog_metadata",
    ),
    (
        "local_thread_catalog_scan_checkpoints",
        "SELECT * FROM local_thread_catalog_scan_checkpoints",
    ),
    (
        "local_thread_catalog_scan_entries",
        "SELECT * FROM local_thread_catalog_scan_entries",
    ),
    (
        "local_thread_catalog_sync_state",
        "SELECT * FROM local_thread_catalog_sync_state",
    ),
    (
        "thread_timeline_ledger",
        "SELECT * FROM thread_timeline_ledger",
    ),
];
fn invalid() -> EffectError {
    EffectError::InvalidEvidence
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn text(v: &Value, expected: &str) -> bool {
    matches!(v, Value::Text(s) if s == expected)
}
fn integer(v: &Value) -> Result<i64, EffectError> {
    match v {
        Value::Integer(n) if (0..MAX_SAFE).contains(n) => Ok(*n),
        _ => Err(invalid()),
    }
}

#[derive(Clone)]
struct Table {
    name: &'static str,
    rows: Vec<Vec<Value>>,
}
fn encode_value(out: &mut Vec<u8>, v: &Value) {
    let (tag, bytes) = match v {
        Value::Null => (0, vec![]),
        Value::Integer(n) => (1, n.to_be_bytes().to_vec()),
        Value::Real(n) => (2, n.to_bits().to_be_bytes().to_vec()),
        Value::Text(s) => (3, s.as_bytes().to_vec()),
        Value::Blob(b) => (4, b.clone()),
    };
    out.push(tag);
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend(bytes);
}
fn encode_row(row: &[Value]) -> Vec<u8> {
    let mut b = Vec::new();
    for v in row {
        encode_value(&mut b, v);
    }
    b
}
fn digest(tables: &[Table]) -> String {
    let mut b = b"agentmender/desktop-catalog-logical/v1\0".to_vec();
    for t in tables {
        let mut rows: Vec<_> = t.rows.iter().map(|r| encode_row(r)).collect();
        rows.sort();
        b.extend_from_slice(&(t.name.len() as u64).to_be_bytes());
        b.extend_from_slice(t.name.as_bytes());
        b.extend_from_slice(&(rows.len() as u64).to_be_bytes());
        for r in rows {
            b.extend_from_slice(&(r.len() as u64).to_be_bytes());
            b.extend(r);
        }
    }
    hash(&b)
}
fn snapshot(db: &Connection) -> Result<Vec<Table>, EffectError> {
    let mut result = Vec::new();
    let mut size = 0usize;
    let mut count = 0usize;
    for (name, sql) in TABLES {
        let mut stmt = db.prepare(sql).map_err(|_| invalid())?;
        let cols = stmt.column_count();
        let mut cursor = stmt.query([]).map_err(|_| invalid())?;
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().map_err(|_| invalid())? {
            count += 1;
            if count > MAX_ROWS {
                return Err(invalid());
            }
            let mut values = Vec::with_capacity(cols);
            for i in 0..cols {
                values.push(row.get::<_, Value>(i).map_err(|_| invalid())?);
            }
            size = size
                .checked_add(encode_row(&values).len())
                .ok_or_else(invalid)?;
            if size > MAX_BYTES {
                return Err(invalid());
            }
            rows.push(values);
        }
        result.push(Table { name, rows });
    }
    Ok(result)
}
fn table<'a>(tables: &'a [Table], name: &str) -> &'a [Vec<Value>] {
    &tables
        .iter()
        .find(|t| t.name == name)
        .expect("fixed table")
        .rows
}
fn table_mut<'a>(tables: &'a mut [Table], name: &str) -> &'a mut Vec<Vec<Value>> {
    &mut tables
        .iter_mut()
        .find(|t| t.name == name)
        .expect("fixed table")
        .rows
}
fn one(rows: &[Vec<Value>], f: impl Fn(&Vec<Value>) -> bool) -> Result<&Vec<Value>, EffectError> {
    let mut it = rows.iter().filter(|r| f(r));
    let row = it.next().ok_or_else(invalid)?;
    if it.next().is_some() {
        return Err(invalid());
    }
    Ok(row)
}
fn schema(db: &Connection) -> Result<(), EffectError> {
    let expected: serde_json::Value = serde_json::from_str(SCHEMA).map_err(|_| invalid())?;
    if hash(SCHEMA.trim_end_matches('\n').as_bytes()) != SCHEMA_HASH {
        return Err(invalid());
    }
    let uv: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| invalid())?;
    let aid: i64 = db
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|_| invalid())?;
    if uv != 34 || aid != 0 {
        return Err(invalid());
    }
    let mut stmt = db
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name")
        .map_err(|_| invalid())?;
    let mut rows = stmt.query([]).map_err(|_| invalid())?;
    let mut actual = Vec::new();
    while let Some(r) = rows.next().map_err(|_| invalid())? {
        actual.push(serde_json::json!({"type": r.get::<_,String>(0).map_err(|_| invalid())?, "name": r.get::<_,String>(1).map_err(|_| invalid())?, "tbl_name": r.get::<_,String>(2).map_err(|_| invalid())?, "sql": r.get::<_,Option<String>>(3).map_err(|_| invalid())?}));
    }
    if expected["sqlite_master"] != serde_json::Value::Array(actual) {
        return Err(invalid());
    }
    Ok(())
}
fn migration_digest(db: &Connection) -> Result<String, EffectError> {
    let expected: serde_json::Value = serde_json::from_str(MIGRATIONS).map_err(|_| invalid())?;
    let entries = expected["migrations"].as_array().ok_or_else(invalid)?;
    let mut wanted = Vec::new();
    for e in entries {
        wanted.push((
            e["migration"]["id"]
                .as_str()
                .ok_or_else(invalid)?
                .to_owned(),
            e["predecessorId"].as_str().map(str::to_owned),
            e["checksum"].as_str().ok_or_else(invalid)?.to_owned(),
        ));
    }
    wanted.sort();
    let mut stmt = db
        .prepare("SELECT id,predecessor_id,checksum FROM codex_schema_migrations ORDER BY id")
        .map_err(|_| invalid())?;
    let actual = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| invalid())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;
    if actual != wanted || wanted.len() != 36 {
        return Err(invalid());
    }
    Ok(hash(&serde_json::to_vec(&actual).map_err(|_| invalid())?))
}

/// Exact local-host target, frozen during planning.
pub(super) struct CatalogTarget {
    pub thread_id: String,
    pub source_kind: String,
}
fn expected_postimage(
    mut all: Vec<Table>,
    target: &CatalogTarget,
) -> Result<(Vec<Table>, String, String, String), EffectError> {
    let id = &target.thread_id;
    if id.is_empty() || !matches!(target.source_kind.as_str(), "cli" | "appServer") {
        return Err(invalid());
    }
    let row = one(table(&all, "local_thread_catalog"), |r| {
        text(&r[0], "local") && text(&r[1], id)
    })?;
    if !text(&row[6], &target.source_kind) {
        return Err(invalid());
    }
    let visible = match row[11] {
        Value::Integer(0) => true,
        Value::Integer(1) => false,
        _ => return Err(invalid()),
    };
    let host = one(table(&all, "local_thread_catalog_hosts"), |r| {
        text(&r[0], "local")
    })?;
    if !text(&host[1], "local") {
        return Err(invalid());
    }
    for (name, col, host_col) in [
        ("automation_runs", 0, None),
        ("inbox_items", 3, None),
        ("automations", 15, None),
        ("active_chat_turns", 3, None),
        ("thread_timeline_ledger", 1, Some(0)),
        ("live_visualization_suggestions", 3, Some(2)),
    ] {
        if table(&all, name)
            .iter()
            .any(|r| text(&r[col], id) && host_col.is_none_or(|h| text(&r[h], "local")))
        {
            return Err(invalid());
        }
    }
    let sync = one(table(&all, "local_thread_catalog_sync_state"), |r| {
        text(&r[0], "local")
    })?;
    let seq = integer(&sync[3])?;
    let seq_hash = hash(&encode_row(sync));
    let meta = one(table(&all, "local_thread_catalog_metadata"), |r| {
        r[0] == Value::Integer(1)
    })?;
    let revision = integer(&meta[1])?;
    let revision_hash = hash(&encode_row(meta));
    let checkpoints = table(&all, "local_thread_catalog_scan_checkpoints")
        .iter()
        .filter(|r| text(&r[0], "local"))
        .count();
    if checkpoints > 1 {
        return Err(invalid());
    }
    let entries = table(&all, "local_thread_catalog_scan_entries")
        .iter()
        .filter(|r| text(&r[0], "local") && text(&r[1], id))
        .count();
    if entries > 1 {
        return Err(invalid());
    }
    // Retained commitment excludes only the four exact rows allowed to change.
    let mut retained = all.clone();
    for name in ["local_thread_catalog", "local_thread_catalog_scan_entries"] {
        table_mut(&mut retained, name).retain(|r| !(text(&r[0], "local") && text(&r[1], id)));
    }
    table_mut(&mut retained, "local_thread_catalog_sync_state").retain(|r| !text(&r[0], "local"));
    table_mut(&mut retained, "local_thread_catalog_metadata").retain(|r| r[0] != Value::Integer(1));
    let retained_hash = digest(&retained);
    table_mut(&mut all, "local_thread_catalog")
        .retain(|r| !(text(&r[0], "local") && text(&r[1], id)));
    one_mut(
        table_mut(&mut all, "local_thread_catalog_sync_state"),
        |r| text(&r[0], "local"),
    )?[3] = Value::Integer(seq + 1);
    if visible {
        one_mut(table_mut(&mut all, "local_thread_catalog_metadata"), |r| {
            r[0] == Value::Integer(1)
        })?[1] = Value::Integer(revision + 1);
    }
    if checkpoints == 1 {
        let rows = table_mut(&mut all, "local_thread_catalog_scan_entries");
        rows.retain(|r| !(text(&r[0], "local") && text(&r[1], id)));
        rows.push(vec![
            Value::Text("local".into()),
            Value::Text(id.clone()),
            Value::Integer(1),
        ]);
    }
    Ok((all, retained_hash, seq_hash, revision_hash))
}
fn one_mut(
    rows: &mut [Vec<Value>],
    f: impl Fn(&Vec<Value>) -> bool,
) -> Result<&mut Vec<Value>, EffectError> {
    let indices: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, r)| f(r).then_some(i))
        .collect();
    if indices.len() != 1 {
        return Err(invalid());
    }
    Ok(&mut rows[indices[0]])
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CatalogObservation {
    pub commitment: DesktopCatalogCommitment,
    pub title: String,
    pub bare_uuid_qualified: bool,
}

/// Called only after the worker's read boundary has been established.
pub(super) fn observe(
    db: &Connection,
    identity: CleanupFileIdentity,
    target: &CatalogTarget,
) -> Result<CatalogObservation, EffectError> {
    schema(db)?;
    let migration_sha256 = migration_digest(db)?;
    let all = snapshot(db)?;
    let row = one(table(&all, "local_thread_catalog"), |r| {
        text(&r[0], "local") && text(&r[1], &target.thread_id)
    })?;
    let title = match &row[2] {
        Value::Text(s) if s.len() <= 16 * 1024 => s.clone(),
        _ => return Err(invalid()),
    };
    let bare_uuid_qualified = [
        ("local_thread_catalog", 0, 1),
        ("local_thread_catalog_scan_entries", 0, 1),
        ("thread_timeline_ledger", 0, 1),
        ("live_visualization_suggestions", 2, 3),
    ]
    .into_iter()
    .all(|(name, host, id)| {
        !table(&all, name)
            .iter()
            .any(|r| !text(&r[host], "local") && text(&r[id], &target.thread_id))
    });
    let preimage_sha256 = digest(&all);
    let (_, retained_sha256, target_sequence_sha256, revision_sha256) =
        expected_postimage(all, target)?;
    Ok(CatalogObservation {
        title,
        bare_uuid_qualified,
        commitment: DesktopCatalogCommitment {
            identity,
            schema_sha256: SCHEMA_HASH.into(),
            migration_sha256,
            preimage_sha256,
            retained_sha256,
            target_rows: 1,
            target_sequence_sha256,
            revision_sha256,
        },
    })
}
/// Holds a SQLite EXCLUSIVE transaction until the fixed action is committed or dropped.
/// The caller must also hold the physical identity and all other writer fences.
pub(super) struct CatalogPostimage {
    digest: String,
    migration_sha256: String,
}

/// A fresh protected read after commit verifies the complete logical postimage.
pub(super) fn verify_postimage(
    db: &Connection,
    expected: &CatalogPostimage,
) -> Result<(), EffectError> {
    schema(db)?;
    if migration_digest(db)? != expected.migration_sha256
        || digest(&snapshot(db)?) != expected.digest
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) struct CatalogFence {
    db: Connection,
}
impl CatalogFence {
    pub(super) fn acquire(db: Connection) -> Result<Self, EffectError> {
        db.busy_timeout(std::time::Duration::ZERO)
            .map_err(|_| invalid())?;
        let mode: String = db
            .query_row("PRAGMA locking_mode=EXCLUSIVE", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        if !mode.eq_ignore_ascii_case("exclusive") {
            return Err(invalid());
        }
        db.execute_batch("BEGIN EXCLUSIVE").map_err(|_| invalid())?;
        let journal: String = db
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        if !journal.eq_ignore_ascii_case("wal") {
            return Err(invalid());
        }
        Ok(Self { db })
    }
    pub(super) fn prevalidate(
        &self,
        target: &CatalogTarget,
        frozen: &DesktopCatalogCommitment,
    ) -> Result<(), EffectError> {
        let actual = observe(&self.db, frozen.identity.clone(), target)?.commitment;
        if actual.schema_sha256 != frozen.schema_sha256
            || actual.migration_sha256 != frozen.migration_sha256
            || actual.preimage_sha256 != frozen.preimage_sha256
            || actual.retained_sha256 != frozen.retained_sha256
            || frozen.target_rows != 1
            || actual.target_sequence_sha256 != frozen.target_sequence_sha256
            || actual.revision_sha256 != frozen.revision_sha256
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn verify(&self, expected: &CatalogPostimage) -> Result<(), EffectError> {
        verify_postimage(&self.db, expected)
    }
    pub(super) fn apply(
        &mut self,
        target: &CatalogTarget,
        frozen: &DesktopCatalogCommitment,
    ) -> Result<CatalogPostimage, EffectError> {
        self.prevalidate(target, frozen)?;
        let all = snapshot(&self.db)?;
        let (expected, _, _, _) = expected_postimage(all, target)?;
        let seq: i64 = self.db.query_row("SELECT observation_sequence FROM local_thread_catalog_sync_state WHERE host_id='local'", [], |r| r.get(0)).map_err(|_| invalid())?;
        let rev: i64 = self
            .db
            .query_row(
                "SELECT catalog_revision FROM local_thread_catalog_metadata WHERE id=1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| invalid())?;
        let checkpoint: bool = self
            .db
            .query_row(
                "SELECT 1 FROM local_thread_catalog_scan_checkpoints WHERE host_id='local'",
                [],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| invalid())?
            .is_some();
        if self.db.execute("UPDATE local_thread_catalog_sync_state SET observation_sequence=?1 WHERE host_id='local' AND observation_sequence=?2", params![seq+1,seq]).map_err(|_| invalid())? != 1 { return Err(invalid()); }
        if checkpoint {
            self.db.execute("INSERT INTO local_thread_catalog_scan_entries(host_id,thread_id,removed) VALUES('local',?1,1) ON CONFLICT(host_id,thread_id) DO UPDATE SET removed=1", [&target.thread_id]).map_err(|_| invalid())?;
        }
        let missing: i64 = self.db.query_row("DELETE FROM local_thread_catalog WHERE host_id='local' AND thread_id=?1 AND source_kind=?2 RETURNING missing_candidate", params![target.thread_id,target.source_kind], |r| r.get(0)).map_err(|_| invalid())?;
        if missing == 0 && self.db.execute("UPDATE local_thread_catalog_metadata SET catalog_revision=?1 WHERE id=1 AND catalog_revision=?2", params![rev+1,rev]).map_err(|_| invalid())? != 1 { return Err(invalid()); }
        let expected_digest = digest(&expected);
        if digest(&snapshot(&self.db)?) != expected_digest {
            return Err(invalid());
        }
        self.db.execute_batch("COMMIT").map_err(|_| invalid())?;
        let postimage = CatalogPostimage {
            digest: expected_digest,
            migration_sha256: frozen.migration_sha256.clone(),
        };
        verify_postimage(&self.db, &postimage)?;
        Ok(postimage)
    }
}
impl Drop for CatalogFence {
    fn drop(&mut self) {
        let _ = self.db.execute_batch("ROLLBACK");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Connection) {
        let root = tempfile::tempdir().unwrap();
        let db = Connection::open(root.path().join("codex.db")).unwrap();
        db.pragma_update(None, "journal_mode", "WAL").unwrap();
        db.execute_batch(include_str!("../../../compatibility/codex/desktop-complete-removal-v1/catalog/migration-ledger-create.sql")).unwrap();
        let doc: serde_json::Value = serde_json::from_str(MIGRATIONS).unwrap();
        for e in doc["migrations"].as_array().unwrap() {
            for op in e["migration"]["operations"].as_array().unwrap() {
                let s = |name: &str| op[name].as_str().unwrap();
                match s("kind") {
                    "exec" => db.execute_batch(s("sql")).unwrap(),
                    "run" => {
                        let p: Vec<_> = op["parameters"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|x| x.as_str().unwrap())
                            .collect();
                        db.execute(s("sql"), rusqlite::params_from_iter(p)).unwrap();
                    }
                    "addColumnIfMissing" => {
                        let mut stmt = db
                            .prepare(&format!("PRAGMA table_info({})", s("table")))
                            .unwrap();
                        let cols = stmt
                            .query_map([], |r| r.get::<_, String>(1))
                            .unwrap()
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();
                        if !cols.iter().any(|c| c == s("column")) {
                            db.execute_batch(&format!(
                                "ALTER TABLE {} ADD COLUMN {} {}",
                                s("table"),
                                s("column"),
                                s("columnDefinition")
                            ))
                            .unwrap();
                        }
                    }
                    _ => panic!("unexpected fixture migration"),
                }
            }
            db.execute("INSERT INTO codex_schema_migrations(id,predecessor_id,checksum,applied_at) VALUES(?1,?2,?3,0)", params![e["migration"]["id"].as_str().unwrap(), e["predecessorId"].as_str(), e["checksum"].as_str().unwrap()]).unwrap();
        }
        db.execute_batch("PRAGMA user_version=34; INSERT INTO local_thread_catalog_hosts VALUES('remote','ssh'); UPDATE local_thread_catalog_sync_state SET observation_sequence=7 WHERE host_id='local'; INSERT INTO local_thread_catalog_sync_state(host_id,observation_sequence) VALUES('remote',12); UPDATE local_thread_catalog_metadata SET catalog_revision=4 WHERE id=1; INSERT INTO local_thread_catalog_scan_checkpoints VALUES('local','opaque',NULL);").unwrap();
        for (host, id, missing) in [
            ("local", "target", 0),
            ("remote", "target", 0),
            ("local", "target2", 0),
        ] {
            db.execute("INSERT INTO local_thread_catalog(host_id,thread_id,display_title,source_created_at,source_updated_at,cwd,source_kind,observation_sequence,missing_candidate,project_id) VALUES(?1,?2,'name',1,2,'same','cli',2,?3,'same')",params![host,id,missing]).unwrap();
        }
        (root, db)
    }
    fn id() -> CleanupFileIdentity {
        CleanupFileIdentity {
            path: "fixture".into(),
            device: 0,
            inode: 0,
            links: 1,
            size_bytes: 0,
            mtime_seconds: 0,
            mtime_nanoseconds: 0,
        }
    }
    fn target() -> CatalogTarget {
        CatalogTarget {
            thread_id: "target".into(),
            source_kind: "cli".into(),
        }
    }
    #[test]
    fn exact_composite_and_full_postimage() {
        let (_root, db) = fixture();
        let t = target();
        let frozen = observe(&db, id(), &t).unwrap().commitment;
        let mut fence = CatalogFence::acquire(db).unwrap();
        let postimage = fence.apply(&t, &frozen).unwrap();
        fence.verify(&postimage).unwrap();
    }
    #[test]
    fn drift_and_related_target_rejected_before_changes() {
        let (_root, db) = fixture();
        let t = target();
        let frozen = observe(&db, id(), &t).unwrap().commitment;
        db.execute(
            "INSERT INTO inbox_items(id,thread_id) VALUES('inbox','target')",
            [],
        )
        .unwrap();
        assert!(
            CatalogFence::acquire(db)
                .unwrap()
                .apply(&t, &frozen)
                .is_err()
        );
        let (_root, db) = fixture();
        db.execute("INSERT INTO automation_runs(thread_id,automation_id,status,created_at,updated_at) VALUES('target','a','x',0,0)",[]).unwrap();
        assert!(observe(&db, id(), &t).is_err());
    }
    #[test]
    fn bare_id_ownership_rejects_foreign_collision() {
        let (_root, db) = fixture();
        let observation = observe(&db, id(), &target()).unwrap();
        assert_eq!(observation.title, "name");
        assert!(!observation.bare_uuid_qualified);
        db.execute(
            "DELETE FROM local_thread_catalog WHERE host_id='remote'",
            [],
        )
        .unwrap();
        assert!(observe(&db, id(), &target()).unwrap().bare_uuid_qualified);
        db.execute(
            "INSERT INTO local_thread_catalog_scan_entries VALUES('remote','target',0)",
            [],
        )
        .unwrap();
        assert!(!observe(&db, id(), &target()).unwrap().bare_uuid_qualified);
    }
    #[test]
    fn schema_and_migration_drift_rejected() {
        let (_root, db) = fixture();
        db.execute_batch("CREATE TABLE unexpected(x)").unwrap();
        assert!(observe(&db, id(), &target()).is_err());
        let (_root, db) = fixture();
        db.execute("UPDATE codex_schema_migrations SET checksum='changed'", [])
            .unwrap();
        assert!(observe(&db, id(), &target()).is_err());
    }
    #[test]
    fn seeded_collateral_and_scan_cases() {
        let mut state = 0x4d3832315f763431u64;
        for _ in 0..256 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let (_root, db) = fixture();
            if state & 1 != 0 {
                db.execute("UPDATE local_thread_catalog SET missing_candidate=1 WHERE host_id='local' AND thread_id='target'", []).unwrap();
            }
            if state & 2 != 0 {
                db.execute(
                    "DELETE FROM local_thread_catalog_scan_checkpoints WHERE host_id='local'",
                    [],
                )
                .unwrap();
            }
            if state & 4 != 0 {
                db.execute(
                    "INSERT INTO local_thread_catalog_scan_entries VALUES('local','target',0)",
                    [],
                )
                .unwrap();
            }
            let t = target();
            let frozen = observe(&db, id(), &t).unwrap().commitment;
            CatalogFence::acquire(db)
                .unwrap()
                .apply(&t, &frozen)
                .unwrap();
        }
    }
}
