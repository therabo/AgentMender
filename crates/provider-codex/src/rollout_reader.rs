//! ADR-0030: bounded resident decoding, never provider materialization.
use agentmender_domain::{DecoderProgress, ReadWork};
use std::{
    cell::RefCell,
    collections::VecDeque,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    time::{Duration, Instant},
};
use zstd::stream::raw::{DParameter, Decoder, Operation};

const DECODED_WORK: u64 = 128 * 1024 * 1024;
const SEEK_WORK: u64 = 96 * 1024 * 1024;
const PHYSICAL_WORK: u64 = 128 * 1024 * 1024;
// Covers a 32 MiB context scan plus an 8 MiB backward probe and read-ahead.
const RECENT_BYTES: usize = 40 * 1024 * 1024 + 16384;
const RESIDENT_READERS: usize = 2;

thread_local! {
    static READERS: RefCell<VecDeque<(String, RolloutReader)>> = const { RefCell::new(VecDeque::new()) };
    static WORK: RefCell<ReadWork> = RefCell::new(ReadWork::default());
}
#[cfg(test)]
pub(super) fn clear_resident_for_test() {
    READERS.with(|r| r.borrow_mut().clear());
}
pub(super) fn work_reset() {
    WORK.with(|w| *w.borrow_mut() = ReadWork::default());
}
pub(super) fn work() -> ReadWork {
    WORK.with(|w| w.borrow().clone())
}
pub(super) fn record_inspected() {
    WORK.with(|w| w.borrow_mut().records_inspected += 1);
}
pub(super) fn record_fts(build: Duration, query: Duration) {
    WORK.with(|work| {
        let mut work = work.borrow_mut();
        work.provider.fts_builds += 1;
        work.provider.fts_build_us += build.as_micros() as u64;
        work.provider.fts_query_us += query.as_micros() as u64;
    });
}

pub(super) fn record_metadata_row() {
    WORK.with(|w| w.borrow_mut().provider.metadata_rows += 1);
}

#[cfg(target_os = "macos")]
pub(super) fn record_hash_bytes(bytes: u64) {
    WORK.with(|w| w.borrow_mut().provider.metadata_hash_bytes += bytes);
}

#[cfg(target_os = "macos")]
pub(super) fn record_snapshot(elapsed: Option<Duration>) {
    WORK.with(|w| {
        let mut work = w.borrow_mut();
        if let Some(elapsed) = elapsed {
            work.provider.sqlite_snapshots += 1;
            work.provider.sqlite_backup_us += elapsed.as_micros() as u64;
        } else {
            work.provider.sqlite_snapshot_reuses += 1;
        }
    });
}
fn failure(code: &'static str) -> io::Error {
    io::Error::other(code)
}
pub(super) fn pending(error: &io::Error) -> bool {
    error.to_string() == "compressed_work_limit"
}

struct Compressed {
    decoder: Decoder<'static>,
    input: Vec<u8>,
    input_at: usize,
    input_end: usize,
    physical_end: u64,
    consumed: u64,
    frontier: u64,
    recent: VecDeque<u8>,
    frame_finished: bool,
    eof: bool,
}
pub(super) struct RolloutReader {
    file: File,
    compressed: Option<Compressed>,
    position: u64,
    decoded_work: u64,
    physical_work: u64,
    end: Instant,
    healthy: bool,
}

