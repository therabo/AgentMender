//! Bounded observation of Claude Code project JSONL metadata.
use super::{ClaudeAdapter, transcript_shape};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, filesystem, io_error, validate_location};
use serde::Deserialize;
use std::{io, path::Path};

const PREFIX: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    session_id: Option<String>,
    agent_id: Option<String>,
    is_sidechain: Option<bool>,
    #[serde(rename = "type")]
    kind: Option<String>,
    cwd: Option<String>,
    created_at: Option<String>,
    timestamp: Option<String>,
    custom_title: Option<String>,
    entrypoint: Option<String>,
}

fn timestamp(value: Option<&str>) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value?)
        .ok()
        .map(|time| time.timestamp())
}

fn displayable(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 4096
        && !value.chars().any(char::is_control)
        && safe_conversation_text(value)
}

impl ClaudeAdapter {
    pub fn local_inventory(&self, limit: usize) -> Result<InventoryObservation, ProviderError> {
        validate_location(&self.location)?;
        let projects = self.location.state_root.join("projects");
        let inventory = filesystem::inventory(&projects, limit)?;
        let mut incomplete =
            inventory.incomplete || inventory.truncated || inventory.permission_denied;
        let instances = inventory
            .directories
            .iter()
            .filter_map(|name| {
                name.to_str()
                    .filter(|name| displayable(name) && name.len() <= 900)
            })
            .map(|name| RegisteredProject {
                identity: ProjectIdentity {
                    provider: ProviderKind::Claude,
                    state_root: self.location.state_root.clone(),
                    locator: ProjectLocator::Namespace(name.into()),
                },
                name: None,
                roots: Vec::new(),
                desktop_backed: false,
            })
            .collect();
        let mut sessions = Vec::new();
        for file in inventory.files {
            let locator = Path::new("projects").join(&file.relative_path);
            let Some(shape) = transcript_shape(&locator) else {
                continue;
            };
            let Ok(bytes) = filesystem::read_prefix(&self.location.state_root, &file.path, PREFIX)
            else {
                incomplete = true;
                continue;
            };
            let mut found = false;
            let mut conflict = false;
            let mut sidechain = false;
            let mut workspace = None;
            let mut workspace_ambiguous = false;
            let mut title = None;
            let mut created_at = None;
            let mut last_active_at = None;
            let complete = file.size_bytes <= bytes.len() as u64;
            let mut lines = bytes.split(|b| *b == b'\n').peekable();
            let mut examined = 0;
            while let Some(line) = lines.next() {
                if line.is_empty() {
                    continue;
                }
                if examined == 96 || (!complete && lines.peek().is_none()) {
                    break;
                }
                examined += 1;
                let Ok(record) = serde_json::from_slice::<Record>(line) else {
                    continue;
                };
                let id = record.session_id.as_deref();
                if shape.agent_id.is_none() && record.is_sidechain == Some(true) {
                    sidechain = true;
                    break;
                }
                if id.is_some_and(|id| id != shape.session_id)
                    || record
                        .agent_id
                        .as_ref()
                        .is_some_and(|id| Some(id) != shape.agent_id.as_ref())
                    || record
                        .entrypoint
                        .as_deref()
                        .is_some_and(|entry| !matches!(entry, "cli" | "sdk-cli" | "claude-desktop"))
                {
                    conflict = true;
                    break;
                }
                if id.is_none() {
                    continue;
                }
                found |= match shape.agent_id.as_ref() {
                    Some(agent) => record.agent_id.as_ref() == Some(agent),
                    None => record.agent_id.is_none(),
                };
                if let Some(cwd) = record
                    .cwd
                    .filter(|cwd| displayable(cwd) && observed_path_is_absolute(Path::new(cwd)))
                {
                    workspace_ambiguous |= workspace.as_ref().is_some_and(|prior| prior != &cwd);
                    workspace = Some(cwd);
                }
                if record.kind.as_deref() == Some("custom-title") {
                    title = record.custom_title.filter(|value| displayable(value));
                }
                created_at = created_at.or(timestamp(record.created_at.as_deref()));
                if matches!(record.kind.as_deref(), Some("user" | "assistant")) {
                    last_active_at = last_active_at.max(timestamp(record.timestamp.as_deref()));
                }
            }
            if !found || conflict || sidechain {
                incomplete |= !sidechain;
                continue;
            }
            if workspace_ambiguous {
                workspace = None;
            }
            let root = &self.location.state_root;
            let native = shape.agent_id.as_ref().map_or_else(
                || shape.session_id.clone(),
                |agent| format!("{}/agent:{agent}", shape.session_id),
            );
            if shape.bucket.len() + native.len() > 1000
                || shape.bucket.len() > 900
                || !displayable(&shape.bucket)
            {
                incomplete = true;
                continue;
            }
            let identity = |native_id| SessionIdentity {
                provider: ProviderKind::Claude,
                state_root: root.clone(),
                namespace: Some(shape.bucket.clone()),
                native_id,
            };
            let parent = shape.agent_id.as_ref().map(|_| identity(shape.session_id));
            sessions.push(Session {
                project: workspace
                    .as_ref()
                    .and_then(|cwd| Path::new(cwd).file_name()?.to_str().map(str::to_owned)),
                identity: identity(native),
                codex_membership: CodexMembership::Unassigned,
                title,
                created_at,
                last_active_at: (complete && lines.next().is_none() && examined < 96)
                    .then_some(last_active_at)
                    .flatten(),
                state: SessionState::Unknown,
                workspace,
                transcript_bytes: Some(file.size_bytes),
                kind: if parent.is_some() {
                    SessionKind::Subagent
                } else {
                    SessionKind::Main
                },
                forked_from: None,
                parent,
            });
        }
        if sessions.is_empty() && inventory.permission_denied {
            return Err(io_error(
                projects,
                io::Error::from(io::ErrorKind::PermissionDenied),
            ));
        }
        Ok(InventoryObservation {
            sessions,
            projects: instances,
            incomplete,
            projects_incomplete: incomplete,
            ..Default::default()
        })
    }
}
