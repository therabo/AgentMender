//! Claude offline cleanup, scoped to the macOS layout exercised for engine 2.1.222.
mod backup;
mod directories;
mod files;
mod writers;

use crate::{
    ClaudeAdapter, SessionEvidence,
    read::{Lines, verify},
    storage::{Budget, EntryKind, history_stats},
};
use agentmender_domain::SessionIdentity;
use agentmender_provider_api::filesystem;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Instant,
};

pub use backup::{Backup, Outcome};
use files::{DirectoryTarget, Target, directory};

const ENGINE: &str = "2.1.222";
const MAX_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TARGETS: usize = 128;
const MAX_RECORDS: usize = 100_000;
pub const RESIDUAL_RISK: &str = "Tieni Claude Desktop, CLI e processi Claude chiusi fino all’esito. Un writer può riaprirsi dopo un controllo. Un’interruzione può lasciare una rimozione parziale recuperabile dal backup.";

#[derive(Clone, Copy, Default, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Standard,
    Deep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    ActiveWriter,
    WriterUnverified,
    OwnershipUnverified,
    Changed,
    UnsupportedFormat,
    InsufficientBackupSpace,
    PartialScope,
}

impl Refusal {
    pub fn message(self) -> &'static str {
        match self {
            Self::ActiveWriter => "Chiudi le istanze Claude. È stato rilevato un writer attivo.",
            Self::WriterUnverified => {
                "Stato dei writer Claude o dei marker di processo da verificare."
            }
            Self::OwnershipUnverified => {
                "Proprietà esclusiva dei file da verificare. Controlla duplicati e fork."
            }
            Self::Changed => "Archivio o backup cambiato. Richiedi una nuova anteprima.",
            Self::UnsupportedFormat => {
                "La rimozione richiede il layout Claude Code 2.1.222 verificato su macOS."
            }
            Self::InsufficientBackupSpace => {
                "Spazio o volume di backup insufficiente per questa rimozione."
            }
            Self::PartialScope => "Il perimetro dei file non può essere enumerato completamente.",
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct Review {
    session: SessionIdentity,
    mode: Mode,
    targets: Vec<Target>,
    empty_directories: Vec<DirectoryTarget>,
    excluded: BTreeSet<&'static str>,
    history_matches: Option<usize>,
    history_state: &'static str,
    history_revision: Option<crate::Revision>,
    history_lock_present: bool,
    desktop_links: usize,
    directories: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    unrelated: BTreeMap<SessionIdentity, crate::Revision>,
    required_bytes: u64,
}

#[derive(Serialize)]
pub struct Preview<'a> {
    session: &'a SessionIdentity,
    mode: Mode,
    operations: &'a [Target],
    empty_directories: &'a [DirectoryTarget],
    logical_bytes: u64,
    backup_required_bytes: u64,
    history_matches: Option<usize>,
    history_state: &'static str,
    excluded: &'a BTreeSet<&'static str>,
    residual_risk: &'static str,
    admission: &'static str,
}

impl Review {
    pub fn preview(&self) -> Preview<'_> {
        Preview {
            session: &self.session,
            mode: self.mode,
            operations: &self.targets,
            empty_directories: &self.empty_directories,
            logical_bytes: self.targets.iter().map(|target| target.bytes).sum(),
            backup_required_bytes: self.required_bytes,
            history_matches: self.history_matches,
            history_state: self.history_state,
            excluded: &self.excluded,
            residual_risk: RESIDUAL_RISK,
            admission: "eligible",
        }
    }
}

fn checkpoint_names(root: &Path, transcript: &Path) -> Result<BTreeSet<String>, Refusal> {
    let mut file =
        filesystem::open_observed_read(root, transcript).map_err(|_| Refusal::Changed)?;
    let size = file.metadata().map_err(|_| Refusal::Changed)?.len();
    if size > MAX_BYTES {
        return Err(Refusal::PartialScope);
    }
    let mut names = BTreeSet::new();
    let mut lines = Lines::new(&mut file, 0, size).map_err(|_| Refusal::Changed)?;
    for _ in 0..MAX_RECORDS {
        let Some(line) = lines.next().map_err(|_| Refusal::Changed)? else {
            return Ok(names);
        };
        let record: serde_json::Value =
            serde_json::from_slice(line.bytes.ok_or(Refusal::PartialScope)?)
                .map_err(|_| Refusal::PartialScope)?;
        if record["type"] != "file-history-snapshot" {
            continue;
        }
        let files = record["snapshot"]["trackedFileBackups"]
            .as_object()
            .ok_or(Refusal::PartialScope)?;
        for backup in files.values() {
            if let Some(name) = backup
                .get("backupFileName")
                .and_then(serde_json::Value::as_str)
                && files::checkpoint_name(name)
            {
                names.insert(name.to_owned());
            }
        }
    }
    Err(Refusal::PartialScope)
}

impl ClaudeAdapter {
    pub fn prepare_cleanup(
        &self,
        session: &SessionIdentity,
        mode: Mode,
        limit: usize,
    ) -> Result<Review, Refusal> {
        self.plan_cleanup(session, mode, limit, true)
    }

