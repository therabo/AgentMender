//! Recognized storage locations, association evidence and read-only diagnostics.
use crate::{
    ClaudeAdapter,
    local::{self, SessionEvidence},
    read::{Lines, Revision, verify},
    safe_path,
};
use agentmender_domain::{SessionIdentity, uuid_shape};
use agentmender_provider_api::{ProviderError, filesystem, io_error};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub const REMOVAL_BLOCKER: &str = "Rimozione disabilitata. Il formato locale non dispone di un contratto verificato per escludere i writer Claude durante backup e rimozione.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootSource {
    Default,
    Environment,
    Explicit,
    Additional,
}

#[derive(Clone, Serialize)]
pub struct RootView {
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    pub path: PathBuf,
    pub platform: &'static str,
    pub kind: &'static str,
    pub source: RootSource,
    pub present: bool,
    pub readable: bool,
    pub same_filesystem_root: bool,
    pub removal_available: bool,
    pub filesystem_identity: Option<String>,
    pub history_present: bool,
    pub process_registry_present: bool,
    pub project_override: Option<String>,
    pub transcript_candidates: usize,
    pub enumeration_incomplete: bool,
}

pub(crate) fn root_view(path: &Path, source: RootSource) -> RootView {
    let info = fs::symlink_metadata(path).ok();
    let readable = info.as_ref().is_some_and(fs::Metadata::is_dir) && safe_path(path);
    #[cfg(unix)]
    let filesystem_identity = info.as_ref().filter(|_| readable).map(|info| {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}", info.dev(), info.ino())
    });
    #[cfg(windows)]
    let filesystem_identity = readable
        .then(|| filesystem::windows_directory_identity(path).ok())
        .flatten()
        .map(|(volume, index)| format!("{volume}:{index}"));
    #[cfg(not(any(unix, windows)))]
    let filesystem_identity = None;
    let candidate = |name| readable && fs::symlink_metadata(path.join(name)).is_ok();
    RootView {
        path: path.into(),
        platform: std::env::consts::OS,
        kind: "project_jsonl",
        source,
        present: info.is_some(),
        readable,
        same_filesystem_root: false,
        removal_available: false,
        filesystem_identity,
        history_present: candidate("history.jsonl"),
        process_registry_present: candidate("sessions"),
        project_override: std::env::var("CLAUDE_CODE_PROJECT_DIR_NAME")
            .ok()
            .filter(|value| {
                value.len() <= 900
                    && local::displayable(value)
                    && !value.contains(['/', '\\', ':'])
                    && value != "."
                    && value != ".."
            }),
        transcript_candidates: 0,
        enumeration_incomplete: false,
    }
}

pub(crate) struct Budget {
    pub(crate) bytes: u64,
    pub(crate) issues: usize,
    pub(crate) deadline: Instant,
    entries: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryKind {
    File,
    Directory,
    Protected,
}

pub(crate) struct Entry {
    pub(crate) path: PathBuf,
    pub(crate) kind: EntryKind,
    bytes: u64,
}

impl Budget {
    pub(crate) fn new(entries: usize) -> Self {
        Self {
            bytes: 256 * 1024 * 1024,
            issues: 0,
            deadline: Instant::now() + Duration::from_secs(10),
            entries,
        }
    }

    pub(crate) fn mark(&mut self) {
        self.issues += 1;
    }

    pub(crate) fn expired(&mut self) -> bool {
        let expired = Instant::now() >= self.deadline;
        if expired {
            self.mark();
        }
        expired
    }

