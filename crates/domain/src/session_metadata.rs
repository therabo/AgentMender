//! Opt-in metadata for conversational selection. None of these fields grants authority.
use crate::{ArtifactId, Completeness, SessionState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMetadata {
    pub identity: ArtifactId,
    pub native_session_id: String,
    pub title: Option<String>,
    pub project_label: Option<String>,
    pub created_at: Option<i64>,
    pub last_active_at: Option<i64>,
    pub state: SessionState,
    pub producer_version_matches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionInventory {
    pub sessions: Vec<SessionMetadata>,
    pub completeness: Completeness,
}
