//! Claude's SDK history helpers require Python/TypeScript. This Rust adapter has
//! no SDK bridge and never starts a conversation to obtain inventory. Live state
//! is metadata-audit-only until an exact release is validated with real captures.
pub mod graph;

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

#[derive(Debug, Clone)]
pub struct DesktopContext {
    pub data_root: PathBuf,
    pub app_bundles: Vec<PathBuf>,
}

pub struct ClaudeAdapter {
    location: ProviderLocation,
    fixture: Option<FixtureManifest>,
    desktop: Option<DesktopContext>,
    desktop_fixture: bool,
}
impl ClaudeAdapter {
    pub fn new(location: ProviderLocation) -> Self {
        Self {
            location,
            fixture: None,
            desktop: None,
            desktop_fixture: false,
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
            desktop: None,
            desktop_fixture: false,
        })
    }

    pub fn with_desktop_context(mut self, context: DesktopContext) -> Self {
        self.desktop = Some(context);
        self
    }

    /// A separately opted-in, sanitized observed fixture; never auto-discovered.
    pub fn from_desktop_fixture(root: PathBuf) -> Result<Self, ProviderError> {
        let root = root
            .canonicalize()
            .map_err(|e| agentmender_provider_api::io_error(&root, e))?;
        let path = root.join(".agentmender-desktop-fixture.json");
        let bytes = filesystem::read_bounded(&root, &path, 4096)?;
        let config: DesktopFixture = serde_json::from_slice(&bytes)
            .map_err(|_| malformed(&path, "invalid desktop fixture provenance"))?;
        if config.provenance != "ObservedLocally"
            || config.desktop_version != "4.29.0"
            || config.code_version != "2.1.222"
        {
            return Err(malformed(&path, "unsupported desktop fixture provenance"));
        }
        Ok(Self {
            location: ProviderLocation {
                state_root: root.join("inventory"),
                executable: None,
            },
            fixture: None,
            desktop: Some(DesktopContext {
                data_root: root.join("data"),
                app_bundles: vec![],
            }),
            desktop_fixture: true,
        })
    }

    fn desktop_evidence(&self, rule: &str) -> Evidence {
        Evidence {
            source: if self.desktop_fixture {
                EvidenceSource::Fixture
            } else {
                EvidenceSource::ProviderMetadata
            },
            confidence: Confidence::Heuristic,
            provider_version: None,
            rule: rule.into(),
        }
    }

    fn fixture_known(&self) -> bool {
        self.fixture
            .as_ref()
            .is_some_and(|f| f.provider_version == FIXTURE_VERSION && f.layout_id == FIXTURE_LAYOUT)
    }

    fn evidence(&self, rule: &str, confidence: Confidence) -> Evidence {
        Evidence {
            source: if self.fixture.is_some() || self.desktop_fixture {
                EvidenceSource::Fixture
            } else {
                EvidenceSource::FilesystemMetadata
            },
            confidence,
            provider_version: if self.desktop_fixture {
                Some("4.29.0".into())
            } else {
                self.fixture.as_ref().map(|f| f.provider_version.clone())
            },
            rule: rule.into(),
        }
    }
}

