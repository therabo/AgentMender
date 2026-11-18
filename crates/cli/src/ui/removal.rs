//! Browser confirmation receipts and operation status, scoped to one launch.
use super::{ApiError, inventory::ProjectView, token};
use agentmender_domain::{ProviderKind, Session, SessionState};
use agentmender_provider_codex::{
    NativeRemoval, NativeUnavailable, ProjectOutcome, ProjectRemovalReview, ProjectStep,
    RemovalOutcome, RemovalRefusal, RemovalReview, Subtree,
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
        RemovalRefusal::AmbiguousNativeLookup => {
            "La ricerca nativa potrebbe selezionare un rollout estraneo. Rimozione bloccata prima del comando."
        }
        RemovalRefusal::AttachedThread => {
            "Una sessione agganciata fa riferimento all’albero. La rimozione è bloccata."
        }
        RemovalRefusal::UnsafeMember => {
            "L’albero include una sessione in uso, uno stato di spawn non verificabile o un percorso di rollout incompatibile."
        }
        RemovalRefusal::Changed => CHANGED,
    })
}

const PREVIEW_TTL: Duration = Duration::from_secs(120);
const CONFIRMATION: &str = "RIMUOVI";
const PROJECT_CONFIRMATION: &str = "RIMUOVI SESSIONI DEL PROGETTO";

const CHANGED: &str =
    "L’archivio o l’eseguibile è cambiato. Aggiorna l’inventario e richiedi una nuova anteprima.";
pub(super) const PROTECTED: &str =
    "Una sessione nell’albero è protetta dalla configurazione di avvio.";
const UNKNOWN_STATE: &str = "Lo stato di archiviazione della sessione non è verificato.";

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
            NativeUnavailable::OverlappingExports => {
                "Una root di export esterna si sovrappone all’archivio delle sessioni."
            }
            NativeUnavailable::SplitStorage => {
                "Il database risiede in una root separata. Il contratto nativo locale richiede la stessa root."
            }
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
    } else if session.state == SessionState::Unknown {
        Some(UNKNOWN_STATE)
    } else {
        None
    }
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum OperationStatus {
    Preparing,
    Ready,
    Running,
    Partial,
    Refused,
    Uncertain,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<ProjectProgress>,
}

impl OperationView {
    fn new(status: OperationStatus, native_id: String) -> Self {
        let message = match status {
            OperationStatus::Preparing => "Verifica delle sessioni associate al progetto.",
            OperationStatus::Ready => "Il piano è pronto per la conferma.",
            OperationStatus::Partial => "Rimozione interrotta. Solo i sottoalberi indicati come verificati risultano rimossi.",
            OperationStatus::Refused => "Le verifiche del progetto hanno impedito la rimozione.",
            OperationStatus::Uncertain => "La rimozione del sottoalbero in corso ha un esito incerto.",
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
            project: None,
        }
    }
}

#[derive(Clone, Serialize)]
struct ProjectProgress {
    key: String,
    name: String,
    removed: Vec<Subtree>,
    remaining: Vec<Subtree>,
    uncertain: Option<Subtree>,
}

pub(super) enum ReviewedRemoval {
    Session(Box<RemovalReview>),
    Project(Box<ProjectRemovalReview>),
}

struct Preview {
    evidence: ReviewedRemoval,
    project: Option<ProjectProgress>,
    expires: Instant,
}

impl Preview {
    fn confirmation(&self) -> &'static str {
        match self.evidence {
            ReviewedRemoval::Session(_) => CONFIRMATION,
            ReviewedRemoval::Project(_) => PROJECT_CONFIRMATION,
        }
    }
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

