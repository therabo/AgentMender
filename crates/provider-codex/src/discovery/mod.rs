//! Bounded provider discovery. Evidence of location does not authorize mutation.
mod platform;

use agentmender_domain::{ProviderKind, canonical_uuid};
use agentmender_provider_api::{ProviderLocation, filesystem, process};
#[cfg(not(unix))]
use same_file::Handle;
use serde::Deserialize;
use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageKind {
    Home,
    Sessions,
    ArchivedSessions,
    StateDatabase,
    HistoryDatabase,
    DesktopCatalog,
    ExportCandidate,
    ArtifactSessionExports,
    ExternalExports,
    Attachments,
    GeneratedImages,
    Cache,
    Logs,
    Worktrees,
    UserOutput,
    DesktopProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Access {
    Accessible,
    Unavailable,
    Linked,
    NetworkPath,
}

#[derive(Clone, Debug)]
pub struct Storage {
    pub kind: StorageKind,
    pub path: PathBuf,
    pub identity: Option<String>,
    pub observed: bool,
    pub recognized: bool,
    pub owned: bool,
    pub access: Access,
    pub inventory_available: bool,
}

struct Installation {
    kind: &'static str,
    version: Option<String>,
    executable: Option<PathBuf>,
}

pub struct Discovery {
    pub selected_root: PathBuf,
    pub storage: Vec<Storage>,
    pub location: ProviderLocation,
}

#[derive(Default, Deserialize)]
struct Settings {
    sqlite_home: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    desktop: Option<DesktopSettings>,
}

#[derive(Default, Deserialize)]
struct DesktopSettings {
    #[serde(rename = "git-worktree-root")]
    worktree_root: Option<PathBuf>,
    #[serde(rename = "workspace-root")]
    workspace_root: Option<PathBuf>,
}

fn absolute(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| path.is_absolute())
}

fn environment(name: &str) -> Option<PathBuf> {
    absolute(env::var_os(name).map(PathBuf::from))
}

fn network_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        matches!(path.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::UNC(..) | Prefix::VerbatimUNC(..) | Prefix::DeviceNS(..)))
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

pub(crate) fn access(path: &Path) -> Access {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| part == std::path::Component::ParentDir)
    {
        return Access::Unavailable;
    }
    if network_path(path) {
        return Access::NetworkPath;
    }
    // A junction or symlink at any component can redirect a lookup into an
    // unrelated profile. Stock macOS /var and /tmp system aliases are handled
    // by supplying the canonical archive path.
    for part in path
        .ancestors()
        .filter(|part| part.is_absolute())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        match fs::symlink_metadata(part) {
            Ok(meta) if filesystem::is_link(&meta) => return Access::Linked,
            Ok(_) => {}
            Err(_) => return Access::Unavailable,
        }
    }
    Access::Accessible
}

pub(crate) fn identity(path: &Path) -> Option<String> {
    if access(path) != Access::Accessible {
        return None;
    }
    let mut hash = DefaultHasher::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let info = fs::symlink_metadata(path).ok()?;
        (info.dev(), info.ino()).hash(&mut hash);
    }
    #[cfg(not(unix))]
    Handle::from_path(path).ok()?.hash(&mut hash);
    Some(format!("{:016x}", hash.finish()))
}

fn settings(root: &Path) -> Settings {
    let path = root.join("config.toml");
    if access(&path) != Access::Accessible {
        return Settings::default();
    }
    filesystem::read_bounded(root, &path, 256 * 1024)
        .ok()
        .and_then(|bytes| toml::from_str::<Settings>(std::str::from_utf8(&bytes).ok()?).ok())
        .unwrap_or_default()
}

fn archive_format(root: &Path) -> bool {
    let path = root.join("state_5.sqlite");
    if access(&path) != Access::Accessible {
        return false;
    }
    filesystem::read_prefix(root, &path, 16).is_ok_and(|bytes| bytes == b"SQLite format 3\0")
}

