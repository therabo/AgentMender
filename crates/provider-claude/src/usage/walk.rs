//! Bounded metadata walk. File contents and link targets are outside this API.
use crate::Access;
use std::{
    path::Path,
    time::{Duration, Instant},
};

const MAX_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    Directory,
    Link,
    Special,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Metadata {
    pub(super) kind: Kind,
    pub(super) logical: u64,
    pub(super) allocated: Option<u64>,
    pub(super) identity: Option<String>,
    pub(super) hardlinked: bool,
    pub(super) modified: Option<i64>,
    pub(super) revision: Option<(i64, i128)>,
    volume: Option<String>,
}

pub(super) struct Budget {
    pub(super) remaining: usize,
    pub(super) deadline: Instant,
}

impl Budget {
    pub(super) fn new(entries: usize, time: Duration) -> Self {
        Self {
            remaining: entries,
            deadline: Instant::now() + time,
        }
    }

    fn take(&mut self) -> bool {
        if self.remaining == 0 || Instant::now() >= self.deadline {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

pub(super) enum Event {
    Entry(Metadata),
    Gap,
}

struct Walk<'a> {
    budget: &'a mut Budget,
    descend: &'a dyn Fn(&Path) -> bool,
    visit: &'a mut dyn FnMut(&Path, Event),
    volume: Option<String>,
}

pub(super) fn metadata_tree(
    source: &Path,
    start: &Path,
    budget: &mut Budget,
    descend: &dyn Fn(&Path) -> bool,
    visit: &mut dyn FnMut(&Path, Event),
) -> Access {
    if !crate::local_path(source) || !crate::local_path(start) || !start.starts_with(source) {
        return Access::Unavailable;
    }
    let mut walk = Walk {
        budget,
        descend,
        visit,
        volume: None,
    };
    walk.run(source, start)
}

impl Walk<'_> {
    fn entry(&mut self, relative: &Path, info: &Metadata, depth: usize) -> bool {
        if depth == 0 && self.volume.is_none() {
            self.volume.clone_from(&info.volume);
        }
        if info.volume.is_some() && self.volume != info.volume {
            (self.visit)(relative, Event::Gap);
            return false;
        }
        (self.visit)(relative, Event::Entry(info.clone()));
        match info.kind {
            Kind::File | Kind::Link => false,
            Kind::Special => {
                (self.visit)(relative, Event::Gap);
                false
            }
            Kind::Directory if depth >= MAX_DEPTH || !(self.descend)(relative) => {
                (self.visit)(relative, Event::Gap);
                false
            }
            Kind::Directory => true,
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use rustix::{
        fd::{AsFd, OwnedFd},
        fs::{AtFlags, Dir, FileType, Mode, OFlags, Stat, fstat, open, openat, statat},
        io::Errno,
    };
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Component};

    const OPTIONS: OFlags = OFlags::RDONLY
        .union(OFlags::DIRECTORY)
        .union(OFlags::NOFOLLOW)
        .union(OFlags::CLOEXEC);

    fn stat(info: &Stat) -> Metadata {
        let kind = match FileType::from_raw_mode(info.st_mode) {
            FileType::RegularFile => Kind::File,
            FileType::Directory => Kind::Directory,
            FileType::Symlink => Kind::Link,
            _ => Kind::Special,
        };
        Metadata {
            kind,
            logical: u64::try_from(info.st_size).unwrap_or_default(),
            allocated: Some(
                u64::try_from(info.st_blocks)
                    .unwrap_or_default()
                    .saturating_mul(512),
            ),
            identity: Some(format!("{}:{}", info.st_dev, info.st_ino)),
            hardlinked: kind == Kind::File && info.st_nlink > 1,
            modified: Some(info.st_mtime),
            revision: Some((info.st_ctime, i128::from(info.st_ctime_nsec))),
            volume: Some(info.st_dev.to_string()),
        }
    }

    fn open_directory(path: &Path) -> Result<OwnedFd, Errno> {
        let mut handle = open("/", OPTIONS, Mode::empty())?;
        for component in path.components() {
            if let Component::Normal(name) = component {
                handle = openat(&handle, name, OPTIONS, Mode::empty())?;
            }
        }
        Ok(handle)
    }

    impl Walk<'_> {
        pub(super) fn run(&mut self, source: &Path, start: &Path) -> Access {
            if source != start {
                let handle = match open_directory(source) {
                    Ok(handle) => handle,
                    Err(error) => return access(error),
                };
                let Ok(info) = fstat(&handle) else {
                    return Access::Unavailable;
                };
                self.volume = Some(info.st_dev.to_string());
            }
            let (Some(parent), Some(name)) = (start.parent(), start.file_name()) else {
                return Access::Unavailable;
            };
            let handle = match open_directory(parent) {
                Ok(handle) => handle,
                Err(error) => return access(error),
            };
            self.scan_at(&handle, name, Path::new(""), 0)
        }

        fn scan_at<Fd: AsFd>(
            &mut self,
            directory: Fd,
            name: &OsStr,
            relative: &Path,
            depth: usize,
        ) -> Access {
            if !self.budget.take() {
                (self.visit)(relative, Event::Gap);
                return Access::Unavailable;
            }
            let initial = match statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(info) => stat(&info),
                Err(error) => return access(error),
            };
            if !self.entry(relative, &initial, depth) {
                return Access::Available;
            }
            let inspect = || {
                let fd = openat(&directory, name, OPTIONS, Mode::empty())?;
                if stat(&fstat(&fd)?) != initial {
                    return Err(Errno::STALE);
                }
                Dir::new(fd)
            };
            let Ok(mut entries) = inspect() else {
                (self.visit)(relative, Event::Gap);
                return Access::Unavailable;
            };
            while let Some(item) = entries.read() {
                let Ok(item) = item else {
                    (self.visit)(relative, Event::Gap);
                    break;
                };
                let bytes = item.file_name().to_bytes();
                if matches!(bytes, b"." | b"..") {
                    continue;
                }
                if self.budget.remaining == 0 || Instant::now() >= self.budget.deadline {
                    (self.visit)(relative, Event::Gap);
                    break;
                }
                let child = OsStr::from_bytes(bytes);
                let child_path = relative.join(child);
                if entries.fd().ok().is_none_or(|fd| {
                    self.scan_at(fd, child, &child_path, depth + 1) != Access::Available
                }) {
                    (self.visit)(&child_path, Event::Gap);
                }
            }
            let same = statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW)
                .is_ok_and(|current| stat(&current) == initial);
            if !same {
                (self.visit)(relative, Event::Gap);
            }
            Access::Available
        }
    }

    fn access(error: Errno) -> Access {
        if error == Errno::NOENT {
            Access::Missing
        } else {
            Access::Unavailable
        }
    }
}

