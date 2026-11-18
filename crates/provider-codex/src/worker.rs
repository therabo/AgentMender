//! Fixed operations in the confined Codex reader process.
#[cfg(target_os = "macos")]
mod cascade;
#[cfg(target_os = "macos")]
mod inventory;
#[cfg(target_os = "macos")]
mod projects;

use crate::{CodexAdapter, Guard, removal::RemovalRefusal};
use agentmender_domain::{
    CodexMembership, InventoryObservation, ProjectIdentity, ProjectLocator, ProviderKind,
    RegisteredProject, Session, SessionIdentity,
};
use agentmender_provider_api::{
    ProviderError, evidence::EvidenceSandbox, process, validate_location,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

const MAX_REQUEST: usize = 64 * 1024;
const MAX_OUTPUT: usize = 128 * 1024;
const PAGE_ROWS: usize = 128;
const MAX_PROJECT_ROOTS: usize = 32;
const MAX_PROJECT_MEMBERS: usize = 256;
#[cfg(target_os = "macos")]
const MAX_SCOPE_MEMBERS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DecoderProgress {
    pub decoded: u64,
    pub compressed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub decoder_progress: Option<DecoderProgress>,
    pub byte_offset: u64,
    pub record_index: usize,
    pub chunk_offset: usize,
    pub partial: bool,
    pub skipping_record: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceAnchor {
    pub locator: Option<PathBuf>,
    pub path: PathBuf,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptCursor {
    pub(super) identity: SessionIdentity,
    pub(super) anchor: SourceAnchor,
    pub(super) checkpoint: Checkpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptRole {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptMessage {
    pub role: TranscriptRole,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptPage {
    pub messages: Vec<TranscriptMessage>,
    pub next: Option<TranscriptCursor>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RemovalEvidence {
    pub session: Session,
    pub revision: String,
    pub scope: RemovalScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RemovalScope {
    pub members: Vec<ReviewedMember>,
    pub database_identity: (u64, u64),
    pub unrelated_revision: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectCatalog {
    revision: String,
    projects: Vec<RegisteredProject>,
    legacy: BTreeMap<String, Option<ProjectLocator>>,
    incomplete: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectEvidence {
    pub registration: RegisteredProject,
    pub archive_revision: String,
    pub trees: Vec<RemovalEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewedMember {
    pub id: String,
    pub rollouts: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualEvidence {
    pub canonical_rows: usize,
    pub rollouts: usize,
    pub spawn_edges: usize,
    pub unrelated_changed: bool,
}

#[derive(Serialize, Deserialize)]
enum Verification {
    Verified,
    Residual(ResidualEvidence),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryPage {
    sessions: Vec<Session>,
    next: Option<String>,
    revision: String,
    examined: usize,
    incomplete: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    source_root: PathBuf,
    private_root: PathBuf,
    probe_path: PathBuf,
    operation: Operation,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Operation {
    Inventory {
        after: String,
        limit: usize,
    },
    Transcript {
        identity: SessionIdentity,
        cursor: Option<Box<TranscriptCursor>>,
    },
    Removal(SessionIdentity),
    ProjectCatalog,
    ProjectRemoval(ProjectIdentity),
    VerifyRemoval(RemovalScope),
}

#[derive(Serialize, Deserialize)]
enum Response {
    Unavailable,
    Stale,
    Inventory(InventoryPage),
    Transcript(TranscriptPage),
    Removal(Box<Result<RemovalEvidence, RemovalRefusal>>),
    ProjectCatalog(ProjectCatalog),
    ProjectRemoval(Result<ProjectEvidence, RemovalRefusal>),
    Verification(Verification),
}

pub(super) fn unavailable() -> ProviderError {
    ProviderError::CapabilityUnavailable("bounded Codex read unavailable".into())
}

pub(super) fn stale() -> ProviderError {
    ProviderError::InvalidScope("stale_codex_source".into())
}

#[cfg(target_os = "macos")]
struct Snapshot {
    files: Vec<crate::snapshot::FileFingerprint>,
    db: rusqlite::Connection,
}

#[cfg(target_os = "macos")]
fn observe(request: &Request, resident: &mut Option<Snapshot>) -> Result<Response, ProviderError> {
    use crate::{reader, snapshot};
    use std::time::Instant;

    let end = Instant::now() + Duration::from_secs(8);
    let root = &request.source_root;
    let before = snapshot::fingerprints(root)?;
    let db = match resident.take() {
        Some(snapshot) if !snapshot::fingerprints_changed(&snapshot.files, &before) => snapshot.db,
        _ => snapshot::read_snapshot(root, end)?,
    };
    let mut transcript = None;
    let response = match &request.operation {
        Operation::Inventory { after, limit } => {
            let revision = reader::hash(&serde_json::to_vec(&before).map_err(|_| unavailable())?);
            Response::Inventory(inventory::page(&db, root, after, *limit, revision, end)?)
        }
        Operation::Removal(identity) => {
            let revision = reader::hash(&serde_json::to_vec(&before).map_err(|_| unavailable())?);
            Response::Removal(Box::new(cascade::plan(&db, root, identity, revision, end)))
        }
        Operation::ProjectCatalog => {
            let revision = reader::hash(&serde_json::to_vec(&before).map_err(|_| unavailable())?);
            Response::ProjectCatalog(projects::read(
                &db,
                root,
                &inventory::layout(&db)?,
                revision,
            ))
        }
        Operation::ProjectRemoval(project) => {
            let revision = reader::hash(&serde_json::to_vec(&before).map_err(|_| unavailable())?);
            Response::ProjectRemoval(cascade::project_plan(&db, root, project, revision, end))
        }
        Operation::VerifyRemoval(scope) => {
            Response::Verification(cascade::verify(&db, root, scope, end)?)
        }
        Operation::Transcript { identity, cursor } => {
            let candidate = inventory::exact(&db, root, &identity.native_id, end)?;
            if &candidate.session.identity != identity || !candidate.session.is_main() {
                return Err(stale());
            }
            let (page, anchor) = reader::page(
                identity,
                candidate.locator.as_deref(),
                cursor.as_deref(),
                end,
            )?;
            transcript = Some((identity, anchor));
            Response::Transcript(page)
        }
    };
    let after = snapshot::fingerprints(root)?;
    if !snapshot::fingerprints_changed(&before, &after) {
        *resident = Some(Snapshot { files: after, db });
        return Ok(response);
    }

    // An unrelated mutation can change the SQLite snapshot during a preview.
    // Release it, then prove the same selected row locator and physical rollout.
    let (identity, anchor) = transcript.ok_or_else(stale)?;
    if !snapshot::same_primary_identity(&before, &after) {
        return Err(stale());
    }
    drop(db);
    let db = snapshot::read_snapshot(root, end)?;
    let current = inventory::exact(&db, root, &identity.native_id, end)?;
    if &current.session.identity != identity
        || !current.session.is_main()
        || reader::current_anchor(identity, current.locator.as_deref())? != anchor
        || !snapshot::same_primary_identity(&after, &snapshot::fingerprints(root)?)
    {
        return Err(stale());
    }
    Ok(response)
}

pub fn run_worker() -> Result<(), ProviderError> {
    #[cfg(not(target_os = "macos"))]
    {
        Err(unavailable())
    }

    #[cfg(target_os = "macos")]
    {
        let mut input = std::io::stdin().lock();
        let mut output = std::io::stdout().lock();
        let mut paths = None;
        let mut resident = None;
        loop {
            let mut bytes = Vec::new();
            if input
                .by_ref()
                .take((MAX_REQUEST + 1) as u64)
                .read_until(b'\n', &mut bytes)
                .map_err(|_| unavailable())?
                == 0
            {
                return Ok(());
            }
            if bytes.len() > MAX_REQUEST || bytes.last() != Some(&b'\n') {
                return Err(unavailable());
            }
            let request: Request = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
            crate::snapshot::validate_worker_paths(
                &request.source_root,
                &request.private_root,
                &request.probe_path,
            )?;
            let current = (
                &request.source_root,
                &request.private_root,
                &request.probe_path,
            );
            if paths
                .as_ref()
                .is_some_and(|(source, private, probe)| (source, private, probe) != current)
            {
                return Err(unavailable());
            }
            paths.get_or_insert_with(|| (current.0.clone(), current.1.clone(), current.2.clone()));
            let response = match observe(&request, &mut resident) {
                Ok(response) => response,
                Err(ProviderError::InvalidScope(reason)) if reason == "stale_codex_source" => {
                    Response::Stale
                }
                Err(_) => Response::Unavailable,
            };
            let mut encoded = serde_json::to_vec(&response).map_err(|_| unavailable())?;
            encoded.push(b'\n');
            if encoded.len() > MAX_OUTPUT {
                return Err(unavailable());
            }
            output
                .write_all(&encoded)
                .and_then(|_| output.flush())
                .map_err(|_| unavailable())?;
        }
    }
}

pub(super) struct Connection {
    child: Guard,
    sandbox: EvidenceSandbox,
    input: mpsc::SyncSender<Vec<u8>>,
    output: mpsc::Receiver<std::io::Result<Vec<u8>>>,
}

impl Connection {
    fn start(adapter: &CodexAdapter) -> Result<Self, ProviderError> {
        let sandbox = EvidenceSandbox::new(&adapter.location.state_root, &adapter.home)?;
        let mut child = Guard(process::spawn_codex_content_capture(
            &adapter.helper,
            &sandbox,
        )?);
        let mut input = child.0.stdin.take().ok_or_else(unavailable)?;
        let mut output = BufReader::new(child.0.stdout.take().ok_or_else(unavailable)?);
        let (sender, requests) = mpsc::sync_channel::<Vec<u8>>(1);
        let (responses, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            while let Ok(bytes) = requests.recv() {
                let mut response = Vec::new();
                let result = input
                    .write_all(&bytes)
                    .and_then(|_| input.flush())
                    .and_then(|_| {
                        output
                            .by_ref()
                            .take((MAX_OUTPUT + 1) as u64)
                            .read_until(b'\n', &mut response)
                    })
                    .and_then(|_| {
                        if response.len() > MAX_OUTPUT || response.last() != Some(&b'\n') {
                            Err(std::io::Error::other("Codex read frame refused"))
                        } else {
                            Ok(response)
                        }
                    });
                let failed = result.is_err();
                if responses.send(result).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            sandbox,
            input: sender,
            output: receiver,
        })
    }

    fn call(&mut self, operation: Operation) -> Result<Response, ProviderError> {
        self.sandbox.validate()?;
        if self
            .child
            .0
            .try_wait()
            .map_err(|_| unavailable())?
            .is_some()
        {
            return Err(unavailable());
        }
        let request = Request {
            source_root: self.sandbox.source_root().into(),
            private_root: self.sandbox.path().into(),
            probe_path: self.sandbox.probe_path().into(),
            operation,
        };
        let mut bytes = serde_json::to_vec(&request).map_err(|_| unavailable())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_REQUEST {
            return Err(unavailable());
        }
        self.input.try_send(bytes).map_err(|_| unavailable())?;
        let bytes = self
            .output
            .recv_timeout(Duration::from_secs(12))
            .map_err(|_| unavailable())?
            .map_err(|_| unavailable())?;
        serde_json::from_slice(&bytes).map_err(|_| unavailable())
    }
}

impl CodexAdapter {
    fn observe(&self, operation: Operation) -> Result<Response, ProviderError> {
        validate_location(&self.location)?;
        let mut slot = self.connection.lock().map_err(|_| unavailable())?;
        if slot.is_none() {
            *slot = Some(Connection::start(self)?);
        }
        let result = slot.as_mut().expect("worker started").call(operation);
        if result.is_err() {
            *slot = None;
        }
        match result? {
            Response::Unavailable => Err(unavailable()),
            Response::Stale => Err(stale()),
            response => Ok(response),
        }
    }

    pub fn local_inventory(&self, limit: usize) -> Result<InventoryObservation, ProviderError> {
        let mut sessions = Vec::new();
        let mut after = String::new();
        let mut revision = None;
        let root = self
            .location
            .state_root
            .canonicalize()
            .map_err(|_| unavailable())?;
        let mut remaining = limit;
        let mut incomplete = false;
        while remaining > 0 {
            let limit = remaining.min(PAGE_ROWS);
            let Response::Inventory(page) = self.observe(Operation::Inventory {
                after: after.clone(),
                limit,
            })?
            else {
                return Err(unavailable());
            };
            if revision
                .as_ref()
                .is_some_and(|expected| expected != &page.revision)
            {
                return Err(stale());
            }
            if page.examined > limit
                || page.sessions.len() > page.examined
                || page.sessions.iter().any(|session| {
                    let id = &session.identity;
                    id.provider != ProviderKind::Codex
                        || id.state_root != root
                        || id.namespace.is_some()
                        || !agentmender_domain::canonical_uuid(&id.native_id)
                })
            {
                return Err(unavailable());
            }
            revision = Some(page.revision);
            remaining -= page.examined;
            incomplete |= page.incomplete;
            sessions.extend(page.sessions);
            match page.next {
                Some(next) if next > after && page.examined > 0 => {
                    after = next;
                    incomplete |= remaining == 0;
                }
                Some(_) => return Err(unavailable()),
                None => break,
            }
        }
        let Response::ProjectCatalog(catalog) = self.observe(Operation::ProjectCatalog)? else {
            return Err(unavailable());
        };
        if Some(&catalog.revision) != revision.as_ref() {
            return Err(stale());
        }
        let registered: BTreeSet<_> = catalog
            .projects
            .iter()
            .map(|project| &project.identity.locator)
            .collect();
        let mut projects_incomplete = catalog.incomplete;
        let mut codex_members = BTreeMap::new();
        for session in sessions.iter().filter(|session| session.is_main()) {
            let legacy = catalog.legacy.get(&session.identity.native_id);
            let membership = match &session.codex_membership {
                CodexMembership::Unassigned => legacy.and_then(Option::as_ref).cloned(),
                CodexMembership::Native(id) => {
                    let native = ProjectLocator::Codex(id.clone());
                    if registered.contains(&native)
                        && legacy.is_none_or(|claim| claim.as_ref() == Some(&native))
                    {
                        Some(native)
                    } else {
                        projects_incomplete = true;
                        None
                    }
                }
                CodexMembership::Unknown => {
                    projects_incomplete = true;
                    None
                }
            };
            if let Some(locator) = membership {
                codex_members.insert(session.identity.native_id.clone(), locator);
            }
        }
        Ok(InventoryObservation {
            sessions,
            projects: catalog.projects,
            codex_members,
            incomplete,
            projects_incomplete,
        })
    }

    pub fn transcript_page(
        &self,
        identity: &SessionIdentity,
        cursor: Option<TranscriptCursor>,
    ) -> Result<TranscriptPage, ProviderError> {
        let Response::Transcript(page) = self.observe(Operation::Transcript {
            identity: identity.clone(),
            cursor: cursor.map(Box::new),
        })?
        else {
            return Err(unavailable());
        };
        if page
            .next
            .as_ref()
            .is_some_and(|next| &next.identity != identity)
            || page
                .messages
                .iter()
                .map(|msg| msg.text.len())
                .sum::<usize>()
                > 16 * 1024
        {
            return Err(unavailable());
        }
        Ok(page)
    }

    pub(super) fn removal_evidence(
        &self,
        identity: &SessionIdentity,
    ) -> Result<RemovalEvidence, RemovalRefusal> {
        match self.observe(Operation::Removal(identity.clone()))? {
            Response::Removal(result) => {
                let evidence = (*result)?;
                if &evidence.session.identity == identity && evidence.session.is_main() {
                    return Ok(evidence);
                }
                Err(RemovalRefusal::IncompleteRelationships)
            }
            _ => Err(RemovalRefusal::IncompleteRelationships),
        }
    }

    pub(super) fn project_evidence(
        &self,
        project: &ProjectIdentity,
    ) -> Result<ProjectEvidence, RemovalRefusal> {
        let Response::ProjectRemoval(result) =
            self.observe(Operation::ProjectRemoval(project.clone()))?
        else {
            return Err(RemovalRefusal::IncompleteRelationships);
        };
        let evidence = result?;
        let mut members = std::collections::BTreeSet::new();
        if evidence.registration.identity != *project || evidence.registration.desktop_backed
            || evidence.trees.len() > MAX_PROJECT_ROOTS
            || evidence.trees.iter().any(|tree| {
                !tree.session.is_main()
                    || tree.session.identity.state_root != project.state_root
                    || tree.session.identity.provider != project.provider
                    || !matches!((&project.locator, &tree.session.codex_membership),
                        (ProjectLocator::Codex(expected), CodexMembership::Native(actual)) if actual == expected)
                    || !tree
                        .scope
                        .members
                        .iter()
                        .any(|member| member.id == tree.session.identity.native_id)
                    || tree.scope.members.iter().any(|member| {
                        !agentmender_domain::canonical_uuid(&member.id)
                            || !members.insert(&member.id)
                    })
            })
            || members.len() > MAX_PROJECT_MEMBERS
        {
            return Err(RemovalRefusal::IncompleteRelationships);
        }
        Ok(evidence)
    }

    pub(super) fn verify_removal(
        &self,
        scope: RemovalScope,
    ) -> Result<Option<ResidualEvidence>, ProviderError> {
        match self.observe(Operation::VerifyRemoval(scope))? {
            Response::Verification(Verification::Verified) => Ok(None),
            Response::Verification(Verification::Residual(details)) => Ok(Some(details)),
            _ => Err(unavailable()),
        }
    }
}
