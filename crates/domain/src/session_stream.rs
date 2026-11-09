//! Source-bound navigation and bounded sequential search progress. No authority.
use crate::{ContextReference, ConversationEvidence};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptCheckpoint {
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextDirection {
    #[default]
    Around,
    Forward,
    Backward,
    Turn,
    Occurrences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryBoundary {
    pub native_session_id: String,
    pub sort_value: Option<i64>,
}
