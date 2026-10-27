//! Read-only ports and shared bounded observation helpers. No mutation port exists.
pub mod filesystem;
pub mod process;

use agentmender_domain::{
    DiscoveryScope, DiscoverySnapshot, Provider, ProviderKind, observed_path_is_absolute,
};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider not installed: {0}")]
    ProviderNotInstalled(ProviderKind),
    #[error("provider version unknown: {0}")]
    ProviderVersionUnknown(ProviderKind),
    #[error("unsupported provider: {0}")]
    ProviderUnsupported(ProviderKind),
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

pub trait ProviderAdapter {
    fn kind(&self) -> ProviderKind;
    fn detect(&self) -> Result<Provider, ProviderError>;
    fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError>;
}

#[derive(Debug, Clone)]
pub struct ProviderLocation {
    pub state_root: PathBuf,
    pub executable: Option<PathBuf>,
}

pub fn validate_scope(scope: &DiscoveryScope) -> Result<(), ProviderError> {
    if !(1..=100_000).contains(&scope.max_entries) {
        return Err(ProviderError::InvalidScope(
            "max_entries must be 1..=100000".into(),
        ));
    }
    if let Some(path) = &scope.project_path {
        if !observed_path_is_absolute(path)
            || path.as_os_str().as_encoded_bytes().len() > 4096
            || path
                .to_str()
                .is_none_or(|s| s.chars().any(char::is_control))
        {
            return Err(ProviderError::InvalidScope(
                "project path must be absolute UTF-8, without controls, at most 4096 bytes".into(),
            ));
        }
    }
    Ok(())
}

pub fn validate_location(location: &ProviderLocation) -> Result<(), ProviderError> {
    if !location.state_root.is_absolute()
        || location
            .executable
            .as_ref()
            .is_some_and(|p| !p.is_absolute())
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_and_oversized_project_scopes_are_rejected() {
        for project in [
            PathBuf::from("relative"),
            PathBuf::from(format!("/{}", "x".repeat(4096))),
        ] {
            assert!(
                validate_scope(&DiscoveryScope {
                    project_path: Some(project),
                    ..Default::default()
                })
                .is_err()
            );
        }
    }
    #[test]
    fn relative_live_provider_locations_are_rejected() {
        assert!(
            validate_location(&ProviderLocation {
                state_root: "relative".into(),
                executable: None
            })
            .is_err()
        );
    }
}
