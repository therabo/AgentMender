//! Inventory, conversation previews and scoped session removal for Claude Code.
mod artifacts;
#[cfg(target_os = "macos")]
pub mod cleanup;
mod diagnostics;
mod local;
mod read;
mod storage;
mod usage;

pub use artifacts::StorageDetail;
pub use diagnostics::{Access, Reconciliation};
pub use local::{Cursor, SessionEvidence};
pub use storage::{REMOVAL_NOTICE, RootSource, RootView};
pub use usage::StorageInventory;

use agentmender_domain::{ProviderKind, SessionIdentity, uuid_shape};
use agentmender_provider_api::{ProviderError, filesystem, process};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct DesktopContext {
    pub data_root: PathBuf,
    pub app_bundles: Vec<PathBuf>,
}

pub struct ClaudeAdapter {
    roots: Vec<(PathBuf, RootSource)>,
    desktop: Vec<DesktopContext>,
    cache: Mutex<BTreeMap<SessionIdentity, local::Cached>>,
    installations: Vec<InstallationView>,
}

#[derive(Clone, Serialize)]
pub struct InstallationView {
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    pub path: PathBuf,
    pub source: &'static str,
    pub version: Option<String>,
    pub confirmed: bool,
}

impl ClaudeAdapter {
    pub fn new(
        roots: Vec<(PathBuf, RootSource)>,
        desktop: Vec<DesktopContext>,
        cli_candidates: Vec<PathBuf>,
        registered_desktop: Vec<InstallationView>,
    ) -> Result<Self, ProviderError> {
        if roots.is_empty()
            || roots.iter().any(|(path, _)| !local_path(path))
            || desktop.iter().any(|item| !local_path(&item.data_root))
        {
            return Err(ProviderError::InvalidScope(
                "invalid Claude archive roots".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        let roots = roots
            .into_iter()
            .filter(|(path, _)| seen.insert(path.clone()))
            .collect();
        let mut seen = BTreeSet::new();
        let desktop: Vec<_> = desktop
            .into_iter()
            .filter(|item| seen.insert(item.data_root.clone()))
            .collect();
        let mut installations = registered_desktop;
        installations.extend(observe_installations(&desktop, cli_candidates));
        Ok(Self {
            roots,
            desktop,
            cache: Mutex::default(),
            installations,
        })
    }

    pub fn installations(&self) -> &[InstallationView] {
        &self.installations
    }

    pub fn desktop_roots(&self) -> impl Iterator<Item = &Path> {
        self.desktop
            .iter()
            .map(|desktop| desktop.data_root.as_path())
    }
}

fn observe_installations(
    desktops: &[DesktopContext],
    candidates: Vec<PathBuf>,
) -> Vec<InstallationView> {
    let mut installations = Vec::new();
    let mut seen = BTreeSet::new();
    for candidate in candidates
        .into_iter()
        .filter(|path| local_path(path))
        .take(16)
    {
        let Ok(path) = candidate.canonicalize() else {
            continue;
        };
        if safe_path(&path) && path.is_file() && seen.insert(path.clone()) {
            let version = process::probe_version(ProviderKind::Claude, &path).ok();
            installations.push(InstallationView {
                confirmed: version.is_some(),
                path,
                source: "cli",
                version,
            });
        }
    }
    for desktop in desktops {
        for path in &desktop.app_bundles {
            if !safe_path(&path.join("Contents/Info.plist")) || !seen.insert(path.clone()) {
                continue;
            }
            if let Ok(metadata) = process::read_bundle_metadata(path)
                && metadata.identifier == "com.anthropic.claudefordesktop"
            {
                installations.push(InstallationView {
                    path: path.clone(),
                    source: "desktop_bundle",
                    version: Some(metadata.version),
                    confirmed: true,
                });
            }
        }
        let engines = desktop.data_root.join("claude-code");
        if !safe_path(&engines) {
            continue;
        }
        if let Ok(entries) = fs::read_dir(engines) {
            for entry in entries.take(128).flatten() {
                let name = entry.file_name();
                let Some(version) = name.to_str().filter(|value| strict_version(value)) else {
                    continue;
                };
                let bundle = entry.path().join("claude.app");
                let executable = bundle.join("Contents/MacOS/claude");
                if safe_path(&bundle.join("Contents/Info.plist"))
                    && safe_path(&executable)
                    && executable.is_file()
                    && process::read_bundle_metadata(&bundle).is_ok_and(|metadata| {
                        metadata.identifier == "com.anthropic.claude-code"
                            && metadata.executable == "claude"
                            && metadata.version == version
                    })
                {
                    installations.push(InstallationView {
                        path: executable,
                        source: "desktop_engine_bundle",
                        version: Some(version.into()),
                        confirmed: true,
                    });
                }
            }
        }
    }
    installations
}

struct TranscriptShape {
    bucket: String,
    session_id: String,
    agent_id: Option<String>,
}
fn transcript_shape(path: &Path) -> Option<TranscriptShape> {
    let parts: Vec<_> = path.iter().map(|p| p.to_str()).collect::<Option<_>>()?;
    if parts.len() < 3 || parts[0] != "projects" || parts[1].contains(['/', '\\', ':']) {
        return None;
    }
    if parts.len() == 3 {
        let session_id = parts[2].strip_suffix(".jsonl")?;
        return uuid_shape(session_id).then(|| TranscriptShape {
            bucket: parts[1].into(),
            session_id: session_id.into(),
            agent_id: None,
        });
    }
    if parts.len() == 5 && parts[3] == "subagents" && uuid_shape(parts[2]) {
        let agent_id = parts[4].strip_prefix("agent-")?.strip_suffix(".jsonl")?;
        if agent_id.is_empty()
            || agent_id.len() > 128
            || !agent_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return None;
        }
        return Some(TranscriptShape {
            bucket: parts[1].into(),
            session_id: parts[2].into(),
            agent_id: Some(agent_id.into()),
        });
    }
    None
}
fn strict_version(v: &str) -> bool {
    let parts: Vec<_> = v.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
                && p.parse::<u32>().is_ok()
        })
}
fn local_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if !matches!(path.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return false;
        }
    }
    path.is_absolute()
        && path.components().all(|part| {
            !matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}

fn safe_path(path: &Path) -> bool {
    local_path(path)
        && path.ancestors().all(|ancestor| {
            fs::symlink_metadata(ancestor).is_ok_and(|meta| !filesystem::is_link(&meta))
        })
}