/// A bounded working-set lease. Only a validated exact source key may reuse it.
pub(super) struct ResidentReader {
    key: String,
    reader: Option<RolloutReader>,
}
impl std::ops::Deref for ResidentReader {
    type Target = RolloutReader;
    fn deref(&self) -> &Self::Target {
        self.reader.as_ref().expect("live lease")
    }
}
impl std::ops::DerefMut for ResidentReader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reader.as_mut().expect("live lease")
    }
}
impl Drop for ResidentReader {
    fn drop(&mut self) {
        if let Some(reader) = self.reader.take()
            && reader.healthy
            && reader.compressed.is_some()
        {
            READERS.with(|cache| {
                let mut cache = cache.borrow_mut();
                cache.retain(|(key, _)| key != &self.key);
                while cache.len() >= RESIDENT_READERS {
                    cache.pop_front();
                }
                cache.push_back((self.key.clone(), reader));
            });
        }
    }
}
impl Read for ResidentReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reader.as_mut().expect("live lease").read(bytes)
    }
}
impl Seek for ResidentReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.reader.as_mut().expect("live lease").seek(from)
    }
}
impl RolloutReader {
    pub fn resident(
        file: &File,
        compressed: bool,
        key: String,
        end: Instant,
        progress: Option<&DecoderProgress>,
    ) -> io::Result<ResidentReader> {
        let cached = if compressed {
            READERS.with(|cache| {
                let mut cache = cache.borrow_mut();
                cache
                    .iter()
                    .position(|(k, _)| k == &key)
                    .and_then(|i| cache.remove(i))
                    .map(|(_, r)| r)
            })
        } else {
            None
        };
        let mut reader = cached.map_or_else(|| Self::new(file, compressed, end), Ok)?;
        reader.end = end;
        reader.decoded_work = 0;
        reader.physical_work = 0;
        if progress.is_some_and(|p| {
            reader
                .progress()
                .is_none_or(|now| now.decoded < p.decoded || now.compressed < p.compressed)
        }) {
            return Err(failure("compressed_continuation_expired"));
        }
        Ok(ResidentReader {
            key,
            reader: Some(reader),
        })
    }
    pub fn new(file: &File, compressed: bool, end: Instant) -> io::Result<Self> {
        let mut file = file.try_clone()?;
        file.seek(SeekFrom::Start(0))?;
        let state = if compressed {
            let mut decoder = Decoder::new()?;
            decoder.set_parameter(DParameter::WindowLogMax(23))?;
            Some(Compressed {
                decoder,
                input: vec![0; 131072],
                input_at: 0,
                input_end: 0,
                physical_end: file.metadata()?.len(),
                consumed: 0,
                frontier: 0,
                recent: VecDeque::new(),
                frame_finished: false,
                eof: false,
            })
        } else {
            None
        };
        Ok(Self {
            file,
            compressed: state,
            position: 0,
            decoded_work: 0,
            physical_work: 0,
            end,
            healthy: true,
        })
    }
    pub fn progress(&self) -> Option<DecoderProgress> {
        self.compressed.as_ref().map(|s| DecoderProgress {
            decoded: s.frontier,
            compressed: s.consumed,
        })
    }
    fn reset(&mut self) -> io::Result<()> {
        let decoded = self.decoded_work;
        let physical = self.physical_work;
        let mut fresh = Self::new(&self.file, true, self.end)?;
        fresh.decoded_work = decoded;
        fresh.physical_work = physical;
        *self = fresh;
        Ok(())
    }
}
impl Read for RolloutReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if Instant::now() >= self.end {
            return Err(failure("compressed_deadline"));
        }
        let Some(state) = &mut self.compressed else {
            let count = self.file.read(buffer)?;
            WORK.with(|w| w.borrow_mut().decoded_bytes += count as u64);
            return Ok(count);
        };
        if self.position < state.frontier {
            let start = state.frontier.saturating_sub(state.recent.len() as u64);
            if self.position < start {
                return Err(failure("compressed_continuation_expired"));
            }
            let count = buffer.len().min((state.frontier - self.position) as usize);
            let at = (self.position - start) as usize;
            for (out, byte) in buffer[..count]
                .iter_mut()
                .zip(state.recent.range(at..at + count))
            {
                *out = *byte;
            }
            self.position += count as u64;
            return Ok(count);
        }
        if state.eof {
            return Ok(0);
        }
        loop {
            if Instant::now() >= self.end {
                return Err(failure("compressed_deadline"));
            }
            if self.decoded_work >= DECODED_WORK || self.physical_work >= PHYSICAL_WORK {
                return Err(failure("compressed_work_limit"));
            }
            if state.input_at == state.input_end {
                let physical = self.file.stream_position()?;
                let limit = state
                    .input
                    .len()
                    .min((state.physical_end.saturating_sub(physical)) as usize)
                    .min((PHYSICAL_WORK - self.physical_work) as usize);
                state.input_end = self.file.read(&mut state.input[..limit])?;
                state.input_at = 0;
                self.physical_work += state.input_end as u64;
                WORK.with(|w| w.borrow_mut().compressed_bytes += state.input_end as u64);
            }
            if state.input_end == 0 && state.frame_finished {
                state.eof = true;
                return Ok(0);
            }
            let limit = buffer
                .len()
                .min(8192)
                .min((DECODED_WORK - self.decoded_work) as usize);
            let result = state.decoder.run_on_buffers(
                &state.input[state.input_at..state.input_end],
                &mut buffer[..limit],
            );
            let status = match result {
                Ok(status) => status,
                Err(_) => {
                    self.healthy = false;
                    return Err(failure("compressed_source_invalid"));
                }
            };
            state.input_at += status.bytes_read;
            state.consumed += status.bytes_read as u64;
            state.frame_finished = status.remaining == 0;
            self.decoded_work += status.bytes_written as u64;
            WORK.with(|w| w.borrow_mut().decoded_bytes += status.bytes_written as u64);
            if status.bytes_written > 0 {
                let count = status.bytes_written;
                let excess = (state.recent.len() + count).saturating_sub(RECENT_BYTES);
                state.recent.drain(..excess);
                if state.recent.len() + count > state.recent.capacity() {
                    let capacity = (state.recent.capacity().max(8192) * 2).min(RECENT_BYTES);
                    state
                        .recent
                        .reserve_exact(capacity.saturating_sub(state.recent.len()));
                }
                state.recent.extend(&buffer[..count]);
                state.frontier += count as u64;
                self.position += count as u64;
                return Ok(count);
            }
            if state.input_end == 0 {
                if state.frame_finished {
                    state.eof = true;
                    return Ok(0);
                }
                self.healthy = false;
                return Err(failure("compressed_source_invalid"));
            }
            if status.bytes_read == 0 {
                self.healthy = false;
                return Err(failure("compressed_source_invalid"));
            }
        }
    }
}
impl Seek for RolloutReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let Some(state) = &self.compressed else {
            self.position = self.file.seek(from)?;
            return Ok(self.position);
        };
        let SeekFrom::Start(target) = from else {
            return Err(failure("compressed_seek_unsupported"));
        };
        let start = state.frontier.saturating_sub(state.recent.len() as u64);
        if target >= start && target <= state.frontier {
            self.position = target;
            return Ok(target);
        }
        if target < start {
            self.reset()?;
        }
        // Skip only from the current decoder frontier, never replay cached bytes.
        self.position = self.compressed.as_ref().expect("compressed").frontier;
        let mut buffer = [0; 8192];
        while self.position < target {
            if self.decoded_work >= SEEK_WORK {
                return Err(failure("compressed_work_limit"));
            }
            let count = buffer.len().min((target - self.position) as usize);
            let before = self.decoded_work;
            if self.read(&mut buffer[..count])? == 0 {
                return Err(failure("compressed_seek_past_end"));
            }
            WORK.with(|w| w.borrow_mut().replay_bytes += self.decoded_work - before);
        }
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn until() -> Instant {
        Instant::now() + Duration::from_secs(30)
    }
    #[test]
    fn resident_stream_crosses_old_total_limit_without_replay_and_with_bounded_memory() {
        let source = tempfile::tempfile().unwrap();
        let mut encoder = zstd::stream::write::Encoder::new(&source, 1).unwrap();
        encoder.window_log(20).unwrap();
        io::copy(
            &mut io::repeat(b'x').take(DECODED_WORK * 2 + 1),
            &mut encoder,
        )
        .unwrap();
        encoder.finish().unwrap();
        let mut offset = 0;
        work_reset();
        for _ in 0..5 {
            let mut reader =
                RolloutReader::resident(&source, true, "linear".into(), until(), None).unwrap();
            reader.seek(SeekFrom::Start(offset)).unwrap();
            let count =
                io::copy(&mut reader.by_ref().take(64 * 1024 * 1024), &mut io::sink()).unwrap();
            offset += count;
            assert!(reader.compressed.as_ref().unwrap().recent.len() <= RECENT_BYTES);
            if count == 0 {
                break;
            }
        }
        assert_eq!(offset, DECODED_WORK * 2 + 1);
        assert_eq!(work().decoded_bytes, offset);
        assert_eq!(work().replay_bytes, 0);
    }
    #[test]
    fn decoded_work_exhaustion_preserves_safe_continuation_without_total_history_ceiling() {
        let source = tempfile::tempfile().unwrap();
        let mut encoder = zstd::stream::write::Encoder::new(&source, 1).unwrap();
        encoder.window_log(20).unwrap();
        io::copy(&mut io::repeat(0).take(DECODED_WORK + 19), &mut encoder).unwrap();
        encoder.finish().unwrap();
        let progress;
        {
            let mut reader =
                RolloutReader::resident(&source, true, "budget-resume".into(), until(), None)
                    .unwrap();
            assert!(pending(
                &io::copy(&mut reader, &mut io::sink()).unwrap_err()
            ));
            assert_eq!(reader.decoded_work, DECODED_WORK);
            assert!(reader.compressed.as_ref().unwrap().recent.capacity() <= RECENT_BYTES);
            progress = reader.progress().unwrap();
        }
        let mut reader = RolloutReader::resident(
            &source,
            true,
            "budget-resume".into(),
            until(),
            Some(&progress),
        )
        .unwrap();
        reader.seek(SeekFrom::Start(progress.decoded)).unwrap();
        assert_eq!(io::copy(&mut reader, &mut io::sink()).unwrap(), 19);
    }

    #[test]
    fn replay_budget_is_resumable_and_expired_state_refuses() {
        let source = tempfile::tempfile().unwrap();
        let mut encoder = zstd::stream::write::Encoder::new(&source, 1).unwrap();
        encoder.window_log(20).unwrap();
        io::copy(&mut io::repeat(0).take(DECODED_WORK * 2), &mut encoder).unwrap();
        encoder.finish().unwrap();
        let mut progress = None;
        for _ in 0..3 {
            let mut reader =
                RolloutReader::resident(&source, true, "replay".into(), until(), progress.as_ref())
                    .unwrap();
            match reader.seek(SeekFrom::Start(DECODED_WORK * 2 - 1)) {
                Ok(_) => {
                    assert!(progress.is_some());
                    break;
                }
                Err(e) => assert!(pending(&e)),
            }
            let next = reader.progress();
            assert_ne!(progress, next);
            progress = next;
        }
        READERS.with(|r| r.borrow_mut().clear());
        assert!(
            RolloutReader::resident(&source, true, "replay".into(), until(), progress.as_ref())
                .is_err()
        );
    }
    #[test]
    fn corrupt_truncated_window_and_deadline_refuse() {
        for bytes in [
            b"not zstd".to_vec(),
            zstd::encode_all(b"content".as_slice(), 1).unwrap()[..8].to_vec(),
        ] {
            let mut file = tempfile::tempfile().unwrap();
            std::io::Write::write_all(&mut file, &bytes).unwrap();
            let mut reader = RolloutReader::new(&file, true, until()).unwrap();
            assert!(io::copy(&mut reader, &mut io::sink()).is_err());
        }
        let source = tempfile::tempfile().unwrap();
        let mut encoder = zstd::stream::write::Encoder::new(&source, 1).unwrap();
        encoder.window_log(24).unwrap();
        io::copy(&mut io::repeat(0).take(16 * 1024 * 1024), &mut encoder).unwrap();
        encoder.finish().unwrap();
        let mut reader = RolloutReader::new(&source, true, until()).unwrap();
        assert!(reader.read(&mut [0; 1]).is_err());
        reader.end = Instant::now();
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap_err().to_string(),
            "compressed_deadline"
        );
    }
    #[test]
    fn physical_file_length_is_not_admission_and_concatenated_frames_validate_eof() {
        let source = tempfile::tempfile().unwrap();
        for data in [b"first\n".as_slice(), b"second\n".as_slice()] {
            zstd::stream::copy_encode(data, &source, 1).unwrap();
        }
        let mut reader = RolloutReader::new(&source, true, until()).unwrap();
        let mut text = String::new();
        reader.read_to_string(&mut text).unwrap();
        assert_eq!(text, "first\nsecond\n");
        source.set_len(64 * 1024 * 1024 + 1).unwrap();
        assert!(RolloutReader::new(&source, true, until()).is_ok());
    }
}
