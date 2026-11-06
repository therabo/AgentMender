//! Fixed, bounded protected observation of the Desktop catalog.
use crate::desktop_catalog::{self, CatalogObservation, CatalogTarget};
use agentmender_domain::CleanupFileIdentity;
use agentmender_persistence::EffectError;
use agentmender_provider_api::{ProviderError, evidence::EvidenceSandbox, process};
use rusqlite::{Connection, OpenFlags};
use rustix::fs::{self, Mode, OFlags};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    os::{fd::OwnedFd, unix::fs::MetadataExt},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

fn bad() -> EffectError {
    EffectError::InvalidEvidence
}
fn unavailable() -> ProviderError {
    ProviderError::CapabilityUnavailable("protected Desktop catalog observation unavailable".into())
}
fn dir(parent: &impl std::os::fd::AsFd, name: &std::ffi::OsStr) -> Result<OwnedFd, EffectError> {
    fs::openat(
        parent,
        name,
        crate::capture::directory_search_flags(),
        Mode::empty(),
    )
    .map_err(|_| bad())
}
fn root(path: &Path) -> Result<OwnedFd, EffectError> {
    if !path.is_absolute() {
        return Err(bad());
    }
    let mut fd = fs::open("/", crate::capture::directory_search_flags(), Mode::empty())
        .map_err(|_| bad())?;
    for part in path.components() {
        match part {
            Component::RootDir => (),
            Component::Normal(n) => fd = dir(&fd, n)?,
            _ => return Err(bad()),
        }
    }
    Ok(fd)
}
// Darwin and Linux use different widths for device/inode metadata.
#[allow(clippy::unnecessary_cast)]
fn meta(
    fd: &OwnedFd,
    name: &str,
    path: PathBuf,
) -> Result<Option<CleanupFileIdentity>, EffectError> {
    // statat observes the directory entry without opening and closing a second
    // descriptor for the SQLite file: closing one would release process fcntl locks.
    let stat = match fs::statat(fd, name, fs::AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => stat,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(_) => return Err(bad()),
    };
    if fs::FileType::from_raw_mode(stat.st_mode) != fs::FileType::RegularFile
        || stat.st_nlink as u64 != 1
    {
        return Err(bad());
    }
    let m = std::fs::symlink_metadata(&path).map_err(|_| bad())?;
    if !m.is_file()
        || m.nlink() != 1
        || m.dev() != stat.st_dev as u64
        || m.ino() != stat.st_ino as u64
    {
        return Err(bad());
    }
    Ok(Some(CleanupFileIdentity {
        path,
        device: m.dev(),
        inode: m.ino(),
        links: m.nlink(),
        size_bytes: m.len(),
        mtime_seconds: m.mtime(),
        mtime_nanoseconds: m.mtime_nsec() as u32,
    }))
}
fn layout(
    desktop_root: &CleanupFileIdentity,
) -> Result<Vec<Option<CleanupFileIdentity>>, EffectError> {
    let r = root(&desktop_root.path)?;
    let m = File::from(dir(&r, ".".as_ref())?)
        .metadata()
        .map_err(|_| bad())?;
    if m.dev() != desktop_root.device || m.ino() != desktop_root.inode || !m.is_dir() {
        return Err(bad());
    }
    let d = dir(&r, "sqlite".as_ref())?;
    let dm = File::from(dir(&d, ".".as_ref())?)
        .metadata()
        .map_err(|_| bad())?;
    if dm.dev() != desktop_root.device {
        return Err(bad());
    }
    let base = desktop_root.path.join("sqlite");
    let entries = [
        "codex.db",
        "codex.db-wal",
        "codex.db-shm",
        "codex.db-journal",
    ]
    .into_iter()
    .map(|n| meta(&d, n, base.join(n)))
    .collect::<Result<Vec<_>, _>>()?;
    if entries[0].is_none()
        || entries[3].is_some()
        || entries
            .iter()
            .flatten()
            .any(|x| x.device != desktop_root.device)
    {
        return Err(bad());
    }
    Ok(entries)
}
// A clean closed WAL database has no sidecars, and SQLite normally attempts
// to create them even for read-only opens. Immutable mode is safe here only
// with both sidecars absent and the unchanged physical layout checked after
// backup; a WAL-format header is required because immutable mode reports
// journal_mode=delete regardless of the underlying header.
#[cfg(any(target_os = "macos", test))]
fn open_source(
    root_id: &CleanupFileIdentity,
    before: &[Option<CleanupFileIdentity>],
) -> Result<(Connection, bool), EffectError> {
    let path = root_id.path.join("sqlite/codex.db");
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    if before[1].is_some() || before[2].is_some() {
        return Connection::open_with_flags(path, flags)
            .map(|db| (db, false))
            .map_err(|_| bad());
    }
    use std::{io::Read, os::unix::ffi::OsStrExt};
    let root_fd = root(&root_id.path)?;
    let sqlite = dir(&root_fd, "sqlite".as_ref())?;
    let fd = fs::openat(
        &sqlite,
        "codex.db",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| bad())?;
    let mut file = File::from(fd);
    let m = file.metadata().map_err(|_| bad())?;
    let identity = before[0].as_ref().ok_or_else(bad)?;
    if !m.is_file()
        || m.dev() != identity.device
        || m.ino() != identity.inode
        || m.nlink() != 1
        || m.len() != identity.size_bytes
        || m.mtime() != identity.mtime_seconds
        || m.mtime_nsec() != i64::from(identity.mtime_nanoseconds)
    {
        return Err(bad());
    }
    let mut header = [0u8; 20];
    file.read_exact(&mut header).map_err(|_| bad())?;
    if &header[..16] != b"SQLite format 3\0" || header[18] != 2 || header[19] != 2 {
        return Err(bad());
    }
    drop(file);
    let mut uri = String::from("file:");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(&mut uri, "%{byte:02X}").map_err(|_| bad())?;
        }
    }
    uri.push_str("?immutable=1");
    Connection::open_with_flags(uri, flags | OpenFlags::SQLITE_OPEN_URI)
        .map(|db| (db, true))
        .map_err(|_| bad())
}
#[cfg(target_os = "macos")]
fn snapshot(
    desktop_root: &CleanupFileIdentity,
    target: &CatalogTarget,
) -> Result<CatalogObservation, EffectError> {
    let before = layout(desktop_root)?;
    let (source, immutable) = open_source(desktop_root, &before)?;
    crate::capture::configure(&source).map_err(|_| bad())?;
    source
        .execute_batch("PRAGMA query_only=ON")
        .map_err(|_| bad())?;
    if !immutable {
        let mode: String = source
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .map_err(|_| bad())?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(bad());
        }
    }
    let mut memory = Connection::open_in_memory().map_err(|_| bad())?;
    crate::capture::configure(&memory).map_err(|_| bad())?;
    crate::capture::bounded_backup(
        &source,
        &mut memory,
        Instant::now() + Duration::from_secs(3),
    )
    .map_err(|_| bad())?;
    drop(source);
    if layout(desktop_root)? != before {
        return Err(bad());
    }
    desktop_catalog::observe(&memory, before[0].clone().ok_or_else(bad)?, target)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    source_root: PathBuf,
    mirror_root: PathBuf,
    probe_path: PathBuf,
    desktop_root: CleanupFileIdentity,
    thread_id: String,
    source_kind: String,
}

