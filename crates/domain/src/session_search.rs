//! Bounded read evidence only. No caller, ownership, approval or effect authority.
use crate::{
    ArtifactId, EntityId, GraphNodeKind, ProviderKind, SessionMetadata, normalize_search_text,
    search_match_offset,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SEARCH_BATCH: usize = 100;
pub const SEARCH_INVENTORY: usize = 10_000;
pub const TRANSCRIPT_BYTES: usize = 1024 * 1024;
pub const RECORD_BYTES: usize = 256 * 1024;
pub const SEARCH_BYTES: usize = 32 * 1024 * 1024;

/// The conclusion for one retrieved candidate, independent of search execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateVerificationState {
    Confirmed,
    Unconfirmed,
    Rejected,
}

/// The execution state of the lexical search and candidate evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchExecutionState {
    InProgress,
    Completed,
    Interrupted,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchPurpose {
    #[default]
    Diagnostic,
    Management,
    Discovery,
}

/// A caller claimed by MCP routing metadata. This carries only read exclusion.
#[derive(Clone)]
pub struct ApparentCodexCaller(String);

impl ApparentCodexCaller {
    pub fn from_metadata(thread: &str, session: Option<&str>) -> Option<Self> {
        if thread.len() != 36
            || !thread.bytes().enumerate().all(|(i, b)| match i {
                8 | 13 | 18 | 23 => b == b'-',
                _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
            })
            || session.is_some_and(|id| id != thread)
        {
            return None;
        }
        Some(Self(thread.into()))
    }

    pub fn native_id(&self) -> &str {
        &self.0
    }

    pub fn identity(&self, root: &std::path::Path) -> ArtifactId {
        ArtifactId {
            provider: ProviderKind::Codex,
            state_root: root.into(),
            class: GraphNodeKind::Session,
            local_id: EntityId::new(format!("codex:session:{}", self.0)).expect("validated UUID"),
        }
    }
}