impl ProviderAdapter for ClaudeAdapter {
    fn build_graph(&self, snapshot: &DiscoverySnapshot) -> Result<ArtifactGraph, ProviderError> {
        graph::build_graph(snapshot)
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    fn detect(&self) -> Result<Provider, ProviderError> {
        if self.fixture.is_none() {
            validate_location(&self.location)?;
        }
        if self.desktop.as_ref().is_some_and(|c| {
            !c.data_root.is_absolute()
                || c.data_root.components().any(|p| {
                    matches!(
                        p,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )
                })
                || c.app_bundles.iter().any(|p| !p.is_absolute())
        }) {
            return Err(ProviderError::InvalidScope(
                "Desktop context paths must be absolute without parent traversal".into(),
            ));
        }
        let mut installations = Vec::new();
        let mut diagnostics = Vec::new();
        let mut desktop_known = false;
        if let Some(context) = &self.desktop {
            if self.desktop_fixture {
                desktop_known = true;
                installations.push(ProviderInstallation {
                    surface: InstallationSurface::Desktop,
                    location: context.data_root.clone(),
                    executable: None,
                    version: Some("4.29.0".into()),
                    version_source: VersionSource::Fixture,
                });
                installations.push(ProviderInstallation {
                    surface: InstallationSurface::BundledCli,
                    location: context.data_root.join("claude-code/2.1.222/claude.app"),
                    executable: None,
                    version: Some("2.1.222".into()),
                    version_source: VersionSource::Fixture,
                });
            } else {
                for bundle in &context.app_bundles {
                    if safe_path(bundle)
                        && let Ok(meta) = process::read_bundle_metadata(bundle)
                        && meta.identifier == "com.anthropic.claudefordesktop"
                        && !meta.version.is_empty()
                    {
                        desktop_known |= meta.version == "4.29.0";
                        installations.push(ProviderInstallation {
                            surface: InstallationSurface::Desktop,
                            location: bundle.clone(),
                            executable: None,
                            version: Some(meta.version),
                            version_source: VersionSource::BundleMetadata,
                        });
                    }
                }
                let code_root = context.data_root.join("claude-code");
                discover_bundles(
                    &code_root,
                    &mut installations,
                    &mut diagnostics,
                    process::read_bundle_metadata,
                    |exe| process::probe_version(ProviderKind::Claude, exe),
                );
            }
        }
        if let Some(exe) = &self.location.executable
            && safe_path(exe)
            && std::fs::symlink_metadata(exe).is_ok_and(|m| m.is_file())
        {
            let version = match process::probe_version(self.kind(), exe) {
                Ok(version) => Some(version),
                Err(_) => {
                    diagnostics.push(diag(
                        DiagnosticCode::CapabilityUnavailable,
                        Some(exe),
                        "sandboxed explicit CLI version probe failed",
                    ));
                    None
                }
            };
            installations.push(ProviderInstallation {
                surface: InstallationSurface::Cli,
                location: exe.clone(),
                executable: Some(exe.clone()),
                version,
                version_source: VersionSource::ExecutableProbe,
            });
        }
        let (selected, ambiguous) =
            select_candidate(&installations, self.location.executable.is_some());
        if ambiguous {
            diagnostics.push(diag(
                DiagnosticCode::CapabilityUnavailable,
                None,
                "multiple bundled CLI candidates; no active runtime inferred",
            ));
        }
        if desktop_known && !desktop_context_versions_known(&installations) {
            desktop_known = false;
            diagnostics.push(diag(DiagnosticCode::CapabilityUnavailable,None,"Desktop versions have conflicting or unknown metadata contexts; no active writer version inferred"));
        }
        let known = self.fixture_known() || desktop_known;
        let mut capabilities =
            BTreeSet::from([Capability::MetadataAudit, Capability::ArtifactDiscovery]);
        if known {
            capabilities.extend([Capability::SessionDiscovery, Capability::ProjectDiscovery]);
        }
        if desktop_known {
            capabilities.insert(Capability::ArchivedSessionDiscovery);
        }
        if self.fixture_known() {
            capabilities.insert(Capability::ParentSessionDiscovery);
        }
        let evidence_status = if self.fixture.is_some() {
            EvidenceStatus::SyntheticOnly
        } else if self.desktop_fixture {
            EvidenceStatus::FixtureVerified
        } else if !installations.is_empty() {
            EvidenceStatus::RealPartial
        } else {
            EvidenceStatus::Unverified
        };
        Ok(Provider {
            kind: self.kind(),
            presence: if !installations.is_empty() {
                Presence::Installed
            } else if self.location.state_root.exists()
                || self.desktop.as_ref().is_some_and(|c| c.data_root.exists())
            {
                Presence::StateOnly
            } else {
                Presence::NotDetected
            },
            state_root: self.location.state_root.clone(),
            fingerprint: ProviderFingerprint {
                executable: selected.and_then(|i| i.executable.clone()),
                executable_version: selected.and_then(|i| i.version.clone()),
                adapter_version: env!("CARGO_PKG_VERSION").into(),
                interface_id: None,
                layout_id: if self.fixture_known() {
                    Some(FIXTURE_LAYOUT.into())
                } else if desktop_known {
                    Some("claude-desktop-4.29.0-metadata-v1".into())
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
                reason: if desktop_known {
                    "Desktop 4.29.0 structural metadata context; historical transcript producers are unknown".into()
                } else if self.fixture_known() {
                    "synthetic fixture contract only".into()
                } else {
                    "unknown Desktop context; metadata audit only".into()
                },
            },
            installations,
            evidence_status,
            diagnostics,
        })
    }

    fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
        validate_scope(scope)?;
        let mut snapshot = DiscoverySnapshot::empty(self.detect()?);
        snapshot
            .diagnostics
            .extend(snapshot.provider.diagnostics.clone());
        if self.desktop.is_some() {
            return self.discover_desktop(scope, snapshot);
        }
        snapshot.coverage = if self.fixture_known() {
            "synthetic fixture contract: bounded transcript metadata prefixes and optional subagent sidecars; no conversation output, ownership, archive status or complete history claims"
        } else {
            "regular-file metadata under configured root only; no transcript or sidecar JSON parsed; live Claude compatibility release gate pending"
        }.into();
        if snapshot.provider.presence == Presence::NotDetected {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::ProviderNotDetected,
                None,
                "Claude executable and configured state root are absent",
            ));
            return Ok(snapshot);
        }
        let inventory = filesystem::inventory(&self.location.state_root, scope.max_entries)?;
        snapshot.diagnostics.extend(inventory.diagnostics);
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
        assert_eq!(snapshot.provider.presence, Presence::NotDetected);
        assert_eq!(snapshot.completeness, Completeness::Unavailable);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::ProviderNotDetected)
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

