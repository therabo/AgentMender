//! Filesystem half of the scoped cleanup fence. No paths originate from the CLI.
use agentmender_domain::{CleanupFileIdentity, CleanupManifest};
use agentmender_persistence::EffectError;
use rustix::fs::{self, AtFlags, Dir, FlockOperation, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader, Read},
    os::{
        fd::OwnedFd,
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path, PathBuf},
};

const LIMIT: usize = 100_000;
const HEADER: u64 = 1024 * 1024;
fn bad<T>() -> Result<T, EffectError> {
    Err(EffectError::InvalidEvidence)
}
fn directory(
    parent: &impl std::os::fd::AsFd,
    name: &std::ffi::OsStr,
) -> Result<OwnedFd, EffectError> {
    fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| EffectError::InvalidEvidence)
}
fn root(path: &Path) -> Result<OwnedFd, EffectError> {
    if !path.is_absolute() {
        return bad();
    }
    let mut fd = fs::open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| EffectError::InvalidEvidence)?;
    for c in path.components() {
        match c {
            Component::RootDir => {}
            Component::Normal(n) => fd = directory(&fd, n)?,
            _ => return bad(),
        }
    }
    Ok(fd)
}
fn parent(
    root: &OwnedFd,
    rootpath: &Path,
    path: &Path,
) -> Result<(OwnedFd, std::ffi::OsString), EffectError> {
    let relative = path
        .strip_prefix(rootpath)
        .map_err(|_| EffectError::InvalidEvidence)?;
    let mut parts = relative.components().peekable();
    let mut fd = fs::openat(
        root,
        ".",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| EffectError::InvalidEvidence)?;
    while let Some(c) = parts.next() {
        let Component::Normal(n) = c else {
            return bad();
        };
        if parts.peek().is_none() {
            return Ok((fd, n.to_owned()));
        }
        fd = directory(&fd, n)?;
    }
    bad()
}
fn open_file(parent: &OwnedFd, name: &std::ffi::OsStr) -> Result<File, EffectError> {
    let fd = fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| EffectError::InvalidEvidence)?;
    let f = File::from(fd);
    let m = f.metadata().map_err(|_| EffectError::InvalidEvidence)?;
    if !m.is_file() || m.nlink() != 1 {
        return bad();
    }
    Ok(f)
}
fn matches_identity(f: &File, i: &CleanupFileIdentity) -> bool {
    f.metadata().is_ok_and(|m| {
        m.is_file()
            && m.dev() == i.device
            && m.ino() == i.inode
            && m.nlink() == i.links
            && m.len() == i.size_bytes
            && m.mtime() == i.mtime_seconds
            && m.mtime_nsec() == i.mtime_nanoseconds as i64
    })
}
fn check_root(fd: &OwnedFd, identity: &CleanupFileIdentity) -> Result<(), EffectError> {
    let m = File::from(
        fs::openat(
            fd,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| EffectError::InvalidEvidence)?,
    )
    .metadata()
    .map_err(|_| EffectError::InvalidEvidence)?;
    if m.dev() != identity.device || m.ino() != identity.inode {
        return bad();
    }
    Ok(())
}
fn names(fd: &OwnedFd) -> Result<Vec<std::ffi::OsString>, EffectError> {
    use std::os::unix::ffi::OsStringExt;
    let mut result = Vec::new();
    for item in Dir::read_from(fd).map_err(|_| EffectError::InvalidEvidence)? {
        let item = item.map_err(|_| EffectError::InvalidEvidence)?;
        let n = item.file_name().to_bytes();
        if n == b"." || n == b".." {
            continue;
        }
        if result.len() >= LIMIT {
            return bad();
        }
        result.push(std::ffi::OsString::from_vec(n.to_vec()));
    }
    result.sort();
    Ok(result)
}
fn lock(fd: File) -> Result<File, EffectError> {
    fs::flock(&fd, FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| EffectError::Unavailable)?;
    Ok(fd)
}
/// Acquired in the exact provider order and held through post-verification.
pub(crate) struct WriterFence {
    locks: Vec<(PathBuf, File)>,
}

