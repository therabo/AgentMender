use std::{collections::HashMap, fs, sync::Mutex};

use agentmender_application::{
    local_inventory::{InventoryPage, InventoryQuery, InventorySort, LocalInventory},
    projects::{self, Project},
};
use agentmender_domain::*;
use agentmender_provider_api::ProviderError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{ApiError, App, removal};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Coverage {
    Partial,
    Unavailable,
    NotDetected,
}

#[derive(Clone, Serialize)]
pub(super) struct ProviderView {
    pub id: ProviderKind,
    pub presence: Presence,
    pub coverage: Coverage,
    pub inventory_incomplete: bool,
    pub coverage_description: &'static str,
    pub total: usize,
    pub registered_projects: usize,
    pub project_conversations: usize,
    pub recent_conversations: usize,
    pub projects_incomplete: bool,
    pub removal_available: bool,
    pub removal_reason: Option<&'static str>,
}

pub(super) const NO_CLAUDE_REMOVAL: &str =
    "Claude Code è in sola lettura. La rimozione per singola sessione non è disponibile.";

pub(super) struct Snapshot {
    info: ProviderView,
    version: Option<String>,
    conversation_preview: bool,
    index: LocalInventory,
    recents: LocalInventory,
    ids: HashMap<String, usize>,
    projects: HashMap<String, Project>,
    codex_project_keys: HashMap<String, String>,
}

impl Snapshot {
    fn load(app: &App, provider: ProviderKind) -> Self {
        let (root, detected) = match provider {
            ProviderKind::Codex => (&app.codex_root, app.codex.detect()),
            ProviderKind::Claude => (&app.claude_root, app.claude.detect()),
        };
        let detected = detected.unwrap_or_default();
        let transcript_preview = provider == ProviderKind::Codex && cfg!(target_os = "macos");
        let blocked_root = provider == ProviderKind::Codex
            && matches!(
                app.discovery.selected_access(),
                agentmender_provider_codex::discovery::Access::Linked
                    | agentmender_provider_codex::discovery::Access::NetworkPath
            );
        let root_check = if blocked_root {
            Err(std::io::ErrorKind::PermissionDenied.into())
        } else {
            fs::symlink_metadata(root)
        };
        let presence = if detected.detected {
            Presence::Installed
        } else if root_check.is_ok() {
            Presence::StateOnly
        } else {
            Presence::NotDetected
        };
        let (coverage, coverage_description, observation) = match root_check {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if presence == Presence::NotDetected {
                    (
                        Coverage::NotDetected,
                        "Installazione e archivio locale non rilevati.",
                        InventoryObservation::default(),
                    )
                } else {
                    (
                        Coverage::Unavailable,
                        "Il provider è installato, ma l'archivio configurato non è stato trovato.",
                        InventoryObservation::default(),
                    )
                }
            }
            Ok(info) if info.is_dir() => {
                let result = match provider {
                    ProviderKind::Codex => app.codex.local_inventory(app.limit),
                    ProviderKind::Claude => app.claude.local_inventory(app.limit),
                };
                match result {
                    Ok(observation) => {
                        let description = if observation.incomplete
                            || observation.projects_incomplete
                        {
                            "La lettura è incompleta. Altre sessioni o progetti potrebbero essere presenti."
                        } else if provider == ProviderKind::Codex {
                            "Dati del database Codex locale riconosciuto. Sono visibili solo le conversazioni principali verificate."
                        } else {
                            "Conversazioni principali riconosciute nei transcript di progetto. Titoli e date possono essere incompleti."
                        };
                        (Coverage::Partial, description, observation)
                    }
                    Err(ProviderError::PermissionDenied(_)) => (
                        Coverage::Unavailable,
                        "Accesso negato all’archivio del provider.",
                        InventoryObservation::default(),
                    ),
                    Err(ProviderError::InvalidScope(reason)) if reason == "stale_codex_source" => (
                        Coverage::Unavailable,
                        "L’archivio è cambiato durante la lettura. Aggiorna l’inventario.",
                        InventoryObservation::default(),
                    ),
                    Err(_) if provider == ProviderKind::Codex && !transcript_preview => (
                        Coverage::Unavailable,
                        "La lettura confinata dell’archivio Codex è attualmente disponibile su macOS.",
                        InventoryObservation::default(),
                    ),
                    Err(_) => (
                        Coverage::Unavailable,
                        "L’archivio non è leggibile o usa un formato non riconosciuto.",
                        InventoryObservation::default(),
                    ),
                }
            }
            _ => (
                Coverage::Unavailable,
                "La directory dell’archivio non è accessibile o è un collegamento.",
                InventoryObservation::default(),
            ),
        };
        let can_read = matches!(coverage, Coverage::Partial);
        let removal_reason = match provider {
            ProviderKind::Claude => Some(NO_CLAUDE_REMOVAL),
            ProviderKind::Codex if !can_read => Some("L'archivio Codex è indisponibile."),
            ProviderKind::Codex => removal::native_reason(&app.native),
        };
        let grouped = projects::build(&observation);
        let project_conversations = grouped
            .projects
            .iter()
            .map(|project| project.sessions.len())
            .sum();
        let mut codex_project_keys = HashMap::new();
        let projects: HashMap<_, _> = grouped
            .projects
            .into_iter()
            .map(|project| {
                let key = opaque_key(&project.identity);
                if project.identity.provider == ProviderKind::Codex {
                    codex_project_keys.extend(
                        project
                            .sessions
                            .sessions()
                            .map(|session| (session.identity.native_id.clone(), key.clone())),
                    );
                }
                (key, project)
            })
            .collect();
        let index = LocalInventory::new(observation.sessions);
        let ids = index
            .sessions()
            .enumerate()
            .map(|(i, row)| (key(row), i))
            .collect();
        Self {
            info: ProviderView {
                id: provider,
                presence,
                coverage,
                coverage_description,
                inventory_incomplete: observation.incomplete,
                projects_incomplete: observation.projects_incomplete,
                registered_projects: projects.len(),
                project_conversations,
                recent_conversations: grouped.recents.len(),
                total: index.len(),
                removal_available: removal_reason.is_none(),
                removal_reason,
            },
            version: detected.version,
            conversation_preview: transcript_preview && can_read,
            index,
            recents: grouped.recents,
            ids,
            projects,
            codex_project_keys,
        }
    }

    fn selected(&self, key: &str) -> Result<&Session, ApiError> {
        self.ids
            .get(key)
            .and_then(|&position| self.index.get(position))
            .ok_or_else(|| ApiError::missing("Sessione non presente nell’inventario corrente."))
    }
}

