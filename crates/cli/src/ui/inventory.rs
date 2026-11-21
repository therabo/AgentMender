use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    sync::Mutex,
};

use agentmender_application::{
    local_inventory::{InventoryPage, InventoryQuery, InventorySort, LocalInventory},
    projects::{self, Project, ProjectKey},
};
use agentmender_domain::*;
use agentmender_provider_api::ProviderError;
use agentmender_provider_claude::{Access, REMOVAL_NOTICE, SessionEvidence};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{ApiError, App, removal};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Coverage {
    Available,
    Unavailable,
    NotDetected,
}

#[derive(Clone, Serialize)]
pub(super) struct ProviderView {
    pub id: ProviderKind,
    pub coverage: Coverage,
    pub archive_error: Option<&'static str>,
    pub total: usize,
    pub sessions_observed_only: bool,
    pub project_count: usize,
    pub projects_observed_only: bool,
    pub unassigned_sessions: usize,
    pub observed_transcript_bytes: Option<u64>,
    #[serde(skip)]
    removal_reason: Option<&'static str>,
}

struct Observation {
    coverage: Coverage,
    error: Option<&'static str>,
    inventory: InventoryObservation,
    sessions_incomplete: bool,
    claude_evidence: BTreeMap<SessionIdentity, SessionEvidence>,
}

pub(super) struct Snapshot {
    info: ProviderView,
    conversation_preview: bool,
    index: LocalInventory,
    recents: LocalInventory,
    ids: HashMap<String, usize>,
    projects: HashMap<String, Project>,
    claude_evidence: BTreeMap<SessionIdentity, SessionEvidence>,
}

impl Observation {
    fn codex(app: &App) -> Self {
        let blocked_root = matches!(
            app.discovery.selected_access(),
            agentmender_provider_codex::discovery::Access::Linked
                | agentmender_provider_codex::discovery::Access::NetworkPath
        );
        let root_check = if blocked_root {
            Err(std::io::ErrorKind::PermissionDenied.into())
        } else {
            fs::symlink_metadata(&app.discovery.selected_root)
        };
        let (coverage, error, inventory) = match root_check {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if app
                    .codex
                    .detect()
                    .is_ok_and(|installation| installation.detected)
                {
                    (
                        Coverage::Unavailable,
                        Some(
                            "Il provider è installato, ma l'archivio configurato non è stato trovato.",
                        ),
                        InventoryObservation::default(),
                    )
                } else {
                    (
                        Coverage::NotDetected,
                        Some("Installazione e archivio locale non rilevati."),
                        InventoryObservation::default(),
                    )
                }
            }
            Ok(info) if info.is_dir() => match app.codex.local_inventory(app.limit) {
                Ok(observation) => (Coverage::Available, None, observation),
                Err(ProviderError::PermissionDenied(_)) => (
                    Coverage::Unavailable,
                    Some("Accesso negato all’archivio del provider."),
                    InventoryObservation::default(),
                ),
                Err(ProviderError::InvalidScope(reason)) if reason == "stale_codex_source" => (
                    Coverage::Unavailable,
                    Some("L’archivio è cambiato durante la lettura. Aggiorna l’inventario."),
                    InventoryObservation::default(),
                ),
                Err(_) if !cfg!(target_os = "macos") => (
                    Coverage::Unavailable,
                    Some(
                        "La lettura confinata dell’archivio Codex è attualmente disponibile su macOS.",
                    ),
                    InventoryObservation::default(),
                ),
                Err(_) => (
                    Coverage::Unavailable,
                    Some("L’archivio non è leggibile o usa un formato non riconosciuto."),
                    InventoryObservation::default(),
                ),
            },
            _ => (
                Coverage::Unavailable,
                Some("La directory dell’archivio non è accessibile o è un collegamento."),
                InventoryObservation::default(),
            ),
        };
        Self {
            coverage,
            error,
            sessions_incomplete: inventory.incomplete,
            inventory,
            claude_evidence: BTreeMap::new(),
        }
    }

    fn claude(app: &App) -> Self {
        let catalog = app.claude.local_catalog();
        let (coverage, error) = if catalog.roots.iter().any(|root| {
            matches!(root.projects_access, Access::Available)
                || (root.readable && root.projects_access == Access::Missing)
        }) {
            (Coverage::Available, None)
        } else if catalog.roots.iter().any(|root| root.present)
            || app.claude.installations().iter().any(|item| item.confirmed)
        {
            (
                Coverage::Unavailable,
                Some("Nessun archivio di progetto accessibile."),
            )
        } else {
            (
                Coverage::NotDetected,
                Some("Installazione e archivio locale non rilevati."),
            )
        };
        Self {
            coverage,
            error,
            inventory: catalog.inventory,
            sessions_incomplete: catalog.session_scope_incomplete,
            claude_evidence: catalog.evidence,
        }
    }
}

