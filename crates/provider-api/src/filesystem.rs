//! Bounded read observations, never provider writes. Not an atomic snapshot.
use crate::{ProviderError, io_error};
use agentmender_domain::{Diagnostic, DiagnosticCode};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

/// Replacement is supported only for plain files whose metadata can be kept
/// exactly by the reviewed same-directory publication operation. Querying a
/// descriptor neither edits permissions nor follows a replacement path.
pub fn plain_replacement_provenance(
    file: &std::fs::File,
) -> Result<Option<Vec<u8>>, ProviderError> {
    let unavailable =
        || ProviderError::CapabilityUnavailable("replacement metadata is unsupported".into());
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "hurd"))]
    {
        use std::os::unix::fs::MetadataExt;
        let m = file.metadata().map_err(|_| unavailable())?;
        if !m.is_file() || m.nlink() != 1 || m.mode() & 0o7022 != 0 {
            return Err(unavailable());
        }
        let mut names = [0; 128];
        let count = rustix::fs::flistxattr(file, &mut names[..]).map_err(|_| unavailable())?;
        #[cfg(target_os = "macos")]
        {
            use std::os::macos::fs::MetadataExt as _;
            if m.st_flags() != 0 {
                return Err(unavailable());
            }
            crate::macos_sandbox::verify_no_descriptor_acl(file)?;
            if &names[..count] == b"com.apple.provenance\0" {
                let mut value = [0; 64];
                let count = rustix::fs::fgetxattr(file, "com.apple.provenance", &mut value[..])
                    .map_err(|_| unavailable())?;
                if count == 0 {
                    return Err(unavailable());
                }
                return Ok(Some(value[..count].to_vec()));
            }
        }
        if count == 0 {
            Ok(None)
        } else {
            Err(unavailable())
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "hurd")))]
    {
        let _ = file;
        Err(unavailable())
    }
}

#[derive(Debug, Clone)]
pub struct ObservedFile {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub size_bytes: u64,
}
#[derive(Debug, Default)]
pub struct Inventory {
    pub files: Vec<ObservedFile>,
    pub diagnostics: Vec<Diagnostic>,
    pub truncated: bool,
}

fn diagnostic(code: DiagnosticCode, path: &Path, message: &str) -> Diagnostic {
    Diagnostic {
        code,
        message: message.into(),
        path: Some(path.into()),
    }
}

/// Enumerate regular-file metadata without following links. Unknown entries stay visible.
/// The entry budget includes directories and links, preventing an empty-directory DoS.
pub fn inventory(root: &Path, max_entries: usize) -> Result<Inventory, ProviderError> {
    let mut result = Inventory::default();
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(e) => return Err(io_error(root, e)),
    };
    if metadata.file_type().is_symlink() {
        result.diagnostics.push(diagnostic(
            DiagnosticCode::LinkSkipped,
            root,
            "provider inventory root is a link; skipped",
        ));
        return Ok(result);
    }
    if !metadata.is_dir() {
        return Err(ProviderError::MalformedProviderState {
            path: root.into(),
            detail: "inventory root is not a directory".into(),
        });
    }
    let canonical_root = root.canonicalize().map_err(|e| io_error(root, e))?;
    let mut pending = vec![root.to_path_buf()];
    let mut remaining = max_entries;
    while let Some(directory) = pending.pop() {
        // Revalidate before descending. Concurrent path replacement is not snapshot isolation.
        let directory_meta = match fs::symlink_metadata(&directory) {
            Ok(meta) => meta,
            Err(_) => {
                result.diagnostics.push(diagnostic(
                    DiagnosticCode::ConcurrentChange,
                    &directory,
                    "directory changed during observation",
                ));
                continue;
            }
        };
        if directory_meta.file_type().is_symlink() || !directory_meta.is_dir() {
            result.diagnostics.push(diagnostic(
                DiagnosticCode::LinkSkipped,
                &directory,
                "changed or linked directory skipped",
            ));
            continue;
        }
        match directory.canonicalize() {
            Ok(path) if path.starts_with(&canonical_root) => {}
            _ => {
                result.diagnostics.push(diagnostic(
                    DiagnosticCode::LinkSkipped,
                    &directory,
                    "directory escaped observation root",
                ));
                continue;
            }
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(e) => {
                let code = if e.kind() == std::io::ErrorKind::PermissionDenied {
                    DiagnosticCode::PermissionDenied
                } else {
                    DiagnosticCode::IncompleteDiscovery
                };
                result.diagnostics.push(diagnostic(
                    code,
                    &directory,
                    "directory could not be enumerated",
                ));
                continue;
            }
        };
        // Bound collection before sorting: enormous directories cannot exhaust memory.
        let mut paths = Vec::new();
        for entry in entries {
            if remaining == 0 {
                result.truncated = true;
                break;
            }
            remaining -= 1;
            match entry {
                Ok(entry) => paths.push(entry.path()),
                Err(_) => result.diagnostics.push(diagnostic(
                    DiagnosticCode::IncompleteDiscovery,
                    &directory,
                    "directory entry unavailable",
                )),
            }
        }
        paths.sort();
        for path in paths {
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => {
                    result.diagnostics.push(diagnostic(
                        DiagnosticCode::ConcurrentChange,
                        &path,
                        "entry changed during observation",
                    ));
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                result.diagnostics.push(diagnostic(
                    DiagnosticCode::LinkSkipped,
                    &path,
                    "symbolic link skipped",
                ));
            } else if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                result.files.push(ObservedFile {
                    relative_path: path
                        .strip_prefix(root)
                        .map_err(|_| ProviderError::InvalidScope("path escaped root".into()))?
                        .into(),
                    path,
                    size_bytes: metadata.len(),
                });
            } else {
                result.diagnostics.push(diagnostic(
                    DiagnosticCode::UnknownArtifact,
                    &path,
                    "special filesystem entry skipped",
                ));
            }
        }
        if result.truncated {
            break;
        }
    }
    if result.truncated {
        result.diagnostics.push(diagnostic(
            DiagnosticCode::LimitReached,
            root,
            "entry budget reached; inventory is partial",
        ));
    }
    result
        .files
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(result)
}