fn key(session: &Session) -> String {
    opaque_key(&session.identity)
}

fn opaque_key(identity: &impl Serialize) -> String {
    let id = serde_json::to_vec(identity).expect("observed identity serializes");
    Sha256::digest(id)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Default)]
pub(super) struct Inventories {
    codex: Mutex<Option<Snapshot>>,
    claude: Mutex<Option<Snapshot>>,
}

impl Inventories {
    fn slot(&self, provider: ProviderKind) -> &Mutex<Option<Snapshot>> {
        match provider {
            ProviderKind::Codex => &self.codex,
            ProviderKind::Claude => &self.claude,
        }
    }

    fn with<T>(&self, app: &App, provider: ProviderKind, read: impl FnOnce(&Snapshot) -> T) -> T {
        let mut slot = self.slot(provider).lock().expect("inventory lock");
        let inventory = slot.get_or_insert_with(|| Snapshot::load(app, provider));
        read(inventory)
    }

    pub(super) fn invalidate(&self, provider: ProviderKind) {
        *self.slot(provider).lock().expect("inventory lock") = None;
    }

    pub(super) fn info(&self, app: &App, provider: ProviderKind) -> ProviderView {
        self.with(app, provider, |snapshot| snapshot.info.clone())
    }

    pub(super) fn list(
        &self,
        app: &App,
        provider: ProviderKind,
        query: &InventoryQuery,
    ) -> PageView {
        self.with(app, provider, |snapshot| {
            PageView::sessions(app, snapshot, snapshot.index.page(query))
        })
    }

