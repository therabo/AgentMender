use std::{
    collections::HashMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use agentmender_domain::{LocalSession, SessionState};
use agentmender_provider_codex::{CodexAdapter, LocalRemovalEvidence};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{ApiError, token};

// Exact Darwin arm64 executable exercised against the disposable archive.
const CODEX_SHA256: &str = "3ea20787e106af94573d3d6c19480aed93a1334b0dd32b6b04e2f780ce82cf4b";
const PREVIEW_TTL: Duration = Duration::from_secs(120);
const CONFIRMATION: &str = "RIMUOVI";

pub(super) const UNSUPPORTED: &str =
    "Il binario Codex selezionato non corrisponde al profilo di rimozione verificato.";
pub(super) const AFFECTED_UNKNOWN: &str = "Verifica di archivio, transcript o subagent non riuscita. Codex potrebbe rimuovere anche i subagent collegati.";
pub(super) const PROTECTED: &str = "La sessione è protetta dalla configurazione di avvio.";
pub(super) const NOT_ARCHIVED: &str = "La rimozione è disponibile solo per le sessioni archiviate.";

#[derive(Clone, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    revision: Option<(u64, i64, i64, i64, i64)>,
}

fn file_identity(path: &Path, directory: bool) -> Option<FileIdentity> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        use std::os::unix::fs::MetadataExt;

        let meta = fs::symlink_metadata(path).ok()?;
        if (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
            return None;
        }
        Some(FileIdentity {
            device: meta.dev(),
            inode: meta.ino(),
            revision: (!directory).then(|| {
                (
                    meta.len(),
                    meta.mtime(),
                    meta.mtime_nsec(),
                    meta.ctime(),
                    meta.ctime_nsec(),
                )
            }),
        })
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = (path, directory);
        None
    }
}

pub(super) struct NativeProfile {
    executable: PathBuf,
    executable_identity: FileIdentity,
    root: PathBuf,
    root_identity: FileIdentity,
}

struct DigestWriter(Sha256);

impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl NativeProfile {
    pub(super) fn detect(executable: Option<&Path>, root: &Path) -> Option<Self> {
        let root_identity = file_identity(root, true)?;
        let root = root.canonicalize().ok()?;
        let executable = executable?.canonicalize().ok()?;
        let executable_identity = file_identity(&executable, false)?;
        let mut stream = fs::File::open(&executable).ok()?.take(512 * 1024 * 1024);
        let mut digest = DigestWriter(Sha256::new());
        io::copy(&mut stream, &mut digest).ok()?;
        let hash: String = digest
            .0
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let profile = Self {
            executable,
            executable_identity,
            root,
            root_identity,
        };
        (hash == CODEX_SHA256 && profile.is_current()).then_some(profile)
    }

    fn is_current(&self) -> bool {
        file_identity(&self.executable, false).as_ref() == Some(&self.executable_identity)
            && file_identity(&self.root, true).as_ref() == Some(&self.root_identity)
    }

    pub(super) fn prepare(
        &self,
        adapter: &CodexAdapter,
        session: &LocalSession,
    ) -> Result<LocalRemovalEvidence, ApiError> {
        if !self.is_current() {
            return Err(ApiError::conflict(UNSUPPORTED));
        }
        let evidence = adapter
            .local_removal_evidence(&session.metadata.native_session_id)
            .map_err(|_| ApiError::conflict(AFFECTED_UNKNOWN))?;
        if &evidence.session != session
            || evidence.session.metadata.identity.state_root != self.root
        {
            return Err(ApiError::conflict(
                "La sessione è cambiata. Aggiorna l’inventario e richiedi una nuova anteprima.",
            ));
        }
        Ok(evidence)
    }

    pub(super) fn execute(
        &self,
        adapter: &CodexAdapter,
        review: &LocalRemovalEvidence,
    ) -> OperationStatus {
        let Ok(id) = uuid::Uuid::parse_str(&review.session.metadata.native_session_id) else {
            return OperationStatus::Changed;
        };
        // Fresh exact row, archive, spawn relationships and physical transcript
        // are checked by the confined reader immediately before native dispatch.
        if self.prepare(adapter, &review.session).as_ref().ok() != Some(review)
            || !self.is_current()
        {
            return OperationStatus::Changed;
        }
        if crate::native_delete::dispatch(&self.executable, &self.root, id, true) {
            OperationStatus::CommandSucceeded
        } else {
            OperationStatus::OutcomeUnknown
        }
    }
}

