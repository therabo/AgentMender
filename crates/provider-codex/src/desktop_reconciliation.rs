//! Remove local thread projections and request a Desktop sidebar refresh.
use rusqlite::{Connection, OpenFlags, TransactionBehavior, config::DbConfig};
use std::{
    fs, io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Duration,
};

const CATALOGS: [&str; 2] = ["codex.db", "codex-dev.db"];

pub(super) fn remove_local_thread(
    root: &Path,
    id: &str,
    errors: &mut Vec<String>,
) -> Vec<&'static str> {
    let mut changed = Vec::new();
    for name in CATALOGS {
        let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
            let Some((path, no_sidecars)) = catalog_path(root, name)? else {
                return Ok(false);
            };
            delete_entry(&path, id, no_sidecars)
        })();
        match result {
            Ok(true) => changed.push(name),
            Ok(false) => {}
            Err(error) => errors.push(format!("sqlite/{name}: {error}")),
        }
    }
    changed
}

pub(super) fn schedule_desktop_catalog_reconciliation(
    root: &Path,
    changed: Vec<&str>,
    errors: &mut Vec<String>,
) {
    for name in changed {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let (path, _) = catalog_path(root, name)?
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
            let mut db = open_writer(&path)?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            if !tx.query_row(COMPATIBLE_SCHEMA, [], |row| row.get::<_, bool>(0))? {
                return Err(
                    io::Error::other("Schema del catalogo Desktop non riconosciuto.").into(),
                );
            }
            let marker = tx.query_row(
                "SELECT EXISTS (
                    SELECT 1 FROM pragma_table_xinfo('local_thread_catalog_sync_state', 'main')
                    WHERE name = 'last_full_reconciled_at' AND hidden = 0
                )",
                [],
                |row| row.get::<_, bool>(0),
            )?;
            if marker {
                tx.execute(
                    "UPDATE local_thread_catalog_sync_state
                     SET last_full_reconciled_at = NULL
                     WHERE host_id COLLATE BINARY = 'local' AND last_full_reconciled_at IS NOT NULL",
                    [],
                )?;
            }
            tx.commit()?;
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!("sqlite/{name}, riconciliazione sidebar: {error}"));
        }
    }
}

fn catalog_path(root: &Path, name: &str) -> io::Result<Option<(PathBuf, bool)>> {
    // SQLite opens sidecars itself. Verify the archive and the exact known paths.
    agentmender_provider_api::rewrite::directory(root)?;
    let directory = root.join("sqlite");
    match fs::symlink_metadata(&directory) {
        Ok(meta) if meta.is_dir() => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
        _ => return Err(io::Error::other("Directory SQLite protetta.")),
    }
    let database = directory.join(name);
    if !regular_file(&database)? {
        return Ok(None);
    }
    let wal = regular_file(&database.with_extension("db-wal"))?;
    let shm = regular_file(&database.with_extension("db-shm"))?;
    if wal != shm || regular_file(&database.with_extension("db-journal"))? {
        return Err(io::Error::other(
            "File ausiliari SQLite incompleti o occupati.",
        ));
    }
    Ok(Some((database, !wal)))
}

fn regular_file(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && meta.nlink() == 1 => Ok(true),
        Ok(_) => Err(io::Error::other("File SQLite protetto.")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn immutable_uri(path: &Path) -> std::ffi::OsString {
    use std::{fmt::Write, os::unix::ffi::OsStrExt};
    let mut uri = String::from("file:");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(byte as char);
        } else {
            write!(uri, "%{byte:02X}").expect("writing a string cannot fail");
        }
    }
    uri.push_str("?mode=ro&immutable=1");
    uri.into()
}

const OPEN_FLAGS: OpenFlags =
    OpenFlags::SQLITE_OPEN_NOFOLLOW.union(OpenFlags::SQLITE_OPEN_NO_MUTEX);

