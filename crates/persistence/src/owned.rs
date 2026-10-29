//! Explicit private storage capability. Ancestor replacement by the same user is
//! not fully prevented; descriptor-relative traversal is reserved for M10.
use agentmender_domain::RecoveryDigest;
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
};

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn checked_path(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        if path
            .as_os_str()
            .as_bytes()
            .split(|b| *b == b'/')
            .any(|p| p == b"." || p == b"..")
        {
            return Err(invalid("non-normal path"));
        }
    }
    #[cfg(windows)]
    {
        if path
            .as_os_str()
            .to_string_lossy()
            .split(['/', '\\'])
            .any(|p| p == "." || p == "..")
        {
            return Err(invalid("non-normal path"));
        }
    }
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(invalid("absolute normalized path required"));
    }
    #[cfg(windows)]
    for c in path.components() {
        if let Component::Normal(s) = c {
            let n = s.to_string_lossy().to_ascii_uppercase();
            let base = n.split('.').next().unwrap_or("");
            if n.contains(':')
                || n.ends_with(' ')
                || n.ends_with('.')
                || matches!(
                    base,
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
            {
                return Err(invalid("unsafe Windows path"));
            }
        }
    }
    Ok(())
}
fn resolved(path: &Path) -> io::Result<PathBuf> {
    let mut missing = Vec::new();
    let mut p = path;
    loop {
        match fs::canonicalize(p) {
            Ok(mut result) => {
                for item in missing.iter().rev() {
                    result.push(item);
                }
                return Ok(result);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // canonicalize follows links: a dangling link is present even
                // though its target is not. Treat it as protected uncertainty,
                // never as a plain missing component.
                match fs::symlink_metadata(p) {
                    Ok(_) => return Err(invalid("unresolved existing path component")),
                    Err(link_error) if link_error.kind() == io::ErrorKind::NotFound => {}
                    Err(link_error) => return Err(link_error),
                }
                missing.push(
                    p.file_name()
                        .ok_or_else(|| invalid("invalid path"))?
                        .to_os_string(),
                );
                p = p.parent().ok_or_else(|| invalid("invalid path"))?;
            }
            Err(e) => return Err(e),
        }
    }
}
fn overlap(a: &Path, b: &Path) -> bool {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        a.starts_with(b) || b.starts_with(a)
    }
    #[cfg(any(windows, target_os = "macos"))]
    {
        let aa: Vec<_> = a
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect();
        let bb: Vec<_> = b
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect();
        aa.starts_with(&bb) || bb.starts_with(&aa)
    }
}
// Compare an existing endpoint with every existing ancestor of the other
// endpoint. This detects aliases whose textual canonical spelling differs.
fn physical_overlap(a: &Path, b: &Path) -> io::Result<bool> {
    fn contains_existing(endpoint: &Path, candidate: &Path) -> io::Result<bool> {
        let endpoint_meta = match fs::metadata(endpoint) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        #[cfg(windows)]
        let endpoint_id = windows_identity(&windows_handle(&fs::canonicalize(endpoint)?)?)?;
        for ancestor in candidate.ancestors() {
            match fs::metadata(ancestor) {
                Ok(m) => {
                    #[cfg(unix)]
                    if same(&endpoint_meta, &m) {
                        return Ok(true);
                    }
                    #[cfg(windows)]
                    {
                        let _ = (&endpoint_meta, &m);
                        let id = windows_identity(&windows_handle(&fs::canonicalize(ancestor)?)?)?;
                        if id.0 == endpoint_id.0 && id.1 == endpoint_id.1 {
                            return Ok(true);
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(false)
    }
    if contains_existing(a, b)? || contains_existing(b, a)? {
        return Ok(true);
    }
    // Both endpoints may be absent under aliases of the same existing
    // directory. Compare that directory's identity before the absent suffixes.
    fn anchor(path: &Path) -> io::Result<(PathBuf, Vec<std::ffi::OsString>, fs::Metadata)> {
        let mut current = path;
        let mut tail = Vec::new();
        loop {
            match fs::metadata(current) {
                Ok(meta) => {
                    tail.reverse();
                    return Ok((current.to_path_buf(), tail, meta));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    tail.push(
                        current
                            .file_name()
                            .ok_or_else(|| invalid("invalid path"))?
                            .to_os_string(),
                    );
                    current = current.parent().ok_or_else(|| invalid("invalid path"))?;
                }
                Err(e) => return Err(e),
            }
        }
    }
    let (anchor_a, tail_a, meta_a) = anchor(a)?;
    let (anchor_b, tail_b, meta_b) = anchor(b)?;
    #[cfg(unix)]
    let same_anchor = {
        let _ = (&anchor_a, &anchor_b);
        same(&meta_a, &meta_b)
    };
    #[cfg(windows)]
    let same_anchor = {
        let _ = (&meta_a, &meta_b);
        let aa = windows_identity(&windows_handle(&fs::canonicalize(anchor_a)?)?)?;
        let bb = windows_identity(&windows_handle(&fs::canonicalize(anchor_b)?)?)?;
        aa.0 == bb.0 && aa.1 == bb.1
    };
    if !same_anchor {
        return Ok(false);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    return Ok(tail_a.starts_with(&tail_b) || tail_b.starts_with(&tail_a));
    #[cfg(any(target_os = "macos", windows))]
    {
        // Missing components have already passed validate_missing_tail.
        let aa: Vec<_> = tail_a
            .iter()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .collect();
        let bb: Vec<_> = tail_b
            .iter()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .collect();
        Ok(aa.starts_with(&bb) || bb.starts_with(&aa))
    }
}
// For absent tails on case-insensitive filesystems, Unicode equivalence is
// filesystem-specific and cannot be established by stat. Fail closed there.
#[cfg(any(target_os = "macos", windows))]
fn validate_missing_tail(path: &Path) -> io::Result<()> {
    let mut p = path;
    loop {
        match fs::metadata(p) {
            Ok(_) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let component = p.file_name().ok_or_else(|| invalid("invalid path"))?;
                if !component.to_str().is_some_and(str::is_ascii) {
                    return Err(invalid("non-ASCII missing path cannot be compared safely"));
                }
                p = p.parent().ok_or_else(|| invalid("invalid path"))?;
            }
            Err(e) => return Err(e),
        }
    }
}
#[cfg(unix)]
fn same(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}
#[cfg(windows)]
fn same(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    a.creation_time() == b.creation_time() && a.file_attributes() == b.file_attributes()
}
#[cfg(windows)]
fn windows_handle(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(0x02200000)
        .open(path)
}
#[cfg(windows)]
fn windows_identity(file: &File) -> io::Result<(u64, u64, u64)> {
    let info = winapi_util::file::information(file)?;
    if info.file_attributes() & 0x400 != 0 {
        return Err(invalid("reparse point rejected"));
    }
    Ok((
        info.volume_serial_number(),
        info.file_index(),
        info.number_of_links(),
    ))
}
#[cfg(windows)]
fn windows_match(file: &File, path: &Path, is_file: bool) -> io::Result<()> {
    let a = windows_identity(file)?;
    let b = windows_identity(&windows_handle(path)?)?;
    if a != b || (is_file && a.2 != 1) {
        return Err(invalid("unsafe Windows file identity"));
    }
    Ok(())
}
fn no_link(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    true
}
#[derive(Clone, Debug)]
pub struct OwnedStateRoot {
    path: PathBuf,
    identity: fs::Metadata,
    protected: Vec<PathBuf>,
    #[cfg(windows)]
    root_handle: std::sync::Arc<File>,
}
pub(crate) struct RecoveryStage {
    pub(crate) file: File,
    path: PathBuf,
}
#[derive(Clone, Copy)]
pub(crate) enum FixedFile {
    Database,
    AuthorityKey,
    Journal,
    Lock,
    SqliteRollbackJournal,
    SqliteWal,
    SqliteShm,
}
impl FixedFile {
    fn name(self) -> &'static str {
        match self {
            Self::AuthorityKey => "authority.key",
            Self::Database => "transactions.sqlite3",
            Self::Journal => "transactions.journal",
            Self::Lock => "transactions.lock",
            Self::SqliteRollbackJournal => "transactions.sqlite3-journal",
            Self::SqliteWal => "transactions.sqlite3-wal",
            Self::SqliteShm => "transactions.sqlite3-shm",
        }
    }
}
impl OwnedStateRoot {
    pub fn create(path: &Path, protected: &[PathBuf]) -> io::Result<Self> {
        Self::check_overlap(path, protected)?;
        #[cfg(unix)]
        let b = {
            use std::os::unix::fs::DirBuilderExt;
            let mut b = fs::DirBuilder::new();
            b.mode(0o700);
            b
        };
        #[cfg(not(unix))]
        let b = fs::DirBuilder::new();
        b.create(path)?;
        #[cfg(windows)]
        crate::windows_permissions::secure_new_root(path)?;
        let root = Self::open(path, protected)?;
        root.sync_parent()?;
        Ok(root)
    }
    pub fn open(path: &Path, protected: &[PathBuf]) -> io::Result<Self> {
        Self::check_overlap(path, protected)?;
        let identity = fs::symlink_metadata(path)?;
        if !identity.is_dir() || !no_link(&identity) {
            return Err(invalid("root must be an unlinked directory"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if identity.mode() & 0o7777 != 0o700
                || identity.uid() != rustix::process::geteuid().as_raw()
            {
                return Err(invalid("root permissions must be 0700"));
            }
        }
        #[cfg(windows)]
        crate::windows_permissions::validate(path)?;
        #[cfg(windows)]
        let root_handle = std::sync::Arc::new(windows_handle(path)?);
        Ok(Self {
            #[cfg(windows)]
            root_handle,
            path: path.to_path_buf(),
            identity,
            protected: protected.to_vec(),
        })
    }
    fn check_overlap(path: &Path, protected: &[PathBuf]) -> io::Result<()> {
        checked_path(path)?;
        if protected.is_empty() {
            return Err(invalid("protected roots required"));
        }
        #[cfg(any(target_os = "macos", windows))]
        validate_missing_tail(path)?;
        let root = resolved(path)?;
        for p in protected {
            checked_path(p)?;
            #[cfg(any(target_os = "macos", windows))]
            validate_missing_tail(p)?;
            if overlap(path, p) || overlap(&root, &resolved(p)?) || physical_overlap(path, p)? {
                return Err(invalid("state root overlaps protected provider state"));
            }
        }
        Ok(())
    }
    pub(crate) fn revalidate(&self) -> io::Result<()> {
        Self::check_overlap(&self.path, &self.protected)?;
        let m = fs::symlink_metadata(&self.path)?;
        if !m.is_dir() || !no_link(&m) || !same(&self.identity, &m) {
            return Err(invalid("state root identity changed"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if m.mode() & 0o7777 != 0o700 || m.uid() != self.identity.uid() {
                return Err(invalid("state root permissions changed"));
            }
        }
        #[cfg(windows)]
        {
            crate::windows_permissions::validate(&self.path)?;
            windows_match(&self.root_handle, &self.path, false)?;
        }
        Ok(())
    }
    pub(crate) fn create_recovery_stage(&self) -> io::Result<RecoveryStage> {
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| io::Error::other("recovery nonce unavailable"))?;
        let suffix: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let name = format!("recovery-stage-{suffix}.tmp");
        let file = self.open_child(&name, true, true)?;
        Ok(RecoveryStage {
            file,
            path: self.path.join(name),
        })
    }
    /// Publish only a created recovery staging file. Hard-link creation is
    /// atomic and fails if the destination exists; cleanup is limited to the
    /// staging name after checking both links identify the held file.
    pub(crate) fn publish_recovery(
        &self,
        stage: RecoveryStage,
        digest: &RecoveryDigest,
    ) -> io::Result<()> {
        self.revalidate()?;
        let before = fs::symlink_metadata(&stage.path)?;
        self.check_file(&before)?;
        let held = stage.file.metadata()?;
        self.check_file(&held)?;
        if !same(&before, &held) {
            return Err(invalid("staging identity changed"));
        }
        #[cfg(windows)]
        windows_match(&stage.file, &stage.path, true)?;
        let final_path = self.path.join(format!("recovery-{digest}.json"));
        fs::hard_link(&stage.path, &final_path)?;
        self.revalidate()?;
        let staged = fs::symlink_metadata(&stage.path)?;
        let published = fs::symlink_metadata(&final_path)?;
        if !staged.is_file()
            || !published.is_file()
            || !no_link(&staged)
            || !no_link(&published)
            || !same(&held, &staged)
            || !same(&held, &published)
        {
            return Err(invalid("published recovery identity changed"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if staged.nlink() != 2
                || published.nlink() != 2
                || staged.uid() != self.identity.uid()
                || staged.mode() & 0o7777 != 0o600
                || published.mode() & 0o7777 != 0o600
            {
                return Err(invalid("unsafe published recovery links"));
            }
        }
        #[cfg(windows)]
        {
            let held_id = windows_identity(&stage.file)?;
            let staged_id = windows_identity(&windows_handle(&stage.path)?)?;
            let final_id = windows_identity(&windows_handle(&final_path)?)?;
            if held_id != staged_id || held_id != final_id || held_id.2 != 2 {
                return Err(invalid("unsafe published recovery links"));
            }
            crate::windows_permissions::validate(&stage.path)?;
            crate::windows_permissions::validate(&final_path)?;
        }
        // SAFETY BOUNDARY: remove only this validated random staging name;
        // the immutable digest-named recovery artifact is never removed.
        fs::remove_file(&stage.path)?;
        self.sync_dir()?;
        Ok(())
    }
    /// Check sidecars before SQLite can inspect them. A DELETE-mode database
    /// cannot safely accept WAL or shared-memory state from another mode.
    pub(crate) fn validate_sqlite_sidecars(&self) -> io::Result<()> {
        self.revalidate()?;
        for sidecar in [FixedFile::SqliteWal, FixedFile::SqliteShm] {
            match fs::symlink_metadata(self.fixed_path(sidecar)) {
                Ok(_) => return Err(invalid("unexpected SQLite WAL sidecar")),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        match self.open_existing_fixed(FixedFile::SqliteRollbackJournal, false) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn fixed_path(&self, file: FixedFile) -> PathBuf {
        self.path.join(file.name())
    }
    pub(crate) fn open_fixed(&self, file: FixedFile) -> io::Result<File> {
        self.open_child(file.name(), true, false)
    }
    pub(crate) fn open_existing_fixed(&self, file: FixedFile, writable: bool) -> io::Result<File> {
        self.open_child_existing(file.name(), writable)
    }
    pub(crate) fn open_child_existing(&self, name: &str, writable: bool) -> io::Result<File> {
        self.revalidate()?;
        let path = self.path.join(name);
        let before = fs::symlink_metadata(&path)?;
        self.check_file(&before)?;
        let mut opts = OpenOptions::new();
        opts.read(true).write(writable);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.custom_flags(0x00200000);
        }
        let f = opts.open(&path)?;
        let m = f.metadata()?;
        self.check_file(&m)?;
        let after = fs::symlink_metadata(&path)?;
        self.check_file(&after)?;
        if !same(&before, &m) || !same(&after, &m) {
            return Err(invalid("file identity changed"));
        }
        #[cfg(windows)]
        {
            crate::windows_permissions::validate(&path)?;
            windows_match(&f, &path, true)?;
        }
        self.revalidate()?;
        Ok(f)
    }
    pub(crate) fn open_child(&self, name: &str, write: bool, create_new: bool) -> io::Result<File> {
        self.revalidate()?;
        let path = self.path.join(name);
        let before = match fs::symlink_metadata(&path) {
            Ok(m) => {
                self.check_file(&m)?;
                Some(m)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let mut opts = OpenOptions::new();
        opts.read(true).write(write);
        if create_new || (write && before.is_none()) {
            opts.create_new(true);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.custom_flags(0x00200000);
        }
        let f = opts.open(&path)?;
        let opened = f.metadata()?;
        self.check_file(&opened)?;
        let after = fs::symlink_metadata(&path)?;
        self.check_file(&after)?;
        if !same(&opened, &after) || before.as_ref().is_some_and(|m| !same(m, &opened)) {
            return Err(invalid("file identity changed"));
        }
        #[cfg(windows)]
        {
            // Only normalize ownership of the new empty inode created here.
            // Existing state must already satisfy the strict owner/DACL checks.
            windows_match(&f, &path, true)?;
            if before.is_none() {
                crate::windows_permissions::own_new_file(&path)?;
            }
            crate::windows_permissions::validate(&path)?;
            windows_match(&f, &path, true)?;
        }
        self.revalidate()?;
        if before.is_none() {
            f.sync_all()?;
            self.sync_dir()?;
        }
        Ok(f)
    }
    fn check_file(&self, m: &fs::Metadata) -> io::Result<()> {
        if !m.is_file() || !no_link(m) {
            return Err(invalid("regular unlinked file required"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if m.nlink() != 1 || m.mode() & 0o7777 != 0o600 || m.uid() != self.identity.uid() {
                return Err(invalid("unsafe file permissions or identity"));
            }
        }
        Ok(())
    }
    pub(crate) fn sync_dir(&self) -> io::Result<()> {
        self.revalidate()?;
        #[cfg(unix)]
        File::open(&self.path)?.sync_all()?;
        Ok(())
    }
    fn sync_parent(&self) -> io::Result<()> {
        #[cfg(unix)]
        File::open(
            self.path
                .parent()
                .ok_or_else(|| invalid("root has no parent"))?,
        )?
        .sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_hardlinked_and_linked_fixed_files() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("owned");
        let root = OwnedStateRoot::create(&path, &[tmp.path().join("provider")]).unwrap();
        let file = root.open_fixed(FixedFile::Journal).unwrap();
        drop(file);
        fs::hard_link(root.fixed_path(FixedFile::Journal), path.join("other-link")).unwrap();
        assert!(root.open_fixed(FixedFile::Journal).is_err());
    }
    #[test]
    fn sqlite_sidecars_are_checked_without_creating_them() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("owned");
        let root = OwnedStateRoot::create(&path, &[tmp.path().join("provider")]).unwrap();
        root.validate_sqlite_sidecars().unwrap();
        assert!(!root.fixed_path(FixedFile::SqliteRollbackJournal).exists());
        let f = root.open_fixed(FixedFile::SqliteRollbackJournal).unwrap();
        drop(f);
        root.validate_sqlite_sidecars().unwrap();
        fs::hard_link(
            root.fixed_path(FixedFile::SqliteRollbackJournal),
            path.join("linked"),
        )
        .unwrap();
        assert!(root.validate_sqlite_sidecars().is_err());
        fs::remove_file(path.join("linked")).unwrap();
        let f = root.open_fixed(FixedFile::SqliteWal).unwrap();
        drop(f);
        assert!(root.validate_sqlite_sidecars().is_err());
        fs::remove_file(root.fixed_path(FixedFile::SqliteWal)).unwrap();
        let f = root.open_fixed(FixedFile::SqliteShm).unwrap();
        drop(f);
        assert!(root.validate_sqlite_sidecars().is_err());
    }
    #[test]
    fn interrupted_recovery_stage_does_not_publish_partial_final() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("owned");
        let root = OwnedStateRoot::create(&path, &[tmp.path().join("provider")]).unwrap();
        let digest: RecoveryDigest =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .parse()
                .unwrap();
        let mut stage = root.create_recovery_stage().unwrap();
        stage.file.write_all(b"partial").unwrap();
        stage.file.sync_all().unwrap();
        let staged = stage.path.clone();
        drop(stage); // Simulated interruption before publication.
        assert_eq!(fs::read(staged).unwrap(), b"partial");
        assert!(!path.join(format!("recovery-{digest}.json")).exists());
    }
}
