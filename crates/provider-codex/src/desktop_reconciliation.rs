//! Best-effort refresh request for the Desktop projection after native deletion.
use rusqlite::{Connection, OpenFlags, config::DbConfig};
use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

/// Clear the full-reconciliation timestamp for a stale local catalog entry.
/// The caller invokes this only after a confirmed native delete exits zero.
pub fn schedule_desktop_catalog_reconciliation(root: &Path, deleted_id: Uuid) {
    if !root.is_absolute() {
        return;
    }
    let mut checked = PathBuf::new();
    for part in root.components() {
        if !matches!(part, Component::RootDir | Component::Normal(_)) {
            return;
        }
        checked.push(part);
        if !fs::symlink_metadata(&checked).is_ok_and(|meta| meta.is_dir()) {
            return;
        }
    }

    let directory = root.join("sqlite");
    if !fs::symlink_metadata(&directory).is_ok_and(|meta| meta.is_dir()) {
        return;
    }

    let id = deleted_id.hyphenated().to_string();
    for name in ["codex.db", "codex-dev.db"] {
        let database = directory.join(name);
        if !fs::symlink_metadata(&database).is_ok_and(|meta| meta.is_file() && meta.nlink() == 1)
            || !fs::symlink_metadata(database.with_extension("db-journal"))
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
        {
            continue;
        }
        let no_sidecars = match (
            sidecar_exists(&database.with_extension("db-wal")),
            sidecar_exists(&database.with_extension("db-shm")),
        ) {
            (Ok(false), Ok(false)) => true,
            (Ok(true), Ok(true)) => false,
            _ => continue,
        };
        let _ = request_full_reconciliation(&database, &id, no_sidecars);
    }
}

fn sidecar_exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && meta.nlink() == 1 => Ok(true),
        Ok(_) => Err(io::Error::from(io::ErrorKind::InvalidData)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn request_full_reconciliation(path: &Path, id: &str, no_sidecars: bool) -> rusqlite::Result<()> {
    let flags = OpenFlags::SQLITE_OPEN_NOFOLLOW | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if no_sidecars {
        // An ordinary read of a closed WAL database can create sidecars.
        // Use this snapshot only to skip unnecessary writes.
        let probe = Connection::open_with_flags(
            crate::scoped_capture::immutable_uri(path),
            flags | OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )?;
        probe.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
        probe.pragma_update(None, "temp_store", "memory")?;
        if !needs_reconciliation(&probe, id)? {
            return Ok(());
        }
    }

    let mut db = Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    db.busy_timeout(Duration::ZERO)?;
    db.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
    // Leave checkpointing and WAL/SHM cleanup to Desktop.
    db.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
    db.pragma_update(None, "wal_autocheckpoint", 0)?;
    db.pragma_update(None, "temp_store", "memory")?;

    let mode: String = db.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if mode != "wal" {
        return Ok(());
    }

    // The schema inspection and guarded update share a snapshot. A concurrent
    // writer can make the upgrade fail with SQLITE_BUSY; the caller moves on.
    let tx = db.transaction()?;
    if needs_reconciliation(&tx, id)? {
        tx.execute(
            "UPDATE local_thread_catalog_sync_state
             SET last_full_reconciled_at = NULL
             WHERE host_id COLLATE BINARY = 'local' AND last_full_reconciled_at IS NOT NULL",
            [],
        )?;
    }
    tx.commit()
}

fn needs_reconciliation(db: &Connection, id: &str) -> rusqlite::Result<bool> {
    if !db.query_row(COMPATIBLE_SCHEMA, [], |row| row.get::<_, bool>(0))? {
        return Ok(false);
    }
    db.query_row(
        "SELECT EXISTS (
             SELECT 1 FROM local_thread_catalog
             WHERE host_id COLLATE BINARY = 'local' AND thread_id COLLATE BINARY = ?1
         ) AND EXISTS (
             SELECT 1 FROM local_thread_catalog_sync_state
             WHERE host_id COLLATE BINARY = 'local' AND last_full_reconciled_at IS NOT NULL
         )",
        [id],
        |row| row.get(0),
    )
}

