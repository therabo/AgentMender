//! Read-only usage endpoint with a bounded cache and one in-flight inventory.
use super::{ApiError, App, read};
use agentmender_domain::ProviderKind;
use agentmender_provider_claude::StorageInventory;
use axum::{
    Json,
    extract::State,
    http::{StatusCode, Uri},
};
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Serialize)]
pub(super) struct SessionUsage {
    pub key: String,
    pub title: Option<String>,
    pub native_id: String,
    pub namespace: Option<String>,
    pub logical_bytes: u64,
    pub exclusive_logical_bytes: u64,
}

#[derive(Serialize)]
pub(super) struct Footprints {
    largest_sessions: Vec<SessionUsage>,
    associated_transcript_bytes: u64,
    exclusive_transcript_bytes: u64,
    catalog_complete: bool,
}

impl Footprints {
    pub(super) fn new(entries: impl Iterator<Item = SessionUsage>, catalog_complete: bool) -> Self {
        let mut largest_sessions: Vec<SessionUsage> = Vec::new();
        let mut associated_transcript_bytes = 0u64;
        let mut exclusive_transcript_bytes = 0u64;
        for row in entries {
            associated_transcript_bytes =
                associated_transcript_bytes.saturating_add(row.logical_bytes);
            exclusive_transcript_bytes =
                exclusive_transcript_bytes.saturating_add(row.exclusive_logical_bytes);
            let rank = largest_sessions.partition_point(|item| {
                item.logical_bytes > row.logical_bytes
                    || (item.logical_bytes == row.logical_bytes && item.key < row.key)
            });
            if rank < 20 {
                largest_sessions.insert(rank, row);
                largest_sessions.truncate(20);
            }
        }
        Self {
            largest_sessions,
            associated_transcript_bytes,
            exclusive_transcript_bytes,
            catalog_complete,
        }
    }
}

#[derive(Serialize)]
struct Usage {
    #[serde(flatten)]
    inventory: StorageInventory,
    #[serde(flatten)]
    sessions: Footprints,
}

pub(super) struct CachedUsage {
    measured: Instant,
    value: serde_json::Value,
}

fn busy() -> ApiError {
    ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        "Inventario Claude in aggiornamento.",
    )
}

pub(super) async fn inspect(
    State(app): State<Arc<App>>,
    uri: Uri,
) -> Result<Json<serde_json::Value>, ApiError> {
    if uri.query().is_some() {
        return Err(ApiError::bad(
            "L’inventario usa le root configurate all’avvio.",
        ));
    }
    read(app, |app| {
        let mut slot = app.claude_usage.try_lock().map_err(|_| busy())?;
        if let Some(cached) = slot
            .as_ref()
            .filter(|value| value.measured.elapsed() < Duration::from_secs(20))
        {
            return Ok(cached.value.clone());
        }
        let value = serde_json::to_value(Usage {
            inventory: app.claude.storage_inventory(),
            sessions: app.inventory.claude_footprints(app),
        })
        .map_err(|_| ApiError::internal())?;
        *slot = Some(CachedUsage {
            measured: Instant::now(),
            value: value.clone(),
        });
        Ok(value)
    })
    .await
}

pub(super) async fn refresh(State(app): State<Arc<App>>) -> Result<StatusCode, ApiError> {
    let _ = read(app, |app| {
        let mut slot = app.claude_usage.try_lock().map_err(|_| busy())?;
        *slot = None;
        app.inventory.invalidate(ProviderKind::Claude);
        Ok(())
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
