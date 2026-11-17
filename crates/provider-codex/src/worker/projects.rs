//! Registered project metadata read from the same confined SQLite snapshot as threads.
use super::{ProjectCatalog, inventory, stale, unavailable};
use crate::reader;
use agentmender_domain::{
    ProjectIdentity, ProjectLocator, ProviderKind, RegisteredProject, canonical_uuid,
    observed_path_is_absolute, safe_conversation_text,
};
use agentmender_provider_api::{ProviderError, filesystem};
use rusqlite::Connection as Sqlite;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};

const MAX_PROJECTS: usize = 512;
const MAX_ROOTS: usize = 16;
const MAX_DESKTOP_BYTES: usize = 4 * 1024 * 1024;

fn label(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 4096
        && !value.chars().any(char::is_control)
        && safe_conversation_text(value)
}

fn project_root(value: &str) -> bool {
    label(value) && observed_path_is_absolute(Path::new(value))
}

fn registration(
    root: &Path,
    locator: ProjectLocator,
    name: String,
    roots: Vec<String>,
    desktop_backed: bool,
) -> RegisteredProject {
    RegisteredProject {
        identity: ProjectIdentity {
            provider: ProviderKind::Codex,
            state_root: root.into(),
            locator,
        },
        name: Some(name),
        roots,
        desktop_backed,
    }
}

