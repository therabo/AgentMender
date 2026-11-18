//! Verified recovery copies and descriptor-relative, no-clobber file moves.
use super::{
    ClaudeAdapter, MAX_BYTES, MAX_TARGETS, Mode, Refusal, Review, directories,
    files::{self, DirectoryTarget, Target},
    writers,
};
use agentmender_domain::SessionIdentity;
use rustix::fs::{self as fs_at, AtFlags, FlockOperation, RenameFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    path::Component,
};

const MANIFEST: &str = "manifest.json";
const MAX_MANIFEST: u64 = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Copying,
    Ready,
    Mutating,
    Verified,
    Partial,
    Restoring,
    Restored,
    Discarding,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    id: String,
    session: SessionIdentity,
    mode: Mode,
    phase: Phase,
    targets: Vec<Target>,
    empty_directories: Vec<DirectoryTarget>,
    restored: BTreeMap<usize, (u64, u64)>,
    pending_restore: Option<(usize, (u64, u64))>,
}

impl Manifest {
    fn file_parent(&self, dir: &File, target: &Target) -> io::Result<File> {
        let path = target.root.join(&target.relative_path);
        let parent = directories::parent(dir, &self.empty_directories, &path, target.parent())?;
        target.check_parent(&parent)?;
        Ok(parent)
    }

    fn restored_identity(&self, index: usize) -> Option<&(u64, u64)> {
        self.restored.get(&index).or_else(|| {
            self.pending_restore
                .as_ref()
                .filter(|(pending, _)| *pending == index)
                .map(|(_, identity)| identity)
        })
    }
}

#[derive(Serialize)]
pub struct Backup {
    id: String,
    native_id: String,
    mode: Mode,
    phase: Phase,
    logical_bytes: u64,
    can_restore: bool,
    can_discard: bool,
}

#[derive(Serialize)]
pub struct Outcome {
    status: &'static str,
    backup_id: Option<String>,
    logical_bytes: u64,
    reason: Option<Refusal>,
}

impl Outcome {
    fn new(
        status: &'static str,
        id: Option<&str>,
        logical_bytes: u64,
        reason: Option<Refusal>,
    ) -> Self {
        Self {
            status,
            backup_id: id.map(str::to_owned),
            logical_bytes,
            reason,
        }
    }

    fn refused(reason: Refusal, id: Option<&str>) -> Self {
        Self::new(
            if reason == Refusal::Changed {
                "changed"
            } else {
                "refused"
            },
            id,
            0,
            Some(reason),
        )
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn object(index: usize) -> String {
    format!("file-{index}")
}
fn holding(index: usize) -> String {
    format!("holding-{index}")
}
fn restoring(index: usize) -> String {
    format!("restore-{index}")
}

fn sync_manifest(dir: &File, manifest: &Manifest) -> io::Result<()> {
    let pending = format!("manifest-{:?}.new", manifest.phase);
    // This name belongs to the private backup. A departed process may have
    // left a temporary manifest. Preserve it unless it is a regular file.
    if !files::absent(dir, OsStr::new(&pending))? {
        files::read_at(dir, OsStr::new(&pending))?;
        fs_at::unlinkat(dir, pending.as_str(), AtFlags::empty())?;
    }
    let mut file = files::new_file(dir, &pending)?;
    let bytes = serde_json::to_vec(manifest)?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err(io::ErrorKind::InvalidData.into());
    }
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs_at::renameat(dir, pending.as_str(), dir, MANIFEST)?;
    dir.sync_all()
}

fn store_lock(root: &File) -> io::Result<File> {
    let name = "cleanup.lock";
    let file = match files::new_file(root, name) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            files::read_at(root, OsStr::new(name))?
        }
        Err(error) => return Err(error),
    };
    fs_at::flock(&file, FlockOperation::NonBlockingLockExclusive)?;
    Ok(file)
}

fn transfer_copy(source: &mut File, target: &Target, dest: &File, name: &str) -> io::Result<File> {
    let mut copy = files::new_file(dest, name)?;
    if files::transfer(source, target.bytes, &mut copy)? != target.sha256 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    copy.sync_all()?;
    target.verify_data(files::read_at(dest, OsStr::new(name))?)?;
    Ok(copy)
}

