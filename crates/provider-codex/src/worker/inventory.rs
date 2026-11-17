//! Fixed-column views of the snapshot. No request supplies SQL or a filesystem path.
use super::*;
use crate::reader::{self, ObservedSource};
use agentmender_domain::{
    SessionKind, SessionState, canonical_uuid, observed_path_is_absolute, safe_conversation_text,
};
use rusqlite::{Connection as Sqlite, Row};
use std::{collections::BTreeMap, path::Path, time::Instant};

pub(super) struct Candidate {
    pub session: Session,
    pub locator: Option<PathBuf>,
    pub relations: Relationships,
}

#[derive(Debug)]
pub(super) struct Relationships {
    pub source: ObservedSource,
    pub fork: Option<String>,
    pub attached: Option<String>,
    pub thread_source: Option<String>,
}

impl Relationships {
    pub fn from_row(row: &Row<'_>, offset: usize) -> Result<Self, ProviderError> {
        let text = |index: usize| -> Result<Option<String>, ProviderError> {
            let value: Option<String> = row.get(offset + index).map_err(|_| unavailable())?;
            if value.as_ref().is_some_and(|value| value.len() > 4096) {
                return Err(unavailable());
            }
            Ok(value)
        };
        let fork = text(1)?;
        let attached = text(2)?;
        if fork.iter().chain(&attached).any(|id| !canonical_uuid(id)) {
            return Err(unavailable());
        }
        Ok(Self {
            source: ObservedSource::from_database(text(0)?.as_deref())?,
            fork,
            attached,
            thread_source: text(3)?,
        })
    }
}

pub(super) fn columns(
    db: &Sqlite,
    table: &'static str,
) -> Result<BTreeMap<String, String>, ProviderError> {
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

pub(super) fn is_table(db: &Sqlite, table: &str) -> bool {
    db.query_row(
        "SELECT type FROM sqlite_master WHERE name=?1",
        [table],
        |row| row.get::<_, String>(0),
    )
    .is_ok_and(|kind| kind == "table")
}

pub(super) fn has_edges(db: &Sqlite) -> bool {
    is_table(db, "thread_spawn_edges")
        && columns(db, "thread_spawn_edges").is_ok_and(|cols| {
            ["parent_thread_id", "child_thread_id"]
                .iter()
                .all(|name| cols.get(*name).is_some_and(|ty| ty == "TEXT"))
        })
}

pub(super) struct Layout {
    pub projection: String,
    pub relations: String,
    pub metadata_only: bool,
    pub edges: bool,
}

pub(super) fn layout(db: &Sqlite) -> Result<Layout, ProviderError> {
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
    let fields = [
        "source",
        "forked_from_id",
        "attached_thread_parent_id",
        "thread_source",
    ];
    if fields
        .iter()
        .any(|name| cols.contains_key(*name) && !has_text(name))
    {
        return Err(unavailable());
    }
    let relations = fields.map(optional).join(",");
    Ok(Layout {
        projection: format!(
            "SELECT id,{title},cwd,created_at,updated_at,archived,{locator},{relations} FROM threads"
        ),
        relations,
        metadata_only: has_text("forked_from_id") && has_text("attached_thread_parent_id"),
        edges: has_edges(db),
    })
}

fn identity(root: &Path, native_id: String) -> SessionIdentity {
    SessionIdentity {
        provider: ProviderKind::Codex,
        state_root: root.into(),
        namespace: None,
        native_id,
    }
}

fn incoming(db: &Sqlite, id: &str) -> Result<Option<String>, ProviderError> {
    // Treat a duplicate, noncanonical or ill-typed inbound edge as ambiguity,
    // even if its parent does not have a row in this snapshot.
    let mut query = db
        .prepare("SELECT parent_thread_id,child_thread_id FROM thread_spawn_edges WHERE child_thread_id=?1 COLLATE NOCASE LIMIT 2")
        .map_err(|_| unavailable())?;
    let mut rows = query.query([id]).map_err(|_| unavailable())?;
    let Some(row) = rows.next().map_err(|_| unavailable())? else {
        return Ok(None);
    };
    let parent: String = row.get(0).map_err(|_| unavailable())?;
    let child: String = row.get(1).map_err(|_| unavailable())?;
    if rows.next().map_err(|_| unavailable())?.is_some()
        || child != id
        || parent == id
        || !canonical_uuid(&parent)
    {
        return Err(unavailable());
    }
    Ok(Some(parent))
}

pub(super) fn row(
    db: &Sqlite,
    root: &Path,
    row: &Row<'_>,
    layout: &Layout,
    end: Instant,
) -> Result<Candidate, ProviderError> {
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
    let locator: Option<String> = row.get(6).map_err(|_| unavailable())?;
    if locator.as_ref().is_some_and(|path| path.len() > 4096) {
        return Err(unavailable());
    }
    let locator = locator.map(PathBuf::from);
    let relations = Relationships::from_row(row, 7)?;
    let incoming = if layout.edges {
        incoming(db, &id)?
    } else {
        None
    };
    let parent = match (&relations.source, incoming) {
        (ObservedSource::Spawned(source), Some(edge)) if *source != edge => {
            return Err(unavailable());
        }
        (ObservedSource::Spawned(source), None) => Some(source.clone()),
        (_, edge) => edge.or_else(|| relations.attached.clone()),
    };
    let transcript = reader::open_transcript(root, locator.as_deref()).ok();
    let forked_from = relations.fork.clone();
    let kind = if parent.is_some() {
        SessionKind::Subagent
    } else if forked_from.is_some() {
        SessionKind::Fork
    } else if layout.edges
        && relations.source == ObservedSource::Main
        && relations
            .thread_source
            .as_deref()
            .is_none_or(|source| source == "user")
    {
        match transcript.as_ref() {
            Some((file, path))
                if reader::relationship_header(file, path, &id, end)
                    .is_ok_and(|meta| meta.is_main(&id)) =>
            {
                SessionKind::Main
            }
            None if locator.is_none() && layout.metadata_only => SessionKind::Main,
            _ => SessionKind::Unknown,
        }
    } else {
        SessionKind::Unknown
    };
    let transcript_bytes = transcript
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
            parent: parent.map(|parent| identity(root, parent)),
        },
        locator,
        relations,
    })
}