#[derive(Serialize)]
pub(super) struct ProjectPreviewView {
    pub id: String,
    pub confirmation: &'static str,
    pub valid_seconds: u64,
    pub status: OperationStatus,
    pub provider: ProviderKind,
    pub project_key: String,
    pub project_name: String,
    pub roots: Vec<String>,
    pub main_conversations: usize,
    pub descendants: usize,
    pub total_sessions: usize,
    pub subtrees: Vec<Subtree>,
    pub scope: &'static str,
    pub excluded: [&'static str; 4],
    pub complete: bool,
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

    fn reserve_preview(&self) -> Result<String, ApiError> {
        self.require_idle()?;
        if self.previews.len() >= 32 {
            return Err(ApiError::conflict(
                "Troppe anteprime in attesa. Attendi la loro scadenza.",
            ));
        }
        token()
    }

    fn require_idle(&self) -> Result<(), ApiError> {
        if self.running || self.completed.len() >= 512 {
            return Err(ApiError::conflict(
                "Un’operazione è in corso o è stato raggiunto il limite della sessione locale.",
            ));
        }
        Ok(())
    }

    fn require_new_target(&self, id: &str) -> Result<(), ApiError> {
        if self.completed.values().any(|item| {
            if let Some(project) = &item.project {
                project
                    .removed
                    .iter()
                    .chain(&project.uncertain)
                    .any(|tree| tree.main_id == id)
            } else {
                item.native_id == id && item.status != OperationStatus::Changed
            }
        }) {
            return Err(ApiError::conflict(
                "Esiste già un esito per questo UUID nella sessione locale.",
            ));
        }
        Ok(())
    }

    pub(super) fn issue(&mut self, evidence: RemovalReview) -> Result<PreviewView, ApiError> {
        self.prune();
        self.require_new_target(evidence.native_id())?;
        self.previews.retain(|_, previous| {
            !matches!(&previous.evidence, ReviewedRemoval::Session(old) if old.native_id() == evidence.native_id())
        });
        let id = self.reserve_preview()?;
        let view = PreviewView {
            id: id.clone(),
            confirmation: CONFIRMATION,
            valid_seconds: PREVIEW_TTL.as_secs(),
            main_id: evidence.native_id().into(),
            main_title: evidence.session().title.clone(),
            spawned_descendants: evidence.total_sessions() - 1,
            total_sessions: evidence.total_sessions(),
        };
        self.previews.insert(
            id,
            Preview {
                evidence: ReviewedRemoval::Session(Box::new(evidence)),
                project: None,
                expires: Instant::now() + PREVIEW_TTL,
            },
        );
        Ok(view)
    }

    pub(super) fn issue_project(
        &mut self,
        project: ProjectView,
        evidence: ProjectRemovalReview,
    ) -> Result<ProjectPreviewView, ApiError> {
        self.prune();
        let subtrees: Vec<_> = evidence.subtrees().collect();
        for tree in &subtrees {
            self.require_new_target(&tree.main_id)?;
        }
        self.previews.retain(|_, previous| {
            previous
                .project
                .as_ref()
                .is_none_or(|old| old.key != project.key)
        });
        let id = self.reserve_preview()?;
        let total = subtrees
            .iter()
            .map(|tree| tree.total_sessions)
            .sum::<usize>();
        let view = ProjectPreviewView {
            id: id.clone(),
            confirmation: PROJECT_CONFIRMATION,
            valid_seconds: PREVIEW_TTL.as_secs(),
            status: OperationStatus::Ready,
            provider: ProviderKind::Codex,
            project_key: project.key.clone(),
            project_name: project.name.clone(),
            roots: project.roots,
            main_conversations: subtrees.len(),
            descendants: total - subtrees.len(),
            total_sessions: total,
            subtrees: subtrees.clone(),
            scope: "sessions",
            excluded: [
                "Workspace, repository e worktree",
                "Configurazione e credenziali",
                "Archivi e risorse degli altri progetti",
                "Registrazione del progetto e preferenze Desktop",
            ],
            complete: true,
        };
        self.previews.insert(
            id,
            Preview {
                evidence: ReviewedRemoval::Project(Box::new(evidence)),
                project: Some(ProjectProgress {
                    key: project.key,
                    name: project.name,
                    removed: Vec::new(),
                    remaining: subtrees,
                    uncertain: None,
                }),
                expires: Instant::now() + PREVIEW_TTL,
            },
        );
        Ok(view)
    }

    pub(super) fn start(
        &mut self,
        key: &str,
        confirmation: &str,
    ) -> Result<ReviewedRemoval, ApiError> {
        self.prune();
        let preview = self
            .previews
            .get(key)
            .filter(|preview| confirmation == preview.confirmation())
            .ok_or_else(|| {
                ApiError::conflict("Conferma o anteprima non valida, scaduta o già usata.")
            })?;
        self.require_idle()?;
        let operation = match &preview.evidence {
            ReviewedRemoval::Session(review) => {
                self.require_new_target(review.native_id())?;
                OperationView::new(OperationStatus::Running, review.native_id().into())
            }
            ReviewedRemoval::Project(review) => {
                for tree in review.subtrees() {
                    self.require_new_target(&tree.main_id)?;
                }
                let mut view = OperationView::new(OperationStatus::Preparing, String::new());
                view.project = preview.project.clone();
                view
            }
        };
        let preview = self.previews.remove(key).expect("under exclusive lock");
        self.running = true;
        self.completed.insert(key.into(), operation);
        Ok(preview.evidence)
    }

    pub(super) fn project_step(&mut self, key: &str, step: ProjectStep) {
        let view = self.completed.get_mut(key).expect("running operation");
        let project = view.project.as_mut().expect("project operation");
        match step {
            ProjectStep::Dispatching(tree) => {
                project.remaining.remove(0);
                view.status = OperationStatus::Running;
                view.message = format!(
                    "Sottoalberi verificati: {}. Rimozione in corso.",
                    project.removed.len()
                );
                project.uncertain = Some(tree);
            }
            ProjectStep::Verified => {
                project.removed.extend(project.uncertain.take());
                view.status = OperationStatus::Preparing;
                view.message = format!(
                    "Sottoalberi verificati: {}. Verifica del piano in corso.",
                    project.removed.len()
                );
            }
        }
    }

    pub(super) fn finish_project(&mut self, key: &str, outcome: ProjectOutcome) {
        let view = self.completed.get_mut(key).expect("running operation");
        let mut project = view.project.take().expect("project operation");
        if outcome != ProjectOutcome::Uncertain
            && let Some(tree) = project.uncertain.take()
        {
            project.remaining.insert(0, tree);
        }
        let status = match outcome {
            ProjectOutcome::Verified => OperationStatus::Verified,
            _ if !project.removed.is_empty() => OperationStatus::Partial,
            ProjectOutcome::Changed => OperationStatus::Changed,
            ProjectOutcome::Refused(_) => OperationStatus::Refused,
            ProjectOutcome::Uncertain => OperationStatus::Uncertain,
        };
        *view = OperationView::new(status, String::new());
        if let ProjectOutcome::Refused(reason) = outcome {
            view.message.push(' ');
            view.message.push_str(refusal_error(reason).1);
        }
        view.project = Some(project);
        self.running = false;
    }

    pub(super) fn interrupted(&mut self, key: &str) {
        if self
            .completed
            .get(key)
            .is_some_and(|view| view.project.is_some())
        {
            self.finish_project(key, ProjectOutcome::Uncertain);
        } else {
            self.finish(key, RemovalOutcome::NativeUncertain(None));
        }
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
