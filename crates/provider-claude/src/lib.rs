//! Claude's SDK history helpers require Python/TypeScript. This Rust adapter has
//! no SDK bridge and never starts a conversation to obtain inventory. Live state
//! is metadata-audit-only until an exact release is validated with real captures.
use agentmender_domain::*;
use agentmender_provider_api::{
    ProviderAdapter, ProviderError, ProviderLocation, filesystem, process, validate_location,
    validate_scope,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

const FIXTURE_VERSION: &str = "2.1.311";
const FIXTURE_LAYOUT: &str = "claude-jsonl-2.1.311-synthetic-v1";
const PREFIX_BYTES: usize = 256 * 1024;
const MAX_LINES: usize = 128;
const MAX_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureManifest {
    schema_version: u32,
    provider: String,
    provider_version: String,
    layout_id: String,
    synthetic: bool,
}

pub struct ClaudeAdapter {
    location: ProviderLocation,
    fixture: Option<FixtureManifest>,
}
impl ClaudeAdapter {
    pub fn new(location: ProviderLocation) -> Self {
        Self {
            location,
            fixture: None,
        }
    }

    /// Opt-in test-data boundary. Never infers a live installation's version.
    pub fn from_fixture(root: PathBuf) -> Result<Self, ProviderError> {
        let path = root.join(".agentmender-fixture.json");
        let bytes = filesystem::read_bounded(&root, &path, 4096)?;
        let manifest: FixtureManifest = serde_json::from_slice(&bytes)
            .map_err(|_| malformed(&path, "invalid fixture provenance manifest"))?;
        if manifest.schema_version != 1 || manifest.provider != "claude" || !manifest.synthetic {
            return Err(malformed(
                &path,
                "only explicitly synthetic Claude fixture manifests are accepted",
            ));
        }
        Ok(Self {
            location: ProviderLocation {
                state_root: root,
                executable: None,
            },
            fixture: Some(manifest),
        })
    }

    fn fixture_known(&self) -> bool {
        self.fixture
            .as_ref()
            .is_some_and(|f| f.provider_version == FIXTURE_VERSION && f.layout_id == FIXTURE_LAYOUT)
    }

    fn evidence(&self, rule: &str, confidence: Confidence) -> Evidence {
        Evidence {
            source: if self.fixture.is_some() {
                EvidenceSource::Fixture
            } else {
                EvidenceSource::FilesystemMetadata
            },
            confidence,
            provider_version: self.fixture.as_ref().map(|f| f.provider_version.clone()),
            rule: rule.into(),
        }
    }
}

impl ProviderAdapter for ClaudeAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    fn detect(&self) -> Result<Provider, ProviderError> {
        if self.fixture.is_none() {
            validate_location(&self.location)?;
        }
        let probe = self
            .location
            .executable
            .as_deref()
            .map(|exe| process::probe_version(self.kind(), exe));
        let version = probe
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .cloned();
        let root_exists = std::fs::symlink_metadata(&self.location.state_root).is_ok();
        let known = self.fixture_known();
        let mut capabilities =
            BTreeSet::from([Capability::MetadataAudit, Capability::ArtifactDiscovery]);
        if known {
            capabilities.extend([
                Capability::SessionDiscovery,
                Capability::ProjectDiscovery,
                Capability::ParentSessionDiscovery,
            ]);
        }
        Ok(Provider {
            kind: self.kind(),
            presence: if self
                .location
                .executable
                .as_ref()
                .is_some_and(|path| path.is_file())
            {
                Presence::Installed
            } else if root_exists {
                Presence::StateOnly
            } else {
                Presence::Missing
            },
            state_root: self.location.state_root.clone(),
            fingerprint: ProviderFingerprint {
                executable: self.location.executable.clone(),
                executable_version: version,
                adapter_version: env!("CARGO_PKG_VERSION").into(),
                interface_id: None,
                layout_id: if known {
                    Some(FIXTURE_LAYOUT.into())
                } else {
                    None
                },
            },
            capabilities,
            compatibility: CompatibilityProfile {
                status: if known {
                    CompatibilityStatus::ReadOnlyKnown
                } else {
                    CompatibilityStatus::AuditOnlyUnknown
                },
                interface_stability: InterfaceStability::InternalImplementationDetail,
                reason: if known {
                    "synthetic fixture contract only; no real Claude release capture has validated this layout".into()
                } else if let Some(Err(error)) = probe {
                    format!("{error}; live transcript parsing remains unavailable")
                } else {
                    "live exact-release layout evidence pending; SDK history helpers require a runtime bridge that is not implemented; transcript contents are not parsed".into()
                },
            },
        })
    }

    fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
        validate_scope(scope)?;
        let mut snapshot = DiscoverySnapshot::empty(self.detect()?);
        snapshot.coverage = if self.fixture_known() {
            "synthetic fixture contract: bounded transcript metadata prefixes and optional subagent sidecars; no conversation output, ownership, archive status or complete history claims"
        } else {
            "regular-file metadata under configured root only; no transcript or sidecar JSON parsed; live Claude compatibility release gate pending"
        }.into();
        if snapshot.provider.presence == Presence::Missing {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::ProviderNotInstalled,
                None,
                "Claude executable and configured state root are absent",
            ));
            return Ok(snapshot);
        }
        let inventory = filesystem::inventory(&self.location.state_root, scope.max_entries)?;
        snapshot.diagnostics = inventory.diagnostics;
        snapshot.completeness = if inventory.truncated || !snapshot.diagnostics.is_empty() {
            Completeness::Partial
        } else {
            Completeness::Complete
        };
        if !self.fixture_known() {
            snapshot.completeness = Completeness::Partial;
            snapshot.diagnostics.push(diag(DiagnosticCode::ProviderVersionUnknown, None, "no validated live Claude transcript profile; session and project discovery unavailable"));
            if self.location.executable.is_some()
                && snapshot.provider.fingerprint.executable_version.is_none()
            {
                snapshot.diagnostics.push(diag(DiagnosticCode::CapabilityUnavailable, None, "write-denied provider version probe unavailable or failed; no unsandboxed retry"));
            }
            if scope.project_path.is_some() {
                snapshot.diagnostics.push(diag(DiagnosticCode::CapabilityUnavailable, None, "project filtering requires validated explicit project metadata; no files attributed to this project"));
                return Ok(snapshot);
            }
            for file in inventory.files {
                push_artifact(self, &mut snapshot, &file, ArtifactKind::Unknown);
            }
            snapshot.normalize();
            return Ok(snapshot);
        }

        if !scope.include_archived {
            snapshot.completeness = Completeness::Partial;
            snapshot.diagnostics.push(diag(DiagnosticCode::CapabilityUnavailable, None, "Claude archive status is unknown; active-only filtering cannot establish session membership"));
            for file in inventory
                .files
                .iter()
                .filter(|_| scope.project_path.is_none())
            {
                if file.relative_path != Path::new(".agentmender-fixture.json") {
                    push_artifact(self, &mut snapshot, file, ArtifactKind::Unknown);
                }
            }
            snapshot.normalize();
            return Ok(snapshot);
        }

        let mut parsed = Vec::new();
        for file in &inventory.files {
            if file.relative_path == Path::new(".agentmender-fixture.json") {
                continue;
            }
            if let Some(shape) = transcript_shape(&file.relative_path) {
                match parse_metadata(&self.location.state_root, file, &shape) {
                    Ok(metadata) => parsed.push((file, shape, metadata)),
                    Err(message) => {
                        snapshot.diagnostics.push(diag(
                            DiagnosticCode::MalformedProviderState,
                            Some(&file.path),
                            message,
                        ));
                        snapshot.completeness = Completeness::Partial;
                    }
                }
            }
        }

        let mut session_ids = BTreeMap::new();
        let mut records = Vec::new();
        for (file, shape, metadata) in parsed {
            if metadata.limited {
                snapshot.diagnostics.push(diag(
                    DiagnosticCode::LimitReached,
                    Some(&file.path),
                    "transcript metadata prefix budget reached; later records were not examined",
                ));
                snapshot.completeness = Completeness::Partial;
            }
            if scope
                .project_path
                .as_ref()
                .is_some_and(|p| metadata.cwd.as_ref() != Some(p))
            {
                continue;
            }
            let role = shape
                .agent_id
                .as_ref()
                .map_or_else(|| "root".into(), |agent| format!("agent:{agent}"));
            let Some(id) = entity(&format!(
                "claude:session:{}:{}:{}",
                shape.bucket, shape.session_id, role
            )) else {
                snapshot.diagnostics.push(diag(
                    DiagnosticCode::MalformedProviderState,
                    Some(&file.path),
                    "session identity exceeds bounds",
                ));
                snapshot.completeness = Completeness::Partial;
                continue;
            };
            let project_id = metadata.cwd.as_ref().and_then(|cwd| {
                let id = entity(&format!("claude:project:{}", cwd.to_string_lossy()))?;
                let workspace_id = entity(&format!("claude:workspace:{}", cwd.to_string_lossy()))?;
                if !snapshot.workspaces.iter().any(|w| w.id == workspace_id) {
                    snapshot.workspaces.push(Workspace {
                        id: workspace_id.clone(),
                        provider: ProviderKind::Claude,
                        path: Some(cwd.clone()),
                        evidence: self.evidence(
                            "explicit_cwd_workspace_observation_no_existence_claim",
                            Confidence::Heuristic,
                        ),
                    });
                }
                if !snapshot.projects.iter().any(|p| p.id == id) {
                    snapshot.projects.push(Project {
                        id: id.clone(),
                        workspace_id: Some(workspace_id),
                        provider_project_id: None,
                        path: Some(cwd.clone()),
                        evidence: self.evidence(
                            "explicit_transcript_cwd_not_decoded_bucket",
                            Confidence::Heuristic,
                        ),
                    });
                }
                Some(id)
            });
            if project_id.is_none() {
                snapshot.diagnostics.push(diag(
                    DiagnosticCode::IncompleteDiscovery,
                    Some(&file.path),
                    "no usable explicit cwd; project mapping remains unknown",
                ));
                snapshot.completeness = Completeness::Partial;
            }
            let provider_session_id = shape.agent_id.as_ref().map_or_else(
                || shape.session_id.clone(),
                |agent| format!("{}/agent:{agent}", shape.session_id),
            );
            snapshot.sessions.push(Session {
                id: id.clone(),
                provider_session_id,
                project_id,
                state: SessionState::Unknown,
                last_active: metadata.timestamp,
                evidence: self
                    .evidence("synthetic_explicit_session_metadata", Confidence::Heuristic),
            });
            session_ids.insert(
                (
                    shape.bucket.clone(),
                    shape.session_id.clone(),
                    shape.agent_id.clone(),
                ),
                id.clone(),
            );
            records.push((file, shape, id));
        }
        let included: BTreeSet<_> = records
            .iter()
            .map(|(file, _, _)| file.relative_path.clone())
            .collect();
        let mut included_sidecars = BTreeSet::new();
        for (file, shape, id) in records {
            if let Some(agent_id) = shape.agent_id.as_ref() {
                let sidecar = file.path.with_extension("meta.json");
                if let Some(meta_file) = inventory.files.iter().find(|f| f.path == sidecar) {
                    included_sidecars.insert(meta_file.relative_path.clone());
                    match read_sidecar(&self.location.state_root, &meta_file.path) {
                        Ok(meta) => {
                            if let Some(parent_agent) = meta.parent_agent_id {
                                if parent_agent == *agent_id {
                                    snapshot.diagnostics.push(diag(
                                        DiagnosticCode::MalformedProviderState,
                                        Some(&sidecar),
                                        "self-referencing parent agent metadata rejected",
                                    ));
                                    snapshot.completeness = Completeness::Partial;
                                } else if let Some(parent) = session_ids.get(&(
                                    shape.bucket.clone(),
                                    shape.session_id.clone(),
                                    Some(parent_agent),
                                )) {
                                    snapshot.references.push(Reference {
                                        source: id.clone(),
                                        target: parent.clone(),
                                        kind: ReferenceKind::SpawnedBy,
                                        evidence: self.evidence(
                                            "explicit_sidecar_parentAgentId",
                                            Confidence::Heuristic,
                                        ),
                                    });
                                } else {
                                    snapshot.diagnostics.push(diag(DiagnosticCode::IncompleteDiscovery, Some(&sidecar), "explicit parent agent could not be resolved in observed scope"));
                                    snapshot.completeness = Completeness::Partial;
                                }
                            }
                        }
                        Err(_) => {
                            snapshot.diagnostics.push(diag(
                                DiagnosticCode::MalformedProviderState,
                                Some(&sidecar),
                                "subagent metadata sidecar is unreadable or malformed",
                            ));
                            snapshot.completeness = Completeness::Partial;
                        }
                    }
                }
                // sessionId groups a subagent with its root transcript. It does
                // not establish its immediate spawning parent or ownership.
                if let Some(root) = session_ids.get(&(shape.bucket, shape.session_id, None)) {
                    snapshot.references.push(Reference {
                        source: id.clone(),
                        target: root.clone(),
                        kind: ReferenceKind::MapsTo,
                        evidence: self.evidence(
                            "explicit_subagent_sessionId_groups_root",
                            Confidence::Heuristic,
                        ),
                    });
                }
            }
        }
        for file in &inventory.files {
            if file.relative_path == Path::new(".agentmender-fixture.json") {
                continue;
            }
            if scope.project_path.is_some()
                && !included.contains(&file.relative_path)
                && !included_sidecars.contains(&file.relative_path)
            {
                continue;
            }
            let kind = if included.contains(&file.relative_path) {
                ArtifactKind::Transcript
            } else if included_sidecars.contains(&file.relative_path) {
                ArtifactKind::SessionMetadata
            } else {
                ArtifactKind::Unknown
            };
            if kind == ArtifactKind::Unknown {
                snapshot.diagnostics.push(diag(DiagnosticCode::UnknownArtifact, Some(&file.path), "uninterpreted file remains protected; no session or project ownership inferred"));
                snapshot.completeness = Completeness::Partial;
            }
            push_artifact(self, &mut snapshot, file, kind);
        }
        snapshot.normalize();
        Ok(snapshot)
    }
}

