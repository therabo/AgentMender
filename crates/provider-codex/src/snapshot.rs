//! Read-only bounded SQLite snapshots in the kernel-attested worker.
use agentmender_provider_api::{ProviderError, filesystem};
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
    limits::Limit,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

const DB_NAME: &str = "state_5.sqlite";
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_DATABASE_BYTES: u64 = 32 * 1024 * 1024;
const CAPTURE_TIME: Duration = Duration::from_secs(3);
const SOURCE_NAMES: [&str; 3] = [DB_NAME, "state_5.sqlite-wal", "state_5.sqlite-shm"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FileFingerprint {
    name: String,
    record: Option<FileRecord>,
    stable_read: bool,
}

#[cfg(target_os = "macos")]
pub(super) fn same_primary_identity(before: &[FileFingerprint], after: &[FileFingerprint]) -> bool {
    match (before.first(), after.first()) {
        (Some(a), Some(b))
            if a.name == "state_5.sqlite" && b.name == a.name && a.stable_read && b.stable_read =>
        {
            match (&a.record, &b.record) {
                (Some(a), Some(b)) => (a.device, a.inode, a.mode) == (b.device, b.inode, b.mode),
                _ => false,
            }
        }
        _ => false,
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileRecord {
    bytes: u64,
    hash: String,
    inode: u64,
    device: u64,
    mode: u32,
    modified: (i64, i64),
    changed: (i64, i64),
}

fn unavailable(message: &str) -> ProviderError {
    ProviderError::CapabilityUnavailable(message.into())
}
fn sqlite_error(_: rusqlite::Error) -> ProviderError {
    // SQLite errors can contain private paths or values.
    unavailable("SQLite evidence capture rejected or unavailable")
}
fn check_deadline(deadline: Instant) -> Result<(), ProviderError> {
    if Instant::now() >= deadline {
        Err(unavailable("evidence capture deadline exceeded"))
    } else {
        Ok(())
    }
}
fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(unix)]
fn metadata_record(meta: &fs::Metadata, hash: String) -> FileRecord {
    use std::os::unix::fs::MetadataExt;
    FileRecord {
        bytes: meta.len(),
        hash,
        inode: meta.ino(),
        device: meta.dev(),
        mode: meta.mode(),
        modified: (meta.mtime(), meta.mtime_nsec()),
        changed: (meta.ctime(), meta.ctime_nsec()),
    }
}
#[cfg(not(unix))]
fn metadata_record(meta: &fs::Metadata, hash: String) -> FileRecord {
    FileRecord {
        bytes: meta.len(),
        hash,
        inode: 0,
        device: 0,
        mode: 0,
        modified: (0, 0),
        changed: (0, 0),
    }
}

pub(super) fn fingerprints(root: &Path) -> Result<Vec<FileFingerprint>, ProviderError> {
    let deadline = Instant::now() + CAPTURE_TIME;
    let mut result = Vec::new();
    for name in SOURCE_NAMES {
        check_deadline(deadline)?;
        let path = root.join(name);
        let before = match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && name != DB_NAME => {
                result.push(FileFingerprint {
                    name: name.into(),
                    record: None,
                    stable_read: true,
                });
                continue;
            }
            _ => {
                return Err(unavailable(
                    "source fingerprint requires regular unlinked files",
                ));
            }
        };
        if before.len() > MAX_FILE_BYTES as u64 {
            return Err(unavailable("source fingerprint byte budget exceeded"));
        }
        let bytes = filesystem::read_bounded(root, &path, MAX_FILE_BYTES)
            .map_err(|_| unavailable("source fingerprint could not be read safely"))?;
        let hash = sha256(&bytes);
        let after = fs::symlink_metadata(&path)
            .map_err(|_| unavailable("source changed during fingerprint observation"))?;
        if !after.is_file() || after.file_type().is_symlink() {
            return Err(unavailable("source changed during fingerprint observation"));
        }
        let record = metadata_record(&after, hash.clone());
        let stable_read = metadata_record(&before, hash) == record;
        result.push(FileFingerprint {
            name: name.into(),
            record: Some(record),
            stable_read,
        });
        check_deadline(deadline)?;
    }
    Ok(result)
}

pub(super) fn fingerprints_changed(before: &[FileFingerprint], after: &[FileFingerprint]) -> bool {
    before != after || before.iter().chain(after).any(|file| !file.stable_read)
}

pub(super) fn configure(connection: &Connection) -> Result<(), ProviderError> {
    for (limit, value) in [
        (Limit::SQLITE_LIMIT_LENGTH, 1024 * 1024),
        (Limit::SQLITE_LIMIT_SQL_LENGTH, 64 * 1024),
        (Limit::SQLITE_LIMIT_COLUMN, 128),
        // VACUUM attaches one internal temporary database. No user SQL is accepted.
        (Limit::SQLITE_LIMIT_ATTACHED, 1),
        (Limit::SQLITE_LIMIT_WORKER_THREADS, 0),
    ] {
        connection.set_limit(limit, value).map_err(sqlite_error)?;
    }
    connection
        .busy_timeout(Duration::from_millis(50))
        .map_err(sqlite_error)?;
    connection
        .execute_batch("PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY;")
        .map_err(sqlite_error)
}

pub(super) fn bounded_backup(
    from: &Connection,
    to: &mut Connection,
    deadline: Instant,
) -> Result<(), ProviderError> {
    let page_size = u64::try_from(
        from.query_row("PRAGMA page_size", [], |row| row.get::<_, i64>(0))
            .map_err(sqlite_error)?,
    )
    .map_err(|_| unavailable("invalid SQLite page size"))?;
    let pages = u64::try_from(
        from.query_row("PRAGMA page_count", [], |row| row.get::<_, i64>(0))
            .map_err(sqlite_error)?,
    )
    .map_err(|_| unavailable("invalid SQLite page count"))?;
    if pages.saturating_mul(page_size) > MAX_DATABASE_BYTES {
        return Err(unavailable("SQLite snapshot memory budget exceeded"));
    }
    let backup = Backup::new(from, to).map_err(sqlite_error)?;
    loop {
        check_deadline(deadline)?;
        let step = backup.step(128).map_err(sqlite_error)?;
        if u64::try_from(backup.progress().pagecount)
            .unwrap_or(u64::MAX)
            .saturating_mul(page_size)
            > MAX_DATABASE_BYTES
        {
            return Err(unavailable("SQLite snapshot memory budget exceeded"));
        }
        match step {
            StepResult::Done => return Ok(()),
            StepResult::Busy | StepResult::Locked => std::thread::sleep(Duration::from_millis(5)),
            StepResult::More => {}
            _ => return Err(unavailable("SQLite backup returned an unsupported state")),
        }
    }
}

pub(super) fn validate_worker_paths(
    source_root: &Path,
    private_root: &Path,
    probe_path: &Path,
) -> Result<(), ProviderError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let canonical =
        |path: &Path| path.is_absolute() && path.canonicalize().is_ok_and(|p| p == path);
    let private_meta = fs::symlink_metadata(private_root)
        .map_err(|_| unavailable("private evidence home unavailable"))?;
    let source_meta =
        fs::symlink_metadata(source_root).map_err(|_| unavailable("source root unavailable"))?;
    let parent = probe_path
        .parent()
        .ok_or_else(|| unavailable("probe parent missing"))?;
    let probe_meta =
        fs::symlink_metadata(parent).map_err(|_| unavailable("denial probe unavailable"))?;
    let prefix = |path: &Path, expected: &str| {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(expected))
    };
    if !canonical(source_root)
        || !canonical(private_root)
        || !canonical(parent)
        || !private_meta.is_dir()
        || private_meta.file_type().is_symlink()
        || !source_meta.is_dir()
        || source_meta.file_type().is_symlink()
        || !probe_meta.is_dir()
        || probe_meta.file_type().is_symlink()
        || private_meta.permissions().mode() & 0o777 != 0o700
        || probe_meta.permissions().mode() & 0o777 != 0o700
        || private_meta.uid() != probe_meta.uid()
        || private_root.parent() != Some(Path::new("/private/tmp"))
        || parent.parent() != Some(Path::new("/private/tmp"))
        || !prefix(private_root, "agentmender-evidence-")
        || !prefix(parent, "agentmender-denial-")
        || probe_path.file_name() != Some(std::ffi::OsStr::new("must-remain-absent"))
        || probe_path.exists()
        || private_root.starts_with(source_root)
        || source_root.starts_with(private_root)
        || parent == private_root
        || std::env::var_os("HOME").as_deref() != Some(private_root.as_os_str())
        || std::env::var_os("CODEX_HOME").as_deref() != Some(private_root.as_os_str())
        || std::env::var_os("TMPDIR").as_deref() != Some(private_root.as_os_str())
        || fs::read_dir(private_root)
            .map_err(|_| unavailable("private directory cannot be inspected"))?
            .next()
            .is_some()
    {
        return Err(unavailable("internal capture capability paths rejected"));
    }
    agentmender_provider_api::process::verify_source_write_denial(source_root)?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(probe_path)
    {
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Ok(()),
        _ => Err(unavailable("OS evidence write-denial canary failed")),
    }
}

fn open_provider_db(root: &Path) -> Result<Connection, ProviderError> {
    let source = Connection::open_with_flags(
        root.join("state_5.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(sqlite_error)?;
    configure(&source)?;
    source
        .execute_batch("PRAGMA query_only=ON")
        .map_err(sqlite_error)?;
    Ok(source)
}

#[cfg(target_os = "macos")]
pub(super) fn read_snapshot(root: &Path, end: Instant) -> Result<Connection, ProviderError> {
    let source = open_provider_db(root)?;
    let mut memory = Connection::open_in_memory().map_err(sqlite_error)?;
    configure(&memory)?;
    bounded_backup(
        &source,
        &mut memory,
        end.min(Instant::now() + Duration::from_secs(3)),
    )?;
    Ok(memory)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_sqlite_omits_retired_full_text_search() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let enabled: bool = db
            .query_row(
                "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!enabled);
    }
}