pub fn discovery_binding(id: &ArtifactId) -> String {
    let bytes = serde_json::to_vec(&("agentmender/discovery/v1", id)).expect("identity serializes");
    search_digest(&bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchParameters {
    #[serde(default)]
    pub structured: Option<crate::StructuredQuery>,
    pub query: String,
    pub include_archived: bool,
    pub include_content: bool,
    pub project_label: Option<String>,
    pub purpose: SearchPurpose,
    pub page_size: usize,
    pub excerpts: bool,
}
impl SearchParameters {
    pub fn valid(&self) -> bool {
        self.structured
            .as_ref()
            .is_none_or(crate::StructuredQuery::valid)
            && self.query.len() <= 256
            && !self.query.chars().any(char::is_control)
            && self.query.split_whitespace().count() <= 16
            && (1..=20).contains(&self.page_size)
            && self
                .project_label
                .as_ref()
                .is_none_or(|p| !p.is_empty() && p.len() <= 256 && !p.chars().any(char::is_control))
    }
    pub fn reads_content(&self) -> bool {
        self.include_content
            || self
                .structured
                .as_ref()
                .is_some_and(|q| q.filter.needs_content())
    }
    pub fn ranking_terms(&self) -> Vec<String> {
        let mut terms = self.terms();
        if let Some(q) = &self.structured {
            terms.extend(q.filter.text_terms());
        }
        terms.sort();
        terms.dedup();
        terms
    }
    pub fn terms(&self) -> Vec<String> {
        self.query
            .split_whitespace()
            .map(normalize_search_text)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// DiscoveryPriority uses this key within each native-ID inventory window.
pub fn discovery_title_weight(metadata: &SessionMetadata, terms: &[String]) -> usize {
    let title = normalize_search_text(metadata.title.as_deref().unwrap_or_default());
    terms
        .iter()
        .filter(|term| title.contains(term.as_str()))
        .map(|term| term.len().min(32))
        .sum()
}

/// Trusted typed host-composition input, never MCP/model metadata. Production
/// launch declarations alone cannot construct an admitted identity.
#[derive(Debug, Clone)]
pub struct ManagementIdentity {
    pub identity: ArtifactId,
    pub native_session_id: String,
    pub revision: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchContinuation {
    pub scope: String,
    pub inventory_fingerprint: String,
    #[serde(default)]
    pub corpus_fingerprint: String,
    #[serde(default)]
    pub inventory_after: Option<crate::InventoryBoundary>,
    pub batch_fingerprint: Option<String>,
    pub start: usize,
    pub end: Option<usize>,
    pub result_offset: usize,
    #[serde(default)]
    pub stream: Option<crate::TranscriptCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchReadRequest {
    #[serde(default)]
    pub cached: Vec<crate::CachedSessionSearch>,
    pub parameters: SearchParameters,
    pub excluded: Vec<ArtifactId>,
    pub management_session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_session: Option<String>,
    pub continuation: SearchContinuation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    VerifiedRelevant,
    Related,
    Contradiction,
    Insufficient,
}

impl Disposition {
    pub fn verification_state(self) -> crate::CandidateVerificationState {
        match self {
            Self::VerifiedRelevant => crate::CandidateVerificationState::Confirmed,
            Self::Insufficient => crate::CandidateVerificationState::Unconfirmed,
            Self::Related | Self::Contradiction => crate::CandidateVerificationState::Rejected,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelationship {
    Direct,
    Discussion,
    Retrospective,
    Contradiction,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub native_session_id: String,
    pub disposition: Disposition,
    pub evidence: Vec<usize>,
    #[serde(default)]
    pub outcomes: Vec<crate::search_intent::Outcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<crate::search_intent::SearchActivity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<EvidenceRelationship>,
    pub explanation: String,
    #[serde(default)]
    pub missing_evidence: Option<String>,
    #[serde(default)]
    pub plausible_alternative: bool,
}

/// Internal read-only scope for one captured conversational inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortScope {
    pub id: String,
    pub caller: String,
    pub excluded: Vec<ArtifactId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortPosition {
    pub index: usize,
    pub after: Option<String>,
    pub stream: Option<crate::TranscriptCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortRead {
    pub scope: CohortScope,
    pub parameters: SearchParameters,
    pub position: CohortPosition,
    /// Reacquire one existing identity using its frozen query metadata.
    pub refresh: Option<String>,
}

pub const COHORT_BATCH: usize = 5;
pub const COHORT_SCAN_PAGES: usize = 12;

/// A staged result awaiting fresh verification before publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortReceipt {
    pub metadata: SessionMetadata,
    pub basis: ReceiptBasis,
    pub assessment: Option<Assessment>,
    pub citations: Vec<ContextReference>,
    pub metadata_ref: Option<MetadataSelection>,
    pub title_match: Option<crate::TitleMatchQuality>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptBasis {
    VerifiedMetadata,
    SemanticEvidence,
    PlausibleEvidence,
    RetrievedUnverified,
}

impl ReceiptBasis {
    pub fn confirmed(self) -> bool {
        matches!(self, Self::VerifiedMetadata | Self::SemanticEvidence)
    }
}

impl CohortReceipt {
    pub fn verification_reference(&self) -> Option<SelectionReference> {
        if let Some(reference) = &self.metadata_ref {
            Some(SelectionReference::Metadata(reference.clone()))
        } else {
            self.citations
                .first()
                .cloned()
                .map(SelectionReference::Transcript)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CohortQueue {
    Candidates { after: Option<String> },
    Record { receipts: Vec<CohortReceipt> },
    Receipts { after: Option<String> },
    KeepAlive,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CohortQueuePage {
    Candidates(Vec<SessionSearchHit>),
    Receipts(Vec<CohortReceipt>),
    Acknowledged { spool_bytes: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortPage {
    pub sessions: Vec<SessionSearchHit>,
    pub unresolved: Vec<SessionSearchHit>,
    pub coverage: SearchCoverage,
    pub next: Option<CohortPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationEvidence {
    pub source: String,
    pub record_index: usize,
    #[serde(default)]
    pub position: Option<crate::MessagePosition>,
    pub excerpt: Option<String>,
    pub truncated: bool,
    pub source_fingerprint: String,
    pub source_bytes: usize,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSearchHit {
    #[serde(default)]
    pub source_coverage: Option<crate::SourceCoverage>,
    #[serde(default)]
    pub query_observation: crate::QueryObservation,
    #[serde(default)]
    pub content_complete: bool,
    pub metadata: SessionMetadata,
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub match_sources: Vec<String>,
    pub evidence: Vec<ConversationEvidence>,
    pub content_status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchCoverage {
    #[serde(default)]
    pub work: crate::ReadWork,
    pub eligible_observed: usize,
    pub sessions_inspected: usize,
    #[serde(default)]
    pub unresolved_sessions: usize,
    #[serde(default)]
    pub fully_traversed_sessions: usize,
    pub bytes_read: usize,
    pub matched_in_batch: usize,
    pub unsupported_sessions: usize,
    pub unreadable_sessions: usize,
    pub truncated_sessions: usize,
    pub suppressed_records: usize,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSearchPage {
    #[serde(default)]
    pub unresolved: Vec<SessionSearchHit>,
    pub sessions: Vec<SessionSearchHit>,
    pub coverage: SearchCoverage,
    pub inventory_fingerprint: String,
    pub batch_fingerprint: String,
    pub next: Option<SearchContinuation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceVerification {
    pub sessions: Vec<Option<SessionMetadata>>,
    pub work: crate::ProviderWork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReference {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoder_progress: Option<crate::DecoderProgress>,
    pub native_session_id: String,
    pub local_id: crate::EntityId,
    pub record_index: usize,
    #[serde(default)]
    pub position: Option<crate::MessagePosition>,
    pub source_fingerprint: String,
    pub source_bytes: usize,
    pub purpose: SearchPurpose,
    pub binding_revision: String,
}

/// A commitment to the observed row in the configured Codex metadata store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataSelection {
    pub provider: ProviderKind,
    pub local_id: EntityId,
    pub native_session_id: String,
    pub binding_revision: String,
    pub metadata_fingerprint: String,
}

impl MetadataSelection {
    pub fn new(metadata: &SessionMetadata, binding: &str) -> Self {
        let bytes = serde_json::to_vec(&(
            "agentmender/codex-thread-metadata/v1",
            &metadata.identity,
            &metadata.native_session_id,
            &metadata.title,
            &metadata.project_label,
            metadata.state,
            metadata.created_at,
            metadata.last_active_at,
        ))
        .expect("metadata facts serialize");
        Self {
            provider: metadata.identity.provider,
            local_id: metadata.identity.local_id.clone(),
            native_session_id: metadata.native_session_id.clone(),
            binding_revision: binding.into(),
            metadata_fingerprint: search_digest(&bytes),
        }
    }
}

/// The two sources of read-only selection. Preserve the transcript wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SelectionReference {
    Transcript(ContextReference),
    Metadata(MetadataSelection),
}

impl SelectionReference {
    pub fn native_session_id(&self) -> &str {
        match self {
            Self::Transcript(r) => &r.native_session_id,
            Self::Metadata(r) => &r.native_session_id,
        }
    }

    pub fn local_id(&self) -> &EntityId {
        match self {
            Self::Transcript(r) => &r.local_id,
            Self::Metadata(r) => &r.local_id,
        }
    }

    pub fn binding_revision(&self) -> &str {
        match self {
            Self::Transcript(r) => &r.binding_revision,
            Self::Metadata(r) => &r.binding_revision,
        }
    }

    pub fn is_discovery(&self) -> bool {
        match self {
            Self::Transcript(r) => r.purpose == SearchPurpose::Discovery,
            Self::Metadata(r) => r.provider == ProviderKind::Codex,
        }
    }
}

pub fn search_scope(
    parameters: &SearchParameters,
    root: &std::path::Path,
    excluded: &[ArtifactId],
    revision: &str,
) -> String {
    let bytes = serde_json::to_vec(&(parameters, root, excluded, revision))
        .expect("search values serialize");
    search_digest(&bytes)
}

fn search_digest(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// Conservative suppression; never described as perfect secret detection.
pub fn safe_conversation_text(text: &str) -> bool {
    safe_lowered_conversation_text(&text.to_lowercase())
}

fn safe_lowered_conversation_text(lower: &str) -> bool {
    ![
        "-----begin",
        "authorization:",
        "bearer ",
        "sk-",
        "ghp_",
        "github_pat_",
        "api_key",
        "api-key",
        "api key",
        "access_token",
        "refresh_token",
        "password",
        "passwd",
        "secret=",
        "secret:",
        "private_key",
        "client_secret",
        "aws_secret",
        "token=",
        "token:",
        "cookie:",
        "set-cookie:",
        "process.env",
        "printenv",
        "config.toml",
        "[mcp_servers",
        "<environment_context>",
    ]
    .iter()
    .any(|p| lower.contains(p))
}

pub fn bounded_excerpt(text: &str, terms: &[String], max: usize) -> (String, bool) {
    let chars: Vec<_> = text.chars().collect();
    let offset = search_match_offset(text, terms);
    let at = text[..offset].chars().count();
    let start = at.saturating_sub(80);
    let end = chars.len().min(start + max);
    let excerpt: String = chars[start..end]
        .iter()
        .map(|c| {
            if c.is_control() || matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
                ' '
            } else {
                *c
            }
        })
        .collect();
    (excerpt, start > 0 || end < chars.len())
}

/// Explainable lexical evidence; messages are never interpreted as instructions.
pub struct SearchRelevance {
    required: Vec<String>,
    terms: Vec<String>,
    matched: BTreeSet<String>,
    sources: BTreeSet<String>,
    pub score: u32,
}
impl SearchRelevance {
    pub fn new(parameters: &SearchParameters) -> Self {
        Self::with_terms(parameters.terms(), parameters.ranking_terms())
    }

    pub fn with_terms(required: Vec<String>, terms: Vec<String>) -> Self {
        Self {
            required,
            terms,
            matched: BTreeSet::new(),
            sources: BTreeSet::new(),
            score: 0,
        }
    }
    pub fn observe(&mut self, source: &str, text: &str) -> usize {
        self.observe_prepared(source, &crate::PreparedText::new(text))
    }

    pub fn observe_prepared(&mut self, source: &str, text: &crate::PreparedText<'_>) -> usize {
        if !safe_lowered_conversation_text(text.lowercase()) {
            return 0;
        }
        let folded = text.normalized();
        let weight = match source {
            "title" => 2,
            "user_message" => 4,
            "assistant_message" => 3,
            _ => 1,
        };
        let mut hits = 0;
        for term in &self.terms {
            if folded.contains(term) {
                self.matched.insert(term.clone());
                hits += 1;
                self.score = self.score.saturating_add(weight);
            }
        }
        if hits > 0 {
            self.sources.insert(source.into());
        }
        hits
    }
    pub fn matches(&self) -> bool {
        self.required.iter().all(|term| self.matched.contains(term))
    }
    pub fn facts(self) -> (u32, Vec<String>, Vec<String>) {
        (
            self.score,
            self.matched.into_iter().collect(),
            self.sources.into_iter().collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excerpts_are_character_bounded_with_unicode_case_folding_and_no_controls() {
        let input = format!("{}Cesium camera\n{}", "İ".repeat(600), "🪐".repeat(600));
        let (excerpt, truncated) = bounded_excerpt(&input, &["cesium".into()], 320);
        assert!(truncated);
        assert!(excerpt.contains("Cesium"));
        assert!(excerpt.chars().count() <= 320);
        assert!(!excerpt.chars().any(char::is_control));
    }
    #[test]
    fn secret_records_do_not_contribute_matches_and_lexical_terms_cross_messages() {
        let p = SearchParameters {
            structured: None,
            query: "Earth CESIUM earth".into(),
            include_archived: true,
            include_content: true,
            project_label: None,
            purpose: SearchPurpose::Diagnostic,
            page_size: 20,
            excerpts: true,
        };
        let mut rank = SearchRelevance::new(&p);
        assert_eq!(rank.observe("user_message", "Cesium password=hidden"), 0);
        rank.observe("user_message", "Earth renders slowly");
        assert!(!rank.matches());
        rank.observe("assistant_message", "Cesium camera");
        assert!(rank.matches());
        let (score, terms, sources) = rank.facts();
        assert_eq!(score, 7);
        assert_eq!(terms, ["cesium", "earth"]);
        assert_eq!(sources.len(), 2);
    }
}