fn open_writer(path: &Path) -> rusqlite::Result<Connection> {
    let db = Connection::open_with_flags(path, OPEN_FLAGS | OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
    // Leave checkpointing and WAL/SHM cleanup to Desktop.
    db.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
    db.pragma_update(None, "wal_autocheckpoint", 0)?;
    db.pragma_update(None, "temp_store", "memory")?;
    Ok(db)
}

fn delete_entry(
    path: &Path,
    id: &str,
    no_sidecars: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    if no_sidecars {
        // A regular read of a closed WAL database can create sidecars.
        let probe = Connection::open_with_flags(
            immutable_uri(path),
            OPEN_FLAGS | OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )?;
        probe.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
        if !selected_entry(&probe, id)? {
            return Ok(false);
        }
    }

    let mut db = open_writer(path)?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if !selected_entry(&tx, id)? {
        return Ok(false);
    }
    if !tx.query_row(COMPATIBLE_SCHEMA, [], |row| row.get::<_, bool>(0))? {
        return Err(io::Error::other("Schema del catalogo Desktop non riconosciuto.").into());
    }
    let host_verified = tx.query_row(
        "SELECT EXISTS (SELECT 1 FROM local_thread_catalog_hosts
                        WHERE host_id COLLATE BINARY = 'local' AND host_kind COLLATE BINARY = 'local')",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if !host_verified {
        return Err(io::Error::other("Host locale del catalogo Desktop non verificabile.").into());
    }
    let sync_updated = tx.execute(
        "UPDATE local_thread_catalog_sync_state SET observation_sequence = observation_sequence + 1
         WHERE host_id COLLATE BINARY = 'local'",
        [],
    )?;
    let tombstone = tx.execute(
        "INSERT INTO local_thread_catalog_scan_entries (host_id, thread_id, removed)
         VALUES ('local', ?1, 1)
         ON CONFLICT(host_id, thread_id) DO UPDATE SET removed = 1
         WHERE host_id COLLATE BINARY = 'local' AND thread_id COLLATE BINARY = ?1",
        [id],
    )?;
    tx.execute(
        "DELETE FROM local_thread_catalog
         WHERE host_id COLLATE BINARY = 'local' AND thread_id COLLATE BINARY = ?1",
        [id],
    )?;
    let revision_updated = tx.execute(
        "UPDATE local_thread_catalog_metadata SET catalog_revision = catalog_revision + 1 WHERE id = 1",
        [],
    )?;
    if sync_updated != 1 || tombstone != 1 || revision_updated != 1 {
        return Err(
            io::Error::other("Stato o revisione del catalogo Desktop non verificabili.").into(),
        );
    }
    tx.commit()?;
    Ok(true)
}

fn selected_entry(db: &Connection, id: &str) -> rusqlite::Result<bool> {
    if !db.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_table_list
                        WHERE schema = 'main' AND type = 'table' AND name = 'local_thread_catalog')",
        [],
        |row| row.get::<_, bool>(0),
    )? {
        return Ok(false);
    }
    db.query_row(
        "SELECT EXISTS (SELECT 1 FROM local_thread_catalog
                        WHERE host_id COLLATE BINARY = 'local' AND thread_id COLLATE BINARY = ?1)",
        [id],
        |row| row.get(0),
    )
}

