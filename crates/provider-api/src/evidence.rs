//! An opaque capability for one fresh disposable evidence directory (ADR-0003).
//! No constructor accepts a destination directory or an existing writable home.
use crate::{ProviderError, io_error};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[derive(Debug)]
pub struct EvidenceSandbox {
    directory: TempDir,
    probe: TempDir,
    source_root: PathBuf,
    real_home: PathBuf,
    canonical_directory: PathBuf,
    probe_path: PathBuf,
}
impl EvidenceSandbox {
    pub fn new(source_root: &Path, real_home: &Path) -> Result<Self, ProviderError> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (source_root, real_home);
            Err(ProviderError::CapabilityUnavailable(
                "disposable evidence isolation is only verified on macOS".into(),
            ))
        }
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            if !source_root.is_absolute() || !real_home.is_absolute() {
                return Err(ProviderError::InvalidScope(
                    "evidence source and real home must be absolute".into(),
                ));
            }
            let metadata =
                std::fs::symlink_metadata(source_root).map_err(|e| io_error(source_root, e))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ProviderError::InvalidScope(
                    "evidence source must be a real directory".into(),
                ));
            }
            let source_root = source_root
                .canonicalize()
                .map_err(|e| io_error(source_root, e))?;
            let real_home = real_home
                .canonicalize()
                .map_err(|e| io_error(real_home, e))?;
            // A fixed system temp base avoids TMPDIR redirecting writes into provider state.
            let base = Path::new("/private/tmp")
                .canonicalize()
                .map_err(|e| io_error("/private/tmp", e))?;
            let protected = [
                source_root.clone(),
                real_home.join(".codex"),
                real_home.join(".claude"),
                real_home.join("Library/Application Support/Claude"),
            ];
            for path in &protected {
                let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
                if base.starts_with(&resolved) {
                    return Err(ProviderError::InvalidScope(
                        "system temporary base overlaps protected provider state".into(),
                    ));
                }
            }
            let directory = tempfile::Builder::new()
                .prefix("agentmender-evidence-")
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir_in(&base)
                .map_err(|e| io_error(&base, e))?;
            let probe = tempfile::Builder::new()
                .prefix("agentmender-denial-")
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir_in(&base)
                .map_err(|e| io_error(&base, e))?;
            let canonical_directory = directory
                .path()
                .canonicalize()
                .map_err(|e| io_error(directory.path(), e))?;
            for path in &protected {
                let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
                if canonical_directory.starts_with(&resolved)
                    || resolved.starts_with(&canonical_directory)
                {
                    return Err(ProviderError::InvalidScope(
                        "disposable evidence overlaps protected state".into(),
                    ));
                }
            }
            let probe_path = probe.path().join("must-remain-absent");
            Ok(Self {
                directory,
                probe,
                source_root,
                real_home,
                canonical_directory,
                probe_path,
            })
        }
    }
    pub fn path(&self) -> &Path {
        &self.canonical_directory
    }
    pub fn source_root(&self) -> &Path {
        &self.source_root
    }
    pub fn real_home(&self) -> &Path {
        &self.real_home
    }
    pub fn probe_path(&self) -> &Path {
        &self.probe_path
    }
    pub fn validate(&self) -> Result<(), ProviderError> {
        let metadata = std::fs::symlink_metadata(self.directory.path())
            .map_err(|e| io_error(self.directory.path(), e))?;
        let canonical = self
            .directory
            .path()
            .canonicalize()
            .map_err(|e| io_error(self.directory.path(), e))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || canonical != self.canonical_directory
            || !self.probe.path().is_dir()
            || self.probe_path.exists()
        {
            return Err(ProviderError::InvalidScope(
                "disposable evidence identity or denial probe changed".into(),
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o777 != 0o700 {
                return Err(ProviderError::InvalidScope(
                    "disposable evidence directory must be private".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn directory_is_fresh_private_disjoint_and_removed_on_drop() {
        let source = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let sandbox = EvidenceSandbox::new(source.path(), home.path()).unwrap();
        let path = sandbox.path().to_path_buf();
        assert!(sandbox.validate().is_ok());
        assert!(!path.starts_with(source.path()));
        drop(sandbox);
        assert!(!path.exists());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn protected_system_temp_base_is_rejected_before_creation() {
        let home = tempfile::tempdir().unwrap();
        assert!(EvidenceSandbox::new(Path::new("/private/tmp"), home.path()).is_err());
    }
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unsupported_platform_never_allocates_writable_evidence() {
        assert!(EvidenceSandbox::new(Path::new("/unused"), Path::new("/unused")).is_err());
    }
}
