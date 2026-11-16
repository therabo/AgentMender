use std::{fs, path::Path, sync::Mutex};

use agentmender_application::local_inventory::{InventoryQuery, InventorySort, LocalInventory};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderAdapter, ProviderError};
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
    pub coverage_description: &'static str,
    pub total: usize,
    pub version: Option<String>,
    pub conversation_preview: bool,
    pub removal_available: bool,
    pub removal_reason: Option<&'static str>,
}

pub(super) const NO_CLAUDE_REMOVAL: &str =
    "Claude Code è in sola lettura. La rimozione per singola sessione non è disponibile.";

pub(super) struct Snapshot {
    info: ProviderView,
    index: LocalInventory,
    ids: std::collections::HashMap<String, usize>,
}

impl Snapshot {
    fn load(app: &App, provider: ProviderKind) -> Self {
        let (root, adapter, transcript_preview): (&Path, &dyn ProviderAdapter, bool) =
            match provider {
                ProviderKind::Codex => (&app.codex_root, &app.codex, cfg!(target_os = "macos")),
                ProviderKind::Claude => (&app.claude_root, &app.claude, false),
            };
        let detected = adapter.detect().ok();
        let version = detected
            .as_ref()
            .and_then(|p| p.fingerprint.executable_version.clone());
        let root_check = fs::symlink_metadata(root);
        let presence = if detected.is_some_and(|p| p.presence == Presence::Installed) {
            Presence::Installed
        } else if root_check.is_ok() {
            Presence::StateOnly
        } else {
            Presence::NotDetected
        };
        let (coverage, coverage_description, sessions) = match root_check {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if presence == Presence::NotDetected {
                    (
                        Coverage::NotDetected,
                        "Installazione e archivio locale non rilevati.",
                        Vec::new(),
                    )
                } else {
                    (
                        Coverage::Unavailable,
                        "Il provider è installato, ma l'archivio configurato non è stato trovato.",
                        Vec::new(),
                    )
                }
            }
            Ok(info) if info.is_dir() => {
                let result = match provider {
                    ProviderKind::Codex => app.codex.local_inventory(app.limit),
                    ProviderKind::Claude => app.claude.local_inventory(app.limit),
                };
                match result {
                    Ok(sessions) => {
                        let description = if sessions.len() >= app.limit {
                            "Raggiunto il limite di inventario. Altre sessioni potrebbero essere presenti."
                        } else if provider == ProviderKind::Codex {
                            "Dati del database Codex locale riconosciuto. Alcune sessioni o relazioni potrebbero mancare."
                        } else {
                            "Metadati riconosciuti nei transcript di progetto. Formati, titoli e date possono essere incompleti."
                        };
                        (Coverage::Partial, description, sessions)
                    }
                    Err(ProviderError::PermissionDenied(_)) => (
                        Coverage::Unavailable,
                        "Accesso negato all’archivio del provider.",
                        Vec::new(),
                    ),
                    Err(ProviderError::InvalidScope(reason))
                        if reason == "stale_search_evidence" =>
                    {
                        (
                            Coverage::Unavailable,
                            "L’archivio è cambiato durante la lettura. Aggiorna l’inventario.",
                            Vec::new(),
                        )
                    }
                    Err(_) if provider == ProviderKind::Codex && !transcript_preview => (
                        Coverage::Unavailable,
                        "La lettura confinata dell’archivio Codex è attualmente disponibile su macOS.",
                        Vec::new(),
                    ),
                    Err(_) => (
                        Coverage::Unavailable,
                        "L’archivio non è leggibile o usa un formato non riconosciuto.",
                        Vec::new(),
                    ),
                }
            }
            _ => (
                Coverage::Unavailable,
                "La directory dell’archivio non è accessibile o è un collegamento.",
                Vec::new(),
            ),
        };
        let can_read = matches!(coverage, Coverage::Partial);
        let removal_reason = match provider {
            ProviderKind::Claude => Some(NO_CLAUDE_REMOVAL),
            ProviderKind::Codex if !can_read => Some("L'archivio Codex è indisponibile."),
            ProviderKind::Codex if app.native.is_none() => Some(removal::UNSUPPORTED),
            _ => None,
        };
        let index = LocalInventory::new(sessions);
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
                total: index.len(),
                version,
                conversation_preview: transcript_preview && can_read,
                removal_available: removal_reason.is_none(),
                removal_reason,
            },
            index,
            ids,
        }
    }

    fn selected(&self, key: &str) -> Result<&LocalSession, ApiError> {
        self.ids
            .get(key)
            .and_then(|&position| self.index.get(position))
            .ok_or_else(|| ApiError::missing("Sessione non presente nell’inventario corrente."))
    }
}

fn key(session: &LocalSession) -> String {
    let id = serde_json::to_vec(&session.metadata.identity).expect("observed identity serializes");
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
            let page = snapshot.index.page(query);
            PageView {
                provider: snapshot.info.clone(),
                total: page.total,
                rows: page
                    .records
                    .into_iter()
                    .map(|row| SessionView::new(app, snapshot, row))
                    .collect(),
            }
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
            let children: Vec<_> = snapshot
                .index
                .children(session)
                .take(51)
                .map(|row| row.metadata.native_session_id.clone())
                .collect();
            Ok(DetailView {
                session: SessionView::new(app, snapshot, session),
                version: snapshot.info.version.clone(),
                coverage: snapshot.info.coverage,
                conversation_preview: snapshot.info.conversation_preview
                    && session.transcript_bytes.is_some(),
                relationships_limited: children.len() > 50,
                children: children.into_iter().take(50).collect(),
            })
        })
    }

    pub(super) fn selected(
        &self,
        app: &App,
        provider: ProviderKind,
        key: &str,
    ) -> Result<LocalSession, ApiError> {
        self.with(app, provider, |snapshot| snapshot.selected(key).cloned())
    }
}

#[derive(Serialize)]
pub(super) struct PageView {
    provider: ProviderView,
    total: usize,
    rows: Vec<SessionView>,
}

#[derive(Serialize)]
pub(super) struct DetailView {
    session: SessionView,
    version: Option<String>,
    coverage: Coverage,
    conversation_preview: bool,
    relationships_limited: bool,
    children: Vec<String>,
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
    kind: LocalSessionKind,
    parent: Option<String>,
    forked_from: Option<String>,
    protected: bool,
    removal_reason: Option<&'static str>,
}

impl SessionView {
    fn new(app: &App, snapshot: &Snapshot, session: &LocalSession) -> Self {
        let meta = &session.metadata;
        let protected = meta.identity.provider == ProviderKind::Codex
            && app.protected.contains(&meta.native_session_id);
        let removal_reason = snapshot
            .info
            .removal_reason
            .or_else(|| removal::refusal(session, app.native.is_some(), &app.protected))
            .or_else(|| {
                snapshot
                    .index
                    .children(session)
                    .next()
                    .map(|_| removal::AFFECTED_UNKNOWN)
            });
        Self {
            key: key(session),
            native_id: meta.native_session_id.clone(),
            provider: meta.identity.provider,
            title: meta.title.clone(),
            project: meta.project_label.clone(),
            workspace: session.workspace.clone(),
            created_at: meta.created_at,
            last_active_at: meta.last_active_at,
            state: meta.state,
            transcript_bytes: session.transcript_bytes,
            kind: session.kind,
            parent: meta.parent_native_session_id.clone(),
            forked_from: session.forked_from.clone(),
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
