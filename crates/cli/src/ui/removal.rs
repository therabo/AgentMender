//! Browser confirmation receipts and operation status, scoped to one launch.
use super::{ApiError, token};
use agentmender_domain::{Session, SessionState};
use agentmender_provider_codex::{RemovalOutcome, RemovalRefusal, RemovalReview};
use serde::Serialize;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub(super) fn refusal_error(error: RemovalRefusal) -> ApiError {
    ApiError::conflict(match error {
        RemovalRefusal::Unsupported => UNSUPPORTED,
        RemovalRefusal::Protected => PROTECTED,
        RemovalRefusal::SourceUnavailable => AFFECTED_UNKNOWN,
        RemovalRefusal::Changed => {
            "La sessione è cambiata. Aggiorna l’inventario e richiedi una nuova anteprima."
        }
    })
}

const PREVIEW_TTL: Duration = Duration::from_secs(120);
const CONFIRMATION: &str = "RIMUOVI";

pub(super) const UNSUPPORTED: &str =
    "Il binario Codex selezionato non corrisponde al profilo di rimozione verificato.";
pub(super) const AFFECTED_UNKNOWN: &str = "Verifica di archivio, transcript o subagent non riuscita. Codex potrebbe rimuovere anche i subagent collegati.";
pub(super) const PROTECTED: &str = "La sessione è protetta dalla configurazione di avvio.";
pub(super) const NOT_ARCHIVED: &str = "La rimozione è disponibile solo per le sessioni archiviate.";

pub(super) fn refusal(
    session: &Session,
    supported: bool,
    protected: &[String],
) -> Option<&'static str> {
    if protected.contains(&session.identity.native_id) {
        Some(PROTECTED)
    } else if !supported {
        Some(UNSUPPORTED)
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
    evidence: RemovalReview,
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

impl From<RemovalOutcome> for OperationStatus {
    fn from(value: RemovalOutcome) -> Self {
        match value {
            RemovalOutcome::Changed => Self::Changed,
            RemovalOutcome::CommandSucceeded => Self::CommandSucceeded,
            RemovalOutcome::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}
