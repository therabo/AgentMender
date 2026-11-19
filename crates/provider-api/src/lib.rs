//! Bounded filesystem observations and fixed provider process entry points.
pub mod evidence;
pub mod filesystem;
#[cfg(target_os = "macos")]
mod macos_sandbox;
pub mod process;
#[cfg(unix)]
pub mod rewrite;

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("capability unavailable: {0}")]
    CapabilityUnavailable(String),
    #[error("malformed provider state at {path}: {detail}")]
    MalformedProviderState { path: PathBuf, detail: String },
    #[error("permission denied at {0}")]
    PermissionDenied(PathBuf),
    #[error("discovery failed at {path}: {source}")]
    DiscoveryFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid discovery scope: {0}")]
    InvalidScope(String),
}

#[derive(Debug, Clone)]
pub struct ProviderLocation {
    pub state_root: PathBuf,
    pub executable: Option<PathBuf>,
}

pub fn validate_location(location: &ProviderLocation) -> Result<(), ProviderError> {
    if !location.state_root.is_absolute()
        || location
            .executable
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
    {
        return Err(ProviderError::InvalidScope(
            "provider state root and executable must be absolute".into(),
        ));
    }
    Ok(())
}

pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> ProviderError {
    let path = path.into();
    if source.kind() == std::io::ErrorKind::PermissionDenied {
        ProviderError::PermissionDenied(path)
    } else {
        ProviderError::DiscoveryFailed { path, source }
    }
}
