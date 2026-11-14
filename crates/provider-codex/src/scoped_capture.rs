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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub desktop_closed_snapshot: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_session_metadata: bool,
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
    pub expanded_retained_database_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_metadata: Option<agentmender_domain::SessionInventory>,
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

/// Deliberately excludes preview, first_user_message, rollout bodies and full cwd.
fn metadata_inventory(
    db: &Connection,
    root: &std::path::Path,
) -> Result<agentmender_domain::SessionInventory, ProviderError> {
    use agentmender_domain::*;
    let mut stmt = db.prepare("SELECT id, title, cwd, created_at, updated_at, archived, cli_version FROM threads ORDER BY id LIMIT 501").map_err(|_| unavailable())?;
    let mut rows = stmt.query([]).map_err(|_| unavailable())?;
    let mut sessions = Vec::new();
    let mut completeness = Completeness::Complete;
    let mut bytes = 0;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        if sessions.len() == 500 {
            completeness = Completeness::Partial;
            break;
        }
        let id: String = row.get(0).map_err(|_| unavailable())?;
        if !canonical_uuid(&id) {
            return Err(unavailable());
        }
        let title: String = row.get(1).map_err(|_| unavailable())?;
        let cwd: String = row.get(2).map_err(|_| unavailable())?;
        let title = (!title.trim().is_empty() && title.len() <= 1024).then_some(title);
        let project_label = std::path::Path::new(&cwd)
            .file_name()
            .and_then(|p| p.to_str())
            .filter(|p| !p.is_empty() && p.len() <= 256)
            .map(str::to_owned);
        bytes += title.as_ref().map_or(0, String::len)
            + project_label.as_ref().map_or(0, String::len)
            + 256;
        if bytes > 128 * 1024 {
            completeness = Completeness::Partial;
            break;
        }
        let archived: i64 = row.get(5).map_err(|_| unavailable())?;
        let version: String = row.get(6).map_err(|_| unavailable())?;
        let timestamp = |index| -> Result<Option<i64>, ProviderError> {
            let value: i64 = row.get(index).map_err(|_| unavailable())?;
            Ok((value >= 0).then_some(value))
        };
        sessions.push(SessionMetadata {
            identity: ArtifactId {
                provider: ProviderKind::Codex,
                state_root: root.into(),
                class: GraphNodeKind::Session,
                local_id: crate::scoped_id("session", &id)?,
            },
            native_session_id: id,
            title,
            project_label,
            created_at: timestamp(3)?,
            last_active_at: timestamp(4)?,
            state: match archived {
                0 => SessionState::Active,
                1 => SessionState::Archived,
                _ => SessionState::Unknown,
            },
            producer_version_matches: version == crate::scoped_profile::SCOPED_CODEX_VERSION,
            producer_version: None,
            parent_native_session_id: None,
        });
    }
    Ok(SessionInventory {
        sessions,
        completeness,
        read_compatibility: None,
    })
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

#[cfg(unix)]
type DesktopStateIdentity = (u64, u64, u64, i64, i64, i64, i64);

// For Desktop only, a closed checkpointed WAL database may have no sidecars.
// Observe fixed names through a no-follow root descriptor and reject journals,
// hard links, cross-device database files and non-WAL SQLite headers.
#[cfg(unix)]
#[allow(clippy::unnecessary_cast)]
fn desktop_closed_state(
    root: &std::path::Path,
) -> Result<(bool, Vec<DesktopStateIdentity>), ProviderError> {
    use rustix::fs::{self, AtFlags, Mode, OFlags};
    use std::{io::Read, os::unix::fs::MetadataExt};
    if !root.is_absolute() || root.canonicalize().map_err(|_| unavailable())? != root {
        return Err(unavailable());
    }
    let fd = fs::open(root, capture::directory_search_flags(), Mode::empty())
        .map_err(|_| unavailable())?;
    let root_meta = std::fs::File::from(
        fs::openat(&fd, ".", capture::directory_search_flags(), Mode::empty())
            .map_err(|_| unavailable())?,
    )
    .metadata()
    .map_err(|_| unavailable())?;
    let path_meta = std::fs::symlink_metadata(root).map_err(|_| unavailable())?;
    if !path_meta.is_dir()
        || path_meta.dev() != root_meta.dev()
        || path_meta.ino() != root_meta.ino()
    {
        return Err(unavailable());
    }
    let database = std::fs::File::from(
        fs::openat(
            &fd,
            capture::DB_NAME,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| unavailable())?,
    );
    let db_meta = database.metadata().map_err(|_| unavailable())?;
    let entry =
        fs::statat(&fd, capture::DB_NAME, AtFlags::SYMLINK_NOFOLLOW).map_err(|_| unavailable())?;
    if !db_meta.is_file()
        || db_meta.nlink() != 1
        || db_meta.dev() != root_meta.dev()
        || db_meta.dev() != entry.st_dev as u64
        || db_meta.ino() != entry.st_ino as u64
    {
        return Err(unavailable());
    }
    let mut header = [0u8; 20];
    (&database)
        .read_exact(&mut header)
        .map_err(|_| unavailable())?;
    if &header[..16] != b"SQLite format 3\0" || header[18] != 2 || header[19] != 2 {
        return Err(unavailable());
    }
    let missing = |name: &str| match fs::statat(&fd, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(rustix::io::Errno::NOENT) => Ok(true),
        Ok(_) => Ok(false),
        Err(_) => Err(unavailable()),
    };
    if !missing("state_5.sqlite-journal")? {
        return Err(unavailable());
    }
    let absent = missing("state_5.sqlite-wal")? && missing("state_5.sqlite-shm")?;
    let record = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    Ok((absent, vec![record(&root_meta), record(&db_meta)]))
}