fn select_candidate(
    installations: &[ProviderInstallation],
    explicit: bool,
) -> (Option<&ProviderInstallation>, bool) {
    let viable = |i: &&ProviderInstallation| {
        i.executable.is_some()
            && i.version.is_some()
            && i.version_source == VersionSource::ExecutableProbe
    };
    if explicit {
        return (
            installations
                .iter()
                .filter(|i| i.surface == InstallationSurface::Cli)
                .find(viable),
            false,
        );
    }
    let candidates: Vec<_> = installations
        .iter()
        .filter(|i| i.surface == InstallationSurface::BundledCli)
        .filter(viable)
        .collect();
    (
        if candidates.len() == 1 {
            Some(candidates[0])
        } else {
            None
        },
        candidates.len() > 1,
    )
}

fn discover_bundles<M, P>(
    root: &Path,
    installations: &mut Vec<ProviderInstallation>,
    diagnostics: &mut Vec<Diagnostic>,
    metadata: M,
    probe: P,
) where
    M: Fn(&Path) -> Result<process::BundleMetadata, ProviderError>,
    P: Fn(&Path) -> Result<String, ProviderError>,
{
    if !safe_path(root) {
        return;
    }
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => {
            diagnostics.push(diag(
                DiagnosticCode::CapabilityUnavailable,
                Some(root),
                "bundled CLI directory could not be read",
            ));
            return;
        }
    };
    for (index, entry) in entries.enumerate() {
        if index == 128 {
            diagnostics.push(diag(
                DiagnosticCode::LimitReached,
                Some(root),
                "bundled CLI version directory limit reached",
            ));
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                diagnostics.push(diag(
                    DiagnosticCode::CapabilityUnavailable,
                    Some(root),
                    "bundled CLI directory entry could not be read",
                ));
                continue;
            }
        };
        let name = entry.file_name();
        let Some(version) = name.to_str().filter(|v| strict_version(v)) else {
            continue;
        };
        let version_dir = entry.path();
        if !safe_path(&version_dir)
            || !std::fs::symlink_metadata(&version_dir).is_ok_and(|m| m.is_dir())
        {
            continue;
        }
        let bundle = version_dir.join("claude.app");
        let executable = bundle.join("Contents/MacOS/claude");
        if !safe_path(&bundle.join("Contents/Info.plist"))
            || !safe_path(&executable)
            || !std::fs::symlink_metadata(&executable).is_ok_and(|m| m.is_file())
        {
            continue;
        }
        let meta = match metadata(&bundle) {
            Ok(meta) => meta,
            Err(_) => {
                diagnostics.push(diag(
                    DiagnosticCode::CapabilityUnavailable,
                    Some(&bundle),
                    "bundled CLI metadata unavailable",
                ));
                continue;
            }
        };
        if meta.identifier != "com.anthropic.claude-code"
            || meta.executable != "claude"
            || meta.version != version
        {
            continue;
        }
        let (observed_version, source) = match probe(&executable) {
            Ok(observed) if observed == meta.version => {
                (Some(observed), VersionSource::ExecutableProbe)
            }
            Ok(_) => {
                diagnostics.push(diag(
                    DiagnosticCode::MalformedProviderState,
                    Some(&bundle),
                    "bundled CLI probe conflicts with bundle version",
                ));
                continue;
            }
            Err(_) => {
                diagnostics.push(diag(
                    DiagnosticCode::CapabilityUnavailable,
                    Some(&executable),
                    "sandboxed bundled CLI probe unavailable; bundle version only",
                ));
                (Some(meta.version), VersionSource::BundleMetadata)
            }
        };
        installations.push(ProviderInstallation {
            surface: InstallationSurface::BundledCli,
            location: bundle,
            executable: Some(executable),
            version: observed_version,
            version_source: source,
        });
    }
}

