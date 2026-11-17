//! Browser confirmation receipts and operation status, scoped to one launch.
use super::{ApiError, token};
use agentmender_domain::{Session, SessionState};
use agentmender_provider_codex::{
    NativeRemoval, NativeUnavailable, RemovalOutcome, RemovalRefusal, RemovalReview,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub(super) fn refusal_error(error: RemovalRefusal) -> ApiError {
    ApiError::conflict(match error {
        RemovalRefusal::NotMain => "Seleziona una conversazione principale verificata.",
        RemovalRefusal::Protected => PROTECTED,
        RemovalRefusal::UnsupportedStorage => {
            "Il formato dell’archivio non rientra nel contratto di rimozione Codex verificato."
        }
        RemovalRefusal::IncompleteRelationships => {
            "Copertura delle relazioni o dei transcript incompleta. Impossibile verificare l’intero albero."
        }
        RemovalRefusal::ExternalFork => {
            "Un fork esterno o la sua cronologia fa riferimento all’albero. La rimozione è bloccata."
        }
        RemovalRefusal::AttachedThread => {
            "Una sessione agganciata fa riferimento all’albero. La rimozione è bloccata."
        }
        RemovalRefusal::UnsafeMember => {
            "L’albero include una sessione attiva, una sessione in uso o un transcript in un formato o percorso di rimozione non supportato."
        }
        RemovalRefusal::Changed => CHANGED,
    })
}

const PREVIEW_TTL: Duration = Duration::from_secs(120);
const CONFIRMATION: &str = "RIMUOVI";

const CHANGED: &str =
    "L’archivio o l’eseguibile è cambiato. Aggiorna l’inventario e richiedi una nuova anteprima.";
pub(super) const PROTECTED: &str =
    "Una sessione nell’albero è protetta dalla configurazione di avvio.";
const NOT_ARCHIVED: &str = "La rimozione è disponibile solo per le sessioni archiviate.";

pub(super) fn native_reason(
    native: &Result<NativeRemoval, NativeUnavailable>,
) -> Option<&'static str> {
    match native {
        Ok(native) if native.is_current() => None,
        Ok(_) => Some(CHANGED),
        Err(reason) => Some(match reason {
            NativeUnavailable::Platform => {
                "La rimozione Codex è verificata solo su macOS con Apple Silicon."
            }
            NativeUnavailable::Executable => {
                "Eseguibile Codex regolare e avviabile non disponibile."
            }
            NativeUnavailable::Root => "Directory dell’archivio Codex non verificabile all’avvio.",
            NativeUnavailable::VersionProbe => {
                "Impossibile verificare la versione di Codex nel processo confinato."
            }
            NativeUnavailable::VersionContract => {
                "La versione di Codex non ha un contratto di rimozione a cascata verificato."
            }
            NativeUnavailable::CommandProbe => {
                "Il comando nativo Codex per la rimozione tramite UUID non è disponibile."
            }
        }),
    }
}

pub(super) fn refusal(session: &Session, protected: &[String]) -> Option<&'static str> {
    if protected.contains(&session.identity.native_id) {
        Some(PROTECTED)
    } else if session.state != SessionState::Archived {
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
    Verified,
    NativeUncertain,
    VerificationIncomplete,
    Residual,
}

#[derive(Clone, Serialize)]
pub(super) struct OperationView {
    pub status: OperationStatus,
    pub native_id: String,
    pub message: String,
}

impl OperationView {
    fn new(status: OperationStatus, native_id: String) -> Self {
        let message = match status {
            OperationStatus::Running => "Codex sta eseguendo il comando nativo.",
            OperationStatus::Changed => "L’archivio o l’eseguibile è cambiato dopo l’anteprima. Il comando non è stato avviato.",
            OperationStatus::Verified => "Verificata la rimozione delle sessioni, dei transcript attesi e dei relativi legami di origine.",
            OperationStatus::NativeUncertain => "Il comando nativo non ha confermato l’esito. Sono possibili effetti parziali.",
            OperationStatus::VerificationIncomplete => "Il comando nativo è riuscito, ma non è stato possibile verificare l’archivio dopo l’esecuzione.",
            OperationStatus::Residual => "La verifica ha rilevato modifiche inattese o elementi ancora presenti.",
        }.into();
        Self {
            status,
            native_id,
            message,
        }
    }
}

struct Preview {
    evidence: RemovalReview,
    expires: Instant,
}

#[derive(Serialize)]
pub(super) struct PreviewView {
    pub id: String,
    pub confirmation: &'static str,
    pub valid_seconds: u64,
    pub main_id: String,
    pub main_title: Option<String>,
    pub spawned_descendants: usize,
    pub total_sessions: usize,
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

    pub(super) fn issue(&mut self, evidence: RemovalReview) -> Result<PreviewView, ApiError> {
        self.prune();
        let id = evidence.native_id();
        if self.running || self.completed.len() >= 512 {
            return Err(ApiError::conflict(
                "Un’operazione è in corso o è stato raggiunto il limite della sessione locale.",
            ));
        }
        if self
            .completed
            .values()
            .any(|item| item.native_id == id && item.status != OperationStatus::Changed)
        {
            return Err(ApiError::conflict(
                "Esiste già un esito per questo UUID nella sessione locale.",
            ));
        }
        self.previews
            .retain(|_, previous| previous.evidence.native_id() != id);
        if self.previews.len() >= 32 {
            return Err(ApiError::conflict(
                "Troppe anteprime in attesa. Attendi la loro scadenza.",
            ));
        }
        let key = token()?;
        let view = PreviewView {
            id: key.clone(),
            confirmation: CONFIRMATION,
            valid_seconds: PREVIEW_TTL.as_secs(),
            main_id: id.into(),
            main_title: evidence.session().title.clone(),
            spawned_descendants: evidence.total_sessions() - 1,
            total_sessions: evidence.total_sessions(),
        };
        self.previews.insert(
            key.clone(),
            Preview {
                evidence,
                expires: Instant::now() + PREVIEW_TTL,
            },
        );
        Ok(view)
    }

    pub(super) fn start(
        &mut self,
        key: &str,
        confirmation: &str,
    ) -> Result<RemovalReview, ApiError> {
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
            OperationView::new(OperationStatus::Running, review.native_id().into()),
        );
        Ok(review)
    }

    pub(super) fn finish(&mut self, key: &str, outcome: RemovalOutcome) {
        if let Some(operation) = self.completed.get_mut(key) {
            let status = match &outcome {
                RemovalOutcome::Changed => OperationStatus::Changed,
                RemovalOutcome::Verified => OperationStatus::Verified,
                RemovalOutcome::NativeUncertain(_) => OperationStatus::NativeUncertain,
                RemovalOutcome::VerificationIncomplete => OperationStatus::VerificationIncomplete,
                RemovalOutcome::Residual(_) => OperationStatus::Residual,
            };
            *operation = OperationView::new(status, std::mem::take(&mut operation.native_id));
            if let RemovalOutcome::Residual(residual)
            | RemovalOutcome::NativeUncertain(Some(residual)) = outcome
            {
                operation.message.push_str(&format!(
                    " Righe di sessione ancora presenti: {}. Transcript: {}. Legami di origine: {}.",
                    residual.canonical_rows, residual.rollouts, residual.spawn_edges,
                ));
                if residual.unrelated_changed {
                    operation.message.push_str(
                        " Sono cambiate anche le evidenze di sessioni esterne all’albero.",
                    );
                }
            }
        }
        self.running = false;
    }

    pub(super) fn get(&self, key: &str) -> Option<OperationView> {
        self.completed.get(key).cloned()
    }
}
