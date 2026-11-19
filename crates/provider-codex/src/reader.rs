//! Exact Codex rollout locators and bounded JSONL paging.
use crate::{
    rollout_reader::{self, RolloutReader},
    worker::{
        Checkpoint, SourceAnchor, TranscriptCursor, TranscriptMessage, TranscriptPage, stale,
        unavailable,
    },
};
use agentmender_domain::{SessionIdentity, TranscriptRole, safe_conversation_text};
use agentmender_provider_api::{ProviderError, filesystem};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    time::Instant,
};

const RECORD_BYTES: usize = 256 * 1024;
const STREAM_RECORD_BYTES: usize = 8 * 1024 * 1024;
const SESSION_SLICE_BYTES: usize = 12 * 1024 * 1024;
const PAGE_OUTPUT_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HistoryMode {
    Legacy,
    Paginated,
}

struct Message {
    role: TranscriptRole,
    index: usize,
    text: String,
    offset: u64,
}

struct Scan {
    messages: Vec<Message>,
    next: Option<Checkpoint>,
    end: Checkpoint,
    complete: bool,
}

struct OpenTranscript {
    file: fs::File,
    size: u64,
    anchor: SourceAnchor,
}

impl OpenTranscript {
    fn new(identity: &SessionIdentity, locator: Option<&Path>) -> Result<Self, ProviderError> {
        let (file, path) = open_transcript(&identity.state_root, locator)?;
        let metadata = file.metadata().map_err(|_| unavailable())?;
        let anchor = SourceAnchor {
            locator: locator.map(Path::to_path_buf),
            path,
            revision: stamp(&metadata),
        };
        Ok(Self {
            file,
            size: metadata.len(),
            anchor,
        })
    }
}

#[cfg(target_os = "macos")]
pub(super) fn current_anchor(
    identity: &SessionIdentity,
    locator: Option<&Path>,
) -> Result<SourceAnchor, ProviderError> {
    Ok(OpenTranscript::new(identity, locator)?.anchor)
}

fn content_limit(code: &'static str) -> ProviderError {
    ProviderError::CapabilityUnavailable(code.into())
}
// A closed diagnostic vocabulary, not arbitrary provider/OS error messages.
fn rollout_io(error: std::io::Error) -> ProviderError {
    match error.to_string().as_str() {
        "compressed_source_invalid" => content_limit("compressed_source_invalid"),
        "compressed_work_limit" => content_limit("compressed_work_limit"),
        "compressed_continuation_expired" => stale(),
        "compressed_seek_past_end" => stale(),
        _ => unavailable(),
    }
}
pub(super) fn compressed_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".jsonl.zst"))
}
pub(super) fn other_representation(path: &Path) -> PathBuf {
    if compressed_path(path) {
        path.with_extension("")
    } else {
        let mut name = path.as_os_str().to_os_string();
        name.push(".zst");
        PathBuf::from(name)
    }
}
pub(super) fn hash(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
pub(super) fn stamp(m: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            m.dev(),
            m.ino(),
            m.nlink(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec()
        )
    }
    #[cfg(not(unix))]
    {
        format!("{}:{:?}", m.len(), m.modified())
    }
}
pub(super) fn open_transcript(
    root: &Path,
    locator: Option<&Path>,
) -> Result<(fs::File, PathBuf), ProviderError> {
    let path = locator.ok_or_else(|| content_limit("missing_supported_rollout_locator"))?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| content_limit("rollout_locator_outside_root"))?;
    if !matches!(relative.components().next(),Some(Component::Normal(n)) if n=="sessions" || n=="archived_sessions")
        || (path.extension().is_none_or(|v| v != "jsonl") && !compressed_path(path))
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(content_limit("unsupported_rollout_location"));
    }
    match filesystem::open_source_read(root, path) {
        Ok(file) => return Ok((file, path.to_path_buf())),
        Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {}
        Err(error) => return Err(error),
    }
    // Codex compresses selected rollouts without updating legacy metadata locators.
    // Prefer only the exact alternate representation before fixed relocations.
    let counterpart = other_representation(path);
    match filesystem::open_source_read(root, &counterpart) {
        Ok(file) => return Ok((file, counterpart)),
        Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {}
        Err(error) => return Err(error),
    }
    // Fixed counterparts only: no recursive scan, root widening or auth lookup.
    let name = path.file_name().ok_or_else(unavailable)?;
    let rest: PathBuf = relative.components().skip(1).collect();
    let mut alternatives = BTreeSet::from([
        root.join("archived_sessions").join(name),
        root.join("archived_sessions").join(&rest),
        root.join("sessions").join(&rest),
    ]);
    if let Some(date) = name
        .to_str()
        .and_then(|n| n.strip_prefix("rollout-"))
        .and_then(|n| n.get(..10))
        && date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    {
        alternatives.insert(
            root.join("sessions")
                .join(&date[..4])
                .join(&date[5..7])
                .join(&date[8..10])
                .join(name),
        );
    }
    alternatives.remove(path);
    alternatives.extend(
        alternatives
            .iter()
            .map(|p| other_representation(p))
            .collect::<Vec<_>>(),
    );
    alternatives.remove(&counterpart);
    let mut found = None;
    for alternative in alternatives {
        match filesystem::open_source_read(root, &alternative) {
            Ok(file) => {
                if found.is_some() {
                    return Err(content_limit("ambiguous_rollout_counterparts"));
                }
                found = Some((file, alternative));
            }
            Err(ProviderError::CapabilityUnavailable(code)) if code == "source file missing" => {}
            Err(error) => return Err(error),
        }
    }
    found.ok_or_else(|| content_limit("source file missing"))
}
fn header_bytes(file: &fs::File, compressed: bool, end: Instant) -> Result<Vec<u8>, ProviderError> {
    let mut header_reader = BufReader::with_capacity(
        8192,
        RolloutReader::new(file, compressed, end).map_err(rollout_io)?,
    );
    let mut header_bytes = Vec::new();
    let count = header_reader
        .by_ref()
        .take((RECORD_BYTES + 1) as u64)
        .read_until(b'\n', &mut header_bytes)
        .map_err(rollout_io)?;
    drop(header_reader);
    if count == 0 || count > RECORD_BYTES {
        return Err(content_limit("bounded_session_header_unavailable"));
    }
    Ok(header_bytes)
}

