//! Exact executable and one-session evidence for the admitted native command.
use crate::{CodexAdapter, worker::RemovalEvidence};
use agentmender_domain::{Session, SessionState};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

// Exact Darwin arm64 executable exercised against the disposable archive.
const CODEX_SHA256: &str = "3ea20787e106af94573d3d6c19480aed93a1334b0dd32b6b04e2f780ce82cf4b";
#[derive(Clone, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    revision: Option<(u64, i64, i64, i64, i64)>,
}

fn file_identity(path: &Path, directory: bool) -> Option<FileIdentity> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        use std::os::unix::fs::MetadataExt;

        let meta = fs::symlink_metadata(path).ok()?;
        if (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
            return None;
        }
        Some(FileIdentity {
            device: meta.dev(),
            inode: meta.ino(),
            revision: (!directory).then(|| {
                (
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

pub struct NativeRemoval {
    executable: PathBuf,
    executable_identity: FileIdentity,
    root: PathBuf,
    root_identity: FileIdentity,
    protected: Vec<String>,
}

pub struct RemovalReview(RemovalEvidence);

impl RemovalReview {
    pub fn native_id(&self) -> &str {
        &self.0.session.identity.native_id
    }
}

pub enum RemovalRefusal {
    Unsupported,
    Protected,
    SourceUnavailable,
    Changed,
}

pub enum RemovalOutcome {
    Changed,
    CommandSucceeded,
    OutcomeUnknown,
}

struct DigestWriter(Sha256);

impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl NativeRemoval {
    pub fn detect(executable: Option<&Path>, root: &Path, protected: Vec<String>) -> Option<Self> {
        let root_identity = file_identity(root, true)?;
        let root = root.canonicalize().ok()?;
        let executable = executable?.canonicalize().ok()?;
        let executable_identity = file_identity(&executable, false)?;
        let mut stream = fs::File::open(&executable).ok()?.take(512 * 1024 * 1024);
        let mut digest = DigestWriter(Sha256::new());
        io::copy(&mut stream, &mut digest).ok()?;
        let hash: String = digest
            .0
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let profile = Self {
            executable,
            executable_identity,
            root,
            root_identity,
            protected,
        };
        (hash == CODEX_SHA256 && profile.is_current()).then_some(profile)
    }

    pub fn is_current(&self) -> bool {
        file_identity(&self.executable, false).as_ref() == Some(&self.executable_identity)
            && file_identity(&self.root, true).as_ref() == Some(&self.root_identity)
    }

    pub fn prepare(
        &self,
        adapter: &CodexAdapter,
        session: &Session,
    ) -> Result<RemovalReview, RemovalRefusal> {
        if !self.is_current() || session.identity.state_root != self.root {
            return Err(RemovalRefusal::Unsupported);
        }
        if self.protected.contains(&session.identity.native_id) {
            return Err(RemovalRefusal::Protected);
        }
        if session.state != SessionState::Archived {
            return Err(RemovalRefusal::SourceUnavailable);
        }
        let evidence = adapter
            .removal_evidence(&session.identity)
            .map_err(|_| RemovalRefusal::SourceUnavailable)?;
        if &evidence.session != session {
            return Err(RemovalRefusal::Changed);
        }
        Ok(RemovalReview(evidence))
    }

    pub fn execute(&self, adapter: &CodexAdapter, review: RemovalReview) -> RemovalOutcome {
        let Ok(id) = uuid::Uuid::parse_str(review.native_id()) else {
            return RemovalOutcome::Changed;
        };
        // The worker rechecks the archived row, scoped rollout and spawn table
        // immediately before this single confined dispatch.
        if self
            .prepare(adapter, &review.0.session)
            .ok()
            .map(|fresh| fresh.0)
            .as_ref()
            != Some(&review.0)
            || !self.is_current()
        {
            return RemovalOutcome::Changed;
        }
        if agentmender_provider_api::process::run_confined_codex_delete(
            &self.executable,
            &self.root,
            id,
            Duration::from_secs(180),
        ) {
            #[cfg(target_os = "macos")]
            crate::desktop_reconciliation::schedule_desktop_catalog_reconciliation(&self.root, id);
            RemovalOutcome::CommandSucceeded
        } else {
            RemovalOutcome::OutcomeUnknown
        }
    }
}
