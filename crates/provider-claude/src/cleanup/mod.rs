//! Permanent removal of provider-scoped local Claude files.
mod history;

use crate::{
    ClaudeAdapter, SessionEvidence,
    artifacts::exact_sidecar,
    read::verify,
    storage::{Budget, EntryKind, history_stats},
};
use agentmender_domain::SessionIdentity;
use rustix::fs::{self as fs, AtFlags, FileType, OFlags};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Standard,
    Deep,
}

#[derive(Serialize)]
pub struct Target {
    #[serde(with = "agentmender_domain::path_serde")]
    root: PathBuf,
    #[serde(with = "agentmender_domain::path_serde")]
    relative_path: PathBuf,
    bytes: u64,
    #[serde(skip)]
    identity: (i64, u64),
}

#[derive(Serialize)]
pub struct Plan {
    mode: Mode,
    files: Vec<Target>,
    logical_bytes: u64,
    history_matches: Option<usize>,
    notes: Vec<&'static str>,
    #[serde(skip)]
    directories: Vec<(PathBuf, PathBuf)>,
    #[serde(skip)]
    history: Option<(PathBuf, String)>,
}

#[derive(Serialize)]
pub struct Outcome {
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub history_rows: usize,
    pub errors: Vec<String>,
}

impl Plan {
    pub fn delete(self) -> Outcome {
        let mut outcome = Outcome {
            removed_files: 0,
            removed_bytes: 0,
            history_rows: 0,
            errors: self.notes.into_iter().map(str::to_owned).collect(),
        };
        for (index, target) in self.files.into_iter().enumerate() {
            match target.delete() {
                Ok(bytes) => {
                    outcome.removed_files += 1;
                    outcome.removed_bytes += bytes;
                }
                Err(error) => {
                    outcome
                        .errors
                        .push(format!("{}: {error}", target.relative_path.display()));
                    if index == 0 {
                        return outcome;
                    }
                }
            }
        }
        for (root, path) in self.directories {
            if let Err(error) = remove_directory(&root, &path)
                && !matches!(
                    error.kind(),
                    io::ErrorKind::DirectoryNotEmpty | io::ErrorKind::NotFound
                )
            {
                outcome.errors.push(format!("{}: {error}", path.display()));
            }
        }
        if let Some((root, id)) = self.history {
            match history::remove_rows(&root, &id) {
                Ok(count) => outcome.history_rows = count,
                Err(error) => outcome.errors.push(format!("history.jsonl: {error}")),
            }
        }
        outcome
    }
}

impl ClaudeAdapter {
    pub fn removal_plan(
        &self,
        identity: &SessionIdentity,
        evidence: &SessionEvidence,
        mode: Mode,
    ) -> io::Result<Plan> {
        let root = &identity.state_root;
        let transcript = self
            .transcript_path(identity)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        verify(root, &transcript, &evidence.revision).map_err(|_| io::ErrorKind::InvalidData)?;
        let mut plan = Plan {
            mode,
            files: Vec::new(),
            logical_bytes: 0,
            history_matches: None,
            notes: Vec::new(),
            directories: Vec::new(),
            history: None,
        };
        plan.add_file(root, &transcript)?;
        let id = &identity.native_id;
        let mut budget = Budget::new(8192);
        let namespace = transcript
            .parent()
            .expect("recognized transcript namespace");
        for entry in budget.list(namespace) {
            if entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix(id))
                .is_some_and(exact_sidecar)
            {
                plan.candidate(root, &entry.path);
            }
        }

        let nested = transcript.with_extension("");
        let agents = nested.join("subagents");
        let mut known_agents = BTreeSet::new();
        for agent in &evidence.subagent_artifacts {
            if verify(&agent.root, &agent.path, &agent.revision).is_ok() {
                known_agents.insert(agent.path.clone());
                known_agents.insert(agent.path.with_extension("meta.json"));
                plan.candidate(root, &agent.path);
                plan.candidate(root, &agent.path.with_extension("meta.json"));
            } else {
                plan.note("Subagent cambiato o non leggibile.");
            }
        }
        if budget
            .list(&agents)
            .iter()
            .any(|entry| !known_agents.contains(&entry.path))
        {
            plan.note("Risorse non riconosciute nella directory dei subagent.");
        }
        plan.prune(root, &agents);
        for entry in budget.list(&nested) {
            if entry.path == agents {
                continue;
            }
            if matches!(
                entry.path.file_name().and_then(|name| name.to_str()),
                Some("tool-results" | "mcp-tasks")
            ) {
                plan.walk(root, &entry.path, &mut budget, 0);
            } else {
                plan.note("Risorse non riconosciute nella directory della sessione.");
            }
        }
        plan.prune(root, &nested);

