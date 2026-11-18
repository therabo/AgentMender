//! Claude-specific storage roots and measurements. No content reads or writes.
mod walk;

use crate::{Access, ClaudeAdapter, RootSource, storage::REMOVAL_NOTICE};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Duration,
};
use walk::{Budget, Event, Kind, Metadata, metadata_tree};

const MAX_ENTRIES: usize = 60_000;
const MAX_GROUPS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RootKind {
    Code,
    Desktop,
    Temporary,
    Runtime,
    SharedCache,
    SharedLogs,
    GlobalConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    ExclusiveVerified,
    SharedVerified,
    Unknown,
    Global,
    UserOwnedOutput,
    InUse,
}

#[derive(Clone)]
pub(crate) struct Source {
    pub(crate) path: PathBuf,
    pub(crate) kind: RootKind,
    pub(crate) evidence: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Measurement {
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub files: usize,
    pub directories: usize,
    pub links: usize,
    pub hardlink_paths: Option<usize>,
    pub last_modified: Option<i64>,
    pub complete: bool,
}

impl Default for Measurement {
    fn default() -> Self {
        Self {
            logical_bytes: 0,
            allocated_bytes: cfg!(unix).then_some(0),
            files: 0,
            directories: 0,
            links: 0,
            hardlink_paths: cfg!(unix).then_some(0),
            last_modified: None,
            complete: true,
        }
    }
}

#[derive(Default)]
struct Tally {
    measured: Measurement,
    hardlinks: BTreeSet<String>,
}

impl Tally {
    fn record(&mut self, info: &Metadata) {
        let m = &mut self.measured;
        m.last_modified = m.last_modified.max(info.modified);
        match info.kind {
            Kind::Link => m.links += 1,
            Kind::Directory => m.directories += 1,
            Kind::Special => m.complete = false,
            Kind::File => {
                m.files += 1;
                m.logical_bytes = m.logical_bytes.saturating_add(info.logical);
                if let Some(count) = &mut m.hardlink_paths {
                    *count += usize::from(info.hardlinked);
                }
                if info.hardlinked
                    && let Some(id) = &info.identity
                    && !self.hardlinks.insert(id.clone())
                {
                    return;
                }
                m.allocated_bytes = m
                    .allocated_bytes
                    .zip(info.allocated)
                    .map(|(sum, next)| sum.saturating_add(next));
            }
        }
    }
}

#[derive(Serialize)]
pub struct LargeFile {
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    pub relative_path: PathBuf,
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
}

#[derive(Default)]
struct GroupTally {
    tally: Tally,
    largest: Vec<LargeFile>,
    formats: BTreeMap<String, usize>,
}

impl GroupTally {
    fn record(&mut self, relative: &Path, info: &Metadata) {
        self.tally.record(info);
        if info.kind != Kind::File {
            return;
        }
        let extension = relative
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| {
                value.len() <= 12 && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
            .map(str::to_ascii_lowercase)
            .unwrap_or_else(|| "other".into());
        let label = if self.formats.len() < 24 || self.formats.contains_key(&extension) {
            extension
        } else {
            "other".into()
        };
        *self.formats.entry(label).or_default() += 1;
        let rank = self
            .largest
            .partition_point(|row| row.logical_bytes >= info.logical);
        if rank < 3 {
            self.largest.insert(
                rank,
                LargeFile {
                    relative_path: relative.into(),
                    logical_bytes: info.logical,
                    allocated_bytes: info.allocated,
                },
            );
            self.largest.truncate(3);
        }
    }
}

#[derive(Serialize)]
pub struct UsageGroup {
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    pub relative_path: PathBuf,
    pub category: &'static str,
    pub ownership: Ownership,
    pub size: Measurement,
    pub largest_files: Vec<LargeFile>,
    pub formats: Vec<(String, usize)>,
}

#[derive(Serialize)]
pub struct UsageRoot {
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    pub path: PathBuf,
    kind: RootKind,
    pub evidence: &'static str,
    pub access: Access,
    pub size: Measurement,
    pub groups: Vec<UsageGroup>,
}

#[derive(Serialize)]
pub struct StorageInventory {
    pub roots: Vec<UsageRoot>,
    pub size: Measurement,
    pub complete: bool,
    pub entries_examined: usize,
    pub writer_observation: &'static str,
    pub reclaimable_logical_bytes: u64,
    pub offline_cleanup_supported: bool,
    pub removal_reason: &'static str,
}

fn desktop_shared(name: &str) -> bool {
    name.ends_with("Cache")
        || matches!(
            name,
            "Cache" | "Code Cache" | "Shared Dictionary" | "blob_storage"
        )
}

fn group_policy(kind: RootKind, name: &str) -> (&'static str, Ownership, bool) {
    use Ownership::*;
    match kind {
        RootKind::Code => {
            match name {
                "projects" => ("project_artifacts", Unknown, true),
                "file-history" | "session-env" | "tasks" | "todos" | "debug"
                | "shell-snapshots" | "image-cache" | "uploads" | "dev-mods" | "sessions"
                | "plans" => ("session_state", Unknown, true),
                "history.jsonl" | "history.jsonl.lock" | "paste-cache" => {
                    ("shared_data", SharedVerified, true)
                }
                "cache" | "downloads" | "telemetry" | "usage-data" | "stats-cache.json"
                | "statsig" => ("global_cache", Global, true),
                "backups" | "feedback-bundles" => ("protected_backup", UserOwnedOutput, true),
                "memory"
                | "plugins"
                | "skills"
                | "agents"
                | "commands"
                | "settings.json"
                | "settings.local.json"
                | ".claude.json"
                | ".credentials.json"
                | ".last-cleanup" => ("global_state", Global, true),
                "worktrees" => ("user_output", UserOwnedOutput, false),
                _ => ("unclassified", Unknown, false),
            }
        }
        RootKind::Desktop => match name {
            "claude-code-sessions" => ("desktop_code", SharedVerified, true),
            "claude-code" => ("runtime", Global, true),
            "local-agent-mode-sessions" | "local-agent-mode-worktrees" => {
                ("cowork", UserOwnedOutput, true)
            }
            "git-shadow" | "git-worktrees.json" => ("worktrees", UserOwnedOutput, false),
            "Crashpad" | "sentry" => ("shared_logs", Global, true),
            name if desktop_shared(name) => ("global_cache", Global, true),
            "IndexedDB" | "Local Storage" | "Session Storage" | "Extension State"
            | "Partitions" | "SharedStorage" | "WebStorage" | "VideoDecodeStats"
            | "spaces-present" | "shared_proto_db" => ("desktop_shared", SharedVerified, true),
            _ => ("desktop_shared", SharedVerified, false),
        },
        RootKind::Temporary => ("temporary", Unknown, true),
        RootKind::Runtime => ("runtime", Global, true),
        RootKind::SharedCache => ("global_cache", Global, true),
        RootKind::SharedLogs => ("shared_logs", Global, true),
        RootKind::GlobalConfig => ("global_state", Global, true),
    }
}

fn first(relative: &Path) -> &Path {
    relative
        .components()
        .next()
        .map(|part| Path::new(part.as_os_str()))
        .unwrap_or(Path::new(""))
}

fn usage_root(
    source: Source,
    budget: &mut Budget,
    overall: &mut Tally,
    boundaries: &BTreeSet<PathBuf>,
    runtime_in_use: bool,
) -> UsageRoot {
    let mut total = Tally::default();
    let mut groups = BTreeMap::<PathBuf, GroupTally>::new();
    let root_name = source
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let policy = |relative: &Path| {
        if relative.as_os_str().is_empty() {
            return true;
        }
        if boundaries.contains(&source.path.join(relative)) {
            return false;
        }
        if source.kind == RootKind::Code
            && relative.starts_with("projects")
            && let Some(child) = relative.components().nth(2)
        {
            let allowed = child
                .as_os_str()
                .to_str()
                .is_some_and(|name| name == "memory" || agentmender_domain::uuid_shape(name));
            return allowed;
        }
        group_policy(source.kind, first(relative).to_str().unwrap_or("")).2
    };
    let access = metadata_tree(
        &source.path,
        &source.path,
        budget,
        &policy,
        &mut |relative, event| {
            let root_directory = relative.as_os_str().is_empty()
                && matches!(&event, Event::Entry(meta) if meta.kind == Kind::Directory);
            let key = first(relative);
            match &event {
                Event::Entry(info) => {
                    total.record(info);
                    overall.record(info);
                }
                Event::Gap => total.measured.complete = false,
            }
            if root_directory || (relative.as_os_str().is_empty() && matches!(event, Event::Gap)) {
                return;
            }
            if groups.len() >= MAX_GROUPS && !groups.contains_key(key) {
                total.measured.complete = false;
                return;
            }
            let group = groups.entry(key.into()).or_default();
            match event {
                Event::Entry(info) => group.record(relative, &info),
                Event::Gap => group.tally.measured.complete = false,
            }
        },
    );
    if access == Access::Unavailable {
        total.measured.complete = false;
    }
    let mut groups: Vec<_> = groups
        .into_iter()
        .map(|(path, data)| {
            let name = match path.to_str() {
                Some("") => root_name,
                Some(name) => name,
                None => "",
            };
            let (category, ownership, _) = group_policy(source.kind, name);
            let ownership = if runtime_in_use {
                Ownership::InUse
            } else {
                ownership
            };
            let mut formats: Vec<_> = data.formats.into_iter().collect();
            formats.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            formats.truncate(4);
            UsageGroup {
                relative_path: path,
                category,
                ownership,
                size: data.tally.measured,
                largest_files: data.largest,
                formats,
            }
        })
        .collect();
    groups.sort_unstable_by(|a, b| {
        b.size
            .allocated_bytes
            .unwrap_or(b.size.logical_bytes)
            .cmp(&a.size.allocated_bytes.unwrap_or(a.size.logical_bytes))
            .then(a.relative_path.cmp(&b.relative_path))
    });
    UsageRoot {
        path: source.path,
        kind: source.kind,
        evidence: source.evidence,
        access,
        size: total.measured,
        groups,
    }
}

fn temporary_roots() -> Vec<Source> {
    #[cfg(unix)]
    let leaf = format!("claude-{}", rustix::process::getuid().as_raw());
    #[cfg(not(unix))]
    let leaf = "claude";
    let mut bases = Vec::new();
    if let Some(path) = std::env::var_os("CLAUDE_CODE_TMPDIR")
        .map(PathBuf::from)
        .filter(|path| crate::local_path(path))
    {
        bases.push((path, "claude_code_tmpdir"));
    }
    #[cfg(unix)]
    bases.push((
        PathBuf::from(if cfg!(target_os = "macos") {
            "/private/tmp"
        } else {
            "/tmp"
        }),
        "unix_provider_default",
    ));
    bases.push((std::env::temp_dir(), "system_temp"));
    for key in ["TMPDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            bases.push((PathBuf::from(value), "environment_temp"));
        }
    }
    let mut seen = BTreeSet::new();
    bases
        .into_iter()
        .filter(|(base, _)| crate::local_path(base))
        .filter_map(|(base, evidence)| {
            // macOS documents /tmp and /var as system aliases. Preserve every other
            // component for the no-follow scanner, including environment overrides.
            #[cfg(target_os = "macos")]
            let base = if base.starts_with("/tmp") || base.starts_with("/var") {
                Path::new("/private").join(base.strip_prefix("/").ok()?)
            } else {
                base
            };
            let path = base.join(&leaf);
            seen.insert(path.clone()).then_some(Source {
                path,
                kind: RootKind::Temporary,
                evidence,
            })
        })
        .collect()
}

impl ClaudeAdapter {
    pub fn storage_inventory(&self) -> StorageInventory {
        let mut sources = Vec::<Source>::new();
        for (path, source) in &self.roots {
            let evidence = match source {
                RootSource::Default => "provider_default",
                RootSource::Environment => "claude_config_dir",
                RootSource::Explicit | RootSource::Additional => "configured_root",
            };
            sources.push(Source {
                path: path.clone(),
                kind: RootKind::Code,
                evidence,
            });
            if *source == RootSource::Default
                && let Some(home) = path.parent()
            {
                sources.push(Source {
                    path: home.join(".claude.json"),
                    kind: RootKind::GlobalConfig,
                    evidence: "provider_configuration",
                });
            }
        }
        for root in self.desktop_roots() {
            sources.push(Source {
                path: root.into(),
                kind: RootKind::Desktop,
                evidence: "desktop_data_root",
            });
            if cfg!(target_os = "macos")
                && root.ends_with("Library/Application Support/Claude")
                && self
                    .installations()
                    .iter()
                    .any(|item| item.confirmed && item.source == "desktop_bundle")
                && let Some(library) = root.ancestors().nth(2)
            {
                sources.push(Source {
                    path: library.join("Caches/com.anthropic.claudefordesktop"),
                    kind: RootKind::SharedCache,
                    evidence: "native_bundle_identifier",
                });
                sources.push(Source {
                    path: library.join("Logs/Claude"),
                    kind: RootKind::SharedLogs,
                    evidence: "native_application_log_location",
                });
            }
        }
        for item in self.installations().iter().filter(|item| item.confirmed) {
            if item.source == "desktop_engine_bundle" {
                continue; // Counted in the Desktop data root.
            }
            if !sources.iter().any(|root| item.path.starts_with(&root.path)) {
                sources.push(Source {
                    path: item.path.clone(),
                    kind: RootKind::Runtime,
                    evidence: item.source,
                });
            }
        }
        sources.extend(self.session_temp_roots());
        sources.sort_by(|a, b| a.path.cmp(&b.path));
        sources.dedup_by(|a, b| a.path == b.path);
        let boundaries: BTreeSet<_> = sources.iter().map(|root| root.path.clone()).collect();
        let mut budget = Budget::new(MAX_ENTRIES, Duration::from_secs(6));
        let mut total = Tally::default();
        let mut roots = Vec::new();
        let writers = agentmender_provider_api::process::claude_writer_observation();
        for source in sources {
            let runtime_in_use = source.kind == RootKind::Runtime
                && writers
                    .executables
                    .iter()
                    .any(|path| path.starts_with(&source.path));
            roots.push(usage_root(
                source,
                &mut budget,
                &mut total,
                &boundaries,
                runtime_in_use,
            ));
        }
        total.measured.complete &= roots.iter().all(|root| root.size.complete);
        roots.sort_unstable_by(|a, b| {
            b.size
                .logical_bytes
                .cmp(&a.size.logical_bytes)
                .then(a.path.cmp(&b.path))
        });
        StorageInventory {
            roots,
            complete: total.measured.complete,
            size: total.measured,
            entries_examined: MAX_ENTRIES - budget.remaining,
            writer_observation: writers.status,
            reclaimable_logical_bytes: 0,
            offline_cleanup_supported: cfg!(target_os = "macos"),
            removal_reason: REMOVAL_NOTICE,
        }
    }