/// Fixed CLI worker: it refuses source access unless kernel denial is attested.
pub fn run_catalog_evidence_worker() -> Result<(), ProviderError> {
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
        let request: Request = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        if request.source_root != request.desktop_root.path {
            return Err(unavailable());
        }
        crate::capture::validate_worker_request(&crate::capture::CaptureRequest {
            profile: crate::capture::ExactProfile::V165,
            source_root: request.source_root,
            mirror_root: request.mirror_root,
            probe_path: request.probe_path,
        })?;
        process::verify_catalog_write_denial(&request.desktop_root.path)?;
        let result = snapshot(
            &request.desktop_root,
            &CatalogTarget {
                thread_id: request.thread_id,
                source_kind: request.source_kind,
            },
        )
        .map_err(|_| unavailable())?;
        let bytes = serde_json::to_vec(&result).map_err(|_| unavailable())?;
        if bytes.len() > 32768 {
            return Err(unavailable());
        }
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|_| unavailable())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(unavailable())
    }
}

/// Read-only planning observation; the helper is the fixed internal executable.
pub(super) fn observe_catalog(
    adapter: &crate::CodexAdapter,
    desktop_root: &CleanupFileIdentity,
    thread_id: &str,
    source_kind: &str,
) -> Result<CatalogObservation, ProviderError> {
    use std::{
        io::{Read, Write},
        sync::mpsc,
    };
    let (helper, home) = adapter.evidence_helper.as_ref().ok_or_else(unavailable)?;
    if thread_id.len() != 36
        || !thread_id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
        || !matches!(source_kind, "cli" | "appServer")
    {
        return Err(unavailable());
    }
    let sandbox = EvidenceSandbox::new(&desktop_root.path, home)?;
    if sandbox.source_root() != desktop_root.path {
        return Err(unavailable());
    }
    let request = Request {
        source_root: sandbox.source_root().into(),
        mirror_root: sandbox.path().into(),
        probe_path: sandbox.probe_path().into(),
        desktop_root: desktop_root.clone(),
        thread_id: thread_id.into(),
        source_kind: source_kind.into(),
    };
    let bytes = serde_json::to_vec(&request).map_err(|_| unavailable())?;
    if bytes.len() > 8192 {
        return Err(unavailable());
    }
    let mut child = crate::Guard(process::spawn_codex_catalog_capture(helper, &sandbox)?);
    let mut input = child.0.stdin.take().ok_or_else(unavailable)?;
    let mut output = child.0.stdout.take().ok_or_else(unavailable)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = input.write_all(&bytes);
        drop(input);
        let _ = tx.send(result);
    });
    let (tx2, rx2) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .by_ref()
            .take(32769)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = tx2.send(result);
    });
    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    let bytes = rx2
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    if bytes.len() > 32768 {
        return Err(unavailable());
    }
    loop {
        match child.0.try_wait() {
            Ok(Some(s)) if s.success() => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => return Err(unavailable()),
        }
    }
    let result: CatalogObservation = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    if result.commitment.identity.path != desktop_root.path.join("sqlite/codex.db")
        || result.commitment.identity.device != desktop_root.device
        || result.title.len() > 16384
    {
        return Err(unavailable());
    }
    Ok(result)
}