#[derive(Deserialize)]
struct DesktopFixture {
    provenance: String,
    desktop_version: String,
    code_version: String,
}

fn desktop_context_versions_known(installations: &[ProviderInstallation]) -> bool {
    let desktop: Vec<_> = installations
        .iter()
        .filter(|item| item.surface == InstallationSurface::Desktop)
        .collect();
    !desktop.is_empty()
        && desktop
            .iter()
            .all(|item| item.version.as_deref() == Some("4.29.0"))
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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopMetadata {
    session_id: String,
    cli_session_id: Option<String>,
    cwd: Option<String>,
    origin_cwd: Option<String>,
    created_at: Option<i64>,
    last_activity_at: Option<i64>,
    last_focused_at: Option<i64>,
    is_archived: Option<bool>,
    forked_from_session_id: Option<String>,
}
fn local_id(id: &str) -> bool {
    id.strip_prefix("local_").is_some_and(is_uuid)
}
fn valid_cwd(v: &str) -> bool {
    v.len() <= 768 && !v.chars().any(char::is_control) && observed_path_is_absolute(Path::new(v))
}
fn desktop_shape(path: &Path) -> Option<&str> {
    let p: Vec<_> = path.iter().map(|p| p.to_str()).collect::<Option<_>>()?;
    if p.len() != 3 || !is_uuid(p[0]) || !is_uuid(p[1]) {
        return None;
    }
    let stem = p[2].strip_suffix(".json")?;
    local_id(stem).then_some(stem)
}
impl ClaudeAdapter {
    fn discover_desktop(
        &self,
        scope: &DiscoveryScope,
        mut snapshot: DiscoverySnapshot,
    ) -> Result<DiscoverySnapshot, ProviderError> {
        let context = self
            .desktop
            .as_ref()
            .ok_or_else(|| ProviderError::InvalidScope("Desktop context unavailable".into()))?;
        snapshot.completeness = Completeness::Partial;
        snapshot.evidence_status = EvidenceStatus::Unverified;
        snapshot.coverage = "bounded Desktop sidecars and configured CLI inventory; other histories and session classes omitted; all artifacts protected".into();
        // Divide the entry budget across both independently bounded roots.
        let cli_budget = scope.max_entries / 2;
        let desktop_budget = scope.max_entries - cli_budget;
        let inventory = if cli_budget > 0 && safe_path(&self.location.state_root) {
            filesystem::inventory(&self.location.state_root, cli_budget)?
        } else {
            filesystem::Inventory::default()
        };
        if cli_budget == 0 {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::LimitReached,
                None,
                "CLI inventory entry budget unavailable",
            ));
        }
        snapshot.diagnostics.extend(inventory.diagnostics);
        if inventory.truncated {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::LimitReached,
                None,
                "CLI inventory entry limit reached",
            ));
        }
        if scope.project_path.is_some() {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::IncompleteDiscovery,
                None,
                "unattributed inventory omitted from project scope; ownership is unknown",
            ));
        }
        for file in &inventory.files {
            if scope.project_path.is_some() {
                continue;
            }
            push_artifact(
                self,
                &mut snapshot,
                file,
                if transcript_shape(&file.relative_path).is_some() {
                    ArtifactKind::Transcript
                } else {
                    ArtifactKind::Unknown
                },
            );
        }
        let metadata_root = context.data_root.join("claude-code-sessions");
        let desktop_inventory = if safe_path(&metadata_root) {
            filesystem::inventory(&metadata_root, desktop_budget)?
        } else {
            filesystem::Inventory::default()
        };
        snapshot.diagnostics.extend(desktop_inventory.diagnostics);
        if desktop_inventory.truncated {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::LimitReached,
                None,
                "Desktop metadata entry limit reached",
            ));
        }
        let known = snapshot.provider.compatibility.status == CompatibilityStatus::ReadOnlyKnown;
        if !known {
            snapshot.diagnostics.push(diag(
                DiagnosticCode::ProviderVersionUnknown,
                None,
                "Desktop version is unknown; sidecar contents not interpreted",
            ));
        }
        let mut parsed = Vec::new();
        for file in &desktop_inventory.files {
            let stem = desktop_shape(&file.relative_path);
            let artifact_id = file
                .relative_path
                .to_str()
                .and_then(|p| entity(&format!("claude:desktop-artifact:{p}")));
            if scope.project_path.is_none()
                && let Some(id) = artifact_id
            {
                snapshot.artifacts.push(Artifact {
                    id,
                    kind: if stem.is_some() && known {
                        ArtifactKind::SessionMetadata
                    } else {
                        ArtifactKind::Unknown
                    },
                    locator: file.path.clone(),
                    size_bytes: Some(file.size_bytes),
                    evidence: self.evidence(
                        "desktop_regular_file_metadata_protected",
                        Confidence::Unknown,
                    ),
                });
            }
            let Some(stem) = stem.filter(|_| known) else {
                continue;
            };
            let result = filesystem::read_bounded(&metadata_root, &file.path, 64 * 1024)
                .ok()
                .and_then(|b| serde_json::from_slice::<DesktopMetadata>(&b).ok());
            let Some(m) = result else {
                snapshot.diagnostics.push(diag(
                    DiagnosticCode::MalformedProviderState,
                    Some(&file.path),
                    "Desktop sidecar unreadable, oversized or malformed",
                ));
                continue;
            };
            if m.session_id != stem
                || !local_id(&m.session_id)
                || m.cli_session_id.as_deref().is_some_and(|v| !is_uuid(v))
                || m.forked_from_session_id
                    .as_deref()
                    .is_some_and(|v| !local_id(v))
                || m.cwd.as_deref().is_some_and(|v| !valid_cwd(v))
                || m.origin_cwd.as_deref().is_some_and(|v| !valid_cwd(v))
                || [m.created_at, m.last_activity_at, m.last_focused_at]
                    .into_iter()
                    .flatten()
                    .any(|v| !(0..=253402300799999).contains(&v))
            {
                snapshot.diagnostics.push(diag(
                    DiagnosticCode::MalformedProviderState,
                    Some(&file.path),
                    "Desktop sidecar contains invalid recognized metadata",
                ));
                continue;
            }
            let cwd = m.cwd.as_ref().or(m.origin_cwd.as_ref()).map(PathBuf::from);
            if scope
                .project_path
                .as_ref()
                .is_some_and(|p| cwd.as_ref() != Some(p))
                || (!scope.include_archived && m.is_archived != Some(false))
            {
                continue;
            }
            let namespace = file
                .relative_path
                .parent()
                .and_then(Path::to_str)
                .unwrap_or_default()
                .to_owned();
            let Some(id) = entity(&format!(
                "claude:desktop-session:{namespace}:{}",
                m.session_id
            )) else {
                continue;
            };
            let project_id = cwd.as_ref().and_then(|cwd| {
                let id = entity(&format!("claude:project:{}", cwd.to_string_lossy()))?;
                let workspace_id = entity(&format!("claude:workspace:{}", cwd.to_string_lossy()))?;
                if !snapshot.workspaces.iter().any(|w| w.id == workspace_id) {
                    snapshot.workspaces.push(Workspace {
                        id: workspace_id.clone(),
                        provider: ProviderKind::Claude,
                        path: Some(cwd.clone()),
                        evidence: self.desktop_evidence("explicit_desktop_cwd"),
                    });
                }
                if !snapshot.projects.iter().any(|p| p.id == id) {
                    snapshot.projects.push(Project {
                        id: id.clone(),
                        workspace_id: Some(workspace_id),
                        provider_project_id: None,
                        path: Some(cwd.clone()),
                        evidence: self.desktop_evidence("explicit_desktop_cwd"),
                    });
                }
                Some(id)
            });
            snapshot.sessions.push(Session {
                id: id.clone(),
                provider_session_id: m.session_id.clone(),
                project_id,
                state: match m.is_archived {
                    Some(true) => SessionState::Archived,
                    Some(false) => SessionState::Active,
                    None => SessionState::Unknown,
                },
                last_active: m
                    .last_activity_at
                    .or(m.last_focused_at)
                    .map(|v| v.to_string()),
                evidence: self
                    .desktop_evidence("exact_fields_observed_with_desktop_4_29_unknown_producer"),
            });
            if scope.project_path.is_some()
                && let Some(artifact_id) = file
                    .relative_path
                    .to_str()
                    .and_then(|p| entity(&format!("claude:desktop-artifact:{p}")))
            {
                snapshot.artifacts.push(Artifact {
                    id: artifact_id,
                    kind: ArtifactKind::SessionMetadata,
                    locator: file.path.clone(),
                    size_bytes: Some(file.size_bytes),
                    evidence: self.evidence(
                        "desktop_regular_file_metadata_protected",
                        Confidence::Unknown,
                    ),
                });
            }
            parsed.push((id, namespace, m));
        }
        let mut cli_counts = BTreeMap::new();
        for (_, _, metadata) in &parsed {
            if let Some(cli) = &metadata.cli_session_id {
                *cli_counts.entry(cli).or_insert(0usize) += 1;
            }
        }
        for (id, namespace, m) in &parsed {
            if let Some(fork) = &m.forked_from_session_id {
                if let Some((target, _, _)) = parsed.iter().find(|(_, ns, other)| {
                    ns == namespace && &other.session_id == fork && other.session_id != m.session_id
                }) {
                    snapshot.references.push(Reference {
                        source: id.clone(),
                        target: target.clone(),
                        kind: ReferenceKind::References,
                        evidence: self.desktop_evidence("explicit_forkedFromSessionId"),
                    });
                } else {
                    snapshot.diagnostics.push(diag(
                        DiagnosticCode::IncompleteDiscovery,
                        None,
                        "explicit Desktop fork target is outside observed scope",
                    ));
                }
            }
            if let Some(cli_id) = &m.cli_session_id {
                if cli_counts.get(cli_id).copied().unwrap_or(0) > 1 {
                    snapshot.diagnostics.push(diag(
                        DiagnosticCode::IncompleteDiscovery,
                        None,
                        "multiple Desktop sessions reference the same CLI session",
                    ));
                }
                let matches: Vec<_> = inventory
                    .files
                    .iter()
                    .filter(|f| {
                        transcript_shape(&f.relative_path)
                            .is_some_and(|s| s.agent_id.is_none() && s.session_id == *cli_id)
                    })
                    .collect();
                if matches.len() == 1 {
                    if scope.project_path.is_some()
                        && !snapshot
                            .artifacts
                            .iter()
                            .any(|a| a.locator == matches[0].path)
                    {
                        push_artifact(self, &mut snapshot, matches[0], ArtifactKind::Transcript);
                    }
                    if let Some(target) = matches[0]
                        .relative_path
                        .to_str()
                        .and_then(|p| entity(&format!("claude:artifact:{p}")))
                    {
                        snapshot.references.push(Reference {
                            source: id.clone(),
                            target,
                            kind: ReferenceKind::MapsTo,
                            evidence: self.desktop_evidence("explicit_cliSessionId"),
                        });
                    }
                } else {
                    snapshot.diagnostics.push(diag(
                        DiagnosticCode::IncompleteDiscovery,
                        None,
                        if matches.is_empty() {
                            "explicit CLI session has no observed root transcript"
                        } else {
                            "explicit CLI session maps to multiple root transcripts"
                        },
                    ));
                }
            }
        }
        if known && !parsed.is_empty() {
            snapshot.evidence_status = if self.desktop_fixture {
                EvidenceStatus::FixtureVerified
            } else {
                EvidenceStatus::RealPartial
            };
        }
        snapshot.normalize();
        Ok(snapshot)
    }
}

