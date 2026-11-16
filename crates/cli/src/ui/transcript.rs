use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use agentmender_domain::{ContextReference, ProviderKind, TranscriptCheckpoint};
use agentmender_provider_api::SessionSearchReadPort;
use serde::{Deserialize, Serialize};

use super::{ApiError, App, token};

struct Cursor {
    key: String,
    reference: ContextReference,
    checkpoint: TranscriptCheckpoint,
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
    messages: Vec<Message>,
    next: Option<String>,
    complete: bool,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    text: String,
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
    let unavailable = || {
        ApiError::conflict("Transcript cambiato, non leggibile o in un formato non riconosciuto.")
    };
    let (reference, checkpoint) = if let Some(cursor) = query.cursor {
        let mut cursors = app.cursors.lock().expect("cursor lock");
        cursors
            .0
            .retain(|_, cursor| cursor.expires > Instant::now());
        let saved = cursors
            .0
            .get(&cursor)
            .filter(|saved| saved.key == key)
            .ok_or_else(|| {
                ApiError::bad("Pagina scaduta. Riapri l’anteprima della conversazione.")
            })?;
        (saved.reference.clone(), Some(saved.checkpoint.clone()))
    } else {
        (
            app.codex
                .local_transcript_reference(&session.metadata.native_session_id)
                .map_err(|_| unavailable())?,
            None,
        )
    };
    let result = app
        .codex
        .transcript_page(&reference, checkpoint.clone(), &[])
        .map_err(|_| unavailable())?;
    let next = result
        .next
        .filter(|next| Some(next) != checkpoint.as_ref())
        .map(|checkpoint| {
            let id = token()?;
            let mut cursors = app.cursors.lock().expect("cursor lock");
            if let Some(cursor) = checkpoint_oldest(&cursors.0).filter(|_| cursors.0.len() >= 64) {
                cursors.0.remove(&cursor);
            }
            cursors.0.insert(
                id.clone(),
                Cursor {
                    key: key.into(),
                    reference,
                    checkpoint,
                    expires: Instant::now() + Duration::from_secs(600),
                },
            );
            Ok(id)
        })
        .transpose()?;
    Ok(TranscriptView {
        messages: result
            .messages
            .into_iter()
            .map(|message| Message {
                role: match message.evidence.source.as_str() {
                    "user_message" => "user",
                    "assistant_message" => "assistant",
                    _ => "unknown",
                },
                text: message.evidence.excerpt.unwrap_or_default(),
            })
            .collect(),
        next,
        complete: result.complete,
    })
}

fn checkpoint_oldest(cursors: &HashMap<String, Cursor>) -> Option<String> {
    cursors
        .iter()
        .min_by_key(|(_, cursor)| cursor.expires)
        .map(|(id, _)| id.clone())
}
