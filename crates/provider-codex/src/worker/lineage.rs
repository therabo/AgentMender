//! Verified lineage and presence evidence from the confined archive reader.
use super::{LineageFact, unavailable};
use crate::reader::{self, ObservedSource};
use agentmender_domain::canonical_uuid;
use agentmender_provider_api::{ProviderError, filesystem, rewrite::directory};
use rusqlite::Connection;
use rustix::fs::{self, AtFlags, Dir, FileType};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;

fn native_id(id: &str) -> Option<Uuid> {
    canonical_uuid(id).then(|| Uuid::parse_str(id).expect("canonical UUID"))
}

pub(super) fn read(db: &Connection, root: &Path) -> Result<Vec<LineageFact>, ProviderError> {
    let layout = super::inventory::layout(db)?;
    let mut facts = Vec::new();
    let mut visited = BTreeSet::new();
    let mut table = db.prepare(&layout.projection).map_err(|_| unavailable())?;
    let mut rows = table.query([]).map_err(|_| unavailable())?;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        let Some(id) = row.get::<_, String>(0).ok().and_then(|id| native_id(&id)) else {
            continue;
        };
        let archived = row.get::<_, i64>(5).is_ok_and(|value| value == 1);
        facts.push(LineageFact::Present { id, archived });
        match super::inventory::Relationships::from_row(row, 7) {
            Ok(relations) => {
                if let ObservedSource::Spawned(parent) = relations.source {
                    edge(&mut facts, &parent, id);
                }
                if let Some(parent) = relations.attached {
                    edge(&mut facts, &parent, id);
                }
            }
            Err(_) => facts.push(LineageFact::Uncertain(id)),
        }
        match row.get::<_, Option<String>>(6) {
            Ok(Some(locator)) => match reader::open_transcript(root, Some(Path::new(&locator))) {
                Ok((file, path)) => {
                    visited.insert(path.clone());
                    record_header(
                        &mut facts,
                        &file,
                        &path,
                        Some(id),
                        path.starts_with(root.join("archived_sessions")),
                    );
                }
                Err(ProviderError::CapabilityUnavailable(reason))
                    if reason == "source file missing" => {}
                Err(_) => facts.push(LineageFact::Uncertain(id)),
            },
            Ok(None) => {}
            Err(_) => facts.push(LineageFact::Uncertain(id)),
        }
    }

    if layout.edges {
        let mut table = db
            .prepare("SELECT parent_thread_id, child_thread_id FROM thread_spawn_edges")
            .map_err(|_| unavailable())?;
        let mut rows = table.query([]).map_err(|_| unavailable())?;
        while let Some(row) = rows.next().map_err(|_| unavailable())? {
            if let Some(child) = row.get::<_, String>(1).ok().and_then(|id| native_id(&id)) {
                if let Ok(parent) = row.get::<_, String>(0) {
                    edge(&mut facts, &parent, child);
                } else {
                    facts.push(LineageFact::Uncertain(child));
                }
            }
        }
    } else if super::inventory::is_table(db, "thread_spawn_edges") {
        return Err(unavailable());
    }

    for (folder, archived) in [("sessions", false), ("archived_sessions", true)] {
        scan_rollouts(root, root.join(folder), archived, &visited, &mut facts)
            .map_err(|_| unavailable())?;
    }
    Ok(facts)
}

fn edge(facts: &mut Vec<LineageFact>, parent: &str, child: Uuid) {
    facts.push(match native_id(parent) {
        Some(parent) => LineageFact::Edge { parent, child },
        None => LineageFact::Uncertain(child),
    });
}

fn record_header(
    facts: &mut Vec<LineageFact>,
    file: &std::fs::File,
    path: &Path,
    expected: Option<Uuid>,
    archived: bool,
) {
    let end = Instant::now() + Duration::from_secs(2);
    let id_text = expected.map(|id| id.to_string());
    let Ok(meta) = reader::relationship_header(file, path, id_text.as_deref(), end) else {
        if let Some(id) = expected {
            facts.push(LineageFact::Uncertain(id));
        }
        return;
    };
    let id = native_id(&meta.id).expect("verified rollout UUID");
    facts.push(LineageFact::Present { id, archived });
    match meta.source() {
        Ok(ObservedSource::Spawned(parent)) => edge(facts, &parent, id),
        Ok(_) => {}
        Err(_) => facts.push(LineageFact::Uncertain(id)),
    }
    if let Some(parent) = meta.parent_thread_id {
        edge(facts, &parent, id);
    }
    if meta.attached_thread
        && let Some(parent) = meta.forked_from_id
    {
        edge(facts, &parent, id);
    }
}

fn rollout_id(name: &OsStr) -> Option<Uuid> {
    let name = name.to_str()?;
    let stem = name
        .strip_suffix(".jsonl.zst")
        .or_else(|| name.strip_suffix(".jsonl"))?;
    let (prefix, id) = stem.split_at_checked(stem.len().checked_sub(36)?)?;
    if prefix.is_empty() || prefix.starts_with("rollout-") {
        native_id(id)
    } else {
        None
    }
}

fn scan_rollouts(
    root: &Path,
    folder: PathBuf,
    archived: bool,
    visited: &BTreeSet<PathBuf>,
    facts: &mut Vec<LineageFact>,
) -> io::Result<()> {
    let device = fs::fstat(directory(root)?)?.st_dev;
    let mut pending = vec![folder];
    while let Some(path) = pending.pop() {
        let handle = match directory(&path) {
            Ok(dir) => dir,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if fs::fstat(&handle)?.st_dev != device {
            continue;
        }
        for entry in Dir::read_from(&handle)? {
            let entry = entry?;
            let name = OsStr::from_bytes(entry.file_name().to_bytes());
            if name == "." || name == ".." {
                continue;
            }
            let child = path.join(name);
            let info = match fs::statat(&handle, name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(info) => info,
                Err(rustix::io::Errno::NOENT) => continue,
                Err(error) => return Err(error.into()),
            };
            if FileType::from_raw_mode(info.st_mode) == FileType::Directory {
                if info.st_dev == device {
                    pending.push(child);
                }
            } else if !visited.contains(&child)
                && (child.extension().is_some_and(|ext| ext == "jsonl")
                    || reader::compressed_path(&child))
            {
                let expected = rollout_id(name);
                match filesystem::open_source_read(root, &child) {
                    Ok(file) => record_header(facts, &file, &child, expected, archived),
                    Err(_) => {
                        if let Some(id) = expected {
                            facts.push(LineageFact::Uncertain(id));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
