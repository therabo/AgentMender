//! Fixed-column views of the snapshot. No request supplies SQL or a filesystem path.
use super::*;
use crate::reader;
use agentmender_domain::{
    SessionKind, SessionState, canonical_uuid, observed_path_is_absolute, safe_conversation_text,
};
use agentmender_provider_api::filesystem;
use rusqlite::{Connection as Sqlite, Row};
use std::{collections::BTreeMap, path::Path, time::Instant};

pub(super) struct Candidate {
    pub session: Session,
    pub locator: Option<PathBuf>,
}

fn columns(db: &Sqlite, table: &'static str) -> Result<BTreeMap<String, String>, ProviderError> {
    let sql = match table {
        "threads" => "PRAGMA table_info(threads)",
        "thread_spawn_edges" => "PRAGMA table_info(thread_spawn_edges)",
        _ => unreachable!("fixed table names"),
    };
    let mut statement = db.prepare(sql).map_err(|_| unavailable())?;
    let columns = statement
        .query_map([], |row| {
            Ok((row.get(1)?, row.get::<_, String>(2)?.to_ascii_uppercase()))
        })
        .map_err(|_| unavailable())?
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|_| unavailable())?;
    if columns.len() > 128 {
        return Err(unavailable());
    }
    Ok(columns)
}

fn is_table(db: &Sqlite, table: &str) -> bool {
    db.query_row(
        "SELECT type FROM sqlite_master WHERE name=?1",
        [table],
        |row| row.get::<_, String>(0),
    )
    .is_ok_and(|kind| kind == "table")
}

fn has_edges(db: &Sqlite) -> bool {
    is_table(db, "thread_spawn_edges")
        && columns(db, "thread_spawn_edges").is_ok_and(|cols| {
            ["parent_thread_id", "child_thread_id"]
                .iter()
                .all(|name| cols.get(*name).is_some_and(|ty| ty == "TEXT"))
        })
}

fn projection(db: &Sqlite) -> Result<String, ProviderError> {
    if !is_table(db, "threads") {
        return Err(unavailable());
    }
    let cols = columns(db, "threads")?;
    for (name, kind) in [
        ("id", "TEXT"),
        ("title", "TEXT"),
        ("cwd", "TEXT"),
        ("created_at", "INTEGER"),
        ("updated_at", "INTEGER"),
        ("archived", "INTEGER"),
    ] {
        if cols.get(name).map(String::as_str) != Some(kind) {
            return Err(unavailable());
        }
    }
    let has_text = |column| cols.get(column).is_some_and(|kind| kind == "TEXT");
    let optional = |column: &'static str| if has_text(column) { column } else { "NULL" };
    let title = if has_text("name") {
        "COALESCE(NULLIF(name,''),title)"
    } else {
        "title"
    };
    let locator = optional("rollout_path");
    let source = optional("source");
    let fork = optional("forked_from_id");
    Ok(format!(
        "SELECT id,{title},cwd,created_at,updated_at,archived,{locator},{source},{fork} FROM threads"
    ))
}

fn identity(root: &Path, native_id: String) -> SessionIdentity {
    SessionIdentity {
        provider: ProviderKind::Codex,
        state_root: root.into(),
        namespace: None,
        native_id,
    }
}

fn parent(db: &Sqlite, root: &Path, id: &str) -> Result<Option<SessionIdentity>, ProviderError> {
    let mut query = db
        .prepare("SELECT parent_thread_id FROM thread_spawn_edges WHERE child_thread_id=?1 LIMIT 2")
        .map_err(|_| unavailable())?;
    let mut rows = query.query([id]).map_err(|_| unavailable())?;
    let value = rows
        .next()
        .map_err(|_| unavailable())?
        .map(|row| row.get::<_, String>(0))
        .transpose()
        .map_err(|_| unavailable())?;
    if rows.next().map_err(|_| unavailable())?.is_some() {
        return Ok(None);
    }
    Ok(value
        .filter(|id| canonical_uuid(id))
        .map(|id| identity(root, id)))
}

fn row(db: &Sqlite, root: &Path, row: &Row<'_>, edges: bool) -> Result<Candidate, ProviderError> {
    let id: String = row.get(0).map_err(|_| unavailable())?;
    if !canonical_uuid(&id) {
        return Err(unavailable());
    }
    let text = |index| {
        row.get::<_, Option<String>>(index)
            .map_err(|_| unavailable())
            .map(|value| value.filter(|text| text.len() <= 4096))
    };
    let datetime = |index| {
        row.get::<_, Option<i64>>(index)
            .map_err(|_| unavailable())
            .map(|value| value.filter(|time| *time >= 0))
    };
    let title = text(1)?.filter(|title| !title.trim().is_empty() && safe_conversation_text(title));
    let cwd = text(2)?;
    let project = cwd
        .as_deref()
        .and_then(|path| Path::new(path).file_name()?.to_str())
        .filter(|value| safe_conversation_text(value))
        .map(str::to_owned);
    let workspace = cwd.filter(|path| {
        observed_path_is_absolute(Path::new(path))
            && !path.chars().any(char::is_control)
            && safe_conversation_text(path)
    });
    let state = match row.get::<_, i64>(5).map_err(|_| unavailable())? {
        0 => SessionState::Active,
        1 => SessionState::Archived,
        _ => SessionState::Unknown,
    };
    let locator = text(6)?.map(PathBuf::from);
    let parent = if edges { parent(db, root, &id)? } else { None };
    let forked_from = text(8)?.filter(|id| canonical_uuid(id));
    let kind = if parent.is_some() {
        SessionKind::Subagent
    } else if forked_from.is_some() {
        SessionKind::Fork
    } else {
        match text(7)?.as_deref() {
            Some("cli" | "vscode" | "exec" | "appServer") => SessionKind::Main,
            _ => SessionKind::Unknown,
        }
    };
    let transcript_bytes = reader::open_transcript(root, locator.as_deref())
        .ok()
        .and_then(|(file, _)| file.metadata().ok())
        .map(|meta| meta.len());
    Ok(Candidate {
        session: Session {
            identity: identity(root, id),
            title,
            workspace,
            project,
            created_at: datetime(3)?,
            last_active_at: datetime(4)?,
            state,
            transcript_bytes,
            kind,
            forked_from,
            parent,
        },
        locator,
    })
}