fn provider_marker(root: &Path) -> bool {
    #[derive(Deserialize)]
    struct IndexEntry {
        id: String,
    }
    let index = root.join("session_index.jsonl");
    if access(&index) == Access::Accessible
        && let Ok(bytes) = filesystem::read_prefix(root, &index, 8192)
        && let Some(end) = bytes.iter().position(|byte| *byte == b'\n')
        && serde_json::from_slice::<IndexEntry>(&bytes[..end])
            .is_ok_and(|entry| canonical_uuid(&entry.id))
    {
        return true;
    }
    let desktop = root.join(".codex-global-state.json");
    access(&desktop) == Access::Accessible
        && filesystem::read_bounded(root, &desktop, 2 * 1024 * 1024)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|value| {
                value.get("local-projects").is_some()
                    || value.get("thread-project-assignments").is_some()
            })
}

fn row(kind: StorageKind, path: PathBuf, recognized: bool, owned: bool) -> Storage {
    let access = access(&path);
    let observed = access == Access::Accessible
        || access == Access::Linked
            && path
                .parent()
                .is_some_and(|parent| self::access(parent) == Access::Accessible);
    let export = matches!(
        kind,
        StorageKind::ExportCandidate
            | StorageKind::ArtifactSessionExports
            | StorageKind::ExternalExports
    );
    Storage {
        kind,
        identity: (access == Access::Accessible)
            .then(|| identity(&path))
            .flatten(),
        path,
        observed,
        recognized,
        owned,
        access,
        inventory_available: access == Access::Accessible && export,
    }
}

impl Discovery {
    /// Launch selection is deterministic. Additional roots stay distinct in
    /// Storage and are never attached to the selected session writer.
    pub fn detect(
        home: &Path,
        root_override: Option<PathBuf>,
        executable_override: Option<PathBuf>,
    ) -> Result<Self, &'static str> {
        let configured = env::var_os("CODEX_HOME").map(PathBuf::from);
        let platform_archives = platform::archive_candidates(home);
        let default = home.join(".codex");
        let mut recognized_alternatives = platform_archives
            .iter()
            .filter(|path| archive_format(path) && provider_marker(path));
        let (selected, source) = if let Some(root) = root_override {
            (root, "argomento di avvio")
        } else if let Some(root) = configured.as_ref() {
            (root.clone(), "CODEX_HOME")
        } else if access(&default) == Access::Unavailable
            && let Some(alternative) = recognized_alternatives.next()
            && recognized_alternatives.next().is_none()
        {
            (alternative.clone(), "archivio alternativo riconosciuto")
        } else {
            (default.clone(), "default del provider")
        };
        if !selected.is_absolute() {
            return Err("CODEX_HOME deve essere un percorso assoluto.");
        }
        let homes = std::iter::once((selected.clone(), source, true))
            .chain(absolute(configured).map(|path| (path, "CODEX_HOME", false)))
            .chain([(home.join(".codex"), "default del provider", false)])
            .chain(
                platform_archives
                    .into_iter()
                    .map(|path| (path, "layout piattaforma verificato", false)),
            );
        let mut seen = BTreeSet::new();
        let mut storage = Vec::new();
        for (archive_root, source, primary) in homes {
            let user_home = home;
            let home = archive_root;
            if network_path(&home) && !primary {
                continue;
            }
            let key = identity(&home).unwrap_or_else(|| home.to_string_lossy().into_owned());
            if !seen.insert(key)
                || !primary && !archive_format(&home)
                || source == "layout piattaforma verificato" && !provider_marker(&home)
            {
                continue;
            }
            let mut settings = settings(&home);
            let configured_sqlite = absolute(settings.sqlite_home.take());
            let sqlite = configured_sqlite
                .as_ref()
                .cloned()
                .or_else(|| primary.then(|| environment("CODEX_SQLITE_HOME")).flatten())
                .unwrap_or_else(|| home.clone());
            let recognized = archive_format(&sqlite);
            let owned = recognized && provider_marker(&home);
            storage.push(row(StorageKind::Home, home.clone(), recognized, owned));
            for (kind, name) in [
                (StorageKind::Sessions, "sessions"),
                (StorageKind::ArchivedSessions, "archived_sessions"),
                (StorageKind::DesktopCatalog, "sqlite"),
                (StorageKind::ExportCandidate, "exports"),
                (StorageKind::Attachments, "attachments"),
                (StorageKind::GeneratedImages, "generated_images"),
                (StorageKind::Cache, "cache"),
            ] {
                let path = home.join(name);
                let provider_layout = recognized && kind != StorageKind::ExportCandidate;
                let observation = row(kind, path, provider_layout, provider_layout && owned);
                if observation.observed {
                    storage.push(observation);
                }
            }
            for (kind, name) in [
                (StorageKind::StateDatabase, "state_5.sqlite"),
                (StorageKind::HistoryDatabase, "thread_history_1.sqlite"),
            ] {
                let observation = row(kind, sqlite.join(name), recognized, owned);
                if observation.observed || kind == StorageKind::StateDatabase && sqlite != home {
                    storage.push(observation);
                }
            }
            let logs = absolute(settings.log_dir).unwrap_or_else(|| home.join("log"));
            if network_path(&logs) || fs::symlink_metadata(&logs).is_ok() {
                storage.push(row(StorageKind::Logs, logs, recognized, recognized));
            }
            let desktop = settings.desktop.unwrap_or_default();
            for (kind, path) in [
                (
                    StorageKind::Worktrees,
                    absolute(desktop.worktree_root).unwrap_or_else(|| home.join("worktrees")),
                ),
                (
                    StorageKind::UserOutput,
                    absolute(desktop.workspace_root)
                        .unwrap_or_else(|| platform::user_output(user_home)),
                ),
            ] {
                let observation = row(
                    kind,
                    path,
                    recognized && kind == StorageKind::Worktrees,
                    false,
                );
                if observation.observed || observation.access == Access::NetworkPath {
                    storage.push(observation);
                }
            }
        }

