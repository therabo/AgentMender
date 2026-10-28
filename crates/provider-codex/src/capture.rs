//! Fixed, disposable evidence capture. Production source SQLite access occurs only
//! in the OS-sandboxed internal worker; tests use databases they create themselves.
use agentmender_provider_api::{ProviderError, filesystem};
use rusqlite::{Connection, OpenFlags, backup::Backup, backup::StepResult, limits::Limit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(target_os = "macos")]
use std::io::{Read, Write};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(super) const DB_NAME: &str = "state_5.sqlite";
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum ExactProfile {
    #[serde(rename = "0.164.0-alpha.11.1")]
    V164,
    #[serde(rename = "0.165.0-alpha.1.2")]
    V165,
}
impl ExactProfile {
    pub fn lookup(version: &str) -> Option<Self> {
        match version {
            "0.164.0-alpha.11.1" => Some(Self::V164),
            "0.165.0-alpha.1.2" => Some(Self::V165),
            _ => None,
        }
    }
    pub fn interface_id(self) -> &'static str {
        match self {
            Self::V164 => "codex-app-server-0.164.0-alpha.11.1",
            Self::V165 => "codex-app-server-0.165.0-alpha.1.2",
        }
    }
    fn schema_hash(self) -> &'static str {
        match self {
            Self::V164 => "5a7a23f469f17c68c9713f62a02a0ec97868e232a08cdff38881d922ea58ab4b",
            Self::V165 => "1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647",
        }
    }
}
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_DATABASE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ROWS: u64 = 100_000;
const MAX_REQUEST_BYTES: u64 = 8192;
pub(super) const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const CAPTURE_TIME: Duration = Duration::from_secs(3);
const SOURCE_NAMES: [&str; 3] = [DB_NAME, "state_5.sqlite-wal", "state_5.sqlite-shm"];

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CaptureRequest {
    pub profile: ExactProfile,
    pub source_root: PathBuf,
    pub mirror_root: PathBuf,
    pub probe_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FileFingerprint {
    name: String,
    record: Option<FileRecord>,
    stable_read: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CaptureManifest {
    pub schema_hash: String,
    pub active_rows: u64,
    pub archived_rows: u64,
    pub nonempty_active_previews: u64,
    pub nonempty_archived_previews: u64,
    pub before: Vec<FileFingerprint>,
    pub after: Vec<FileFingerprint>,
}
impl CaptureManifest {
    pub fn validate(&self, profile: ExactProfile) -> Result<(), ProviderError> {
        let valid_fingerprints = |values: &[FileFingerprint]| {
            values.len() == 3
                && values.iter().zip(SOURCE_NAMES).all(|(value, name)| {
                    value.name == name
                        && value.record.as_ref().is_none_or(|record| {
                            record.bytes <= MAX_FILE_BYTES as u64
                                && record.hash.len() == 64
                                && record.hash.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                })
                && values[0].record.is_some()
        };
        if self.schema_hash != profile.schema_hash()
            || self.active_rows.saturating_add(self.archived_rows) > MAX_ROWS
            || self.nonempty_active_previews > self.active_rows
            || self.nonempty_archived_previews > self.archived_rows
            || !valid_fingerprints(&self.before)
            || !valid_fingerprints(&self.after)
        {
            return Err(unavailable("capture manifest failed its fixed profile"));
        }
        Ok(())
    }
    pub fn changed(&self) -> bool {
        fingerprints_changed(&self.before, &self.after)
    }
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

fn configure(connection: &Connection) -> Result<(), ProviderError> {
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

fn bounded_backup(
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

type SchemaEntry = (String, String, String, Option<String>);
fn verify_schema(connection: &Connection, profile: ExactProfile) -> Result<(), ProviderError> {
    let mut statement = connection
        .prepare("SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name LIMIT 129")
        .map_err(sqlite_error)?;
    let entries = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(sqlite_error)?
        .collect::<Result<Vec<SchemaEntry>, _>>()
        .map_err(sqlite_error)?;
    let encoded =
        serde_json::to_vec(&entries).map_err(|_| unavailable("schema encoding rejected"))?;
    if encoded.len() > 64 * 1024 || sha256(&encoded) != profile.schema_hash() {
        return Err(unavailable("exact Codex state schema is not recognized"));
    }
    Ok(())
}

fn capture_database(
    source_root: &Path,
    mirror: &Path,
    profile: ExactProfile,
) -> Result<CaptureManifest, ProviderError> {
    let deadline = Instant::now() + CAPTURE_TIME;
    let before = fingerprints(source_root)?;
    let source = Connection::open_with_flags(
        source_root.join(DB_NAME),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(sqlite_error)?;
    configure(&source)?;
    source
        .execute_batch("PRAGMA query_only=ON")
        .map_err(sqlite_error)?;
    let mut memory = Connection::open_in_memory().map_err(sqlite_error)?;
    configure(&memory)?;
    bounded_backup(&source, &mut memory, deadline)?;
    drop(source);
    verify_schema(&memory, profile)?;
    check_deadline(deadline)?;
    let counts: (i64, i64, i64, i64) =
        memory.query_row("SELECT count(*) FILTER (WHERE archived=0),count(*) FILTER (WHERE archived=1),count(*) FILTER (WHERE archived=0 AND preview<>''),count(*) FILTER (WHERE archived=1 AND preview<>'') FROM threads", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).map_err(sqlite_error)?;
    let count = |value| u64::try_from(value).map_err(|_| unavailable("invalid SQLite row count"));
    let (active_rows, archived_rows, nonempty_active_previews, nonempty_archived_previews) = (
        count(counts.0)?,
        count(counts.1)?,
        count(counts.2)?,
        count(counts.3)?,
    );
    if active_rows.saturating_add(archived_rows) > MAX_ROWS {
        return Err(unavailable("SQLite row budget exceeded"));
    }
    memory.execute_batch("PRAGMA secure_delete=ON;
        DELETE FROM external_agent_config_imports; DELETE FROM guardian_review_feedback;
        DELETE FROM project_idempotency_keys; DELETE FROM remote_control_enrollments;
        DELETE FROM rollout_migration_skipped_rollouts; DELETE FROM thread_attachments;
        DELETE FROM thread_dynamic_tools; DELETE FROM thread_read_receipts;
        UPDATE threads SET
          title=CASE WHEN title IS NULL THEN NULL WHEN title='' THEN '' ELSE '[redacted]' END,
          first_user_message=CASE WHEN first_user_message IS NULL THEN NULL WHEN first_user_message='' THEN '' ELSE '[redacted]' END,
          preview=CASE WHEN preview IS NULL THEN NULL WHEN preview='' THEN '' ELSE '[redacted]' END,
          name=CASE WHEN name IS NULL THEN NULL WHEN name='' THEN '' ELSE '[redacted]' END,
          git_sha=NULL,git_branch=NULL,git_origin_url=NULL,agent_nickname=NULL,agent_role=NULL,
          creator_user_id=NULL,creator_account_id=NULL;
        UPDATE projects SET name='[redacted]',metadata='{}';
        UPDATE thread_sections SET name='[redacted]';
        VACUUM;").map_err(sqlite_error)?;
    check_deadline(deadline)?;
    let check: String = memory
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    if check != "ok" {
        return Err(unavailable("SQLite snapshot integrity rejected"));
    }
    // The first persistent bytes are already scrubbed and vacuumed. create_new
    // prevents replacing an existing file in the capability-owned fresh mirror.
    let target_path = mirror.join(DB_NAME);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let target = options
        .open(&target_path)
        .map_err(|_| unavailable("fresh mirror destination unavailable"))?;
    drop(target);
    let mut destination = Connection::open_with_flags(
        &target_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(sqlite_error)?;
    configure(&destination)?;
    bounded_backup(&memory, &mut destination, deadline)?;
    drop(destination);
    drop(memory);
    let after = fingerprints(source_root)?;
    let manifest = CaptureManifest {
        schema_hash: profile.schema_hash().into(),
        active_rows,
        archived_rows,
        nonempty_active_previews,
        nonempty_archived_previews,
        before,
        after,
    };
    manifest.validate(profile)?;
    Ok(manifest)
}

#[cfg(target_os = "macos")]
fn validate_worker_request(request: &CaptureRequest) -> Result<(), ProviderError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let canonical =
        |path: &Path| path.is_absolute() && path.canonicalize().is_ok_and(|p| p == path);
    let mirror_meta = fs::symlink_metadata(&request.mirror_root)
        .map_err(|_| unavailable("private evidence home unavailable"))?;
    let source_meta = fs::symlink_metadata(&request.source_root)
        .map_err(|_| unavailable("source root unavailable"))?;
    let parent = request
        .probe_path
        .parent()
        .ok_or_else(|| unavailable("probe parent missing"))?;
    let probe_meta =
        fs::symlink_metadata(parent).map_err(|_| unavailable("denial probe unavailable"))?;
    let prefix = |path: &Path, expected: &str| {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(expected))
    };
    if !canonical(&request.source_root)
        || !canonical(&request.mirror_root)
        || !canonical(parent)
        || !mirror_meta.is_dir()
        || mirror_meta.file_type().is_symlink()
        || !source_meta.is_dir()
        || source_meta.file_type().is_symlink()
        || !probe_meta.is_dir()
        || probe_meta.file_type().is_symlink()
        || mirror_meta.permissions().mode() & 0o777 != 0o700
        || probe_meta.permissions().mode() & 0o777 != 0o700
        || mirror_meta.uid() != probe_meta.uid()
        || request.mirror_root.parent() != Some(Path::new("/private/tmp"))
        || parent.parent() != Some(Path::new("/private/tmp"))
        || !prefix(&request.mirror_root, "agentmender-evidence-")
        || !prefix(parent, "agentmender-denial-")
        || request.probe_path.file_name() != Some(std::ffi::OsStr::new("must-remain-absent"))
        || request.probe_path.exists()
        || request.mirror_root.starts_with(&request.source_root)
        || request.source_root.starts_with(&request.mirror_root)
        || parent == request.mirror_root
        || std::env::var_os("HOME").as_deref() != Some(request.mirror_root.as_os_str())
        || std::env::var_os("CODEX_HOME").as_deref() != Some(request.mirror_root.as_os_str())
        || std::env::var_os("TMPDIR").as_deref() != Some(request.mirror_root.as_os_str())
        || fs::read_dir(&request.mirror_root)
            .map_err(|_| unavailable("mirror cannot be inspected"))?
            .next()
            .is_some()
    {
        return Err(unavailable("internal capture capability paths rejected"));
    }
    agentmender_provider_api::process::verify_source_write_denial(&request.source_root)?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&request.probe_path)
    {
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Ok(()),
        _ => Err(unavailable("OS evidence write-denial canary failed")),
    }
}

/// Fixed internal CLI dispatch. Never call this as an unsandboxed source reader.
pub fn run_evidence_capture_worker() -> Result<(), ProviderError> {
    #[cfg(target_os = "macos")]
    {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_REQUEST_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| unavailable("capture request unavailable"))?;
        if bytes.len() as u64 > MAX_REQUEST_BYTES {
            return Err(unavailable("capture request budget exceeded"));
        }
        let request: CaptureRequest = serde_json::from_slice(&bytes)
            .map_err(|_| unavailable("fixed capture request rejected"))?;
        validate_worker_request(&request)?;
        let manifest =
            capture_database(&request.source_root, &request.mirror_root, request.profile)?;
        let bytes = serde_json::to_vec(&manifest)
            .map_err(|_| unavailable("capture manifest encoding rejected"))?;
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(unavailable("capture manifest budget exceeded"));
        }
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|_| unavailable("capture manifest output failed"))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(unavailable(
            "disposable evidence capture isolation is unsupported on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(root: &Path, profile: ExactProfile) -> Connection {
        let connection = Connection::open(root.join(DB_NAME)).unwrap();
        // The captured schema retains sqlite_sequence from a historical migration.
        connection.execute_batch("CREATE TABLE fixture_sequence_seed(value INTEGER PRIMARY KEY AUTOINCREMENT); DROP TABLE fixture_sequence_seed;").unwrap();
        let schema = match profile {
            ExactProfile::V164 => include_str!("../../../compatibility/codex/state-schema-5.json"),
            ExactProfile::V165 => {
                include_str!("../../../compatibility/codex/0.165.0-alpha.1.2/state-schema-5.json")
            }
        };
        let entries: Vec<SchemaEntry> = serde_json::from_str(schema).unwrap();
        assert_eq!(
            sha256(&serde_json::to_vec(&entries).unwrap()),
            profile.schema_hash()
        );
        for kind in ["table", "index", "trigger", "view"] {
            for (entry_kind, name, _, sql) in &entries {
                if entry_kind == kind
                    && name != "sqlite_sequence"
                    && let Some(sql) = sql
                {
                    connection.execute_batch(sql).unwrap();
                }
            }
        }
        verify_schema(&connection, profile).unwrap();
        connection
    }
    fn insert(connection: &Connection, id: &str, preview: &str) {
        connection.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,first_user_message,preview,name) VALUES (?1,'/fixture/rollout',1700000000,1700000001,'cli','openai','/fixture/project',?2,'read-only','never',?2,?2,?2)",rusqlite::params![id,preview]).unwrap();
    }
    #[test]
    fn exact_profiles_bind_schema_and_reject_unknown_worker_tokens() {
        for (profile, other) in [
            (ExactProfile::V164, ExactProfile::V165),
            (ExactProfile::V165, ExactProfile::V164),
        ] {
            let source = tempfile::tempdir().unwrap();
            let mirror = tempfile::tempdir().unwrap();
            let connection = database(source.path(), profile);
            assert!(verify_schema(&connection, other).is_err());
            drop(connection);
            let source_path = source.path().canonicalize().unwrap();
            let mirror_path = mirror.path().canonicalize().unwrap();
            assert!(capture_database(&source_path, &mirror_path, other).is_err());
            assert!(!mirror.path().join(DB_NAME).exists());
            let manifest = capture_database(&source_path, &mirror_path, profile).unwrap();
            assert!(manifest.validate(profile).is_ok());
            assert!(manifest.validate(other).is_err());
        }
        assert!(ExactProfile::lookup("0.165.0-alpha.1.3").is_none());
        assert!(serde_json::from_str::<CaptureRequest>(r#"{"profile":"unknown","sourceRoot":"/none","mirrorRoot":"/none","probePath":"/none"}"#).is_err());
    }
    #[test]
    fn coherent_wal_snapshot_is_scrubbed_before_persistence() {
        for profile in [ExactProfile::V164, ExactProfile::V165] {
            let source = tempfile::tempdir().unwrap();
            let mirror = tempfile::tempdir().unwrap();
            let writer = database(source.path(), profile);
            writer
                .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
                .unwrap();
            insert(&writer, "fixture-one", "PRIVATE_BODY_MUST_NOT_PERSIST");
            insert(&writer, "fixture-empty", "");
            insert(
                &writer,
                "fixture-nul",
                "\0PRIVATE_NUL_BODY_MUST_NOT_PERSIST",
            );
            let manifest = capture_database(
                &source.path().canonicalize().unwrap(),
                &mirror.path().canonicalize().unwrap(),
                profile,
            )
            .unwrap();
            assert_eq!(manifest.active_rows, 3);
            assert_eq!(manifest.nonempty_active_previews, 2);
            // This unit test owns its source and runs without the production sandbox.
            // SQLite may update read marks in SHM; source DB/WAL must still be unchanged.
            assert_eq!(&manifest.before[..2], &manifest.after[..2]);
            let bytes = fs::read(mirror.path().join(DB_NAME)).unwrap();
            assert!(
                !bytes
                    .windows(b"PRIVATE_BODY_MUST_NOT_PERSIST".len())
                    .any(|part| part == b"PRIVATE_BODY_MUST_NOT_PERSIST")
            );
            assert!(
                !bytes
                    .windows(b"PRIVATE_NUL_BODY_MUST_NOT_PERSIST".len())
                    .any(|part| part == b"PRIVATE_NUL_BODY_MUST_NOT_PERSIST")
            );
            let captured = Connection::open_with_flags(
                mirror.path().join(DB_NAME),
                OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(
                captured
                    .query_row(
                        "SELECT preview FROM threads WHERE id='fixture-one'",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                "[redacted]"
            );
            assert_eq!(
                captured
                    .query_row(
                        "SELECT preview FROM threads WHERE id='fixture-empty'",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                ""
            );
            assert_eq!(
                writer
                    .query_row(
                        "SELECT preview FROM threads WHERE id='fixture-one'",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                "PRIVATE_BODY_MUST_NOT_PERSIST"
            );
        }
    }
    #[test]
    fn unknown_and_corrupt_sqlite_never_produce_a_snapshot() {
        for corrupt in [false, true] {
            let source = tempfile::tempdir().unwrap();
            let mirror = tempfile::tempdir().unwrap();
            if corrupt {
                fs::write(source.path().join(DB_NAME), b"not sqlite PRIVATE_BODY").unwrap();
            } else {
                Connection::open(source.path().join(DB_NAME))
                    .unwrap()
                    .execute_batch("CREATE TABLE unknown_state(value TEXT);")
                    .unwrap();
            }
            assert!(
                capture_database(
                    &source.path().canonicalize().unwrap(),
                    &mirror.path().canonicalize().unwrap(),
                    ExactProfile::V164
                )
                .is_err()
            );
            assert!(!mirror.path().join(DB_NAME).exists());
        }
    }
    #[test]
    fn source_changes_are_visible_in_fingerprints() {
        let root = tempfile::tempdir().unwrap();
        let writer = database(root.path(), ExactProfile::V164);
        let before = fingerprints(root.path()).unwrap();
        insert(&writer, "changed", "fixture");
        let after = fingerprints(root.path()).unwrap();
        assert!(fingerprints_changed(&before, &after));
    }
}