    pub(crate) fn session_temp_roots(&self) -> Vec<Source> {
        let auto = self
            .roots
            .iter()
            .any(|(_, source)| matches!(source, RootSource::Default | RootSource::Environment));
        temporary_roots()
            .into_iter()
            .filter(|source| auto || source.evidence == "claude_code_tmpdir")
            .collect()
    }
}

pub(crate) struct MeasuredArtifact {
    pub(crate) access: Access,
    pub(crate) size: Measurement,
    pub(crate) identity: Option<String>,
    pub(crate) kind: Option<Kind>,
}

pub(crate) struct ArtifactMeter(Budget);

impl ArtifactMeter {
    pub(crate) fn new() -> Self {
        Self(Budget::new(8192, Duration::from_secs(3)))
    }

    pub(crate) fn measure(&mut self, source: &Path, path: &Path) -> MeasuredArtifact {
        let mut tally = Tally::default();
        let mut identity = None;
        let mut kind = None;
        let access =
            metadata_tree(
                source,
                path,
                &mut self.0,
                &|_| true,
                &mut |same, event| match event {
                    Event::Entry(meta) => {
                        if same.as_os_str().is_empty() {
                            identity.clone_from(&meta.identity);
                            kind = Some(meta.kind);
                        }
                        tally.record(&meta);
                    }
                    Event::Gap => tally.measured.complete = false,
                },
            );
        tally.measured.complete &= access == Access::Available;
        MeasuredArtifact {
            access,
            size: tally.measured,
            identity,
            kind,
        }
    }
}

pub(crate) use walk::Kind as ArtifactKind;
