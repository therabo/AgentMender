//! Codex inventory, bounded transcript previews and exact native removal.
pub mod discovery;
pub mod exports;
#[cfg(any(target_os = "macos", all(test, unix)))]
mod reader;
#[cfg(any(target_os = "macos", all(test, unix)))]
mod rollout_reader;
#[cfg(target_os = "macos")]
mod snapshot;
mod worker;

use agentmender_domain::{Installation, ProviderKind};
use agentmender_provider_api::{ProviderError, ProviderLocation, process, validate_location};
use std::{path::PathBuf, process::Child, sync::Mutex};

pub use worker::{TranscriptCursor, TranscriptMessage, TranscriptPage, run_worker};

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

    pub fn delete_session(
        &self,
        identity: &agentmender_domain::SessionIdentity,
    ) -> Result<String, String> {
        let root = self
            .location
            .state_root
            .canonicalize()
            .map_err(|error| format!("Archivio Codex non disponibile: {error}"))?;
        if identity.provider != ProviderKind::Codex || identity.state_root != root {
            return Err("Archivio Codex estraneo alla sessione selezionata.".into());
        }
        let id =
            uuid::Uuid::parse_str(&identity.native_id).map_err(|_| "UUID Codex non valido.")?;
        let executable = self
            .location
            .executable
            .as_deref()
            .ok_or("Eseguibile Codex non disponibile.")?;
        let path = root
            .to_str()
            .ok_or("Percorso dell’archivio Codex non rappresentabile.")?;
        let setting = format!("sqlite_home={}", toml::Value::String(path.to_owned()));
        process::codex_delete(executable, &root, &setting, id)
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