/// Read a regular file only after checking every relative component and root scope.
/// No transcript body is logged. Concurrent writers can still invalidate observations;
/// no future ownership or mutation decision may rely on these path checks alone.
fn open_regular(root: &Path, path: &Path) -> Result<fs::File, ProviderError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ProviderError::InvalidScope("read outside observation root".into()))?;
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(ProviderError::InvalidScope(
            "non-normal path component".into(),
        ));
    }
    let mut current = root.to_path_buf();
    for component in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(component) = component {
            current.push(component);
        }
        let meta = fs::symlink_metadata(&current).map_err(|e| io_error(&current, e))?;
        if meta.file_type().is_symlink() {
            return Err(ProviderError::InvalidScope(
                "linked read path rejected".into(),
            ));
        }
    }
    let canonical_root = root.canonicalize().map_err(|e| io_error(root, e))?;
    let canonical_path = path.canonicalize().map_err(|e| io_error(path, e))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(ProviderError::InvalidScope(
            "read escaped observation root".into(),
        ));
    }
    if !fs::symlink_metadata(path)
        .map_err(|e| io_error(path, e))?
        .is_file()
    {
        return Err(ProviderError::InvalidScope(
            "nonregular read rejected".into(),
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT: open the final object rather than its target.
        options.custom_flags(0x00200000);
    }
    let file = options.open(path).map_err(|e| io_error(path, e))?;
    let before = file.metadata().map_err(|e| io_error(path, e))?;
    if !before.is_file() {
        return Err(ProviderError::InvalidScope(
            "nonregular read rejected".into(),
        ));
    }
    Ok(file)
}

pub fn read_bounded(root: &Path, path: &Path, max_bytes: usize) -> Result<Vec<u8>, ProviderError> {
    let file = open_regular(root, path)?;
    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error(path, e))?;
    if bytes.len() > max_bytes {
        return Err(ProviderError::MalformedProviderState {
            path: path.into(),
            detail: "read byte budget exceeded".into(),
        });
    }
    Ok(bytes)
}