pub(super) fn exact(
    db: &Sqlite,
    root: &Path,
    id: &str,
    end: Instant,
) -> Result<Candidate, ProviderError> {
    if !canonical_uuid(id) {
        return Err(stale());
    }
    let layout = layout(db)?;
    let mut query = db
        .prepare(&format!(
            "{} WHERE id=?1 COLLATE NOCASE LIMIT 2",
            layout.projection
        ))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([id]).map_err(|_| unavailable())?;
    let candidate = row(
        db,
        root,
        rows.next().map_err(|_| unavailable())?.ok_or_else(stale)?,
        &layout,
        end,
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
    let layout = layout(db)?;
    let mut query = db
        .prepare(&format!(
            "{} WHERE id>?1 ORDER BY id LIMIT 129",
            layout.projection
        ))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([after]).map_err(|_| unavailable())?;

    let mut sessions = Vec::new();
    let mut bytes = 0;
    let mut count = 0;
    let mut cursor = None;
    let mut incomplete = false;
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
                incomplete,
            });
        }
        let id: String = record.get(0).map_err(|_| unavailable())?;
        if cursor.as_ref().is_some_and(|previous| previous >= &id)
            || !id.is_ascii()
            || id.len() > 128
        {
            return Err(unavailable());
        }
        let duplicate = db
            .prepare("SELECT id FROM threads WHERE id=?1 COLLATE NOCASE LIMIT 1 OFFSET 1")
            .and_then(|mut statement| statement.exists([&id]))
            .map_err(|_| unavailable())?;
        if !duplicate && let Ok(candidate) = row(db, root, record, &layout, end) {
            let size = serde_json::to_vec(&candidate.session)
                .map_err(|_| unavailable())?
                .len();
            if bytes + size > 80 * 1024 && count != 0 {
                return Ok(InventoryPage {
                    sessions,
                    next: cursor,
                    revision,
                    examined: count,
                    incomplete,
                });
            }
            incomplete |= candidate.session.kind == SessionKind::Unknown;
            bytes += size;
            sessions.push(candidate.session);
        } else {
            incomplete = true;
        }
        cursor = Some(id);
        count += 1;
    }
    Ok(InventoryPage {
        sessions,
        next: None,
        revision,
        examined: count,
        incomplete,
    })
}
