//! Fixed SQLite operations for the reviewed scoped profile. The caller holds the
//! provider writer fence and a consumed durable intent throughout this lifetime.
use crate::{
    capture::{self, ExactProfile},
    scoped_capture,
};
use agentmender_domain::CleanupManifest;
use agentmender_persistence::EffectError;
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use sha2::{Digest, Sha256};
use std::time::Duration;

fn invalid() -> EffectError {
    EffectError::InvalidEvidence
}
const ROWS: [(&str, &str); 6] = [
    ("threads", "SELECT * FROM threads WHERE id=?1 ORDER BY id"),
    (
        "thread_dynamic_tools",
        "SELECT * FROM thread_dynamic_tools WHERE thread_id=?1 ORDER BY thread_id, position",
    ),
    (
        "thread_read_receipts",
        "SELECT * FROM thread_read_receipts WHERE thread_id=?1 ORDER BY thread_id",
    ),
    (
        "thread_attachments",
        "SELECT * FROM thread_attachments WHERE thread_id=?1 ORDER BY id",
    ),
    (
        "guardian_review_feedback",
        "SELECT * FROM guardian_review_feedback WHERE thread_id=?1 ORDER BY id",
    ),
    (
        "thread_spawn_edges",
        "SELECT * FROM thread_spawn_edges WHERE parent_thread_id=?1 OR child_thread_id=?1 ORDER BY child_thread_id",
    ),
];
// Static complete schema projection. Each row is independently encoded and sorted;
// SQLite storage order and VACUUM therefore cannot alter this semantic commitment.
const RETAINED: [(&str, &str); 17] = [
    ("_sqlx_migrations", "SELECT * FROM _sqlx_migrations"),
    ("backfill_state", "SELECT * FROM backfill_state"),
    (
        "external_agent_config_imports",
        "SELECT * FROM external_agent_config_imports",
    ),
    (
        "guardian_review_feedback",
        "SELECT * FROM guardian_review_feedback",
    ),
    (
        "project_idempotency_keys",
        "SELECT * FROM project_idempotency_keys",
    ),
    ("project_roots", "SELECT * FROM project_roots"),
    ("projects", "SELECT * FROM projects"),
    (
        "remote_control_enrollments",
        "SELECT * FROM remote_control_enrollments",
    ),
    (
        "rollout_migration_skipped_rollouts",
        "SELECT * FROM rollout_migration_skipped_rollouts",
    ),
    (
        "rollout_migration_state",
        "SELECT * FROM rollout_migration_state",
    ),
    ("sqlite_sequence", "SELECT * FROM sqlite_sequence"),
    ("thread_attachments", "SELECT * FROM thread_attachments"),
    (
        "thread_dynamic_tools",
        "SELECT * FROM thread_dynamic_tools WHERE thread_id IS NOT ?1",
    ),
    (
        "thread_read_receipts",
        "SELECT * FROM thread_read_receipts WHERE thread_id IS NOT ?1",
    ),
    ("thread_sections", "SELECT * FROM thread_sections"),
    ("thread_spawn_edges", "SELECT * FROM thread_spawn_edges"),
    ("threads", "SELECT * FROM threads WHERE id IS NOT ?1"),
];

