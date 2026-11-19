//! Session storage evidence and read-only preflight. Writer observations grant
//! neither authority to issue a receipt nor permission to touch provider data.
use crate::{
    Access, ClaudeAdapter, SessionEvidence,
    read::verify,
    storage::{ArtifactLocation, DirectoryScan, EntryKind, history_stats},
    usage::{ArtifactKind, ArtifactMeter, Measurement, Ownership, RootKind},
};
use agentmender_domain::SessionIdentity;
use agentmender_provider_api::{ProviderError, process};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MAX_ARTIFACTS: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Standard,
    Deep,
    Preserve,
}

#[derive(Serialize)]
pub struct Artifact {
    category: &'static str,
    root_kind: RootKind,
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    root: PathBuf,
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    relative_path: PathBuf,
    identity: Option<String>,
    ownership: Ownership,
    owner: Option<String>,
    evidence: &'static str,
    format: &'static str,
    status: &'static str,
    bytes: Option<u64>,
    access: Access,
    size: Measurement,
    scope: Scope,
    writer_state: &'static str,
}

#[derive(Serialize)]
struct Category {
    key: &'static str,
    files: usize,
    logical_bytes: u64,
    allocated_bytes: Option<u64>,
    state: &'static str,
}

const CATEGORIES: &[&str] = &[
    "transcript",
    "variant",
    "subagent",
    "tool_results",
    "mcp_tasks",
    "checkpoint",
    "images",
    "uploads",
    "scratchpad",
    "desktop_record",
    "sidecar",
    "session_state",
    "shared_data",
    "unknown",
];

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
    pub diagnostic_complete: bool,
    writer_observation: &'static str,
    fork_references: Vec<String>,
    engine_versions: Vec<String>,
    categories: Vec<Category>,
}

struct Review<'a> {
    meter: ArtifactMeter,
    artifacts: Vec<Artifact>,
    complete: bool,
    id: &'a str,
}

#[derive(Clone, Copy)]
struct Rule {
    category: &'static str,
    scope: Scope,
    ownership: Ownership,
    evidence: &'static str,
    format: &'static str,
}

fn candidate(category: &'static str, scope: Scope) -> Rule {
    Rule {
        category,
        scope,
        ownership: Ownership::Unknown,
        evidence: "scoped_path",
        format: "unverified",
    }
}

impl Review<'_> {
    fn add(&mut self, root: &Path, path: &Path, kind: RootKind, rule: Rule) {
        if self.artifacts.len() >= MAX_ARTIFACTS {
            self.complete = false;
            return;
        }
        let mut measured = self.meter.measure(root, path);
        if measured.access == Access::Missing {
            return;
        }
        if matches!(
            measured.kind,
            Some(ArtifactKind::Link | ArtifactKind::Special)
        ) {
            measured.size.complete = false;
        }
        self.complete &= measured.size.complete;
        let linked = measured.size.links > 0 || measured.size.hardlink_paths.unwrap_or(0) > 0;
        let (ownership, evidence) = if linked || !measured.size.complete {
            (Ownership::Unknown, "linked_or_partial")
        } else {
            (rule.ownership, rule.evidence)
        };
        let status = match ownership {
            Ownership::ExclusiveVerified => "read_only",
            Ownership::Unknown if !linked => "unverified",
            _ => "protected",
        };
        let known_owner = (ownership == Ownership::ExclusiveVerified
            || evidence == "reconciled_desktop_record"
            || evidence == "embedded_identity")
            .then(|| self.id.into());
        self.artifacts.push(Artifact {
            category: rule.category,
            root_kind: kind,
            root: root.into(),
            relative_path: path.strip_prefix(root).expect("fixed scope").into(),
            identity: measured.identity,
            ownership,
            owner: known_owner,
            evidence,
            format: rule.format,
            status,
            bytes: (measured.access == Access::Available).then_some(measured.size.logical_bytes),
            access: measured.access,
            size: measured.size,
            scope: rule.scope,
            writer_state: if ownership == Ownership::InUse {
                "in_use"
            } else {
                "unverified"
            },
        });
    }

    fn revalidated(
        &mut self,
        file: &ArtifactLocation,
        kind: RootKind,
        rule: Rule,
    ) -> Result<(), ProviderError> {
        verify(&file.root, &file.path, &file.revision)?;
        let position = self.artifacts.len();
        self.add(&file.root, &file.path, kind, rule);
        verify(&file.root, &file.path, &file.revision)?;
        if let Some(artifact) = self.artifacts.get_mut(position) {
            artifact.identity = file.revision.filesystem_identity();
        }
        Ok(())
    }

    fn transcript(
        &mut self,
        file: &ArtifactLocation,
        category: &'static str,
        exclusive: bool,
    ) -> Result<(), ProviderError> {
        self.revalidated(
            file,
            RootKind::Code,
            Rule {
                category,
                scope: Scope::Standard,
                ownership: if exclusive {
                    Ownership::ExclusiveVerified
                } else {
                    Ownership::Unknown
                },
                evidence: "embedded_identity",
                format: "claude_jsonl",
            },
        )
    }
}

