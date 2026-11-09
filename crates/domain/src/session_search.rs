//! Bounded read evidence only. No caller, ownership, approval or effect authority.
use crate::{ArtifactId, SessionMetadata};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SEARCH_BATCH: usize = 100;
pub const SEARCH_INVENTORY: usize = 10_000;
pub const TRANSCRIPT_BYTES: usize = 1024 * 1024;
pub const RECORD_BYTES: usize = 256 * 1024;
pub const SEARCH_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchPurpose {
    #[default]
    Diagnostic,
    Management,
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
            .map(str::to_lowercase)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// Constructed from independently controlled launch registration, never MCP input.
#[derive(Debug, Clone)]
pub struct ManagementIdentity {
    pub identity: ArtifactId,
    pub native_session_id: String,
    pub revision: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub continuation: SearchContinuation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReference {
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

pub fn search_scope(
    parameters: &SearchParameters,
    root: &std::path::Path,
    excluded: &[ArtifactId],
    revision: &str,
) -> String {
    let bytes = serde_json::to_vec(&(parameters, root, excluded, revision))
        .expect("search values serialize");
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
    let lower = text.to_lowercase();
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
    let lower = text.to_lowercase();
    let offset = terms
        .iter()
        .filter_map(|term| lower.find(term))
        .min()
        .unwrap_or(0);
    // Map the lowercased byte offset back to original characters in one pass,
    // including expanding case folds, without splitting UTF-8.
    let mut lower_bytes = 0;
    let at = chars
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            let here = lower_bytes;
            lower_bytes += c.to_lowercase().map(char::len_utf8).sum::<usize>();
            (here >= offset).then_some(i)
        })
        .unwrap_or(0);
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
        Self {
            required: parameters.terms(),
            terms: parameters.ranking_terms(),
            matched: BTreeSet::new(),
            sources: BTreeSet::new(),
            score: 0,
        }
    }
    pub fn observe(&mut self, source: &str, text: &str) -> usize {
        if !safe_conversation_text(text) {
            return 0;
        }
        let lower = text.to_lowercase();
        let weight = match source {
            "title" => 2,
            "user_message" => 4,
            "assistant_message" => 3,
            _ => 1,
        };
        let mut hits = 0;
        for term in &self.terms {
            if lower.contains(term) {
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