    fn plan_cleanup(
        &self,
        session: &SessionIdentity,
        mode: Mode,
        limit: usize,
        check_space: bool,
    ) -> Result<Review, Refusal> {
        writers::closed(&session.state_root, &session.native_id)?;
        let mut catalog = self.local_catalog(limit);
        if catalog
            .inventory
            .sessions
            .iter()
            .any(|row| row.forked_from.as_deref() == Some(session.native_id.as_str()))
        {
            return Err(Refusal::OwnershipUnverified);
        }
        let evidence = catalog.evidence.remove(session).ok_or(Refusal::Changed)?;
        let desktop_links = catalog
            .desktop_links
            .get(&session.native_id)
            .copied()
            .unwrap_or_default();
        self.plan(
            session,
            &evidence,
            mode,
            catalog.evidence,
            desktop_links,
            check_space,
        )
    }

    fn plan(
        &self,
        session: &SessionIdentity,
        evidence: &SessionEvidence,
        mode: Mode,
        mut unrelated: BTreeMap<SessionIdentity, SessionEvidence>,
        desktop_links: usize,
        check_space: bool,
    ) -> Result<Review, Refusal> {
        if evidence.sidechain || evidence.versions.as_slice() != [ENGINE] {
            return Err(Refusal::UnsupportedFormat);
        }
        if evidence.same_uuid_elsewhere != 0 || evidence.forks != 0 {
            return Err(Refusal::OwnershipUnverified);
        }
        if !evidence.metadata_complete || !evidence.ownership_scope_complete {
            return Err(Refusal::PartialScope);
        }
        let root = &session.state_root;
        let transcript = self
            .transcript_path(session)
            .map_err(|_| Refusal::Changed)?;
        let path = transcript.with_extension("");
        let mut budget = Budget::new(4096);
        let mut targets = Vec::new();
        let mut excluded = BTreeSet::new();
        let mut total = 0u64;
        let mut add = |target_root: &Path,
                       target: &Path,
                       role: &'static str,
                       expected: Option<&crate::Revision>| {
            if targets.len() >= MAX_TARGETS {
                return Err(Refusal::PartialScope);
            }
            let (file, size) = Target::inspect(
                target_root,
                target,
                role,
                expected,
                MAX_BYTES.saturating_sub(total),
            )?;
            total += size;
            targets.push(file);
            Ok(())
        };
        add(root, &transcript, "transcript", Some(&evidence.revision))?;
        let namespace = transcript.parent().ok_or(Refusal::Changed)?;
        let agent_folder = path.join("subagents");
        let mut agents = BTreeSet::new();
        for agent in &evidence.subagent_artifacts {
            if agent.path.parent() != Some(agent_folder.as_path()) {
                return Err(Refusal::OwnershipUnverified);
            }
            add(root, &agent.path, "subagent", Some(&agent.revision))?;
            agents.insert(agent.path.as_path());
        }
        if agents.len() != evidence.subagents {
            return Err(Refusal::PartialScope);
        }
        for entry in budget.list(&agent_folder) {
            if !agents.contains(entry.path.as_path()) {
                excluded.insert("Elementi aggiuntivi nella directory dei subagent");
            }
        }
        for entry in budget.list(&path) {
            if entry.path != agent_folder {
                excluded.insert("Dipendenze annidate da preservare, inclusi task e tool results");
            }
        }
        for entry in budget.list(namespace) {
            if entry.path != transcript
                && entry.path != path
                && entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.strip_prefix(&session.native_id)
                            .is_some_and(|suffix| suffix.starts_with(['.', '-']))
                    })
            {
                excluded.insert("Sidecar o varianti con proprietà non attestata");
            }
        }
        let session_env = root.join("session-env").join(&session.native_id);
        let mut empty_directories = Vec::new();
        if fs_present(&session_env)? {
            if directory(&session_env)
                .and_then(|dir| files::empty(&dir))
                .is_ok()
            {
                empty_directories.extend(DirectoryTarget::inspect(root, &session_env)?);
            } else {
                excluded.insert("Metadati session-env con proprietà non attestata");
            }
        }
        for name in ["tasks", "image-cache", "uploads", "dev-mods"] {
            if fs_present(&root.join(name).join(&session.native_id))? {
                excluded.insert("Stato locale o allegati con proprietà non attestata");
            }
        }
        for (dir, suffix) in [("debug", "txt"), ("todos", "json")] {
            if fs_present(
                &root
                    .join(dir)
                    .join(format!("{}.{suffix}", session.native_id)),
            )? {
                excluded.insert("Log o todo individuali con proprietà non attestata");
            }
        }
        if let Some(namespace) = session.namespace.as_deref() {
            for temp in self.session_temp_roots() {
                let scoped = temp.path.join(namespace).join(&session.native_id);
                if fs_present(&scoped.join("scratchpad"))? || fs_present(&scoped.join("images"))? {
                    excluded.insert("Scratchpad o immagini temporanee protetti");
                }
            }
        }
        if let Some(registration) = &evidence.desktop_artifact {
            excluded.insert("Scheduler e dati Desktop condivisi");
            if self.installations().iter().any(|item| {
                item.confirmed
                    && item.source == "desktop_bundle"
                    && item.version.as_deref() == Some("5.11492.5")
            }) {
                add(
                    &registration.root,
                    &registration.path,
                    "desktop_record",
                    Some(&registration.revision),
                )?;
            } else {
                excluded.insert("Registrazione Desktop di versione non verificata");
            }
        } else if desktop_links != 0
            || evidence.desktop_record
            || matches!(
                evidence.surface,
                Some(crate::local::Surface::Desktop | crate::local::Surface::Mixed)
            )
        {
            excluded.insert("Riferimenti Desktop e scheduler nel perimetro condiviso");
        }
        let checkpoint = root.join("file-history").join(&session.native_id);
        if fs_present(&checkpoint)? {
            if mode == Mode::Deep {
                let valid_names = checkpoint_names(root, &transcript)?;
                let entries = budget.list(&checkpoint);
                if entries.is_empty()
                    || entries.iter().any(|item| {
                        item.kind != EntryKind::File
                            || item
                                .path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .is_none_or(|name| !valid_names.contains(name))
                    })
                {
                    return Err(Refusal::OwnershipUnverified);
                }
                for item in entries {
                    add(root, &item.path, "checkpoint", None)?;
                }
            } else {
                excluded.insert("Checkpoint di rewind");
            }
        } else if mode == Mode::Deep {
            return Err(Refusal::UnsupportedFormat);
        }
        let directories = directory_fingerprints([namespace, &path, &agent_folder, &checkpoint])?;
        if budget.issues != 0 {
            return Err(Refusal::PartialScope);
        }
        let mut removed: BTreeSet<_> = targets
            .iter()
            .map(|target| target.root.join(&target.relative_path))
            .collect();
        for folder in [&agent_folder, &path, &checkpoint] {
            if folder == &checkpoint && mode != Mode::Deep {
                continue;
            }
            if directories[folder]
                .iter()
                .all(|entry| removed.contains(entry))
                && let Some(target) = DirectoryTarget::inspect(root, folder)?
            {
                removed.insert(folder.clone());
                empty_directories.push(target);
            }
        }
        verify(root, &transcript, &evidence.revision).map_err(|_| Refusal::Changed)?;
        let history_revision = history_revision(root);
        let history_lock_present = fs_present(&root.join("history.jsonl.lock"))?;
        let (history_matches, history_state) = history_stats(
            root,
            &session.native_id,
            Instant::now() + std::time::Duration::from_secs(2),
        );
        if history_matches.is_some_and(|count| count > 0)
            || matches!(history_state, "unverified" | "partial_uuid_only")
        {
            excluded.insert("History condivisa conservata integralmente");
        }
        let required_bytes = total.saturating_add(4096 * (targets.len() as u64 + 16));
        if check_space {
            let home = self.backup_home()?;
            files::check_space(&home, &targets, required_bytes)?;
        }
        let agent_prefix = format!("{}/agent:", session.native_id);
        unrelated.retain(|id, _| {
            id != session
                && !(id.state_root == session.state_root
                    && id.namespace == session.namespace
                    && id.native_id.starts_with(&agent_prefix))
        });
        Ok(Review {
            session: session.clone(),
            mode,
            targets,
            empty_directories,
            excluded,
            history_matches,
            history_state,
            history_revision,
            history_lock_present,
            desktop_links,
            directories,
            unrelated: unrelated
                .into_iter()
                .map(|(id, meta)| (id, meta.revision))
                .collect(),
            required_bytes,
        })
    }

    fn backup_home(&self) -> Result<PathBuf, Refusal> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(Refusal::PartialScope)?;
        let location = home.join(".agentmender");
        if !crate::local_path(&home)
            || self
                .roots
                .iter()
                .map(|(root, _)| root.as_path())
                .chain(self.desktop_roots())
                .any(|root| location.starts_with(root) || root.starts_with(&location))
        {
            return Err(Refusal::PartialScope);
        }
        directory(&home).map_err(|_| Refusal::PartialScope)?;
        Ok(home)
    }
}

fn history_revision(root: &Path) -> Option<crate::Revision> {
    let path = root.join("history.jsonl");
    let file = filesystem::open_observed_read(root, &path).ok()?;
    crate::Revision::read(&file, &path).ok()
}

fn directory_fingerprints<'a>(
    paths: impl IntoIterator<Item = &'a Path>,
) -> Result<BTreeMap<PathBuf, BTreeSet<PathBuf>>, Refusal> {
    let mut budget = Budget::new(4096);
    let result = paths
        .into_iter()
        .map(|path| {
            let entries = budget
                .list(path)
                .into_iter()
                .map(|entry| entry.path)
                .collect();
            (path.to_owned(), entries)
        })
        .collect();
    if budget.issues != 0 {
        return Err(Refusal::PartialScope);
    }
    Ok(result)
}

fn fs_present(path: &Path) -> Result<bool, Refusal> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Refusal::PartialScope),
    }
}