/// Retains the same SQLite connection and checks its bound file identity at verification.
pub(super) struct CatalogBoundary {
    fence: desktop_catalog::CatalogFence,
    root: CleanupFileIdentity,
    database: CleanupFileIdentity,
    sidecars: Vec<Option<CleanupFileIdentity>>,
}
impl CatalogBoundary {
    fn bound(&self) -> Result<(), EffectError> {
        let files = layout(&self.root)?;
        let current = files[0].as_ref().ok_or_else(bad)?;
        if current.device != self.database.device
            || current.inode != self.database.inode
            || current.links != 1
        {
            return Err(bad());
        }
        for (current, frozen) in files[1..3].iter().zip(&self.sidecars) {
            if !same_object(current.as_ref(), frozen.as_ref()) {
                return Err(bad());
            }
        }
        Ok(())
    }
    pub(super) fn apply(
        &mut self,
        target: &CatalogTarget,
        frozen: &agentmender_domain::DesktopCatalogCommitment,
    ) -> Result<desktop_catalog::CatalogPostimage, EffectError> {
        self.bound()?;
        let postimage = self.fence.apply(target, frozen)?;
        self.bound()?;
        Ok(postimage)
    }
    pub(super) fn verify(
        &self,
        postimage: &desktop_catalog::CatalogPostimage,
    ) -> Result<(), EffectError> {
        self.bound()?;
        self.fence.verify(postimage)?;
        self.bound()
    }
}