pub(super) fn exact(db: &Sqlite, root: &Path, id: &str) -> Result<Candidate, ProviderError> {
    if !canonical_uuid(id) {
        return Err(stale());
    }
    let mut query = db
        .prepare(&format!("{} WHERE id=?1 LIMIT 2", projection(db)?))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([id]).map_err(|_| unavailable())?;
    let candidate = row(
        db,
        root,
        rows.next().map_err(|_| unavailable())?.ok_or_else(stale)?,
        has_edges(db),
    )?;
    if rows.next().map_err(|_| unavailable())?.is_some()
        || candidate.session.identity.native_id != id
    {
        return Err(stale());
    }
    Ok(candidate)
}

pub(super) fn page(
    db: &Sqlite,
    root: &Path,
    after: &str,
    limit: usize,
    revision: String,
    end: Instant,
) -> Result<InventoryPage, ProviderError> {
    if !after.is_ascii() || after.len() > 128 || !(1..=PAGE_ROWS).contains(&limit) {
        return Err(unavailable());
    }
    let mut query = db
        .prepare(&format!(
            "{} WHERE id>?1 ORDER BY id LIMIT 129",
            projection(db)?
        ))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([after]).map_err(|_| unavailable())?;
    let edges = has_edges(db);
    let mut sessions = Vec::new();
    let mut bytes = 0;
    let mut count = 0;
    let mut cursor = None;
    while let Some(record) = rows.next().map_err(|_| unavailable())? {
        if Instant::now() >= end {
            return Err(unavailable());
        }
        if count == limit {
            return Ok(InventoryPage {
                sessions,
                next: cursor,
                revision,
                examined: count,
            });
        }
        let id: String = record.get(0).map_err(|_| unavailable())?;
        if cursor.as_ref().is_some_and(|previous| previous >= &id)
            || !id.is_ascii()
            || id.len() > 128
        {
            return Err(unavailable());
        }
        if let Ok(candidate) = row(db, root, record, edges) {
            let size = serde_json::to_vec(&candidate.session)
                .map_err(|_| unavailable())?
                .len();
            if bytes + size > 80 * 1024 && count != 0 {
                return Ok(InventoryPage {
                    sessions,
                    next: cursor,
                    revision,
                    examined: count,
                });
            }
            bytes += size;
            sessions.push(candidate.session);
        }
        cursor = Some(id);
        count += 1;
    }
    Ok(InventoryPage {
        sessions,
        next: None,
        revision,
        examined: count,
    })
}

pub(super) fn removal(
    db: &Sqlite,
    root: &Path,
    identity: &SessionIdentity,
    end: Instant,
) -> Result<RemovalEvidence, ProviderError> {
    use std::os::unix::fs::MetadataExt;
    let id = &identity.native_id;
    let Candidate { session, locator } = exact(db, root, id)?;
    if &session.identity != identity || session.state != SessionState::Archived || !has_edges(db) {
        return Err(unavailable());
    }
    // The spawn table covers children that do not have a canonical thread row.
    let has_children = db
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM thread_spawn_edges WHERE parent_thread_id=?1)",
            [id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|_| unavailable())?;
    if has_children {
        return Err(unavailable());
    }
    let (file, path) = reader::open_transcript(root, locator.as_deref())?;
    let suffix = format!("-{id}.jsonl");
    if locator.as_ref() != Some(&path)
        || !path.starts_with(root.join("archived_sessions"))
        || !path
            .file_name()
            .and_then(|part| part.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(&suffix))
    {
        return Err(unavailable());
    }
    match filesystem::open_source_read(root, &reader::other_representation(&path)) {
        Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {}
        _ => return Err(unavailable()),
    }
    let db_meta = filesystem::open_source_read(root, &root.join("state_5.sqlite"))?
        .metadata()
        .map_err(|_| unavailable())?;
    let stamp = reader::stamp(&file.metadata().map_err(|_| unavailable())?);
    reader::header(&file, false, id, end)?;
    let current = filesystem::open_source_read(root, &path)?;
    if reader::stamp(&current.metadata().map_err(|_| unavailable())?) != stamp {
        return Err(stale());
    }
    let revision = reader::hash(
        &serde_json::to_vec(&(&path, stamp, db_meta.dev(), db_meta.ino()))
            .map_err(|_| unavailable())?,
    );
    Ok(RemovalEvidence { session, revision })
}