fn malformed(path: &Path, detail: &str) -> ProviderError {
    ProviderError::MalformedProviderState {
        path: path.into(),
        detail: detail.into(),
    }
}
fn entity(value: &str) -> Option<EntityId> {
    EntityId::new(value).ok()
}
fn diag(code: DiagnosticCode, path: Option<&Path>, message: &str) -> Diagnostic {
    Diagnostic {
        code,
        message: message.into(),
        path: path.map(Path::to_path_buf),
    }
}
fn push_artifact(
    adapter: &ClaudeAdapter,
    snapshot: &mut DiscoverySnapshot,
    file: &filesystem::ObservedFile,
    kind: ArtifactKind,
) {
    if let Some(id) = file
        .relative_path
        .to_str()
        .and_then(|path| entity(&format!("claude:artifact:{path}")))
    {
        snapshot.artifacts.push(Artifact {
            id,
            kind,
            locator: file.path.clone(),
            size_bytes: Some(file.size_bytes),
            evidence: adapter.evidence("regular_file_metadata_protected", Confidence::Unknown),
        });
    } else {
        snapshot.diagnostics.push(diag(
            DiagnosticCode::UnknownArtifact,
            Some(&file.path),
            "artifact identity cannot be represented within bounds",
        ));
        snapshot.completeness = Completeness::Partial;
    }
}

