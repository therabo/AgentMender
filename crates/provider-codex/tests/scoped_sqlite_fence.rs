//! Disposable SQLite concurrency checks for the proposed scoped cleanup fence.
//! These tests exercise SQLite itself; they do not enable provider mutation.
use rusqlite::{Connection, Error, ErrorCode};
use std::time::Duration;
use tempfile::TempDir;

fn fixture() -> (TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.sqlite");
    let db = Connection::open(&path).unwrap();
    db.pragma_update(None, "journal_mode", "WAL").unwrap();
    db.execute_batch(
        "CREATE TABLE threads (id TEXT PRIMARY KEY); INSERT INTO threads VALUES ('a');",
    )
    .unwrap();
    drop(db);
    (root, path)
}

fn open(path: &std::path::Path) -> Connection {
    let db = Connection::open(path).unwrap();
    db.busy_timeout(Duration::ZERO).unwrap();
    db
}

fn is_busy(result: rusqlite::Result<()>) -> bool {
    matches!(result, Err(Error::SqliteFailure(e, _)) if matches!(e.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked))
}

// `locking_mode` must be set before the first access to the main database.
fn acquire_exclusive(db: &Connection) -> rusqlite::Result<()> {
    let mode: String = db.query_row("PRAGMA locking_mode=EXCLUSIVE", [], |r| r.get(0))?;
    assert_eq!(mode.to_ascii_lowercase(), "exclusive");
    db.execute_batch("BEGIN EXCLUSIVE; SELECT count(*) FROM sqlite_master; COMMIT")
}

#[test]
fn attached_idle_wal_connection_prevents_actual_exclusion() {
    let (_root, path) = fixture();
    let existing = open(&path);
    let _: i64 = existing
        .query_row("SELECT count(*) FROM threads", [], |r| r.get(0))
        .unwrap();
    let candidate = open(&path);
    assert!(is_busy(acquire_exclusive(&candidate)));
    drop(existing);
    // Failed acquisition is not a receipt: use a new connection for a new attempt.
    drop(candidate);
    let candidate = open(&path);
    acquire_exclusive(&candidate).unwrap();
}

#[test]
fn exclusion_survives_commit_and_blocks_new_reader_until_close() {
    let (_root, path) = fixture();
    let candidate = open(&path);
    acquire_exclusive(&candidate).unwrap();
    candidate
        .execute_batch("BEGIN EXCLUSIVE; DELETE FROM threads WHERE id='a'; COMMIT")
        .unwrap();
    let other = open(&path);
    let result = other
        .query_row("SELECT count(*) FROM threads", [], |r| r.get::<_, i64>(0))
        .map(|_| ());
    assert!(is_busy(result));
    drop(candidate);
    assert_eq!(
        other
            .query_row("SELECT count(*) FROM threads", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn begin_exclusive_alone_does_not_exclude_new_wal_readers() {
    let (_root, path) = fixture();
    let writer = open(&path);
    writer.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let reader = open(&path);
    assert_eq!(
        reader
            .query_row("SELECT count(*) FROM threads", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

// Invoked as a separate test process by the test below. The environment value
// names only a disposable fixture created by that test.
#[test]
fn child_reader() {
    let Some(path) = std::env::var_os("AGENTMENDER_TEST_SQLITE_FENCE_PATH") else {
        return;
    };
    let reader = open(std::path::Path::new(&path));
    let result = reader
        .query_row("SELECT count(*) FROM threads", [], |r| r.get::<_, i64>(0))
        .map(|_| ());
    assert!(is_busy(result));
}

#[test]
fn exclusion_also_blocks_a_reader_in_another_process() {
    let (_root, path) = fixture();
    let candidate = open(&path);
    acquire_exclusive(&candidate).unwrap();
    candidate.execute_batch("BEGIN EXCLUSIVE; COMMIT").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("child_reader")
        .env("AGENTMENDER_TEST_SQLITE_FENCE_PATH", &path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "child reader did not observe the fence"
    );
}
