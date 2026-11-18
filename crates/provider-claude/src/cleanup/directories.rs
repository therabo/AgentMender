//! Park verified empty directories so recovery can reuse their original identities.
use super::files::{self, DirectoryTarget};
use rustix::fs::{self as fs_at, AtFlags, RenameFlags};
use std::{ffi::OsStr, fs::File, io, path::Path};

fn held(index: usize) -> String {
    format!("directory-{index}")
}

pub(super) fn park(backup: &File, target: &DirectoryTarget, index: usize) -> io::Result<()> {
    let parent = target.parent()?;
    files::empty(&target.checked(&parent, target.name())?)?;
    let name = held(index);
    fs_at::renameat_with(
        &parent,
        target.name(),
        backup,
        name.as_str(),
        RenameFlags::NOREPLACE,
    )?;
    let check = target
        .checked(backup, OsStr::new(&name))
        .and_then(|dir| files::empty(&dir));
    if let Err(error) = check {
        let _ = fs_at::renameat_with(
            backup,
            name.as_str(),
            &parent,
            target.name(),
            RenameFlags::NOREPLACE,
        );
        let _ = parent.sync_all();
        let _ = backup.sync_all();
        return Err(error);
    }
    parent.sync_all()?;
    backup.sync_all()
}

pub(super) fn parent(
    backup: &File,
    all: &[DirectoryTarget],
    path: &Path,
    live: io::Result<File>,
) -> io::Result<File> {
    match live {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let (index, target) = all
                .iter()
                .enumerate()
                .find(|(_, target)| Some(target.path().as_path()) == path.parent())
                .ok_or(error)?;
            let dir = target.checked(backup, OsStr::new(&held(index)))?;
            files::empty(&dir)?;
            Ok(dir)
        }
        result => result,
    }
}

pub(super) fn verify(
    backup: &File,
    all: &[DirectoryTarget],
    must_be_parked: bool,
) -> io::Result<()> {
    for (index, target) in all.iter().enumerate() {
        let parent = parent(backup, all, &target.path(), target.parent())?;
        // Even with a parked parent, the exact input directory identity applies.
        if files::file_id(&parent.metadata()?) != target.parent_id {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let missing = files::absent(&parent, target.name())?;
        let name = held(index);
        let parked = !files::absent(backup, OsStr::new(&name))?;
        if missing != parked || (must_be_parked && !parked) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if parked {
            files::empty(&target.checked(backup, OsStr::new(&name))?)?;
        } else {
            target.checked(&parent, target.name())?;
        }
    }
    Ok(())
}

pub(super) fn restore(backup: &File, target: &DirectoryTarget, index: usize) -> io::Result<()> {
    let parent = target.parent()?;
    let name = held(index);
    if files::absent(backup, OsStr::new(&name))? {
        target.checked(&parent, target.name())?;
        return Ok(());
    }
    files::empty(&target.checked(backup, OsStr::new(&name))?)?;
    fs_at::renameat_with(
        backup,
        name.as_str(),
        &parent,
        target.name(),
        RenameFlags::NOREPLACE,
    )?;
    target.checked(&parent, target.name())?;
    parent.sync_all()?;
    backup.sync_all()
}

pub(super) fn discard(backup: &File, all: &[DirectoryTarget]) -> io::Result<()> {
    for (index, target) in all.iter().enumerate() {
        let name = held(index);
        if !files::absent(backup, OsStr::new(&name))? {
            files::empty(&target.checked(backup, OsStr::new(&name))?)?;
            fs_at::unlinkat(backup, name.as_str(), AtFlags::REMOVEDIR)?;
        }
    }
    backup.sync_all()
}
