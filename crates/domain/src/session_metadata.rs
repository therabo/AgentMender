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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_native_session_id: Option<String>,
}

/// Read capabilities are observations, never mutation or approval authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionReadCompatibility {
    pub metadata_contract: String,
    pub provider_version: Option<String>,
    pub app_server_methods: Vec<String>,
    pub native_interface_status: String,
    pub native_delete_admission: String,
    pub scoped_fallback_admission: String,
    pub additional_cleanup_admission: String,
    pub coverage: String,
    pub relationship_coverage: Completeness,
    pub source_changed_during_capture: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionInventory {
    pub sessions: Vec<SessionMetadata>,
    pub completeness: Completeness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_compatibility: Option<SessionReadCompatibility>,
}