pub(super) fn refusal(
    session: &LocalSession,
    supported: bool,
    protected: &[String],
) -> Option<&'static str> {
    if protected.contains(&session.metadata.native_session_id) {
        Some(PROTECTED)
    } else if !supported {
        Some(UNSUPPORTED)
    } else if session.metadata.state != SessionState::Archived {
        Some(NOT_ARCHIVED)
    } else {
        None
    }
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum OperationStatus {
    Running,
    Changed,
    CommandSucceeded,
    OutcomeUnknown,
}

#[derive(Clone, Serialize)]
pub(super) struct OperationView {
    pub status: OperationStatus,
    pub native_id: String,
    pub message: &'static str,
}

impl OperationView {
    fn new(status: OperationStatus, native_id: String) -> Self {
        let message = match status {
            OperationStatus::Running => "Codex sta eseguendo il comando nativo.",
            OperationStatus::Changed => {
                "La verifica è cambiata dopo l’anteprima. Il comando non è stato avviato."
            }
            OperationStatus::CommandSucceeded => {
                "Il comando nativo ha restituito esito positivo. Residui, catalogo Desktop e spazio recuperato non sono verificati."
            }
            OperationStatus::OutcomeUnknown => {
                "L’esito del comando è incerto e possono esserci effetti parziali. AgentMender non riproverà."
            }
        };
        Self {
            status,
            native_id,
            message,
        }
    }
}

struct Preview {
    evidence: LocalRemovalEvidence,
    expires: Instant,
}

#[derive(Serialize)]
pub(super) struct PreviewView {
    pub id: String,
    pub confirmation: &'static str,
    pub valid_seconds: u64,
}

#[derive(Default)]
pub(super) struct Operations {
    previews: HashMap<String, Preview>,
    completed: HashMap<String, OperationView>,
    running: bool,
}

impl Operations {
    fn prune(&mut self) {
        self.previews
            .retain(|_, preview| preview.expires > Instant::now());
    }

    pub(super) fn issue(
        &mut self,
        evidence: LocalRemovalEvidence,
    ) -> Result<PreviewView, ApiError> {
        self.prune();
        let id = &evidence.session.metadata.native_session_id;
        if self.running || self.completed.len() >= 512 {
            return Err(ApiError::conflict(
                "Un’operazione è in corso o è stato raggiunto il limite della sessione locale.",
            ));
        }
        if self
            .completed
            .values()
            .any(|item| item.native_id == *id && item.status != OperationStatus::Changed)
        {
            return Err(ApiError::conflict(
                "Esiste già un esito per questo UUID nella sessione locale.",
            ));
        }
        self.previews
            .retain(|_, previous| previous.evidence.session.metadata.native_session_id != *id);
        if self.previews.len() >= 32 {
            return Err(ApiError::conflict(
                "Troppe anteprime in attesa. Attendi la loro scadenza.",
            ));
        }
        let key = token()?;
        self.previews.insert(
            key.clone(),
            Preview {
                evidence,
                expires: Instant::now() + PREVIEW_TTL,
            },
        );
        Ok(PreviewView {
            id: key,
            confirmation: CONFIRMATION,
            valid_seconds: PREVIEW_TTL.as_secs(),
        })
    }

    pub(super) fn start(
        &mut self,
        key: &str,
        confirmation: &str,
    ) -> Result<LocalRemovalEvidence, ApiError> {
        self.prune();
        if confirmation != CONFIRMATION || !self.previews.contains_key(key) {
            return Err(ApiError::conflict(
                "Conferma o anteprima non valida, scaduta o già usata.",
            ));
        }
        if self.running || self.completed.len() >= 512 {
            return Err(ApiError::conflict(
                "Un’operazione è in corso o è stato raggiunto il limite della sessione locale.",
            ));
        }
        let review = self
            .previews
            .remove(key)
            .expect("under exclusive lock")
            .evidence;
        self.running = true;
        self.completed.insert(
            key.into(),
            OperationView::new(
                OperationStatus::Running,
                review.session.metadata.native_session_id.clone(),
            ),
        );
        Ok(review)
    }

    pub(super) fn finish(&mut self, key: &str, status: OperationStatus) {
        if let Some(operation) = self.completed.get_mut(key) {
            *operation = OperationView::new(status, std::mem::take(&mut operation.native_id));
        }
        self.running = false;
    }

    pub(super) fn get(&self, key: &str) -> Option<OperationView> {
        self.completed.get(key).cloned()
    }
}
