//! The native deletion scope, built from the complete bounded provider observation.
use super::{inventory, *};
use crate::{
    reader::{self, ObservedSource, RelationshipMeta},
    removal::RemovalRefusal,
};
use agentmender_domain::{
    CodexMembership, ProjectIdentity, ProjectLocator, SessionKind, SessionState, canonical_uuid,
};
use agentmender_provider_api::filesystem;
use rusqlite::{Connection as Sqlite, types::ValueRef};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::fs::MetadataExt,
    path::Path,
    time::Instant,
};

const MAX_RECORDS: usize = 16_384;
const FOLDERS: &[&str] = &["sessions", "archived_sessions"];

fn check_size(count: usize, limit: usize, end: Instant) -> Result<(), ProviderError> {
    if count >= limit || Instant::now() >= end {
        return Err(unavailable());
    }
    Ok(())
}

fn text_id(id: String) -> Result<String, ProviderError> {
    canonical_uuid(&id).then_some(id).ok_or_else(unavailable)
}

#[derive(Serialize)]
struct Edge {
    parent: String,
    status: String,
}

struct Graph {
    inbound: BTreeMap<String, Edge>,
    outbound: BTreeMap<String, Vec<String>>,
}

impl Graph {
    fn read(db: &Sqlite, end: Instant) -> Result<Self, ProviderError> {
        let mut graph = Self {
            inbound: BTreeMap::new(),
            outbound: BTreeMap::new(),
        };
        let mut query = db.prepare(
            "SELECT parent_thread_id,child_thread_id,status FROM thread_spawn_edges ORDER BY child_thread_id",
        ).map_err(|_| unavailable())?;
        let mut rows = query.query([]).map_err(|_| unavailable())?;
        while let Some(row) = rows.next().map_err(|_| unavailable())? {
            check_size(graph.inbound.len(), MAX_RECORDS, end)?;
            let parent = text_id(row.get(0).map_err(|_| unavailable())?)?;
            let child = text_id(row.get(1).map_err(|_| unavailable())?)?;
            let status: String = row.get(2).map_err(|_| unavailable())?;
            if status.len() > 32 || parent == child || graph.inbound.contains_key(&child) {
                return Err(unavailable());
            }
            graph
                .outbound
                .entry(parent.clone())
                .or_default()
                .push(child.clone());
            graph.inbound.insert(child, Edge { parent, status });
        }
        Ok(graph)
    }

    fn subtree(&self, main: &str, end: Instant) -> Result<BTreeSet<String>, RemovalRefusal> {
        if self.inbound.contains_key(main) {
            return Err(RemovalRefusal::IncompleteRelationships);
        }
        let mut pending = vec![main];
        let mut ids = BTreeSet::new();
        while let Some(id) = pending.pop() {
            check_size(ids.len(), MAX_SCOPE_MEMBERS, end)?;
            if !ids.insert(id.to_owned()) {
                return Err(RemovalRefusal::IncompleteRelationships);
            }
            if self
                .inbound
                .get(id)
                .is_some_and(|edge| edge.status != "closed")
            {
                return Err(RemovalRefusal::UnsafeMember);
            }
            if let Some(children) = self.outbound.get(id) {
                pending.extend(children.iter().map(String::as_str));
            }
        }
        Ok(ids)
    }
}

struct Rollout {
    path: PathBuf,
    rollout_id: String,
    stamp: String,
    meta: RelationshipMeta,
    source: ObservedSource,
}

