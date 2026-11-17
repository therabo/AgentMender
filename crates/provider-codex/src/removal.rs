//! Compatibility admission, reviewed scope and dispatch for Codex native deletion.
use crate::{
    CodexAdapter,
    worker::{ProjectEvidence, RemovalEvidence},
};
use agentmender_domain::{ProjectIdentity, ProviderKind, Session, SessionState};
use agentmender_provider_api::{
    ProviderError, filesystem,
    process::{self, NativeExit},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

// These release contracts were exercised with the native cascade fixture on Darwin arm64.
const VERIFIED_VERSIONS: &[&str] = &["0.169.0-alpha.11.1", "0.169.0"];

#[derive(Clone, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    revision: Option<(u32, u64, i64, i64, i64, i64)>,
}

fn file_identity(path: &Path, directory: bool) -> Option<FileIdentity> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        use std::os::unix::fs::MetadataExt;

        let meta = fs::symlink_metadata(path).ok()?;
        if (directory && !meta.is_dir())
            || (!directory && (!meta.is_file() || meta.mode() & 0o111 == 0))
        {
            return None;
        }
        Some(FileIdentity {
            device: meta.dev(),
            inode: meta.ino(),
            revision: (!directory).then(|| {
                (
                    meta.mode(),
                    meta.len(),
                    meta.mtime(),
                    meta.mtime_nsec(),
                    meta.ctime(),
                    meta.ctime_nsec(),
                )
            }),
        })
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = (path, directory);
        None
    }
}

#[derive(Clone, PartialEq, Eq)]
struct NativeIdentity {
    executable: PathBuf,
    executable_file: FileIdentity,
    root_directory: FileIdentity,
}

pub struct NativeRemoval {
    executable_input: PathBuf,
    root_input: PathBuf,
    root: PathBuf,
    identity: NativeIdentity,
    protected: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum NativeUnavailable {
    Platform,
    Executable,
    Root,
    VersionProbe,
    VersionContract,
    CommandProbe,
}

#[derive(PartialEq, Eq)]
pub struct RemovalReview {
    identity: NativeIdentity,
    evidence: RemovalEvidence,
    config_revision: Option<String>,
}

impl RemovalReview {
    pub fn session(&self) -> &Session {
        &self.evidence.session
    }

    pub fn native_id(&self) -> &str {
        &self.session().identity.native_id
    }