/// Descriptor-relative no-follow source read, used only inside attested workers.
/// Returning a reader grants no write or arbitrary command capability.
pub fn open_source_read(root: &Path, path: &Path) -> Result<fs::File, ProviderError> {
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags, open, openat};
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ProviderError::InvalidScope("source read outside root".into()))?;
        let components: Vec<_> = relative.components().collect();
        if components.is_empty()
            || components
                .iter()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(ProviderError::InvalidScope(
                "invalid source components".into(),
            ));
        }
        let mut directory = open(
            root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| ProviderError::CapabilityUnavailable("source directory unavailable".into()))?;
        for component in &components[..components.len() - 1] {
            directory = openat(
                &directory,
                component.as_os_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| {
                ProviderError::CapabilityUnavailable("source ancestor unavailable".into())
            })?;
        }
        let fd = openat(
            &directory,
            components.last().expect("nonempty").as_os_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| {
            ProviderError::CapabilityUnavailable(
                if e == rustix::io::Errno::NOENT {
                    "source file missing"
                } else if e == rustix::io::Errno::LOOP {
                    "source file linked"
                } else {
                    "source file unavailable"
                }
                .into(),
            )
        })?;
        let file = fs::File::from(fd);
        let metadata = file.metadata().map_err(|e| io_error(path, e))?;
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_file() || metadata.nlink() != 1 {
            return Err(ProviderError::InvalidScope(
                "source must be an unshared regular file".into(),
            ));
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, path);
        Err(ProviderError::CapabilityUnavailable(
            "descriptor source reads unsupported".into(),
        ))
    }
}

/// Narrow independent-operator registration admission; no credential creation.
pub fn verify_operator_descriptor(file: &fs::File) -> Result<(), ProviderError> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let m = file
            .metadata()
            .map_err(|_| ProviderError::InvalidScope("operator metadata unavailable".into()))?;
        if rustix::process::geteuid().is_root() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(ProviderError::InvalidScope(
                "independent operator ownership required".into(),
            ));
        }
        crate::macos_sandbox::verify_no_descriptor_acl(file)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = file;
        Err(ProviderError::CapabilityUnavailable(
            "operator registration admission is macOS-only".into(),
        ))
    }
}

/// Read only a prefix of a large transcript; callers must discard an unfinished line.
pub fn read_prefix(root: &Path, path: &Path, max_bytes: usize) -> Result<Vec<u8>, ProviderError> {
    let file = open_regular(root, path)?;
    let mut bytes = Vec::new();
    file.take(max_bytes as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error(path, e))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_are_bounded_and_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data");
        fs::write(&path, "12345").unwrap();
        assert_eq!(read_bounded(dir.path(), &path, 5).unwrap(), b"12345");
        assert!(read_bounded(dir.path(), &path, 4).is_err());
        assert!(read_bounded(dir.path(), &dir.path().join("../escape"), 5).is_err());
    }
    #[test]
    fn inventory_budget_includes_directories() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            fs::create_dir(dir.path().join(i.to_string())).unwrap();
        }
        let found = inventory(dir.path(), 2).unwrap();
        assert!(found.truncated);
        assert!(
            found
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::LimitReached)
        );
    }
    #[cfg(unix)]
    #[test]
    fn linked_file_and_directory_are_never_followed() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let secret = other.path().join("secret");
        fs::write(&secret, "private").unwrap();
        symlink(&secret, dir.path().join("file")).unwrap();
        symlink(other.path(), dir.path().join("directory")).unwrap();
        let found = inventory(dir.path(), 100).unwrap();
        assert!(found.files.is_empty());
        assert_eq!(found.diagnostics.len(), 2);
        assert!(read_bounded(dir.path(), &dir.path().join("file"), 100).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn fifo_is_rejected_without_waiting_for_a_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pipe");
        assert!(
            std::process::Command::new("/usr/bin/mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        assert!(read_bounded(dir.path(), &path, 100).is_err());
        assert!(read_prefix(dir.path(), &path, 100).is_err());
    }
}

#[cfg(test)]
mod replacement_metadata_tests {
    #[cfg(target_os = "macos")]
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn descriptor_admission_accepts_plain_file_and_rejects_acl() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("plain");
        fs::write(&path, b"untouched").unwrap();
        let file = fs::File::open(&path).unwrap();
        plain_replacement_provenance(&file).unwrap();
        assert!(
            std::process::Command::new("/bin/chmod")
                .args(["+a", "everyone allow read"])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        assert!(plain_replacement_provenance(&file).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"untouched");
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn descriptor_admission_rejects_xattr_without_touching_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("plain");
        fs::write(&path, b"untouched").unwrap();
        let file = fs::File::open(&path).unwrap();
        plain_replacement_provenance(&file).unwrap();
        assert!(
            std::process::Command::new("/usr/bin/xattr")
                .args(["-w", "com.agentmender.fixture", "fixture"])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        assert!(plain_replacement_provenance(&file).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"untouched");
    }
}