fn rollout_index(root: &Path, end: Instant) -> Result<BTreeMap<String, Rollout>, ProviderError> {
    let files = filesystem::strict_source_files(root, FOLDERS, MAX_RECORDS, end)?;
    let mut index = BTreeMap::new();
    let mut rollout_ids = BTreeSet::new();
    for path in &files {
        check_size(index.len(), MAX_RECORDS, end)?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(unavailable)?;
        let Some(name) = name.strip_prefix("rollout-") else {
            continue;
        };
        let name = name.strip_suffix(".zst").unwrap_or(name);
        let Some(stem) = name.strip_suffix(".jsonl") else {
            continue;
        };
        let timestamp = stem.as_bytes().get(..19).ok_or_else(unavailable)?;
        if !timestamp
            .iter()
            .zip(b"0000-00-00T00-00-00")
            .all(|(byte, marker)| {
                if *marker == b'0' {
                    byte.is_ascii_digit()
                } else {
                    byte == marker
                }
            })
            || stem.as_bytes().get(19) != Some(&b'-')
        {
            return Err(unavailable());
        }
        let ids = stem.get(20..).ok_or_else(unavailable)?;
        let (id, rollout_id) = ids.split_once('_').unwrap_or((ids, ids));
        if !canonical_uuid(id)
            || !canonical_uuid(rollout_id)
            || index.contains_key(id)
            || !rollout_ids.insert(rollout_id.to_owned())
        {
            return Err(unavailable());
        }
        let file = filesystem::open_source_read(root, path)?;
        let stamp = reader::stamp(&file.metadata().map_err(|_| unavailable())?);
        let meta = reader::relationship_header(&file, path, id, end)?;
        let source = meta.source()?;
        index.insert(
            id.to_owned(),
            Rollout {
                path: path.clone(),
                rollout_id: rollout_id.into(),
                stamp,
                meta,
                source,
            },
        );
    }
    if filesystem::strict_source_files(root, FOLDERS, MAX_RECORDS, end)? != files {
        return Err(stale());
    }
    for rollout in index.values() {
        check_size(0, 1, end)?;
        let file = filesystem::open_source_read(root, &rollout.path)?;
        if reader::stamp(&file.metadata().map_err(|_| unavailable())?) != rollout.stamp {
            return Err(stale());
        }
    }
    Ok(index)
}

fn db_identity(root: &Path) -> Result<(u64, u64), ProviderError> {
    let meta = filesystem::open_source_read(root, &root.join("state_5.sqlite"))?
        .metadata()
        .map_err(|_| unavailable())?;
    Ok((meta.dev(), meta.ino()))
}

fn supported_layout(db: &Sqlite) -> Result<inventory::Layout, RemovalRefusal> {
    // Both exercised binaries use the same state_5 generation and migration
    // boundary. Unknown migrations may change what the native command reads.
    let layout = inventory::layout(db).map_err(|_| RemovalRefusal::UnsupportedStorage)?;
    let threads = inventory::columns(db, "threads")?;
    let edges = inventory::columns(db, "thread_spawn_edges")?;
    let known = layout.edges
        && ["rollout_path", "source", "thread_source", "attached_thread_parent_id"]
            .iter().all(|name| threads.get(*name).is_some_and(|ty| ty == "TEXT"))
        && edges.get("status").is_some_and(|ty| ty == "TEXT")
        && inventory::is_table(db, "_sqlx_migrations")
        && db.query_row(
            "SELECT COUNT(*)=62 AND MIN(version)=1 AND MAX(version)=62 AND SUM(success=1)=62 FROM _sqlx_migrations",
            [], |row| row.get::<_, bool>(0)
        ).unwrap_or(false)
        && expected_triggers(db)?;
    if !known {
        return Err(RemovalRefusal::UnsupportedStorage);
    }
    Ok(layout)
}

fn expected_triggers(db: &Sqlite) -> Result<bool, ProviderError> {
    let mut allowed = BTreeSet::new();
    for field in ["created_at", "updated_at"] {
        allowed.insert(format!(
            "CREATE TRIGGER threads_{field}_ms_after_insert AFTER INSERT ON threads WHEN NEW.{field}_ms IS NULL BEGIN UPDATE threads SET {field}_ms = NEW.{field} * 1000 WHERE id = NEW.id; END"
        ));
        allowed.insert(format!(
            "CREATE TRIGGER threads_{field}_ms_after_update AFTER UPDATE OF {field} ON threads WHEN NEW.{field} != OLD.{field} AND NEW.{field}_ms IS OLD.{field}_ms BEGIN UPDATE threads SET {field}_ms = NEW.{field} * 1000 WHERE id = NEW.id; END"
        ));
    }
    allowed.insert("CREATE TRIGGER threads_recency_at_after_insert AFTER INSERT ON threads WHEN NEW.recency_at_ms = 0 BEGIN UPDATE threads SET recency_at = NEW.updated_at, recency_at_ms = COALESCE(NEW.updated_at_ms, NEW.updated_at * 1000) WHERE id = NEW.id; END".into());
    let mut query = db
        .prepare("SELECT sql FROM sqlite_master WHERE type='trigger' LIMIT 6")
        .map_err(|_| unavailable())?;
    let mut rows = query.query([]).map_err(|_| unavailable())?;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        let sql: String = row.get(0).map_err(|_| unavailable())?;
        if !allowed.remove(&sql.split_ascii_whitespace().collect::<Vec<_>>().join(" ")) {
            return Ok(false);
        }
    }
    Ok(allowed.is_empty())
}

