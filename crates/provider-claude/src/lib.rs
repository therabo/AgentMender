//! Read-only detection and project JSONL inventory for Claude Code.
mod local;

use agentmender_domain::{Installation, ProviderKind, uuid_shape};
use agentmender_provider_api::{ProviderError, ProviderLocation, process, validate_location};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct DesktopContext {
    pub data_root: PathBuf,
    pub app_bundles: Vec<PathBuf>,
}

pub struct ClaudeAdapter {
    location: ProviderLocation,
    desktop: Option<DesktopContext>,
}

impl ClaudeAdapter {
    pub fn new(location: ProviderLocation) -> Self {
        Self {
            location,
            desktop: None,
        }
    }

    pub fn with_desktop_context(mut self, context: DesktopContext) -> Self {
        self.desktop = Some(context);
        self
    }

    pub fn detect(&self) -> Result<Installation, ProviderError> {
        validate_location(&self.location)?;
        let mut detected = false;
        let mut versions = Vec::new();
        if let Some(desktop) = &self.desktop {
            detected = desktop.app_bundles.iter().any(|bundle| {
                safe_path(bundle)
                    && process::read_bundle_metadata(bundle).is_ok_and(|metadata| {
                        metadata.identifier == "com.anthropic.claudefordesktop"
                    })
            });
            if let Ok(entries) = fs::read_dir(desktop.data_root.join("claude-code")) {
                for entry in entries.take(128).flatten() {
                    let name = entry.file_name();
                    let Some(version) = name.to_str().filter(|value| strict_version(value)) else {
                        continue;
                    };
                    let bundle = entry.path().join("claude.app");
                    let executable = bundle.join("Contents/MacOS/claude");
                    if !safe_path(&bundle.join("Contents/Info.plist"))
                        || !safe_path(&executable)
                        || !executable.is_file()
                        || !process::read_bundle_metadata(&bundle).is_ok_and(|metadata| {
                            metadata.identifier == "com.anthropic.claude-code"
                                && metadata.executable == "claude"
                                && metadata.version == version
                        })
                    {
                        continue;
                    }
                    match process::probe_version(ProviderKind::Claude, &executable) {
                        Ok(observed) if observed == version => {
                            detected = true;
                            versions.push(observed);
                        }
                        Err(_) => detected = true,
                        Ok(_) => {}
                    }
                }
            }
        }
        let version = if let Some(executable) = &self.location.executable {
            if safe_path(executable) && executable.is_file() {
                detected = true;
                process::probe_version(ProviderKind::Claude, executable).ok()
            } else {
                None
            }
        } else if versions.len() == 1 {
            versions.pop()
        } else {
            None
        };
        Ok(Installation { detected, version })
    }
}

struct TranscriptShape {
    bucket: String,
    session_id: String,
    agent_id: Option<String>,
}
fn transcript_shape(path: &Path) -> Option<TranscriptShape> {
    let parts: Vec<_> = path.iter().map(|p| p.to_str()).collect::<Option<_>>()?;
    if parts.len() < 3 || parts[0] != "projects" {
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
fn safe_path(path: &Path) -> bool {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return false;
    }
    let mut ancestors: Vec<_> = path.ancestors().collect();
    ancestors.reverse();
    ancestors.into_iter().all(|ancestor| {
        ancestor.is_absolute()
            && std::fs::symlink_metadata(ancestor).is_ok_and(|m| !m.file_type().is_symlink())
    })
}