#[cfg(not(unix))]
mod portable {
    use super::*;
    use agentmender_provider_api::filesystem;
    use std::{fs, time::UNIX_EPOCH};

    fn metadata(path: &Path) -> std::io::Result<Metadata> {
        let meta = fs::symlink_metadata(path)?;
        let kind = if filesystem::is_link(&meta) {
            Kind::Link
        } else if meta.is_file() {
            Kind::File
        } else if meta.is_dir() {
            Kind::Directory
        } else {
            Kind::Special
        };
        #[cfg(windows)]
        let physical = if kind == Kind::Directory {
            Some(filesystem::windows_directory_identity(path)?)
        } else {
            None
        };
        #[cfg(not(windows))]
        let physical: Option<(u64, u64)> = None;
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok());
        Ok(Metadata {
            kind,
            logical: meta.len(),
            allocated: None,
            identity: physical.map(|(volume, id)| format!("{volume}:{id}")),
            hardlinked: false,
            modified: modified.and_then(|time| i64::try_from(time.as_secs()).ok()),
            revision: modified.and_then(|time| {
                Some((
                    i64::try_from(time.as_secs()).ok()?,
                    i128::from(time.subsec_nanos()),
                ))
            }),
            volume: physical.map(|(volume, _)| volume.to_string()),
        })
    }

    impl Walk<'_> {
        pub(super) fn run(&mut self, source: &Path, start: &Path) -> Access {
            if source != start {
                let info = match metadata(source) {
                    Ok(info) => info,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        return Access::Missing;
                    }
                    Err(_) => return Access::Unavailable,
                };
                if info.kind != Kind::Directory || !crate::safe_path(source) {
                    return Access::Unavailable;
                }
                self.volume = info.volume;
            }
            self.scan(start, Path::new(""), 0)
        }

        fn scan(&mut self, path: &Path, relative: &Path, depth: usize) -> Access {
            if !self.budget.take() {
                (self.visit)(relative, Event::Gap);
                return Access::Unavailable;
            }
            if !path.parent().is_some_and(crate::safe_path) {
                return if fs::symlink_metadata(path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
                {
                    Access::Missing
                } else {
                    Access::Unavailable
                };
            }
            let initial = match metadata(path) {
                Ok(info) => info,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Access::Missing;
                }
                Err(_) => return Access::Unavailable,
            };
            if !self.entry(relative, &initial, depth) {
                return Access::Available;
            }
            let Ok(entries) = fs::read_dir(path) else {
                (self.visit)(relative, Event::Gap);
                return Access::Unavailable;
            };
            for item in entries {
                let Ok(item) = item else {
                    (self.visit)(relative, Event::Gap);
                    continue;
                };
                if self.budget.remaining == 0 || Instant::now() >= self.budget.deadline {
                    (self.visit)(relative, Event::Gap);
                    break;
                }
                let child = relative.join(item.file_name());
                if self.scan(&item.path(), &child, depth + 1) != Access::Available {
                    (self.visit)(&child, Event::Gap);
                }
            }
            if !metadata(path).is_ok_and(|current| current == initial) || !crate::safe_path(path) {
                (self.visit)(relative, Event::Gap);
            }
            Access::Available
        }
    }
}
