//! Recognized storage locations, association evidence and read-only diagnostics.
use crate::{
    diagnostics::{Access, DesktopReport, RegistryRoot},
    local,
    read::{Lines, Revision, verify},
    safe_path,
};
use agentmender_domain::uuid_shape;
use agentmender_provider_api::{ProviderError, filesystem, io_error};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[cfg(target_os = "macos")]
pub const REMOVAL_NOTICE: &str =
    "L’anteprima della sessione mostra i file interessati dall’eliminazione.";
#[cfg(not(target_os = "macos"))]
pub const REMOVAL_NOTICE: &str = "Eliminazione Claude disponibile su macOS.";

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
    pub projects_access: Access,
    pub namespaces: usize,
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
    let access = directory_access(path);
    let readable = access == Access::Available;
    let projects_access = if readable {
        directory_access(&path.join("projects"))
    } else {
        access
    };
    #[cfg(unix)]
    let filesystem_identity = fs::symlink_metadata(path)
        .ok()
        .filter(|_| readable)
        .map(|info| {
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
        present: access != Access::Missing,
        readable,
        projects_access,
        namespaces: 0,
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

fn directory_access(path: &Path) -> Access {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Access::Missing,
        Ok(info) if info.is_dir() && safe_path(path) && fs::read_dir(path).is_ok() => {
            Access::Available
        }
        _ => Access::Unavailable,
    }
}

pub(crate) struct Budget {
    pub(crate) bytes: u64,
    pub(crate) issues: usize,
    pub(crate) deadline: Instant,
    entries: usize,
    pub(crate) entry_limit_reached: bool,
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
}

impl Budget {
    pub(crate) fn new(entries: usize) -> Self {
        Self {
            bytes: 256 * 1024 * 1024,
            issues: 0,
            deadline: Instant::now() + Duration::from_secs(10),
            entries,
            entry_limit_reached: false,
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
            if self.entries == 0 {
                self.entry_limit_reached = true;
                self.mark();
                break;
            }
            if self.expired() {
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
                    result.push(Entry { path, kind });
                }
                Err(_) => self.mark(),
            }
        }
        result.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        result
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtifactLocation {
    pub(crate) root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) revision: Revision,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DesktopState {
    pub(crate) archived: Option<bool>,
    pub(crate) created: Option<i64>,
    pub(crate) has_fork: bool,
    pub(crate) parent_cli_id: Option<String>,
    pub(crate) workspaces: Vec<String>,
    pub(crate) artifact: ArtifactLocation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRecord {
    session_id: String,
    cli_session_id: Option<String>,
    is_archived: Option<bool>,
    created_at: Option<u64>,
    forked_from_session_id: Option<String>,
    origin_cwd: Option<String>,
    cwd: Option<String>,
}

fn valid_desktop_id(id: &str) -> bool {
    id.strip_prefix("local_").is_some_and(uuid_shape)
}

fn desktop_record(root: &Path, path: &Path) -> Option<(DesktopRecord, ArtifactLocation)> {
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
        && record.cli_session_id.as_deref().is_none_or(uuid_shape)
        && path.file_stem().and_then(|s| s.to_str()) == Some(record.session_id.as_str()))
    .then_some((
        record,
        ArtifactLocation {
            root: root.into(),
            path: path.into(),
            revision,
        },
    ))
}

#[derive(Default)]
pub(crate) struct DesktopRegistry {
    pub(crate) states: BTreeMap<String, DesktopState>,
    by_cli_id: BTreeMap<String, usize>,
    report: DesktopReport,
}

impl DesktopRegistry {
    pub(crate) fn reconcile(
        mut self,
        candidates: &BTreeMap<String, usize>,
        associated: &BTreeSet<&str>,
        projects_complete: bool,
    ) -> DesktopReport {
        for (id, count) in self.by_cli_id {
            if associated.contains(id.as_str()) {
                self.report.associated += count;
            } else if projects_complete && !candidates.contains_key(&id) {
                self.report.without_transcript += count;
            } else {
                self.report.unresolved += count;
            }
        }
        self.report
    }
}

pub(crate) fn desktop_states(
    desktop_roots: &[super::DesktopContext],
    budget: &mut Budget,
) -> DesktopRegistry {
    let mut observed = DesktopRegistry::default();
    let mut records = BTreeMap::new();
    for desktop in desktop_roots {
        let root = &desktop.data_root;
        let registry = root.join("claude-code-sessions");
        let access = match directory_access(root) {
            Access::Available => directory_access(&registry),
            access => access,
        };
        let original_issues = budget.issues;
        let original_registrations = observed.report.registrations;
        let profiles = match access {
            Access::Available => budget.list(&registry),
            Access::Missing => Vec::new(),
            Access::Unavailable => {
                budget.mark();
                Vec::new()
            }
        };
        for profile in profiles.into_iter().filter(uuid_dir) {
            for account in budget.list(&profile.path).into_iter().filter(uuid_dir) {
                for item in budget.list(&account.path) {
                    if item
                        .path
                        .extension()
                        .is_some_and(|extension| extension == "json")
                        && item
                            .path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .is_some_and(valid_desktop_id)
                    {
                        if item.kind == EntryKind::File
                            && let Some((record, artifact)) = desktop_record(root, &item.path)
                        {
                            observed.report.registrations += 1;
                            if let Some(id) = &record.cli_session_id {
                                *observed.by_cli_id.entry(id.clone()).or_default() += 1;
                            } else {
                                observed.report.without_cli_uuid += 1;
                                observed.report.unresolved += 1;
                            }
                            let account = account.path.clone();
                            if records
                                .insert(record.session_id.clone(), (record, account, artifact))
                                .is_some()
                            {
                                budget.mark();
                            }
                        } else {
                            observed.report.unreadable_records += 1;
                            budget.mark();
                        }
                    }
                }
            }
        }
        let incomplete = access == Access::Unavailable || original_issues != budget.issues;
        observed.report.incomplete |= incomplete;
        observed.report.roots.push(RegistryRoot {
            path: registry,
            access,
            incomplete,
            registrations: observed.report.registrations - original_registrations,
        });
    }
    if observed.report.incomplete {
        // Missing records could hide ambiguity. Do not enrich by UUID.
        return observed;
    }
    let unique = |id: &str| observed.by_cli_id.get(id) == Some(&1);
    observed.states = records
        .values()
        .filter_map(|(record, account, artifact)| {
            let id = record.cli_session_id.as_ref().filter(|id| unique(id))?;
            let parent = record.forked_from_session_id.as_ref();
            let parent_cli_id = parent
                .and_then(|id| records.get(id))
                .filter(|(_, parent_account, _)| parent_account == account)
                .and_then(|(record, _, _)| record.cli_session_id.as_ref().filter(|id| unique(id)))
                .cloned();
            Some((
                id.clone(),
                DesktopState {
                    archived: record.is_archived,
                    created: record
                        .created_at
                        .and_then(|ms| i64::try_from(ms).ok())
                        .and_then(chrono::DateTime::from_timestamp_millis)
                        .map(|time| time.timestamp()),
                    has_fork: parent.is_some(),
                    parent_cli_id,
                    artifact: artifact.clone(),
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
            ))
        })
        .collect();
    observed
}

fn uuid_dir(entry: &Entry) -> bool {
    entry.kind == EntryKind::Directory
        && entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(uuid_shape)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryRow {
    #[serde(alias = "session_id")]
    session_id: Option<String>,
}

pub(crate) fn history_stats(
    root: &Path,
    id: &str,
    deadline: Instant,
) -> (Option<usize>, &'static str) {
    let path = root.join("history.jsonl");
    if fs::symlink_metadata(&path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        return (None, "not_observed");
    }
    match count_history(root, &path, id, deadline) {
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
