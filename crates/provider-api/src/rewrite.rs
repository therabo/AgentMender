//! Scoped replacement of provider files after a successful rewrite.
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, Stat};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Component, Path},
    sync::atomic::{AtomicU64, Ordering},
};

pub const DIR_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

pub fn directory(path: &Path) -> io::Result<File> {
    let mut parts = path.components();
    if parts.next() != Some(Component::RootDir) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut dir = fs::open("/", DIR_FLAGS, Mode::empty())?;
    for part in parts {
        let Component::Normal(name) = part else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        dir = fs::openat(dir, name, DIR_FLAGS, Mode::empty())?;
    }
    Ok(File::from(dir))
}

pub fn open_file(root: &File, name: &str) -> io::Result<File> {
    if Path::new(name).file_name() != Some(OsStr::new(name)) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = fs::openat(root, name, flags, Mode::empty())?;
    let stat = fs::fstat(&file)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || stat.st_nlink != 1 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(File::from(file))
}

pub fn replace(
    root: &File,
    name: &str,
    rewrite: impl FnOnce(&File, &mut dyn Write) -> io::Result<usize>,
    before_commit: impl FnOnce() -> io::Result<()>,
) -> io::Result<usize> {
    let input = match open_file(root, name) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let initial = fs::fstat(&input)?;
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let temp = format!(
        "{name}.agentmender-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    );
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let output = File::from(fs::openat(
        root,
        temp.as_str(),
        flags,
        Mode::RUSR | Mode::WUSR,
    )?);
    let result = (|| {
        let mut writer = BufWriter::new(&output);
        let count = rewrite(&input, &mut writer)?;
        if count > 0 {
            writer.flush()?;
            fs::fchmod(&output, Mode::from_bits_truncate(initial.st_mode & 0o777))?;
            let current = fs::statat(root, name, AtFlags::SYMLINK_NOFOLLOW)?;
            if revision(&initial) != revision(&current) || current.st_nlink != 1 {
                return Err(io::Error::other("File cambiato durante la riscrittura."));
            }
            before_commit()?;
            fs::renameat(root, temp.as_str(), root, name)?;
        }
        Ok(count)
    })();
    if !matches!(result, Ok(count) if count > 0) {
        let _ = fs::unlinkat(root, temp.as_str(), AtFlags::empty());
    }
    result
}

fn revision(stat: &Stat) -> impl PartialEq + use<> {
    (
        stat.st_dev,
        stat.st_ino,
        stat.st_size,
        stat.st_mtime,
        stat.st_mtime_nsec,
        stat.st_ctime,
        stat.st_ctime_nsec,
    )
}

pub fn filter_jsonl(
    input: &File,
    output: &mut dyn Write,
    matches: impl Fn(&[u8]) -> bool,
) -> io::Result<usize> {
    let mut reader = BufReader::new(input);
    let mut line = Vec::new();
    let mut count = 0;
    while reader.read_until(b'\n', &mut line)? != 0 {
        if matches(&line) {
            count += 1;
        } else {
            output.write_all(&line)?;
        }
        line.clear();
    }
    Ok(count)
}