fn check_spawn_reference(
    graph: &Graph,
    ids: &BTreeSet<String>,
    id: &str,
    parent: &str,
) -> Result<(), RemovalRefusal> {
    if (ids.contains(id) || ids.contains(parent))
        && graph
            .inbound
            .get(id)
            .is_none_or(|edge| edge.parent != parent)
    {
        return Err(RemovalRefusal::IncompleteRelationships);
    }
    Ok(())
}

fn check_references(
    db: &Sqlite,
    layout: &inventory::Layout,
    graph: &Graph,
    rollouts: &BTreeMap<String, Rollout>,
    ids: &BTreeSet<String>,
    end: Instant,
) -> Result<BTreeSet<String>, RemovalRefusal> {
    let mut statement = db
        .prepare(&format!(
            "SELECT id,rollout_path,{} FROM threads ORDER BY id",
            layout.relations
        ))
        .map_err(|_| unavailable())?;
    let mut rows = statement.query([]).map_err(|_| unavailable())?;
    let mut canonical_rows = BTreeSet::new();
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        check_size(canonical_rows.len(), MAX_RECORDS, end)?;
        let id = text_id(row.get(0).map_err(|_| unavailable())?)?;
        let relations = inventory::Relationships::from_row(row, 2)?;
        if relations
            .attached
            .as_ref()
            .is_some_and(|parent| ids.contains(parent))
        {
            return Err(RemovalRefusal::AttachedThread);
        }
        if !ids.contains(&id)
            && relations
                .fork
                .as_ref()
                .is_some_and(|parent| ids.contains(parent))
        {
            return Err(RemovalRefusal::ExternalFork);
        }
        if let ObservedSource::Spawned(parent) = &relations.source {
            check_spawn_reference(graph, ids, &id, parent)?;
        }
        let locator: String = row.get(1).map_err(|_| unavailable())?;
        if rollouts
            .get(&id)
            .is_none_or(|rollout| rollout.path != Path::new(&locator))
        {
            return Err(RemovalRefusal::IncompleteRelationships);
        }
        if !canonical_rows.insert(id) {
            return Err(RemovalRefusal::IncompleteRelationships);
        }
    }

    let selected_rollouts: BTreeSet<_> = rollouts
        .iter()
        .filter(|(id, _)| ids.contains(*id))
        .map(|(_, rollout)| &rollout.rollout_id)
        .collect();
    for (id, rollout) in rollouts {
        let meta = &rollout.meta;
        let fork_of_member = meta
            .forked_from_id
            .as_ref()
            .is_some_and(|parent| ids.contains(parent));
        if meta.attached_thread && fork_of_member {
            return Err(RemovalRefusal::AttachedThread);
        }
        if !ids.contains(id)
            && (fork_of_member
                || meta.history_base.as_ref().is_some_and(|base| {
                    selected_rollouts.contains(&base.thread_id) || ids.contains(&base.thread_id)
                }))
        {
            return Err(RemovalRefusal::ExternalFork);
        }
        if let ObservedSource::Spawned(parent) = &rollout.source {
            check_spawn_reference(graph, ids, id, parent)?;
        }
        if let Some(parent) = &meta.parent_thread_id {
            check_spawn_reference(graph, ids, id, parent)?;
        }
        if !ids.contains(id)
            && meta
                .session_id
                .as_ref()
                .is_some_and(|root| ids.contains(root))
        {
            return Err(RemovalRefusal::IncompleteRelationships);
        }
    }
    Ok(canonical_rows)
}