pub(super) fn retained_digest(db: &Connection, target: &str) -> Result<String, EffectError> {
    let mut hash = Sha256::new();
    hash.update(b"agentmender/scoped-retained-db/v1\0");
    let mut total = 0usize;
    for (name, sql) in RETAINED {
        let mut stmt = db.prepare(sql).map_err(|_| invalid())?;
        let columns = stmt.column_count();
        let mut rows = if sql.contains("?1") {
            stmt.query([target])
        } else {
            stmt.query([])
        }
        .map_err(|_| invalid())?;
        let mut encoded = Vec::new();
        while let Some(row) = rows.next().map_err(|_| invalid())? {
            if encoded.len() >= 100_000 {
                return Err(invalid());
            }
            let mut bytes = Vec::new();
            for col in 0..columns {
                let value = row.get_ref(col).map_err(|_| invalid())?;
                let (tag, raw): (u8, Vec<u8>) = match value {
                    ValueRef::Null => (0, vec![]),
                    ValueRef::Integer(v) => (1, v.to_be_bytes().to_vec()),
                    ValueRef::Real(v) => (2, v.to_bits().to_be_bytes().to_vec()),
                    ValueRef::Text(v) => (3, v.to_vec()),
                    ValueRef::Blob(v) => (4, v.to_vec()),
                };
                total = total.checked_add(raw.len()).ok_or_else(invalid)?;
                if total > 32 * 1024 * 1024 {
                    return Err(invalid());
                }
                bytes.push(tag);
                bytes.extend_from_slice(&(raw.len() as u64).to_be_bytes());
                bytes.extend_from_slice(&raw);
            }
            encoded.push(bytes);
        }
        encoded.sort();
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((encoded.len() as u64).to_be_bytes());
        for bytes in encoded {
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub(super) struct DatabaseFence {
    db: Connection,
    prepared: bool,
    manifest: CleanupManifest,
}
impl DatabaseFence {
    pub(super) fn acquire(m: &CleanupManifest) -> Result<Self, EffectError> {
        if !crate::scoped_profile::validate_manifest_profile(m) {
            return Err(invalid());
        }
        #[cfg(unix)]
        let before = std::fs::symlink_metadata(&m.database.path).map_err(|_| invalid())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if !before.is_file()
                || before.file_type().is_symlink()
                || before.dev() != m.database.device
                || before.ino() != m.database.inode
                || before.nlink() != 1
                || before.len() != m.database.size_bytes
                || before.mtime() != m.database.mtime_seconds
                || before.mtime_nsec() != i64::from(m.database.mtime_nanoseconds)
            {
                return Err(invalid());
            }
        }
        let db = Connection::open_with_flags(
            &m.database.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| invalid())?;
        db.busy_timeout(Duration::from_millis(50))
            .map_err(|_| invalid())?;
        // Must precede the first database access. An EXCLUSIVE transaction in
        // this mode acquires the WAL exclusive lock and holds it until drop.
        let mode: String = db
            .query_row("PRAGMA locking_mode=EXCLUSIVE", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        if !mode.eq_ignore_ascii_case("exclusive") {
            return Err(invalid());
        }
        db.execute_batch("PRAGMA foreign_keys=ON; BEGIN EXCLUSIVE;")
            .map_err(|_| invalid())?;
        let journal: String = db
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        if !journal.eq_ignore_ascii_case("wal") {
            return Err(invalid());
        }
        capture::verify_schema(&db, ExactProfile::V165).map_err(|_| invalid())?;
        #[cfg(unix)]
        let after = std::fs::symlink_metadata(&m.database.path).map_err(|_| invalid())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if after.dev() != before.dev()
                || after.ino() != before.ino()
                || after.nlink() != 1
                || after.len() != before.len()
                || after.mtime() != before.mtime()
                || after.mtime_nsec() != before.mtime_nsec()
            {
                return Err(invalid());
            }
        }
        Ok(Self {
            db,
            prepared: false,
            manifest: m.clone(),
        })
    }
    pub(super) fn target_cwd(
        &self,
        m: &CleanupManifest,
    ) -> Result<std::path::PathBuf, EffectError> {
        if m != &self.manifest || self.prepared {
            return Err(invalid());
        }
        let cwd: String = self
            .db
            .query_row(
                "SELECT cwd FROM threads WHERE id=?1",
                [m.native_session_id.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| invalid())?;
        let path = std::path::PathBuf::from(cwd);
        if !path.is_absolute() {
            return Err(invalid());
        }
        Ok(path)
    }
    fn validate(&self, m: &CleanupManifest) -> Result<(), EffectError> {
        capture::verify_schema(&self.db, ExactProfile::V165).map_err(|_| invalid())?;
        validate_known_thread_paths(&self.db, m)?;
        if m.database_payload_paths.len() != 1 {
            return Err(invalid());
        }
        let target = m.native_session_id.as_str();
        for (table, sql) in ROWS {
            let observed =
                scoped_capture::commitment(&self.db, target, table, sql).map_err(|_| invalid())?;
            let expected = m
                .rows
                .iter()
                .find(|r| r.table == table)
                .ok_or_else(invalid)?;
            if observed.count != expected.expected_count
                || observed.sha256 != expected.preimage_sha256
            {
                return Err(invalid());
            }
        }
        let (path, history, version): (String, String, String) = self
            .db
            .query_row(
                "SELECT rollout_path, history_mode, cli_version FROM threads WHERE id=?1",
                [target],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| invalid())?;
        if history != "legacy"
            || version != crate::scoped_profile::SCOPED_CODEX_VERSION
            || std::path::Path::new(&path) != m.database_payload_paths[0].path
            || retained_digest(&self.db, target)? != m.retained_database_sha256
        {
            return Err(invalid());
        }
        let aliases: i64 = self.db.query_row("SELECT count(*) FROM threads WHERE id IS NOT ?1 AND (rollout_path=?2 OR attached_thread_parent_id=?1)", (target, &path), |r| r.get(0)).map_err(|_| invalid())?;
        if aliases != 0 {
            return Err(invalid());
        }
        Ok(())
    }
    fn apply(&self, m: &CleanupManifest) -> Result<(), EffectError> {
        let target = m.native_session_id.as_str();
        let receipts = self
            .db
            .execute(
                "DELETE FROM thread_read_receipts WHERE thread_id=?1",
                [target],
            )
            .map_err(|_| invalid())?;
        let expected = m
            .rows
            .iter()
            .find(|r| r.table == "thread_read_receipts")
            .ok_or_else(invalid)?
            .expected_count;
        if receipts as u64 != expected {
            return Err(invalid());
        }
        let changed = self
            .db
            .execute("DELETE FROM threads WHERE id=?1", [target])
            .map_err(|_| invalid())?;
        if changed != 1 || retained_digest(&self.db, target)? != m.retained_database_sha256 {
            return Err(invalid());
        }
        let tools: i64 = self
            .db
            .query_row(
                "SELECT count(*) FROM thread_dynamic_tools WHERE thread_id=?1",
                [target],
                |r| r.get(0),
            )
            .map_err(|_| invalid())?;
        if tools != 0 {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn prepare_delete(&mut self, m: &CleanupManifest) -> Result<(), EffectError> {
        if self.prepared || &self.manifest != m {
            return Err(invalid());
        }
        self.validate(m)?;
        self.db
            .execute_batch("SAVEPOINT dry_run")
            .map_err(|_| invalid())?;
        let dry = self.apply(m);
        self.db
            .execute_batch("ROLLBACK TO dry_run; RELEASE dry_run")
            .map_err(|_| invalid())?;
        dry?;
        self.validate(m)?;
        if let Err(e) = self.apply(m) {
            let _ = self.db.execute_batch("ROLLBACK");
            return Err(e);
        }
        self.prepared = true;
        Ok(())
    }
    pub(super) fn commit_and_verify(&mut self, m: &CleanupManifest) -> Result<bool, EffectError> {
        if !self.prepared || &self.manifest != m {
            return Err(invalid());
        }
        self.db.execute_batch("COMMIT").map_err(|_| invalid())?;
        self.prepared = false;
        let count: i64 = self
            .db
            .query_row(
                "SELECT count(*) FROM threads WHERE id=?1",
                [m.native_session_id.as_str()],
                |r| r.get(0),
            )
            .map_err(|_| invalid())?;
        Ok(count == 0
            && retained_digest(&self.db, m.native_session_id.as_str())?
                == m.retained_database_sha256)
    }
}

/// Every selected-database row must point to one inventoried primary rollout.
/// Filename identity is checked independently; the file pass verifies headers.
#[cfg(unix)]
fn validate_known_thread_paths(db: &Connection, m: &CleanupManifest) -> Result<(), EffectError> {
    use std::{collections::BTreeSet, path::Component};
    let mut paths = BTreeSet::new();
    let target = m.native_session_id.as_str();
    for row in scoped_capture::known_threads(db).map_err(|_| invalid())? {
        let path = &row.rollout_path;
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || !(path.starts_with(m.root.path.join("sessions"))
                || path.starts_with(m.root.path.join("archived_sessions")))
            || !crate::scoped_files::valid_rollout_name(path, &row.id)
            || !paths.insert(path.clone())
        {
            return Err(invalid());
        }
        let listed = if row.id == target {
            m.payloads
                .iter()
                .any(|p| p.identity.path == *path && p.header_session_id.as_str() == row.id)
        } else {
            m.retained_files.iter().any(|f| f.path == *path)
        };
        if !listed {
            return Err(invalid());
        }
    }
    Ok(())
}
#[cfg(not(unix))]
fn validate_known_thread_paths(_: &Connection, _: &CleanupManifest) -> Result<(), EffectError> {
    Err(invalid())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::scoped_profile::{
        SCOPED_ADAPTER, SCOPED_CODEX_EXECUTABLE_SHA256, SCOPED_CODEX_SCHEMA_SHA256,
        SCOPED_CODEX_VERSION,
    };
    use agentmender_domain::*;
    use std::os::unix::fs::MetadataExt;

    fn fixture() -> (tempfile::TempDir, CleanupManifest) {
        let root = tempfile::tempdir().unwrap();
        let rootpath = root.path().canonicalize().unwrap();
        let dbpath = rootpath.join("state_5.sqlite");
        let db = Connection::open(&dbpath).unwrap();
        db.pragma_update(None, "journal_mode", "WAL").unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE fixture_sequence_seed(value INTEGER PRIMARY KEY AUTOINCREMENT); DROP TABLE fixture_sequence_seed;").unwrap();
        let schema: Vec<(String, String, String, Option<String>)> = serde_json::from_str(
            include_str!("../../../compatibility/codex/0.165.0-alpha.1.2/state-schema-5.json"),
        )
        .unwrap();
        for kind in ["table", "index", "trigger", "view"] {
            for (entry_kind, name, _, sql) in &schema {
                if entry_kind == kind
                    && name != "sqlite_sequence"
                    && let Some(sql) = sql
                {
                    db.execute_batch(sql).unwrap();
                }
            }
        }
        let target = "12345678-1234-1234-1234-123456789abc";
        let path = rootpath.join(format!(
            "sessions/rollout-2026-01-01T00-00-00-{target}.jsonl"
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"fixture").unwrap();
        db.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,cli_version) VALUES (?1,?2,1,1,'cli','openai','/fixture','target','read-only','never','0.166.0')", (target,path.to_str().unwrap())).unwrap();
        let foreign = "12345678-1234-1234-1234-123456789abd";
        let foreign_path = rootpath.join(format!(
            "sessions/rollout-2026-01-01T00-00-01-{foreign}.jsonl"
        ));
        std::fs::write(&foreign_path, b"foreign fixture").unwrap();
        db.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode) VALUES (?1,?2,1,1,'cli','openai','/fixture','foreign','read-only','never')", (foreign, foreign_path.to_str().unwrap())).unwrap();
        db.execute("INSERT INTO thread_dynamic_tools(thread_id,position,name,description,input_schema) VALUES (?1,0,'fixture','fixture','{}')", [target]).unwrap();
        db.execute(
            "INSERT INTO thread_read_receipts(thread_id,revision) VALUES (?1,'fixture')",
            [target],
        )
        .unwrap();
        let rows = ROWS
            .iter()
            .map(|(name, sql)| {
                let r = scoped_capture::commitment(&db, target, name, sql).unwrap();
                CleanupRows {
                    table: r.table,
                    expected_count: r.count,
                    preimage_sha256: r.sha256,
                }
            })
            .collect();
        let retained = retained_digest(&db, target).unwrap();
        drop(db);
        fn identity(path: &std::path::Path) -> CleanupFileIdentity {
            let md = std::fs::metadata(path).unwrap();
            CleanupFileIdentity {
                path: path.into(),
                device: md.dev(),
                inode: md.ino(),
                links: md.nlink(),
                size_bytes: md.len(),
                mtime_seconds: md.mtime(),
                mtime_nanoseconds: md.mtime_nsec() as u32,
            }
        }
        let artifact = |name: &str, class| ArtifactId {
            provider: ProviderKind::Codex,
            state_root: rootpath.clone(),
            class,
            local_id: EntityId::new(name).unwrap(),
        };
        let m = CleanupManifest {
            target: artifact("session", GraphNodeKind::Session),
            native_session_id: EntityId::new(target).unwrap(),
            root: identity(&rootpath),
            profile: CleanupProfile {
                version: SCOPED_CODEX_VERSION.into(),
                executable_sha256: SCOPED_CODEX_EXECUTABLE_SHA256.into(),
                schema_sha256: SCOPED_CODEX_SCHEMA_SHA256.into(),
                adapter: SCOPED_ADAPTER.into(),
                platform: "darwin-arm64".into(),
                journal_mode: "wal".into(),
                history_layout: "legacy-plain".into(),
            },
            database: identity(&dbpath),
            rows,
            database_payload_paths: vec![CleanupDatabasePath { path: path.clone() }],
            payloads: vec![CleanupPayload {
                artifact: artifact("payload", GraphNodeKind::Artifact),
                identity: identity(&path),
                header_session_id: EntityId::new(target).unwrap(),
                header_sha256: "a".repeat(64),
            }],
            optional_index: None,
            retained_database_sha256: retained,
            retained_files: vec![identity(&foreign_path)],
            residuals: vec![],
        };
        (root, m)
    }
    #[test]
    fn exact_delete_preserves_other_state_and_cascades_approved_tools() {
        let (_root, m) = fixture();
        assert!(
            validate_cleanup_manifest(&m),
            "invalid fixture manifest: {m:?}"
        );
        let probe = Connection::open(&m.database.path).unwrap();
        assert!(
            capture::verify_schema(&probe, ExactProfile::V165).is_ok(),
            "schema"
        );
        drop(probe);
        let mut db = DatabaseFence::acquire(&m).unwrap();
        db.prepare_delete(&m).unwrap();
        assert!(db.commit_and_verify(&m).unwrap());
    }
    #[test]
    fn drift_rejects_without_writes() {
        let (_root, mut m) = fixture();
        m.rows
            .iter_mut()
            .find(|r| r.table == "thread_dynamic_tools")
            .unwrap()
            .expected_count += 1;
        let mut db = DatabaseFence::acquire(&m).unwrap();
        assert!(db.prepare_delete(&m).is_err());
        drop(db);
        let db = Connection::open(&m.database.path).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM threads", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM thread_dynamic_tools", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn profile_gate_rejects_well_formed_foreign_layouts() {
        let (_root, m) = fixture();
        assert!(crate::scoped_profile::validate_manifest_profile(&m));
        let mut other = m.clone();
        other.profile.version = "future".into();
        assert!(agentmender_domain::validate_cleanup_manifest(&other));
        assert!(!crate::scoped_profile::validate_manifest_profile(&other));
        other = m.clone();
        other.rows[0].table = "unrecognized".into();
        assert!(agentmender_domain::validate_cleanup_manifest(&other));
        assert!(!crate::scoped_profile::validate_manifest_profile(&other));
        other = m.clone();
        other.database.path = other.root.path.join("foreign.sqlite");
        assert!(agentmender_domain::validate_cleanup_manifest(&other));
        assert!(!crate::scoped_profile::validate_manifest_profile(&other));
    }
    #[test]
    fn other_database_row_outside_inventory_is_refused_without_touching_files() {
        let (_root, mut manifest) = fixture();
        let outside = manifest.root.path.join("outside-history.jsonl");
        let bytes = b"foreign history referencing target";
        std::fs::write(&outside, bytes).unwrap();
        let db = Connection::open(&manifest.database.path).unwrap();
        db.execute(
            "UPDATE threads SET rollout_path=?1 WHERE id=?2",
            (
                outside.to_str().unwrap(),
                "12345678-1234-1234-1234-123456789abd",
            ),
        )
        .unwrap();
        manifest.retained_database_sha256 =
            retained_digest(&db, manifest.native_session_id.as_str()).unwrap();
        let rows = scoped_capture::known_threads(&db).unwrap();
        assert!(rows.iter().any(|row| row.rollout_path == outside));
        assert!(validate_known_thread_paths(&db, &manifest).is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), bytes);
        assert!(manifest.payloads[0].identity.path.exists());
    }
    #[test]
    fn active_wal_reader_prevents_exclusive_fence() {
        let (_root, m) = fixture();
        let other = Connection::open(&m.database.path).unwrap();
        other
            .execute_batch("BEGIN; SELECT * FROM threads;")
            .unwrap();
        assert!(DatabaseFence::acquire(&m).is_err());
    }
    #[test]
    fn fence_blocks_new_wal_reader() {
        let (_root, m) = fixture();
        let _fence = DatabaseFence::acquire(&m).unwrap();
        let other = Connection::open(&m.database.path).unwrap();
        other.busy_timeout(Duration::from_millis(20)).unwrap();
        assert!(
            other
                .query_row("SELECT count(*) FROM threads", [], |r| r.get::<_, i64>(0))
                .is_err()
        );
    }
    #[test]
    fn unknown_schema_and_incident_dependency_block() {
        let (_root, m) = fixture();
        let db = Connection::open(&m.database.path).unwrap();
        db.execute_batch("CREATE TABLE unexpected(x)").unwrap();
        drop(db);
        assert!(DatabaseFence::acquire(&m).is_err());
        let (_root, mut m) = fixture();
        let db = Connection::open(&m.database.path).unwrap();
        db.execute("INSERT INTO thread_spawn_edges(parent_thread_id,child_thread_id,status) VALUES (?1,'foreign','active')", [m.native_session_id.as_str()]).unwrap();
        drop(db);
        let metadata = std::fs::metadata(&m.database.path).unwrap();
        m.database.size_bytes = metadata.len();
        m.database.mtime_seconds = metadata.mtime();
        m.database.mtime_nanoseconds = metadata.mtime_nsec() as u32;
        let mut fence = DatabaseFence::acquire(&m).unwrap();
        assert!(fence.prepare_delete(&m).is_err());
    }
}