#[cfg(test)]
mod desktop_tests {
    use super::*;
    #[test]
    fn safe_path_accepts_existing_absolute_paths_and_rejects_invalid_paths() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let file = nested.join("file");
        std::fs::write(&file, "fixture").unwrap();
        let file = file.canonicalize().unwrap();
        assert!(safe_path(&file));
        assert!(!safe_path(&nested.join("missing")));
        assert!(!safe_path(Path::new("relative/file")));
        // PathBuf::join normalizes parent segments for Windows verbatim paths.
        // Preserve the literal traversal in the input presented to the validator.
        let mut traversal = file.into_os_string();
        traversal.push(format!("{0}..{0}file", std::path::MAIN_SEPARATOR));
        assert!(!safe_path(Path::new(&traversal)));
    }
    #[test]
    fn conflicting_desktop_versions_do_not_establish_a_known_context() {
        let installation = |version: &str| ProviderInstallation {
            surface: InstallationSurface::Desktop,
            location: PathBuf::from("/fixture/Claude.app"),
            executable: None,
            version: Some(version.into()),
            version_source: VersionSource::Fixture,
        };
        assert!(desktop_context_versions_known(&[installation("4.29.0")]));
        assert!(!desktop_context_versions_known(&[
            installation("4.29.0"),
            installation("4.30.0")
        ]));
    }
    fn observed() -> ClaudeAdapter {
        ClaudeAdapter::from_desktop_fixture(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/claude/4.29.0-observed"),
        )
        .unwrap()
    }
    #[test]
    fn observed_metadata_maps_only_explicit_ids() {
        let s = observed().discover(&DiscoveryScope::default()).unwrap();
        assert_eq!(s.sessions.len(), 7);
        assert_eq!(
            s.references
                .iter()
                .filter(|r| r.kind == ReferenceKind::MapsTo)
                .count(),
            7
        );
        assert_eq!(
            s.references
                .iter()
                .filter(|r| r.kind == ReferenceKind::References)
                .count(),
            1
        );
        assert_eq!(
            s.references
                .iter()
                .filter(|r| r.kind == ReferenceKind::SpawnedBy)
                .count(),
            0
        );
        assert_eq!(s.evidence_status, EvidenceStatus::FixtureVerified);
        assert_eq!(s.completeness, Completeness::Partial);
        assert_eq!(s.artifacts.len(), 26);
        assert!(s.artifacts.iter().all(Artifact::is_protected));
    }
    #[test]
    fn desktop_ids_and_versions_are_strict() {
        assert!(strict_version("2.1.222"));
        assert!(!strict_version("2.01.222"));
        assert!(!strict_version("2.1.222-beta"));
        assert!(desktop_shape(Path::new("00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002/local_00000000-0000-0000-0000-000000000003.json")).is_some());
        assert!(desktop_shape(Path::new("scheduled-tasks.json")).is_none());
    }
    #[test]
    fn duplicate_fields_are_rejected_without_values_in_output() {
        let content =
            br#"{"sessionId":"local_00000000-0000-0000-0000-000000000001","sessionId":"PRIVATE_SENTINEL_VALUE"}"#;
        assert!(serde_json::from_slice::<DesktopMetadata>(content).is_err());
    }
    #[test]
    fn unknown_context_never_parses_sidecars() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("claude-code-sessions/00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002/local_00000000-0000-0000-0000-000000000003.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "PRIVATE_SECRET_SENTINEL").unwrap();
        let s = ClaudeAdapter::new(ProviderLocation {
            state_root: root.path().join("inventory"),
            executable: None,
        })
        .with_desktop_context(DesktopContext {
            data_root: root.path().into(),
            app_bundles: vec![],
        })
        .discover(&DiscoveryScope::default())
        .unwrap();
        assert!(s.sessions.is_empty());
        assert!(
            !serde_json::to_string(&s)
                .unwrap()
                .contains("PRIVATE_SECRET_SENTINEL")
        );
    }
    fn temp_desktop(content: &str) -> (tempfile::TempDir, ClaudeAdapter) {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".agentmender-desktop-fixture.json"), r#"{"provenance":"ObservedLocally","desktop_version":"4.29.0","code_version":"2.1.222"}"#).unwrap();
        let dir = root.path().join("data/claude-code-sessions/00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("local_00000000-0000-0000-0000-000000000003.json"),
            content,
        )
        .unwrap();
        let adapter = ClaudeAdapter::from_desktop_fixture(root.path().into()).unwrap();
        (root, adapter)
    }
    #[test]
    fn malformed_and_mismatched_sidecars_stay_protected() {
        for content in [
            r#"{"sessionId":"local_00000000-0000-0000-0000-000000000004"}"#,
            r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003","createdAt":"bad"}"#,
            r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003","cwd":"relative"}"#,
            r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003","sessionId":"PRIVATE_SENTINEL_VALUE"}"#,
        ] {
            let (_root, adapter) = temp_desktop(content);
            let s = adapter.discover(&DiscoveryScope::default()).unwrap();
            assert!(s.sessions.is_empty());
            assert!(s.artifacts.iter().all(Artifact::is_protected));
            assert!(
                s.diagnostics
                    .iter()
                    .any(|d| d.code == DiagnosticCode::MalformedProviderState)
            );
            assert!(
                !serde_json::to_string(&s)
                    .unwrap()
                    .contains("PRIVATE_SENTINEL_VALUE")
            );
        }
    }
    #[test]
    fn desktop_without_cli_is_installed_and_limits_are_partial() {
        let (_root, adapter) =
            temp_desktop(r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003"}"#);
        let s = adapter
            .discover(&DiscoveryScope {
                max_entries: 1,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(s.provider.presence, Presence::Installed);
        assert_eq!(s.completeness, Completeness::Partial);
        assert!(
            s.diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::LimitReached)
        );
    }
    #[cfg(unix)]
    #[test]
    fn desktop_links_are_not_followed() {
        use std::os::unix::fs::symlink;
        let (root, adapter) =
            temp_desktop(r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003"}"#);
        let dir = root.path().join("data/claude-code-sessions/00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002");
        let path = dir.join("local_00000000-0000-0000-0000-000000000003.json");
        std::fs::remove_file(&path).unwrap();
        symlink("/etc/passwd", &path).unwrap();
        let s = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert!(s.sessions.is_empty());
        assert!(
            s.diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::LinkSkipped)
        );
    }
    #[test]
    fn direct_bundle_candidates_are_bounded_and_ambiguous() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        for version in ["2.1.222", "2.1.223"] {
            let bundle = root.join(version).join("claude.app/Contents");
            std::fs::create_dir_all(bundle.join("MacOS")).unwrap();
            std::fs::write(bundle.join("Info.plist"), "fixture").unwrap();
            std::fs::write(bundle.join("MacOS/claude"), "fixture").unwrap();
            // A large Resources tree must not be searched.
            for i in 0..140 {
                std::fs::create_dir_all(bundle.join(format!("Resources/{i}"))).unwrap();
            }
        }
        let mut installations = Vec::new();
        let mut diagnostics = Vec::new();
        discover_bundles(
            &root,
            &mut installations,
            &mut diagnostics,
            |bundle| {
                Ok(process::BundleMetadata {
                    identifier: "com.anthropic.claude-code".into(),
                    executable: "claude".into(),
                    version: bundle
                        .parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .into(),
                })
            },
            |exe| {
                Ok(exe
                    .ancestors()
                    .nth(4)
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into())
            },
        );
        assert_eq!(installations.len(), 2);
        assert!(select_candidate(&installations, false).1);
        assert!(select_candidate(&installations, false).0.is_none());
        installations.pop();
        assert!(select_candidate(&installations, false).0.is_some());
        installations[0].version_source = VersionSource::BundleMetadata;
        assert!(select_candidate(&installations, false).0.is_none());
    }
    #[test]
    fn namespace_and_project_filter_remain_distinct() {
        let (root, adapter) = temp_desktop(
            r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003","cwd":"/project-a","cliSessionId":"00000000-0000-0000-0000-000000000005"}"#,
        );
        let other = root.path().join("data/claude-code-sessions/00000000-0000-0000-0000-000000000009/00000000-0000-0000-0000-000000000002");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("local_00000000-0000-0000-0000-000000000003.json"), r#"{"sessionId":"local_00000000-0000-0000-0000-000000000003","cwd":"/project-b","forkedFromSessionId":"local_00000000-0000-0000-0000-000000000003"}"#).unwrap();
        let inv = root.path().join("inventory/projects/bucket");
        std::fs::create_dir_all(&inv).unwrap();
        std::fs::write(
            inv.join("00000000-0000-0000-0000-000000000005.jsonl"),
            "body never read",
        )
        .unwrap();
        std::fs::write(
            inv.join("00000000-0000-0000-0000-000000000006.jsonl"),
            "unrelated",
        )
        .unwrap();
        let all = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert_eq!(all.sessions.len(), 2);
        assert_ne!(all.sessions[0].id, all.sessions[1].id);
        let scoped = adapter
            .discover(&DiscoveryScope {
                project_path: Some("/project-a".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(scoped.sessions.len(), 1);
        assert_eq!(scoped.artifacts.len(), 2);
        assert!(
            scoped
                .artifacts
                .iter()
                .all(|a| !a.locator.to_string_lossy().contains("000000000006"))
        );
        assert_eq!(
            scoped
                .references
                .iter()
                .filter(|r| r.kind == ReferenceKind::MapsTo)
                .count(),
            1
        );
        assert!(
            !serde_json::to_string(&scoped)
                .unwrap()
                .contains("body never read")
        );
    }
}

#[cfg(test)]
mod integrity_tests;