fn native(
    db: &Sqlite,
    root: &Path,
    layout: &inventory::Layout,
) -> Result<Vec<RegisteredProject>, ProviderError> {
    if !layout.project_id
        || !inventory::is_table(db, "projects")
        || !inventory::is_table(db, "project_roots")
    {
        return Err(unavailable());
    }
    let projects = inventory::columns(db, "projects")?;
    let roots = inventory::columns(db, "project_roots")?;
    if projects.get("id").map(String::as_str) != Some("TEXT")
        || projects.get("name").map(String::as_str) != Some("TEXT")
        || roots.get("project_id").map(String::as_str) != Some("TEXT")
        || roots.get("path").map(String::as_str) != Some("TEXT")
        || roots.get("position").map(String::as_str) != Some("INTEGER")
    {
        return Err(unavailable());
    }
    let mut result = Vec::new();
    let mut statement = db
        .prepare("SELECT id,name FROM projects ORDER BY id LIMIT 513")
        .map_err(|_| unavailable())?;
    let mut roots_query = db
        .prepare("SELECT path FROM project_roots WHERE project_id=?1 ORDER BY position LIMIT 17")
        .map_err(|_| unavailable())?;
    let mut rows = statement.query([]).map_err(|_| unavailable())?;
    let mut seen = BTreeSet::new();
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        let id: String = row.get(0).map_err(|_| unavailable())?;
        let name: String = row.get(1).map_err(|_| unavailable())?;
        if result.len() == MAX_PROJECTS
            || !canonical_uuid(&id)
            || !label(&name)
            || !seen.insert(id.clone())
        {
            return Err(unavailable());
        }
        let roots = roots_query
            .query_map([&id], |row| row.get::<_, String>(0))
            .map_err(|_| unavailable())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| unavailable())?;
        if roots.len() > MAX_ROOTS || !roots.iter().all(|path| project_root(path)) {
            return Err(unavailable());
        }
        result.push(registration(
            root,
            ProjectLocator::Codex(id),
            name,
            roots,
            false,
        ));
    }
    Ok(result)
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct Desktop {
    local_projects: BTreeMap<String, DesktopProject>,
    thread_project_assignments: BTreeMap<String, DesktopAssignment>,
    thread_project_membership_host_ids: BTreeMap<String, String>,
    projectless_thread_ids: Vec<String>,
    app_server_project_id_by_legacy_project_id_by_host: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopProject {
    id: String,
    name: String,
    root_paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopAssignment {
    project_kind: String,
    project_id: String,
}

fn desktop(root: &Path) -> Result<Desktop, ProviderError> {
    let path = root.join(".codex-global-state.json");
    let mut file = match filesystem::open_source_read(root, &path) {
        Ok(file) => file,
        Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {
            return Ok(Desktop::default());
        }
        Err(error) => return Err(error),
    };
    let before = reader::stamp(&file.metadata().map_err(|_| unavailable())?);
    let mut bytes = Vec::new();
    (&mut file)
        .take((MAX_DESKTOP_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    let latest = filesystem::open_source_read(root, &path)?;
    if before != reader::stamp(&latest.metadata().map_err(|_| unavailable())?) {
        return Err(stale());
    }
    if bytes.len() > MAX_DESKTOP_BYTES {
        return Err(unavailable());
    }
    serde_json::from_slice(&bytes).map_err(|_| unavailable())
}

pub(super) fn read(
    db: &Sqlite,
    root: &Path,
    layout: &inventory::Layout,
    revision: String,
) -> ProjectCatalog {
    let mut catalog = ProjectCatalog {
        revision,
        ..Default::default()
    };
    let native_known = match native(db, root, layout) {
        Ok(projects) => {
            catalog.projects = projects;
            true
        }
        Err(_) => {
            catalog.incomplete = true;
            false
        }
    };
    match desktop(root) {
        Ok(state) => merge_desktop(&mut catalog, root, state, native_known),
        Err(_) => catalog.incomplete = true,
    }
    if serde_json::to_vec(&catalog).map_or(true, |bytes| bytes.len() > 112 * 1024) {
        catalog.projects.clear();
        catalog.legacy.clear();
        catalog.incomplete = true;
    }
    catalog
}

fn merge_desktop(catalog: &mut ProjectCatalog, root: &Path, desktop: Desktop, native_known: bool) {
    if desktop.local_projects.len() > MAX_PROJECTS {
        catalog.incomplete = true;
        return;
    }
    let host = format!("local:{}", root.display());
    let migration = desktop
        .app_server_project_id_by_legacy_project_id_by_host
        .get(&host);
    let mut mapped = BTreeSet::new();
    let mut locators = BTreeMap::new();
    for (id, project) in &desktop.local_projects {
        if id != &project.id
            || !canonical_uuid(id)
            || !label(&project.name)
            || project.root_paths.len() > MAX_ROOTS
            || !project.root_paths.iter().all(|path| project_root(path))
        {
            catalog.incomplete = true;
            continue;
        }
        let alias = migration.and_then(|entries| entries.get(id));
        if alias.is_some_and(|id| !canonical_uuid(id)) {
            catalog.incomplete = true;
            continue;
        }
        let native = ProjectLocator::Codex(alias.unwrap_or(id).clone());
        let locator = match catalog
            .projects
            .iter_mut()
            .find(|project| project.identity.locator == native)
        {
            Some(registered) if mapped.insert(native.clone()) => {
                registered.desktop_backed = true;
                catalog.incomplete |= registered.name.as_ref() != Some(&project.name)
                    || registered.roots != project.root_paths;
                native
            }
            Some(_) => {
                catalog.incomplete = true;
                locators.retain(|_, locator| locator != &native);
                continue;
            }
            None if native_known && alias.is_some() => continue,
            None => {
                let locator = ProjectLocator::CodexDesktop(id.clone());
                catalog.projects.push(registration(
                    root,
                    locator.clone(),
                    project.name.clone(),
                    project.root_paths.clone(),
                    true,
                ));
                locator
            }
        };
        locators.insert(id, locator);
    }
    let local = |id: &str| {
        canonical_uuid(id)
            && desktop
                .thread_project_membership_host_ids
                .get(id)
                .is_none_or(|host| host == "local")
    };
    for (id, claim) in desktop
        .thread_project_assignments
        .iter()
        .filter(|(id, _)| local(id))
    {
        if claim.project_kind == "local" {
            let locator = locators.get(&claim.project_id).cloned();
            catalog.incomplete |= locator.is_none();
            catalog.legacy.insert(id.clone(), locator);
        }
    }
    for id in desktop.projectless_thread_ids.iter().filter(|id| local(id)) {
        let previous = catalog.legacy.insert(id.clone(), None);
        catalog.incomplete |= previous.is_some_and(|claim| claim.is_some());
    }
}