        if evidence.same_uuid_elsewhere == 0 && evidence.ownership_scope_complete {
            if let Some(desktop) = &evidence.desktop_artifact {
                if verify(&desktop.root, &desktop.path, &desktop.revision).is_ok() {
                    plan.candidate(&desktop.root, &desktop.path);
                } else {
                    plan.note("Registrazione Desktop cambiata o non leggibile.");
                }
            }
            for directory in ["session-env", "tasks"] {
                plan.walk(root, &root.join(directory).join(id), &mut budget, 0);
            }
            plan.candidate(root, &root.join("debug").join(format!("{id}.txt")));
            if matches!(mode, Mode::Deep) {
                let checkpoints = root.join("file-history").join(id);
                plan.walk(root, &checkpoints, &mut budget, 0);
            }
            plan.history_matches = history_stats(root, id, budget.deadline).0;
            plan.history = Some((root.clone(), id.clone()));
        } else {
            plan.note("Risorse per UUID preservate perché lo stesso ID potrebbe appartenere a più sessioni.");
        }
        if budget.issues != 0 {
            plan.note(
                "Alcune risorse protette o fuori dal limite della scansione sono state conservate.",
            );
        }
        Ok(plan)
    }
}

impl Plan {
    fn note(&mut self, message: &'static str) {
        if !self.notes.contains(&message) {
            self.notes.push(message);
        }
    }

    fn add_file(&mut self, root: &Path, path: &Path) -> io::Result<()> {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        let (parent, name) = open_parent(root, relative)?;
        let info = fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if FileType::from_raw_mode(info.st_mode) != FileType::RegularFile || info.st_nlink != 1 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let bytes = info
            .st_size
            .try_into()
            .map_err(|_| io::ErrorKind::InvalidData)?;
        self.logical_bytes = self.logical_bytes.saturating_add(bytes);
        self.files.push(Target {
            root: root.to_owned(),
            relative_path: relative.to_owned(),
            bytes,
            identity: (i64::from(info.st_dev), info.st_ino),
        });
        Ok(())
    }

    fn candidate(&mut self, root: &Path, path: &Path) {
        if let Err(error) = self.add_file(root, path)
            && error.kind() != io::ErrorKind::NotFound
        {
            self.note("Alcuni file associati sono protetti o non leggibili.");
        }
    }

    fn walk(&mut self, root: &Path, path: &Path, budget: &mut Budget, depth: usize) {
        if depth >= 16 {
            self.note("Directory di sessione oltre il limite di profondità.");
            return;
        }
        for entry in budget.list(path) {
            match entry.kind {
                EntryKind::File => self.candidate(root, &entry.path),
                EntryKind::Directory => self.walk(root, &entry.path, budget, depth + 1),
                EntryKind::Protected => self.note("Collegamento o file speciale preservato."),
            }
        }
        self.prune(root, path);
    }

    fn prune(&mut self, root: &Path, path: &Path) {
        if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir()) {
            self.directories.push((
                root.to_owned(),
                path.strip_prefix(root).expect("fixed scope").to_owned(),
            ));
        }
    }
}

impl Target {
    fn delete(&self) -> io::Result<u64> {
        let (parent, name) = open_parent(&self.root, &self.relative_path)?;
        let current = fs::statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if (i64::from(current.st_dev), current.st_ino) != self.identity
            || current.st_nlink != 1
            || FileType::from_raw_mode(current.st_mode) != FileType::RegularFile
        {
            return Err(io::Error::other("File cambiato dopo l’anteprima."));
        }
        fs::unlinkat(parent, name, AtFlags::empty())?;
        Ok(current.st_size.try_into().unwrap_or_default())
    }
}

const DIR_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

fn directory(path: &Path) -> io::Result<File> {
    let mut parts = path.components();
    if parts.next() != Some(Component::RootDir) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut dir = fs::open("/", DIR_FLAGS, fs::Mode::empty())?;
    for part in parts {
        let Component::Normal(name) = part else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        dir = fs::openat(dir, name, DIR_FLAGS, fs::Mode::empty())?;
    }
    Ok(File::from(dir))
}

fn open_parent<'a>(root: &Path, relative: &'a Path) -> io::Result<(File, &'a std::ffi::OsStr)> {
    let name = relative.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    let parent = relative.parent().ok_or(io::ErrorKind::InvalidInput)?;
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut dir = directory(root)?;
    let device = fs::fstat(&dir)?.st_dev;
    for component in parent.components() {
        dir = File::from(fs::openat(
            &dir,
            component.as_os_str(),
            DIR_FLAGS,
            fs::Mode::empty(),
        )?);
        if fs::fstat(&dir)?.st_dev != device {
            return Err(io::Error::other(
                "Percorso su un volume estraneo all’archivio.",
            ));
        }
    }
    Ok((dir, name))
}

fn remove_directory(root: &Path, path: &Path) -> io::Result<()> {
    let (parent, name) = open_parent(root, path)?;
    fs::unlinkat(parent, name, AtFlags::REMOVEDIR)?;
    Ok(())
}
