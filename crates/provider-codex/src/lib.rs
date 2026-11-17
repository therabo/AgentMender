//! Codex inventory, bounded transcript previews and exact native removal.
#[cfg(target_os = "macos")]
mod desktop_reconciliation;
#[cfg(any(target_os = "macos", all(test, unix)))]
mod reader;
mod removal;
#[cfg(any(target_os = "macos", all(test, unix)))]
mod rollout_reader;
#[cfg(target_os = "macos")]
mod snapshot;
mod worker;

use agentmender_domain::{Installation, ProviderKind};
use agentmender_provider_api::{ProviderError, ProviderLocation, process, validate_location};
use std::{path::PathBuf, process::Child, sync::Mutex};

pub use removal::{
    NativeRemoval, NativeUnavailable, RemovalOutcome, RemovalRefusal, RemovalReview,
};
pub use worker::{
    ResidualEvidence, TranscriptCursor, TranscriptMessage, TranscriptPage, run_worker,
};

pub struct CodexAdapter {
    location: ProviderLocation,
    helper: PathBuf,
    home: PathBuf,
    connection: Mutex<Option<worker::Connection>>,
}

impl CodexAdapter {
    pub fn new(location: ProviderLocation, helper: PathBuf, home: PathBuf) -> Self {
        Self {
            location,
            helper,
            home,
            connection: Mutex::new(None),
        }
    }

    pub fn detect(&self) -> Result<Installation, ProviderError> {
        validate_location(&self.location)?;
        let executable = self
            .location
            .executable
            .as_deref()
            .filter(|path| path.is_file());
        Ok(Installation {
            detected: executable.is_some(),
            version: executable
                .and_then(|path| process::probe_version(ProviderKind::Codex, path).ok()),
        })
    }
}

struct Guard(Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
