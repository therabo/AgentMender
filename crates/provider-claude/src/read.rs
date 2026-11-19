//! Revision-bound, bounded reads of recognized Claude storage.
use agentmender_provider_api::{ProviderError, filesystem, io_error};
use std::{
    fs::{File, Metadata},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Take},
    path::Path,
    time::SystemTime,
};

pub(crate) const MAX_RECORD: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub(crate) bytes: u64,
    pub(crate) modified: Option<SystemTime>,
    created: Option<SystemTime>,
    #[cfg(unix)]
    physical: (u64, u64, i64, i64),
    #[cfg(windows)]
    physical: (u64, u64),
    #[cfg(windows)]
    edges: [u8; 32],
}

impl Revision {
    fn from_metadata(meta: Metadata, _file: &File) -> io::Result<Self> {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            bytes: meta.len(),
            modified: meta.modified().ok(),
            created: meta.created().ok(),
            #[cfg(unix)]
            physical: (meta.dev(), meta.ino(), meta.ctime(), meta.ctime_nsec()),
            #[cfg(windows)]
            physical: filesystem::windows_file_identity(_file)?,
            #[cfg(windows)]
            edges: {
                use sha2::{Digest, Sha256};
                use std::os::windows::fs::FileExt;
                let mut hash = Sha256::new();
                let mut edge = [0; 4096];
                for offset in [0, meta.len().saturating_sub(edge.len() as u64)] {
                    let size = _file.seek_read(&mut edge, offset)?;
                    hash.update(&edge[..size]);
                }
                hash.finalize().into()
            },
        })
    }

    pub(crate) fn filesystem_identity(&self) -> Option<String> {
        #[cfg(any(unix, windows))]
        return Some(format!("{}:{}", self.physical.0, self.physical.1));
        #[cfg(not(any(unix, windows)))]
        None
    }

    pub(crate) fn read(file: &File, path: &Path) -> Result<Self, ProviderError> {
        file.metadata()
            .and_then(|meta| Self::from_metadata(meta, file))
            .map_err(|e| io_error(path, e))
    }
}

pub(crate) fn verify(root: &Path, path: &Path, revision: &Revision) -> Result<(), ProviderError> {
    let current = filesystem::open_observed_read(root, path)?;
    if Revision::read(&current, path)? != *revision {
        return Err(ProviderError::InvalidScope("Claude source changed".into()));
    }
    Ok(())
}

pub(crate) struct Line<'a> {
    pub(crate) end: u64,
    pub(crate) terminated: bool,
    pub(crate) bytes: Option<&'a [u8]>,
}

pub(crate) struct Lines<'a> {
    reader: BufReader<Take<&'a mut File>>,
    line: Vec<u8>,
    pub(crate) offset: u64,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(file: &'a mut File, start: u64, end: u64) -> io::Result<Self> {
        file.seek(SeekFrom::Start(start))?;
        Ok(Self {
            reader: BufReader::with_capacity(32 * 1024, file.take(end - start)),
            line: Vec::new(),
            offset: start,
        })
    }

    /// A clipped, oversized or unterminated record advances the raw offset
    /// without yielding text. A reader never allocates for the skipped bytes.
    pub(crate) fn next(&mut self) -> io::Result<Option<Line<'_>>> {
        self.line.clear();
        let mut oversized = false;
        let mut advanced = false;
        loop {
            let buf = self.reader.fill_buf()?;
            if buf.is_empty() {
                return Ok(advanced.then_some(Line {
                    end: self.offset,
                    terminated: false,
                    bytes: None,
                }));
            }
            let newline = buf.iter().position(|byte| *byte == b'\n');
            let consumed = newline.map_or(buf.len(), |index| index + 1);
            if self.line.len() + consumed > MAX_RECORD {
                oversized = true;
            }
            if !oversized {
                self.line.extend_from_slice(&buf[..consumed]);
            }
            self.reader.consume(consumed);
            self.offset += consumed as u64;
            advanced = true;
            if newline.is_some() {
                return Ok(Some(Line {
                    end: self.offset,
                    terminated: true,
                    bytes: (!oversized).then_some(self.line.as_slice()),
                }));
            }
        }
    }
}
