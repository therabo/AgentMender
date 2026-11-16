//! Read-only metadata for the local session manager.
use crate::{ArtifactId, SessionMetadata};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSessionKind {
    Main,
    Subagent,
    Fork,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSession {
    pub metadata: SessionMetadata,
    pub workspace: Option<String>,
    pub transcript_bytes: Option<u64>,
    pub kind: LocalSessionKind,
    pub forked_from: Option<String>,
    pub parent: Option<ArtifactId>,
}
