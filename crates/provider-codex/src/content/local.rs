//! Local UI observations from the same confined snapshot as conversation reads.
#[cfg(target_os = "macos")]
use super::*;
use agentmender_domain::LocalSession;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct LocalPage {
    pub sessions: Vec<LocalSession>,
    pub next: Option<String>,
    pub revision: String,
    pub skipped: bool,
}

/// Exact review facts. There is no command, approval or browser-supplied path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalRemovalEvidence {
    pub session: LocalSession,
    revision: String,
}

#[cfg(target_os = "macos")]
fn workspace(row: &rusqlite::Row<'_>) -> Option<String> {
    row.get::<_, Option<String>>(2).ok().flatten().filter(|v| {
        v.len() <= 4096
            && observed_path_is_absolute(Path::new(v))
            && !v.chars().any(char::is_control)
            && safe_conversation_text(v)
    })
}

#[cfg(target_os = "macos")]
fn edge_columns(db: &Connection) -> bool {
    metadata::columns(db, "thread_spawn_edges").is_ok_and(|cols| {
        ["parent_thread_id", "child_thread_id"]
            .iter()
            .all(|name| cols.get(*name).is_some_and(|ty| ty == "TEXT"))
    }) && db
        .query_row(
            "SELECT type FROM sqlite_master WHERE name='thread_spawn_edges'",
            [],
            |r| r.get::<_, String>(0),
        )
        .is_ok_and(|kind| kind == "table")
}

#[cfg(target_os = "macos")]
fn local_row(
    db: &Connection,
    root: &Path,
    row: &rusqlite::Row<'_>,
    edges: bool,
) -> Result<(Candidate, LocalSession), ProviderError> {
    let mut candidate = candidate_row(row, root)?;
    let parent = if edges {
        let mut query = db
            .prepare(
                "SELECT parent_thread_id FROM thread_spawn_edges WHERE child_thread_id=?1 LIMIT 2",
            )
            .map_err(|_| unavailable())?;
        let mut rows = query
            .query([&candidate.metadata.native_session_id])
            .map_err(|_| unavailable())?;
        let value = rows
            .next()
            .map_err(|_| unavailable())?
            .map(|r| r.get::<_, String>(0))
            .transpose()
            .map_err(|_| unavailable())?;
        if rows.next().map_err(|_| unavailable())?.is_some() {
            None
        } else {
            value.filter(|id| metadata::uuid(id))
        }
    } else {
        None
    };
    candidate.metadata.parent_native_session_id = parent.clone();
    let kind = if parent.is_some() {
        LocalSessionKind::Subagent
    } else {
        match row.get::<_, Option<String>>(8).ok().flatten().as_deref() {
            Some("cli" | "vscode" | "exec" | "appServer") => LocalSessionKind::Main,
            _ => LocalSessionKind::Unknown,
        }
    };
    let forked_from = row
        .get::<_, Option<String>>(9)
        .ok()
        .flatten()
        .filter(|id| metadata::uuid(id));
    let result = LocalSession {
        metadata: candidate.metadata.clone(),
        workspace: workspace(row),
        transcript_bytes: open_transcript(&candidate, root)
            .ok()
            .and_then(|(file, _)| file.metadata().ok())
            .map(|meta| meta.len()),
        kind: if forked_from.is_some() && parent.is_none() {
            LocalSessionKind::Fork
        } else {
            kind
        },
        forked_from,
        parent: parent.as_ref().map(|id| identity(root, id)).transpose()?,
    };
    Ok((candidate, result))
}

#[cfg(target_os = "macos")]
fn statement(columns: &BTreeMap<String, String>, predicate: &str) -> String {
    let (title, producer, path) = metadata_projection(columns);
    let optional = |column: &'static str| {
        if columns.get(column).is_some_and(|ty| ty == "TEXT") {
            column
        } else {
            "NULL"
        }
    };
    let source = optional("source");
    let fork = optional("forked_from_id");
    format!(
        "SELECT id,{title},cwd,created_at,updated_at,archived,{producer},{path},{source},{fork} FROM threads {predicate}"
    )
}

