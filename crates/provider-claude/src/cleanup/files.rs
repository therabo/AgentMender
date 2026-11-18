//! Descriptor-relative primitives for the reviewed targets and private backup store.
use super::Refusal;
use rustix::fs::{self as fs_at, AtFlags, Mode, OFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs::{File, Metadata},
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

pub(super) fn directory(path: &Path) -> io::Result<File> {
    let mut parts = path.components();
    if parts.next() != Some(Component::RootDir) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut dir = File::from(fs_at::open("/", DIRECTORY, Mode::empty())?);
    for part in parts {
        let Component::Normal(name) = part else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        dir = File::from(fs_at::openat(&dir, name, DIRECTORY, Mode::empty())?);
    }
    Ok(dir)
}

pub(super) fn directory_at(parent: &File, name: &str) -> io::Result<File> {
    Ok(File::from(fs_at::openat(
        parent,
        name,
        DIRECTORY,
        Mode::empty(),
    )?))
}

pub(super) fn make_private(parent: &File, name: &str, existing: bool) -> io::Result<File> {
    match fs_at::mkdirat(parent, name, Mode::RUSR | Mode::WUSR | Mode::XUSR) {
        Ok(()) => parent.sync_all()?,
        Err(rustix::io::Errno::EXIST) if existing => {}
        Err(error) => return Err(error.into()),
    }
    let dir = directory_at(parent, name)?;
    require_private(&dir)?;
    Ok(dir)
}

pub(super) fn require_private(dir: &File) -> io::Result<()> {
    let metadata = dir.metadata()?;
    if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o077 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

pub(super) fn read_at(dir: &File, name: &OsStr) -> io::Result<File> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = File::from(fs_at::openat(dir, name, flags, Mode::empty())?);
    let meta = file.metadata()?;
    if !meta.is_file() || meta.nlink() != 1 || meta.dev() != dir.metadata()?.dev() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(file)
}

pub(super) fn new_file(dir: &File, name: &str) -> io::Result<File> {
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    Ok(File::from(fs_at::openat(
        dir,
        name,
        flags,
        Mode::RUSR | Mode::WUSR,
    )?))
}

pub(super) fn absent(dir: &File, name: &OsStr) -> io::Result<bool> {
    match fs_at::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(_) => Ok(false),
        Err(rustix::io::Errno::NOENT) => Ok(true),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn file_id(meta: &Metadata) -> (u64, u64) {
    (meta.dev(), meta.ino())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Target {
    #[serde(with = "agentmender_domain::path_serde")]
    pub(super) root: PathBuf,
    #[serde(with = "agentmender_domain::path_serde")]
    pub(super) relative_path: PathBuf,
    pub(super) role: String,
    pub(super) bytes: u64,
    allocated_bytes: u64,
    root_id: (u64, u64),
    parent_id: (u64, u64),
    identity: (u64, u64),
    modified: (i64, i64),
    changed: (i64, i64),
    pub(super) sha256: String,
    mode: u32,
}

impl Target {
    pub(super) fn inspect(
        root: &Path,
        path: &Path,
        role: &str,
        expected: Option<&crate::Revision>,
        limit: u64,
    ) -> Result<(Self, u64), Refusal> {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Refusal::OwnershipUnverified)?;
        let root_handle = directory(root).map_err(|_| Refusal::Changed)?;
        let parent =
            directory(path.parent().ok_or(Refusal::Changed)?).map_err(|_| Refusal::Changed)?;
        let name = path.file_name().ok_or(Refusal::Changed)?;
        let mut source = read_at(&parent, name).map_err(|_| Refusal::OwnershipUnverified)?;
        let meta = source.metadata().map_err(|_| Refusal::Changed)?;
        let root_id = file_id(&root_handle.metadata().map_err(|_| Refusal::Changed)?);
        if meta.dev() != root_id.0
            || meta.mode() & 0o7000 != 0
            || meta.uid() != rustix::process::geteuid().as_raw()
        {
            return Err(Refusal::OwnershipUnverified);
        }
        if meta.len() > limit {
            return Err(Refusal::PartialScope);
        }
        let revision = crate::Revision::read(&source, path).map_err(|_| Refusal::Changed)?;
        if expected.is_some_and(|expected| *expected != revision) {
            return Err(Refusal::Changed);
        }
        let sha256 = transfer(&mut source, meta.len(), io::sink()).map_err(|_| Refusal::Changed)?;
        let target = Self {
            root: root.to_owned(),
            relative_path: relative.to_owned(),
            role: role.into(),
            bytes: meta.len(),
            allocated_bytes: meta.blocks().saturating_mul(512),
            root_id,
            parent_id: file_id(&parent.metadata().map_err(|_| Refusal::Changed)?),
            identity: file_id(&meta),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
            sha256,
            mode: meta.mode() & 0o777,
        };
        target.open_current().map_err(|_| Refusal::Changed)?;
        Ok((target, meta.len()))
    }

    pub(super) fn name(&self) -> &OsStr {
        self.relative_path.file_name().expect("validated target")
    }

    pub(super) fn parent(&self) -> io::Result<File> {
        let path = self.root.join(&self.relative_path);
        let parent = directory(path.parent().ok_or(io::ErrorKind::InvalidData)?)?;
        self.check_parent(&parent)?;
        Ok(parent)
    }

    pub(super) fn check_parent(&self, parent: &File) -> io::Result<()> {
        let root = directory(&self.root)?;
        if file_id(&root.metadata()?) != self.root_id
            || file_id(&parent.metadata()?) != self.parent_id
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(())
    }

    pub(super) fn volume(&self) -> u64 {
        self.root_id.0
    }

    pub(super) fn matches(&self, file: &File, renamed: bool) -> io::Result<bool> {
        let meta = file.metadata()?;
        Ok(file_id(&meta) == self.identity
            && meta.len() == self.bytes
            && (meta.mtime(), meta.mtime_nsec()) == self.modified
            && (renamed || (meta.ctime(), meta.ctime_nsec()) == self.changed)
            && meta.mode() & 0o777 == self.mode)
    }

    pub(super) fn open_current(&self) -> io::Result<(File, File)> {
        let parent = self.parent()?;
        let file = read_at(&parent, self.name())?;
        if !self.matches(&file, false)? {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok((parent, file))
    }

    pub(super) fn verify_data(&self, mut file: File) -> io::Result<()> {
        if transfer(&mut file, self.bytes, io::sink())? != self.sha256 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(())
    }

    pub(super) fn restore_mode(&self, file: &File) -> io::Result<()> {
        fs_at::fchmod(file, Mode::from_bits_truncate((self.mode & 0o777) as u16))?;
        Ok(())
    }
}

pub(super) fn transfer(file: &mut File, bytes: u64, mut dest: impl Write) -> io::Result<String> {
    let mut limited = file.take(bytes.saturating_add(1));
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = limited.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        count += read as u64;
        if count > bytes {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let part = &buffer[..read];
        dest.write_all(part)?;
        hash.update(part);
    }
    if count != bytes {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn backup_parent(home: &Path, create: bool) -> io::Result<Option<File>> {
    let mut dir = directory(home)?;
    for part in [".agentmender", "claude-backups"] {
        if create {
            dir = make_private(&dir, part, true)?;
        } else {
            match directory_at(&dir, part) {
                Ok(next) => {
                    require_private(&next)?;
                    dir = next;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error),
            }
        }
    }
    Ok(Some(dir))
}

pub(super) fn check_space(home: &Path, targets: &[Target], bytes: u64) -> Result<(), Refusal> {
    let dir = backup_parent(home, false)
        .map_err(|_| Refusal::PartialScope)?
        .map_or_else(|| directory(home), Ok)
        .map_err(|_| Refusal::InsufficientBackupSpace)?;
    let limit = fs_at::fstatvfs(&dir).map_err(|_| Refusal::InsufficientBackupSpace)?;
    let volume = dir
        .metadata()
        .map_err(|_| Refusal::InsufficientBackupSpace)?
        .dev();
    if limit.f_bavail.saturating_mul(limit.f_frsize) < bytes
        || targets.iter().any(|target| target.volume() != volume)
    {
        return Err(Refusal::InsufficientBackupSpace);
    }
    Ok(())
}

pub(super) fn checkpoint_name(name: &str) -> bool {
    let Some((hash, version)) = name.rsplit_once("@v") else {
        return false;
    };
    (8..=64).contains(&hash.len())
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && (1..=10).contains(&version.len())
        && version.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DirectoryTarget {
    #[serde(with = "agentmender_domain::path_serde")]
    pub(super) root: PathBuf,
    #[serde(with = "agentmender_domain::path_serde")]
    pub(super) relative_path: PathBuf,
    root_id: (u64, u64),
    pub(super) parent_id: (u64, u64),
    pub(super) identity: (u64, u64),
}

impl DirectoryTarget {
    pub(super) fn inspect(root: &Path, path: &Path) -> Result<Option<Self>, Refusal> {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Refusal::OwnershipUnverified)?;
        let root_id = directory(root)
            .and_then(|file| file.metadata())
            .map(|meta| file_id(&meta))
            .map_err(|_| Refusal::Changed)?;
        let source = match directory(path).and_then(|file| file.metadata()) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Refusal::OwnershipUnverified),
        };
        let parent = directory(path.parent().ok_or(Refusal::Changed)?)
            .and_then(|dir| dir.metadata())
            .map_err(|_| Refusal::Changed)?;
        if source.dev() != root_id.0 || source.uid() != rustix::process::geteuid().as_raw() {
            return Err(Refusal::OwnershipUnverified);
        }
        Ok(Some(Self {
            root: root.to_owned(),
            relative_path: relative.to_owned(),
            root_id,
            parent_id: file_id(&parent),
            identity: file_id(&source),
        }))
    }

    pub(super) fn path(&self) -> PathBuf {
        self.root.join(&self.relative_path)
    }

    pub(super) fn parent(&self) -> io::Result<File> {
        let path = self.path();
        let root = directory(&self.root)?;
        let parent = directory(path.parent().ok_or(io::ErrorKind::InvalidData)?)?;
        if file_id(&root.metadata()?) != self.root_id
            || file_id(&parent.metadata()?) != self.parent_id
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(parent)
    }

    pub(super) fn name(&self) -> &OsStr {
        self.relative_path.file_name().expect("validated directory")
    }

    pub(super) fn checked(&self, parent: &File, name: &OsStr) -> io::Result<File> {
        let dir = File::from(fs_at::openat(parent, name, DIRECTORY, Mode::empty())?);
        if file_id(&dir.metadata()?) != self.identity {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(dir)
    }
}

pub(super) fn empty(dir: &File) -> io::Result<()> {
    for entry in fs_at::Dir::read_from(dir)? {
        if !matches!(entry?.file_name().to_bytes(), b"." | b"..") {
            return Err(io::ErrorKind::DirectoryNotEmpty.into());
        }
    }
    Ok(())
}