// Require ordinary tables, the observed primary keys and a writable nullable
// INTEGER marker. A trigger or foreign-key relationship could widen the write.
const COMPATIBLE_SCHEMA: &str = "
    SELECT (
        SELECT count(*) FROM pragma_table_list
        WHERE schema = 'main' AND type = 'table'
          AND name IN ('local_thread_catalog', 'local_thread_catalog_sync_state')
    ) = 2
    AND (
        SELECT count(*) FROM pragma_table_xinfo('local_thread_catalog', 'main') WHERE pk <> 0
    ) = 2
    AND (
        SELECT count(*)
        FROM pragma_table_xinfo('local_thread_catalog', 'main')
        WHERE (name, pk) IN (('host_id', 1), ('thread_id', 2))
          AND type = 'TEXT' AND [notnull] = 1 AND hidden = 0
    ) = 2
    AND (
        SELECT count(*) FROM pragma_table_xinfo('local_thread_catalog_sync_state', 'main')
    ) = 5
    AND NOT EXISTS (
        VALUES ('host_id', 'TEXT', 0, 1, 0),
               ('watermark_updated_at', 'REAL', 0, 0, 0),
               ('initial_build_complete', 'INTEGER', 1, 0, 0),
               ('observation_sequence', 'INTEGER', 1, 0, 0),
               ('last_full_reconciled_at', 'INTEGER', 0, 0, 0)
        EXCEPT
        SELECT name, type, [notnull], pk, hidden
        FROM pragma_table_xinfo('local_thread_catalog_sync_state', 'main')
    )
    AND NOT EXISTS (
        SELECT 1 FROM main.sqlite_schema
        WHERE type = 'trigger' AND tbl_name = 'local_thread_catalog_sync_state' COLLATE NOCASE
    )
    AND NOT EXISTS (
        SELECT 1 FROM pragma_table_list AS t
        JOIN pragma_foreign_key_list(t.name, 'main') AS fk
        WHERE t.schema = 'main' AND t.type = 'table'
          AND (t.name = 'local_thread_catalog_sync_state' COLLATE NOCASE
               OR fk.[table] = 'local_thread_catalog_sync_state' COLLATE NOCASE)
    )";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::FileControl;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use tempfile::TempDir;

    const ID: &str = "01234567-89ab-4cde-8f01-23456789abcd";
    const OTHER_ID: &str = "00000000-0000-4000-8000-000000000001";
    const SCHEMA: &str = include_str!(
        "../../../compatibility/codex/desktop-complete-removal-v1/catalog/complete-schema.sql"
    );

    fn root() -> TempDir {
        tempfile::tempdir_in("/private/tmp").unwrap()
    }

    fn schedule(root: &Path) {
        schedule_desktop_catalog_reconciliation(root, Uuid::parse_str(ID).unwrap());
    }

    fn catalog(root: &Path, name: &str) -> (std::path::PathBuf, Connection) {
        let directory = root.join("sqlite");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(name);
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "journal_mode", "wal").unwrap();
        db.execute_batch(SCHEMA).unwrap();
        db.execute_batch(
            "INSERT INTO local_thread_catalog_sync_state VALUES
                ('local', 42.5, 1, 7, 123), ('remote', 43.5, 1, 8, 456);",
        )
        .unwrap();
        (path, db)
    }

    fn insert_entry(db: &Connection, host: &str, id: &str) {
        db.execute(
            "INSERT INTO local_thread_catalog
                (host_id, thread_id, display_title, source_created_at, source_updated_at,
                 source_kind, observation_sequence)
             VALUES (?1, ?2, 'Disposable', 1, 1, 'cli', 7)",
            [host, id],
        )
        .unwrap();
    }

    fn local_state(db: &Connection) -> (f64, i64, i64, Option<i64>) {
        db.query_row(
            "SELECT watermark_updated_at, initial_build_complete, observation_sequence,
                    last_full_reconciled_at
             FROM local_thread_catalog_sync_state WHERE host_id = 'local'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
    }

    #[test]
    fn compatible_catalogs_keep_all_entries_and_other_host_state() {
        let root = root();
        for name in ["codex.db", "codex-dev.db"] {
            let (path, db) = catalog(root.path(), name);
            insert_entry(&db, "local", ID);
            insert_entry(&db, "local", OTHER_ID);
            insert_entry(&db, "remote", ID);
            if name == "codex.db" {
                db.file_control(None::<&str>, FileControl::PersistWal(&mut 1))
                    .unwrap();
            }
            db.close().unwrap();
            schedule(root.path());
            for suffix in ["-wal", "-shm"] {
                assert!(
                    root.path()
                        .join("sqlite")
                        .join(format!("{name}{suffix}"))
                        .is_file()
                );
            }
            let db = Connection::open(path).unwrap();
            assert_eq!(local_state(&db), (42.5, 1, 7, None));
            let remote: (f64, i64, i64, i64) = db
                .query_row(
                    "SELECT watermark_updated_at, initial_build_complete, observation_sequence,
                            last_full_reconciled_at
                     FROM local_thread_catalog_sync_state WHERE host_id = 'remote'",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .unwrap();
            assert_eq!(remote, (43.5, 1, 8, 456));
            let entries = db
                .prepare("SELECT host_id, thread_id FROM local_thread_catalog ORDER BY 1, 2")
                .unwrap()
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(
                entries,
                [("local", OTHER_ID), ("local", ID), ("remote", ID)]
                    .map(|(host, id)| (host.to_string(), id.to_string()))
            );
        }
    }

    #[test]
    fn absent_or_remote_only_uuid_does_not_change_local_state() {
        let root = root();
        let (path, db) = catalog(root.path(), "codex.db");
        insert_entry(&db, "local", OTHER_ID);
        insert_entry(&db, "remote", ID);
        insert_entry(&db, "LOCAL", ID);
        db.close().unwrap();
        schedule(root.path());
        assert!(!path.with_extension("db-wal").exists());
        assert!(!path.with_extension("db-shm").exists());
        assert!(!root.path().join("sqlite/codex-dev.db").exists());
        let db = Connection::open(path).unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
    }

    #[test]
    fn unknown_or_side_effecting_schema_is_ignored() {
        let root = root();
        for ddl in [
            "ALTER TABLE local_thread_catalog_sync_state DROP COLUMN last_full_reconciled_at",
            "ALTER TABLE local_thread_catalog RENAME TO other_catalog",
            "CREATE TRIGGER collateral AFTER UPDATE ON local_thread_catalog_sync_state
                 BEGIN DELETE FROM local_thread_catalog; END",
            "CREATE TABLE dependent (
                 host TEXT REFERENCES local_thread_catalog_sync_state(host_id)
             )",
        ] {
            let case = tempfile::tempdir_in(root.path()).unwrap();
            let (path, db) = catalog(case.path(), "codex.db");
            insert_entry(&db, "local", ID);
            db.execute_batch(ddl).unwrap();
            let before = fs::read(&path).unwrap();
            let wal = path.with_extension("db-wal");
            let before_wal = fs::read(&wal).unwrap();
            schedule(case.path());
            assert_eq!(fs::read(path).unwrap(), before);
            assert_eq!(fs::read(wal).unwrap(), before_wal);
            assert!(!case.path().join("sqlite/codex-dev.db").exists());
        }
    }

    #[test]
    fn unexpected_paths_and_links_are_ignored() {
        let outside = root();
        let (db_path, db) = catalog(outside.path(), "codex.db");
        insert_entry(&db, "local", ID);
        let elsewhere = root();
        let real_root = elsewhere.path().join("real");
        fs::create_dir(&real_root).unwrap();
        let linked_root = elsewhere.path().join("linked");
        symlink(outside.path(), &linked_root).unwrap();
        let intermediate_link = elsewhere.path().join("intermediate");
        symlink(outside.path().parent().unwrap(), &intermediate_link).unwrap();
        schedule(&linked_root);
        schedule(&intermediate_link.join(outside.path().file_name().unwrap()));
        schedule(&real_root.join("..").join("linked"));
        schedule(Path::new("sqlite"));

        let sqlite = real_root.join("sqlite");
        symlink(outside.path().join("sqlite"), &sqlite).unwrap();
        schedule(&real_root);
        fs::remove_file(&sqlite).unwrap();
        fs::create_dir(&sqlite).unwrap();
        let candidate = sqlite.join("codex.db");
        let (unknown, unknown_db) = catalog(&real_root, "codex-other.db");
        insert_entry(&unknown_db, "local", ID);
        unknown_db.close().unwrap();
        let unknown_before = fs::read(&unknown).unwrap();
        symlink(&db_path, &candidate).unwrap();
        schedule(&real_root);
        fs::remove_file(&candidate).unwrap();
        fs::hard_link(&db_path, &candidate).unwrap();
        schedule(&real_root);
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
        assert_eq!(fs::read(unknown).unwrap(), unknown_before);
        assert!(!sqlite.join("codex-dev.db").exists());
    }

    #[test]
    fn symlinked_sidecars_are_ignored() {
        let root = root();
        let (path, db) = catalog(root.path(), "codex.db");
        insert_entry(&db, "local", ID);
        db.close().unwrap();
        let outside = self::root();
        let sentinel = outside.path().join("sentinel");
        fs::write(&sentinel, b"outside").unwrap();
        for suffix in ["db-wal", "db-shm", "db-journal"] {
            let sidecar = path.with_extension(suffix);
            symlink(&sentinel, &sidecar).unwrap();
            schedule(root.path());
            assert!(sidecar.is_symlink());
            assert_eq!(fs::read(&sentinel).unwrap(), b"outside");
            fs::remove_file(sidecar).unwrap();
        }
        let db = Connection::open(path).unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
    }

    #[test]
    fn unavailable_catalog_is_skipped() {
        let root = root();
        schedule(root.path());
        assert!(!root.path().join("sqlite").exists());
        let (path, mut db) = catalog(root.path(), "codex.db");
        insert_entry(&db, "local", ID);
        let writer = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        schedule(root.path());
        writer.rollback().unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
        db.close().unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        schedule(root.path());
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
    }
}
