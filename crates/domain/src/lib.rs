//! Provider-neutral observations used by the local manager.
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

mod path_serde;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Codex,
    Claude,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Installed,
    StateOnly,
    NotDetected,
}

#[derive(Debug, Default)]
pub struct Installation {
    pub detected: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionIdentity {
    pub provider: ProviderKind,
    #[serde(with = "path_serde")]
    pub state_root: PathBuf,
    pub namespace: Option<String>,
    pub native_id: String,
}

/// A provider project locator. The contained strings are recorded metadata,
/// never filesystem authorization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProjectLocator {
    Workspace(String),
    Namespace(String),
    Unassigned,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectIdentity {
    pub provider: ProviderKind,
    #[serde(with = "path_serde")]
    pub state_root: PathBuf,
    pub locator: ProjectLocator,
}

impl ProjectIdentity {
    pub fn for_session(session: &Session) -> Self {
        let identity = &session.identity;
        let locator = match identity.provider {
            ProviderKind::Codex => session
                .workspace
                .as_deref()
                .and_then(workspace_identity)
                .map(|path| ProjectLocator::Workspace(path.into())),
            ProviderKind::Claude => identity.namespace.clone().map(ProjectLocator::Namespace),
        };
        Self {
            provider: identity.provider,
            state_root: identity.state_root.clone(),
            locator: locator.unwrap_or(ProjectLocator::Unassigned),
        }
    }
}

/// Keep exact workspace spelling. Ambiguous aliases involving dot segments
/// or repeated separators are unassigned. No filesystem resolution is needed.
pub fn workspace_identity(value: &str) -> Option<&str> {
    if !value.starts_with('/')
        || value.len() <= 1
        || value.len() > 4096
        || value.chars().any(char::is_control)
        || !safe_conversation_text(value)
        || value[1..]
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
    {
        return None;
    }
    Some(value)
}

#[derive(Default)]
pub struct InventoryObservation {
    pub sessions: Vec<Session>,
    pub projects: Vec<ProjectIdentity>,
    pub incomplete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Archived,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Main,
    Subagent,
    Fork,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub identity: SessionIdentity,
    pub title: Option<String>,
    pub workspace: Option<String>,
    pub project: Option<String>,
    pub created_at: Option<i64>,
    pub last_active_at: Option<i64>,
    pub state: SessionState,
    pub transcript_bytes: Option<u64>,
    pub kind: SessionKind,
    pub forked_from: Option<String>,
    pub parent: Option<SessionIdentity>,
}

impl Session {
    pub fn is_main(&self) -> bool {
        self.kind == SessionKind::Main && self.parent.is_none() && self.forked_from.is_none()
    }
}

/// Codex stores UUIDs in lowercase canonical form. Claude locators may use
/// uppercase hex and must still match their own embedded IDs exactly.
pub fn canonical_uuid(id: &str) -> bool {
    uuid_shape(id) && !id.bytes().any(|byte| byte.is_ascii_uppercase())
}

pub fn uuid_shape(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

/// Recognize an observed provider path without resolving it on this host.
/// Provider metadata and fixtures may describe another OS. This is not a safe
/// filesystem access boundary; live roots still require native absolute paths.
pub fn observed_path_is_absolute(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    let Some(text) = path.to_str() else {
        return false;
    };
    let bytes = text.as_bytes();
    text.starts_with('/')
        || text.starts_with("\\\\")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

/// Conservative suppression; never described as perfect secret detection.
pub fn safe_conversation_text(text: &str) -> bool {
    safe_lowered_conversation_text(&text.to_lowercase())
}

fn safe_lowered_conversation_text(lower: &str) -> bool {
    ![
        "-----begin",
        "authorization:",
        "bearer ",
        "sk-",
        "ghp_",
        "github_pat_",
        "api_key",
        "api-key",
        "api key",
        "access_token",
        "refresh_token",
        "password",
        "passwd",
        "secret=",
        "secret:",
        "private_key",
        "client_secret",
        "aws_secret",
        "token=",
        "token:",
        "cookie:",
        "set-cookie:",
        "process.env",
        "printenv",
        "config.toml",
        "[mcp_servers",
        "<environment_context>",
    ]
    .iter()
    .any(|p| lower.contains(p))
}
