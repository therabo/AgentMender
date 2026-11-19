//! Metadata and display text derived from scoped Claude Code JSONL files.
use super::{ClaudeAdapter, TranscriptShape, strict_version, transcript_shape};
use crate::{
    diagnostics::{Access, Exclusion, Reconciliation, ScanIssue, TranscriptReport},
    read::{Lines, Revision, verify},
    storage::{self, ArtifactLocation, DesktopState, DirectoryScan, Entry, EntryKind, RootView},
};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, filesystem, io_error};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const CACHE_SIZE: usize = 4096;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(alias = "session_id")]
    session_id: Option<String>,
    agent_id: Option<String>,
    is_sidechain: Option<bool>,
    #[serde(default)]
    is_meta: bool,
    #[serde(default)]
    is_compact_summary: bool,
    #[serde(default)]
    is_visible_in_transcript_only: bool,
    #[serde(rename = "type")]
    kind: String,
    cwd: Option<String>,
    created_at: Option<String>,
    timestamp: Option<String>,
    custom_title: Option<String>,
    ai_title: Option<String>,
    summary: Option<String>,
    entrypoint: Option<String>,
    version: Option<String>,
    forked_from_session_id: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<Content>,
    model: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Blocks(Vec<Block>),
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

pub(crate) fn timestamp(value: Option<&str>) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value?)
        .ok()
        .map(|time| time.timestamp())
}

pub(crate) fn displayable(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 4096
        && !value.chars().any(char::is_control)
        && safe_conversation_text(value)
}

fn text_visible(value: &str) -> bool {
    !value.trim().is_empty()
        && !value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\r' | '\n'))
        && ![
            "<local-command-",
            "<command-name>",
            "<command-message>",
            "<command-args>",
            "<system-reminder>",
            "<tool_result",
            "<task-notification>",
            "<available-deferred-tools",
            "<claude-code-",
            "[Request interrupted",
        ]
        .iter()
        .any(|marker| value.contains(marker))
        && safe_conversation_text(value)
}

pub(crate) fn title(value: &str) -> Option<String> {
    (value.len() <= 32 * 1024 && text_visible(value)).then(|| {
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(180)
            .collect()
    })
}

fn shorten_text(text: &str, max: usize) -> &str {
    let mut boundary = max.min(text.len());
    while !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &text[..boundary]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Cli,
    Desktop,
    Mixed,
}

