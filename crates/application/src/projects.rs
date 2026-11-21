//! Presentation groups for verified main sessions, before filtering and pagination.
use crate::local_inventory::{
    InventoryPage, InventoryQuery, InventorySort, LocalInventory, compare, folded,
};
use agentmender_domain::{
    InventoryObservation, ProjectIdentity, ProviderKind, Session, SessionIdentity,
};
use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

/// Codex keys retain their registered identity. Claude groups are keyed by
/// workspace across archive roots, while each row retains its session identity.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectKey {
    Registered(ProjectIdentity),
    Workspace(String),
}

impl ProjectKey {
    pub fn provider(&self) -> ProviderKind {
        match self {
            Self::Registered(identity) => identity.provider,
            Self::Workspace(_) => ProviderKind::Claude,
        }
    }
}

pub struct Project {
    pub key: ProjectKey,
    pub name: String,
    pub roots: Vec<String>,
    pub desktop_backed: bool,
    pub membership_incomplete: bool,
    pub sessions: LocalInventory,
    labels: Vec<String>,
    first_created: Option<i64>,
    last_active_at: Option<i64>,
    transcript_bytes: Option<u64>,
}

impl Project {
    pub fn page(&self, query: &InventoryQuery) -> InventoryPage<'_> {
        self.sessions.page_with_labels(query, &self.labels)
    }
}

#[derive(Default)]
struct Group<'a> {
    name: String,
    roots: Vec<String>,
    desktop_backed: bool,
    main: Vec<Session>,
    transcripts: Vec<&'a Session>,
}

pub struct ProjectInventory {
    pub projects: Vec<Project>,
    pub recents: LocalInventory,
}

pub fn build(observation: &InventoryObservation, home: &Path) -> ProjectInventory {
    let mut groups: BTreeMap<_, _> = observation
        .projects
        .iter()
        .map(|project| {
            (
                ProjectKey::Registered(project.identity.clone()),
                Group {
                    name: project.name.clone().expect("observed Codex name"),
                    roots: project.roots.clone(),
                    desktop_backed: project.desktop_backed,
                    ..Group::default()
                },
            )
        })
        .collect();
    let sessions: BTreeMap<_, _> = observation
        .sessions
        .iter()
        .map(|session| (&session.identity, session))
        .collect();
    let mut recents = Vec::new();
    for session in sessions
        .values()
        .copied()
        .filter(|session| session.is_main())
    {
        let group = match project_key(observation, session, home) {
            Some(ProjectKey::Workspace(path)) => {
                let name = Path::new(&path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("workspace basename")
                    .to_owned();
                Some(
                    groups
                        .entry(ProjectKey::Workspace(path.clone()))
                        .or_insert_with(|| Group {
                            name,
                            roots: vec![path],
                            ..Group::default()
                        }),
                )
            }
            Some(key) => groups.get_mut(&key),
            None => None,
        };
        if let Some(group) = group {
            group.main.push(session.clone());
            group.transcripts.push(session);
        } else {
            recents.push(session.clone());
        }
    }
    for session in sessions
        .values()
        .copied()
        .filter(|session| !session.is_main())
    {
        let owner = main_ancestor(session, &sessions);
        let key = owner.and_then(|session| project_key(observation, session, home));
        if let Some(group) = key.as_ref().and_then(|key| groups.get_mut(key)) {
            group.transcripts.push(session);
        }
    }
    let projects = groups
        .into_iter()
        .map(|(key, group)| {
            let mut labels = vec![folded(&group.name)];
            labels.extend(group.roots.iter().map(|root| folded(root)));
            let mut sizes = group
                .transcripts
                .iter()
                .filter_map(|row| row.transcript_bytes);
            let transcript_bytes = sizes
                .next()
                .and_then(|first| sizes.try_fold(first, u64::checked_add));
            Project {
                key,
                name: group.name,
                roots: group.roots,
                desktop_backed: group.desktop_backed,
                membership_incomplete: observation.projects_incomplete,
                last_active_at: group
                    .transcripts
                    .iter()
                    .filter_map(|row| row.last_active_at)
                    .max(),
                first_created: group.main.iter().filter_map(|row| row.created_at).min(),
                transcript_bytes,
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

fn project_key(
    observation: &InventoryObservation,
    session: &Session,
    home: &Path,
) -> Option<ProjectKey> {
    if session.identity.provider == ProviderKind::Claude {
        let workspace = Path::new(session.workspace.as_deref()?);
        let parent = workspace.parent()?;
        // A home or top-level directory is not evidence of a project. Avoid
        // resolving symlinks or inferring a path from Claude's encoded namespace.
        if !workspace.is_absolute()
            || home.starts_with(workspace)
            || parent.file_name().is_none()
            || (parent
                .parent()
                .is_some_and(|ancestor| ancestor.file_name().is_none())
                && parent.file_name().is_some_and(|name| {
                    name.eq_ignore_ascii_case("users") || name.eq_ignore_ascii_case("home")
                }))
            || workspace
                .components()
                .any(|part| part == Component::ParentDir)
        {
            return None;
        }
        let normalized = workspace.components().collect::<std::path::PathBuf>();
        return Some(ProjectKey::Workspace(
            normalized.into_os_string().into_string().ok()?,
        ));
    }
    let locator = observation
        .codex_members
        .get(&session.identity.native_id)?
        .clone();
    Some(ProjectKey::Registered(ProjectIdentity {
        provider: session.identity.provider,
        state_root: session.identity.state_root.clone(),
        locator,
    }))
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
        .then_with(|| left.key.cmp(&right.key))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_groups_by_workspace_across_namespaces_and_archives() {
        let home = if cfg!(windows) {
            "C:/Users/person"
        } else {
            "/Users/person"
        };
        let home = Path::new(home);
        let work = home.join("work");
        let alpha = work.join("alpha");
        let beta = work.join("beta");
        let session =
            |id: &str, archive: &str, namespace: &str, workspace: Option<&Path>| Session {
                identity: SessionIdentity {
                    provider: ProviderKind::Claude,
                    state_root: home.join(archive),
                    namespace: Some(namespace.to_owned()),
                    native_id: id.to_owned(),
                },
                codex_membership: Default::default(),
                title: None,
                workspace: workspace.map(|path| path.to_str().unwrap().to_owned()),
                project: None,
                created_at: None,
                last_active_at: None,
                state: agentmender_domain::SessionState::Unknown,
                transcript_bytes: Some(1024),
                kind: agentmender_domain::SessionKind::Main,
                forked_from: None,
                parent: None,
            };
        let inventory = build(
            &InventoryObservation {
                sessions: vec![
                    session("alpha-1", "archive", "home", Some(&alpha)),
                    session(
                        "alpha-2",
                        "other-archive",
                        "different-namespace",
                        Some(&alpha),
                    ),
                    session("beta-1", "archive", "home", Some(&beta)),
                    session("home-1", "archive", "home", Some(home)),
                    session("ambiguous-1", "archive", "home", None),
                ],
                ..InventoryObservation::default()
            },
            home,
        );

        let counts: BTreeMap<_, _> = inventory
            .projects
            .iter()
            .map(|project| (project.name.as_str(), project.sessions.len()))
            .collect();
        assert_eq!(counts, BTreeMap::from([("alpha", 2), ("beta", 1)]));
        assert_eq!(inventory.recents.len(), 2);
        assert!(
            inventory
                .recents
                .sessions()
                .all(|row| row.identity.namespace.as_deref() == Some("home"))
        );
    }
}