pub(crate) fn exact_sidecar(suffix: &str) -> bool {
    matches!(
        suffix,
        ".ccr-tip.json" | ".precompact.json" | ".desktop-released.json" | ".dir-sync.json"
    ) || suffix
        .strip_prefix('-')
        .and_then(|rest| rest.strip_suffix(".cast"))
        .is_some_and(numeric_stamp)
}

fn numeric_stamp(value: &str) -> bool {
    (1..=16).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn variant(suffix: &str) -> bool {
    suffix
        .strip_prefix(".jsonl.superseded-")
        .is_some_and(numeric_stamp)
        || suffix
            .strip_prefix(".orphaned-")
            .and_then(|rest| rest.strip_suffix(".jsonl"))
            .and_then(|rest| rest.split_once('-'))
            .is_some_and(|(stamp, trail)| {
                numeric_stamp(stamp)
                    && (1..=32).contains(&trail.len())
                    && trail
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
}

fn summarize(key: &'static str, artifacts: &[Artifact], complete: bool) -> Category {
    let matching: Vec<_> = artifacts
        .iter()
        .filter(|artifact| artifact.category == key)
        .collect();
    let files = matching.iter().map(|artifact| artifact.size.files).sum();
    let logical_bytes = matching
        .iter()
        .map(|artifact| artifact.size.logical_bytes)
        .sum();
    // Allocated bytes per artifact can overlap across a linked group.
    let allocated_bytes = sum_allocation(matching.iter().copied());
    let state = if matching.is_empty() {
        if complete {
            "not_observed"
        } else {
            "unverified"
        }
    } else if matching
        .iter()
        .all(|row| row.ownership == Ownership::ExclusiveVerified)
    {
        "exclusive_verified"
    } else if matching.iter().all(|row| row.status == "protected") {
        "protected"
    } else {
        "unverified"
    };
    Category {
        key,
        files,
        logical_bytes,
        allocated_bytes,
        state,
    }
}

fn sum_allocation<'a>(mut items: impl Iterator<Item = &'a Artifact>) -> Option<u64> {
    if !cfg!(unix) {
        return None;
    }
    items.try_fold(0u64, |sum, item| {
        if item.size.links > 0 || item.size.hardlink_paths.unwrap_or(0) > 0 || !item.size.complete {
            None
        } else {
            item.size
                .allocated_bytes
                .map(|bytes| sum.saturating_add(bytes))
        }
    })
}

impl ClaudeAdapter {
    pub fn storage_detail(
        &self,
        identity: &SessionIdentity,
        evidence: &SessionEvidence,
    ) -> Result<StorageDetail, ProviderError> {
        let path = self.transcript_path(identity)?;
        let root = &identity.state_root;
        let immediate = path.parent().expect("transcript namespace");
        let session_path = path.with_extension("");
        let id = &identity.native_id;
        let mut review = Review {
            meter: ArtifactMeter::new(),
            artifacts: Vec::new(),
            complete: evidence.metadata_complete,
            id,
        };
        let exclusive = evidence.ownership_scope_complete
            && evidence.same_uuid_elsewhere == 0
            && evidence.forks == 0;
        review.transcript(
            &ArtifactLocation {
                root: root.clone(),
                path: path.clone(),
                revision: evidence.revision.clone(),
            },
            "transcript",
            exclusive,
        )?;
        let mut verified_agents = BTreeSet::new();
        for agent in evidence.subagent_artifacts.iter().take(MAX_ARTIFACTS) {
            review.transcript(agent, "subagent", exclusive)?;
            verified_agents.insert(&agent.path);
        }

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut budget = DirectoryScan::bounded(4096, deadline);
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
            if exact_sidecar(suffix) {
                sidecar_candidates += 1;
                review.add(
                    root,
                    &entry.path,
                    RootKind::Code,
                    candidate("sidecar", Scope::Standard),
                );
            } else if variant(suffix) {
                review.add(
                    root,
                    &entry.path,
                    RootKind::Code,
                    Rule {
                        evidence: "documented_variant_name",
                        format: "variant_unverified",
                        ..candidate("variant", Scope::Standard)
                    },
                );
            } else if !suffix.is_empty() && suffix != ".jsonl" && suffix.starts_with(['.', '-']) {
                review.add(
                    root,
                    &entry.path,
                    RootKind::Code,
                    candidate("unknown", Scope::Preserve),
                );
            }
        }
        for entry in budget.list(&session_path) {
            match entry.path.file_name().and_then(|name| name.to_str()) {
                Some("subagents") if entry.kind == EntryKind::Directory => {
                    for child in budget.list(&entry.path) {
                        if !verified_agents.contains(&child.path) {
                            review.add(
                                root,
                                &child.path,
                                RootKind::Code,
                                candidate("subagent", Scope::Preserve),
                            );
                        }
                    }
                }
                Some("tool-results" | "mcp-tasks") => {
                    let category = if entry.path.ends_with("tool-results") {
                        "tool_results"
                    } else {
                        "mcp_tasks"
                    };
                    review.add(
                        root,
                        &entry.path,
                        RootKind::Code,
                        candidate(category, Scope::Deep),
                    );
                }
                _ => review.add(
                    root,
                    &entry.path,
                    RootKind::Code,
                    candidate("unknown", Scope::Preserve),
                ),
            }
        }

        let candidates = [
            (
                "session_state",
                root.join("session-env").join(id),
                Scope::Standard,
            ),
            (
                "session_state",
                root.join("tasks").join(id),
                Scope::Standard,
            ),
            (
                "session_state",
                root.join("todos").join(format!("{id}.json")),
                Scope::Standard,
            ),
            (
                "session_state",
                root.join("debug").join(format!("{id}.txt")),
                Scope::Standard,
            ),
            (
                "checkpoint",
                root.join("file-history").join(id),
                Scope::Deep,
            ),
            ("images", root.join("image-cache").join(id), Scope::Deep),
            ("uploads", root.join("uploads").join(id), Scope::Deep),
            ("tool_results", root.join("dev-mods").join(id), Scope::Deep),
        ];
        for (category, path, scope) in candidates {
            review.add(root, &path, RootKind::Code, candidate(category, scope));
        }
        for path in [
            immediate.join("memory"),
            root.join("history.jsonl"),
            root.join("paste-cache"),
        ] {
            review.add(
                root,
                &path,
                RootKind::Code,
                Rule {
                    ownership: Ownership::SharedVerified,
                    evidence: "documented_shared_location",
                    ..candidate("shared_data", Scope::Preserve)
                },
            );
        }
        if let Some(desktop) = &evidence.desktop_artifact {
            review.revalidated(
                desktop,
                RootKind::Desktop,
                Rule {
                    evidence: "reconciled_desktop_record",
                    format: "desktop_local_registration",
                    ..candidate("desktop_record", Scope::Standard)
                },
            )?;
        }
        for temp in self.session_temp_roots() {
            let namespace = identity.namespace.as_deref().expect("validated namespace");
            let scoped = temp.path.join(namespace).join(id);
            review.add(
                &temp.path,
                &scoped.join("scratchpad"),
                RootKind::Temporary,
                Rule {
                    ownership: Ownership::UserOwnedOutput,
                    evidence: "documented_temporary_layout",
                    ..candidate("scratchpad", Scope::Preserve)
                },
            );
            review.add(
                &temp.path,
                &scoped.join("images"),
                RootKind::Temporary,
                candidate("images", Scope::Deep),
            );
        }

        let (history_matches, history_observation) = history_stats(root, id, deadline);
        let registry = root.join("sessions");
        let process_registry = if crate::safe_path(&registry) && registry.is_dir() {
            "present_unverified"
        } else if std::fs::symlink_metadata(&registry).is_ok() {
            "blocked"
        } else {
            "not_observed"
        };
        verify(root, &path, &evidence.revision)?;
        let diagnostic_complete = review.complete
            && budget.issues == 0
            && verified_agents.len() == evidence.subagents
            && !matches!(history_observation, "partial_uuid_only" | "unverified");
        let checkpoint_present = review
            .artifacts
            .iter()
            .any(|entry| entry.category == "checkpoint");
        let categories = CATEGORIES
            .iter()
            .map(|&key| summarize(key, &review.artifacts, diagnostic_complete))
            .collect();
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
            observed: review.artifacts,
            diagnostic_complete,
            writer_observation: process::claude_writer_observation().status,
            fork_references: evidence.fork_references.clone(),
            engine_versions: evidence.versions.clone(),
            categories,
        })
    }
}