#[cfg(unix)]
pub(super) fn immutable_uri(path: &std::path::Path) -> std::ffi::OsString {
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

#[cfg(test)]
pub(super) fn snapshot(
    source_root: &std::path::Path,
    target: &str,
) -> Result<ScopedCaptureResult, ProviderError> {
    snapshot_inner(source_root, target, false, false)
}

#[cfg(all(test, unix))]
pub(super) fn desktop_snapshot(
    source_root: &std::path::Path,
    target: &str,
) -> Result<ScopedCaptureResult, ProviderError> {
    snapshot_inner(source_root, target, true, false)
}

pub(super) fn snapshot_inner(
    source_root: &std::path::Path,
    target: &str,
    desktop: bool,
    include_session_metadata: bool,
) -> Result<ScopedCaptureResult, ProviderError> {
    if !canonical_uuid(target) {
        return Err(unavailable());
    }
    #[cfg(unix)]
    let desktop_before = if desktop {
        Some(desktop_closed_state(source_root)?)
    } else {
        None
    };
    #[cfg(not(unix))]
    if desktop {
        return Err(unavailable());
    }
    let before = capture::fingerprints(source_root)?;
    #[cfg(unix)]
    let immutable = desktop_before.as_ref().is_some_and(|state| state.0);
    #[cfg(not(unix))]
    let immutable = false;
    #[cfg(unix)]
    if let Some(state) = &desktop_before
        && desktop_closed_state(source_root)? != *state
    {
        return Err(unavailable());
    }
    #[cfg(unix)]
    let source_path = if immutable {
        std::path::PathBuf::from(immutable_uri(&source_root.join(capture::DB_NAME)))
    } else {
        source_root.join(capture::DB_NAME)
    };
    #[cfg(not(unix))]
    let source_path = source_root.join(capture::DB_NAME);
    let source = Connection::open_with_flags(
        source_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | if immutable {
                OpenFlags::SQLITE_OPEN_URI
            } else {
                OpenFlags::empty()
            },
    )
    .map_err(|_| unavailable())?;
    capture::configure(&source)?;
    source
        .execute_batch("PRAGMA query_only=ON")
        .map_err(|_| unavailable())?;
    let journal_mode: String = source
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .map_err(|_| unavailable())?;
    if !immutable && !journal_mode.eq_ignore_ascii_case("wal") {
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
    let session_metadata = include_session_metadata
        .then(|| metadata_inventory(&memory, source_root))
        .transpose()?;
    let retained_database_sha256 =
        crate::scoped_storage::retained_digest(&memory, target).map_err(|_| unavailable())?;
    let expanded_retained_database_sha256 =
        crate::scoped_storage::retained_digest_for_profile(&memory, target, true)
            .map_err(|_| unavailable())?;
    drop(memory);
    #[cfg(unix)]
    if immutable
        && desktop_closed_state(source_root)? != *desktop_before.as_ref().ok_or_else(unavailable)?
    {
        return Err(unavailable());
    }
    let after = capture::fingerprints(source_root)?;
    #[cfg(unix)]
    if immutable
        && desktop_closed_state(source_root)? != *desktop_before.as_ref().ok_or_else(unavailable)?
    {
        return Err(unavailable());
    }
    // SQLite may change SHM read marks while reading. The bounded snapshot is
    // accepted only if the database and WAL themselves remain byte-stable.
    if capture::fingerprints_changed(
        if immutable { &before[..] } else { &before[..2] },
        if immutable { &after[..] } else { &after[..2] },
    ) {
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
        expanded_retained_database_sha256,
        session_metadata,
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
        let result = snapshot_inner(
            &request.source_root,
            &request.target_id,
            request.desktop_closed_snapshot,
            request.include_session_metadata,
        )?;
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
        || result.session_metadata.is_some() != request.include_session_metadata
        || result.session_metadata.as_ref().is_some_and(|inventory| {
            inventory.sessions.len() > 500
                || inventory.sessions.iter().any(|s| {
                    s.identity.provider != agentmender_domain::ProviderKind::Codex
                        || s.identity.state_root != request.source_root
                        || s.identity.class != agentmender_domain::GraphNodeKind::Session
                        || s.identity.local_id.as_str()
                            != format!("codex:session:{}", s.native_session_id)
                        || !result
                            .known_threads
                            .iter()
                            .any(|r| r.id == s.native_session_id)
                        || s.title.as_ref().is_some_and(|v| v.len() > 1024)
                        || s.project_label.as_ref().is_some_and(|v| v.len() > 256)
                })
        })
        || !valid_known_threads(&result.known_threads, &request.source_root)
        || (result.rows.first().is_some_and(|r| r.count == 1)
            && !result.known_threads.iter().any(|row| {
                row.id == request.target_id
                    && Some(&row.rollout_path) == result.target_path.as_ref()
            }))
        || result.journal_mode != "wal"
        || result.expanded_retained_database_sha256.len() != 64
        || !result
            .expanded_retained_database_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
    fn opt_in_metadata_bounds_disclose_partial_without_transcript_fields() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE threads(id TEXT,title TEXT,cwd TEXT,created_at INTEGER,updated_at INTEGER,archived INTEGER,cli_version TEXT,preview TEXT)").unwrap();
        for i in 0..501 {
            db.execute("INSERT INTO threads VALUES (?1,?2,'/PRIVATE/mission',1,2,1,'0.166.0','SECRET PREVIEW')", (format!("00000000-0000-0000-0000-{i:012x}"), if i == 0 { "x".repeat(1025) } else { "README".into() })).unwrap();
        }
        let inventory = metadata_inventory(&db, std::path::Path::new("/owned")).unwrap();
        assert_eq!(
            inventory.completeness,
            agentmender_domain::Completeness::Partial
        );
        assert!(inventory.sessions.len() <= 500);
        assert!(inventory.sessions[0].title.is_none());
        assert_eq!(
            inventory.sessions[0].project_label.as_deref(),
            Some("mission")
        );
        let encoded = serde_json::to_string(&inventory).unwrap();
        assert!(!encoded.contains("SECRET") && !encoded.contains("PRIVATE"));
    }

    #[test]
    fn worker_known_thread_metadata_rejects_outside_and_duplicate_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let target_path = root.join("sessions/target.jsonl");
        let outside = root.parent().unwrap().join("outside-fixture/foreign.jsonl");
        let traversal = root.join("sessions/../outside");
        let row = |id: &str, path: &std::path::Path| ScopedKnownThread {
            id: id.into(),
            rollout_path: path.into(),
        };
        let target = "12345678-1234-1234-1234-123456789abc";
        let other = "12345678-1234-1234-1234-123456789abd";
        assert!(valid_known_threads(&[row(target, &target_path)], &root));
        assert!(!valid_known_threads(&[row(target, &outside)], &root));
        assert!(!valid_known_threads(
            &[row(target, &target_path), row(other, &target_path)],
            &root
        ));
        assert!(!valid_known_threads(&[row(target, &traversal)], &root));
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

    #[cfg(unix)]
    #[test]
    fn desktop_closed_wal_capture_requires_clean_stable_fixed_files() {
        let (root, db) = fixture();
        drop(db);
        let root = root.path().canonicalize().unwrap();
        let target = "12345678-1234-1234-1234-123456789abc";
        assert!(desktop_closed_state(&root).unwrap().0);
        let snapshot = desktop_snapshot(&root, target).unwrap();
        assert_eq!(snapshot.rows[0].count, 1);
        std::fs::write(root.join("state_5.sqlite-journal"), b"unexpected").unwrap();
        assert!(desktop_snapshot(&root, target).is_err());
        std::fs::remove_file(root.join("state_5.sqlite-journal")).unwrap();
        std::fs::write(root.join("state_5.sqlite-wal"), b"unexpected").unwrap();
        assert!(!desktop_closed_state(&root).unwrap().0);
    }

    #[test]
    fn historical_worker_request_omits_desktop_selector() {
        let request = ScopedCaptureRequest {
            source_root: "/source".into(),
            mirror_root: "/mirror".into(),
            probe_path: "/probe".into(),
            target_id: "12345678-1234-1234-1234-123456789abc".into(),
            desktop_closed_snapshot: false,
            include_session_metadata: false,
        };
        assert!(
            !serde_json::to_string(&request)
                .unwrap()
                .contains("desktopClosedSnapshot")
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
