//! Rewrite local prompt history under Claude's canonical directory lock.
use agentmender_provider_api::rewrite::{self, DIR_FLAGS, directory};
use rustix::fs::{self, AtFlags, Mode, Timespec, Timestamps, UTIME_NOW, UTIME_OMIT};
use std::{
    fs::File,
    io,
    path::Path,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

const HISTORY: &str = "history.jsonl";
const LOCK: &str = "history.jsonl.lock";

struct HistoryLock<'a> {
    root: &'a File,
    handle: File,
    identity: (i32, u64),
}

impl HistoryLock<'_> {
    fn held(&self) -> bool {
        fs::statat(self.root, LOCK, AtFlags::SYMLINK_NOFOLLOW)
            .is_ok_and(|stat| (stat.st_dev, stat.st_ino) == self.identity)
    }

    fn refresh(&self) -> io::Result<()> {
        if !self.held() {
            return Err(io::Error::other("Lock della cronologia cambiato."));
        }
        fs::futimens(
            &self.handle,
            &Timestamps {
                last_access: Timespec {
                    tv_sec: 0,
                    tv_nsec: UTIME_OMIT,
                },
                last_modification: Timespec {
                    tv_sec: 0,
                    tv_nsec: UTIME_NOW,
                },
            },
        )?;
        Ok(())
    }
}

impl Drop for HistoryLock<'_> {
    fn drop(&mut self) {
        if self.held() {
            let _ = fs::unlinkat(self.root, LOCK, AtFlags::REMOVEDIR);
        }
    }
}

pub(super) fn count_rows(path: &Path, id: &str) -> io::Result<usize> {
    let file = rewrite::open_file(&directory(path)?, HISTORY)?;
    rewrite::filter_jsonl(&file, &mut io::sink(), |line| matches_id(line, id))
}

pub(super) fn remove_rows(path: &Path, id: &str) -> io::Result<usize> {
    let root = directory(path)?;
    if matches!(
        fs::statat(&root, HISTORY, AtFlags::SYMLINK_NOFOLLOW),
        Err(rustix::io::Errno::NOENT)
    ) {
        return Ok(0);
    }
    fs::mkdirat(&root, LOCK, Mode::RUSR | Mode::WUSR | Mode::XUSR).map_err(|error| {
        if error == rustix::io::Errno::EXIST {
            io::Error::other("Cronologia occupata. Le righe sono state conservate.")
        } else {
            error.into()
        }
    })?;
    let handle = fs::openat(&root, LOCK, DIR_FLAGS, Mode::empty())?;
    let observed = fs::fstat(&handle)?;
    let lock = HistoryLock {
        root: &root,
        handle: File::from(handle),
        identity: (observed.st_dev, observed.st_ino),
    };
    with_heartbeat(&lock, || {
        rewrite::replace(
            &root,
            HISTORY,
            |input, output| rewrite::filter_jsonl(input, output, |line| matches_id(line, id)),
            || lock.refresh(),
        )
    })
}

fn with_heartbeat<T>(
    lock: &HistoryLock<'_>,
    rewrite: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    thread::scope(|scope| {
        let (stop, receiver) = mpsc::channel::<()>();
        // Claude's proper-lockfile treats the directory as stale after 10 seconds.
        scope.spawn(move || {
            while let Err(RecvTimeoutError::Timeout) = receiver.recv_timeout(Duration::from_secs(2))
            {
                if lock.refresh().is_err() {
                    break;
                }
            }
        });
        let result = rewrite();
        drop(stop);
        result
    })
}

fn matches_id(line: &[u8], id: &str) -> bool {
    let Ok(row) = serde_json::from_slice::<serde_json::Value>(line) else {
        return false;
    };
    row.get("sessionId")
        .filter(|value| !value.is_null())
        .or_else(|| row.get("session_id"))
        .and_then(serde_json::Value::as_str)
        == Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn renews_the_canonical_directory_lock_while_the_rewrite_is_still_running() {
        let archive = tempfile::tempdir_in("/private/tmp").unwrap();
        let root = directory(archive.path()).unwrap();
        fs::mkdirat(&root, LOCK, Mode::RUSR | Mode::WUSR | Mode::XUSR).unwrap();
        let handle = fs::openat(&root, LOCK, DIR_FLAGS, Mode::empty()).unwrap();
        let initial = fs::fstat(&handle).unwrap();
        let lock = HistoryLock {
            root: &root,
            handle: File::from(handle),
            identity: (initial.st_dev, initial.st_ino),
        };
        with_heartbeat(&lock, || {
            let deadline = Instant::now() + Duration::from_secs(6);
            loop {
                assert!(lock.held());
                let current = fs::fstat(&lock.handle)?;
                if (current.st_mtime, current.st_mtime_nsec)
                    != (initial.st_mtime, initial.st_mtime_nsec)
                {
                    return Ok(());
                }
                assert!(
                    Instant::now() < deadline,
                    "lock heartbeat never updated mtime"
                );
                thread::sleep(Duration::from_millis(50));
            }
        })
        .unwrap();
        drop(lock);
        assert!(!archive.path().join(LOCK).exists());
    }
}
