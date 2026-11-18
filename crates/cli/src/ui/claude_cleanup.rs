//! Claude-only receipts. Browser payloads carry a mode or an opaque key.
use super::{ApiError, App, read, token};
use agentmender_domain::ProviderKind;
use agentmender_provider_claude::cleanup::{Backup, Mode, Outcome, Refusal, Review};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

const VALID: Duration = Duration::from_secs(120);
const REMOVE: &str = "RIMUOVI SESSIONE CLAUDE";
const RESTORE: &str = "RIPRISTINA SESSIONE CLAUDE";
const DISCARD: &str = "ELIMINA BACKUP CLAUDE";
const MAX_RECEIPTS: usize = 24;

enum Action {
    Remove(Box<Review>),
    Backup {
        id: String,
        discard: bool,
        fingerprint: [u8; 32],
    },
}

struct Receipt {
    created: Instant,
    text: &'static str,
    action: Action,
}

#[derive(Default)]
pub(super) struct Operations {
    receipts: BTreeMap<String, Receipt>,
    running: Option<String>,
    completed: BTreeMap<String, (Instant, Option<Outcome>)>,
}

#[derive(Serialize)]
struct ConfirmationView {
    id: String,
    confirmation: &'static str,
    valid_seconds: u64,
}

fn issue(app: &App, action: Action, text: &'static str) -> Result<ConfirmationView, ApiError> {
    let mut state = app
        .claude_cleanup
        .lock()
        .map_err(|_| ApiError::internal())?;
    state
        .receipts
        .retain(|_, receipt| receipt.created.elapsed() < VALID);
    if state.receipts.len() >= MAX_RECEIPTS || state.running.is_some() {
        return Err(ApiError::conflict(
            "Una rimozione Claude è in corso o ci sono troppe anteprime aperte.",
        ));
    }
    let id = token()?;
    state.receipts.insert(
        id.clone(),
        Receipt {
            created: Instant::now(),
            text,
            action,
        },
    );
    Ok(ConfirmationView {
        id,
        confirmation: text,
        valid_seconds: VALID.as_secs(),
    })
}

fn refusal(reason: Refusal) -> Response {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "status": reason,
            "error": reason.message(),
        })),
    )
        .into_response()
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Selection {
    mode: Mode,
}

pub(super) async fn preview(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
    request: Option<Json<Selection>>,
) -> Result<Response, ApiError> {
    let mode = request.map_or(Mode::Standard, |Json(request)| request.mode);
    read(app, move |app| {
        let (session, _) = app.inventory.selected_claude(app, &key)?;
        let review = match app
            .claude
            .prepare_cleanup(&session.identity, mode, app.limit)
        {
            Ok(review) => review,
            Err(reason) => return Ok(refusal(reason)),
        };
        let details = serde_json::to_value(review.preview()).map_err(|_| ApiError::internal())?;
        let confirmation = issue(app, Action::Remove(Box::new(review)), REMOVE)?;
        Ok(Json(serde_json::json!({
            "review": details,
            "id": confirmation.id,
            "confirmation": confirmation.confirmation,
            "valid_seconds": confirmation.valid_seconds,
        }))
        .into_response())
    })
    .await
    .map(|Json(response)| response)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum BackupAction {
    Restore,
    Discard,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectionBackup {
    action: BackupAction,
}

pub(super) async fn backup_preview(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(request): Json<SelectionBackup>,
) -> Result<Response, ApiError> {
    let discard = matches!(request.action, BackupAction::Discard);
    read(app, move |app| {
        let fingerprint = match app.claude.check_backup(&id, discard) {
            Ok(fingerprint) => fingerprint,
            Err(reason) => return Ok(refusal(reason)),
        };
        let text = if discard { DISCARD } else { RESTORE };
        let receipt = issue(
            app,
            Action::Backup {
                id,
                discard,
                fingerprint,
            },
            text,
        )?;
        Ok(Json(receipt).into_response())
    })
    .await
    .map(|Json(response)| response)
}

pub(super) async fn backups(State(app): State<Arc<App>>) -> Result<Json<Vec<Backup>>, ApiError> {
    read(app, |app| {
        app.claude
            .cleanup_backups()
            .map_err(|reason| ApiError::conflict(reason.message()))
    })
    .await
}

pub(super) async fn confirm(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
    Json(request): Json<super::Confirmation>,
) -> Result<StatusCode, ApiError> {
    let action = {
        let mut state = app
            .claude_cleanup
            .lock()
            .map_err(|_| ApiError::internal())?;
        if state.running.is_some() {
            return Err(ApiError::conflict("Una rimozione Claude è già in corso."));
        }
        let receipt = state
            .receipts
            .remove(&key)
            .filter(|receipt| {
                receipt.created.elapsed() < VALID && receipt.text == request.confirmation
            })
            .ok_or_else(|| {
                ApiError::conflict("Conferma Claude scaduta, non valida o già utilizzata.")
            })?;
        state.running = Some(key.clone());
        receipt.action
    };
    tokio::spawn(async move {
        let worker = app.clone();
        let backup = key.clone();
        let outcome = tokio::task::spawn_blocking(move || match action {
            Action::Remove(review) => worker
                .claude
                .execute_cleanup(*review, &backup, worker.limit),
            Action::Backup {
                id,
                discard,
                fingerprint,
            } => worker.claude.act_on_backup(&id, discard, fingerprint),
        })
        .await
        .ok();
        app.inventory.invalidate(ProviderKind::Claude);
        if let Ok(mut cache) = app.claude_usage.lock() {
            *cache = None;
        }
        if let Ok(mut state) = app.claude_cleanup.lock() {
            state.running = None;
            state
                .completed
                .retain(|_, (created, _)| created.elapsed() < Duration::from_secs(3600));
            if state.completed.len() >= 128
                && let Some(oldest) = state
                    .completed
                    .iter()
                    .min_by_key(|(_, (created, _))| created)
                    .map(|(key, _)| key.clone())
            {
                state.completed.remove(&oldest);
            }
            state.completed.insert(key, (Instant::now(), outcome));
        }
    });
    Ok(StatusCode::ACCEPTED)
}

pub(super) async fn operation(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let state = app
        .claude_cleanup
        .lock()
        .map_err(|_| ApiError::internal())?;
    if state.running.as_ref() == Some(&key) {
        return Ok(Json(serde_json::json!({ "status": "running" })));
    }
    match state.completed.get(&key) {
        Some((_, Some(outcome))) => serde_json::to_value(outcome)
            .map(Json)
            .map_err(|_| ApiError::internal()),
        Some((_, None)) => Ok(Json(serde_json::json!({ "status": "uncertain" }))),
        None => Err(ApiError::missing(
            "Operazione Claude non trovata nella sessione locale.",
        )),
    }
}
