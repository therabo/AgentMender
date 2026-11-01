//! Protected, bounded SQLite observations for the separate scoped-cleanup profile.
//! This worker does not assert rollout ownership, readiness, or mutation authority.
use crate::capture::{self, CaptureRequest, ExactProfile};
use agentmender_provider_api::ProviderError;
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedCaptureRequest {
    pub source_root: PathBuf,
    pub mirror_root: PathBuf,
    pub probe_path: PathBuf,
    pub target_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedRowCommitment {
    pub table: String,
    pub count: u64,
    pub sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedKnownThread {
    pub id: String,
    pub rollout_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedCaptureResult {
    pub schema_sha256: String,
    pub journal_mode: String,
    pub target_path: Option<PathBuf>,
    pub history_mode: Option<String>,
    pub archived: Option<bool>,
    pub cwd: Option<PathBuf>,
    pub dependent_rows: u64,
    pub rows: Vec<ScopedRowCommitment>,
    pub known_threads: Vec<ScopedKnownThread>,
    pub retained_database_sha256: String,
}

/// Only fixed identity and locator columns cross the protected boundary.
pub(super) fn known_threads(db: &Connection) -> Result<Vec<ScopedKnownThread>, ProviderError> {
    let mut stmt = db
        .prepare("SELECT id, rollout_path FROM threads ORDER BY id")
        .map_err(|_| unavailable())?;
    let mut rows = stmt.query([]).map_err(|_| unavailable())?;
    let mut values = Vec::new();
    let mut bytes = 0usize;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        let id: String = row.get(0).map_err(|_| unavailable())?;
        let path: String = row.get(1).map_err(|_| unavailable())?;
        bytes = bytes
            .checked_add(id.len())
            .and_then(|n| n.checked_add(path.len()))
            .ok_or_else(unavailable)?;
        if !canonical_uuid(&id) || path.is_empty() || values.len() >= 100_000 || bytes > 256 * 1024
        {
            return Err(unavailable());
        }
        values.push(ScopedKnownThread {
            id,
            rollout_path: path.into(),
        });
    }
    Ok(values)
}

fn valid_known_threads(rows: &[ScopedKnownThread], root: &std::path::Path) -> bool {
    use std::{collections::BTreeSet, path::Component};
    if rows.len() > 100_000 {
        return false;
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut bytes = 0usize;
    for row in rows {
        let Some(path) = row.rollout_path.to_str() else {
            return false;
        };
        let Some(next) = bytes
            .checked_add(row.id.len())
            .and_then(|n| n.checked_add(path.len()))
        else {
            return false;
        };
        bytes = next;
        if bytes > 256 * 1024
            || !canonical_uuid(&row.id)
            || !row.rollout_path.is_absolute()
            || row
                .rollout_path
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || !(row.rollout_path.starts_with(root.join("sessions"))
                || row.rollout_path.starts_with(root.join("archived_sessions")))
            || !ids.insert(&row.id)
            || !paths.insert(&row.rollout_path)
        {
            return false;
        }
    }
    true
}

fn unavailable() -> ProviderError {
    ProviderError::CapabilityUnavailable("scoped SQLite evidence rejected or unavailable".into())
}

fn canonical_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

// A length-delimited, typed encoding is hashed in memory; no SQLite values or
// transcript bodies cross the worker boundary. SQL and ordering are fixed here.
pub(super) fn commitment(
    db: &Connection,
    target: &str,
    table: &str,
    sql: &str,
) -> Result<ScopedRowCommitment, ProviderError> {
    let mut statement = db.prepare(sql).map_err(|_| unavailable())?;
    let columns = statement.column_count();
    let mut rows = statement.query([target]).map_err(|_| unavailable())?;
    let mut hash = Sha256::new();
    hash.update(b"agentmender/scoped-sql-rows/v1\0");
    hash.update(table.as_bytes());
    let mut count = 0u64;
    let mut size = 0u64;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        count = count.checked_add(1).ok_or_else(unavailable)?;
        if count > 100_000 {
            return Err(unavailable());
        }
        for index in 0..columns {
            let value = row.get_ref(index).map_err(|_| unavailable())?;
            let (tag, bytes): (u8, Vec<u8>) = match value {
                ValueRef::Null => (0, Vec::new()),
                ValueRef::Integer(n) => (1, n.to_be_bytes().to_vec()),
                ValueRef::Real(n) => (2, n.to_bits().to_be_bytes().to_vec()),
                ValueRef::Text(v) => (3, v.to_vec()),
                ValueRef::Blob(v) => (4, v.to_vec()),
            };
            size = size
                .checked_add(bytes.len() as u64)
                .ok_or_else(unavailable)?;
            if size > 32 * 1024 * 1024 {
                return Err(unavailable());
            }
            hash.update([tag]);
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
    }
    Ok(ScopedRowCommitment {
        table: table.into(),
        count,
        sha256: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
    })
}

pub(super) fn snapshot(
    source_root: &std::path::Path,
    target: &str,
) -> Result<ScopedCaptureResult, ProviderError> {
    if !canonical_uuid(target) {
        return Err(unavailable());
    }
    let before = capture::fingerprints(source_root)?;
    let source = Connection::open_with_flags(
        source_root.join(capture::DB_NAME),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| unavailable())?;
    capture::configure(&source)?;
    source
        .execute_batch("PRAGMA query_only=ON")
        .map_err(|_| unavailable())?;
    let journal_mode: String = source
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(|_| unavailable())?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(unavailable());
    }
    let mut memory = Connection::open_in_memory().map_err(|_| unavailable())?;
    capture::configure(&memory)?;
    capture::bounded_backup(
        &source,
        &mut memory,
        Instant::now() + Duration::from_secs(3),
    )?;
    drop(source);
    // The schemas have the same reviewed fingerprint; this is not a native profile lookup.
    capture::verify_schema(&memory, ExactProfile::V165)?;
    let queries = [
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
    let rows = queries
        .iter()
        .map(|(name, sql)| commitment(&memory, target, name, sql))
        .collect::<Result<Vec<_>, _>>()?;
    let (target_path, history_mode, archived, cwd) = if rows[0].count == 1 {
        let (path, history, archived, cwd, version): (String, String, i64, String, String) = memory
            .query_row(
                "SELECT rollout_path, history_mode, archived, cwd, cli_version FROM threads WHERE id=?1",
                [target],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|_| unavailable())?;
        if version != crate::scoped_profile::SCOPED_CODEX_VERSION {
            return Err(unavailable());
        }
        let archived = match archived {
            0 => false,
            1 => true,
            _ => return Err(unavailable()),
        };
        (
            Some(PathBuf::from(path)),
            Some(history),
            Some(archived),
            Some(PathBuf::from(cwd)),
        )
    } else {
        (None, None, None, None)
    };
    let dependent_rows: i64 = if let Some(path) = &target_path {
        memory.query_row("SELECT count(*) FROM threads WHERE id<>?1 AND (rollout_path=?2 OR attached_thread_parent_id=?1)", (target, path.to_str().ok_or_else(unavailable)?), |r| r.get(0)).map_err(|_| unavailable())?
    } else {
        0
    };
    let dependent_rows = u64::try_from(dependent_rows).map_err(|_| unavailable())?;
    let known_threads = known_threads(&memory)?;
    let retained_database_sha256 =
        crate::scoped_storage::retained_digest(&memory, target).map_err(|_| unavailable())?;
    drop(memory);
    let after = capture::fingerprints(source_root)?;
    // SQLite may change SHM read marks while reading. The bounded snapshot is
    // accepted only if the database and WAL themselves remain byte-stable.
    if capture::fingerprints_changed(&before[..2], &after[..2]) {
        return Err(unavailable());
    }
    Ok(ScopedCaptureResult {
        schema_sha256: crate::scoped_profile::SCOPED_CODEX_SCHEMA_SHA256.into(),
        journal_mode: "wal".into(),
        target_path,
        history_mode,
        archived,
        cwd,
        dependent_rows,
        rows,
        known_threads,
        retained_database_sha256,
    })
}

/// Fixed internal CLI worker; source access is permitted only after sandbox validation.
pub fn run_scoped_evidence_worker() -> Result<(), ProviderError> {
    #[cfg(target_os = "macos")]
    {
        use std::io::{Read, Write};
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(8193)
            .read_to_end(&mut bytes)
            .map_err(|_| unavailable())?;
        if bytes.len() > 8192 {
            return Err(unavailable());
        }
        let request: ScopedCaptureRequest =
            serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        let validation = CaptureRequest {
            profile: ExactProfile::V165,
            source_root: request.source_root.clone(),
            mirror_root: request.mirror_root,
            probe_path: request.probe_path,
        };
        capture::validate_worker_request(&validation)?;
        let result = snapshot(&request.source_root, &request.target_id)?;
        let output = serde_json::to_vec(&result).map_err(|_| unavailable())?;
        if output.len() > 512 * 1024 {
            return Err(unavailable());
        }
        std::io::stdout()
            .write_all(&output)
            .map_err(|_| unavailable())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(unavailable())
    }
}

/// Parse only the fixed worker response after a bounded child exchange.
/// The caller must have launched `child` through the OS-denied capture sandbox.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(super) fn read_worker_result(
    child: std::process::Child,
    request: &ScopedCaptureRequest,
) -> Result<ScopedCaptureResult, ProviderError> {
    use std::io::{Read, Write};
    use std::sync::mpsc;
    let mut child = crate::Guard(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let request_bytes = serde_json::to_vec(request).map_err(|_| unavailable())?;
    if request_bytes.len() > 8192 {
        return Err(unavailable());
    }
    let mut input = child.0.stdin.take().ok_or_else(unavailable)?;
    let mut output = child.0.stdout.take().ok_or_else(unavailable)?;
    let (sent, written) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = input.write_all(&request_bytes);
        drop(input);
        let _ = sent.send(result);
    });
    let (sent, read) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .by_ref()
            .take(512 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sent.send(result);
    });
    written
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    let bytes = read
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    if bytes.len() > 512 * 1024 {
        return Err(unavailable());
    }
    loop {
        match child.0.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => return Err(unavailable()),
        }
    }
    let result: ScopedCaptureResult = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    let expected = [
        "threads",
        "thread_dynamic_tools",
        "thread_read_receipts",
        "thread_attachments",
        "guardian_review_feedback",
        "thread_spawn_edges",
    ];
    if result.schema_sha256 != crate::scoped_profile::SCOPED_CODEX_SCHEMA_SHA256
        || !valid_known_threads(&result.known_threads, &request.source_root)
        || (result.rows.first().is_some_and(|r| r.count == 1)
            && !result.known_threads.iter().any(|row| {
                row.id == request.target_id
                    && Some(&row.rollout_path) == result.target_path.as_ref()
            }))
        || result.journal_mode != "wal"
        || result.retained_database_sha256.len() != 64
        || !result
            .retained_database_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || result.rows.len() != expected.len()
        || result.rows.iter().zip(expected).any(|(row, name)| {
            row.table != name
                || row.count > 100_000
                || row.sha256.len() != 64
                || !row
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        || result
            .target_path
            .as_ref()
            .is_some_and(|path| !path.is_absolute() || !path.starts_with(&request.source_root))
        || result.target_path.is_some() != (result.rows[0].count == 1)
        || result.history_mode.is_some() != (result.rows[0].count == 1)
        || result.archived.is_some() != (result.rows[0].count == 1)
        || result.cwd.is_some() != (result.rows[0].count == 1)
        || result.cwd.as_ref().is_some_and(|cwd| !cwd.is_absolute())
        || result
            .history_mode
            .as_ref()
            .is_some_and(|mode| mode.len() > 32)
    {
        return Err(unavailable());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_known_thread_metadata_rejects_outside_and_duplicate_paths() {
        let root = std::path::Path::new("/fixture");
        let row = |id: &str, path: &str| ScopedKnownThread {
            id: id.into(),
            rollout_path: path.into(),
        };
        let target = "12345678-1234-1234-1234-123456789abc";
        let other = "12345678-1234-1234-1234-123456789abd";
        assert!(valid_known_threads(
            &[row(target, "/fixture/sessions/target.jsonl")],
            root
        ));
        assert!(!valid_known_threads(
            &[row(target, "/elsewhere/foreign.jsonl")],
            root
        ));
        assert!(!valid_known_threads(
            &[
                row(target, "/fixture/sessions/target.jsonl"),
                row(other, "/fixture/sessions/target.jsonl")
            ],
            root
        ));
        assert!(!valid_known_threads(
            &[row(target, "/fixture/sessions/../outside")],
            root
        ));
    }

    fn fixture() -> (tempfile::TempDir, Connection) {
        let root = tempfile::tempdir().unwrap();
        let db = Connection::open(root.path().join(capture::DB_NAME)).unwrap();
        db.pragma_update(None, "journal_mode", "WAL").unwrap();
        db.execute_batch("CREATE TABLE fixture_sequence_seed(value INTEGER PRIMARY KEY AUTOINCREMENT); DROP TABLE fixture_sequence_seed;").unwrap();
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
        db.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,cli_version,first_user_message,preview) VALUES (?1,'/fixture/history.jsonl',1,2,'cli','openai','/fixture','secret title','read-only','never','0.166.0','secret message','secret message')", ["12345678-1234-1234-1234-123456789abc"]).unwrap();
        (root, db)
    }

    #[test]
    fn scoped_snapshot_contains_only_bounded_metadata_and_commitments() {
        let (root, _db) = fixture();
        let result = snapshot(
            &root.path().canonicalize().unwrap(),
            "12345678-1234-1234-1234-123456789abc",
        )
        .unwrap();
        assert_eq!(result.rows[0].count, 1);
        assert_eq!(result.history_mode.as_deref(), Some("legacy"));
        let encoded = serde_json::to_string(&result).unwrap();
        assert!(!encoded.contains("secret title"));
        assert!(!encoded.contains("secret message"));
        assert!(
            snapshot(
                &root.path().canonicalize().unwrap(),
                "12345678-1234-1234-1234-123456789abC"
            )
            .is_err()
        );
    }

    #[test]
    fn schema_drift_and_rollback_journal_are_rejected() {
        let (root, db) = fixture();
        db.execute_batch("CREATE TABLE unexpected(value TEXT)")
            .unwrap();
        drop(db);
        assert!(
            snapshot(
                &root.path().canonicalize().unwrap(),
                "12345678-1234-1234-1234-123456789abc"
            )
            .is_err()
        );
        let (root, db) = fixture();
        db.pragma_update(None, "journal_mode", "DELETE").unwrap();
        drop(db);
        assert!(
            snapshot(
                &root.path().canonicalize().unwrap(),
                "12345678-1234-1234-1234-123456789abc"
            )
            .is_err()
        );
    }
}
