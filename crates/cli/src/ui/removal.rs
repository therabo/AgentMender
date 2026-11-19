//! Explicit session deletion through the existing loopback authorization.
use super::{ApiError, App, inventory::ProjectView, provider, read};
use agentmender_domain::{ProviderKind, Session};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(super) const PROTECTED: &str = "La sessione è protetta dalla configurazione di avvio.";

pub(super) fn native_reason(app: &App) -> Option<&'static str> {
    if app
        .discovery
        .location
        .executable
        .as_ref()
        .is_some_and(|path| path.is_file())
    {
        None
    } else {
        Some("Eseguibile Codex non disponibile.")
    }
}

pub(super) fn refusal(session: &Session, protected: &[String]) -> Option<&'static str> {
    (session.identity.provider == ProviderKind::Codex
        && protected.contains(&session.identity.native_id))
    .then_some(PROTECTED)
}

#[cfg(target_os = "macos")]
pub(super) struct ClaudeReview {
    id: String,
    key: String,
    plan: agentmender_provider_claude::cleanup::Plan,
}

#[cfg(target_os = "macos")]
pub(super) async fn claude_preview(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    read(app, move |app| {
        let (session, evidence) = app.inventory.selected_claude(app, &key)?;
        let plan = app
            .claude
            .removal_plan(&session.identity, &evidence)
            .map_err(|_| {
                ApiError::conflict(
                    "Transcript Claude cambiato o non leggibile. Aggiorna l’inventario.",
                )
            })?;
        let id = super::token()?;
        let view = serde_json::json!({ "review": id, "plan": &plan });
        *app.claude_review.lock().expect("Claude review") = Some(ClaudeReview { id, key, plan });
        Ok(view)
    })
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirmation {
    confirm: bool,
    review: Option<String>,
}

#[derive(Serialize)]
pub(super) struct ResultView {
    status: &'static str,
    message: String,
    errors: Vec<String>,
}

fn result(removed: usize, message: String, errors: Vec<String>) -> ResultView {
    ResultView {
        status: if errors.is_empty() {
            "deleted"
        } else if removed > 0 {
            "partial"
        } else {
            "failed"
        },
        message,
        errors,
    }
}

pub(super) async fn session(
    State(app): State<Arc<App>>,
    Path((name, key)): Path<(String, String)>,
    Json(request): Json<Confirmation>,
) -> Result<Json<ResultView>, ApiError> {
    if !request.confirm {
        return Err(ApiError::bad("Conferma l’eliminazione definitiva."));
    }
    let provider = provider(&name)?;
    read(app, move |app| {
        let outcome = match provider {
            ProviderKind::Codex => {
                if request.review.is_some() {
                    return Err(ApiError::bad("Conferma Codex non valida."));
                }
                let session = app.inventory.selected(app, provider, &key)?;
                if let Some(reason) = refusal(&session, &app.protected) {
                    return Err(ApiError::conflict(reason));
                }
                match app.codex.delete_session(&session.identity) {
                    Ok(outcome) => {
                        let message = if outcome.errors.is_empty() {
                            outcome.message
                        } else {
                            format!(
                                "Codex ha eliminato la sessione. Pulizia aggiuntiva incompleta. {}",
                                outcome.message,
                            )
                        };
                        result(1, message, outcome.errors)
                    }
                    Err(error) => result(
                        0,
                        "Codex non ha completato l’eliminazione.".into(),
                        vec![error],
                    ),
                }
            }
            ProviderKind::Claude => {
                #[cfg(target_os = "macos")]
                {
                    let review = {
                        let mut slot = app.claude_review.lock().expect("Claude review");
                        slot.take_if(|plan| {
                            plan.key == key && Some(&plan.id) == request.review.as_ref()
                        })
                        .ok_or_else(|| ApiError::conflict("Apri una nuova anteprima Claude."))?
                    };
                    let outcome = review.plan.delete();
                    *app.claude_usage.lock().expect("Claude usage") = None;
                    result(
                        outcome.removed_files,
                        format!(
                            "{} file eliminati · {} byte logici · {} righe di history eliminate.",
                            outcome.removed_files, outcome.removed_bytes, outcome.history_rows,
                        ),
                        outcome.errors,
                    )
                }
                #[cfg(not(target_os = "macos"))]
                return Err(ApiError::conflict(
                    agentmender_provider_claude::REMOVAL_NOTICE,
                ));
            }
        };
        app.inventory.invalidate(provider);
        Ok(outcome)
    })
    .await
}

pub(super) async fn project(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
    Json(request): Json<Confirmation>,
) -> Result<Json<ResultView>, ApiError> {
    if !request.confirm || request.review.is_some() {
        return Err(ApiError::bad("Conferma l’eliminazione definitiva."));
    }
    read(app, move |app| {
        let sessions =
            app.inventory
                .project(app, ProviderKind::Codex, &key, |app, _, project| {
                    if let Some(reason) = ProjectView::new(app, project).removal_reason {
                        return Err(ApiError::conflict(reason));
                    }
                    Ok(project
                        .sessions
                        .sessions()
                        .map(|row| row.identity.clone())
                        .collect::<Vec<_>>())
                })??;
        let mut removed = 0;
        let mut errors = Vec::new();
        for session in &sessions {
            match app.codex.delete_session(session) {
                Ok(outcome) => {
                    removed += 1;
                    errors.extend(outcome.errors.into_iter().map(|error| {
                        format!(
                            "{}: pulizia aggiuntiva incompleta. {error}",
                            session.native_id
                        )
                    }));
                }
                Err(error) => {
                    errors.push(format!("{}: {error}", session.native_id));
                    break;
                }
            }
        }
        app.inventory.invalidate(ProviderKind::Codex);
        Ok(result(
            removed,
            format!(
                "Codex ha confermato {} operazioni su {}.",
                removed,
                sessions.len()
            ),
            errors,
        ))
    })
    .await
}
