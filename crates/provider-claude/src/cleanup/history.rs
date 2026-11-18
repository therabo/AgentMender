//! Rewrite a local prompt history under Claude's canonical directory lock.
use super::directory;
use crate::Revision;
use rustix::fs::{self as fs, AtFlags, FileType, Mode, OFlags};
use std::{
    fs::File,
    io::{self, BufWriter, Read, Write},
    path::Path,
    time::{Duration, Instant},
};

const HISTORY: &str = "history.jsonl";
const LOCK: &str = "history.jsonl.lock";
const MAX_BYTES: u64 = 32 * 1024 * 1024;

fn open_history(root: &File) -> io::Result<File> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = fs::openat(root, HISTORY, flags, Mode::empty())?;
    let stat = fs::fstat(&file)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || stat.st_nlink != 1 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(File::from(file))
}

struct HistoryLock<'a> {
    root: &'a File,
    identity: (i32, u64),
}

impl HistoryLock<'_> {
    fn held(&self) -> bool {
        fs::statat(self.root, LOCK, AtFlags::SYMLINK_NOFOLLOW).is_ok_and(|stat| {
            (stat.st_dev, stat.st_ino) == self.identity
                && FileType::from_raw_mode(stat.st_mode) == FileType::Directory
        })
    }
}

impl Drop for HistoryLock<'_> {
    fn drop(&mut self) {
        if self.held() {
            let _ = fs::unlinkat(self.root, LOCK, AtFlags::REMOVEDIR);
        }
    }
}

pub(super) fn remove_rows(path: &Path, id: &str) -> io::Result<usize> {
    let root = directory(path)?;
    if matches!(
        fs::statat(&root, HISTORY, AtFlags::SYMLINK_NOFOLLOW),
        Err(rustix::io::Errno::NOENT)
    ) {
        return Ok(0);
    }
    let started = Instant::now();
    fs::mkdirat(&root, LOCK, Mode::RUSR | Mode::WUSR | Mode::XUSR).map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            io::Error::other("Cronologia occupata. Le righe sono state conservate.")
        } else {
            error.into()
        }
    })?;
    let observed = fs::statat(&root, LOCK, AtFlags::SYMLINK_NOFOLLOW)?;
    let lock = HistoryLock {
        root: &root,
        identity: (observed.st_dev, observed.st_ino),
    };
    let file = open_history(&root)?;
    let location = path.join(HISTORY);
    let revision = Revision::read(&file, &location).map_err(|_| io::ErrorKind::InvalidData)?;
    let mode = fs::fstat(&file)?.st_mode & 0o777;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::other(
            "Cronologia oltre il limite della riscrittura.",
        ));
    }

    let temp = format!("{HISTORY}.agentmender-{}.tmp", std::process::id());
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let output = File::from(fs::openat(
        &root,
        temp.as_str(),
        flags,
        Mode::RUSR | Mode::WUSR,
    )?);
    let result = (|| {
        let mut writer = BufWriter::new(&output);
        let mut count = 0;
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            // Finish within proper-lockfile's stale interval without stealing
            // an existing lock or keeping a lease across the file deletions.
            if started.elapsed() >= Duration::from_secs(5) {
                return Err(io::Error::other(
                    "Tempo di riscrittura della cronologia esaurito.",
                ));
            }
            if matches_id(line, id) {
                count += 1;
            } else {
                writer.write_all(line)?;
            }
        }
        if count == 0 {
            return Ok(0);
        }
        writer.flush()?;
        fs::fchmod(&output, Mode::from_bits_truncate(mode))?;
        if !lock.held()
            || Revision::read(&open_history(&root)?, &location)
                .map_err(|_| io::ErrorKind::InvalidData)?
                != revision
        {
            return Err(io::Error::other(
                "Cronologia o lock cambiati durante la riscrittura.",
            ));
        }
        fs::renameat(&root, temp.as_str(), &root, HISTORY)?;
        Ok(count)
    })();
    if !matches!(result, Ok(count) if count > 0) {
        let _ = fs::unlinkat(&root, temp.as_str(), AtFlags::empty());
    }
    result
}

fn matches_id(line: &[u8], id: &str) -> bool {
    let Ok(row) = serde_json::from_slice::<serde_json::Value>(line) else {
        return false;
    };
    row.get("sessionId")
        .or_else(|| row.get("session_id"))
        .and_then(serde_json::Value::as_str)
        == Some(id)
}