#[derive(Debug)]
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
        return is_uuid(session_id).then(|| TranscriptShape {
            bucket: parts[1].into(),
            session_id: session_id.into(),
            agent_id: None,
        });
    }
    if parts.len() == 5 && parts[3] == "subagents" && is_uuid(parts[2]) {
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
fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

// Serde discards body/unknown fields; diagnostics never include input or JSON errors.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataLine {
    version: Option<String>,
    session_id: Option<String>,
    agent_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
}
struct TranscriptMetadata {
    cwd: Option<PathBuf>,
    timestamp: Option<String>,
    limited: bool,
}
fn parse_metadata(
    root: &Path,
    file: &filesystem::ObservedFile,
    shape: &TranscriptShape,
) -> Result<TranscriptMetadata, &'static str> {
    let bytes = filesystem::read_prefix(root, &file.path, PREFIX_BYTES)
        .map_err(|_| "transcript metadata prefix could not be read safely")?;
    let mut cwd = None;
    let mut timestamp = None;
    let mut confirmed_version = false;
    let mut confirmed_session = false;
    let mut confirmed_agent = shape.agent_id.is_none();
    let mut limited = file.size_bytes > bytes.len() as u64;
    let lines: Vec<_> = bytes.split(|b| *b == b'\n').collect();
    for (index, line) in lines.iter().enumerate() {
        if index == MAX_LINES {
            limited = true;
            break;
        }
        if line.is_empty() {
            continue;
        }
        if limited && index + 1 == lines.len() {
            break;
        }
        if line.len() > MAX_LINE_BYTES {
            return Err("transcript metadata line exceeds byte limit");
        }
        let metadata: MetadataLine = serde_json::from_slice(line)
            .map_err(|_| "transcript metadata is malformed or has unexpected field types")?;
        if let Some(version) = metadata.version {
            if version != FIXTURE_VERSION {
                return Err(
                    "transcript metadata version differs from pinned synthetic fixture contract",
                );
            }
            confirmed_version = true;
        }
        if let Some(id) = metadata.session_id {
            if id != shape.session_id {
                return Err("explicit sessionId conflicts with transcript locator");
            }
            confirmed_session = true;
        }
        if let Some(agent_id) = metadata.agent_id {
            if Some(&agent_id) != shape.agent_id.as_ref() {
                return Err("explicit agentId conflicts with transcript locator");
            }
            confirmed_agent = true;
        }
        if let Some(path) = metadata.cwd {
            if path.len() > 768
                || path.chars().any(char::is_control)
                || !observed_path_is_absolute(Path::new(&path))
            {
                return Err("explicit cwd is invalid or exceeds bounds");
            }
            let path = PathBuf::from(path);
            if cwd.as_ref().is_some_and(|previous| previous != &path) {
                return Err("conflicting explicit cwd values require manual project resolution");
            }
            cwd = Some(path);
        }
        if let Some(value) = metadata.timestamp {
            if value.len() > 64 || value.chars().any(char::is_control) {
                return Err("timestamp metadata is invalid");
            }
            timestamp = Some(value);
        }
    }
    if !confirmed_version || !confirmed_session || !confirmed_agent {
        return Err(
            "pinned version and explicit session/subagent identity were not established within metadata prefix",
        );
    }
    Ok(TranscriptMetadata {
        cwd,
        timestamp,
        limited,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMetadata {
    parent_agent_id: Option<String>,
}
fn read_sidecar(root: &Path, path: &Path) -> Result<AgentMetadata, ProviderError> {
    let bytes = filesystem::read_bounded(root, path, 4096)?;
    let metadata: AgentMetadata = serde_json::from_slice(&bytes)
        .map_err(|_| malformed(path, "invalid agent metadata sidecar"))?;
    if metadata.parent_agent_id.as_ref().is_some_and(|id| {
        id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    }) {
        return Err(malformed(path, "invalid parent agent identity"));
    }
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_only_does_not_guess_unknown_archive_membership() {
        let adapter = ClaudeAdapter::from_fixture(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/claude/2.1.311-synthetic"),
        )
        .unwrap();
        let snapshot = adapter
            .discover(&DiscoveryScope {
                include_archived: false,
                ..Default::default()
            })
            .unwrap();
        assert!(snapshot.sessions.is_empty());
        assert_eq!(snapshot.completeness, Completeness::Partial);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::CapabilityUnavailable)
        );
    }
    use std::fs;
    const ROOT_SESSION: &str = "11111111-1111-4111-8111-111111111111";

    fn write(root: &Path, path: &str, bytes: impl AsRef<[u8]>) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            ".agentmender-fixture.json",
            include_bytes!("../../../fixtures/claude/2.1.311-synthetic/.agentmender-fixture.json"),
        );
        root
    }
    fn main_line(cwd: &str) -> String {
        serde_json::json!({"type":"user", "version":FIXTURE_VERSION, "sessionId":ROOT_SESSION, "cwd":cwd, "parentUuid":"message-parent-is-not-a-session-parent", "message":{"content":"PRIVATE_SECRET_SENTINEL"}}).to_string()
    }
    fn scan(root: &Path) -> DiscoverySnapshot {
        ClaudeAdapter::from_fixture(root.into())
            .unwrap()
            .discover(&DiscoveryScope::default())
            .unwrap()
    }
    fn main_path() -> String {
        format!("projects/custom-bucket/{ROOT_SESSION}.jsonl")
    }

    #[test]
    fn missing_provider_is_visible_without_reading_state() {
        let root = tempfile::tempdir().unwrap();
        let adapter = ClaudeAdapter::new(ProviderLocation {
            state_root: root.path().join("missing"),
            executable: None,
        });
        let snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert_eq!(snapshot.provider.presence, Presence::Missing);
        assert_eq!(snapshot.completeness, Completeness::Unavailable);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::ProviderNotInstalled)
        );
    }
    #[test]
    fn live_state_even_matching_fixture_version_is_metadata_only() {
        let root = fixture();
        write(root.path(), &main_path(), b"not JSON; must never be parsed");
        let adapter = ClaudeAdapter::new(ProviderLocation {
            state_root: root.path().into(),
            executable: None,
        });
        let snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert_eq!(
            snapshot.provider.compatibility.status,
            CompatibilityStatus::AuditOnlyUnknown
        );
        assert!(snapshot.sessions.is_empty());
        assert!(snapshot.projects.is_empty());
        assert!(
            !snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::MalformedProviderState)
        );
        assert_eq!(snapshot.artifacts.len(), 2);
        assert!(snapshot.artifacts.iter().all(Artifact::is_protected));
    }
    #[test]
    fn unknown_fixture_profile_is_never_used_to_parse_transcripts() {
        let root = fixture();
        write(
            root.path(),
            ".agentmender-fixture.json",
            r#"{"schema_version":1,"provider":"claude","provider_version":"2.1.312","layout_id":"unknown","synthetic":true}"#,
        );
        write(root.path(), &main_path(), "bad JSON");
        let snapshot = scan(root.path());
        assert_eq!(
            snapshot.provider.compatibility.status,
            CompatibilityStatus::AuditOnlyUnknown
        );
        assert!(snapshot.sessions.is_empty());
        assert!(
            !snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::MalformedProviderState)
        );
    }
    #[test]
    fn empty_fixture_is_complete_for_its_declared_boundary() {
        let root = fixture();
        let snapshot = scan(root.path());
        assert_eq!(snapshot.completeness, Completeness::Complete);
        assert!(snapshot.sessions.is_empty());
        assert!(snapshot.provider.fingerprint.executable_version.is_none());
        assert!(snapshot.provider.compatibility.reason.contains("synthetic"));
    }
    #[test]
    fn explicit_cwd_maps_project_and_body_never_leaves_adapter() {
        let root = fixture();
        write(root.path(), &main_path(), main_line("/actual-project"));
        let snapshot = scan(root.path());
        assert_eq!(snapshot.sessions.len(), 1);
        assert_eq!(
            snapshot.projects[0].path,
            Some(PathBuf::from("/actual-project"))
        );
        assert_eq!(snapshot.sessions[0].state, SessionState::Unknown);
        assert!(snapshot.references.is_empty());
        assert!(
            !serde_json::to_string(&snapshot)
                .unwrap()
                .contains("PRIVATE_SECRET_SENTINEL")
        );
        assert!(
            !serde_json::to_string(&snapshot)
                .unwrap()
                .contains("message-parent-is-not-a-session-parent")
        );
    }
    #[test]
    fn checked_in_synthetic_fixture_groups_projects_and_nested_parent() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/claude/2.1.311-synthetic");
        let snapshot = scan(&root);
        assert_eq!(snapshot.completeness, Completeness::Complete);
        assert_eq!(snapshot.projects.len(), 2);
        assert_eq!(snapshot.workspaces.len(), 2);
        assert!(snapshot.projects.iter().all(|p| p.workspace_id.is_some()));
        assert_eq!(snapshot.sessions.len(), 4);
        assert_eq!(
            snapshot
                .references
                .iter()
                .filter(|r| r.kind == ReferenceKind::SpawnedBy)
                .count(),
            1
        );
        assert_eq!(
            snapshot
                .references
                .iter()
                .filter(|r| r.kind == ReferenceKind::MapsTo)
                .count(),
            2
        );
        assert!(snapshot.artifacts.iter().all(Artifact::is_protected));
        let filtered = ClaudeAdapter::from_fixture(root)
            .unwrap()
            .discover(&DiscoveryScope {
                project_path: Some("/synthetic/project-b".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.sessions.len(), 1);
        assert_eq!(filtered.projects.len(), 1);
        assert_eq!(filtered.artifacts.len(), 1);
        assert!(
            !serde_json::to_string(&snapshot)
                .unwrap()
                .contains("SYNTHETIC_PRIVATE_BODY")
        );
    }
    #[test]
    fn malformed_conflicting_and_future_records_are_partial_protected_artifacts() {
        for content in [
            "bad JSON".into(),
            main_line("/project") + "\n{broken and PRIVATE_SECRET_SENTINEL",
            main_line("/project").replace(FIXTURE_VERSION, "2.1.312"),
            main_line("/project") + "\n" + &main_line("/other"),
            main_line("/project").replace(ROOT_SESSION, "22222222-2222-4222-8222-222222222222"),
            format!(
                "{{\"version\":\"{FIXTURE_VERSION}\",\"sessionId\":\"{ROOT_SESSION}\",\"sessionId\":\"conflicting-id\"}}"
            ),
        ] {
            let root = fixture();
            write(root.path(), &main_path(), content);
            let snapshot = scan(root.path());
            assert_eq!(snapshot.completeness, Completeness::Partial);
            assert!(snapshot.sessions.is_empty());
            assert_eq!(snapshot.artifacts.len(), 1);
            assert_eq!(snapshot.artifacts[0].kind, ArtifactKind::Unknown);
            assert!(
                snapshot
                    .diagnostics
                    .iter()
                    .any(|d| d.code == DiagnosticCode::MalformedProviderState)
            );
            assert!(
                !serde_json::to_string(&snapshot)
                    .unwrap()
                    .contains("PRIVATE_SECRET_SENTINEL")
            );
        }
    }
    #[test]
    fn first_metadata_line_is_found_in_large_transcript_with_visible_prefix_limit() {
        let root = fixture();
        let mut content = main_line("/project") + "\n";
        for _ in 0..20000 {
            content.push_str("{\"type\":\"progress\"}\n");
        }
        write(root.path(), &main_path(), content);
        let snapshot = scan(root.path());
        assert_eq!(snapshot.sessions.len(), 1);
        assert_eq!(snapshot.completeness, Completeness::Partial);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::LimitReached)
        );
    }
    #[cfg(unix)]
    #[test]
    fn links_and_external_transcripts_are_skipped() {
        use std::os::unix::fs::symlink;
        let root = fixture();
        let other = tempfile::tempdir().unwrap();
        write(other.path(), "secret", main_line("/external"));
        fs::create_dir_all(root.path().join("projects/custom-bucket")).unwrap();
        symlink(other.path().join("secret"), root.path().join(main_path())).unwrap();
        let snapshot = scan(root.path());
        assert!(snapshot.sessions.is_empty());
        assert_eq!(snapshot.completeness, Completeness::Partial);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::LinkSkipped)
        );
    }
    #[test]
    fn subagent_requires_explicit_agent_identity_and_no_parent_is_invented() {
        let root = fixture();
        write(root.path(), &main_path(), main_line("/project"));
        write(
            root.path(),
            &format!("projects/custom-bucket/{ROOT_SESSION}/subagents/agent-child.jsonl"),
            main_line("/project"),
        );
        let snapshot = scan(root.path());
        assert_eq!(snapshot.sessions.len(), 1);
        assert!(snapshot.references.is_empty());
        assert_eq!(snapshot.completeness, Completeness::Partial);
    }
}
