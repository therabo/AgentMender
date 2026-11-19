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