impl Surface {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "cli" | "sdk-cli" => Some(Self::Cli),
            "claude-desktop" => Some(Self::Desktop),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleSource {
    Custom,
    Generated,
    FirstRequest,
    Fallback,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionEvidence {
    pub metadata_complete: bool,
    #[serde(skip)]
    pub(crate) scan_issues: BTreeSet<ScanIssue>,
    pub modified_at: Option<i64>,
    pub surface: Option<Surface>,
    pub title_source: TitleSource,
    pub versions: Vec<String>,
    pub desktop_record: bool,
    pub sidechain: bool,
    pub same_uuid_elsewhere: usize,
    pub subagents: usize,
    pub forks: usize,
    #[serde(skip)]
    pub(crate) desktop_artifact: Option<ArtifactLocation>,
    #[serde(skip)]
    pub(crate) subagent_artifacts: Vec<ArtifactLocation>,
    #[serde(skip)]
    pub(crate) fork_references: Vec<String>,
    #[serde(skip)]
    pub(crate) ownership_scope_complete: bool,
    #[serde(skip)]
    pub revision: Revision,
}

impl SessionEvidence {
    pub fn transcript_footprint(&self) -> (u64, u64) {
        let total = self
            .subagent_artifacts
            .iter()
            .fold(self.revision.bytes, |sum, artifact| {
                sum.saturating_add(artifact.revision.bytes)
            });
        let exclusive =
            self.ownership_scope_complete && self.same_uuid_elsewhere == 0 && self.forks == 0;
        (total, if exclusive { total } else { 0 })
    }
}

#[derive(Clone)]
pub(crate) struct Cached {
    session: Session,
    evidence: SessionEvidence,
    desktop: Option<DesktopState>,
}

pub struct Catalog {
    pub inventory: InventoryObservation,
    pub evidence: BTreeMap<SessionIdentity, SessionEvidence>,
    pub roots: Vec<RootView>,
    pub reconciliation: Reconciliation,
}

#[derive(Default)]
struct Scan {
    found: bool,
    sidechain: bool,
    exclusion: Option<Exclusion>,
    issues: BTreeSet<ScanIssue>,
    custom: Option<String>,
    generated: Option<String>,
    first: Option<String>,
    workspace: Option<String>,
    workspace_ambiguous: bool,
    workspace_match: Option<usize>,
    foreign_rows: bool,
    surface: Option<Surface>,
    versions: BTreeSet<String>,
    created: Option<i64>,
    first_message_at: Option<i64>,
    last_message_at: Option<i64>,
    fork: Option<String>,
}

impl Record {
    fn role(&self) -> Option<TranscriptRole> {
        match self.kind.as_str() {
            "user" => Some(TranscriptRole::User),
            "assistant" => Some(TranscriptRole::Assistant),
            _ => None,
        }
    }

    fn visible(&self, max: usize) -> Option<String> {
        if self.role().is_none()
            || self.is_meta
            || self.is_visible_in_transcript_only
            || self.is_compact_summary
            || self.message.as_ref()?.model.as_deref() == Some("<synthetic>")
        {
            return None;
        }
        let mut result = String::new();
        let mut append = |text: &str| {
            if !result.is_empty() && result.len() < max {
                result.push('\n');
            }
            result.push_str(shorten_text(text.trim(), max - result.len()));
        };
        match self.message.as_ref()?.content.as_ref()? {
            Content::Text(value) if text_visible(value) => append(value),
            Content::Blocks(parts) => {
                if self.role() == Some(TranscriptRole::User)
                    && parts.iter().any(|part| part.kind == "tool_result")
                {
                    return None;
                }
                for part in parts {
                    if part.kind == "text"
                        && let Some(text) = &part.text
                    {
                        if !text_visible(text) {
                            return None;
                        }
                        append(text);
                    }
                }
            }
            _ => return None,
        }
        (!result.is_empty() && text_visible(&result)).then_some(result)
    }
}

impl Scan {
    fn record(&mut self, record: Record, shape: &TranscriptShape, desktop: Option<&DesktopState>) {
        let sid = record.session_id.as_deref();
        let Some(sid) = sid else {
            // Legacy summaries can be file-scoped. They cannot establish identity.
            if record.kind == "summary"
                && let Some(value) = record.summary.as_deref().and_then(title)
            {
                self.generated = Some(value);
            }
            return;
        };
        if sid != shape.session_id {
            self.foreign_rows = true;
            if desktop.and_then(|state| state.parent_cli_id.as_deref()) != Some(sid) {
                self.exclusion.get_or_insert(Exclusion::IdentityConflict);
            }
            return;
        }
        if shape.agent_id.is_none() && record.is_sidechain == Some(true) {
            self.sidechain = true;
            return;
        }
        if record
            .agent_id
            .as_ref()
            .is_some_and(|agent| Some(agent) != shape.agent_id.as_ref())
        {
            self.exclusion.get_or_insert(Exclusion::IdentityConflict);
            return;
        }
        if let Some(entrypoint) = record.entrypoint.as_deref() {
            let Some(surface) = Surface::parse(entrypoint) else {
                self.exclusion.get_or_insert(Exclusion::UnknownEntrypoint);
                return;
            };
            self.surface = Some(match self.surface {
                Some(prior) if prior != surface => Surface::Mixed,
                _ => surface,
            });
        }
        self.found |= match &shape.agent_id {
            Some(agent) => {
                record.agent_id.as_ref() == Some(agent) && record.is_sidechain == Some(true)
            }
            None => record.role().is_some() && record.agent_id.is_none(),
        };
        if let Some(fork) = record.forked_from_session_id.as_deref() {
            if !uuid_shape(fork)
                || fork == sid
                || self.fork.as_deref().is_some_and(|prior| prior != fork)
            {
                self.exclusion.get_or_insert(Exclusion::UnverifiedFork);
            } else {
                self.fork = Some(fork.into());
            }
        }
        if let Some(cwd) = record
            .cwd
            .as_ref()
            .filter(|cwd| displayable(cwd) && observed_path_is_absolute(Path::new(cwd)))
        {
            self.workspace_ambiguous |= self.workspace.as_ref().is_some_and(|prior| prior != cwd);
            self.workspace.get_or_insert_with(|| cwd.clone());
            if let Some(index) =
                desktop.and_then(|state| state.workspaces.iter().position(|path| path == cwd))
            {
                self.workspace_match =
                    Some(self.workspace_match.map_or(index, |prior| prior.min(index)));
            }
        }
        if let Some(version) = record
            .version
            .as_ref()
            .filter(|value| strict_version(value))
        {
            self.versions.insert(version.clone());
        }
        if let Some(created) = timestamp(record.created_at.as_deref()) {
            self.created = Some(self.created.map_or(created, |prior| prior.min(created)));
        }
        if record.role().is_some()
            && !record.is_meta
            && !record.is_compact_summary
            && let Some(date) = timestamp(record.timestamp.as_deref())
        {
            self.first_message_at =
                Some(self.first_message_at.map_or(date, |prior| prior.min(date)));
            self.last_message_at = self.last_message_at.max(Some(date));
        }
        let generated = match record.kind.as_str() {
            "ai-title" => record.ai_title.as_deref(),
            "summary" => record.summary.as_deref(),
            _ => None,
        };
        if let Some(value) = generated.and_then(title) {
            self.generated = Some(value);
        }
        if record.kind == "custom-title"
            && let Some(value) = &record.custom_title
        {
            // An unsafe newer title also supersedes the former title.
            self.custom = title(value);
        }
        if self.first.is_none() && record.role() == Some(TranscriptRole::User) {
            self.first = record.visible(1024).as_deref().and_then(title);
        }
    }
}

fn scan_file(
    file: impl Read,
    path: &Path,
    bytes: u64,
    shape: &TranscriptShape,
    desktop: Option<&DesktopState>,
) -> Result<Scan, ProviderError> {
    let mut reader = BufReader::new(file.take(bytes));
    let mut line = Vec::new();
    let mut scan = Scan::default();
    while reader
        .read_until(b'\n', &mut line)
        .map_err(|e| io_error(path, e))?
        != 0
    {
        if !line.iter().all(u8::is_ascii_whitespace) {
            match serde_json::from_slice(&line) {
                Ok(record) => scan.record(record, shape, desktop),
                Err(_) => {
                    scan.issues.insert(if line.ends_with(b"\n") {
                        ScanIssue::MalformedRecord
                    } else {
                        ScanIssue::UnterminatedRecord
                    });
                }
            }
        }
        line.clear();
    }
    Ok(scan)
}

fn is_transcript_candidate(entry: &Entry) -> bool {
    entry.kind != EntryKind::Directory
        && entry
            .path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
}

impl ClaudeAdapter {
    pub fn local_catalog(&self) -> Catalog {
        let mut directories = DirectoryScan::default();
        let mut candidates = Vec::new();
        let mut inventory = InventoryObservation::default();
        let mut roots = Vec::new();
        let mut report = TranscriptReport::default();
        let mut folders = BTreeSet::new();
        let mut physical_roots = BTreeSet::new();
        for (path, source) in &self.roots {
            let mut view = storage::root_view(path, *source);
            view.same_filesystem_root = view
                .filesystem_identity
                .as_ref()
                .is_some_and(|id| !physical_roots.insert(id.clone()));
            if view.same_filesystem_root {
                roots.push(view);
                continue;
            }
            let read_at_start = directories.issues;
            let count = candidates.len();
            if view.projects_access == Access::Available {
                let projects = path.join("projects");
                let namespaces = directories.list(&projects);
                for namespace in namespaces
                    .iter()
                    .filter(|entry| entry.kind == EntryKind::Directory)
                {
                    let Some(name) = namespace
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| displayable(name))
                    else {
                        directories.mark();
                        continue;
                    };
                    let identity = ProjectIdentity {
                        provider: ProviderKind::Claude,
                        state_root: path.clone(),
                        locator: ProjectLocator::Namespace(name.into()),
                    };
                    if !folders.insert(identity.clone()) {
                        continue;
                    }
                    view.namespaces += 1;
                    inventory.projects.push(RegisteredProject {
                        identity,
                        name: None,
                        roots: Vec::new(),
                        desktop_backed: false,
                    });
                    for entry in directories.list(&namespace.path) {
                        if is_transcript_candidate(&entry) {
                            candidates.push((roots.len(), entry));
                        } else if entry.kind == EntryKind::Directory
                            && entry
                                .path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_some_and(uuid_shape)
                        {
                            candidates.extend(
                                directories
                                    .list(&entry.path.join("subagents"))
                                    .into_iter()
                                    .filter(is_transcript_candidate)
                                    .map(|entry| (roots.len(), entry)),
                            );
                        }
                    }
                }
            } else if view.projects_access == Access::Unavailable {
                directories.mark();
            }
            roots.push(RootView {
                transcript_candidates: candidates.len() - count,
                enumeration_incomplete: directories.issues != read_at_start,
                ..view
            });
        }
        let mut uuid_counts = BTreeMap::new();
        for (index, entry) in &candidates {
            let relative = entry
                .path
                .strip_prefix(&roots[*index].path)
                .expect("enumerated root");
            if relative.components().count() == 3
                && let Some(id) = entry
                    .path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .filter(|id| uuid_shape(id))
            {
                *uuid_counts.entry(id.to_owned()).or_insert(0usize) += 1;
            }
        }
        let projects_incomplete = roots.iter().any(|root| root.enumeration_incomplete);
        let desktop = storage::desktop_states(&self.desktop, &mut directories);
        let mut evidence = BTreeMap::new();
        let mut cache = self.cache.lock().expect("Claude metadata cache");
        let mut next_cache = BTreeMap::new();
        for (index, entry) in candidates {
            let root = &roots[index].path;
            let path = &entry.path;
            let relative = path.strip_prefix(root).expect("enumerated root");
            let Some(shape) = transcript_shape(relative) else {
                report.exclude(Exclusion::UnrecognizedLocator);
                directories.mark();
                continue;
            };
            if entry.kind != EntryKind::File {
                report.exclude(Exclusion::ProtectedOrUnreadable);
                directories.mark();
                continue;
            }
            let native_id = shape.agent_id.as_ref().map_or_else(
                || shape.session_id.clone(),
                |agent| format!("{}/agent:{agent}", shape.session_id),
            );
            let identity = SessionIdentity {
                provider: ProviderKind::Claude,
                state_root: root.clone(),
                namespace: Some(shape.bucket.clone()),
                native_id,
            };
            let duplicates = uuid_counts.get(&shape.session_id).copied().unwrap_or(0);
            let linked = (!projects_incomplete && shape.agent_id.is_none() && duplicates == 1)
                .then(|| desktop.states.get(&shape.session_id))
                .flatten();
            match observe(path, &shape, identity, linked, &cache) {
                Ok(mut row) => {
                    row.evidence.same_uuid_elsewhere = duplicates.saturating_sub(1);
                    if next_cache.len() < CACHE_SIZE && row.evidence.metadata_complete {
                        next_cache.insert(row.session.identity.clone(), row.clone());
                    }
                    if !row.evidence.metadata_complete {
                        directories.mark();
                    }
                    report.record(&row.session, &row.evidence);
                    evidence.insert(row.session.identity.clone(), row.evidence);
                    inventory.sessions.push(row.session);
                }
                Err(reason) => {
                    report.exclude(reason);
                    directories.mark();
                }
            }
        }
        *cache = next_cache;
        for row in &inventory.sessions {
            if let Some(parent) = &row.parent
                && let Some(agent) = row
                    .identity
                    .native_id
                    .strip_prefix(&format!("{}/agent:", parent.native_id))
                && let Some(meta) = evidence.get(&row.identity)
                && let Ok(parent_path) = self.transcript_path(parent)
            {
                let revision = meta.revision.clone();
                if let Some(source) = evidence.get_mut(parent) {
                    source.subagents += 1;
                    source.subagent_artifacts.push(ArtifactLocation {
                        root: parent.state_root.clone(),
                        path: parent_path
                            .with_extension("")
                            .join("subagents")
                            .join(format!("agent-{agent}.jsonl")),
                        revision,
                    });
                }
            }
            if let Some(fork) = &row.forked_from {
                let mut parent = row.identity.clone();
                parent.native_id = fork.clone();
                if let Some(source) = evidence.get_mut(&parent) {
                    source.forks += 1;
                    source.fork_references.push(row.identity.native_id.clone());
                }
            }
        }
        let desktop_projects: BTreeSet<_> = evidence
            .iter()
            .filter_map(|(id, meta)| {
                (meta.desktop_record)
                    .then_some(id.namespace.as_ref())
                    .flatten()
                    .map(|ns| ProjectIdentity {
                        provider: ProviderKind::Claude,
                        state_root: id.state_root.clone(),
                        locator: ProjectLocator::Namespace(ns.clone()),
                    })
            })
            .collect();
        for project in &mut inventory.projects {
            project.desktop_backed = desktop_projects.contains(&project.identity);
        }
        let associated: BTreeSet<_> = evidence
            .iter()
            .filter(|(_, meta)| meta.desktop_record)
            .map(|(id, _)| id.native_id.as_str())
            .collect();
        let desktop_report = desktop.reconcile(&uuid_counts, &associated, !projects_incomplete);
        inventory.incomplete = directories.issues > 0;
        inventory.projects_incomplete = projects_incomplete;
        for meta in evidence.values_mut() {
            meta.ownership_scope_complete = !inventory.incomplete && !projects_incomplete;
        }
        Catalog {
            inventory,
            evidence,
            roots,
            reconciliation: Reconciliation {
                transcripts: report,
                desktop: desktop_report,
                projects_incomplete,
                out_of_scope: ["chat", "cowork", "code_cloud", "ssh", "wsl"],
            },
        }
    }
}

fn observe(
    path: &Path,
    shape: &TranscriptShape,
    identity: SessionIdentity,
    desktop: Option<&DesktopState>,
    cache: &BTreeMap<SessionIdentity, Cached>,
) -> Result<Cached, Exclusion> {
    let root = &identity.state_root;
    let unreadable = |_| Exclusion::ProtectedOrUnreadable;
    let changed = |_| Exclusion::ChangedDuringScan;
    let mut file = filesystem::open_observed_read(root, path).map_err(unreadable)?;
    let revision = Revision::read(&file, path).map_err(unreadable)?;
    if let Some(prior) = cache
        .get(&identity)
        .filter(|entry| entry.evidence.revision == revision && entry.desktop.as_ref() == desktop)
    {
        verify(root, path, &revision).map_err(changed)?;
        return Ok(prior.clone());
    }
    let scan = scan_file(&mut file, path, revision.bytes, shape, desktop).map_err(unreadable)?;
    verify(root, path, &revision).map_err(changed)?;
    if let Some(reason) = scan.exclusion {
        return Err(reason);
    }
    if !scan.found && !scan.sidechain {
        return Err(if scan.issues.is_empty() {
            Exclusion::MissingIdentity
        } else {
            Exclusion::InvalidRecords
        });
    }
    let desktop_workspace = desktop
        .filter(|_| scan.surface == Some(Surface::Desktop))
        .zip(scan.workspace_match)
        .and_then(|(state, index)| state.workspaces.get(index));
    let desktop = desktop.filter(|_| desktop_workspace.is_some());
    if scan.foreign_rows && desktop.is_none() {
        return Err(Exclusion::UnverifiedFork);
    }
    let workspace = desktop_workspace.cloned().or_else(|| {
        (!scan.workspace_ambiguous)
            .then_some(scan.workspace)
            .flatten()
    });
    let forked_from = desktop
        .and_then(|state| state.parent_cli_id.clone())
        .or(scan.fork);
    let kind = if shape.agent_id.is_some() {
        SessionKind::Subagent
    } else if scan.sidechain {
        SessionKind::Unknown
    } else if forked_from.is_some() || desktop.is_some_and(|state| state.has_fork) {
        SessionKind::Fork
    } else {
        SessionKind::Main
    };
    let parent = shape.agent_id.as_ref().map(|_| SessionIdentity {
        native_id: shape.session_id.clone(),
        ..identity.clone()
    });
    let fallback = format!("Sessione {}", &shape.session_id[..8]);
    let (title, title_source) = if let Some(value) = scan.custom {
        (value, TitleSource::Custom)
    } else if let Some(value) = scan.generated {
        (value, TitleSource::Generated)
    } else if let Some(value) = scan.first {
        (value, TitleSource::FirstRequest)
    } else {
        (fallback, TitleSource::Fallback)
    };
    Ok(Cached {
        desktop: desktop.cloned(),
        session: Session {
            project: workspace
                .as_ref()
                .and_then(|cwd| cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next())
                .map(str::to_owned),
            identity,
            codex_membership: CodexMembership::Unassigned,
            title: Some(title),
            created_at: scan
                .created
                .or_else(|| desktop.and_then(|state| state.created))
                .or(scan.first_message_at),
            last_active_at: scan.last_message_at,
            state: match desktop.and_then(|state| state.archived) {
                Some(true) => SessionState::Archived,
                Some(false) => SessionState::Active,
                None => SessionState::Unknown,
            },
            workspace,
            transcript_bytes: Some(revision.bytes),
            kind,
            forked_from,
            parent,
        },
        evidence: SessionEvidence {
            metadata_complete: scan.issues.is_empty(),
            scan_issues: scan.issues,
            modified_at: revision
                .modified
                .map(|time| chrono::DateTime::<chrono::Utc>::from(time).timestamp()),
            surface: scan.surface,
            title_source,
            versions: scan.versions.into_iter().collect(),
            desktop_record: desktop.is_some(),
            sidechain: scan.sidechain,
            same_uuid_elsewhere: 0,
            subagents: 0,
            forks: 0,
            desktop_artifact: desktop.map(|state| state.artifact.clone()),
            subagent_artifacts: Vec::new(),
            fork_references: Vec::new(),
            ownership_scope_complete: false,
            revision,
        },
    })
}

#[derive(Clone)]
pub struct Cursor {
    identity: SessionIdentity,
    revision: Revision,
    offset: u64,
    discard_fragment: bool,
    incomplete: bool,
}

pub struct TranscriptPage {
    pub messages: Vec<TranscriptMessage>,
    pub next: Option<Cursor>,
    pub complete: bool,
}

const PAGE_BYTES: u64 = 4 * 1024 * 1024;

impl ClaudeAdapter {
    pub fn transcript_page(
        &self,
        identity: &SessionIdentity,
        revision: &Revision,
        cursor: Option<Cursor>,
    ) -> Result<TranscriptPage, ProviderError> {
        let path = self.transcript_path(identity)?;
        let root = &identity.state_root;
        let mut file = filesystem::open_observed_read(root, &path)?;
        if Revision::read(&file, &path)? != *revision
            || cursor
                .as_ref()
                .is_some_and(|saved| &saved.identity != identity || &saved.revision != revision)
        {
            return Err(ProviderError::InvalidScope(
                "Claude transcript revision changed".into(),
            ));
        }
        let offset = cursor.as_ref().map_or(0, |saved| saved.offset);
        let mut discard_fragment = cursor.as_ref().is_some_and(|saved| saved.discard_fragment);
        let mut incomplete = cursor.is_some_and(|saved| saved.incomplete);
        let end = revision.bytes.min(offset.saturating_add(PAGE_BYTES));
        let mut lines = Lines::new(&mut file, offset, end).map_err(|e| io_error(&path, e))?;
        let mut messages = Vec::new();
        let mut text_remaining = 24 * 1024;
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut position = offset;
        for _ in 0..4096 {
            if messages.len() >= 20 || text_remaining < 1024 || Instant::now() >= deadline {
                break;
            }
            // Leave room to find the boundary of a large or skipped record.
            // The last page can instead consume a physical EOF fragment.
            if end < revision.bytes && lines.offset >= end - crate::read::MAX_RECORD as u64 {
                break;
            }
            let Some(line) = lines.next().map_err(|e| io_error(&path, e))? else {
                break;
            };
            position = line.end;
            let discard = discard_fragment;
            discard_fragment = !line.terminated;
            if discard {
                continue;
            }
            let Some(line) = line.bytes else {
                incomplete = true;
                continue;
            };
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let Ok(record) = serde_json::from_slice::<Record>(line) else {
                incomplete = true;
                continue;
            };
            if record.session_id.as_deref() != Some(identity.native_id.as_str())
                || record.is_sidechain == Some(true)
                || record.agent_id.is_some()
                || record
                    .entrypoint
                    .as_deref()
                    .is_some_and(|entry| Surface::parse(entry).is_none())
            {
                continue;
            }
            if let Some(role) = record.role()
                && let Some(text) = record.visible(text_remaining.min(4096))
            {
                text_remaining -= text.len();
                messages.push(TranscriptMessage { role, text });
            }
        }
        verify(root, &path, revision)?;
        let next = (position > offset && position < revision.bytes).then(|| Cursor {
            identity: identity.clone(),
            revision: revision.clone(),
            offset: position,
            discard_fragment,
            incomplete,
        });
        Ok(TranscriptPage {
            messages,
            complete: position == revision.bytes && !incomplete,
            next,
        })
    }

    pub(crate) fn transcript_path(
        &self,
        identity: &SessionIdentity,
    ) -> Result<PathBuf, ProviderError> {
        if identity.provider == ProviderKind::Claude
            && self
                .roots
                .iter()
                .any(|(path, _)| path == &identity.state_root)
            && uuid_shape(&identity.native_id)
            && let Some(namespace) = &identity.namespace
            && let [std::path::Component::Normal(_)] = Path::new(namespace)
                .components()
                .collect::<Vec<_>>()
                .as_slice()
            && !namespace.contains(['/', '\\', ':'])
        {
            return Ok(identity
                .state_root
                .join("projects")
                .join(namespace)
                .join(format!("{}.jsonl", identity.native_id)));
        }
        Err(ProviderError::InvalidScope(
            "invalid Claude session locator".into(),
        ))
    }
}
