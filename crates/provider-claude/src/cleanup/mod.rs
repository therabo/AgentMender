//! Permanent removal of provider-scoped local Claude files.
mod history;

use crate::{ClaudeAdapter, SessionEvidence, artifacts::exact_sidecar, read::verify};
use agentmender_domain::SessionIdentity;
use agentmender_provider_api::rewrite::{DIR_FLAGS, directory};
use rustix::fs::{self, AtFlags, Dir, FileType, Mode, Stat};
use serde::Serialize;
use std::{
    ffi::OsStr,
    fs::File,
    io,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

#[derive(Serialize)]
pub struct Target {
    #[serde(with = "agentmender_domain::path_serde")]
    root: PathBuf,
    #[serde(with = "agentmender_domain::path_serde")]
    relative_path: PathBuf,
    bytes: u64,
    #[serde(skip)]
    identity: (i32, u64),
    #[serde(skip)]
    kind: FileType,
    #[serde(skip)]
    exclusive_file: bool,
}

#[derive(Serialize)]
pub struct Plan {
    files: Vec<Target>,
    directories: Vec<Target>,
    logical_bytes: u64,
    history_matches: Option<usize>,
    notes: Vec<&'static str>,
    #[serde(skip)]
    uuid_scope: Option<UuidScope>,
}

struct UuidScope {
    root: PathBuf,
    id: String,
    transcript: PathBuf,
    roots: Vec<PathBuf>,
    namespace_files: usize,
    namespace_directories: usize,
}

const AMBIGUOUS_UUID: &str =
    "Risorse per UUID preservate perché lo stesso ID potrebbe appartenere a più sessioni.";

fn uuid_is_unique<'a>(mut roots: impl Iterator<Item = &'a Path>, transcript: &Path) -> bool {
    roots.all(|root| unique_transcript(root, transcript).unwrap_or(false))
}

#[derive(Serialize)]
pub struct Outcome {
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub history_rows: usize,
    pub errors: Vec<String>,
}