fn unrelated_revision(
    db: &Sqlite,
    graph: &Graph,
    rollouts: &BTreeMap<String, Rollout>,
    ids: &BTreeSet<String>,
    end: Instant,
) -> Result<String, ProviderError> {
    let mut hash = Sha256::new();
    // Hash each SQLite value with its type and byte length. No row values leave
    // the reader and native bookkeeping may still change other provider tables.
    let mut statement = db
        .prepare("SELECT * FROM threads ORDER BY id")
        .map_err(|_| unavailable())?;
    let id_column = statement.column_index("id").map_err(|_| unavailable())?;
    let columns = statement.column_count();
    let mut rows = statement.query([]).map_err(|_| unavailable())?;
    let mut count = 0;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        check_size(count, MAX_RECORDS, end)?;
        count += 1;
        let id: String = row.get(id_column).map_err(|_| unavailable())?;
        if ids.contains(&id) {
            continue;
        }
        hash.update(b"row");
        for index in 0..columns {
            match row.get_ref(index).map_err(|_| unavailable())? {
                ValueRef::Null => hash.update(b"n"),
                ValueRef::Integer(value) => {
                    hash.update(b"i");
                    hash.update(value.to_le_bytes());
                }
                ValueRef::Real(value) => {
                    hash.update(b"r");
                    hash.update(value.to_le_bytes());
                }
                ValueRef::Text(value) | ValueRef::Blob(value) => {
                    hash.update(if matches!(row.get_ref(index), Ok(ValueRef::Text(_))) {
                        b"t"
                    } else {
                        b"b"
                    });
                    hash.update(value.len().to_le_bytes());
                    hash.update(value);
                }
            }
        }
    }
    for (child, edge) in &graph.inbound {
        if !ids.contains(child) && !ids.contains(&edge.parent) {
            hash.update(serde_json::to_vec(&(child, edge)).map_err(|_| unavailable())?);
        }
    }
    for (id, rollout) in rollouts {
        if !ids.contains(id) {
            hash.update(
                serde_json::to_vec(&(id, &rollout.path, &rollout.stamp))
                    .map_err(|_| unavailable())?,
            );
        }
    }
    Ok(reader::hash(&hash.finalize()))
}

pub(super) fn plan(
    db: &Sqlite,
    root: &Path,
    identity: &SessionIdentity,
    revision: String,
    end: Instant,
) -> Result<RemovalEvidence, RemovalRefusal> {
    let archive = RemovalArchive::read(db, root, end)?;
    archive.tree(identity, &revision)
}

struct RemovalArchive<'a> {
    db: &'a Sqlite,
    root: &'a Path,
    layout: inventory::Layout,
    graph: Graph,
    rollouts: BTreeMap<String, Rollout>,
    end: Instant,
}

impl<'a> RemovalArchive<'a> {
    fn read(db: &'a Sqlite, root: &'a Path, end: Instant) -> Result<Self, RemovalRefusal> {
        Ok(Self {
            db,
            root,
            layout: supported_layout(db)?,
            graph: Graph::read(db, end)?,
            rollouts: rollout_index(root, end)?,
            end,
        })
    }

    fn tree(
        &self,
        identity: &SessionIdentity,
        revision: &str,
    ) -> Result<RemovalEvidence, RemovalRefusal> {
        let Self {
            db,
            root,
            layout,
            graph,
            rollouts,
            end,
        } = self;
        let end = *end;
        let main = &identity.native_id;
        let ids = graph.subtree(main, end)?;
        let candidate = inventory::exact(db, root, main, end)?;
        if &candidate.session.identity != identity || !candidate.session.is_main() {
            return Err(RemovalRefusal::NotMain);
        }
        let canonical_rows = check_references(db, layout, graph, rollouts, &ids, end)?;

        let mut members = Vec::with_capacity(ids.len());
        for id in &ids {
            let rollout = rollouts.get(id);
            if canonical_rows.contains(id) {
                let canonical = inventory::exact(db, root, id, end)?;
                if canonical.session.state != SessionState::Archived {
                    return Err(RemovalRefusal::UnsafeMember);
                }
                if rollout.is_none_or(|rollout| canonical.locator.as_ref() != Some(&rollout.path))
                    || canonical.relations.attached.is_some()
                {
                    return Err(RemovalRefusal::IncompleteRelationships);
                }
                if id != main
                    && !matches!(&canonical.relations.source,
                ObservedSource::Spawned(parent) if parent == &graph.inbound[id].parent)
                {
                    return Err(RemovalRefusal::IncompleteRelationships);
                }
            }
            if let Some(rollout) = rollout {
                if !rollout.path.starts_with(root.join("archived_sessions"))
                    || reader::compressed_path(&rollout.path)
                    || rollout.rollout_id != *id
                    || rollout
                        .path
                        .to_str()
                        .is_none_or(|path| path.len() > 512 || path.chars().any(char::is_control))
                {
                    return Err(RemovalRefusal::UnsafeMember);
                }
                if id != main
                    && (!matches!(&rollout.source,
                ObservedSource::Spawned(parent) if parent == &graph.inbound[id].parent)
                        || rollout.meta.attached_thread
                        || rollout
                            .meta
                            .session_id
                            .as_ref()
                            .is_some_and(|root| root != id && root != main))
                {
                    return Err(RemovalRefusal::IncompleteRelationships);
                }
            }
            members.push(ReviewedMember {
                id: id.clone(),
                rollout: rollout.map(|rollout| rollout.path.clone()),
            });
        }
        let affected: Vec<_> = ids
            .iter()
            .map(|id| (id, rollouts.get(id).map(|r| &r.stamp)))
            .collect();
        let revision =
            reader::hash(&serde_json::to_vec(&(revision, affected)).map_err(|_| unavailable())?);
        let scope = RemovalScope {
            members,
            database_identity: db_identity(root)?,
            unrelated_revision: unrelated_revision(db, graph, rollouts, &ids, end)?,
        };
        if serde_json::to_vec(&scope).map_err(|_| unavailable())?.len() > 48 * 1024 {
            return Err(RemovalRefusal::IncompleteRelationships);
        }
        Ok(RemovalEvidence {
            session: candidate.session,
            revision,
            scope,
        })
    }
}