pub(super) fn header(
    file: &fs::File,
    compressed: bool,
    native_id: &str,
    end: Instant,
) -> Result<(usize, HistoryMode), ProviderError> {
    let header_bytes = header_bytes(file, compressed, end)?;
    let record: serde_json::Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| content_limit("header_not_session_meta"))?;
    if record["type"] != "session_meta" {
        return Err(content_limit("header_not_session_meta"));
    }
    let payload = &record["payload"];
    if payload["id"].as_str() != Some(native_id) {
        return Err(content_limit("header_identity_mismatch"));
    }
    if payload["cli_version"]
        .as_str()
        .is_none_or(|s| s.is_empty() || s.len() > 128)
    {
        return Err(content_limit("header_producer_missing"));
    }
    let format = match payload.get("history_mode").and_then(|v| v.as_str()) {
        None | Some("legacy") => HistoryMode::Legacy,
        Some("paginated") => HistoryMode::Paginated,
        _ => return Err(content_limit("unknown_history_mode")),
    };
    Ok((header_bytes.len(), format))
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ObservedSource {
    Main,
    Spawned(String),
    Other,
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum SourceValue {
    Name(String),
    Structured { subagent: Option<SubagentSource> },
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SubagentSource {
    Review,
    Compact,
    MemoryConsolidation,
    ThreadSpawn { parent_thread_id: String },
    Other(String),
}

#[cfg(target_os = "macos")]
impl ObservedSource {
    fn from_marker(source: Option<&SourceValue>) -> Result<Self, ProviderError> {
        match source {
            Some(SourceValue::Name(source))
                if matches!(
                    source.as_str(),
                    "cli" | "vscode" | "exec" | "mcp" | "appServer"
                ) =>
            {
                Ok(Self::Main)
            }
            Some(SourceValue::Structured {
                subagent: Some(SubagentSource::ThreadSpawn { parent_thread_id }),
            }) if agentmender_domain::canonical_uuid(parent_thread_id) => {
                Ok(Self::Spawned(parent_thread_id.clone()))
            }
            Some(SourceValue::Structured {
                subagent: Some(SubagentSource::ThreadSpawn { .. }),
            }) => Err(unavailable()),
            Some(SourceValue::Structured {
                subagent: Some(SubagentSource::Other(source)),
            }) if source.is_empty() => Err(unavailable()),
            _ => Ok(Self::Other),
        }
    }

    pub fn from_database(source: Option<&str>) -> Result<Self, ProviderError> {
        let marker = match source {
            Some(source) if source.starts_with(['{', '"', '[']) => {
                Some(serde_json::from_str(source).map_err(|_| unavailable())?)
            }
            Some(source) => Some(SourceValue::Name(source.into())),
            None => None,
        };
        Self::from_marker(marker.as_ref())
    }
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
struct RelationshipHeader {
    #[serde(rename = "type")]
    kind: String,
    payload: RelationshipMeta,
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
pub(super) struct RelationshipMeta {
    pub id: String,
    cli_version: String,
    source: Option<SourceValue>,
    pub session_id: Option<String>,
    pub parent_thread_id: Option<String>,
    pub forked_from_id: Option<String>,
    pub forked_from_ordinal_exclusive: Option<u64>,
    #[serde(default)]
    pub attached_thread: bool,
    pub thread_source: Option<String>,
    pub history_base: Option<HistoryBase>,
}

#[cfg(target_os = "macos")]
#[derive(serde::Deserialize)]
pub(super) struct HistoryBase {
    pub thread_id: String,
}

#[cfg(target_os = "macos")]
impl RelationshipMeta {
    pub fn source(&self) -> Result<ObservedSource, ProviderError> {
        ObservedSource::from_marker(self.source.as_ref())
    }

    pub fn is_main(&self, id: &str) -> bool {
        self.source()
            .is_ok_and(|source| source == ObservedSource::Main)
            && self
                .session_id
                .as_deref()
                .is_none_or(|session| session == id)
            && self.parent_thread_id.is_none()
            && self.forked_from_id.is_none()
            && self.forked_from_ordinal_exclusive.is_none()
            && !self.attached_thread
            && self.history_base.is_none()
            && self
                .thread_source
                .as_deref()
                .is_none_or(|source| source == "user")
    }
}

#[cfg(target_os = "macos")]
pub(super) fn relationship_header(
    file: &fs::File,
    path: &Path,
    id: Option<&str>,
    end: Instant,
) -> Result<RelationshipMeta, ProviderError> {
    let header: RelationshipHeader =
        serde_json::from_slice(&header_bytes(file, compressed_path(path), end)?)
            .map_err(|_| unavailable())?;
    let meta = header.payload;
    if header.kind != "session_meta"
        || !agentmender_domain::canonical_uuid(&meta.id)
        || id.is_some_and(|id| meta.id != id)
        || meta.cli_version.is_empty()
        || meta.cli_version.len() > 128
        || [
            meta.session_id.as_deref(),
            meta.parent_thread_id.as_deref(),
            meta.forked_from_id.as_deref(),
            meta.history_base
                .as_ref()
                .map(|base| base.thread_id.as_str()),
        ]
        .into_iter()
        .flatten()
        .any(|id| !agentmender_domain::canonical_uuid(id))
        || (meta.attached_thread || meta.forked_from_ordinal_exclusive.is_some())
            && meta.forked_from_id.is_none()
    {
        return Err(unavailable());
    }
    Ok(meta)
}

fn scan(
    transcript: &OpenTranscript,
    identity: &SessionIdentity,
    end: Instant,
    checkpoint: Option<&Checkpoint>,
) -> Result<Scan, ProviderError> {
    let file = &transcript.file;
    let compressed = compressed_path(&transcript.anchor.path);
    let size = transcript.size;
    if checkpoint.is_some_and(|cursor| !compressed && cursor.byte_offset > size) {
        return Err(stale());
    }
    let (count, format) = header(file, compressed, &identity.native_id, end)?;
    let mut offset = checkpoint.map_or(count as u64, |c| c.byte_offset);
    if offset < count as u64 {
        return Err(stale());
    }
    let key =
        hash(&serde_json::to_vec(&(identity, &transcript.anchor)).map_err(|_| unavailable())?);
    let mut reader = BufReader::with_capacity(
        8192,
        RolloutReader::resident(
            file,
            compressed,
            key,
            end,
            checkpoint.and_then(|p| p.decoder_progress.as_ref()),
        )
        .map_err(rollout_io)?,
    );
    let mut replay_pending = false;
    if let Err(error) = reader.seek(SeekFrom::Start(offset)) {
        if rollout_reader::pending(&error) {
            replay_pending = true;
        } else {
            return Err(rollout_io(error));
        }
    }
    let mut index = checkpoint.map_or(1, |c| c.record_index);
    let mut partial = checkpoint.is_some_and(|c| c.partial) || format == HistoryMode::Paginated;
    let mut messages = vec![];
    let mut recognized = false;
    let mut captured_bytes = 0usize;
    let mut skipping = checkpoint.is_some_and(|c| c.skipping_record);
    let mut bytes_read = 0;
    let limit = SESSION_SLICE_BYTES.saturating_sub(count);
    let mut reached_eof = false;
    while !replay_pending && (compressed || offset < size) && bytes_read < limit {
        if Instant::now() >= end || captured_bytes >= PAGE_OUTPUT_BYTES {
            break;
        }
        let remaining = limit.saturating_sub(bytes_read);
        // A bounded per-record buffer, even for malformed multi-gigabyte lines.
        let mut line = Vec::new();
        let read_result = reader
            .by_ref()
            .take(remaining.min(STREAM_RECORD_BYTES + 1) as u64)
            .read_until(b'\n', &mut line);
        let read = match read_result {
            Ok(read) => read,
            Err(error) if rollout_reader::pending(&error) => {
                replay_pending = true;
                break;
            }
            Err(error) => return Err(rollout_io(error)),
        };
        if read == 0 {
            reached_eof = true;
            break;
        }
        bytes_read += read;
        let complete_line = line.last() == Some(&b'\n')
            || (!compressed && offset + read as u64 == size)
            || (compressed && read < remaining.min(STREAM_RECORD_BYTES + 1));
        if !complete_line {
            if line.len() <= STREAM_RECORD_BYTES && !skipping {
                // Revisit this record with a fresh work budget. No bytes are lost.
                break;
            }
            partial = true;
            skipping = true;
            offset += read as u64;
            continue;
        }
        let current_offset = offset;
        offset += read as u64;
        index += 1;
        if skipping || line.len() > STREAM_RECORD_BYTES {
            partial = true;
            skipping = false;
            continue;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let mut mark_unsupported = || partial = true;
        let record: serde_json::Value = match serde_json::from_slice(&line) {
            Ok(v) => v,
            Err(_) => {
                mark_unsupported();
                continue;
            }
        };
        let kind = record.get("type").and_then(|v| v.as_str());
        let payload = record.get("payload").unwrap_or(&serde_json::Value::Null);
        match kind {
            Some("response_item") => {
                // Paginated history persists both model and display views. Use
                // only completed display items there, never double count them.
                if format == HistoryMode::Paginated {
                    continue;
                }
                if payload.get("type").and_then(|v| v.as_str()) != Some("message") {
                    if !matches!(
                        payload.get("type").and_then(|v| v.as_str()),
                        Some(
                            "reasoning"
                                | "function_call"
                                | "function_call_output"
                                | "custom_tool_call"
                                | "custom_tool_call_output"
                                | "web_search_call"
                        )
                    ) {
                        mark_unsupported();
                    }
                    continue;
                }
                let (role, part_type) = match payload.get("role").and_then(|v| v.as_str()) {
                    Some("user") => (TranscriptRole::User, "input_text"),
                    Some("assistant") => (TranscriptRole::Assistant, "output_text"),
                    Some("system" | "developer") => continue,
                    _ => {
                        mark_unsupported();
                        continue;
                    }
                };
                if payload.get("channel").and_then(|v| v.as_str()) == Some("analysis")
                    || payload.get("phase").and_then(|v| v.as_str()) == Some("analysis")
                    || payload
                        .get("recipient")
                        .and_then(|v| v.as_str())
                        .is_some_and(|v| v != "all")
                {
                    continue;
                }
                if payload
                    .get("channel")
                    .filter(|v| !v.is_null())
                    .is_some_and(|v| v != "final" && v != "commentary")
                    || payload
                        .get("phase")
                        .filter(|v| !v.is_null())
                        .is_some_and(|v| v != "final_answer" && v != "commentary")
                {
                    mark_unsupported();
                    continue;
                }
                let Some(content) = payload.get("content").and_then(|v| v.as_array()) else {
                    mark_unsupported();
                    continue;
                };
                let mut text = String::new();
                for part in content {
                    if part.get("type").and_then(|v| v.as_str()) != Some(part_type) {
                        mark_unsupported();
                        continue;
                    }
                    if let Some(value) = part.get("text").and_then(|v| v.as_str()) {
                        text.push_str(value);
                        text.push(' ');
                    } else {
                        mark_unsupported();
                    }
                }
                text.pop();
                recognized = true;
                if !safe_conversation_text(&text) {
                    partial = true;
                    continue;
                }
                if !text.trim().is_empty() {
                    captured_bytes = captured_bytes.saturating_add(text.len());
                    messages.push(Message {
                        role,
                        index,
                        text,
                        offset: current_offset,
                    });
                }
            }
            Some("event_msg") if format == HistoryMode::Paginated => {
                if payload.get("type").and_then(|v| v.as_str()) != Some("item_completed") {
                    continue;
                }
                if payload.get("thread_id").and_then(|v| v.as_str()) != Some(&identity.native_id) {
                    mark_unsupported();
                    continue;
                }
                let item = &payload["item"];
                let (role, part_type) = match item.get("type").and_then(|v| v.as_str()) {
                    Some("UserMessage") => (TranscriptRole::User, "text"),
                    Some("AgentMessage") => (TranscriptRole::Assistant, "Text"),
                    _ => continue,
                };
                if item
                    .get("phase")
                    .filter(|v| !v.is_null())
                    .is_some_and(|v| v != "final_answer" && v != "commentary")
                {
                    continue;
                }
                let Some(parts) = item.get("content").and_then(|v| v.as_array()) else {
                    continue;
                };
                let mut text = String::new();
                for part in parts {
                    if part.get("type").and_then(|v| v.as_str()) == Some(part_type)
                        && let Some(value) = part.get("text").and_then(|v| v.as_str())
                    {
                        text.push_str(value);
                        text.push(' ');
                    }
                }
                text.pop();
                recognized = true;
                if !safe_conversation_text(&text) {
                    partial = true;
                } else if !text.trim().is_empty() {
                    captured_bytes = captured_bytes.saturating_add(text.len());
                    messages.push(Message {
                        role,
                        index,
                        text,
                        offset: current_offset,
                    });
                }
            }
            Some("turn_context") => {}
            Some("event_msg") => {
                if !matches!(
                    payload.get("type").and_then(|v| v.as_str()),
                    Some(
                        "token_count"
                            | "task_started"
                            | "task_complete"
                            | "turn_aborted"
                            | "agent_reasoning"
                            | "agent_reasoning_raw_content"
                            | "agent_reasoning_section_break"
                            | "agent_reasoning_raw_content_section_break"
                            | "exec_command_begin"
                            | "exec_command_end"
                            | "patch_apply_begin"
                            | "patch_apply_end"
                    )
                ) {
                    mark_unsupported();
                }
            }
            _ => {
                mark_unsupported();
            }
        }
    }
    if compressed && !reached_eof && !replay_pending {
        // EOF verifies frame termination and any advertised checksum; corruption cannot turn
        // a valid prefix into complete negative evidence.
        match reader.fill_buf() {
            Ok(bytes) => reached_eof = bytes.is_empty(),
            Err(error) if rollout_reader::pending(&error) => replay_pending = true,
            Err(error) => return Err(rollout_io(error)),
        }
    }
    let decoder_progress = if replay_pending {
        reader.get_ref().progress()
    } else {
        None
    };
    drop(reader);
    if transcript.anchor.revision != stamp(&file.metadata().map_err(|_| unavailable())?)
        || transcript.anchor
            != OpenTranscript::new(identity, transcript.anchor.locator.as_deref())?.anchor
    {
        return Err(stale());
    }
    let progress = Checkpoint {
        decoder_progress,
        byte_offset: offset,
        record_index: index,
        chunk_offset: 0,
        partial,
        skipping_record: skipping,
    };
    let next = (if compressed {
        !reached_eof
    } else {
        offset < size
    })
    .then(|| progress.clone());
    if let (Some(prior), Some(next)) = (checkpoint, &next)
        && prior.byte_offset == next.byte_offset
        && prior.record_index == next.record_index
        && prior.decoder_progress == next.decoder_progress
    {
        return Err(content_limit("transcript_no_progress"));
    }
    let complete = next.is_none() && !partial && (recognized || checkpoint.is_some());
    Ok(Scan {
        messages,
        next,
        end: progress,
        complete,
    })
}

fn safe_chunk(text: &str, offset: usize, bytes: usize) -> Result<(String, usize), ProviderError> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Err(stale());
    }
    let mut end = (offset + bytes).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == offset && offset < text.len() {
        return Err(content_limit("transcript_budget_cannot_fit_codepoint"));
    }
    let result: String = text[offset..end]
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                ' '
            } else {
                c
            }
        })
        .collect();
    Ok((result, end))
}
pub(super) fn page(
    identity: &SessionIdentity,
    locator: Option<&Path>,
    cursor: Option<&TranscriptCursor>,
    end: Instant,
) -> Result<(TranscriptPage, SourceAnchor), ProviderError> {
    let transcript = OpenTranscript::new(identity, locator)?;
    if cursor
        .is_some_and(|cursor| &cursor.identity != identity || cursor.anchor != transcript.anchor)
    {
        return Err(stale());
    }
    let seed = cursor.map(|cursor| &cursor.checkpoint);
    let scan = scan(&transcript, identity, end, seed)?;
    let mut messages = Vec::new();
    let mut remaining = PAGE_OUTPUT_BYTES;
    let mut next = scan.next;
    for message in scan.messages {
        let offset = seed
            .filter(|cursor| cursor.byte_offset == message.offset)
            .map_or(0, |cursor| cursor.chunk_offset);
        if remaining == 0 {
            next = Some(Checkpoint {
                byte_offset: message.offset,
                record_index: message.index - 1,
                chunk_offset: 0,
                skipping_record: false,
                ..scan.end.clone()
            });
            break;
        }
        let (text, consumed) = safe_chunk(&message.text, offset, remaining)?;
        remaining -= text.len();
        messages.push(TranscriptMessage {
            role: message.role,
            text,
        });
        if consumed < message.text.len() {
            next = Some(Checkpoint {
                byte_offset: message.offset,
                record_index: message.index - 1,
                chunk_offset: consumed,
                skipping_record: false,
                ..scan.end.clone()
            });
            break;
        }
    }
    let anchor = transcript.anchor;
    let complete = next.is_none() && scan.complete;
    let next = next.map(|checkpoint| TranscriptCursor {
        identity: identity.clone(),
        anchor: anchor.clone(),
        checkpoint,
    });
    if next.is_some() && next.as_ref() == cursor {
        return Err(stale());
    }
    Ok((
        TranscriptPage {
            messages,
            next,
            complete,
        },
        anchor,
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use agentmender_domain::ProviderKind;
    use serde_json::json;
    use std::io::Write;

    const ID: &str = "00000000-0000-0000-0000-000000000101";
    const OTHER: &str = "00000000-0000-0000-0000-000000000102";

    fn fixture(root: &Path) -> (SessionIdentity, PathBuf) {
        let directory = root.join("sessions");
        fs::create_dir(&directory).unwrap();
        (
            SessionIdentity {
                provider: ProviderKind::Codex,
                state_root: root.into(),
                namespace: None,
                native_id: ID.into(),
            },
            directory.join(format!("rollout-2026-11-01T12-00-00-{ID}.jsonl")),
        )
    }

    fn common_header(id: &str, mode: &str) -> serde_json::Value {
        json!({"type": "session_meta", "payload": {
            "id": id, "cli_version": "fixture", "history_mode": mode
        }})
    }

    fn visible(body: &str) -> serde_json::Value {
        json!({"type": "response_item", "payload": {
            "type": "message", "role": "assistant", "content": [{"type": "output_text", "text": body}]
        }})
    }

    fn records(rows: &[serde_json::Value]) -> Vec<u8> {
        rows.iter()
            .flat_map(|row| {
                let mut bytes = serde_json::to_vec(row).unwrap();
                bytes.push(b'\n');
                bytes
            })
            .collect()
    }

    fn read(
        identity: &SessionIdentity,
        path: &Path,
        next: Option<&TranscriptCursor>,
    ) -> Result<TranscriptPage, ProviderError> {
        page(
            identity,
            Some(path),
            next,
            Instant::now() + std::time::Duration::from_secs(8),
        )
        .map(|(page, _)| page)
    }

    #[test]
    fn plain_and_compressed_pages_preserve_utf8_and_suppress_secret_and_internal_messages() {
        let text = "éλ界 ".repeat(6000);
        let input = records(&[
            common_header(ID, "legacy"),
            visible(&text),
            visible("password: distinct-secret-marker"),
            json!({"type": "response_item", "payload": {"type": "message", "role": "assistant",
                   "channel": "analysis", "content": [{"type": "output_text", "text": "hidden reasoning"}]}}),
        ]);
        for compressed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let (identity, path) = fixture(root.path());
            if compressed {
                let encoded = zstd::stream::encode_all(&input[..], 1).unwrap();
                fs::write(other_representation(&path), encoded).unwrap();
            } else {
                fs::write(&path, &input).unwrap();
            }

            let mut observed = String::new();
            let mut next = None;
            for count in 0..8 {
                let result = read(&identity, &path, next.as_ref()).unwrap();
                let bytes: usize = result.messages.iter().map(|item| item.text.len()).sum();
                assert!(bytes <= PAGE_OUTPUT_BYTES);
                assert!(!result.complete);
                for message in result.messages {
                    assert_eq!(message.role, TranscriptRole::Assistant);
                    observed.push_str(&message.text);
                }
                next = result.next;
                if next.is_none() {
                    assert!(count > 0);
                    break;
                }
            }
            assert!(next.is_none());
            assert_eq!(observed, text);
        }
    }

    #[test]
    fn paginated_rollout_only_shows_matching_completed_items_and_keeps_partial_coverage() {
        let root = tempfile::tempdir().unwrap();
        let (identity, path) = fixture(root.path());
        let done = |id, body| {
            json!({"type":"event_msg","payload":{
                "type":"item_completed","thread_id":id,"item":{"type":"AgentMessage",
                "content":[{"type":"Text","text":body}]}
            }})
        };
        fs::write(
            &path,
            records(&[
                common_header(ID, "paginated"),
                visible("raw model representation"),
                done(OTHER, "different session"),
                done(ID, "visible display item"),
            ]),
        )
        .unwrap();
        let page = read(&identity, &path, None).unwrap();
        assert_eq!(page.messages.len(), 1);
        assert_eq!(page.messages[0].text, "visible display item");
        assert!(!page.complete);
        assert!(page.next.is_none());
    }

    #[test]
    fn mismatched_header_and_changed_or_foreign_cursor_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let (identity, path) = fixture(root.path());
        fs::write(
            &path,
            records(&[common_header(OTHER, "legacy"), visible("foreign")]),
        )
        .unwrap();
        assert!(read(&identity, &path, None).is_err());

        let body = records(&[
            common_header(ID, "legacy"),
            visible(&"x".repeat(PAGE_OUTPUT_BYTES + 3)),
        ]);
        fs::write(&path, &body).unwrap();
        let next = read(&identity, &path, None).unwrap().next.unwrap();
        let other = tempfile::tempdir().unwrap();
        let (other_identity, other_path) = fixture(other.path());
        fs::write(&other_path, &body).unwrap();
        assert!(read(&other_identity, &other_path, Some(&next)).is_err());

        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        assert!(read(&identity, &path, Some(&next)).is_err());
    }

    #[test]
    fn overlong_record_is_skipped_and_following_message_remains_visible() {
        let root = tempfile::tempdir().unwrap();
        let (identity, path) = fixture(root.path());
        let mut bytes = records(&[common_header(ID, "legacy")]);
        bytes.extend(std::iter::repeat_n(b'x', STREAM_RECORD_BYTES + 64));
        bytes.push(b'\n');
        bytes.extend(records(&[visible("after the oversized record")]));
        fs::write(&path, bytes).unwrap();
        let page = read(&identity, &path, None).unwrap();
        assert_eq!(page.messages.len(), 1);
        assert_eq!(page.messages[0].text, "after the oversized record");
        assert!(!page.complete);
    }
}
