//! Source-bound navigation and bounded sequential search progress. No authority.
use crate::{ContextReference, ConversationEvidence};
use serde::{Deserialize, Serialize};

/// Transient decoder progress, sealed inside references. Not a source position
/// chosen by a model and not a portable decoder snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecoderProgress {
    pub decoded: u64,
    pub compressed: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadWork {
    pub compressed_bytes: u64,
    pub decoded_bytes: u64,
    pub replay_bytes: u64,
    pub records_inspected: u64,
    #[serde(default, skip_serializing_if = "ProviderWork::is_empty")]
    pub provider: ProviderWork,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderWork {
    pub metadata_hash_bytes: u64,
    pub sqlite_snapshots: u64,
    pub sqlite_snapshot_reuses: u64,
    pub sqlite_backup_us: u64,
    pub metadata_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cohort_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cohort_capture_us: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cohort_spool_bytes: u64,
    pub fts_builds: u64,
    pub fts_build_us: u64,
    pub fts_query_us: u64,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

impl ProviderWork {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn add(&mut self, work: &Self) {
        self.metadata_hash_bytes += work.metadata_hash_bytes;
        self.sqlite_snapshots += work.sqlite_snapshots;
        self.sqlite_snapshot_reuses += work.sqlite_snapshot_reuses;
        self.sqlite_backup_us += work.sqlite_backup_us;
        self.metadata_rows += work.metadata_rows;
        self.cohort_rows += work.cohort_rows;
        self.cohort_capture_us += work.cohort_capture_us;
        self.cohort_spool_bytes = self.cohort_spool_bytes.max(work.cohort_spool_bytes);
        self.fts_builds += work.fts_builds;
        self.fts_build_us += work.fts_build_us;
        self.fts_query_us += work.fts_query_us;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCoverage {
    pub decoded_bytes: u64,
    pub physical_eof: bool,
    pub logical_complete: bool,
    pub skipped_records: usize,
    pub unsupported_records: usize,
    pub suppressed_records: usize,
    pub external_history_unaccounted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessagePosition {
    pub byte_offset: u64,
    pub byte_length: u64,
    pub ordinal: usize,
    pub message_id: Option<String>,
    pub turn_id: Option<String>,
    pub timestamp: Option<String>,
    pub chunk_offset: usize,
    pub chunk_total: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptCheckpoint {
    #[serde(default)]
    pub skipped_records: usize,
    #[serde(default)]
    pub unsupported_records: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoder_progress: Option<DecoderProgress>,
    pub native_session_id: String,
    pub source_fingerprint: String,
    pub byte_offset: u64,
    pub record_index: usize,
    pub history_mode: String,
    #[serde(default)]
    pub skipping_record: bool,
    #[serde(default)]
    pub chunk_offset: usize,
    #[serde(default)]
    pub query_observation: crate::QueryObservation,
    pub partial: bool,
    pub suppressed_records: usize,
    pub matched_terms: Vec<String>,
    pub match_sources: Vec<String>,
    pub score: u32,
    pub evidence: Vec<ConversationEvidence>,
    #[serde(default)]
    pub text_matches: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextDirection {
    Start,
    #[default]
    Around,
    Forward,
    Backward,
    Turn,
    Occurrences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextOptions {
    /// UTF-8 byte offset within the authenticated parent message.
    #[serde(default)]
    pub message_offset: Option<usize>,
    #[serde(default)]
    pub before: usize,
    #[serde(default)]
    pub after: usize,
    #[serde(default = "context_messages")]
    pub max_messages: usize,
    #[serde(default = "context_bytes")]
    pub max_bytes: usize,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub direction: ContextDirection,
}
fn context_messages() -> usize {
    20
}
fn context_bytes() -> usize {
    16 * 1024
}
impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            message_offset: None,
            before: 0,
            after: 0,
            max_messages: context_messages(),
            max_bytes: context_bytes(),
            query: None,
            direction: ContextDirection::Around,
        }
    }
}
impl ContextOptions {
    pub fn valid(&self) -> bool {
        self.message_offset.is_none_or(|n| n <= 8 * 1024 * 1024)
            && self.before <= 100
            && self.after <= 100
            && (1..=20).contains(&self.max_messages)
            && (1..=16 * 1024).contains(&self.max_bytes)
            && self.query.as_ref().is_none_or(|q| {
                !q.trim().is_empty() && q.len() <= 256 && !q.chars().any(char::is_control)
            })
            && (!matches!(self.direction, ContextDirection::Occurrences) || self.query.is_some())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextWindow {
    #[serde(default)]
    pub source_coverage: SourceCoverage,
    #[serde(default)]
    pub work: ReadWork,
    pub messages: Vec<ConversationEvidence>,
    pub previous: Option<ContextReference>,
    pub next: Option<ContextReference>,
    pub at_start: bool,
    pub at_end: bool,
    pub partial: bool,
    pub bytes_read: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptMessage {
    pub evidence: ConversationEvidence,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptPage {
    #[serde(default)]
    pub work: ReadWork,
    /// Physical EOF alone does not establish logical-session completeness.
    #[serde(default)]
    pub physical_eof: bool,
    pub end: TranscriptCheckpoint,
    pub messages: Vec<TranscriptMessage>,
    pub next: Option<TranscriptCheckpoint>,
    pub complete: bool,
    pub source_revision: String,
    pub bytes_read: usize,
}

/// Internal trusted-composition cache. Never accepted as an MCP argument.
/// The worker must revalidate exact source revision and query scope before reuse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CachedSessionSearch {
    pub native_session_id: String,
    pub source_fingerprint: String,
    pub query_scope: String,
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub match_sources: Vec<String>,
    pub evidence: Vec<ConversationEvidence>,
    pub query_observation: crate::QueryObservation,
    pub content_complete: bool,
}

/// Keyset boundary between bounded metadata windows; time values are UTC seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryBoundary {
    pub native_session_id: String,
    pub sort_value: Option<i64>,
}