pub(super) fn project_plan(
    db: &Sqlite,
    root: &Path,
    project: &ProjectIdentity,
    revision: String,
    end: Instant,
) -> Result<ProjectEvidence, RemovalRefusal> {
    let ProjectLocator::Codex(project_id) = &project.locator else {
        return Err(RemovalRefusal::NotMain);
    };
    if project.provider != ProviderKind::Codex
        || project.state_root != root
        || !canonical_uuid(project_id)
    {
        return Err(RemovalRefusal::Changed);
    }
    let archive = RemovalArchive::read(db, root, end)?;
    let catalog = super::projects::read(db, root, &archive.layout, revision.clone());
    if catalog.incomplete {
        return Err(RemovalRefusal::IncompleteRelationships);
    }
    let registration = catalog
        .projects
        .into_iter()
        .find(|item| item.identity == *project)
        .ok_or(RemovalRefusal::Changed)?;
    if registration.desktop_backed {
        return Err(RemovalRefusal::IncompleteRelationships);
    }
    let empty = BTreeSet::new();
    let canonical = check_references(
        db,
        &archive.layout,
        &archive.graph,
        &archive.rollouts,
        &empty,
        end,
    )?;
    let mut query = db
        .prepare(&format!("{} ORDER BY id", archive.layout.projection))
        .map_err(|_| unavailable())?;
    let mut rows = query.query([]).map_err(|_| unavailable())?;
    let mut mains = Vec::new();
    let mut assigned = BTreeSet::new();
    let mut other_projects = BTreeSet::new();
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        check_size(0, 1, end)?;
        let id: String = row.get(0).map_err(|_| unavailable())?;
        match inventory::membership(row, 11) {
            CodexMembership::Native(value) if &value == project_id => {
                if catalog
                    .legacy
                    .get(&id)
                    .is_some_and(|claim| claim.as_ref() != Some(&project.locator))
                {
                    return Err(RemovalRefusal::IncompleteRelationships);
                }
                let candidate = inventory::row(db, root, row, &archive.layout, end)?;
                if candidate.session.kind == SessionKind::Unknown {
                    return Err(RemovalRefusal::IncompleteRelationships);
                }
                if candidate.session.is_main() {
                    check_size(mains.len(), MAX_PROJECT_ROOTS, end)?;
                    mains.push(candidate.session.identity);
                }
                assigned.insert(id);
            }
            CodexMembership::Unknown => return Err(RemovalRefusal::IncompleteRelationships),
            CodexMembership::Native(_) => {
                other_projects.insert(id);
            }
            CodexMembership::Unassigned => {}
        }
    }
    if archive.rollouts.iter().any(|(id, rollout)| {
        !canonical.contains(id)
            && !matches!(&rollout.source, ObservedSource::Spawned(_))
            && rollout.meta.forked_from_id.is_none()
            && !rollout.meta.attached_thread
    }) {
        // A rollout-only main has no verifiable project membership.
        return Err(RemovalRefusal::IncompleteRelationships);
    }

    let mut trees = Vec::with_capacity(mains.len());
    let mut members = BTreeSet::new();
    for id in mains {
        let tree = archive.tree(&id, &revision)?;
        for member in &tree.scope.members {
            check_size(members.len(), MAX_PROJECT_MEMBERS, end)?;
            if other_projects.contains(&member.id)
                || catalog.legacy.get(&member.id).is_some_and(|claim| {
                    claim
                        .as_ref()
                        .is_some_and(|locator| locator != &project.locator)
                })
                || !members.insert(member.id.clone())
            {
                return Err(RemovalRefusal::IncompleteRelationships);
            }
        }
        trees.push(tree);
    }
    if !assigned.is_subset(&members) {
        return Err(RemovalRefusal::IncompleteRelationships);
    }
    let evidence = ProjectEvidence {
        registration,
        archive_revision: unrelated_revision(db, &archive.graph, &archive.rollouts, &empty, end)?,
        trees,
    };
    if serde_json::to_vec(&evidence)
        .map_err(|_| unavailable())?
        .len()
        > 112 * 1024
    {
        return Err(RemovalRefusal::IncompleteRelationships);
    }
    Ok(evidence)
}

