//! Codex metadata observations through the official, version-specific App Server API.
//! Real SQLite is observed only by the isolated evidence worker; no lifecycle
//! mutation is exposed through the read-only adapter. Lifecycle methods are
//! receipt-gated in a separate private module; no transcript readers are exposed.
use agentmender_domain::*;
use agentmender_provider_api::{
    ProviderAdapter, ProviderError, ProviderLocation, filesystem, process, validate_location,
    validate_scope,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Child,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod capture;
#[cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]
mod content;
#[cfg(unix)]
mod desktop_catalog;
#[cfg(unix)]
mod desktop_catalog_capture;
#[cfg(unix)]
mod desktop_cleanup;
#[cfg(unix)]
mod desktop_json;
#[cfg(unix)]
mod desktop_legacy;
#[cfg(unix)]
mod desktop_shared;
pub mod graph;
mod lifecycle;
#[cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]
mod metadata;
mod rollout_reader;
pub use content::run_content_capture_worker;
mod planning;
pub use metadata::run_metadata_capture_worker;
#[cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]
mod scoped_capture;
#[cfg(unix)]
mod scoped_cleanup;
#[cfg(unix)]
mod scoped_config;
#[cfg(unix)]
mod scoped_files;
#[cfg_attr(not(unix), allow(dead_code))]
mod scoped_profile;
#[cfg_attr(not(unix), allow(dead_code))]
mod scoped_storage;
pub use capture::run_evidence_capture_worker;
#[cfg(unix)]
pub use desktop_catalog_capture::run_catalog_evidence_worker;
pub use lifecycle::CodexLifecycleAdapter;
pub use scoped_capture::{
    ScopedCaptureRequest, ScopedCaptureResult, ScopedRowCommitment, run_scoped_evidence_worker,
};
#[cfg(unix)]
pub use scoped_cleanup::CodexScopedCleanupAdapter;
#[cfg(not(unix))]
mod scoped_unsupported;
#[cfg(not(unix))]
pub use scoped_unsupported::CodexScopedCleanupAdapter;

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINE: usize = 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 16 * 1024;
const SOURCES: [&str; 10] = [
    "cli",
    "vscode",
    "exec",
    "appServer",
    "subAgent",
    "subAgentReview",
    "subAgentCompact",
    "subAgentThreadSpawn",
    "subAgentOther",
    "unknown",
];

pub struct CodexAdapter {
    location: ProviderLocation,
    fixture: Option<Fixture>,
    evidence_helper: Option<(PathBuf, PathBuf)>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    kind: String,
    schema_version: String,
    complete: bool,
    pages: Vec<FixturePage>,
    #[serde(default)]
    provenance: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixturePage {
    archived: bool,
    response: Value,
}
#[derive(Deserialize)]
struct InitializeResponse {
    #[serde(rename = "codexHome")]
    codex_home: PathBuf,
    #[serde(rename = "platformFamily")]
    _platform_family: String,
    #[serde(rename = "platformOs")]
    _platform_os: String,
    #[serde(rename = "userAgent")]
    _user_agent: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    data: Vec<WireThread>,
    next_cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireThread {
    id: String,
    cwd: PathBuf,
    updated_at: i64,
    #[serde(default)]
    parent_thread_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    status: WireStatus,
}
#[derive(Deserialize)]
#[serde(tag = "type")]
enum WireStatus {
    #[serde(rename = "notLoaded")]
    NotLoaded,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "systemError")]
    SystemError,
    #[serde(rename = "active")]
    Active {
        #[serde(rename = "activeFlags")]
        _active_flags: Vec<String>,
    },
}
struct Page {
    archived: bool,
    result: ListResponse,
}

impl CodexAdapter {
    pub fn new(location: ProviderLocation) -> Self {
        Self {
            location,
            fixture: None,
            evidence_helper: None,
        }
    }
    /// The CLI supplies its own fixed internal capture entry point. No helper is
    /// inferred by library callers, fixture tests, or unsupported installations.
    pub fn with_evidence_helper(mut self, helper: PathBuf, real_home: PathBuf) -> Self {
        self.evidence_helper = Some((helper, real_home));
        self
    }
    /// Synthetic, sanitized protocol responses only. The manifest is never a live version probe.
    pub fn from_fixture(root: PathBuf) -> Result<Self, ProviderError> {
        let path = root.join("manifest.json");
        let bytes = filesystem::read_bounded(&root, &path, MAX_BYTES)?;
        let fixture: Fixture = serde_json::from_slice(&bytes)
            .map_err(|_| malformed(&path, "invalid fixture manifest"))?;
        if fixture.kind != "agentmender_codex_app_server_fixture" {
            return Err(malformed(&path, "fixture provenance marker missing"));
        }
        if fixture
            .provenance
            .as_deref()
            .is_some_and(|value| value != "observed_locally_sanitized")
        {
            return Err(malformed(&path, "unsupported fixture provenance"));
        }
        Ok(Self {
            location: ProviderLocation {
                state_root: root,
                executable: None,
            },
            fixture: Some(fixture),
            evidence_helper: None,
        })
    }
    fn provider(&self) -> Provider {
        let (version, probe_failure) = if let Some(fixture) = &self.fixture {
            (Some(fixture.schema_version.clone()), None)
        } else if let Some(exe) = self
            .location
            .executable
            .as_deref()
            .filter(|exe| exe.is_file())
        {
            match process::probe_version(ProviderKind::Codex, exe) {
                Ok(version) => (Some(version), None),
                Err(error) => (None, Some(error.to_string())),
            }
        } else {
            (None, None)
        };
        let profile = version.as_deref().and_then(capture::ExactProfile::lookup);
        let known = profile.is_some();
        let reason = if let Some(failure) = probe_failure {
            format!("Version probe unavailable under read-only boundary: {failure}")
        } else if self.fixture.is_some() {
            if known { "Synthetic fixture of the exact generated App Server schema; not a live executable assertion" } else { "Unknown synthetic schema; filesystem metadata audit only" }.into()
        } else if known {
            "Exact generated protocol recognized; live discovery requires a scrubbed SQLite snapshot and OS-enforced isolation".into()
        } else {
            "No verified exact schema; filesystem metadata audit only".into()
        };
        Provider {
            kind: ProviderKind::Codex,
            presence: if self.fixture.is_some()
                || self
                    .location
                    .executable
                    .as_deref()
                    .is_some_and(Path::is_file)
            {
                Presence::Installed
            } else if self.location.state_root.exists() {
                Presence::StateOnly
            } else {
                Presence::NotDetected
            },
            state_root: self.location.state_root.clone(),
            fingerprint: ProviderFingerprint {
                executable: self.location.executable.clone(),
                executable_version: if self.fixture.is_some() {
                    None
                } else {
                    version.clone()
                },
                adapter_version: env!("CARGO_PKG_VERSION").into(),
                interface_id: profile.map(|profile| profile.interface_id().into()),
                layout_id: None,
            },
            capabilities: BTreeSet::from([Capability::MetadataAudit]),
            compatibility: CompatibilityProfile {
                status: if known {
                    CompatibilityStatus::ReadOnlyKnown
                } else {
                    CompatibilityStatus::AuditOnlyUnknown
                },
                interface_stability: if known {
                    InterfaceStability::OfficialExperimental
                } else {
                    InterfaceStability::Unsupported
                },
                reason,
            },
            installations: self
                .location
                .executable
                .as_ref()
                .filter(|exe| exe.is_file())
                .map(|exe| ProviderInstallation {
                    surface: if exe
                        .components()
                        .any(|part| part.as_os_str() == "Applications")
                    {
                        InstallationSurface::BundledCli
                    } else {
                        InstallationSurface::Cli
                    },
                    location: exe.clone(),
                    executable: Some(exe.clone()),
                    version: version.clone(),
                    version_source: VersionSource::ExecutableProbe,
                })
                .into_iter()
                .collect(),
            evidence_status: if let Some(fixture) = &self.fixture {
                if fixture.provenance.is_some() {
                    EvidenceStatus::FixtureVerified
                } else {
                    EvidenceStatus::SyntheticOnly
                }
            } else if self
                .location
                .executable
                .as_ref()
                .is_some_and(|p| p.is_file())
                || self.location.state_root.exists()
            {
                EvidenceStatus::RealPartial
            } else {
                EvidenceStatus::Unverified
            },
            diagnostics: Vec::new(),
        }
    }
}
impl ProviderAdapter for CodexAdapter {
    fn build_graph(&self, snapshot: &DiscoverySnapshot) -> Result<ArtifactGraph, ProviderError> {
        graph::build_graph(snapshot)
    }
    fn planning_context(
        &self,
        graph: &ArtifactGraph,
        scope: &DiscoveryScope,
    ) -> Result<PlanningContext, ProviderError> {
        planning::planning_context(graph, scope)
    }
    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }
    fn detect(&self) -> Result<Provider, ProviderError> {
        if self.fixture.is_none() {
            validate_location(&self.location)?;
        }
        Ok(self.provider())
    }
    fn discover(&self, scope: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
        validate_scope(scope)?;
        if self.fixture.is_none() {
            validate_location(&self.location)?;
        }
        let provider = self.provider();
        let mut snapshot = DiscoverySnapshot::empty(provider);
        snapshot.completeness = Completeness::Partial;
        snapshot.coverage = "Codex state-root regular-file metadata; every artifact protected. Native metadata, when available, comes from an exact-schema, scrubbed SQLite Backup API mirror through official thread/list; database rows, native summaries and rollout-file counts are distinct; partial across provider stores and not global runtime activity.".into();
        let inventory = filesystem::inventory(&self.location.state_root, scope.max_entries)?;
        snapshot.diagnostics = inventory.diagnostics;
        if scope.project_path.is_some() && !inventory.files.is_empty() {
            diag(
                &mut snapshot,
                DiagnosticCode::IncompleteDiscovery,
                "Unattributed provider files omitted from project-filtered inventory; file paths do not establish project ownership",
            );
        } else {
            for file in inventory.files {
                let artifact_id = artifact_identity(&file.relative_path);
                let Some(artifact_id) = artifact_id else {
                    diag(
                        &mut snapshot,
                        DiagnosticCode::UnknownArtifact,
                        "Regular file path cannot be represented as a valid lossless identity; protected metadata entry omitted",
                    );
                    continue;
                };
                snapshot.artifacts.push(Artifact {
                    id: artifact_id,
                    kind: ArtifactKind::Unknown,
                    locator: file.path,
                    size_bytes: Some(file.size_bytes),
                    evidence: Evidence {
                        source: EvidenceSource::FilesystemMetadata,
                        confidence: Confidence::Unknown,
                        provider_version: snapshot.provider.fingerprint.executable_version.clone(),
                        rule: "protected_regular_file_metadata_only".into(),
                    },
                });
            }
        }
        if snapshot.provider.presence == Presence::NotDetected {
            diag(
                &mut snapshot,
                DiagnosticCode::ProviderNotDetected,
                "Codex executable and state root were not detected in the inspected scope",
            );
        }
        if snapshot.provider.compatibility.status == CompatibilityStatus::AuditOnlyUnknown {
            let code = if self.fixture.is_none()
                && self
                    .location
                    .executable
                    .as_deref()
                    .is_some_and(Path::is_file)
                && snapshot.provider.fingerprint.executable_version.is_none()
            {
                DiagnosticCode::CapabilityUnavailable
            } else if snapshot.provider.fingerprint.executable_version.is_none() {
                DiagnosticCode::ProviderVersionUnknown
            } else {
                DiagnosticCode::ProviderUnsupported
            };
            diag(
                &mut snapshot,
                code,
                "Exact Codex protocol schema unavailable; file contents were not parsed",
            );
            snapshot.normalize();
            return Ok(snapshot);
        }
        let mut capture_observation = None;
        let result = if let Some(fixture) = &self.fixture {
            fixture_pages(fixture, scope)
        } else if let (Some(exe), Some((helper, real_home))) =
            (&self.location.executable, &self.evidence_helper)
        {
            evidence_pages(
                exe,
                &self.location.state_root,
                helper,
                real_home,
                scope,
                snapshot
                    .provider
                    .fingerprint
                    .executable_version
                    .as_deref()
                    .and_then(capture::ExactProfile::lookup)
                    .expect("known profile"),
            )
            .map(|(pages, complete, observation)| {
                capture_observation = Some(observation);
                (pages, complete)
            })
        } else {
            Err(ProviderError::CapabilityUnavailable(
                "Codex disposable evidence helper unavailable".into(),
            ))
        };
        match result {
            Ok((pages, complete)) => {
                let source = if self.fixture.is_some() {
                    EvidenceSource::Fixture
                } else {
                    EvidenceSource::ProviderSnapshot
                };
                let evidence = Evidence {
                    source,
                    confidence: if self
                        .fixture
                        .as_ref()
                        .is_some_and(|fixture| fixture.provenance.is_none())
                    {
                        Confidence::Heuristic
                    } else {
                        Confidence::Strong
                    },
                    provider_version: if let Some(fixture) = &self.fixture {
                        Some(fixture.schema_version.clone())
                    } else {
                        snapshot.provider.fingerprint.executable_version.clone()
                    },
                    rule: "official_thread_list_metadata_use_state_db_only".into(),
                };
                let mut seen = BTreeSet::new();
                let mut projects = BTreeSet::new();
                for page in pages {
                    for record in page.result.data {
                        if !observed_path_is_absolute(&record.cwd) || record.updated_at < 0 {
                            diag(
                                &mut snapshot,
                                DiagnosticCode::MalformedProviderState,
                                "Thread contains invalid metadata; record skipped",
                            );
                            continue;
                        }
                        if scope
                            .project_path
                            .as_ref()
                            .is_some_and(|path| path != &record.cwd)
                        {
                            continue;
                        }
                        if snapshot.sessions.len() >= scope.max_entries {
                            diag(
                                &mut snapshot,
                                DiagnosticCode::LimitReached,
                                "Thread entry budget reached",
                            );
                            break;
                        }
                        let session_id = match scoped_id("session", &record.id) {
                            Ok(id) => id,
                            Err(_) => {
                                diag(
                                    &mut snapshot,
                                    DiagnosticCode::MalformedProviderState,
                                    "Thread identity invalid; record skipped",
                                );
                                continue;
                            }
                        };
                        let identities = record.cwd.to_str().and_then(|cwd| {
                            let workspace = scoped_id("workspace", cwd).ok()?;
                            let project =
                                scoped_id("project", record.project_id.as_deref().unwrap_or(cwd))
                                    .ok()?;
                            Some((workspace, project))
                        });
                        let Some((workspace_id, project_id)) = identities else {
                            diag(
                                &mut snapshot,
                                DiagnosticCode::MalformedProviderState,
                                "Thread workspace or project identity invalid; record skipped",
                            );
                            continue;
                        };
                        if !seen.insert(session_id.clone()) {
                            diag(
                                &mut snapshot,
                                DiagnosticCode::ConcurrentChange,
                                "Duplicate thread observed across pages; first observation retained",
                            );
                            continue;
                        }
                        if !snapshot.workspaces.iter().any(|w| w.id == workspace_id) {
                            snapshot.workspaces.push(Workspace {
                                id: workspace_id.clone(),
                                provider: ProviderKind::Codex,
                                path: Some(record.cwd.clone()),
                                evidence: evidence.clone(),
                            });
                        }
                        if projects.insert(project_id.clone()) {
                            let mut project_evidence = evidence.clone();
                            if record.project_id.is_none() {
                                project_evidence.confidence = Confidence::Heuristic;
                                project_evidence.rule =
                                    "cwd_group_without_native_project_assignment".into();
                            }
                            snapshot.projects.push(Project {
                                id: project_id.clone(),
                                workspace_id: Some(workspace_id),
                                provider_project_id: record.project_id,
                                path: Some(record.cwd),
                                evidence: project_evidence,
                            });
                        } else if let Some(project) =
                            snapshot.projects.iter_mut().find(|p| p.id == project_id)
                            && project
                                .path
                                .as_ref()
                                .is_some_and(|path| path != &record.cwd)
                        {
                            // One native project can span worktrees/current directories.
                            // Do not promote the first observed cwd to its canonical root.
                            project.path = None;
                            project.workspace_id = None;
                        }
                        if let Some(parent) = record.parent_thread_id {
                            match scoped_id("session", &parent) {
                                Ok(target) => snapshot.references.push(Reference {
                                    source: session_id.clone(),
                                    target,
                                    kind: ReferenceKind::SpawnedBy,
                                    evidence: evidence.clone(),
                                }),
                                Err(_) => diag(
                                    &mut snapshot,
                                    DiagnosticCode::MalformedProviderState,
                                    "Parent identity invalid; relationship skipped",
                                ),
                            }
                        }
                        let _observed_runtime_status = record.status;
                        snapshot.sessions.push(Session {
                            id: session_id,
                            provider_session_id: record.id,
                            project_id: Some(project_id),
                            state: if page.archived {
                                SessionState::Archived
                            } else {
                                SessionState::Active
                            },
                            last_active: Some(record.updated_at.to_string()),
                            evidence: evidence.clone(),
                        });
                    }
                }
                snapshot.provider.capabilities.extend([
                    Capability::SessionDiscovery,
                    Capability::ParentSessionDiscovery,
                    Capability::ProjectDiscovery,
                ]);
                if scope.include_archived {
                    snapshot
                        .provider
                        .capabilities
                        .insert(Capability::ArchivedSessionDiscovery);
                }
                if !complete {
                    diag(
                        &mut snapshot,
                        DiagnosticCode::IncompleteDiscovery,
                        "App Server pages incomplete for requested archive scope",
                    );
                }
                if let Some(observation) = capture_observation {
                    let native_count = snapshot.sessions.len();
                    snapshot.coverage.push_str(&format!(
                        " Captured database rows: {} active, {} archived; nonempty previews: {} active, {} archived; native summaries returned for requested scope: {native_count}. Filesystem artifact counts describe separately bounded metadata observations.",
                        observation.manifest.active_rows, observation.manifest.archived_rows,
                        observation.manifest.nonempty_active_previews, observation.manifest.nonempty_archived_previews));
                    snapshot.provider.fingerprint.layout_id = Some(format!(
                        "codex-state-5:{}",
                        observation.manifest.schema_hash
                    ));
                    snapshot.evidence_status = if complete && !observation.stale {
                        EvidenceStatus::SnapshotVerified
                    } else {
                        EvidenceStatus::RealPartial
                    };
                    if observation.stale {
                        diag(
                            &mut snapshot,
                            DiagnosticCode::SnapshotStale,
                            "Source DB/WAL/SHM changed or could not be fingerprinted across capture and enumeration; the SQLite capture remains coherent but is stale relative to live source activity",
                        );
                        diag(
                            &mut snapshot,
                            DiagnosticCode::ConcurrentChange,
                            "Independent source changes are outside the isolated worker write-denial guarantee",
                        );
                    }
                } else if let Some(fixture) = &self.fixture {
                    snapshot.evidence_status = if fixture.provenance.is_some() {
                        EvidenceStatus::FixtureVerified
                    } else {
                        EvidenceStatus::SyntheticOnly
                    };
                }
                // Metadata audit has intentionally unknown files and App Server omits other stores.
                snapshot.completeness = Completeness::Partial;
            }
            Err(error) => {
                let code = match error {
                    ProviderError::MalformedProviderState { .. } => {
                        DiagnosticCode::MalformedProviderState
                    }
                    _ => DiagnosticCode::CapabilityUnavailable,
                };
                diag(
                    &mut snapshot,
                    code,
                    "Codex snapshot or App Server metadata unavailable under strict source read-only isolation; protected file metadata remains available",
                );
                if self.fixture.is_none() {
                    diag(
                        &mut snapshot,
                        DiagnosticCode::SnapshotUnavailable,
                        "Disposable evidence capture was not verified; an empty native list does not establish absence of sessions",
                    );
                }
            }
        }
        snapshot.normalize();
        Ok(snapshot)
    }
}
fn artifact_identity(relative: &Path) -> Option<EntityId> {
    relative
        .to_str()
        .and_then(|value| scoped_id("artifact", value).ok())
}
fn scoped_id(kind: &str, value: &str) -> Result<EntityId, ProviderError> {
    EntityId::new(value)
        .map_err(|_| ProviderError::InvalidScope("invalid metadata identity".into()))?;
    EntityId::new(format!("codex:{kind}:{value}"))
        .map_err(|_| ProviderError::InvalidScope("excessive metadata identity".into()))
}
fn malformed(path: &Path, detail: &str) -> ProviderError {
    ProviderError::MalformedProviderState {
        path: path.into(),
        detail: detail.into(),
    }
}
fn diag(snapshot: &mut DiscoverySnapshot, code: DiagnosticCode, message: &str) {
    snapshot.diagnostics.push(Diagnostic {
        code,
        message: message.into(),
        path: None,
    });
}
fn decode_page(response: &Value, root: &Path) -> Result<ListResponse, ProviderError> {
    // Parsing errors are deliberately static: serde diagnostics can contain private content.
    let result = response
        .get("result")
        .ok_or_else(|| malformed(root, "thread/list result missing"))?;
    serde_json::from_value(result.clone())
        .map_err(|_| malformed(root, "invalid typed thread/list metadata"))
}
fn fixture_pages(
    fixture: &Fixture,
    scope: &DiscoveryScope,
) -> Result<(Vec<Page>, bool), ProviderError> {
    let mut pages = Vec::new();
    let mut active_seen = false;
    let mut archived_seen = false;
    let mut active_pending = false;
    let mut archived_pending = false;
    for page in fixture.pages.iter().take(256) {
        if page.archived && !scope.include_archived {
            continue;
        }
        let result = decode_page(&page.response, Path::new("fixture"))?;
        if page.archived {
            archived_seen = true;
            archived_pending = result.next_cursor.is_some();
        } else {
            active_seen = true;
            active_pending = result.next_cursor.is_some();
        }
        pages.push(Page {
            archived: page.archived,
            result,
        });
    }
    Ok((
        pages,
        fixture.complete
            && fixture.pages.len() <= 256
            && active_seen
            && (!scope.include_archived || archived_seen)
            && !active_pending
            && !archived_pending,
    ))
}

