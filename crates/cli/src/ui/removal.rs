//! Explicit session deletion through the existing loopback authorization.
use super::{ApiError, App, inventory::ProjectView, provider, read};
use agentmender_domain::{ProviderKind, Session, SessionIdentity};
use agentmender_provider_codex::{Deletion, DeletionPlan};
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

pub(super) struct CodexReview {
    id: String,
    key: String,
    project: bool,
    plan: DeletionPlan,
}

fn codex_review(
    app: &App,
    key: String,
    project: bool,
    sessions: &[SessionIdentity],
) -> Result<serde_json::Value, ApiError> {
    if let Some(reason) = native_reason(app) {
        return Err(ApiError::conflict(reason));
    }
    let plan = app
        .codex
        .deletion_plan(sessions, &app.protected)
        .map_err(|_| {
            ApiError::conflict("Grafo Codex non verificabile o contenente una sessione protetta.")
        })?;
    let id = super::token()?;
    let view = serde_json::json!({ "review": id, "plan": &plan });
    *app.codex_review.lock().expect("Codex review") = Some(CodexReview {
        id,
        key,
        project,
        plan,
    });
    Ok(view)
}

pub(super) async fn codex_session_preview(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    read(app, move |app| {
        let session = app.inventory.selected(app, ProviderKind::Codex, &key)?;
        if let Some(reason) = refusal(&session, &app.protected) {
            return Err(ApiError::conflict(reason));
        }
        codex_review(app, key, false, &[session.identity])
    })
    .await
}

pub(super) async fn codex_project_preview(
    State(app): State<Arc<App>>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
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
        codex_review(app, key, true, &sessions)
    })
    .await
}

fn confirm_codex(
    app: &App,
    key: String,
    project: bool,
    review: Option<String>,
) -> Result<ResultView, ApiError> {
    let review = app
        .codex_review
        .lock()
        .expect("Codex review")
        .take_if(|plan| {
            plan.key == key && plan.project == project && Some(&plan.id) == review.as_ref()
        })
        .ok_or_else(|| ApiError::conflict("Apri una nuova anteprima Codex."))?;
    let Deletion {
        native_succeeded,
        verified_absent,
        messages,
        errors,
    } = app.codex.delete_reviewed(review.plan).map_err(|_| {
        ApiError::conflict("Comando nativo Codex non disponibile per l’archivio selezionato.")
    })?;
    let mut message = format!(
        "Comandi nativi riusciti · {native_succeeded}. UUID verificati assenti · {verified_absent}."
    );
    if !errors.is_empty() && (native_succeeded > 0 || verified_absent > 0) {
        message.push_str(" Operazione incompleta.");
    }
    if !project && let Some(detail) = messages.first() {
        message.push(' ');
        message.push_str(detail);
    }
    let mut view = result(native_succeeded.max(verified_absent), message, errors);
    view.codex_counts = Some(CodexCounts {
        native_succeeded,
        verified_absent,
    });
    Ok(view)
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
    #[serde(flatten)]
    codex_counts: Option<CodexCounts>,
}

#[derive(Serialize)]
struct CodexCounts {
    native_succeeded: usize,
    verified_absent: usize,
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
        codex_counts: None,
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
            ProviderKind::Codex => confirm_codex(app, key, false, request.review)?,
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
    if !request.confirm {
        return Err(ApiError::bad("Conferma l’eliminazione definitiva."));
    }
    read(app, move |app| {
        let outcome = confirm_codex(app, key, true, request.review)?;
        app.inventory.invalidate(ProviderKind::Codex);
        Ok(outcome)
    })
    .await
}