    pub(crate) fn list(&mut self, path: &Path) -> Vec<Entry> {
        if self.expired() {
            return Vec::new();
        }
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Ok(metadata) if metadata.is_dir() && safe_path(path) => {}
            _ => {
                self.mark();
                return Vec::new();
            }
        }
        let Ok(entries) = fs::read_dir(path) else {
            self.mark();
            return Vec::new();
        };
        let mut result = Vec::new();
        for entry in entries {
            if self.entries == 0 || self.expired() {
                self.mark();
                break;
            }
            self.entries -= 1;
            let Ok(entry) = entry else {
                self.mark();
                continue;
            };
            let path = entry.path();
            match fs::symlink_metadata(&path) {
                Ok(meta) => {
                    let kind = if filesystem::is_link(&meta) {
                        self.mark();
                        EntryKind::Protected
                    } else if meta.is_dir() {
                        EntryKind::Directory
                    } else if meta.is_file() {
                        EntryKind::File
                    } else {
                        self.mark();
                        EntryKind::Protected
                    };
                    result.push(Entry {
                        path,
                        kind,
                        bytes: meta.len(),
                    });
                }
                Err(_) => self.mark(),
            }
        }
        result.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        result
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DesktopState {
    pub(crate) archived: Option<bool>,
    pub(crate) created: Option<i64>,
    pub(crate) has_fork: bool,
    pub(crate) parent_cli_id: Option<String>,
    pub(crate) workspaces: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRecord {
    session_id: String,
    cli_session_id: String,
    is_archived: Option<bool>,
    created_at: Option<u64>,
    forked_from_session_id: Option<String>,
    origin_cwd: Option<String>,
    cwd: Option<String>,
}

fn valid_desktop_id(id: &str) -> bool {
    id.strip_prefix("local_").is_some_and(uuid_shape)
}

fn desktop_record(root: &Path, path: &Path) -> Option<DesktopRecord> {
    let mut file = filesystem::open_observed_read(root, path).ok()?;
    let revision = Revision::read(&file, path).ok()?;
    if revision.bytes > 256 * 1024 {
        return None;
    }
    let mut data = Vec::new();
    (&mut file)
        .take(revision.bytes.saturating_add(1))
        .read_to_end(&mut data)
        .ok()?;
    verify(root, path, &revision).ok()?;
    let record: DesktopRecord = serde_json::from_slice(&data).ok()?;
    (valid_desktop_id(&record.session_id)
        && uuid_shape(&record.cli_session_id)
        && path.file_stem().and_then(|s| s.to_str()) == Some(record.session_id.as_str()))
    .then_some(record)
}

pub(crate) fn desktop_states(
    desktop_roots: &[super::DesktopContext],
    budget: &mut Budget,
) -> BTreeMap<String, DesktopState> {
    let mut records = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let original_issues = budget.issues;
    for desktop in desktop_roots {
        let root = &desktop.data_root;
        let registry = root.join("claude-code-sessions");
        for profile in budget.list(&registry).into_iter().filter(uuid_dir) {
            for account in budget.list(&profile.path).into_iter().filter(uuid_dir) {
                for item in budget.list(&account.path) {
                    if item.kind == EntryKind::File
                        && item
                            .path
                            .extension()
                            .is_some_and(|extension| extension == "json")
                        && item
                            .path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .is_some_and(valid_desktop_id)
                    {
                        if let Some(record) = desktop_record(root, &item.path) {
                            if !seen.insert(record.cli_session_id.clone()) {
                                duplicates.insert(record.cli_session_id.clone());
                            }
                            let account = account.path.clone();
                            if records
                                .insert(record.session_id.clone(), (record, account))
                                .is_some()
                            {
                                budget.mark();
                            }
                        } else {
                            budget.mark();
                        }
                    }
                }
            }
        }
    }
    if original_issues != budget.issues {
        // Missing records could hide ambiguity. Do not enrich by UUID.
        return BTreeMap::new();
    }
    records
        .values()
        .filter(|(record, _)| !duplicates.contains(&record.cli_session_id))
        .map(|(record, account)| {
            let parent = record.forked_from_session_id.as_ref();
            let parent_cli_id = parent
                .and_then(|id| records.get(id))
                .filter(|(record, parent_account)| {
                    parent_account == account && !duplicates.contains(&record.cli_session_id)
                })
                .map(|(record, _)| record.cli_session_id.clone());
            (
                record.cli_session_id.clone(),
                DesktopState {
                    archived: record.is_archived,
                    created: record
                        .created_at
                        .and_then(|ms| i64::try_from(ms).ok())
                        .and_then(chrono::DateTime::from_timestamp_millis)
                        .map(|time| time.timestamp()),
                    has_fork: parent.is_some(),
                    parent_cli_id,
                    workspaces: [record.origin_cwd.as_ref(), record.cwd.as_ref()]
                        .into_iter()
                        .flatten()
                        .filter(|cwd| {
                            local::displayable(cwd)
                                && agentmender_domain::observed_path_is_absolute(Path::new(cwd))
                        })
                        .cloned()
                        .collect(),
                },
            )
        })
        .collect()
}

fn uuid_dir(entry: &Entry) -> bool {
    entry.kind == EntryKind::Directory
        && entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(uuid_shape)
}

#[derive(Serialize)]
pub struct StorageDetail {
    pub transcript_files: usize,
    pub logical_bytes: u64,
    pub subagents: usize,
    pub forks: usize,
    pub sidecar_candidates: usize,
    pub checkpoint_present: bool,
    pub history_matches: Option<usize>,
    pub history_observation: &'static str,
    pub process_registry: &'static str,
    pub observed: Vec<Artifact>,
    pub complete: bool,
    pub standard_available: bool,
    pub deep_available: bool,
    pub backup_status: &'static str,
    pub removal_reason: &'static str,
}

#[derive(Serialize)]
pub struct Artifact {
    category: &'static str,
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    relative_path: PathBuf,
    evidence: &'static str,
    status: &'static str,
    bytes: Option<u64>,
}

impl ClaudeAdapter {
    pub fn storage_detail(
        &self,
        identity: &SessionIdentity,
        evidence: &SessionEvidence,
    ) -> Result<StorageDetail, ProviderError> {
        let path = self.transcript_path(identity)?;
        let root = &identity.state_root;
        verify(root, &path, &evidence.revision)?;
        let immediate = path.parent().expect("transcript namespace");
        let mut budget = Budget::new(4096);
        budget.deadline = Instant::now() + Duration::from_secs(2);
        let id = &identity.native_id;
        let mut observed = Vec::new();
        let mut record = |path: &Path, category, source, status, bytes| {
            observed.push(Artifact {
                category,
                relative_path: path.strip_prefix(root).expect("archive artifact").into(),
                evidence: source,
                status,
                bytes,
            });
        };
        record(
            &path,
            "transcript",
            "embedded_identity",
            "read_only",
            Some(evidence.revision.bytes),
        );
        let mut sidecar_candidates = 0;
        for entry in budget.list(immediate) {
            let Some(suffix) = entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix(id))
            else {
                continue;
            };
            if matches!(
                suffix,
                ".ccr-tip.json" | ".precompact.json" | ".desktop-released.json" | ".dir-sync.json"
            ) || suffix
                .strip_prefix('-')
                .and_then(|rest| rest.strip_suffix(".cast"))
                .is_some_and(|stamp| {
                    (1..=16).contains(&stamp.len())
                        && stamp.bytes().all(|byte| byte.is_ascii_digit())
                })
            {
                sidecar_candidates += 1;
                record(
                    &entry.path,
                    "sidecar",
                    "filename_only",
                    "unverified",
                    (entry.kind == EntryKind::File).then_some(entry.bytes),
                );
            }
        }
        let matches = [
            ("session_directory", immediate.join(id)),
            ("session_environment", root.join("session-env").join(id)),
            ("tasks", root.join("tasks").join(id)),
            ("debug", root.join("debug").join(format!("{id}.txt"))),
            ("checkpoint", root.join("file-history").join(id)),
            ("shared_memory", immediate.join("memory")),
            ("shared_history", root.join("history.jsonl")),
        ];
        let mut checkpoint_present = false;
        for (category, candidate) in matches {
            if candidate.parent().is_some_and(safe_path)
                && let Ok(info) = fs::symlink_metadata(&candidate)
            {
                checkpoint_present |= category == "checkpoint";
                let shared = category.starts_with("shared_");
                record(
                    &candidate,
                    category,
                    if shared {
                        "fixed_location"
                    } else {
                        "uuid_path_only"
                    },
                    if shared { "protected" } else { "unverified" },
                    (info.is_file() && !filesystem::is_link(&info)).then_some(info.len()),
                );
            }
        }
        let (history_matches, history_observation) = history_stats(root, id, &budget);
        let registry = root.join("sessions");
        let process_registry = if safe_path(&registry) && registry.is_dir() {
            "present_unverified"
        } else if fs::symlink_metadata(&registry).is_ok() {
            "blocked"
        } else {
            "not_observed"
        };
        verify(root, &path, &evidence.revision)?;
        Ok(StorageDetail {
            transcript_files: 1,
            logical_bytes: evidence.revision.bytes,
            subagents: evidence.subagents,
            forks: evidence.forks,
            sidecar_candidates,
            checkpoint_present,
            history_matches,
            history_observation,
            process_registry,
            observed,
            complete: false,
            standard_available: false,
            deep_available: false,
            backup_status: "not_created",
            removal_reason: REMOVAL_BLOCKER,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryRow {
    #[serde(alias = "session_id")]
    session_id: Option<String>,
}

fn history_stats(root: &Path, id: &str, budget: &Budget) -> (Option<usize>, &'static str) {
    let path = root.join("history.jsonl");
    if fs::symlink_metadata(&path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return (None, "not_observed");
    }
    match count_history(root, &path, id, budget.deadline) {
        Ok((count, true)) => (Some(count), "uuid_only"),
        Ok((count, false)) => (Some(count), "partial_uuid_only"),
        Err(_) => (None, "unverified"),
    }
}

fn count_history(
    root: &Path,
    path: &Path,
    id: &str,
    deadline: Instant,
) -> Result<(usize, bool), ProviderError> {
    let mut file = filesystem::open_observed_read(root, path)?;
    let revision = Revision::read(&file, path)?;
    let end = revision.bytes.min(8 * 1024 * 1024);
    let mut lines = Lines::new(&mut file, 0, end).map_err(|error| io_error(path, error))?;
    let mut count = 0;
    let mut complete = end == revision.bytes;
    for _ in 0..50_000 {
        if Instant::now() >= deadline {
            break;
        }
        let Some(line) = lines.next().map_err(|error| io_error(path, error))? else {
            break;
        };
        match line.bytes {
            Some(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => {}
            Some(bytes) => match serde_json::from_slice::<HistoryRow>(bytes) {
                Ok(row) => count += usize::from(row.session_id.as_deref() == Some(id)),
                Err(_) => complete = false,
            },
            None => complete = false,
        }
    }
    complete &= lines.offset == revision.bytes;
    verify(root, path, &revision)?;
    Ok((count, complete))
}