struct CaptureObservation {
    manifest: capture::CaptureManifest,
    stale: bool,
}

fn evidence_pages(
    exe: &Path,
    root: &Path,
    helper: &Path,
    real_home: &Path,
    scope: &DiscoveryScope,
    profile: capture::ExactProfile,
) -> Result<(Vec<Page>, bool, CaptureObservation), ProviderError> {
    use agentmender_provider_api::evidence::EvidenceSandbox;
    let sandbox = EvidenceSandbox::new(root, real_home)?;
    let request = capture::CaptureRequest {
        profile,
        source_root: sandbox.source_root().into(),
        mirror_root: sandbox.path().into(),
        probe_path: sandbox.probe_path().into(),
    };
    let manifest =
        read_capture_manifest(process::spawn_codex_capture(helper, &sandbox)?, &request)?;
    let transport = Transport::from_child(
        process::spawn_codex_evidence(exe, &sandbox)?,
        Duration::from_secs(5),
    )?;
    // query_pages owns the transport: the provider is reaped before source
    // comparison and, on every return path, before the owned mirror is dropped.
    let (pages, complete) = query_pages(transport, sandbox.path(), scope)?;
    let stale = manifest.changed()
        || capture::fingerprints(sandbox.source_root())
            .map(|after| capture::fingerprints_changed(&manifest.after, &after))
            .unwrap_or(true);
    Ok((pages, complete, CaptureObservation { manifest, stale }))
}

