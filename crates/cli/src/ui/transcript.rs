use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use agentmender_domain::ProviderKind;
use agentmender_provider_codex::TranscriptCursor;
use serde::{Deserialize, Serialize};

use super::{ApiError, App, token};

struct Cursor {
    key: String,
    checkpoint: TranscriptCursor,
    expires: Instant,
}

#[derive(Default)]
pub(super) struct TranscriptCursors(HashMap<String, Cursor>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TranscriptQuery {
    cursor: Option<String>,
}

#[derive(Serialize)]
pub(super) struct TranscriptView {
    messages: Vec<agentmender_provider_codex::TranscriptMessage>,
    next: Option<String>,
    complete: bool,
}

pub(super) fn page(
    app: &App,
    provider: ProviderKind,
    key: &str,
    query: TranscriptQuery,
) -> Result<TranscriptView, ApiError> {
    if provider != ProviderKind::Codex || !cfg!(target_os = "macos") {
        return Err(ApiError::conflict(
            "Anteprima del transcript non disponibile per questo provider.",
        ));
    }
    let session = app.inventory.selected(app, provider, key)?;
    let checkpoint = if let Some(cursor) = query.cursor {
        let mut cursors = app.cursors.lock().expect("cursor lock");
        let now = Instant::now();
        cursors.0.retain(|_, cursor| cursor.expires > now);
        let saved = cursors
            .0
            .get(&cursor)
            .filter(|saved| saved.key == key)
            .ok_or_else(|| {
                ApiError::bad("Pagina scaduta. Riapri l’anteprima della conversazione.")
            })?;
        Some(saved.checkpoint.clone())
    } else {
        None
    };
    let result = app
        .codex
        .transcript_page(&session.identity, checkpoint)
        .map_err(|_| {
            ApiError::conflict(
                "Transcript cambiato, non leggibile o in un formato non riconosciuto.",
            )
        })?;
    let next = result
        .next
        .map(|checkpoint| {
            let id = token()?;
            let mut cursors = app.cursors.lock().expect("cursor lock");
            if cursors.0.len() >= 64
                && let Some(oldest) = cursors
                    .0
                    .iter()
                    .min_by_key(|(_, cursor)| cursor.expires)
                    .map(|(id, _)| id.clone())
            {
                cursors.0.remove(&oldest);
            }
            cursors.0.insert(
                id.clone(),
                Cursor {
                    key: key.into(),
                    checkpoint,
                    expires: Instant::now() + Duration::from_secs(600),
                },
            );
            Ok(id)
        })
        .transpose()?;
    Ok(TranscriptView {
        messages: result.messages,
        next,
        complete: result.complete,
    })
}