// Compare object identity only for sidecars: acquiring a SQLite lock may update
// their contents and timestamps. The logical preimage is independently checked
// under the exclusive lock before any target mutation.
fn same_object(left: Option<&CleanupFileIdentity>, right: Option<&CleanupFileIdentity>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.path == b.path
                && a.device == b.device
                && a.inode == b.inode
                && a.links == 1
                && b.links == 1
        }
        _ => false,
    }
}
fn acquisition_layout_valid(
    before: &[Option<CleanupFileIdentity>],
    after: &[Option<CleanupFileIdentity>],
) -> bool {
    before.len() == 4
        && after.len() == 4
        && before[0].is_some()
        && before[0] == after[0]
        && before[3].is_none()
        && after[3].is_none()
        && (1..3).all(|i| before[i].is_none() || same_object(before[i].as_ref(), after[i].as_ref()))
}
fn acquire_layout(
    root: &CleanupFileIdentity,
    database: &CleanupFileIdentity,
) -> Result<CatalogBoundary, EffectError> {
    let before = layout(root)?;
    if before[0].as_ref() != Some(database) {
        return Err(bad());
    }
    let db = Connection::open_with_flags(
        root.path.join("sqlite/codex.db"),
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| bad())?;
    let fence = desktop_catalog::CatalogFence::acquire(db)?;
    let after = layout(root)?;
    if !acquisition_layout_valid(&before, &after) {
        return Err(bad());
    }
    Ok(CatalogBoundary {
        fence,
        root: root.clone(),
        database: database.clone(),
        sidecars: after[1..3].to_vec(),
    })
}
/// Fixed execution boundary, available only after durable intent.
pub(super) fn acquire_catalog(
    execution: &crate::desktop_cleanup::DesktopExecution<'_>,
) -> Result<CatalogBoundary, EffectError> {
    let manifest = execution.manifest();
    let desktop = manifest.desktop.as_ref().ok_or_else(bad)?;
    let boundary = acquire_layout(&desktop.desktop_root, &desktop.catalog.identity)?;
    let target = CatalogTarget {
        thread_id: manifest.primary.native_session_id.as_str().to_owned(),
        source_kind: desktop.source_kind.clone(),
    };
    boundary.fence.prevalidate(&target, &desktop.catalog)?;
    Ok(boundary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    fn identity(path: &Path) -> CleanupFileIdentity {
        let m = std::fs::metadata(path).unwrap();
        CleanupFileIdentity {
            path: path.into(),
            device: m.dev(),
            inode: m.ino(),
            links: m.nlink(),
            size_bytes: m.len(),
            mtime_seconds: m.mtime(),
            mtime_nanoseconds: m.mtime_nsec() as u32,
        }
    }
    #[test]
    fn closed_wal_database_acquires_without_sidecars_and_detects_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let sqlite = root.join("sqlite");
        std::fs::create_dir(&sqlite).unwrap();
        let path = sqlite.join("codex.db");
        {
            let db = Connection::open(&path).unwrap();
            db.pragma_update(None, "journal_mode", "WAL").unwrap();
            db.execute_batch("CREATE TABLE fixture(value INTEGER); INSERT INTO fixture VALUES(1);")
                .unwrap();
        }
        let root_id = identity(&root);
        let before = layout(&root_id).unwrap();
        assert!(before[1].is_none() && before[2].is_none());
        let frozen = before[0].clone().unwrap();
        let boundary = acquire_layout(&root_id, &frozen).unwrap();
        boundary.bound().unwrap();
        let after = layout(&root_id).unwrap();
        assert!(
            after[1].is_some(),
            "exclusive WAL acquisition must exercise sidecar creation"
        );
        std::fs::rename(&path, sqlite.join("replaced.db")).unwrap();
        std::fs::write(&path, b"replacement").unwrap();
        assert!(boundary.bound().is_err());
    }
    #[test]
    fn immutable_read_requires_clean_wal_header_and_preserves_layout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root # percent%");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let sqlite = root.join("sqlite");
        std::fs::create_dir(&sqlite).unwrap();
        let path = sqlite.join("codex.db");
        {
            let db = Connection::open(&path).unwrap();
            db.pragma_update(None, "journal_mode", "WAL").unwrap();
            db.execute_batch("CREATE TABLE fixture(value INTEGER); INSERT INTO fixture VALUES(7);")
                .unwrap();
        }
        let id = identity(&root);
        let before = layout(&id).unwrap();
        assert!(before[1].is_none() && before[2].is_none());
        let (db, immutable) = open_source(&id, &before).unwrap();
        assert!(immutable);
        let value: i64 = db
            .query_row("SELECT value FROM fixture", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, 7);
        drop(db);
        assert_eq!(layout(&id).unwrap(), before);
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "journal_mode", "DELETE").unwrap();
        drop(db);
        let before = layout(&id).unwrap();
        assert!(open_source(&id, &before).is_err());
    }
    #[test]
    fn acquisition_rejects_changed_existing_sidecar_and_main() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let sqlite = root.join("sqlite");
        std::fs::create_dir(&sqlite).unwrap();
        std::fs::write(sqlite.join("codex.db"), b"db").unwrap();
        std::fs::write(sqlite.join("codex.db-wal"), b"wal").unwrap();
        let before = layout(&identity(&root)).unwrap();
        let mut after = before.clone();
        assert!(acquisition_layout_valid(&before, &after));
        after[1].as_mut().unwrap().mtime_seconds += 1;
        assert!(acquisition_layout_valid(&before, &after));
        after[1].as_mut().unwrap().inode += 1;
        assert!(!acquisition_layout_valid(&before, &after));
        after = before.clone();
        after[1] = None;
        assert!(!acquisition_layout_valid(&before, &after));
        after = before.clone();
        after[0].as_mut().unwrap().mtime_seconds += 1;
        assert!(!acquisition_layout_valid(&before, &after));
    }
    #[test]
    fn layout_rejects_symlinks_hardlinks_and_journals() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let sqlite = root.join("sqlite");
        std::fs::create_dir(&sqlite).unwrap();
        let db = sqlite.join("codex.db");
        std::fs::write(&db, b"owned fixture").unwrap();
        let id = identity(&root);
        assert!(layout(&id).is_ok());
        std::fs::hard_link(&db, root.join("alias")).unwrap();
        assert!(layout(&id).is_err());
        std::fs::remove_file(root.join("alias")).unwrap();
        std::fs::write(sqlite.join("codex.db-journal"), b"journal").unwrap();
        assert!(layout(&id).is_err());
        std::fs::remove_file(sqlite.join("codex.db-journal")).unwrap();
        std::fs::remove_file(&db).unwrap();
        symlink(root.join("elsewhere"), &db).unwrap();
        assert!(layout(&id).is_err());
        std::fs::remove_file(db).unwrap();
        std::fs::remove_dir(sqlite).unwrap();
        std::fs::create_dir(root.join("other")).unwrap();
        symlink(root.join("other"), root.join("sqlite")).unwrap();
        assert!(layout(&id).is_err());
    }
}
