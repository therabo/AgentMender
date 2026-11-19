//! Bounded read observations, never provider writes. Not an atomic snapshot.
use crate::{ProviderError, io_error};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

/// Detect Windows junctions and other reparse points as well as Unix symlinks.
pub fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
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
        if is_link(&meta) {
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
    if !before.is_file() || is_link(&before) {
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

#[cfg(windows)]
pub fn windows_file_identity(file: &fs::File) -> std::io::Result<(u64, u64)> {
    let info = winapi_util::file::information(winapi_util::HandleRef::from_file(file))?;
    if info.number_of_links() != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "shared file identity",
        ));
    }
    Ok((info.volume_serial_number(), info.file_index()))
}

#[cfg(windows)]
pub fn windows_directory_identity(path: &Path) -> std::io::Result<(u64, u64)> {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = fs::OpenOptions::new()
        .read(true)
        // FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT.
        .custom_flags(0x02200000)
        .open(path)?;
    let meta = dir.metadata()?;
    if !meta.is_dir() || is_link(&meta) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "linked directory identity",
        ));
    }
    let info = winapi_util::file::information(winapi_util::HandleRef::from_file(&dir))?;
    Ok((info.volume_serial_number(), info.file_index()))
}

/// Read-only provider observation. Unix uses descriptor-relative component
/// traversal. Windows checks all ancestors and opens the leaf without following
/// a reparse point. Callers must revalidate the observation before reuse.
pub fn open_observed_read(root: &Path, path: &Path) -> Result<fs::File, ProviderError> {
    if !path.starts_with(root) || !path.is_absolute() {
        return Err(ProviderError::InvalidScope(
            "read outside observation root".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let file = open_source_read(Path::new("/"), path)?;
        let root_meta = fs::symlink_metadata(root).map_err(|e| io_error(root, e))?;
        let file_meta = file.metadata().map_err(|e| io_error(path, e))?;
        if !root_meta.is_dir() || root_meta.dev() != file_meta.dev() {
            return Err(ProviderError::InvalidScope(
                "source outside the archive volume".into(),
            ));
        }
        Ok(file)
    }
    #[cfg(windows)]
    {
        for ancestor in root.ancestors() {
            let metadata = fs::symlink_metadata(ancestor).map_err(|e| io_error(ancestor, e))?;
            if is_link(&metadata) || !metadata.is_dir() {
                return Err(ProviderError::InvalidScope(
                    "linked observation root".into(),
                ));
            }
        }
        let identity = windows_directory_identity(root).map_err(|e| io_error(root, e))?;
        let file = open_regular(root, path)?;
        let (volume, _) = windows_file_identity(&file).map_err(|e| io_error(path, e))?;
        if volume != identity.0 || windows_directory_identity(root).ok() != Some(identity) {
            return Err(ProviderError::InvalidScope(
                "observation root or volume changed".into(),
            ));
        }
        Ok(file)
    }
    #[cfg(not(any(unix, windows)))]
    open_regular(root, path)
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
        .map_err(source_open_error)?;
        for component in &components[..components.len() - 1] {
            directory = openat(
                &directory,
                component.as_os_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(source_open_error)?;
        }
        let fd = openat(
            &directory,
            components.last().expect("nonempty").as_os_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(source_open_error)?;
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

/// Closed, path-free source diagnostics; never export the OS error's text.
#[cfg(unix)]
fn source_open_error(error: rustix::io::Errno) -> ProviderError {
    use rustix::io::Errno;
    ProviderError::CapabilityUnavailable(
        match error {
            Errno::NOENT => "source file missing",
            Errno::ACCESS | Errno::PERM => "source read permission denied",
            Errno::LOOP | Errno::NOTDIR => "source file linked or non-directory",
            _ => "source file unavailable",
        }
        .into(),
    )
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