pub(super) fn verify(
    db: &Sqlite,
    root: &Path,
    scope: &RemovalScope,
    end: Instant,
) -> Result<Verification, ProviderError> {
    if supported_layout(db).is_err() || db_identity(root)? != scope.database_identity {
        return Err(unavailable());
    }
    let ids: BTreeSet<_> = scope
        .members
        .iter()
        .map(|member| member.id.clone())
        .collect();
    if ids.is_empty()
        || ids.len() != scope.members.len()
        || ids.len() > MAX_SCOPE_MEMBERS
        || !ids.iter().all(|id| canonical_uuid(id))
    {
        return Err(unavailable());
    }
    let graph = Graph::read(db, end)?;
    let rollouts = rollout_index(root, end)?;
    let mut residual = ResidualEvidence::default();
    let mut row_exists = db
        .prepare("SELECT id FROM threads WHERE id=?1 COLLATE NOCASE LIMIT 1")
        .map_err(|_| unavailable())?;
    for member in &scope.members {
        residual.canonical_rows +=
            usize::from(row_exists.exists([&member.id]).map_err(|_| unavailable())?);
        let mut file_remains = rollouts.contains_key(&member.id);
        if let Some(path) = &member.rollout {
            match filesystem::open_source_read(root, path) {
                Ok(_) => file_remains = true,
                Err(ProviderError::CapabilityUnavailable(code))
                    if code == "source file missing" => {}
                Err(_) => return Err(unavailable()),
            }
        }
        residual.rollouts += usize::from(file_remains);
    }
    residual.spawn_edges = graph
        .inbound
        .iter()
        .filter(|(child, edge)| ids.contains(*child) || ids.contains(&edge.parent))
        .count();
    residual.unrelated_changed =
        unrelated_revision(db, &graph, &rollouts, &ids, end)? != scope.unrelated_revision;
    Ok(if residual == ResidualEvidence::default() {
        Verification::Verified
    } else {
        Verification::Residual(residual)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "ab000000-1111-4111-8111-000000000001";
    const CHILD: &str = "ab000000-1111-4111-8111-000000000002";
    const LEAF: &str = "ab000000-1111-4111-8111-000000000003";

    fn graph(edges: &[(&str, &str)]) -> Result<Graph, ProviderError> {
        let db = Sqlite::open_in_memory().unwrap();
        db.execute(
            "CREATE TABLE thread_spawn_edges(parent_thread_id TEXT, child_thread_id TEXT, status TEXT)",
            [],
        ).unwrap();
        for (parent, child) in edges {
            db.execute(
                "INSERT INTO thread_spawn_edges VALUES(?1,?2,'closed')",
                [parent, child],
            )
            .unwrap();
        }
        Graph::read(&db, Instant::now() + Duration::from_secs(1))
    }

    #[test]
    fn duplicate_conflicting_and_malformed_edges_fail_closed() {
        for edges in [
            [(ROOT, CHILD), (ROOT, CHILD)],
            [(ROOT, LEAF), (CHILD, LEAF)],
            [(ROOT, CHILD), (CHILD, "malformed")],
            [(ROOT, CHILD), ("malformed", LEAF)],
            [(ROOT, CHILD), (CHILD, CHILD)],
        ] {
            assert!(graph(&edges).is_err());
        }
    }

    #[test]
    fn a_cyclic_root_is_ineligible_and_edge_only_descendants_are_included() {
        let end = Instant::now() + Duration::from_secs(1);
        let cycle = graph(&[(ROOT, CHILD), (CHILD, LEAF), (LEAF, ROOT)]).unwrap();
        assert!(matches!(
            cycle.subtree(ROOT, end),
            Err(RemovalRefusal::IncompleteRelationships)
        ));
        let tree = graph(&[(ROOT, CHILD), (CHILD, LEAF)]).unwrap();
        assert_eq!(
            tree.subtree(ROOT, end)
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            [ROOT, CHILD, LEAF]
        );
    }
}
