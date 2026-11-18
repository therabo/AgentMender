//! Export inventory and diagnostic preview in a separate authorization namespace.
use super::{ApiError, App, read};
use agentmender_provider_codex::exports::{Group, Inventory, Size};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    sync::{Arc, TryLockError},
    time::{Duration, Instant},
};

const CACHE_AGE: Duration = Duration::from_secs(20);
const MAX_SELECTION: usize = 32;
const PROTECTED: &str = "La rimozione export richiede un contratto verificato dell’esportatore e un’autorizzazione dedicata.";

pub(super) struct CachedInventory {
    snapshot: Arc<Inventory>,
    measured: Instant,
}

fn snapshot(app: &App, fresh: bool) -> Result<Arc<Inventory>, ApiError> {
    let mut cache = app.exports.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Inventario export in aggiornamento.",
        ),
        TryLockError::Poisoned(_) => ApiError::internal(),
    })?;
    if let Some(value) = cache
        .as_ref()
        .filter(|item| !fresh && item.measured.elapsed() < CACHE_AGE)
    {
        return Ok(value.snapshot.clone());
    }
    let inventory = Arc::new(
        app.discovery
            .inspect_exports()
            .map_err(|_| ApiError::internal())?,
    );
    *cache = Some(CachedInventory {
        snapshot: inventory.clone(),
        measured: Instant::now(),
    });
    Ok(inventory)
}

pub(super) async fn discover(
    State(app): State<Arc<App>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(
        serde_json::to_value(&app.discovery).map_err(|_| ApiError::internal())?,
    ))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Sort {
    #[default]
    Size,
    Modified,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Filter {
    search: String,
    sort: Sort,
    offset: usize,
    limit: usize,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            search: String::new(),
            sort: Sort::Size,
            offset: 0,
            limit: 20,
        }
    }
}

#[derive(Serialize)]
pub(super) struct Page {
    rows: Vec<Group>,
    total: usize,
    group_count: usize,
    size: Size,
    complete: bool,
    removal_available: bool,
    removal_reason: &'static str,
}

pub(super) async fn list(
    State(app): State<Arc<App>>,
    Query(filter): Query<Filter>,
) -> Result<Json<Page>, ApiError> {
    if filter.search.len() > 256 || !(1..=100).contains(&filter.limit) || filter.offset > 4_096 {
        return Err(ApiError::bad("Filtri export fuori limite."));
    }
    read(app, move |app| {
        let inventory = snapshot(app, false)?;
        let query = filter.search.trim().to_lowercase();
        let mut groups: Vec<_> = inventory
            .groups
            .iter()
            .filter(|group| {
                query.is_empty()
                    || group.name.to_lowercase().contains(&query)
                    || group.root.to_lowercase().contains(&query)
            })
            .collect();
        groups.sort_unstable_by(|left, right| {
            let primary = match filter.sort {
                Sort::Size => right.size.logical_bytes.cmp(&left.size.logical_bytes),
                Sort::Modified => right.modified_at.cmp(&left.modified_at),
            };
            primary.then_with(|| left.key.cmp(&right.key))
        });
        Ok(Page {
            total: groups.len(),
            rows: groups
                .into_iter()
                .skip(filter.offset)
                .take(filter.limit)
                .cloned()
                .collect(),
            size: inventory.size.clone(),
            group_count: inventory.groups.len(),
            complete: inventory.complete,
            removal_available: false,
            removal_reason: PROTECTED,
        })
    })
    .await
}

pub(super) async fn refresh(State(app): State<Arc<App>>) -> Result<StatusCode, ApiError> {
    let mut cache = app.exports.try_lock().map_err(|_| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Inventario export in aggiornamento.",
        )
    })?;
    *cache = None;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    keys: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct Preview {
    groups: Vec<Group>,
    size: Size,
    removable: usize,
    protected: usize,
    complete: bool,
    receipt: Option<String>,
    reason: &'static str,
}

pub(super) async fn preview(
    State(app): State<Arc<App>>,
    Json(selection): Json<Selection>,
) -> Result<Json<Preview>, ApiError> {
    let keys: BTreeSet<_> = selection.keys.iter().collect();
    if keys.is_empty()
        || keys.len() > MAX_SELECTION
        || keys.len() != selection.keys.len()
        || keys
            .iter()
            .any(|key| key.len() != 64 || !key.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(ApiError::bad(
            "Seleziona da 1 a 32 gruppi distinti dall’inventario.",
        ));
    }
    read(app, move |app| {
        // Diagnostics recompute all metadata before resolving opaque keys, so
        // a replaced group cannot silently substitute a stale selection.
        let inventory = snapshot(app, true)?;
        let mut groups = Vec::with_capacity(selection.keys.len());
        let mut size = Size {
            allocated_bytes: Some(0),
            ..Size::default()
        };
        let mut complete = true;
        for key in selection.keys {
            let group = inventory
                .groups
                .iter()
                .find(|item| item.key == key)
                .ok_or_else(|| {
                    ApiError::conflict("Selezione cambiata. Aggiorna l’inventario export.")
                })?;
            size.logical_bytes = size.logical_bytes.saturating_add(group.size.logical_bytes);
            size.files = size.files.saturating_add(group.size.files);
            size.directories = size.directories.saturating_add(group.size.directories);
            size.allocated_bytes = size
                .allocated_bytes
                .zip(group.size.allocated_bytes)
                .filter(|_| !group.has_shared_entries())
                .map(|(total, bytes)| total.saturating_add(bytes));
            complete &= group.complete;
            groups.push(group.clone());
        }
        Ok(Preview {
            protected: groups.len(),
            groups,
            size,
            complete,
            removable: 0,
            receipt: None,
            reason: PROTECTED,
        })
    })
    .await
}

pub(super) async fn confirm(Path(_key): Path<String>) -> ApiError {
    // No release currently has an export writer contract. A session receipt or
    // a user-provided confirmation never grants this capability.
    ApiError::conflict(PROTECTED)
}
