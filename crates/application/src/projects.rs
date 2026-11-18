//! Provider-scoped grouping before filtering, sorting and pagination.
use crate::local_inventory::{
    InventoryPage, InventoryQuery, InventorySort, LocalInventory, compare, folded,
};
use agentmender_domain::{
    InventoryObservation, ProjectIdentity, ProjectLocator, ProviderKind, RegisteredProject,
    Session, SessionIdentity,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Project {
    pub identity: ProjectIdentity,
    pub name: String,
    pub workspace: Option<String>,
    pub roots: Vec<String>,
    pub desktop_backed: bool,
    pub membership_incomplete: bool,
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

pub struct ProjectInventory {
    pub projects: Vec<Project>,
    pub recents: LocalInventory,
}

pub fn build(observation: &InventoryObservation) -> ProjectInventory {
    let mut groups: BTreeMap<_, (&RegisteredProject, Group<'_>)> = observation
        .projects
        .iter()
        .map(|project| (&project.identity, (project, Group::default())))
        .collect();
    let sessions: BTreeMap<_, _> = observation
        .sessions
        .iter()
        .map(|session| (&session.identity, session))
        .collect();
    let mut recents = Vec::new();
    for session in observation
        .sessions
        .iter()
        .filter(|session| session.is_main())
    {
        if let Some((_, group)) = identity(observation, session)
            .as_ref()
            .and_then(|id| groups.get_mut(id))
        {
            group.main.push(session.clone());
            group.transcripts.push(session);
        } else {
            recents.push(session.clone());
        }
    }
    for session in observation
        .sessions
        .iter()
        .filter(|session| !session.is_main())
    {
        let owner = if session.identity.provider == ProviderKind::Claude && session.parent.is_some()
        {
            Some(session)
        } else {
            main_ancestor(session, &sessions)
        };
        let project = owner.and_then(|session| identity(observation, session));
        if let Some((_, group)) = project.as_ref().and_then(|id| groups.get_mut(id)) {
            group.transcripts.push(session);
        }
    }
    let projects = groups
        .into_values()
        .map(|(registered, group)| {
            let identity = registered.identity.clone();
            let (name, workspace) = match &identity.locator {
                ProjectLocator::Codex(_) | ProjectLocator::CodexDesktop(_) => (
                    registered.name.clone().expect("observed Codex name"),
                    registered.roots.first().cloned(),
                ),
                ProjectLocator::Namespace(namespace) => {
                    let workspaces: BTreeSet<_> = group
                        .main
                        .iter()
                        .filter_map(|row| row.workspace.as_deref())
                        .collect();
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
            labels.extend(registered.roots.iter().map(|root| folded(root)));
            if let ProjectLocator::Namespace(namespace) = &identity.locator {
                labels.push(folded(namespace));
                labels.extend(workspace.iter().map(|path| folded(path)));
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
                roots: registered.roots.clone(),
                desktop_backed: registered.desktop_backed,
                membership_incomplete: observation.projects_incomplete,
                last_active_at: group
                    .transcripts
                    .iter()
                    .filter_map(|row| row.last_active_at)
                    .max(),
                first_created: group.main.iter().filter_map(|row| row.created_at).min(),
                transcript_bytes,
                transcript_bytes_incomplete: sizes_incomplete,
                incomplete: observation.incomplete || observation.projects_incomplete,
                sessions: LocalInventory::new(group.main),
                labels,
            }
        })
        .collect();
    ProjectInventory {
        projects,
        recents: LocalInventory::new(recents),
    }
}

fn identity(observation: &InventoryObservation, session: &Session) -> Option<ProjectIdentity> {
    let locator = match session.identity.provider {
        ProviderKind::Codex => observation
            .codex_members
            .get(&session.identity.native_id)
            .cloned(),
        ProviderKind::Claude => session
            .identity
            .namespace
            .clone()
            .map(ProjectLocator::Namespace),
    }?;
    Some(ProjectIdentity {
        provider: session.identity.provider,
        state_root: session.identity.state_root.clone(),
        locator,
    })
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
                    && query.min_bytes.is_none()
                    && query.max_bytes.is_none()
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
