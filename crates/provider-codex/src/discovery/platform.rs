#[cfg(any(target_os = "macos", windows))]
use super::{Access, access};
use super::{Installation, Storage, environment};
#[cfg(any(target_os = "macos", target_os = "linux", windows))]
use super::{StorageKind, row};
#[cfg(any(target_os = "macos", windows))]
use agentmender_provider_api::process;
use std::path::{Path, PathBuf};

pub(super) fn archive_candidates(home: &Path) -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        [
            ("XDG_DATA_HOME", ".local/share"),
            ("XDG_CONFIG_HOME", ".config"),
            ("XDG_STATE_HOME", ".local/state"),
        ]
        .into_iter()
        .map(|(key, fallback)| {
            environment(key)
                .unwrap_or_else(|| home.join(fallback))
                .join("codex")
        })
        .collect()
    }
    #[cfg(windows)]
    {
        [
            ("APPDATA", "AppData/Roaming"),
            ("LOCALAPPDATA", "AppData/Local"),
        ]
        .into_iter()
        .map(|(key, fallback)| {
            environment(key)
                .unwrap_or_else(|| home.join(fallback))
                .join("OpenAI")
                .join("Codex")
        })
        .collect()
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        vec![home.join("Library/Application Support/OpenAI/Codex")]
    }
}

pub(super) fn user_output(home: &Path) -> PathBuf {
    home.join("Documents").join("Codex")
}

pub(super) fn cli_candidates(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        home.join(".local/bin")
            .join(if cfg!(windows) { "codex.exe" } else { "codex" }),
        home.join(".cargo/bin")
            .join(if cfg!(windows) { "codex.exe" } else { "codex" }),
    ];
    if cfg!(target_os = "macos") {
        paths.extend(["/opt/homebrew/bin/codex", "/usr/local/bin/codex"].map(PathBuf::from));
    }
    if cfg!(windows) {
        if let Some(base) = environment("CODEX_INSTALL_DIR") {
            paths.extend([base.join("codex.exe"), base.join("bin").join("codex.exe")]);
        }
        if let Some(base) = environment("LOCALAPPDATA") {
            paths.push(
                base.join("OpenAI")
                    .join("Codex")
                    .join("bin")
                    .join("codex.exe"),
            );
        }
        if let Some(base) = environment("APPDATA") {
            paths.push(base.join("npm").join("codex.cmd"));
        }
    }
    paths
}

pub(super) fn desktop(home: &Path) -> (Vec<Installation>, Vec<Storage>) {
    #[cfg(target_os = "macos")]
    {
        macos(home)
    }
    #[cfg(windows)]
    {
        let _ = home;
        windows()
    }
    #[cfg(target_os = "linux")]
    {
        let base = environment("XDG_CACHE_HOME").unwrap_or_else(|| home.join(".cache"));
        let cache = base.join("codex");
        // A name-based XDG candidate is metadata only. Upstream CLI evidence
        // does not establish a Desktop profile or ownership here.
        let row = row(StorageKind::Cache, cache, false, false);
        (vec![], row.observed.then_some(row).into_iter().collect())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = home;
        (vec![], vec![])
    }
}

#[cfg(target_os = "macos")]
fn macos(home: &Path) -> (Vec<Installation>, Vec<Storage>) {
    let mut installations = Vec::new();
    for parent in [PathBuf::from("/Applications"), home.join("Applications")] {
        for name in ["ChatGPT.app", "Codex.app"] {
            let bundle = parent.join(name);
            if access(&bundle) != Access::Accessible {
                continue;
            }
            let Ok(info) = process::read_bundle_metadata(&bundle) else {
                continue;
            };
            let executable_name = Path::new(&info.executable);
            if info.identifier != "com.openai.codex"
                || executable_name.components().count() != 1
                || !matches!(
                    executable_name.components().next(),
                    Some(std::path::Component::Normal(_))
                )
            {
                continue;
            }
            let desktop_binary = bundle.join("Contents/MacOS").join(executable_name);
            if access(&desktop_binary) != Access::Accessible || !desktop_binary.is_file() {
                continue;
            }
            let resources = bundle.join("Contents/Resources");
            let executable = [
                resources.join("codex-cli/CodexCLI.app/Contents/MacOS/codex"),
                resources.join("codex"),
            ]
            .into_iter()
            .find(|path| access(path) == Access::Accessible && path.is_file());
            installations.push(Installation {
                kind: "desktop",
                version: Some(info.version),
                executable,
            });
        }
    }
    let mut storage = Vec::new();
    let desktop = home.join("Library/Application Support/Codex");
    let has_profile = access(&desktop.join("Last Version")) == Access::Accessible
        && access(&desktop.join("Default")) == Access::Accessible;
    let owned = !installations.is_empty() && has_profile;
    for (kind, path) in [
        (StorageKind::DesktopProfile, desktop.clone()),
        (
            StorageKind::DesktopProfile,
            home.join("Library/Application Support/com.openai.codex"),
        ),
        (StorageKind::Cache, home.join("Library/Caches/Codex")),
        (
            StorageKind::Cache,
            home.join("Library/Caches/com.openai.codex"),
        ),
        (
            StorageKind::Logs,
            home.join("Library/Logs/com.openai.codex"),
        ),
        (
            StorageKind::ArtifactSessionExports,
            desktop.join("artifact-sessions/exports"),
        ),
        (
            StorageKind::Worktrees,
            home.join(".codex-workspaces/worktrees"),
        ),
    ] {
        let item = row(kind, path, owned, owned && kind != StorageKind::Worktrees);
        if item.observed {
            storage.push(item);
        }
    }
    (installations, storage)
}

#[cfg(windows)]
fn windows() -> (Vec<Installation>, Vec<Storage>) {
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Appx {
        name: String,
        publisher_id: String,
        install_location: PathBuf,
        package_family_name: String,
        version: String,
    }
    let mut installations = Vec::new();
    let mut storage = Vec::new();
    let Ok(bytes) = process::codex_windows_packages() else {
        return (installations, storage);
    };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let packages = serde_json::from_slice::<Vec<Appx>>(bytes)
        .or_else(|_| serde_json::from_slice::<Appx>(bytes).map(|package| vec![package]))
        .unwrap_or_default();
    for package in packages.into_iter().take(8) {
        if !matches!(package.name.as_str(), "OpenAI.Codex" | "OpenAI.CodexBeta")
            || package.publisher_id != "2p2nqsd0c76g0"
            || package.package_family_name != format!("{}_{}", package.name, package.publisher_id)
            || !package.install_location.is_absolute()
            || package.version.len() > 48
            || !package
                .version
                .bytes()
                .all(|c| c.is_ascii_digit() || c == b'.')
        {
            continue;
        }
        let path = package.install_location;
        let executable = [
            path.join("app/resources/codex.exe"),
            path.join("resources/codex.exe"),
            path.join("codex.exe"),
        ]
        .into_iter()
        .find(|path| access(path) == Access::Accessible && path.is_file());
        installations.push(Installation {
            kind: "desktop",
            version: Some(package.version),
            executable,
        });
        if let Some(local) = environment("LOCALAPPDATA") {
            let package = local.join("Packages").join(package.package_family_name);
            for (kind, suffix) in [
                (StorageKind::DesktopProfile, "LocalCache/Roaming/Codex"),
                (StorageKind::DesktopProfile, "LocalState"),
                (StorageKind::Cache, "LocalCache/Local/Codex"),
            ] {
                let observation = row(kind, package.join(suffix), true, true);
                if observation.observed {
                    storage.push(observation);
                }
            }
        }
    }
    (installations, storage)
}
