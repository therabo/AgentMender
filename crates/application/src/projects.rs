//! Provider-scoped grouping before filtering, sorting and pagination.
use crate::local_inventory::{
    InventoryPage, InventoryQuery, InventorySort, LocalInventory, compare, folded,
};
use agentmender_domain::{
    InventoryObservation, ProjectIdentity, ProjectLocator, ProviderKind, Session, SessionIdentity,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Project {
    pub identity: ProjectIdentity,
    pub name: String,
    pub workspace: Option<String>,
    pub transcript_bytes: Option<u64>,
    pub transcript_bytes_incomplete: bool,
    pub last_active_at: Option<i64>,
    pub incomplete: bool,
    pub sessions: LocalInventory,
    labels: Vec<String>,
    first_created: Option<i64>,
}

impl Project {
    pub fn page(&self, query: &InventoryQuery) -> InventoryPage<'_> {
        self.sessions.page_with_labels(query, &self.labels)
    }
}

#[derive(Default)]
struct Group<'a> {
    main: Vec<Session>,
    transcripts: Vec<&'a Session>,
}

pub fn build(observation: &InventoryObservation) -> Vec<Project> {
    let mut groups: BTreeMap<_, Group<'_>> = observation
        .projects
        .iter()
        .cloned()
        .map(|id| (id, Group::default()))
        .collect();
    let sessions: BTreeMap<_, _> = observation
        .sessions
        .iter()
        .map(|session| (&session.identity, session))
        .collect();
    for session in observation
        .sessions
        .iter()
        .filter(|session| session.is_main())
    {
        let group = groups
            .entry(ProjectIdentity::for_session(session))
            .or_default();
        group.main.push(session.clone());
        group.transcripts.push(session);
    }
    for session in observation
        .sessions
        .iter()
        .filter(|session| !session.is_main())
    {
        let project =
            if session.identity.provider == ProviderKind::Claude && session.parent.is_some() {
                Some(ProjectIdentity::for_session(session))
            } else {
                main_ancestor(session, &sessions).map(ProjectIdentity::for_session)
            };
        if let Some(group) = project.as_ref().and_then(|id| groups.get_mut(id)) {
            group.transcripts.push(session);
        }
    }
    groups
        .into_iter()
        .map(|(identity, group)| {
            let workspaces: BTreeSet<_> = group
                .main
                .iter()
                .filter_map(|row| row.workspace.as_deref())
                .collect();
            let (name, workspace): (String, Option<String>) = match &identity.locator {
                ProjectLocator::Unassigned => ("Senza progetto".into(), None),
                ProjectLocator::Workspace(path) => (basename(path).into(), Some(path.clone())),
                ProjectLocator::Namespace(namespace) => {
                    let workspace = (workspaces.len() == 1)
                        .then(|| workspaces.first().expect("one workspace").to_string());
                    let name = workspace
                        .as_deref()
                        .map_or(namespace.as_str(), basename)
                        .into();
                    (name, workspace)
                }
            };
            let mut labels = vec![folded(&name)];
            if let Some(workspace) = &workspace {
                labels.push(folded(workspace));
            }
            if let ProjectLocator::Namespace(namespace) = &identity.locator {
                labels.push(folded(namespace));
            }
            let mut sizes = group
                .transcripts
                .iter()
                .filter_map(|row| row.transcript_bytes)
                .peekable();
            let transcript_bytes = if sizes.peek().is_some() {
                sizes.try_fold(0u64, u64::checked_add)
            } else {
                None
            };
            // A Codex metadata scan sees canonical rows. Edge-only members can
            // have additional rollouts that are discovered only at preflight.
            let sizes_incomplete = identity.provider == ProviderKind::Codex
                || observation.incomplete
                || group
                    .transcripts
                    .iter()
                    .any(|row| row.transcript_bytes.is_none())
                || (!group.transcripts.is_empty() && transcript_bytes.is_none());
            Project {
                identity,
                name,
                workspace,
                last_active_at: group
                    .transcripts
                    .iter()
                    .filter_map(|row| row.last_active_at)
                    .max(),
                first_created: group.main.iter().filter_map(|row| row.created_at).min(),
                transcript_bytes,
                transcript_bytes_incomplete: sizes_incomplete,
                incomplete: observation.incomplete,
                sessions: LocalInventory::new(group.main),
                labels,
            }
        })
        .collect()
}

fn basename(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

fn main_ancestor<'a>(
    session: &Session,
    sessions: &BTreeMap<&SessionIdentity, &'a Session>,
) -> Option<&'a Session> {
    let mut parent = session.parent.as_ref();
    for _ in 0..128 {
        let session = *sessions.get(parent?)?;
        if session.is_main() {
            return Some(session);
        }
        parent = session.parent.as_ref();
    }
    None
}

pub fn page<'a>(
    projects: impl Iterator<Item = &'a Project>,
    query: &InventoryQuery,
) -> (usize, Vec<&'a Project>) {
    let search = folded(query.search.trim());
    let mut matches: Vec<_> = projects
        .filter(|project| {
            if project.sessions.is_empty() {
                return query.state.is_none()
                    && query.since.is_none()
                    && query.before.is_none()
                    && search
                        .split_whitespace()
                        .all(|word| project.labels.iter().any(|label| label.contains(word)));
            }
            project
                .sessions
                .matching(query, &project.labels)
                .next()
                .is_some()
        })
        .collect();
    matches.sort_unstable_by(|left, right| {
        let ascending = query.ascending;
        match query.sort {
            InventorySort::Activity => {
                compare(left.last_active_at, right.last_active_at, ascending)
            }
            InventorySort::Created => compare(left.first_created, right.first_created, ascending),
            InventorySort::Size => {
                compare(left.transcript_bytes, right.transcript_bytes, ascending)
            }
        }
        .then_with(|| left.identity.cmp(&right.identity))
    });
    (
        matches.len(),
        matches
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect(),
    )
}