        let (mut installations, desktop_storage) = platform::desktop(home);
        storage.extend(desktop_storage);
        if let Some(path) = environment("ARTIFACT_SESSION_HOST_EXPORT_ROOT") {
            let mut external = row(StorageKind::ExternalExports, path, false, false);
            if selected.starts_with(&external.path) || home.starts_with(&external.path) {
                external.inventory_available = false;
            }
            storage.insert(0, external);
        }
        let candidates = executable_override
            .iter()
            .cloned()
            .chain(process::find_executables(ProviderKind::Codex))
            .chain(platform::cli_candidates(home));
        let mut seen_paths = BTreeSet::new();
        for mut candidate in candidates.take(32) {
            if network_path(&candidate) {
                continue;
            }
            if let Ok(path) = candidate.canonicalize() {
                candidate = path;
            }
            if !seen_paths.insert(candidate.clone()) || !candidate.is_file() {
                continue;
            }
            let version = if seen_paths.len() <= 3 {
                process::probe_version(ProviderKind::Codex, &candidate).ok()
            } else {
                None
            };
            installations.push(Installation {
                kind: "cli",
                version,
                executable: Some(candidate),
            });
        }
        let executable = executable_override.or_else(|| {
            installations
                .iter()
                .find(|item| item.kind == "cli" && item.version.is_some())
                .or_else(|| installations.iter().find(|item| item.executable.is_some()))
                .and_then(|item| item.executable.clone())
        });
        Ok(Self {
            selected_root: selected.clone(),
            storage,
            location: ProviderLocation {
                state_root: selected,
                executable,
            },
        })
    }

    pub fn selected_access(&self) -> Access {
        access(&self.selected_root)
    }

    pub fn inspect_exports(&self) -> crate::exports::Inventory {
        crate::exports::Inventory::scan(&self.storage)
    }
}