#[cfg(target_os = "macos")]
pub(super) fn page(
    db: &Connection,
    root: &Path,
    after: &str,
    revision: String,
    end: Instant,
) -> Result<LocalPage, ProviderError> {
    if !after.is_empty() && (!after.is_ascii() || after.len() > 128) {
        return Err(unavailable());
    }
    let mut query = db
        .prepare(&statement(
            &thread_columns(db)?,
            "WHERE id>?1 ORDER BY id LIMIT 129",
        ))
        .map_err(|_| unavailable())?;
    let edges = edge_columns(db);
    let mut rows = query.query([after]).map_err(|_| unavailable())?;
    let mut sessions = Vec::new();
    let mut bytes = 0;
    let mut count = 0;
    let mut cursor = None;
    let mut skipped = false;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        deadline(end)?;
        if count == 128 || bytes > 80 * 1024 {
            return Ok(LocalPage {
                sessions,
                next: cursor,
                revision,
                skipped,
            });
        }
        let id: String = row.get(0).map_err(|_| unavailable())?;
        if cursor.as_ref().is_some_and(|previous| previous >= &id) {
            return Err(unavailable());
        }
        if !id.is_ascii() || id.len() > 128 {
            return Err(unavailable());
        }
        cursor = Some(id);
        count += 1;
        if let Ok((_, session)) = local_row(db, root, row, edges) {
            bytes += serde_json::to_vec(&session)
                .map_err(|_| unavailable())?
                .len();
            sessions.push(session);
        } else {
            skipped = true;
        }
    }
    Ok(LocalPage {
        sessions,
        next: None,
        revision,
        skipped,
    })
}

#[cfg(target_os = "macos")]
fn exact(
    db: &Connection,
    root: &Path,
    id: &str,
) -> Result<(Candidate, LocalSession), ProviderError> {
    let mut query = db
        .prepare(&statement(&thread_columns(db)?, "WHERE id=?1 LIMIT 2"))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([id]).map_err(|_| unavailable())?;
    let result = local_row(
        db,
        root,
        rows.next().map_err(|_| unavailable())?.ok_or_else(stale)?,
        edge_columns(db),
    )?;
    if rows.next().map_err(|_| unavailable())?.is_some() {
        return Err(stale());
    }
    Ok(result)
}

#[cfg(target_os = "macos")]
pub(super) fn source(
    db: &Connection,
    root: &Path,
    id: &str,
) -> Result<ContextReference, ProviderError> {
    let (candidate, session) = exact(db, root, id)?;
    let (file, _) = open_transcript(&candidate, root)?;
    let meta = file.metadata().map_err(|_| unavailable())?;
    Ok(ContextReference {
        decoder_progress: None,
        local_id: session.metadata.identity.local_id,
        native_session_id: session.metadata.native_session_id,
        record_index: 1,
        position: None,
        source_fingerprint: hash(stamp(&meta).as_bytes()),
        source_bytes: usize::try_from(meta.len()).map_err(|_| unavailable())?,
        purpose: SearchPurpose::Diagnostic,
        binding_revision: String::new(),
    })
}

#[cfg(target_os = "macos")]
pub(super) fn removal(
    db: &Connection,
    root: &Path,
    id: &str,
    end: Instant,
) -> Result<LocalRemovalEvidence, ProviderError> {
    let (candidate, session) = exact(db, root, id)?;
    if session.metadata.state != SessionState::Archived || !edge_columns(db) {
        return Err(unavailable());
    }
    // Inspect the actual spawn table, including children without a thread row.
    let has_children = db
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM thread_spawn_edges WHERE parent_thread_id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|_| unavailable())?;
    if has_children {
        return Err(unavailable());
    }
    let (file, path) = open_transcript(&candidate, root)?;
    let suffix = format!("-{id}.jsonl");
    if candidate.path.as_ref() != Some(&path)
        || !path.starts_with(root.join("archived_sessions"))
        || !path
            .file_name()
            .and_then(|part| part.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(&suffix))
    {
        return Err(unavailable());
    }
    // An exact known second representation could be removed by the native
    // command as well. It has no approved footprint in this preview.
    match filesystem::open_source_read(root, &other_representation(&path)) {
        Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {}
        _ => return Err(unavailable()),
    }
    let db_file = filesystem::open_source_read(root, &root.join("state_5.sqlite"))?;
    let db_meta = db_file.metadata().map_err(|_| unavailable())?;
    use std::os::unix::fs::MetadataExt;
    let file_stamp = stamp(&file.metadata().map_err(|_| unavailable())?);
    read_rollout_header(&file, false, id, end, &mut 0)?;
    let current = filesystem::open_source_read(root, &path)?;
    if stamp(&current.metadata().map_err(|_| unavailable())?) != file_stamp {
        return Err(stale());
    }
    let revision = hash(
        &serde_json::to_vec(&(&path, file_stamp, db_meta.dev(), db_meta.ino()))
            .map_err(|_| unavailable())?,
    );
    Ok(LocalRemovalEvidence { session, revision })
}
