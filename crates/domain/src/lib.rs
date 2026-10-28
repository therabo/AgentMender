//! Provider-neutral observations. These types grant no authority to mutate state.
use std::{
    collections::BTreeSet,
    fmt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod path_serde;

pub mod graph;
pub use graph::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("identity must be nonempty, at most 1024 bytes, and contain no control characters")]
pub struct InvalidIdentity;

/// Opaque, provider-scoped identity. Never interpret this value as a filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntityId(String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentity> {
        let value = value.into();
        if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
            return Err(InvalidIdentity);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for EntityId {
    type Error = InvalidIdentity;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<EntityId> for String {
    fn from(value: EntityId) -> Self {
        value.0
    }
}
impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Installed,
    StateOnly,
    /// No installation was detected within the inspected scope.
    #[serde(alias = "missing")]
    NotDetected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    MetadataAudit,
    SessionDiscovery,
    ArchivedSessionDiscovery,
    ParentSessionDiscovery,
    ProjectDiscovery,
    ArtifactDiscovery,
}
pub type ProviderCapabilities = BTreeSet<Capability>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceStability {
    OfficialStable,
    OfficialExperimental,
    InternalImplementationDetail,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    ReadOnlyKnown,
    AuditOnlyUnknown,
}

/// Strength and origin of the evidence behind an observation, independent of compatibility.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    #[default]
    Unverified,
    SyntheticOnly,
    FixtureVerified,
    RealPartial,
    RealVerified,
    SnapshotVerified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationSurface {
    Cli,
    Desktop,
    BundledCli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionSource {
    ExecutableProbe,
    BundleMetadata,
    Fixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInstallation {
    pub surface: InstallationSurface,
    #[serde(with = "path_serde")]
    pub location: PathBuf,
    #[serde(with = "path_serde::optional")]
    pub executable: Option<PathBuf>,
    pub version: Option<String>,
    pub version_source: VersionSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderFingerprint {
    #[serde(with = "path_serde::optional")]
    pub executable: Option<PathBuf>,
    pub executable_version: Option<String>,
    pub adapter_version: String,
    /// Exact schema/profile identifier, not a semantic-version range.
    pub interface_id: Option<String>,
    pub layout_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityProfile {
    pub status: CompatibilityStatus,
    pub interface_stability: InterfaceStability,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub kind: ProviderKind,
    pub presence: Presence,
    #[serde(with = "path_serde")]
    pub state_root: PathBuf,
    pub fingerprint: ProviderFingerprint,
    pub capabilities: ProviderCapabilities,
    pub compatibility: CompatibilityProfile,
    #[serde(default)]
    pub installations: Vec<ProviderInstallation>,
    #[serde(default)]
    pub evidence_status: EvidenceStatus,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    ProviderApi,
    ProviderMetadata,
    ProviderSnapshot,
    FilesystemMetadata,
    Fixture,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Authoritative,
    Strong,
    Heuristic,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub source: EvidenceSource,
    pub confidence: Confidence,
    pub provider_version: Option<String>,
    /// Rule name only; never raw transcript text or credentials.
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: EntityId,
    pub provider: ProviderKind,
    #[serde(with = "path_serde::optional")]
    pub path: Option<PathBuf>,
    pub evidence: Evidence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: EntityId,
    pub workspace_id: Option<EntityId>,
    pub provider_project_id: Option<String>,
    #[serde(with = "path_serde::optional")]
    pub path: Option<PathBuf>,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    Archived,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: EntityId,
    pub provider_session_id: String,
    pub project_id: Option<EntityId>,
    pub state: SessionState,
    pub last_active: Option<String>,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Transcript,
    SessionMetadata,
    Attachment,
    Plan,
    Todo,
    Worktree,
    Index,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: EntityId,
    pub kind: ArtifactKind,
    #[serde(with = "path_serde")]
    pub locator: PathBuf,
    pub size_bytes: Option<u64>,
    pub evidence: Evidence,
    // Ownership is intentionally absent until M2. Every artifact is protected.
}
impl Artifact {
    pub const fn is_protected(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    References,
    SpawnedBy,
    MapsTo,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub source: EntityId,
    pub target: EntityId,
    pub kind: ReferenceKind,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    ProviderNotInstalled,
    ProviderNotDetected,
    SnapshotUnavailable,
    SnapshotStale,
    ProviderVersionUnknown,
    ProviderUnsupported,
    CapabilityUnavailable,
    MalformedProviderState,
    PermissionDenied,
    UnknownArtifact,
    LimitReached,
    LinkSkipped,
    ConcurrentChange,
    IncompleteDiscovery,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    #[serde(with = "path_serde::optional")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    /// Complete only for the declared interface and scope, not all provider stores.
    Complete,
    Partial,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryScope {
    #[serde(with = "path_serde::optional")]
    pub project_path: Option<PathBuf>,
    pub include_archived: bool,
    pub max_entries: usize,
}
impl Default for DiscoveryScope {
    fn default() -> Self {
        Self {
            project_path: None,
            include_archived: true,
            max_entries: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverySnapshot {
    pub provider: Provider,
    #[serde(default)]
    pub evidence_status: EvidenceStatus,
    pub completeness: Completeness,
    /// Human-readable declaration of the inventory boundary.
    pub coverage: String,
    pub workspaces: Vec<Workspace>,
    pub projects: Vec<Project>,
    pub sessions: Vec<Session>,
    pub artifacts: Vec<Artifact>,
    pub references: Vec<Reference>,
    pub diagnostics: Vec<Diagnostic>,
}
impl DiscoverySnapshot {
    pub fn empty(provider: Provider) -> Self {
        let evidence_status = provider.evidence_status;
        Self {
            provider,
            evidence_status,
            completeness: Completeness::Unavailable,
            coverage: "no inventory collected".into(),
            workspaces: Vec::new(),
            projects: Vec::new(),
            sessions: Vec::new(),
            artifacts: Vec::new(),
            references: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
    pub fn normalize(&mut self) {
        self.workspaces.sort_by(|a, b| a.id.cmp(&b.id));
        self.projects.sort_by(|a, b| a.id.cmp(&b.id));
        self.sessions.sort_by(|a, b| a.id.cmp(&b.id));
        self.artifacts.sort_by(|a, b| a.id.cmp(&b.id));
        self.references
            .sort_by(|a, b| (&a.source, &a.target).cmp(&(&b.source, &b.target)));
        self.diagnostics
            .sort_by(|a, b| (&a.path, &a.message).cmp(&(&b.path, &b.message)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn provider() -> Provider {
        Provider {
            kind: ProviderKind::Codex,
            presence: Presence::NotDetected,
            state_root: PathBuf::from("/state"),
            fingerprint: ProviderFingerprint {
                executable: None,
                executable_version: None,
                adapter_version: "test".into(),
                interface_id: None,
                layout_id: None,
            },
            capabilities: ProviderCapabilities::new(),
            compatibility: CompatibilityProfile {
                status: CompatibilityStatus::AuditOnlyUnknown,
                interface_stability: InterfaceStability::Unsupported,
                reason: "unknown".into(),
            },
            installations: Vec::new(),
            evidence_status: EvidenceStatus::Unverified,
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn evidence_status_and_provenance_are_independent_of_compatibility_and_coverage() {
        let mut provider = provider();
        provider.evidence_status = EvidenceStatus::RealPartial;
        let mut snapshot = DiscoverySnapshot::empty(provider);
        assert_eq!(snapshot.evidence_status, EvidenceStatus::RealPartial);
        snapshot.evidence_status = EvidenceStatus::SnapshotVerified;
        snapshot.coverage = "bounded snapshot".into();
        let encoded = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(encoded["evidence_status"], "snapshot_verified");
        assert_eq!(encoded["provider"]["evidence_status"], "real_partial");
        assert_eq!(
            encoded["provider"]["compatibility"]["status"],
            "audit_only_unknown"
        );
        assert_eq!(encoded["completeness"], "unavailable");
        assert_eq!(encoded["coverage"], "bounded snapshot");
        let evidence = Evidence {
            source: EvidenceSource::ProviderSnapshot,
            confidence: Confidence::Strong,
            provider_version: None,
            rule: "mirror".into(),
        };
        assert_eq!(
            serde_json::to_value(evidence).unwrap()["source"],
            "provider_snapshot"
        );
        assert_eq!(
            serde_json::from_value::<DiscoverySnapshot>(encoded).unwrap(),
            snapshot
        );
    }

    #[test]
    fn installation_surfaces_keep_separate_version_provenance() {
        let mut provider = provider();
        provider.installations = vec![
            ProviderInstallation {
                surface: InstallationSurface::Desktop,
                location: "/Applications/Example.app".into(),
                executable: None,
                version: Some("4.29".into()),
                version_source: VersionSource::BundleMetadata,
            },
            ProviderInstallation {
                surface: InstallationSurface::BundledCli,
                location: "/Applications/Example.app/Contents/Resources".into(),
                executable: Some("/Applications/Example.app/Contents/Resources/cli".into()),
                version: Some("2.1.222".into()),
                version_source: VersionSource::ExecutableProbe,
            },
        ];
        let value = serde_json::to_value(&provider).unwrap();
        assert_eq!(
            value["installations"][0]["version_source"],
            "bundle_metadata"
        );
        assert_eq!(value["installations"][1]["surface"], "bundled_cli");
        assert_eq!(value["installations"][1]["version"], "2.1.222");
        assert_eq!(serde_json::from_value::<Provider>(value).unwrap(), provider);
    }

    #[test]
    fn legacy_records_default_to_unverified_and_not_detected_is_bounded() {
        let mut value = serde_json::to_value(provider()).unwrap();
        assert_eq!(value["presence"], "not_detected");
        assert_eq!(
            serde_json::from_str::<Presence>("\"missing\"").unwrap(),
            Presence::NotDetected
        );
        let object = value.as_object_mut().unwrap();
        object.remove("installations");
        object.remove("evidence_status");
        object.remove("diagnostics");
        let decoded: Provider = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.evidence_status, EvidenceStatus::Unverified);
        assert!(decoded.installations.is_empty());
        assert!(decoded.diagnostics.is_empty());
        assert!(decoded.capabilities.is_empty());
        assert_eq!(
            decoded.compatibility.status,
            CompatibilityStatus::AuditOnlyUnknown
        );
        let mut snapshot = serde_json::to_value(DiscoverySnapshot::empty(decoded)).unwrap();
        snapshot.as_object_mut().unwrap().remove("evidence_status");
        assert_eq!(
            serde_json::from_value::<DiscoverySnapshot>(snapshot)
                .unwrap()
                .evidence_status,
            EvidenceStatus::Unverified
        );
        assert_eq!(
            serde_json::to_value(DiagnosticCode::ProviderNotDetected).unwrap(),
            "provider_not_detected"
        );
        assert_eq!(
            serde_json::to_value(DiagnosticCode::SnapshotUnavailable).unwrap(),
            "snapshot_unavailable"
        );
        assert_eq!(
            serde_json::to_value(DiagnosticCode::SnapshotStale).unwrap(),
            "snapshot_stale"
        );
    }

    #[test]
    fn observed_paths_preserve_provider_platform_semantics() {
        for path in [
            "/synthetic/project",
            r"C:\synthetic\project",
            r"\\server\share\project",
        ] {
            assert!(observed_path_is_absolute(Path::new(path)));
        }
        for path in ["relative", r"C:relative"] {
            assert!(!observed_path_is_absolute(Path::new(path)));
        }
    }
    #[test]
    fn identities_are_validated_during_deserialization() {
        for invalid in ["", "a\nb", &"x".repeat(1025)] {
            assert!(EntityId::new(invalid).is_err());
            let encoded = serde_json::to_string(invalid).unwrap();
            assert!(serde_json::from_str::<EntityId>(&encoded).is_err());
        }
        let id = EntityId::new("codex:session:abc").unwrap();
        assert_eq!(
            serde_json::from_str::<EntityId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
    }
    #[test]
    fn every_artifact_is_protected_without_ownership_inference() {
        let artifact = Artifact {
            id: EntityId::new("claude:artifact:a").unwrap(),
            kind: ArtifactKind::Unknown,
            locator: PathBuf::from("untrusted/../path"),
            size_bytes: None,
            evidence: Evidence {
                source: EvidenceSource::FilesystemMetadata,
                confidence: Confidence::Unknown,
                provider_version: None,
                rule: "observed".into(),
            },
        };
        assert!(artifact.is_protected());
    }
}