fn write_backup(dir: &File, manifest: &mut Manifest) -> io::Result<()> {
    sync_manifest(dir, manifest)?;
    for (index, target) in manifest.targets.iter().enumerate() {
        let (_, mut source) = target.open_current()?;
        transfer_copy(&mut source, target, dir, &object(index))?;
        if !target.matches(&source, false)? {
            return Err(io::ErrorKind::InvalidData.into());
        }
        target.open_current()?;
    }
    manifest.phase = Phase::Ready;
    sync_manifest(dir, manifest)
}

fn move_verified(dir: &File, target: &Target, index: usize) -> io::Result<()> {
    let (parent, original) = target.open_current()?;
    let name = holding(index);
    fs_at::renameat_with(
        &parent,
        target.name(),
        dir,
        name.as_str(),
        RenameFlags::NOREPLACE,
    )?;
    // A writer may replace the path after opening. Inspect what was moved,
    // and put unexpected entries back without replacing a recreated file.
    let check = || {
        let moved = files::read_at(dir, OsStr::new(&name))?;
        if !target.matches(&moved, true)? || !target.matches(&original, true)? {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        let moved_revision = crate::Revision::read(&moved, &target.relative_path)
            .map_err(|_| io::ErrorKind::InvalidData)?;
        target.verify_data(moved)?;
        if crate::Revision::read(&original, &target.relative_path)
            .map_err(|_| io::ErrorKind::InvalidData)?
            != moved_revision
        {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        Ok(())
    };
    if let Err(error) = check() {
        let _ = fs_at::renameat_with(
            dir,
            name.as_str(),
            &parent,
            target.name(),
            RenameFlags::NOREPLACE,
        );
        let _ = parent.sync_all();
        let _ = dir.sync_all();
        return Err(error);
    }
    parent.sync_all()?;
    dir.sync_all()?;
    fs_at::unlinkat(dir, name.as_str(), AtFlags::empty())?;
    dir.sync_all()
}

fn move_targets(
    dir: &File,
    manifest: &Manifest,
    mut closed: impl FnMut(&SessionIdentity) -> Result<(), Refusal>,
) -> Result<u64, (u64, Refusal)> {
    let mut bytes = 0;
    for (index, target) in manifest.targets.iter().enumerate() {
        closed(&manifest.session).map_err(|reason| (bytes, reason))?;
        move_verified(dir, target, index).map_err(|_| (bytes, Refusal::Changed))?;
        bytes += target.bytes;
    }
    Ok(bytes)
}

fn closed(session: &SessionIdentity) -> Result<(), Refusal> {
    writers::closed(&session.state_root, &session.native_id)
}

impl ClaudeAdapter {
    pub fn execute_cleanup(&self, reviewed: Review, id: &str, limit: usize) -> Outcome {
        if !valid_id(id) {
            return Outcome::refused(Refusal::Changed, None);
        }
        let fresh = || self.plan_cleanup(&reviewed.session, reviewed.mode, limit, false);
        match fresh() {
            Ok(plan) if plan == reviewed => {}
            Ok(_) => return Outcome::refused(Refusal::Changed, None),
            Err(reason) => return Outcome::refused(reason, None),
        }
        let Ok(home) = self.backup_home() else {
            return Outcome::refused(Refusal::PartialScope, None);
        };
        let Ok(Some(root)) = files::backup_parent(&home, true) else {
            return Outcome::refused(Refusal::PartialScope, None);
        };
        let Ok(_lock) = store_lock(&root) else {
            return Outcome::refused(Refusal::WriterUnverified, None);
        };
        if let Err(reason) = files::check_space(&home, &reviewed.targets, reviewed.required_bytes) {
            return Outcome::refused(reason, None);
        }
        let Ok(backup) = files::make_private(&root, id, false) else {
            return Outcome::refused(Refusal::Changed, None);
        };
        let mut manifest = Manifest {
            version: 1,
            id: id.into(),
            session: reviewed.session.clone(),
            mode: reviewed.mode,
            phase: Phase::Copying,
            targets: reviewed.targets.clone(),
            empty_directories: reviewed.empty_directories.clone(),
            restored: BTreeMap::new(),
            pending_restore: None,
        };
        if write_backup(&backup, &mut manifest).is_err() {
            return Outcome::refused(Refusal::Changed, Some(id));
        }
        match fresh() {
            Ok(plan) if plan == reviewed => {}
            Ok(_) => return Outcome::refused(Refusal::Changed, Some(id)),
            Err(reason) => return Outcome::refused(reason, Some(id)),
        }
        manifest.phase = Phase::Mutating;
        if sync_manifest(&backup, &manifest).is_err() {
            return Outcome::refused(Refusal::Changed, Some(id));
        }
        let bytes = match move_targets(&backup, &manifest, closed) {
            Ok(bytes) => bytes,
            Err((bytes, reason)) => {
                return Outcome::new("recovery_required", Some(id), bytes, Some(reason));
            }
        };
        for (index, target) in manifest.empty_directories.iter().enumerate() {
            if let Err(reason) = closed(&manifest.session) {
                return Outcome::new("recovery_required", Some(id), bytes, Some(reason));
            }
            if directories::park(&backup, target, index).is_err() {
                return Outcome::new("recovery_required", Some(id), bytes, Some(Refusal::Changed));
            }
        }
        let catalog = self.local_catalog(limit);
        let desktop_links = catalog
            .desktop_links
            .get(&manifest.session.native_id)
            .copied()
            .unwrap_or_default();
        let expected_desktop_links = reviewed.desktop_links.saturating_sub(
            manifest
                .targets
                .iter()
                .filter(|target| target.role == "desktop_record")
                .count(),
        );
        let unrelated: BTreeMap<_, _> = catalog
            .evidence
            .into_iter()
            .map(|(id, evidence)| (id, evidence.revision))
            .collect();
        let missing = manifest.targets.iter().all(|target| {
            matches!(
                manifest
                    .file_parent(&backup, target)
                    .and_then(|parent| files::absent(&parent, target.name())),
                Ok(true)
            )
        });
        let mut expected_directories = reviewed.directories;
        for path in manifest
            .targets
            .iter()
            .map(|target| target.root.join(&target.relative_path))
            .chain(manifest.empty_directories.iter().map(DirectoryTarget::path))
        {
            if let Some(entries) = path
                .parent()
                .and_then(|parent| expected_directories.get_mut(parent))
            {
                entries.remove(&path);
            }
        }
        let actual_directories =
            super::directory_fingerprints(expected_directories.keys().map(|path| path.as_path()));
        let history_unchanged = super::history_revision(&manifest.session.state_root)
            == reviewed.history_revision
            && super::fs_present(&manifest.session.state_root.join("history.jsonl.lock"))
                == Ok(reviewed.history_lock_present);
        if !missing
            || catalog.inventory.incomplete
            || desktop_links != expected_desktop_links
            || unrelated != reviewed.unrelated
            || actual_directories != Ok(expected_directories)
            || !history_unchanged
            || directories::verify(&backup, &manifest.empty_directories, true).is_err()
            || writers::closed(&manifest.session.state_root, &manifest.session.native_id).is_err()
        {
            return Outcome::new("uncertain", Some(id), bytes, Some(Refusal::Changed));
        }
        manifest.phase = if reviewed.excluded.is_empty() {
            Phase::Verified
        } else {
            Phase::Partial
        };
        if sync_manifest(&backup, &manifest).is_err() {
            return Outcome::new("uncertain", Some(id), bytes, Some(Refusal::Changed));
        }
        Outcome::new(
            if manifest.phase == Phase::Verified {
                "verified"
            } else {
                "partial"
            },
            Some(id),
            bytes,
            None,
        )
    }

    fn load_manifest(&self, root: &File, id: &str) -> io::Result<(File, Manifest)> {
        if !valid_id(id) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let dir = files::directory_at(root, id)?;
        files::require_private(&dir)?;
        let mut bytes = Vec::new();
        files::read_at(&dir, OsStr::new(MANIFEST))?
            .take(MAX_MANIFEST + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_MANIFEST {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        if !self.valid_manifest(&manifest, id) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok((dir, manifest))
    }

    fn valid_manifest(&self, manifest: &Manifest, id: &str) -> bool {
        let Ok(transcript) = self.transcript_path(&manifest.session) else {
            return false;
        };
        if manifest.version != 1
            || manifest.id != id
            || manifest.targets.is_empty()
            || manifest.targets.len() > MAX_TARGETS
            || manifest.restored.len() > manifest.targets.len()
            || manifest.targets[0].role != "transcript"
        {
            return false;
        }
        let root = &manifest.session.state_root;
        let nested = transcript.with_extension("");
        let agents = nested.join("subagents");
        let checkpoint = root.join("file-history").join(&manifest.session.native_id);
        let env = root.join("session-env").join(&manifest.session.native_id);
        let allowed_dirs = [&env, &agents, &nested, &checkpoint];
        let mut previous = None;
        for target in &manifest.empty_directories {
            let Some(order) = allowed_dirs.iter().position(|path| **path == target.path()) else {
                return false;
            };
            if &target.root != root
                || previous.is_some_and(|prev| prev >= order)
                || (order == 3 && manifest.mode != Mode::Deep)
                || !target
                    .relative_path
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return false;
            }
            previous = Some(order);
        }
        let mut seen = BTreeSet::new();
        let mut bytes = 0u64;
        manifest.targets.iter().all(|target| {
            let parts = target.relative_path.components();
            if target.relative_path.as_os_str().is_empty()
                || !parts
                    .clone()
                    .into_iter()
                    .all(|part| matches!(part, Component::Normal(_)))
                || !valid_id(&target.sha256)
            {
                return false;
            }
            let path = target.root.join(&target.relative_path);
            bytes = bytes.saturating_add(target.bytes);
            if bytes > MAX_BYTES || !seen.insert(path.clone()) {
                return false;
            }
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            match target.role.as_str() {
                "transcript" => &target.root == root && path == transcript,
                "subagent" => {
                    &target.root == root
                        && path.parent() == Some(&agents)
                        && filename
                            .strip_prefix("agent-")
                            .and_then(|name| name.strip_suffix(".jsonl"))
                            .is_some_and(|id| {
                                !id.is_empty()
                                    && id.bytes().all(|b| {
                                        b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
                                    })
                            })
                }
                "checkpoint" => {
                    manifest.mode == Mode::Deep
                        && &target.root == root
                        && path.parent() == Some(&checkpoint)
                        && files::checkpoint_name(filename)
                }
                "desktop_record" => {
                    self.desktop_roots().any(|root| root == target.root)
                        && target.relative_path.starts_with("claude-code-sessions")
                        && target.relative_path.components().count() == 4
                        && filename
                            .strip_prefix("local_")
                            .and_then(|name| name.strip_suffix(".json"))
                            .is_some_and(agentmender_domain::uuid_shape)
                }
                _ => false,
            }
        })
    }

    pub fn cleanup_backups(&self) -> Result<Vec<Backup>, Refusal> {
        let home = self.backup_home()?;
        let Some(root) = files::backup_parent(&home, false).map_err(|_| Refusal::PartialScope)?
        else {
            return Ok(Vec::new());
        };
        let entries = fs_at::Dir::read_from(&root).map_err(|_| Refusal::PartialScope)?;
        let mut out = Vec::new();
        for entry in entries.take(1024) {
            let entry = entry.map_err(|_| Refusal::PartialScope)?;
            let Ok(id) = entry.file_name().to_str() else {
                continue;
            };
            if let Ok((_, m)) = self.load_manifest(&root, id) {
                out.push(Backup {
                    id: m.id,
                    native_id: m.session.native_id,
                    mode: m.mode,
                    phase: m.phase,
                    logical_bytes: m.targets.iter().map(|target| target.bytes).sum(),
                    can_restore: can_restore(m.phase),
                    can_discard: can_discard(m.phase),
                });
            }
        }
        out.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub fn check_backup(&self, id: &str, discard: bool) -> Result<[u8; 32], Refusal> {
        let home = self.backup_home()?;
        let root = files::backup_parent(&home, false)
            .map_err(|_| Refusal::Changed)?
            .ok_or(Refusal::Changed)?;
        let (_, manifest) = self
            .load_manifest(&root, id)
            .map_err(|_| Refusal::Changed)?;
        if discard && can_discard(manifest.phase) {
            return fingerprint(&manifest);
        }
        if !discard && can_restore(manifest.phase) {
            writers::closed(&manifest.session.state_root, &manifest.session.native_id)?;
            return fingerprint(&manifest);
        }
        Err(Refusal::Changed)
    }

    pub fn act_on_backup(&self, id: &str, discard: bool, expected: [u8; 32]) -> Outcome {
        let Ok(home) = self.backup_home() else {
            return Outcome::refused(Refusal::PartialScope, None);
        };
        let Ok(Some(root)) = files::backup_parent(&home, false) else {
            return Outcome::refused(Refusal::Changed, None);
        };
        let Ok(_lock) = store_lock(&root) else {
            return Outcome::refused(Refusal::WriterUnverified, Some(id));
        };
        let Ok((dir, mut manifest)) = self.load_manifest(&root, id) else {
            return Outcome::refused(Refusal::Changed, None);
        };
        if fingerprint(&manifest) != Ok(expected) {
            return Outcome::refused(Refusal::Changed, Some(id));
        }
        if discard && can_discard(manifest.phase) {
            return match discard_backup(&root, &dir, &mut manifest) {
                Ok(()) => Outcome::new("discarded", None, 0, None),
                Err(_) => Outcome::new("uncertain", Some(id), 0, Some(Refusal::Changed)),
            };
        }
        if discard || !can_restore(manifest.phase) {
            return Outcome::refused(Refusal::Changed, Some(id));
        }
        if let Err(reason) =
            writers::closed(&manifest.session.state_root, &manifest.session.native_id)
        {
            return Outcome::refused(reason, Some(id));
        }
        // Check all backup data and destinations before restoring even one.
        if directories::verify(&dir, &manifest.empty_directories, false).is_err() {
            return Outcome::refused(Refusal::Changed, Some(id));
        }
        for (index, target) in manifest.targets.iter().enumerate() {
            if restore_preflight(&dir, index, target, &manifest).is_err() {
                return Outcome::refused(Refusal::Changed, Some(id));
            }
        }
        manifest.phase = Phase::Restoring;
        if restore_targets(&dir, &mut manifest, closed).is_err() {
            return Outcome::new("recovery_required", Some(id), 0, Some(Refusal::Changed));
        }
        Outcome::new("restored", Some(id), 0, None)
    }
}

fn fingerprint(manifest: &Manifest) -> Result<[u8; 32], Refusal> {
    let bytes = serde_json::to_vec(manifest).map_err(|_| Refusal::Changed)?;
    Ok(Sha256::digest(bytes).into())
}

fn can_restore(phase: Phase) -> bool {
    matches!(
        phase,
        Phase::Verified | Phase::Partial | Phase::Mutating | Phase::Restoring
    )
}

fn can_discard(phase: Phase) -> bool {
    matches!(
        phase,
        Phase::Ready | Phase::Verified | Phase::Partial | Phase::Restored | Phase::Discarding
    )
}

fn restore_preflight(
    dir: &File,
    index: usize,
    target: &Target,
    manifest: &Manifest,
) -> io::Result<bool> {
    let parent = manifest.file_parent(dir, target)?;
    let hold = holding(index);
    if !files::absent(dir, OsStr::new(&hold))? {
        let file = files::read_at(dir, OsStr::new(&hold))?;
        if !target.matches(&file, true)? {
            return Err(io::ErrorKind::InvalidData.into());
        }
        target.verify_data(file)?;
    }
    target.verify_data(files::read_at(dir, OsStr::new(&object(index)))?)?;
    if files::absent(&parent, target.name())? {
        if manifest.restored.contains_key(&index) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        return Ok(true);
    }
    let file = files::read_at(&parent, target.name())?;
    if !target.matches(&file, true)?
        && Some(&files::file_id(&file.metadata()?)) != manifest.restored_identity(index)
    {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    target.verify_data(file)?;
    Ok(false)
}

fn restore_targets(
    dir: &File,
    manifest: &mut Manifest,
    mut closed: impl FnMut(&SessionIdentity) -> Result<(), Refusal>,
) -> io::Result<()> {
    sync_manifest(dir, manifest)?;
    for (index, target) in manifest.empty_directories.iter().enumerate().rev() {
        closed(&manifest.session).map_err(|_| io::ErrorKind::WouldBlock)?;
        directories::restore(dir, target, index)?;
    }
    for index in 0..manifest.targets.len() {
        let target = manifest.targets[index].clone();
        if !restore_preflight(dir, index, &target, manifest)? {
            if let Some((_, identity)) = manifest
                .pending_restore
                .filter(|(pending, _)| *pending == index)
            {
                manifest.restored.insert(index, identity);
                manifest.pending_restore = None;
                sync_manifest(dir, manifest)?;
            }
            if manifest.restored.contains_key(&index) {
                let hold = holding(index);
                if !files::absent(dir, OsStr::new(&hold))? {
                    fs_at::unlinkat(dir, hold.as_str(), AtFlags::empty())?;
                    dir.sync_all()?;
                }
            }
            continue;
        }
        let parent = target.parent()?;
        let staging = restoring(index);
        if !files::absent(dir, OsStr::new(&staging))? {
            files::read_at(dir, OsStr::new(&staging))?;
            fs_at::unlinkat(dir, staging.as_str(), AtFlags::empty())?;
        }
        let mut source = files::read_at(dir, OsStr::new(&object(index)))?;
        let copy = transfer_copy(&mut source, &target, dir, &staging)?;
        target.restore_mode(&copy)?;
        copy.sync_all()?;
        let identity = files::file_id(&copy.metadata()?);
        manifest.pending_restore = Some((index, identity));
        sync_manifest(dir, manifest)?;
        closed(&manifest.session).map_err(|_| io::ErrorKind::WouldBlock)?;
        fs_at::renameat_with(
            dir,
            staging.as_str(),
            &parent,
            target.name(),
            RenameFlags::NOREPLACE,
        )?;
        parent.sync_all()?;
        manifest.restored.insert(index, identity);
        manifest.pending_restore = None;
        let hold = holding(index);
        if !files::absent(dir, OsStr::new(&hold))? {
            fs_at::unlinkat(dir, hold.as_str(), AtFlags::empty())?;
        }
        sync_manifest(dir, manifest)?;
    }
    for (index, target) in manifest.targets.iter().enumerate() {
        if restore_preflight(dir, index, target, manifest)? {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    manifest.phase = Phase::Restored;
    sync_manifest(dir, manifest)
}

fn discard_backup(parent: &File, dir: &File, manifest: &mut Manifest) -> io::Result<()> {
    manifest.phase = Phase::Discarding;
    sync_manifest(dir, manifest)?;
    directories::discard(dir, &manifest.empty_directories)?;
    for index in 0..manifest.targets.len() {
        let name = object(index);
        if !files::absent(dir, OsStr::new(&name))? {
            files::read_at(dir, OsStr::new(&name))?;
            fs_at::unlinkat(dir, name.as_str(), AtFlags::empty())?;
        }
    }
    // Remove the manifest only once all expected files and no surprises remain.
    for entry in fs_at::Dir::read_from(dir)? {
        if !matches!(
            entry?.file_name().to_bytes(),
            b"." | b".." | b"manifest.json"
        ) {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    fs_at::unlinkat(dir, MANIFEST, AtFlags::empty())?;
    dir.sync_all()?;
    fs_at::unlinkat(parent, manifest.id.as_str(), AtFlags::REMOVEDIR)?;
    parent.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn fixture() -> (tempfile::TempDir, File, Manifest, Vec<PathBuf>) {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().canonicalize().unwrap();
        let backup = root.join("backup");
        fs::create_dir(&backup).unwrap();
        let backup = files::directory(&backup).unwrap();
        let paths = vec![root.join("main.jsonl"), root.join("agent.jsonl")];
        let targets = paths
            .iter()
            .map(|path| {
                fs::write(path, b"sample Claude archive\n").unwrap();
                Target::inspect(&root, path, "transcript", None, 1000)
                    .unwrap()
                    .0
            })
            .collect();
        let manifest = Manifest {
            version: 1,
            id: "e".repeat(64),
            phase: Phase::Copying,
            mode: Mode::Standard,
            session: SessionIdentity {
                provider: agentmender_domain::ProviderKind::Claude,
                state_root: root,
                namespace: Some("fixture".into()),
                native_id: "eb000000-1111-4111-8111-000000000001".into(),
            },
            targets,
            empty_directories: Vec::new(),
            restored: BTreeMap::new(),
            pending_restore: None,
        };
        (fixture, backup, manifest, paths)
    }

    #[test]
    fn backup_is_required_and_writer_checks_gate_individual_moves() {
        let (_fixture, backup, mut manifest, paths) = fixture();
        assert_eq!(
            files::check_space(&manifest.session.state_root, &manifest.targets, u64::MAX),
            Err(Refusal::InsufficientBackupSpace)
        );
        write_backup(&backup, &mut manifest).unwrap();
        manifest.phase = Phase::Mutating;
        sync_manifest(&backup, &manifest).unwrap();
        assert_eq!(
            move_targets(&backup, &manifest, |_| Err(Refusal::ActiveWriter)),
            Err((0, Refusal::ActiveWriter))
        );
        assert!(paths.iter().all(|path| path.exists()));

        let mut checks = 0;
        let result = move_targets(&backup, &manifest, |_| {
            checks += 1;
            if checks == 1 {
                Ok(())
            } else {
                Err(Refusal::ActiveWriter)
            }
        });
        assert_eq!(
            result,
            Err((manifest.targets[0].bytes, Refusal::ActiveWriter))
        );
        assert!(!paths[0].exists() && paths[1].exists());
        // Reload the durable phase and recover just the missing file.
        let mut recorded: Manifest =
            serde_json::from_reader(files::read_at(&backup, OsStr::new(MANIFEST)).unwrap())
                .unwrap();
        assert_eq!(recorded.phase, Phase::Mutating);
        recorded.phase = Phase::Restoring;
        restore_targets(&backup, &mut recorded, |_| Ok(())).unwrap();
        assert_eq!(recorded.phase, Phase::Restored);
        assert!(
            paths
                .iter()
                .all(|path| fs::read(path).unwrap() == b"sample Claude archive\n")
        );
    }

    #[test]
    fn interrupted_publish_resumes_using_the_saved_destination_identity() {
        let (_fixture, backup, mut manifest, paths) = fixture();
        write_backup(&backup, &mut manifest).unwrap();
        move_targets(&backup, &manifest, |_| Ok(())).unwrap();
        manifest.phase = Phase::Restoring;
        let name = restoring(0);
        let mut source = files::read_at(&backup, OsStr::new(&object(0))).unwrap();
        let copy = transfer_copy(&mut source, &manifest.targets[0], &backup, &name).unwrap();
        let identity = files::file_id(&copy.metadata().unwrap());
        manifest.pending_restore = Some((0, identity));
        sync_manifest(&backup, &manifest).unwrap();
        let parent = manifest.targets[0].parent().unwrap();
        fs_at::renameat_with(
            &backup,
            name.as_str(),
            &parent,
            manifest.targets[0].name(),
            RenameFlags::NOREPLACE,
        )
        .unwrap();
        let mut recorded: Manifest =
            serde_json::from_reader(files::read_at(&backup, OsStr::new(MANIFEST)).unwrap())
                .unwrap();
        restore_targets(&backup, &mut recorded, |_| Ok(())).unwrap();
        assert_eq!(recorded.phase, Phase::Restored);
        assert!(
            paths
                .iter()
                .all(|path| fs::read(path).unwrap() == b"sample Claude archive\n")
        );
        assert_eq!(files::file_id(&fs::metadata(&paths[0]).unwrap()), identity);
    }
}