/// Existing WAL coordination files must be regular, single-linked files on
/// the same device. Handles remain live until after SQLite closes.
pub(crate) struct DatabaseSidecars {
    root: PathBuf,
    files: Vec<(String, File)>,
}
impl DatabaseSidecars {
    pub(crate) fn acquire(manifest: &CleanupManifest) -> Result<Self, EffectError> {
        Self::from_root(&manifest.root)
    }
    fn from_root(identity: &CleanupFileIdentity) -> Result<Self, EffectError> {
        let fd = root(&identity.path)?;
        check_root(&fd, identity)?;
        let mut files = Vec::new();
        for name in ["state_5.sqlite-wal", "state_5.sqlite-shm"] {
            let file = open_file(&fd, name.as_ref())?;
            if file
                .metadata()
                .map_err(|_| EffectError::InvalidEvidence)?
                .dev()
                != identity.device
            {
                return bad();
            }
            files.push((name.to_owned(), file));
        }
        if !matches!(
            fs::statat(&fd, "state_5.sqlite-journal", AtFlags::SYMLINK_NOFOLLOW),
            Err(rustix::io::Errno::NOENT)
        ) {
            return bad();
        }
        Ok(Self {
            root: identity.path.clone(),
            files,
        })
    }
    pub(crate) fn validate(&self, manifest: &CleanupManifest) -> Result<(), EffectError> {
        if self.root != manifest.root.path {
            return bad();
        }
        let fd = root(&self.root)?;
        check_root(&fd, &manifest.root)?;
        for (name, held) in &self.files {
            let metadata = held.metadata().map_err(|_| EffectError::InvalidEvidence)?;
            let stat = fs::statat(&fd, name.as_str(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| EffectError::InvalidEvidence)?;
            if !metadata.is_file()
                || metadata.nlink() != 1
                || stat.st_nlink as u64 != 1
                || stat.st_dev as u64 != metadata.dev()
                || stat.st_ino as u64 != metadata.ino()
            {
                return bad();
            }
        }
        if !matches!(
            fs::statat(&fd, "state_5.sqlite-journal", AtFlags::SYMLINK_NOFOLLOW),
            Err(rustix::io::Errno::NOENT)
        ) {
            return bad();
        }
        Ok(())
    }
}
impl WriterFence {
    pub(crate) fn acquire(manifest: &CleanupManifest) -> Result<Self, EffectError> {
        let root = root(&manifest.root.path)?;
        check_root(&root, &manifest.root)?;
        let tmp = directory(&root, ".tmp".as_ref())?;
        let maintenance = lock(open_file(&tmp, "rollout-maintenance.lock".as_ref())?)?;
        let writers = directory(&root, "thread-writer-locks".as_ref())?;
        let coordination = lock(open_file(&writers, ".coordination.lock".as_ref())?)?;
        let mut locks = vec![
            (
                manifest.root.path.join(".tmp/rollout-maintenance.lock"),
                maintenance,
            ),
            (
                manifest
                    .root
                    .path
                    .join("thread-writer-locks/.coordination.lock"),
                coordination,
            ),
        ];
        for name in names(&writers)? {
            let n = name.as_bytes();
            if n == b".coordination.lock" {
                continue;
            }
            if !valid_lock_name(n) {
                return bad();
            }
            locks.push((
                manifest.root.path.join("thread-writer-locks").join(&name),
                lock(open_file(&writers, &name)?)?,
            ));
        }
        Ok(Self { locks })
    }
    pub(crate) fn validate(&self, manifest: &CleanupManifest) -> Result<(), EffectError> {
        let fd = root(&manifest.root.path)?;
        check_root(&fd, &manifest.root)?;
        let writers = directory(&fd, "thread-writer-locks".as_ref())?;
        let names = names(&writers)?;
        if names.len() + 1 != self.locks.len() {
            return bad();
        }
        for (path, file) in &self.locks {
            let (parent, name) = parent(&fd, &manifest.root.path, path)?;
            let current = open_file(&parent, &name)?;
            let a = file.metadata().map_err(|_| EffectError::InvalidEvidence)?;
            let b = current
                .metadata()
                .map_err(|_| EffectError::InvalidEvidence)?;
            if a.dev() != b.dev() || a.ino() != b.ino() || a.nlink() != 1 || b.nlink() != 1 {
                return bad();
            }
        }
        Ok(())
    }
}
fn valid_lock_name(n: &[u8]) -> bool {
    n.len() == 41 && n.ends_with(b".lock") && valid_uuid(&n[..36])
}
fn valid_uuid(n: &[u8]) -> bool {
    n.len() == 36
        && n.iter().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                *b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(b)
            }
        })
}
pub(super) fn valid_rollout_name(path: &Path, id: &str) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let suffix = format!("-{id}.jsonl");
    let Some(stamp) = name
        .strip_prefix("rollout-")
        .and_then(|s| s.strip_suffix(&suffix))
    else {
        return false;
    };
    let b = stamp.as_bytes();
    if b.len() != 19
        || b.iter().enumerate().any(|(i, c)| {
            if [4, 7, 13, 16].contains(&i) {
                *c != b'-'
            } else if i == 10 {
                *c != b'T'
            } else {
                !c.is_ascii_digit()
            }
        })
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| {
        std::str::from_utf8(&b[range])
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
    };
    let (Some(y), Some(m), Some(d), Some(h), Some(min), Some(sec)) = (
        parse(0..4),
        parse(5..7),
        parse(8..10),
        parse(11..13),
        parse(14..16),
        parse(17..19),
    ) else {
        return false;
    };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    y > 0 && d >= 1 && d <= days && h < 24 && min < 60 && sec < 60
}
struct UniqueJson(serde_json::Value);
impl<'de> serde::Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(n.into()))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = a.next_element::<UniqueJson>()? {
                    out.push(v.0);
                }
                Ok(UniqueJson(out.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    let value = a.next_value::<UniqueJson>()?;
                    if out.insert(key, value.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(UniqueJson(out.into()))
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn header(file: &File) -> Result<(String, String, Option<String>), EffectError> {
    let mut line = Vec::new();
    let mut reader = BufReader::new(
        file.try_clone()
            .map_err(|_| EffectError::InvalidEvidence)?
            .take(HEADER + 1),
    );
    reader
        .read_until(b'\n', &mut line)
        .map_err(|_| EffectError::InvalidEvidence)?;
    if line.is_empty() || line.len() as u64 > HEADER || line.last() != Some(&b'\n') {
        return bad();
    }
    let value = serde_json::from_slice::<UniqueJson>(&line)
        .map_err(|_| EffectError::InvalidEvidence)?
        .0;
    if value.get("type").and_then(|v| v.as_str()) != Some("session_meta") {
        return bad();
    }
    let payload = value
        .get("payload")
        .and_then(|v| v.as_object())
        .ok_or(EffectError::InvalidEvidence)?;
    // History-base and attachment forms are unsupported until their closure is modeled.
    fn unsupported(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
                let key = key.to_ascii_lowercase().replace('_', "");
                ((key.contains("historybase")
                    || key.contains("attachment")
                    || key.contains("revert"))
                    && !value.is_null())
                    || (key.contains("attachedthread")
                        && !value.is_null()
                        && value != &serde_json::Value::Bool(false))
                    || (key == "historymode"
                        && !value.is_null()
                        && value.as_str() != Some("legacy"))
                    || unsupported(value)
            }),
            serde_json::Value::Array(values) => values.iter().any(unsupported),
            _ => false,
        }
    }
    if unsupported(&value)
        || payload
            .get("attached_thread")
            .is_some_and(|v| !v.is_null() && v != &serde_json::Value::Bool(false))
        || payload
            .get("history_mode")
            .is_some_and(|v| !v.is_null() && v.as_str() != Some("legacy"))
    {
        return bad();
    }
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(EffectError::InvalidEvidence)?
        .to_owned();
    if !valid_uuid(id.as_bytes()) {
        return bad();
    }
    Ok((
        id,
        Sha256::digest(&line)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        payload
            .get("cli_version")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    ))
}
fn walk(
    fd: &OwnedFd,
    path: &Path,
    out: &mut Vec<(PathBuf, File)>,
    depth: usize,
    total: &mut usize,
) -> Result<(), EffectError> {
    if depth > 8 {
        return bad();
    }
    for n in names(fd)? {
        *total += 1;
        if *total > LIMIT {
            return bad();
        }
        let p = path.join(&n);
        let st = fs::statat(fd, &n, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|_| EffectError::InvalidEvidence)?;
        let ty = fs::FileType::from_raw_mode(st.st_mode);
        if ty == fs::FileType::Directory {
            let child = directory(fd, &n)?;
            walk(&child, &p, out, depth + 1, total)?;
        } else if ty == fs::FileType::RegularFile
            && n.as_bytes().starts_with(b"rollout-")
            && n.as_bytes().ends_with(b".jsonl")
        {
            out.push((p, open_file(fd, &n)?));
        } else if n.as_bytes().starts_with(b"rollout-") {
            return bad();
        }
    }
    Ok(())
}
fn inventory(
    manifest: &CleanupManifest,
    rootfd: &OwnedFd,
    absent: bool,
) -> Result<(), EffectError> {
    let mut files = Vec::new();
    let mut total = 0;
    for name in ["sessions", "archived_sessions"] {
        match directory(rootfd, name.as_ref()) {
            Ok(fd) => walk(
                &fd,
                &manifest.root.path.join(name),
                &mut files,
                0,
                &mut total,
            )?,
            Err(_) => {
                if !matches!(
                    fs::statat(rootfd, name, AtFlags::SYMLINK_NOFOLLOW),
                    Err(rustix::io::Errno::NOENT)
                ) {
                    return bad();
                }
                if manifest
                    .payloads
                    .iter()
                    .any(|p| p.identity.path.starts_with(manifest.root.path.join(name)))
                    || manifest
                        .retained_files
                        .iter()
                        .any(|p| p.path.starts_with(manifest.root.path.join(name)))
                {
                    return bad();
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for (path, file) in files {
        let (id, digest, version) = header(&file)?;
        if !ids.insert(id.clone()) {
            return bad();
        }
        if !valid_rollout_name(&path, &id) {
            return bad();
        }
        if let Some(p) = manifest.payloads.iter().find(|p| p.identity.path == path) {
            if absent
                || !matches_identity(&file, &p.identity)
                || id != p.header_session_id.as_str()
                || digest != p.header_sha256
                || version.as_deref() != Some(crate::scoped_profile::SCOPED_CODEX_VERSION)
            {
                return bad();
            }
        } else if let Some(retained) = manifest.retained_files.iter().find(|p| p.path == path) {
            if !matches_identity(&file, retained) || id == manifest.native_session_id.as_str() {
                return bad();
            }
        } else {
            return bad();
        }
        if !seen.insert(path) {
            return bad();
        }
    }
    if manifest
        .retained_files
        .iter()
        .any(|f| !seen.contains(&f.path))
        || (!absent
            && manifest
                .payloads
                .iter()
                .any(|f| !seen.contains(&f.identity.path)))
    {
        return bad();
    }
    Ok(())
}

/// Bounded metadata-only inventory for planning; never reads transcript bodies.
type RolloutObservation = (CleanupFileIdentity, String, String, Option<String>);
pub(crate) fn inspect_rollouts(path: &Path) -> Result<Vec<RolloutObservation>, EffectError> {
    let rootfd = root(path)?;
    let mut files = Vec::new();
    let mut total = 0;
    for name in ["sessions", "archived_sessions"] {
        match directory(&rootfd, name.as_ref()) {
            Ok(fd) => walk(&fd, &path.join(name), &mut files, 0, &mut total)?,
            Err(_)
                if matches!(
                    fs::statat(&rootfd, name, AtFlags::SYMLINK_NOFOLLOW),
                    Err(rustix::io::Errno::NOENT)
                ) => {}
            Err(_) => return bad(),
        }
    }
    let mut ids = BTreeSet::new();
    let mut out = Vec::new();
    for (path, file) in files {
        let (id, digest, version) = header(&file)?;
        if !valid_rollout_name(&path, &id) {
            return bad();
        }
        if !ids.insert(id.clone()) {
            return bad();
        }
        let m = file.metadata().map_err(|_| EffectError::InvalidEvidence)?;
        out.push((
            CleanupFileIdentity {
                path,
                device: m.dev(),
                inode: m.ino(),
                links: m.nlink(),
                size_bytes: m.len(),
                mtime_seconds: m.mtime(),
                mtime_nanoseconds: m.mtime_nsec() as u32,
            },
            id,
            digest,
            version,
        ));
    }
    Ok(out)
}

pub(crate) struct FileObservation {
    pub root: CleanupFileIdentity,
    pub database: CleanupFileIdentity,
    pub payload: CleanupFileIdentity,
    pub header_sha256: String,
    pub retained_files: Vec<CleanupFileIdentity>,
}
fn identity(path: PathBuf, file: &File) -> Result<CleanupFileIdentity, EffectError> {
    let m = file.metadata().map_err(|_| EffectError::InvalidEvidence)?;
    Ok(CleanupFileIdentity {
        path,
        device: m.dev(),
        inode: m.ino(),
        links: m.nlink(),
        size_bytes: m.len(),
        mtime_seconds: m.mtime(),
        mtime_nanoseconds: m.mtime_nsec() as u32,
    })
}
/// Collect only metadata and bounded first lines; no lock creation or acquisition.
fn no_history_store(fd: &OwnedFd) -> Result<(), EffectError> {
    for name in [
        "thread_history_1.sqlite",
        "thread_history_1.sqlite-wal",
        "thread_history_1.sqlite-shm",
        "thread_history_1.sqlite-journal",
    ] {
        if !matches!(
            fs::statat(fd, name, AtFlags::SYMLINK_NOFOLLOW),
            Err(rustix::io::Errno::NOENT)
        ) {
            return bad();
        }
    }
    Ok(())
}
pub(crate) fn observe(
    path: &Path,
    target: &str,
    db_target_path: &Path,
) -> Result<FileObservation, EffectError> {
    let fd = root(path)?;
    no_history_store(&fd)?;
    let root_file = File::from(
        fs::openat(
            &fd,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| EffectError::InvalidEvidence)?,
    );
    let root_identity = identity(path.to_owned(), &root_file)?;
    let _sidecars = DatabaseSidecars::from_root(&root_identity)?;
    let database_file = open_file(&fd, "state_5.sqlite".as_ref())?;
    let database = identity(path.join("state_5.sqlite"), &database_file)?;
    if database.device != root_identity.device {
        return bad();
    }
    let tmp = directory(&fd, ".tmp".as_ref())?;
    let _maintenance = open_file(&tmp, "rollout-maintenance.lock".as_ref())?;
    let writers = directory(&fd, "thread-writer-locks".as_ref())?;
    let _coordination = open_file(&writers, ".coordination.lock".as_ref())?;
    for name in names(&writers)? {
        if name.as_bytes() == b".coordination.lock" {
            continue;
        }
        if !valid_lock_name(name.as_bytes()) {
            return bad();
        }
        let _writer = open_file(&writers, &name)?;
    }
    let mut payload = None;
    let mut retained_files = Vec::new();
    for (file, id, digest, version) in inspect_rollouts(path)? {
        if file.device != root_identity.device {
            return bad();
        }
        if id == target {
            if file.path != db_target_path
                || payload.is_some()
                || version.as_deref() != Some(crate::scoped_profile::SCOPED_CODEX_VERSION)
            {
                return bad();
            }
            payload = Some((file, digest));
        } else {
            retained_files.push(file);
        }
    }
    let (payload, header_sha256) = payload.ok_or(EffectError::InvalidEvidence)?;
    check_root(&fd, &root_identity)?;
    Ok(FileObservation {
        root: root_identity,
        database,
        payload,
        header_sha256,
        retained_files,
    })
}
pub(crate) fn validate_before_unlink(m: &CleanupManifest) -> Result<(), EffectError> {
    if !agentmender_domain::validate_cleanup_manifest(m) {
        return bad();
    }
    let fd = root(&m.root.path)?;
    check_root(&fd, &m.root)?;
    no_history_store(&fd)?;
    inventory(m, &fd, false)
}
pub(crate) fn validate_and_unlink(m: &CleanupManifest) -> Result<u64, EffectError> {
    validate_before_unlink(m)?;
    if m.payloads.len() != 1 {
        return bad();
    }
    let fd = root(&m.root.path)?;
    check_root(&fd, &m.root)?;
    let p = &m.payloads[0];
    let (parent, name) = parent(&fd, &m.root.path, &p.identity.path)?;
    let f = open_file(&parent, &name)?;
    if !matches_identity(&f, &p.identity) {
        return bad();
    }
    let st = fs::statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|_| EffectError::InvalidEvidence)?;
    if st.st_ino as u64 != p.identity.inode || st.st_dev as u64 != p.identity.device {
        return bad();
    }
    fs::unlinkat(&parent, &name, AtFlags::empty()).map_err(|_| EffectError::Transport)?;
    fs::fsync(&parent).map_err(|_| EffectError::Transport)?;
    Ok(p.identity.size_bytes)
}
pub(crate) fn verify_retained_and_absence(m: &CleanupManifest) -> Result<bool, EffectError> {
    let fd = root(&m.root.path)?;
    check_root(&fd, &m.root)?;
    no_history_store(&fd)?;
    inventory(m, &fd, true)?;
    Ok(true)
}

pub(crate) fn reconcile_files(
    m: &CleanupManifest,
) -> Result<(agentmender_domain::CleanupActionOutcome, bool), EffectError> {
    let fd = root(&m.root.path)?;
    check_root(&fd, &m.root)?;
    no_history_store(&fd)?;
    if inventory(m, &fd, true).is_ok() {
        return Ok((
            agentmender_domain::CleanupActionOutcome::AlreadyAbsent,
            true,
        ));
    }
    if inventory(m, &fd, false).is_ok() {
        return Ok((agentmender_domain::CleanupActionOutcome::Retained, true));
    }
    Err(EffectError::InvalidEvidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoped_profile::*;
    use agentmender_domain::*;
    use std::fs;
    fn id(s: &str) -> EntityId {
        EntityId::new(s).unwrap()
    }
    fn fixture() -> (tempfile::TempDir, CleanupManifest) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().canonicalize().unwrap();
        fs::create_dir(path.join("sessions")).unwrap();
        fs::create_dir(path.join(".tmp")).unwrap();
        fs::create_dir(path.join("thread-writer-locks")).unwrap();
        fs::write(path.join(".tmp/rollout-maintenance.lock"), b"").unwrap();
        fs::write(path.join("thread-writer-locks/.coordination.lock"), b"").unwrap();
        fs::write(path.join("state_5.sqlite"), b"db").unwrap();
        fs::write(path.join("state_5.sqlite-wal"), b"wal").unwrap();
        fs::write(path.join("state_5.sqlite-shm"), b"shm").unwrap();
        let native = "12345678-1234-1234-1234-123456789abc";
        let target = path.join(format!(
            "sessions/rollout-2026-01-01T00-00-00-{native}.jsonl"
        ));
        let child = path.join(
            "sessions/rollout-2026-01-01T00-00-01-aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.jsonl",
        );
        fs::write(
            &target,
            format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{native}\",\"cli_version\":\"0.166.0\"}}}}\nbody\n"),
        )
        .unwrap();
        fs::write(&child, "{\"type\":\"session_meta\",\"payload\":{\"id\":\"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\"}}\nretained\n").unwrap();
        let o = observe(&path, native, &target).unwrap();
        let artifact = |local: &str, class| ArtifactId {
            provider: ProviderKind::Codex,
            state_root: path.clone(),
            class,
            local_id: id(local),
        };
        let m = CleanupManifest {
            target: artifact("target", GraphNodeKind::Session),
            native_session_id: id(native),
            root: o.root,
            profile: CleanupProfile {
                version: SCOPED_CODEX_VERSION.into(),
                executable_sha256: SCOPED_CODEX_EXECUTABLE_SHA256.into(),
                schema_sha256: SCOPED_CODEX_SCHEMA_SHA256.into(),
                adapter: SCOPED_ADAPTER.into(),
                platform: "darwin-arm64".into(),
                journal_mode: "wal".into(),
                history_layout: "legacy-plain".into(),
            },
            database: o.database,
            rows: [
                "threads",
                "thread_dynamic_tools",
                "thread_read_receipts",
                "thread_attachments",
                "guardian_review_feedback",
                "thread_spawn_edges",
            ]
            .iter()
            .map(|table| CleanupRows {
                table: (*table).into(),
                expected_count: u64::from(*table == "threads"),
                preimage_sha256: "a".repeat(64),
            })
            .collect(),
            database_payload_paths: vec![CleanupDatabasePath { path: target }],
            payloads: vec![CleanupPayload {
                artifact: artifact("payload", GraphNodeKind::Artifact),
                identity: o.payload,
                header_session_id: id(native),
                header_sha256: o.header_sha256,
            }],
            optional_index: None,
            retained_database_sha256: "b".repeat(64),
            retained_files: o.retained_files,
            residuals: vec![],
        };
        (temp, m)
    }
    #[test]
    fn exact_unlink_preserves_other_rollout_and_unknown_file() {
        let (_temp, m) = fixture();
        assert!(agentmender_domain::validate_cleanup_manifest(&m));
        let retained = m.retained_files[0].path.clone();
        let payload = m.payloads[0].identity.path.clone();
        fs::write(m.root.path.join("sessions/notes.txt"), b"unknown").unwrap();
        let _fence = WriterFence::acquire(&m).unwrap();
        assert_eq!(
            validate_and_unlink(&m).unwrap(),
            m.payloads[0].identity.size_bytes
        );
        assert!(!payload.exists());
        assert!(retained.exists());
        assert!(m.root.path.join("sessions/notes.txt").exists());
        assert!(verify_retained_and_absence(&m).unwrap());
    }
    #[test]
    fn generated_filesystem_cases_preserve_other_sessions_and_unknown_bytes() {
        let mut seed = 0x6d38_2f53_636f_7065_u64;
        for _ in 0..128 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let (_temp, m) = fixture();
            let target = &m.payloads[0].identity.path;
            let retained = &m.retained_files[0].path;
            let original = fs::read(retained).unwrap();
            let unknown = m.root.path.join("sessions/unknown.cache");
            let bytes = seed.to_be_bytes();
            fs::write(&unknown, bytes).unwrap();
            let _fence = WriterFence::acquire(&m).unwrap();
            if seed & 1 == 0 {
                assert_eq!(
                    validate_and_unlink(&m).unwrap(),
                    m.payloads[0].identity.size_bytes
                );
                assert!(!target.exists());
            } else {
                use std::io::Write;
                fs::OpenOptions::new()
                    .append(true)
                    .open(target)
                    .unwrap()
                    .write_all(&bytes)
                    .unwrap();
                assert!(validate_and_unlink(&m).is_err());
                assert!(target.exists());
            }
            assert_eq!(fs::read(retained).unwrap(), original);
            assert_eq!(fs::read(unknown).unwrap(), bytes);
        }
    }
    #[test]
    fn writer_contention_and_changed_or_linked_payload_refuse() {
        let (_temp, m) = fixture();
        let fence = WriterFence::acquire(&m).unwrap();
        assert!(WriterFence::acquire(&m).is_err());
        drop(fence);
        fs::write(&m.payloads[0].identity.path, b"changed").unwrap();
        assert!(validate_before_unlink(&m).is_err());
        assert!(m.payloads[0].identity.path.exists());
    }
    #[test]
    fn nested_history_reference_and_namespace_symlink_refuse() {
        let (_temp, m) = fixture();
        let child = &m.retained_files[0].path;
        fs::write(child, "{\"type\":\"session_meta\",\"payload\":{\"id\":\"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\",\"source\":{\"history_base\":{}}}}\n").unwrap();
        assert!(validate_before_unlink(&m).is_err());
        std::os::unix::fs::symlink("sessions", m.root.path.join("archived_sessions")).unwrap();
        assert!(inspect_rollouts(&m.root.path).is_err());
    }
    #[test]
    fn duplicate_keys_and_attached_or_paginated_headers_refuse() {
        let (_temp, m) = fixture();
        let file = &m.payloads[0].identity.path;
        for text in [
            r#"{"type":"session_meta","payload":{"id":"12345678-1234-1234-1234-123456789abc","id":"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"}}"#,
            r#"{"type":"session_meta","payload":{"id":"12345678-1234-1234-1234-123456789abc","attached_thread":true}}"#,
            r#"{"type":"session_meta","payload":{"id":"12345678-1234-1234-1234-123456789abc","history_mode":"paginated"}}"#,
        ] {
            fs::write(file, format!("{text}\n")).unwrap();
            assert!(inspect_rollouts(&m.root.path).is_err());
        }
    }
    #[test]
    fn hardlinks_and_lock_inode_replacement_refuse() {
        let (_temp, m) = fixture();
        let fence = WriterFence::acquire(&m).unwrap();
        fence.validate(&m).unwrap();
        let lock = m.root.path.join("thread-writer-locks/.coordination.lock");
        fs::rename(&lock, m.root.path.join("old-lock")).unwrap();
        fs::write(&lock, b"").unwrap();
        assert!(fence.validate(&m).is_err());
        drop(fence);
        fs::hard_link(&m.payloads[0].identity.path, m.root.path.join("hardlink")).unwrap();
        assert!(validate_before_unlink(&m).is_err());
    }
    #[test]
    fn sqlite_sidecar_links_refuse_before_database_open() {
        let (_temp, m) = fixture();
        let wal = m.root.path.join("state_5.sqlite-wal");
        let shm = m.root.path.join("state_5.sqlite-shm");
        fs::write(&wal, b"wal").unwrap();
        fs::write(&shm, b"shm").unwrap();
        let guard = DatabaseSidecars::acquire(&m).unwrap();
        guard.validate(&m).unwrap();
        drop(guard);
        fs::hard_link(&wal, m.root.path.join("wal-alias")).unwrap();
        assert!(DatabaseSidecars::acquire(&m).is_err());
        fs::remove_file(m.root.path.join("wal-alias")).unwrap();
        fs::remove_file(&shm).unwrap();
        std::os::unix::fs::symlink(&wal, &shm).unwrap();
        assert!(DatabaseSidecars::acquire(&m).is_err());
    }
    #[test]
    fn rollout_names_require_real_timestamp_and_exact_id() {
        let id = "12345678-1234-1234-1234-123456789abc";
        let path = |stamp: &str| PathBuf::from(format!("rollout-{stamp}-{id}.jsonl"));
        assert!(valid_rollout_name(&path("2024-02-29T23-59-59"), id));
        for stamp in [
            "2023-02-29T00-00-00",
            "2026-13-01T00-00-00",
            "2026-01-01T24-00-00",
            "2026-01-01",
            "2026-01-01T00-00-60",
        ] {
            assert!(!valid_rollout_name(&path(stamp), id));
        }
    }
}