    pub fn total_sessions(&self) -> usize {
        self.evidence.scope.members.len()
    }
}

#[derive(PartialEq, Eq)]
pub struct ProjectRemovalReview {
    project: ProjectIdentity,
    identity: NativeIdentity,
    evidence: ProjectEvidence,
    config_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Subtree {
    pub main_id: String,
    pub total_sessions: usize,
}

impl ProjectRemovalReview {
    pub fn subtrees(&self) -> impl Iterator<Item = Subtree> + '_ {
        self.evidence.trees.iter().map(|tree| Subtree {
            main_id: tree.session.identity.native_id.clone(),
            total_sessions: tree.scope.members.len(),
        })
    }
}

pub enum ProjectStep {
    Dispatching(Subtree),
    Verified,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectOutcome {
    Verified,
    Changed,
    Refused(RemovalRefusal),
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemovalRefusal {
    NotMain,
    Protected,
    UnsupportedStorage,
    IncompleteRelationships,
    ExternalFork,
    AttachedThread,
    UnsafeMember,
    Changed,
}

impl From<ProviderError> for RemovalRefusal {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::InvalidScope(reason) if reason == "stale_codex_source" => Self::Changed,
            _ => Self::IncompleteRelationships,
        }
    }
}

pub enum RemovalOutcome {
    Changed,
    Verified,
    NativeUncertain(Option<crate::worker::ResidualEvidence>),
    VerificationIncomplete,
    Residual(crate::worker::ResidualEvidence),
}

fn config_revision(root: &Path) -> Result<Option<String>, RemovalRefusal> {
    match filesystem::open_source_read(root, &root.join("config.toml")) {
        Ok(file) => {
            let metadata = file
                .metadata()
                .map_err(|_| RemovalRefusal::IncompleteRelationships)?;
            #[cfg(any(target_os = "macos", all(test, unix)))]
            return Ok(Some(crate::reader::stamp(&metadata)));
            #[cfg(not(any(target_os = "macos", all(test, unix))))]
            {
                let _ = metadata;
                Err(RemovalRefusal::IncompleteRelationships)
            }
        }
        Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn check_writer_locks(root: &Path, evidence: &RemovalEvidence) -> Result<(), RemovalRefusal> {
    for member in &evidence.scope.members {
        let path = root
            .join("thread-writer-locks")
            .join(format!("{}.lock", member.id));
        match filesystem::open_source_read(root, &path) {
            Ok(file) => match file.try_lock_shared() {
                Ok(()) => {}
                Err(fs::TryLockError::WouldBlock) => return Err(RemovalRefusal::UnsafeMember),
                Err(fs::TryLockError::Error(_)) => {
                    return Err(RemovalRefusal::IncompleteRelationships);
                }
            },
            Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

impl NativeRemoval {
    pub fn detect(
        executable: Option<&Path>,
        root: &Path,
        protected: Vec<String>,
    ) -> Result<Self, NativeUnavailable> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            return Err(NativeUnavailable::Platform);
        }
        let root_identity = file_identity(root, true).ok_or(NativeUnavailable::Root)?;
        let root_input = root.to_path_buf();
        let root = root.canonicalize().map_err(|_| NativeUnavailable::Root)?;
        let executable_input =
            std::path::absolute(executable.ok_or(NativeUnavailable::Executable)?)
                .map_err(|_| NativeUnavailable::Executable)?;
        let executable = executable_input
            .canonicalize()
            .map_err(|_| NativeUnavailable::Executable)?;
        let executable_identity =
            file_identity(&executable, false).ok_or(NativeUnavailable::Executable)?;
        let profile = Self {
            executable_input,
            root_input,
            root,
            identity: NativeIdentity {
                executable,
                executable_file: executable_identity,
                root_directory: root_identity,
            },
            protected,
        };
        let version = process::probe_version(ProviderKind::Codex, &profile.identity.executable)
            .map_err(|_| NativeUnavailable::VersionProbe)?;
        if !VERIFIED_VERSIONS.contains(&version.as_str()) {
            return Err(NativeUnavailable::VersionContract);
        }
        if !process::probe_codex_delete(&profile.identity.executable) {
            return Err(NativeUnavailable::CommandProbe);
        }
        if !profile.is_current() {
            return Err(NativeUnavailable::Executable);
        }
        Ok(profile)
    }

    fn root_is_current(&self) -> bool {
        self.root_input
            .canonicalize()
            .is_ok_and(|path| path == self.root)
            && file_identity(&self.root_input, true).as_ref() == Some(&self.identity.root_directory)
            && file_identity(&self.root, true).as_ref() == Some(&self.identity.root_directory)
    }

    pub fn is_current(&self) -> bool {
        self.executable_input
            .canonicalize()
            .is_ok_and(|path| path == self.identity.executable)
            && file_identity(&self.identity.executable, false).as_ref()
                == Some(&self.identity.executable_file)
            && self.root_is_current()
    }

    pub fn prepare(
        &self,
        adapter: &CodexAdapter,
        session: &Session,
    ) -> Result<RemovalReview, RemovalRefusal> {
        if !session.is_main() {
            return Err(RemovalRefusal::NotMain);
        }
        if !self.is_current() || session.identity.state_root != self.root {
            return Err(RemovalRefusal::Changed);
        }
        if self.protected.contains(&session.identity.native_id) {
            return Err(RemovalRefusal::Protected);
        }
        if session.state != SessionState::Archived {
            return Err(RemovalRefusal::UnsafeMember);
        }
        let config_before = config_revision(&self.root)?;
        let evidence = adapter.removal_evidence(&session.identity)?;
        if &evidence.session != session
            || !self.is_current()
            || config_revision(&self.root)? != config_before
        {
            return Err(RemovalRefusal::Changed);
        }
        if evidence
            .scope
            .members
            .iter()
            .any(|member| self.protected.contains(&member.id))
        {
            return Err(RemovalRefusal::Protected);
        }
        check_writer_locks(&self.root, &evidence)?;
        Ok(RemovalReview {
            identity: self.identity.clone(),
            evidence,
            config_revision: config_before,
        })
    }

    pub fn prepare_project(
        &self,
        adapter: &CodexAdapter,
        project: &ProjectIdentity,
    ) -> Result<ProjectRemovalReview, RemovalRefusal> {
        let review = self.observe_project(adapter, project)?;
        if review.evidence.trees.is_empty() {
            return Err(RemovalRefusal::Changed);
        }
        Ok(review)
    }

    fn observe_project(
        &self,
        adapter: &CodexAdapter,
        project: &ProjectIdentity,
    ) -> Result<ProjectRemovalReview, RemovalRefusal> {
        if !self.is_current()
            || project.state_root != self.root
            || project.provider != ProviderKind::Codex
        {
            return Err(RemovalRefusal::Changed);
        }
        let config_before = config_revision(&self.root)?;
        let evidence = adapter.project_evidence(project)?;
        for tree in &evidence.trees {
            if tree
                .scope
                .members
                .iter()
                .any(|member| self.protected.contains(&member.id))
            {
                return Err(RemovalRefusal::Protected);
            }
            check_writer_locks(&self.root, tree)?;
        }
        if !self.is_current() || config_revision(&self.root)? != config_before {
            return Err(RemovalRefusal::Changed);
        }
        Ok(ProjectRemovalReview {
            project: project.clone(),
            identity: self.identity.clone(),
            evidence,
            config_revision: config_before,
        })
    }

    pub fn execute_project(
        &self,
        adapter: &CodexAdapter,
        review: ProjectRemovalReview,
        mut progress: impl FnMut(ProjectStep),
    ) -> ProjectOutcome {
        // Each verified tree leaves an exact fingerprint of the rest of
        // the archive. Check it while rebuilding every remaining scope.
        let mut expected_archive = None;
        let subtrees: Vec<_> = review.subtrees().collect();

        for (index, original) in review.evidence.trees.iter().enumerate() {
            let fresh = match self.observe_project(adapter, &review.project) {
                Ok(fresh) => fresh,
                Err(RemovalRefusal::Changed) => return ProjectOutcome::Changed,
                Err(error) => return ProjectOutcome::Refused(error),
            };
            if let Some(expected) = &expected_archive {
                if &fresh.evidence.archive_revision != expected
                    || fresh.identity != review.identity
                    || fresh.config_revision != review.config_revision
                    || fresh.evidence.trees.first().is_none_or(|tree| {
                        tree.scope.database_identity != original.scope.database_identity
                    })
                    || !fresh.subtrees().eq(subtrees[index..].iter().cloned())
                {
                    return ProjectOutcome::Changed;
                }
            } else if fresh != review {
                return ProjectOutcome::Changed;
            }
            let tree = fresh
                .evidence
                .trees
                .into_iter()
                .next()
                .expect("verified root");
            expected_archive = Some(tree.scope.unrelated_revision.clone());
            progress(ProjectStep::Dispatching(subtrees[index].clone()));
            match self.dispatch(
                adapter,
                RemovalReview {
                    identity: fresh.identity,
                    evidence: tree,
                    config_revision: fresh.config_revision,
                },
            ) {
                RemovalOutcome::Verified => progress(ProjectStep::Verified),
                RemovalOutcome::Changed => return ProjectOutcome::Changed,
                _ => return ProjectOutcome::Uncertain,
            }
        }
        ProjectOutcome::Verified
    }

    pub fn execute(&self, adapter: &CodexAdapter, review: RemovalReview) -> RemovalOutcome {
        let Ok(fresh) = self.prepare(adapter, review.session()) else {
            return RemovalOutcome::Changed;
        };
        if fresh != review {
            return RemovalOutcome::Changed;
        }
        self.dispatch(adapter, review)
    }

    fn dispatch(&self, adapter: &CodexAdapter, review: RemovalReview) -> RemovalOutcome {
        let Ok(id) = uuid::Uuid::parse_str(review.native_id()) else {
            return RemovalOutcome::Changed;
        };
        let scope = review.evidence.scope;
        let rollouts: Vec<_> = scope
            .members
            .iter()
            .filter_map(|member| member.rollout.as_deref())
            .collect();
        let exit = process::run_confined_codex_delete(
            &self.identity.executable,
            &self.root,
            id,
            &rollouts,
            Duration::from_secs(180),
            || {
                self.is_current()
                    && config_revision(&self.root)
                        .is_ok_and(|value| value == review.config_revision)
            },
        );
        if exit == NativeExit::Refused {
            return RemovalOutcome::Changed;
        }
        let verification = self
            .root_is_current()
            .then(|| adapter.verify_removal(scope))
            .filter(|_| self.root_is_current());
        match (exit, verification) {
            (NativeExit::Success, Some(Ok(None))) if self.root_is_current() => {
                #[cfg(target_os = "macos")]
                crate::desktop_reconciliation::schedule_desktop_catalog_reconciliation(
                    &self.root, id,
                );
                RemovalOutcome::Verified
            }
            (NativeExit::Success, Some(Ok(Some(residual)))) => RemovalOutcome::Residual(residual),
            (NativeExit::Success, _) => RemovalOutcome::VerificationIncomplete,
            (_, observation) => RemovalOutcome::NativeUncertain(
                observation
                    .and_then(Result::ok)
                    .map(Option::unwrap_or_default),
            ),
        }
    }
}