impl Snapshot {
    fn load(app: &App, provider: ProviderKind) -> Self {
        let observed = match provider {
            ProviderKind::Codex => Observation::codex(app),
            ProviderKind::Claude => Observation::claude(app),
        };
        let mut observation = observed.inventory;
        let can_read = matches!(observed.coverage, Coverage::Available);
        // The index, project pages and unassigned pages share one set of
        // canonical session records. UUIDs from distinct archives stay distinct.
        let mut seen = BTreeSet::new();
        observation.sessions.reverse();
        observation
            .sessions
            .retain(|session| seen.insert(session.identity.clone()));
        observation.sessions.reverse();
        let removal_reason = match provider {
            ProviderKind::Claude => Some(REMOVAL_NOTICE),
            ProviderKind::Codex if !can_read => Some("L'archivio Codex è indisponibile."),
            ProviderKind::Codex => removal::native_reason(app),
        };
        let grouped = projects::build(&observation, &app.home);
        let projects: HashMap<_, _> = grouped
            .projects
            .into_iter()
            .map(|project| (project_key(&project.key), project))
            .collect();
        let index = LocalInventory::new(observation.sessions);
        let observed_transcript_bytes =
            observed_transcript_bytes(&index, &observed.claude_evidence);
        let ids = index
            .sessions()
            .enumerate()
            .map(|(i, row)| (key(row), i))
            .collect();
        Self {
            info: ProviderView {
                id: provider,
                coverage: observed.coverage,
                archive_error: observed.error,
                total: index.len(),
                sessions_observed_only: observed.sessions_incomplete,
                project_count: projects.len(),
                projects_observed_only: observation.projects_incomplete,
                unassigned_sessions: grouped.recents.len(),
                observed_transcript_bytes,
                removal_reason,
            },
            conversation_preview: can_read
                && (provider == ProviderKind::Claude || cfg!(target_os = "macos")),
            index,
            recents: grouped.recents,
            ids,
            projects,
            claude_evidence: observed.claude_evidence,
        }
    }

    fn selected(&self, key: &str) -> Result<&Session, ApiError> {
        self.ids
            .get(key)
            .and_then(|&position| self.index.get(position))
            .ok_or_else(|| ApiError::missing("Sessione non presente nell’inventario corrente."))
    }
}

fn observed_transcript_bytes(
    index: &LocalInventory,
    claude_evidence: &BTreeMap<SessionIdentity, SessionEvidence>,
) -> Option<u64> {
    let mut claude_transcripts = BTreeSet::new();
    let mut total = None;
    for session in index.sessions() {
        let Some(bytes) = session.transcript_bytes else {
            continue;
        };
        // Codex's verified rollout header binds it to a single canonical UUID.
        // Claude may observe copies of that UUID in separate roots or namespaces.
        if session.identity.provider == ProviderKind::Claude {
            let Some(digest) = claude_evidence
                .get(&session.identity)
                .and_then(|evidence| evidence.transcript_digest)
            else {
                continue;
            };
            if !claude_transcripts.insert((bytes, digest)) {
                continue;
            }
        }
        total = Some(total.unwrap_or(0u64).checked_add(bytes)?);
    }
    total
}

fn key(session: &Session) -> String {
    opaque_key(&session.identity)
}