// Only ordinary tables with known keys and columns may be mutated. Triggers
// and foreign-key relationships could extend the mutation to other rows.
const COMPATIBLE_SCHEMA: &str = "
    WITH required(tbl, col, pk) AS (
        VALUES ('local_thread_catalog', 'host_id', 1),
               ('local_thread_catalog', 'thread_id', 2),
               ('local_thread_catalog', 'observation_sequence', 0),
               ('local_thread_catalog', 'missing_candidate', 0),
               ('local_thread_catalog_scan_entries', 'host_id', 1),
               ('local_thread_catalog_scan_entries', 'thread_id', 2),
               ('local_thread_catalog_scan_entries', 'removed', 0),
               ('local_thread_catalog_hosts', 'host_id', 1),
               ('local_thread_catalog_hosts', 'host_kind', 0),
               ('local_thread_catalog_metadata', 'id', 1),
               ('local_thread_catalog_metadata', 'catalog_revision', 0),
               ('local_thread_catalog_scan_checkpoints', 'host_id', 1),
               ('local_thread_catalog_scan_checkpoints', 'checkpoint', 0),
               ('local_thread_catalog_sync_state', 'host_id', 1),
               ('local_thread_catalog_sync_state', 'observation_sequence', 0)
    ), mutated(name) AS (
        VALUES ('local_thread_catalog'), ('local_thread_catalog_scan_entries'),
               ('local_thread_catalog_metadata'), ('local_thread_catalog_sync_state')
    )
    SELECT NOT EXISTS (
        SELECT tbl, col, pk FROM required
        EXCEPT
        SELECT t.name, c.name, c.pk FROM pragma_table_list AS t
        JOIN pragma_table_xinfo(t.name, 'main') AS c
        WHERE t.schema = 'main' AND t.type = 'table' AND c.hidden = 0
    )
    AND NOT EXISTS (
        SELECT 1 FROM main.sqlite_schema
        WHERE type = 'trigger' AND tbl_name COLLATE NOCASE IN (SELECT name FROM mutated)
    )
    AND NOT EXISTS (
        SELECT 1 FROM pragma_table_list AS t
        JOIN pragma_foreign_key_list(t.name, 'main') AS fk
        WHERE t.schema = 'main' AND t.type = 'table'
          AND fk.[table] COLLATE NOCASE IN (SELECT name FROM mutated)
    )";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::FileControl;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use tempfile::TempDir;

    const ID: &str = "01234567-89ab-4cde-8f01-23456789abcd";
    const OTHER_ID: &str = "00000000-0000-4000-8000-000000000001";
    const SCHEMA: &str = include_str!("../test_data/desktop-reconciliation.sql");

    fn root() -> TempDir {
        tempfile::tempdir_in("/private/tmp").unwrap()
    }

    fn clean(root: &Path) -> Vec<String> {
        let mut errors = Vec::new();
        let catalogs = remove_local_thread(root, ID, &mut errors);
        schedule_desktop_catalog_reconciliation(root, catalogs, &mut errors);
        errors
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
    fn removes_only_local_thread_and_updates_catalog_scan_and_sidebar() {
        let root = root();
        for name in ["codex.db", "codex-dev.db"] {
            let (path, db) = catalog(root.path(), name);
            insert_entry(&db, "local", ID);
            insert_entry(&db, "local", OTHER_ID);
            insert_entry(&db, "remote", ID);
            db.execute("INSERT INTO local_thread_catalog_scan_entries VALUES ('local', ?1, 0), ('remote', ?1, 0)", [ID]).unwrap();
            if name == "codex.db" {
                db.file_control(None::<&str>, FileControl::PersistWal(&mut 1))
                    .unwrap();
            }
            db.close().unwrap();
            assert!(clean(root.path()).is_empty());
            for suffix in ["-wal", "-shm"] {
                assert!(
                    root.path()
                        .join("sqlite")
                        .join(format!("{name}{suffix}"))
                        .is_file()
                );
            }
            let db = Connection::open(path).unwrap();
            assert_eq!(local_state(&db), (42.5, 1, 8, None));
            for (host, removed) in [("local", 1), ("remote", 0)] {
                assert_eq!(db.query_row(
                    "SELECT removed FROM local_thread_catalog_scan_entries WHERE host_id = ?1 AND thread_id = ?2",
                    [host, ID], |row| row.get::<_, i64>(0),
                ).unwrap(), removed);
            }
            assert_eq!(
                db.query_row(
                    "SELECT catalog_revision FROM local_thread_catalog_metadata",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
                26
            );
            assert_eq!(db.query_row("SELECT checkpoint FROM local_thread_catalog_scan_checkpoints WHERE host_id = 'local'", [], |row| row.get::<_, String>(0)).unwrap(), "in-flight");
            assert!(clean(root.path()).is_empty());
            assert_eq!(local_state(&db), (42.5, 1, 8, None));
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
                [("local", OTHER_ID), ("remote", ID)]
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
        assert!(clean(root.path()).is_empty());
        assert!(!path.with_extension("db-wal").exists());
        assert!(!path.with_extension("db-shm").exists());
        assert!(!root.path().join("sqlite/codex-dev.db").exists());
        let db = Connection::open(path).unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
    }

    #[test]
    fn unknown_or_side_effecting_schema_is_preserved() {
        let root = root();
        for ddl in [
            "ALTER TABLE local_thread_catalog_metadata DROP COLUMN catalog_revision",
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
            let errors = clean(case.path());
            assert_eq!(errors.is_empty(), ddl.contains("RENAME TO"));
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
        assert_eq!(clean(&linked_root).len(), 2);
        assert_eq!(
            clean(&intermediate_link.join(outside.path().file_name().unwrap())).len(),
            2
        );
        assert_eq!(clean(&real_root.join("..").join("linked")).len(), 2);
        assert_eq!(clean(Path::new("sqlite")).len(), 2);

        let sqlite = real_root.join("sqlite");
        symlink(outside.path().join("sqlite"), &sqlite).unwrap();
        assert!(!clean(&real_root).is_empty());
        fs::remove_file(&sqlite).unwrap();
        fs::create_dir(&sqlite).unwrap();
        let candidate = sqlite.join("codex.db");
        let (unknown, unknown_db) = catalog(&real_root, "codex-other.db");
        insert_entry(&unknown_db, "local", ID);
        unknown_db.close().unwrap();
        let unknown_before = fs::read(&unknown).unwrap();
        symlink(&db_path, &candidate).unwrap();
        assert!(!clean(&real_root).is_empty());
        fs::remove_file(&candidate).unwrap();
        fs::hard_link(&db_path, &candidate).unwrap();
        assert!(!clean(&real_root).is_empty());
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
            assert_eq!(clean(root.path()).len(), 1);
            assert!(sidecar.is_symlink());
            assert_eq!(fs::read(&sentinel).unwrap(), b"outside");
            fs::remove_file(sidecar).unwrap();
        }
        let db = Connection::open(path).unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
    }

    #[test]
    fn waits_for_a_temporarily_busy_catalog() {
        let root = root();
        let (path, mut db) = catalog(root.path(), "codex.db");
        insert_entry(&db, "local", ID);
        let writer = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        std::thread::scope(|scope| {
            let cleanup = scope.spawn(|| clean(root.path()));
            std::thread::sleep(Duration::from_millis(200));
            writer.commit().unwrap();
            assert!(cleanup.join().unwrap().is_empty());
        });
        assert_eq!(local_state(&db), (42.5, 1, 8, None));
        let connection = open_writer(&path).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "busy_timeout", |row| row.get::<_, i64>(0))
                .unwrap(),
            5000
        );
    }

    #[test]
    fn unavailable_catalog_is_skipped() {
        let root = root();
        assert!(clean(root.path()).is_empty());
        assert!(!root.path().join("sqlite").exists());
        let (path, mut db) = catalog(root.path(), "codex.db");
        insert_entry(&db, "local", ID);
        let writer = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        assert_eq!(clean(root.path()).len(), 1);
        writer.rollback().unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
        db.close().unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        assert_eq!(clean(root.path()).len(), 1);
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(local_state(&db), (42.5, 1, 7, Some(123)));
    }
}