    pub(super) fn recents(&self, app: &App, query: &InventoryQuery) -> PageView {
        self.with(app, ProviderKind::Codex, |snapshot| {
            PageView::sessions(app, snapshot, snapshot.recents.page(query))
        })
    }

    pub(super) fn projects(
        &self,
        app: &App,
        provider: ProviderKind,
        query: &InventoryQuery,
    ) -> ProjectsPage {
        self.with(app, provider, |snapshot| {
            let (total, rows) = projects::page(snapshot.projects.values(), query);
            ProjectsPage {
                provider: snapshot.info.clone(),
                total,
                rows: rows
                    .into_iter()
                    .map(|project| ProjectView::new(app, project))
                    .collect(),
            }
        })
    }

    pub(super) fn project<T>(
        &self,
        app: &App,
        provider: ProviderKind,
        key: &str,
        read: impl FnOnce(&App, &Snapshot, &Project) -> T,
    ) -> Result<T, ApiError> {
        self.with(app, provider, |snapshot| {
            snapshot
                .projects
                .get(key)
                .map(|project| read(app, snapshot, project))
                .ok_or_else(|| ApiError::missing("Progetto non presente nell’inventario corrente."))
        })
    }

    pub(super) fn detail(
        &self,
        app: &App,
        provider: ProviderKind,
        key: &str,
    ) -> Result<DetailView, ApiError> {
        self.with(app, provider, |snapshot| {
            let session = snapshot.selected(key)?;
            Ok(DetailView {
                session: SessionView::new(app, snapshot, session),
                version: snapshot.version.clone(),
                conversation_preview: snapshot.conversation_preview
                    && session.transcript_bytes.is_some(),
            })
        })
    }

    pub(super) fn selected(
        &self,
        app: &App,
        provider: ProviderKind,
        key: &str,
    ) -> Result<Session, ApiError> {
        self.with(app, provider, |snapshot| snapshot.selected(key).cloned())
    }
}

pub(super) const NO_CLAUDE_PROJECT_REMOVAL: &str = "Sola lettura. Proprietà di sidecar e riferimenti condivisi e coordinamento dei writer Claude Code non verificati.";

#[derive(Serialize)]
pub(super) struct ProjectsPage {
    provider: ProviderView,
    total: usize,
    rows: Vec<ProjectView>,
}

#[derive(Serialize)]
pub(super) struct ProjectView {
    pub key: String,
    provider: ProviderKind,
    pub name: String,
    workspace: Option<String>,
    namespace: Option<String>,
    pub roots: Vec<String>,
    registration: &'static str,
    main_conversations: usize,
    last_active_at: Option<i64>,
    transcript_bytes: Option<u64>,
    transcript_bytes_incomplete: bool,
    incomplete: bool,
    persistent_instance: bool,
    removal_available: bool,
    pub removal_reason: Option<&'static str>,
}

impl ProjectView {
    pub(super) fn new(app: &App, project: &Project) -> Self {
        let namespace = match &project.identity.locator {
            ProjectLocator::Namespace(namespace) => Some(namespace.clone()),
            _ => None,
        };
        let registration = match &project.identity.locator {
            ProjectLocator::Codex(_) if project.desktop_backed => "native_and_desktop",
            ProjectLocator::Codex(_) => "native",
            ProjectLocator::CodexDesktop(_) => "desktop",
            ProjectLocator::Namespace(_) => "namespace",
        };
        let removal_reason = if project.identity.provider == ProviderKind::Claude {
            Some(NO_CLAUDE_PROJECT_REMOVAL)
        } else if project.desktop_backed {
            Some(
                "La rimozione in blocco richiede appartenenze native verificabili. Le assegnazioni Desktop consentono la consultazione.",
            )
        } else if project.membership_incomplete {
            Some("L’appartenenza delle sessioni non è verificabile con la copertura corrente.")
        } else if project.sessions.is_empty() {
            Some("Il progetto registrato non ha conversazioni principali osservate.")
        } else {
            removal::native_reason(&app.native).or_else(|| {
                project
                    .sessions
                    .sessions()
                    .find_map(|session| removal::refusal(session, &app.protected))
            })
        };
        Self {
            key: opaque_key(&project.identity),
            provider: project.identity.provider,
            name: project.name.clone(),
            workspace: project.workspace.clone(),
            persistent_instance: true,
            roots: project.roots.clone(),
            registration,
            namespace,
            main_conversations: project.sessions.len(),
            last_active_at: project.last_active_at,
            transcript_bytes: project.transcript_bytes,
            transcript_bytes_incomplete: project.transcript_bytes_incomplete,
            incomplete: project.incomplete,
            removal_available: removal_reason.is_none(),
            removal_reason,
        }
    }
}

