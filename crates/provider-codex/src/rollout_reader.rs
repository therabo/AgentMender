//! Bounded, read-only decoding. No native materialization or provider writes.
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    time::Instant,
};

const COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const DECODED_WORK: u64 = 128 * 1024 * 1024;
type Decoder = zstd::stream::read::Decoder<'static, BufReader<io::Take<File>>>;

pub(super) struct RolloutReader {
    file: File,
    decoder: Option<Decoder>,
    position: u64,
    work: u64,
    end: Instant,
}

fn failure(code: &'static str) -> io::Error {
    io::Error::other(code)
}

impl RolloutReader {
    pub fn new(file: &File, compressed: bool, end: Instant) -> io::Result<Self> {
        if compressed && file.metadata()?.len() > COMPRESSED_BYTES {
            return Err(failure("compressed_source_limit"));
        }
        let mut reader = Self {
            file: file.try_clone()?,
            decoder: None,
            position: 0,
            work: 0,
            end,
        };
        if compressed {
            reader.reset()?;
        }
        Ok(reader)
    }

    fn reset(&mut self) -> io::Result<()> {
        self.decoder = None;
        self.file.seek(SeekFrom::Start(0))?;
        // Also cap physical reads if a live writer grows the file after open.
        // The final source identity check independently refuses that change.
        let input = self.file.try_clone()?.take(COMPRESSED_BYTES);
        let mut decoder = zstd::stream::read::Decoder::new(input)?;
        // Bound memory even when the input advertises a maliciously large window.
        decoder.window_log_max(23)?;
        self.decoder = Some(decoder);
        self.position = 0;
        Ok(())
    }
}

impl Read for RolloutReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if Instant::now() >= self.end {
            return Err(failure("compressed_work_limit"));
        }
        let count = if let Some(decoder) = &mut self.decoder {
            let remaining = DECODED_WORK.saturating_sub(self.work);
            if remaining == 0 {
                return Err(failure("compressed_work_limit"));
            }
            let limit = buffer.len().min(8192).min(remaining as usize);
            decoder
                .read(&mut buffer[..limit])
                .map_err(|_| failure("compressed_source_invalid"))?
        } else {
            self.file.read(buffer)?
        };
        self.position += count as u64;
        self.work += count as u64;
        if Instant::now() >= self.end {
            return Err(failure("compressed_work_limit"));
        }
        Ok(count)
    }
}

impl Seek for RolloutReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        if self.decoder.is_none() {
            self.position = self.file.seek(from)?;
            return Ok(self.position);
        }
        let SeekFrom::Start(target) = from else {
            return Err(failure("compressed_seek_unsupported"));
        };
        if target > DECODED_WORK {
            return Err(failure("compressed_work_limit"));
        }
        if target < self.position {
            self.reset()?;
        }
        let mut buffer = [0; 8192];
        while self.position < target {
            let count = buffer.len().min((target - self.position) as usize);
            if self.read(&mut buffer[..count])? == 0 {
                return Err(failure("compressed_seek_past_end"));
            }
        }
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn bounded_stream_replays_decoded_offsets_and_refuses_expansion() {
        let source = tempfile::tempfile().unwrap();
        let data = b"first\nsecond\nthird\n";
        zstd::stream::copy_encode(data.as_slice(), &source, 1).unwrap();
        let mut reader =
            RolloutReader::new(&source, true, Instant::now() + Duration::from_secs(10)).unwrap();
        reader.seek(SeekFrom::Start(6)).unwrap();
        let mut text = [0; 6];
        reader.read_exact(&mut text).unwrap();
        assert_eq!(&text, b"second");
        reader.seek(SeekFrom::Start(0)).unwrap();
        reader.read_exact(&mut text).unwrap();
        assert_eq!(&text, b"first\n");
        assert!(reader.seek(SeekFrom::Start(100)).is_err());
        assert!(reader.seek(SeekFrom::Start(DECODED_WORK + 1)).is_err());
        reader.seek(SeekFrom::Start(0)).unwrap();
        reader.work = DECODED_WORK - 3;
        assert_eq!(reader.read(&mut text).unwrap(), 3);
        assert!(reader.read(&mut text).is_err());
    }

    #[test]
    fn compressed_expansion_window_file_size_and_deadline_are_bounded() {
        let source = tempfile::tempfile().unwrap();
        let mut encoder = zstd::stream::write::Encoder::new(&source, 1).unwrap();
        encoder.window_log(20).unwrap();
        io::copy(&mut io::repeat(0).take(DECODED_WORK + 1), &mut encoder).unwrap();
        encoder.finish().unwrap();
        let mut reader =
            RolloutReader::new(&source, true, Instant::now() + Duration::from_secs(10)).unwrap();
        assert_eq!(
            io::copy(&mut reader, &mut io::sink())
                .unwrap_err()
                .to_string(),
            "compressed_work_limit"
        );
        assert_eq!(reader.work, DECODED_WORK);
        source.set_len(COMPRESSED_BYTES + 1).unwrap();
        assert!(
            RolloutReader::new(&source, true, Instant::now() + Duration::from_secs(10)).is_err()
        );

        let source = tempfile::tempfile().unwrap();
        let mut encoder = zstd::stream::write::Encoder::new(&source, 1).unwrap();
        encoder.window_log(24).unwrap();
        io::copy(&mut io::repeat(0).take(16 * 1024 * 1024), &mut encoder).unwrap();
        encoder.finish().unwrap();
        let mut reader =
            RolloutReader::new(&source, true, Instant::now() + Duration::from_secs(10)).unwrap();
        assert!(reader.read(&mut [0; 1]).is_err());
        reader.end = Instant::now();
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap_err().to_string(),
            "compressed_work_limit"
        );
    }
}