fn read_capture_manifest(
    child: Child,
    request: &capture::CaptureRequest,
) -> Result<capture::CaptureManifest, ProviderError> {
    let mut child = Guard(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    let unavailable =
        || ProviderError::CapabilityUnavailable("bounded disposable capture worker failed".into());
    let profile = request.profile;
    let request = serde_json::to_vec(request).map_err(|_| unavailable())?;
    if request.len() > 8192 {
        return Err(unavailable());
    }
    let mut input = child.0.stdin.take().ok_or_else(unavailable)?;
    let mut output = child.0.stdout.take().ok_or_else(unavailable)?;
    let (sent, written) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = input.write_all(&request);
        drop(input);
        let _ = sent.send(result);
    });
    let (sent, read) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .by_ref()
            .take(capture::MAX_MANIFEST_BYTES + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sent.send(result);
    });
    written
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    let bytes = read
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    if bytes.len() as u64 > capture::MAX_MANIFEST_BYTES {
        return Err(unavailable());
    }
    loop {
        match child.0.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => return Err(unavailable()),
        }
    }
    let manifest: capture::CaptureManifest =
        serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    manifest.validate(profile)?;
    Ok(manifest)
}

struct Guard(Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Transport {
    child: Guard,
    incoming: mpsc::Receiver<Result<Value, ()>>,
    deadline: Instant,
}
impl Transport {
    fn from_child(child: Child, timeout: Duration) -> Result<Self, ProviderError> {
        let mut child = Guard(child);
        let output = child
            .0
            .stdout
            .take()
            .ok_or_else(|| ProviderError::CapabilityUnavailable("stdout unavailable".into()))?;
        let (sender, incoming) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut input = BufReader::new(output);
            let mut total = 0;
            loop {
                let mut line = Vec::new();
                match input
                    .by_ref()
                    .take((MAX_LINE + 1) as u64)
                    .read_until(b'\n', &mut line)
                {
                    Ok(0) => break,
                    Ok(_) => {
                        total += line.len();
                        if line.len() > MAX_LINE || total > MAX_BYTES {
                            let _ = sender.send(Err(()));
                            break;
                        }
                        let parsed = serde_json::from_slice(&line).map_err(|_| ());
                        if sender.send(parsed).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(Err(()));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            incoming,
            deadline: Instant::now() + timeout,
        })
    }
    fn send(&mut self, value: Value) -> Result<(), ProviderError> {
        let mut bytes = serde_json::to_vec(&value)
            .map_err(|_| ProviderError::CapabilityUnavailable("request encoding failed".into()))?;
        bytes.push(b'\n');
        if bytes.len() > MAX_REQUEST_BYTES {
            self.stop();
            return Err(ProviderError::CapabilityUnavailable(
                "App Server request byte budget exceeded".into(),
            ));
        }
        let mut input = self
            .child
            .0
            .stdin
            .take()
            .ok_or_else(|| ProviderError::CapabilityUnavailable("stdin unavailable".into()))?;
        let (sender, written) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let result = input.write_all(&bytes);
            let _ = sender.send((input, result));
        });
        match written.recv_timeout(self.deadline.saturating_duration_since(Instant::now())) {
            Ok((input, Ok(()))) => {
                self.child.0.stdin = Some(input);
                Ok(())
            }
            _ => {
                self.stop();
                Err(ProviderError::CapabilityUnavailable(
                    "App Server stdin failed or exceeded total deadline".into(),
                ))
            }
        }
    }
    fn stop(&mut self) {
        let _ = self.child.0.kill();
        let _ = self.child.0.wait();
    }
    fn response(&mut self, request_id: u64) -> Result<Value, ProviderError> {
        for _ in 0..256 {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            let value = self
                .incoming
                .recv_timeout(remaining)
                .map_err(|_| {
                    ProviderError::CapabilityUnavailable(
                        "App Server exited or timed out under write denial".into(),
                    )
                })?
                .map_err(|_| {
                    ProviderError::CapabilityUnavailable(
                        "App Server response budget or format invalid".into(),
                    )
                })?;
            if value.get("id").and_then(Value::as_u64) == Some(request_id) {
                if value.get("error").is_some() {
                    return Err(ProviderError::CapabilityUnavailable(
                        "read-only App Server request rejected".into(),
                    ));
                }
                return Ok(value);
            }
            // Unexpected server requests are never answered or executed.
            if value.get("id").is_some() {
                return Err(ProviderError::CapabilityUnavailable(
                    "unexpected App Server request".into(),
                ));
            }
        }
        Err(ProviderError::CapabilityUnavailable(
            "App Server notification budget exceeded".into(),
        ))
    }
}
fn validate_initialize(response: &Value, root: &Path) -> Result<(), ProviderError> {
    let result = response
        .get("result")
        .ok_or_else(|| malformed(root, "initialize result missing"))?;
    let initialized: InitializeResponse = serde_json::from_value(result.clone())
        .map_err(|_| malformed(root, "initialize result does not match captured schema"))?;
    if !initialized.codex_home.is_absolute() || !root.is_absolute() {
        return Err(malformed(
            root,
            "initialize returned a nonabsolute state home",
        ));
    }
    let matching = if root.exists() {
        match (root.canonicalize(), initialized.codex_home.canonicalize()) {
            (Ok(expected), Ok(observed)) => expected == observed,
            _ => false,
        }
    } else {
        lexical_absolute(root) == lexical_absolute(&initialized.codex_home)
    };
    if !matching {
        return Err(malformed(
            root,
            "initialize state home differs from requested provider root",
        ));
    }
    Ok(())
}
fn lexical_absolute(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}
fn query_pages(
    mut transport: Transport,
    root: &Path,
    scope: &DiscoveryScope,
) -> Result<(Vec<Page>, bool), ProviderError> {
    transport.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"agentmender","title":"AgentMender","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}}}))?;
    validate_initialize(&transport.response(1)?, root)?;
    transport.send(json!({"method":"initialized","params":{}}))?;
    let mut pages = Vec::new();
    let mut request_id = 2;
    let mut total = 0;
    for archived in [false, true] {
        if archived && !scope.include_archived {
            break;
        }
        let mut cursor = None::<String>;
        let mut cursors = BTreeSet::new();
        loop {
            let remaining = scope.max_entries.saturating_sub(total);
            if remaining == 0 || pages.len() >= 256 {
                return Ok((pages, false));
            }
            let mut params = json!({"cursor":cursor,"limit":remaining.min(100),"archived":archived,"sortKey":"created_at","sortDirection":"desc","sourceKinds":SOURCES,"useStateDbOnly":true,"modelProviders":[]});
            if let Some(path) = &scope.project_path {
                params["cwd"] = json!(path);
            }
            transport.send(json!({"id":request_id,"method":"thread/list","params":params}))?;
            let response = transport.response(request_id)?;
            request_id += 1;
            let result = decode_page(&response, root)?;
            total += result.data.len();
            cursor = result.next_cursor.clone();
            pages.push(Page { archived, result });
            match &cursor {
                None => break,
                Some(value) if cursors.insert(value.clone()) => {}
                Some(_) => return Ok((pages, false)),
            }
        }
    }
    Ok((pages, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_native_project_can_span_multiple_observed_workspaces() {
        let dir = tempfile::tempdir().unwrap();
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known/manifest.json");
        let mut value: Value = serde_json::from_slice(&std::fs::read(root).unwrap()).unwrap();
        value["pages"][0]["response"]["result"]["data"][1]["cwd"] =
            json!("/synthetic/another-worktree");
        std::fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        let snapshot = CodexAdapter::from_fixture(dir.path().into())
            .unwrap()
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(snapshot.workspaces.len(), 3);
        let project = snapshot
            .projects
            .iter()
            .find(|p| p.provider_project_id.as_deref() == Some("synthetic-project"))
            .unwrap();
        assert!(project.path.is_none());
        assert!(project.workspace_id.is_none());
    }
    fn fixture(name: &str) -> CodexAdapter {
        CodexAdapter::from_fixture(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/codex")
                .join(name),
        )
        .unwrap()
    }
    #[test]
    fn new_observed_fixture_preserves_profile_and_provenance() {
        let adapter = fixture("observed-snapshot-0.165.0-alpha.1.2");
        let snapshot = adapter
            .discover(&DiscoveryScope {
                include_archived: true,
                ..DiscoveryScope::default()
            })
            .unwrap();
        assert_eq!(
            snapshot.provider.fingerprint.interface_id.as_deref(),
            Some("codex-app-server-0.165.0-alpha.1.2")
        );
        assert_eq!(
            snapshot.provider.compatibility.status,
            CompatibilityStatus::ReadOnlyKnown
        );
        assert_eq!(snapshot.evidence_status, EvidenceStatus::FixtureVerified);
        assert_eq!(snapshot.sessions.len(), 5);
        let provenance: Value = serde_json::from_str(include_str!(
            "../../../fixtures/codex/observed-snapshot-0.165.0-alpha.1.2/provenance.json"
        ))
        .unwrap();
        assert_eq!(provenance["sourceStableDuringSample"], false);
        assert!(
            provenance["limitations"]
                .as_str()
                .unwrap()
                .contains("RealPartial/SnapshotStale")
        );
    }
    #[test]
    fn observed_fixture_and_synthetic_fixture_keep_distinct_evidence_status() {
        let observed = fixture("observed-snapshot")
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(observed.sessions.len(), 5);
        assert_eq!(observed.evidence_status, EvidenceStatus::FixtureVerified);
        assert_eq!(observed.completeness, Completeness::Partial);
        let synthetic = fixture("known")
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(synthetic.evidence_status, EvidenceStatus::SyntheticOnly);
    }
    #[test]
    fn fixture_discovers_archives_parents_and_projects_without_live_version_claim() {
        let snapshot = fixture("known")
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(snapshot.sessions.len(), 3);
        assert_eq!(snapshot.projects.len(), 2);
        assert_eq!(snapshot.references.len(), 1);
        assert_eq!(
            snapshot
                .sessions
                .iter()
                .filter(|s| s.state == SessionState::Archived)
                .count(),
            1
        );
        assert_eq!(snapshot.provider.fingerprint.executable_version, None);
        assert!(
            snapshot
                .sessions
                .iter()
                .all(|s| s.evidence.source == EvidenceSource::Fixture)
        );
        assert!(snapshot.artifacts.iter().all(Artifact::is_protected));
        assert!(snapshot.diagnostics.is_empty());
    }
    #[test]
    fn unknown_schema_stays_metadata_only() {
        let snapshot = fixture("unknown")
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert!(snapshot.sessions.is_empty());
        assert!(!snapshot.artifacts.is_empty());
        assert_eq!(
            snapshot.provider.compatibility.status,
            CompatibilityStatus::AuditOnlyUnknown
        );
    }
    #[test]
    fn malformed_and_missing_protocol_results_are_static_diagnostics() {
        for name in ["malformed", "missing-result"] {
            let snapshot = fixture(name).discover(&DiscoveryScope::default()).unwrap();
            assert!(snapshot.sessions.is_empty());
            assert!(
                snapshot
                    .diagnostics
                    .iter()
                    .any(|d| d.code == DiagnosticCode::MalformedProviderState)
            );
        }
    }
    #[test]
    fn partial_pages_remain_partial_and_visible() {
        let snapshot = fixture("partial")
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(snapshot.sessions.len(), 1);
        assert_eq!(snapshot.completeness, Completeness::Partial);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::IncompleteDiscovery)
        );
    }
    #[test]
    fn archive_and_project_filters_apply() {
        let snapshot = fixture("known")
            .discover(&DiscoveryScope {
                include_archived: false,
                project_path: Some("/synthetic/project".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(snapshot.sessions.len(), 2);
        assert!(
            snapshot
                .sessions
                .iter()
                .all(|s| s.state == SessionState::Active)
        );
    }
    #[test]
    fn missing_live_provider_is_protected_audit_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("missing");
        let snapshot = CodexAdapter::new(ProviderLocation {
            state_root: root,
            executable: None,
        })
        .discover(&DiscoveryScope::default())
        .unwrap();
        assert_eq!(snapshot.provider.presence, Presence::NotDetected);
        assert!(snapshot.sessions.is_empty());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::ProviderNotDetected)
        );
    }
    #[test]
    fn body_text_is_not_exposed_in_normalized_output() {
        let snapshot = fixture("known")
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert!(
            !serde_json::to_string(&snapshot)
                .unwrap()
                .contains("Synthetic fixture only")
        );
    }
    #[test]
    fn artifact_identity_survives_sibling_insertion() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("z-file"), "synthetic").unwrap();
        let adapter = CodexAdapter::new(ProviderLocation {
            state_root: dir.path().into(),
            executable: None,
        });
        let before = adapter.discover(&DiscoveryScope::default()).unwrap();
        std::fs::write(dir.path().join("a-file"), "synthetic").unwrap();
        let after = adapter.discover(&DiscoveryScope::default()).unwrap();
        let previous = &before.artifacts[0];
        assert_eq!(
            after
                .artifacts
                .iter()
                .find(|a| a.locator == previous.locator)
                .unwrap()
                .id,
            previous.id
        );
    }
    #[test]
    fn duplicate_pages_keep_first_observation_and_normalize_deterministically() {
        let adapter = fixture("duplicate");
        let first = adapter.discover(&DiscoveryScope::default()).unwrap();
        let second = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.sessions.len(), 3);
        assert!(
            first
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::ConcurrentChange)
        );
    }
    #[test]
    fn project_scope_omits_unattributable_file_metadata() {
        let snapshot = fixture("known")
            .discover(&DiscoveryScope {
                project_path: Some("/synthetic/project".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(snapshot.artifacts.is_empty());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::IncompleteDiscovery)
        );
    }
    #[test]
    fn nonexistent_executable_does_not_establish_installed_presence() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = CodexAdapter::new(ProviderLocation {
            state_root: dir.path().into(),
            executable: Some(dir.path().join("missing-executable")),
        });
        assert_eq!(adapter.detect().unwrap().presence, Presence::StateOnly);
        assert!(
            adapter
                .discover(&DiscoveryScope::default())
                .unwrap()
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::ProviderVersionUnknown)
        );
    }
    #[test]
    fn invalid_project_identity_preserves_valid_records() {
        let dir = tempfile::tempdir().unwrap();
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known/manifest.json");
        let mut value: Value = serde_json::from_slice(&std::fs::read(source).unwrap()).unwrap();
        value["pages"][0]["response"]["result"]["data"][0]["projectId"] = json!("x".repeat(1100));
        std::fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        let snapshot = CodexAdapter::from_fixture(dir.path().into())
            .unwrap()
            .discover(&DiscoveryScope::default())
            .unwrap();
        assert_eq!(snapshot.sessions.len(), 2);
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::MalformedProviderState)
        );
    }
    #[cfg(unix)]
    #[test]
    fn unrepresentable_file_identity_is_rejected_without_lossy_collision() {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![255]));
        assert!(artifact_identity(&path).is_none());
    }
    #[cfg(unix)]
    #[test]
    fn invalid_file_identity_is_visible_as_static_diagnostic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("control\ncharacter"), "synthetic").unwrap();
        let snapshot = CodexAdapter::new(ProviderLocation {
            state_root: dir.path().into(),
            executable: None,
        })
        .discover(&DiscoveryScope::default())
        .unwrap();
        assert!(snapshot.artifacts.is_empty());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::UnknownArtifact)
        );
        assert!(serde_json::to_string(&snapshot).is_ok());
    }
    fn initialized(home: &Path) -> Value {
        json!({"id":1,"result":{"codexHome":home,"platformFamily":"unix","platformOs":"macos","userAgent":"synthetic-schema-client"}})
    }
    #[test]
    fn initialize_requires_every_captured_field_and_matching_absolute_home() {
        let root = tempfile::tempdir().unwrap();
        assert!(validate_initialize(&initialized(root.path()), root.path()).is_ok());
        let canonical = root.path().canonicalize().unwrap();
        assert!(validate_initialize(&initialized(&canonical), root.path()).is_ok());
        for key in ["codexHome", "platformFamily", "platformOs", "userAgent"] {
            let mut response = initialized(root.path());
            response["result"].as_object_mut().unwrap().remove(key);
            assert!(
                validate_initialize(&response, root.path()).is_err(),
                "{key}"
            );
        }
        assert!(validate_initialize(&initialized(Path::new("relative")), root.path()).is_err());
        let other = tempfile::tempdir().unwrap();
        assert!(validate_initialize(&initialized(other.path()), root.path()).is_err());
        assert!(validate_initialize(&json!({"id":1}), root.path()).is_err());
    }
    #[test]
    fn initialize_compares_nonexistent_homes_lexically_without_guessing_user_agent_version() {
        let root = tempfile::tempdir().unwrap();
        let expected = root.path().join("not-created");
        assert!(validate_initialize(&initialized(&expected), &expected).is_ok());
        assert!(
            validate_initialize(&initialized(&root.path().join("different")), &expected).is_err()
        );
    }
    #[cfg(unix)]
    fn stalled_transport(timeout: Duration) -> Transport {
        use std::process::{Command, Stdio};
        // Test-only fake child: no provider home or provider operation is involved.
        let child = Command::new("/bin/sleep")
            .arg("10")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Transport::from_child(child, timeout).unwrap()
    }
    #[cfg(unix)]
    #[test]
    fn oversized_request_fails_and_reaps_test_child() {
        let mut transport = stalled_transport(Duration::from_secs(1));
        assert!(
            transport
                .send(json!({"oversized":"x".repeat(MAX_REQUEST_BYTES)}))
                .is_err()
        );
        assert!(transport.child.0.try_wait().unwrap().is_some());
    }
    #[cfg(unix)]
    #[test]
    fn stalled_stdin_obeys_total_deadline_and_reaps_test_child() {
        let mut transport = stalled_transport(Duration::from_millis(100));
        let started = Instant::now();
        let message = json!({"padding":"x".repeat(MAX_REQUEST_BYTES/2)});
        let mut failed = false;
        for _ in 0..256 {
            if transport.send(message.clone()).is_err() {
                failed = true;
                break;
            }
        }
        assert!(
            failed,
            "test child never reads stdin; bounded pipe must fill"
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(transport.child.0.try_wait().unwrap().is_some());
    }
    #[cfg(unix)]
    #[test]
    fn failed_probe_preserves_capability_reason_and_diagnostic() {
        let root = tempfile::tempdir().unwrap();
        let adapter = CodexAdapter::new(ProviderLocation {
            state_root: root.path().into(),
            executable: Some("/usr/bin/false".into()),
        });
        let provider = adapter.detect().unwrap();
        assert!(
            provider
                .compatibility
                .reason
                .contains("Version probe unavailable under read-only boundary")
        );
        let snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::CapabilityUnavailable)
        );
    }
    #[test]
    fn relative_live_locations_are_rejected_before_probing() {
        let adapter = CodexAdapter::new(ProviderLocation {
            state_root: "relative".into(),
            executable: None,
        });
        assert!(adapter.detect().is_err());
        assert!(adapter.discover(&DiscoveryScope::default()).is_err());
    }
}

#[cfg(test)]
mod integrity_tests;