#[derive(Serialize)]
pub(super) struct PageView {
    provider: ProviderView,
    total: usize,
    rows: Vec<SessionView>,
}

impl PageView {
    pub(super) fn project(
        app: &App,
        snapshot: &Snapshot,
        project: &Project,
        query: &InventoryQuery,
    ) -> Self {
        Self::sessions(app, snapshot, project.page(query))
    }

    fn sessions(app: &App, snapshot: &Snapshot, page: InventoryPage<'_>) -> Self {
        Self {
            provider: snapshot.info.clone(),
            total: page.total,
            rows: page
                .records
                .into_iter()
                .map(|row| SessionView::new(app, snapshot, row))
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub(super) struct DetailView {
    session: SessionView,
    version: Option<String>,
    conversation_preview: bool,
}

#[derive(Serialize)]
struct SessionView {
    key: String,
    native_id: String,
    provider: ProviderKind,
    title: Option<String>,
    project: Option<String>,
    workspace: Option<String>,
    created_at: Option<i64>,
    last_active_at: Option<i64>,
    state: SessionState,
    transcript_bytes: Option<u64>,
    kind: SessionKind,
    protected: bool,
    removal_reason: Option<&'static str>,
}

impl SessionView {
    fn new(app: &App, snapshot: &Snapshot, session: &Session) -> Self {
        let meta = session;
        let protected = meta.identity.provider == ProviderKind::Codex
            && app.protected.contains(&meta.identity.native_id);
        let removal_reason = snapshot
            .info
            .removal_reason
            .or_else(|| removal::refusal(session, &app.protected));
        Self {
            key: key(session),
            native_id: meta.identity.native_id.clone(),
            provider: meta.identity.provider,
            title: meta.title.clone(),
            project: if meta.identity.provider == ProviderKind::Codex {
                snapshot
                    .codex_project_keys
                    .get(&meta.identity.native_id)
                    .and_then(|key| snapshot.projects.get(key))
                    .map(|project| project.name.clone())
            } else {
                meta.project.clone()
            },
            workspace: session.workspace.clone(),
            created_at: meta.created_at,
            last_active_at: meta.last_active_at,
            state: meta.state,
            transcript_bytes: session.transcript_bytes,
            kind: session.kind,
            protected,
            removal_reason,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Filter {
    search: Option<String>,
    state: Option<SessionState>,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    sort: Option<InventorySort>,
    ascending: Option<bool>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl TryFrom<Filter> for InventoryQuery {
    type Error = ApiError;
    fn try_from(value: Filter) -> Result<Self, Self::Error> {
        let search = value.search.unwrap_or_default();
        let offset = value.offset.unwrap_or_default();
        let limit = value.limit.unwrap_or(25);
        if search.len() > 512
            || search.chars().any(char::is_control)
            || offset > 100_000
            || !(1..=100).contains(&limit)
            || value
                .since
                .zip(value.until)
                .is_some_and(|(start, end)| start > end)
        {
            return Err(ApiError::bad("Filtri o paginazione non validi."));
        }
        let to_seconds = |day: NaiveDate| {
            day.and_hms_opt(0, 0, 0)
                .expect("midnight")
                .and_utc()
                .timestamp()
        };
        Ok(Self {
            search,
            state: value.state,
            since: value.since.map(to_seconds),
            before: value.until.map(to_seconds).map(|time| time + 86_400),
            sort: value.sort.unwrap_or_default(),
            ascending: value.ascending.unwrap_or_default(),
            offset,
            limit,
        })
    }
}