fn project_key(key: &ProjectKey) -> String {
    match key {
        ProjectKey::Registered(identity) => opaque_key(identity),
        ProjectKey::Workspace(path) => opaque_key(&("claude_workspace", path)),
    }
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

    pub(super) fn unassigned(
        &self,
        app: &App,
        provider: ProviderKind,
        query: &InventoryQuery,
    ) -> PageView {
        self.with(app, provider, |snapshot| {
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
                conversation_preview: snapshot.conversation_preview
                    && session.transcript_bytes.is_some(),
                claude_cleanup_supported: provider == ProviderKind::Claude
                    && cfg!(target_os = "macos"),
            })
        })
    }

    pub(super) fn selected_claude(
        &self,
        app: &App,
        key: &str,
    ) -> Result<(Session, SessionEvidence), ApiError> {
        self.with(app, ProviderKind::Claude, |snapshot| {
            let session = snapshot.selected(key)?;
            let evidence = snapshot
                .claude_evidence
                .get(&session.identity)
                .ok_or_else(ApiError::internal)?;
            Ok((session.clone(), evidence.clone()))
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

pub(super) const NO_CLAUDE_PROJECT_REMOVAL: &str =
    "Seleziona una conversazione Claude Code individuale.";

#[derive(Serialize)]
pub(super) struct ProjectsPage {
    provider: ProviderView,
    total: usize,
    rows: Vec<ProjectView>,
}

#[derive(Serialize)]
pub(super) struct ProjectView {
    key: String,
    provider: ProviderKind,
    name: String,
    roots: Vec<String>,
    main_conversations: usize,
    removal_available: bool,
    pub removal_reason: Option<&'static str>,
}

impl ProjectView {
    pub(super) fn new(app: &App, project: &Project) -> Self {
        let provider = project.key.provider();
        let removal_reason = if provider == ProviderKind::Claude {
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
            removal::native_reason(app).or_else(|| {
                project
                    .sessions
                    .sessions()
                    .find_map(|session| removal::refusal(session, &app.protected))
            })
        };
        Self {
            key: project_key(&project.key),
            provider,
            name: project.name.clone(),
            roots: project.roots.clone(),
            main_conversations: project.sessions.len(),
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
    conversation_preview: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    claude_cleanup_supported: bool,
}

#[derive(Serialize)]
struct SessionView {
    key: String,
    native_id: String,
    title: Option<String>,
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
            title: meta.title.clone(),
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
        {
            return Err(ApiError::bad("Filtri o paginazione non validi."));
        }
        Ok(Self {
            search,
            state: value.state,
            sort: value.sort.unwrap_or_default(),
            ascending: value.ascending.unwrap_or_default(),
            offset,
            limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed_codex_sizes(sizes: &[Option<u64>]) -> Option<u64> {
        let index = LocalInventory::new(
            sizes
                .iter()
                .enumerate()
                .map(|(index, &bytes)| Session {
                    identity: SessionIdentity {
                        provider: ProviderKind::Codex,
                        state_root: Default::default(),
                        namespace: None,
                        native_id: index.to_string(),
                    },
                    codex_membership: CodexMembership::Unassigned,
                    title: None,
                    workspace: None,
                    project: None,
                    created_at: None,
                    last_active_at: None,
                    state: SessionState::Unknown,
                    transcript_bytes: bytes,
                    kind: SessionKind::Main,
                    forked_from: None,
                    parent: None,
                })
                .collect(),
        );
        observed_transcript_bytes(&index, &BTreeMap::new())
    }

    #[test]
    fn observed_storage_skips_missing_sizes_and_detects_overflow() {
        assert_eq!(
            observed_codex_sizes(&[Some(4096), None, Some(1024)]),
            Some(5120)
        );
        assert_eq!(observed_codex_sizes(&[Some(0), None]), Some(0));
        assert_eq!(observed_codex_sizes(&[None, None]), None);
        assert_eq!(observed_codex_sizes(&[Some(u64::MAX), Some(1)]), None);
    }
}