impl Plan {
    pub fn delete(mut self) -> Outcome {
        let uuid_scope = self.uuid_scope.take().filter(|scope| {
            if uuid_is_unique(scope.roots.iter().map(PathBuf::as_path), &scope.transcript) {
                return true;
            }
            self.files.truncate(scope.namespace_files);
            self.directories.truncate(scope.namespace_directories);
            self.notes.push(AMBIGUOUS_UUID);
            false
        });
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
        for target in self.directories.into_iter().rev() {
            if let Err(error) = target.delete()
                && error.kind() != io::ErrorKind::NotFound
            {
                outcome
                    .errors
                    .push(format!("{}: {error}", target.relative_path.display()));
            }
        }
        if let Some(scope) = uuid_scope {
            match history::remove_rows(&scope.root, &scope.id) {
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
    ) -> io::Result<Plan> {
        let root = &identity.state_root;
        let transcript = self
            .transcript_path(identity)
            .map_err(|_| io::ErrorKind::InvalidInput)?;
        verify(root, &transcript, &evidence.revision).map_err(|_| io::ErrorKind::InvalidData)?;
        let mut plan = Plan {
            files: Vec::new(),
            logical_bytes: 0,
            history_matches: None,
            notes: Vec::new(),
            directories: Vec::new(),
            uuid_scope: None,
        };
        plan.add_file(root, &transcript)?;
        let id = &identity.native_id;
        let namespace = transcript
            .parent()
            .expect("recognized transcript namespace");
        if let Err(error) = plan.sidecars(root, namespace, id) {
            plan.note_scan_error(error);
        }

        let protected_roots: Vec<_> = self
            .roots
            .iter()
            .map(|(path, _)| path.as_path())
            .chain(self.desktop_roots())
            .collect();
        plan.walk(root, &transcript.with_extension(""), &protected_roots);

        // Ownership of UUID-only files does not depend on diagnostic scan budgets.
        if uuid_is_unique(
            self.roots.iter().map(|(path, _)| path.as_path()),
            &transcript,
        ) {
            plan.uuid_scope = Some(UuidScope {
                root: root.clone(),
                id: id.clone(),
                transcript,
                roots: self.roots.iter().map(|(path, _)| path.clone()).collect(),
                namespace_files: plan.files.len(),
                namespace_directories: plan.directories.len(),
            });
            if let Some(desktop) = &evidence.desktop_artifact {
                if verify(&desktop.root, &desktop.path, &desktop.revision).is_ok() {
                    plan.candidate(&desktop.root, &desktop.path);
                } else {
                    plan.notes
                        .push("Registrazione Desktop cambiata o non leggibile.");
                }
            }
            for directory in ["session-env", "tasks", "file-history"] {
                plan.walk(root, &root.join(directory).join(id), &protected_roots);
            }
            plan.candidate(root, &root.join("debug").join(format!("{id}.txt")));
            plan.history_matches = history::count_rows(root, id).ok();
        } else {
            plan.notes.push(AMBIGUOUS_UUID);
        }
        Ok(plan)
    }
}

impl Plan {
    fn note_scan_error(&mut self, error: io::Error) {
        const NOTE: &str = "Alcune risorse associate sono protette o non leggibili.";
        if error.kind() != io::ErrorKind::NotFound && !self.notes.contains(&NOTE) {
            self.notes.push(NOTE);
        }
    }

    fn add_file(&mut self, root: &Path, path: &Path) -> io::Result<()> {
        let relative = relative_to(root, path)?;
        let (parent, name) = open_parent(root, relative)?;
        let info = fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if FileType::from_raw_mode(info.st_mode) != FileType::RegularFile || info.st_nlink != 1 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.add(Target::new(root, relative, &info, true));
        Ok(())
    }

    fn candidate(&mut self, root: &Path, path: &Path) {
        if let Err(error) = self.add_file(root, path) {
            self.note_scan_error(error);
        }
    }

    fn sidecars(&mut self, root: &Path, namespace: &Path, id: &str) -> io::Result<()> {
        let (parent, name) = open_parent(root, relative_to(root, namespace)?)?;
        let dir = open_child_dir(&parent, name)?;
        for entry in Dir::read_from(&dir)? {
            let entry = entry?;
            let name = OsStr::from_bytes(entry.file_name().to_bytes());
            if name
                .to_str()
                .and_then(|name| name.strip_prefix(id))
                .is_some_and(exact_sidecar)
            {
                self.candidate(root, &namespace.join(name));
            }
        }
        Ok(())
    }

    fn add(&mut self, target: Target) {
        if target.kind == FileType::Directory {
            self.directories.push(target);
        } else {
            self.logical_bytes = self.logical_bytes.saturating_add(target.bytes);
            self.files.push(target);
        }
    }

    fn walk(&mut self, root: &Path, path: &Path, protected_roots: &[&Path]) {
        let mut pending = vec![path.to_owned()];
        while let Some(path) = pending.pop() {
            if protected_roots.contains(&path.as_path()) {
                self.note_scan_error(io::ErrorKind::PermissionDenied.into());
                continue;
            }
            if let Err(error) = self.visit(root, &path, &mut pending) {
                self.note_scan_error(error);
            }
        }
    }

    fn visit(&mut self, root: &Path, path: &Path, pending: &mut Vec<PathBuf>) -> io::Result<()> {
        let relative = relative_to(root, path)?;
        let (parent, name) = open_parent(root, relative)?;
        let info = fs::statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        let kind = FileType::from_raw_mode(info.st_mode);
        if info.st_dev != fs::fstat(&parent)?.st_dev {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        self.add(Target::new(root, relative, &info, false));
        if kind == FileType::Directory {
            let dir = open_child_dir(&parent, name)?;
            for entry in Dir::read_from(&dir)? {
                let entry = entry?;
                let name = entry.file_name().to_bytes();
                if name != b"." && name != b".." {
                    pending.push(path.join(OsStr::from_bytes(name)));
                }
            }
        }
        Ok(())
    }
}

fn unique_transcript(root: &Path, selected: &Path) -> io::Result<bool> {
    let projects = match directory(&root.join("projects")) {
        Ok(projects) => projects,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error),
    };
    let transcript_name = selected
        .file_name()
        .expect("recognized transcript filename");
    for entry in Dir::read_from(&projects)? {
        let entry = entry?;
        let name = OsStr::from_bytes(entry.file_name().to_bytes());
        if name == "."
            || name == ".."
            || root.join("projects").join(name).join(transcript_name) == selected
        {
            continue;
        }
        let info = fs::statat(&projects, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if FileType::from_raw_mode(info.st_mode) == FileType::Directory {
            let namespace = open_child_dir(&projects, name)?;
            match fs::statat(&namespace, transcript_name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(_) => return Ok(false),
                Err(rustix::io::Errno::NOENT) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(true)
}

impl Target {
    fn new(root: &Path, relative: &Path, info: &Stat, exclusive_file: bool) -> Self {
        Self {
            root: root.to_owned(),
            relative_path: relative.to_owned(),
            bytes: logical_bytes(info),
            identity: (info.st_dev, info.st_ino),
            kind: FileType::from_raw_mode(info.st_mode),
            exclusive_file,
        }
    }

    fn delete(&self) -> io::Result<u64> {
        let (parent, name) = open_parent(&self.root, &self.relative_path)?;
        let current = fs::statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if (current.st_dev, current.st_ino) != self.identity
            || (self.exclusive_file && current.st_nlink != 1)
            || FileType::from_raw_mode(current.st_mode) != self.kind
        {
            return Err(io::Error::other("Risorsa cambiata dopo l’anteprima."));
        }
        let flags = if self.kind == FileType::Directory {
            AtFlags::REMOVEDIR
        } else {
            AtFlags::empty()
        };
        fs::unlinkat(parent, name, flags)?;
        Ok(logical_bytes(&current))
    }
}

fn logical_bytes(info: &Stat) -> u64 {
    if FileType::from_raw_mode(info.st_mode) == FileType::RegularFile && info.st_nlink == 1 {
        info.st_size.try_into().unwrap_or_default()
    } else {
        0
    }
}

fn relative_to<'a>(root: &Path, path: &'a Path) -> io::Result<&'a Path> {
    path.strip_prefix(root)
        .map_err(|_| io::ErrorKind::InvalidInput.into())
}

fn open_child_dir(parent: &File, name: &OsStr) -> io::Result<File> {
    let dir = fs::openat(parent, name, DIR_FLAGS, Mode::empty())?;
    if fs::fstat(&dir)?.st_dev != fs::fstat(parent)?.st_dev {
        return Err(io::Error::other(
            "Percorso su un volume estraneo all’archivio.",
        ));
    }
    Ok(File::from(dir))
}

fn open_parent<'a>(root: &Path, relative: &'a Path) -> io::Result<(File, &'a OsStr)> {
    let name = relative.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    let parent = relative.parent().ok_or(io::ErrorKind::InvalidInput)?;
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut dir = directory(root)?;
    for component in parent.components() {
        dir = open_child_dir(&dir, component.as_os_str())?;
    }
    Ok((dir, name))
}
