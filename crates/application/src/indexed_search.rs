//! Optional private derived cache over the existing read port. No source writer.
use crate::QueryControl;
use agentmender_domain::*;
use agentmender_persistence::{IndexDocument, SearchTextIndex};
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

pub type SharedSearchIndex = Arc<Mutex<SearchTextIndex>>;
pub struct IndexedSearchReader<'a> {
    pub reader: &'a dyn SessionSearchReadPort,
    pub index: SharedSearchIndex,
    pub root: &'a Path,
}
fn unavailable() -> ProviderError {
    ProviderError::CapabilityUnavailable("private_search_index_unavailable".into())
}
fn encode<T: serde::Serialize>(value: &T) -> Result<String, ProviderError> {
    serde_json::to_string(value).map_err(|_| unavailable())
}
fn source_key(metadata: &SessionMetadata) -> Result<String, ProviderError> {
    encode(&metadata.identity)
}
impl SessionSearchReadPort for IndexedSearchReader<'_> {
    fn search_sessions(
        &self,
        request: &SearchReadRequest,
    ) -> Result<SessionSearchPage, ProviderError> {
        let mut request = request.clone();
        // Cached facts never enter from a model. The provider still checks the
        // fresh inventory, exclusions, source revision and full query scope.
        request.cached.clear();
        if request.parameters.reads_content() {
            let index = self.index.lock().map_err(|_| unavailable())?;
            if !index.status().map_err(|_| unavailable())?.persistent {
                return self.reader.search_sessions(&request);
            }
            let cache_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            let mut cached_bytes = 0usize;
            for stored in index.source_states(1000).map_err(|_| unavailable())? {
                if !stored.exhausted {
                    continue;
                }
                let metadata: SessionMetadata =
                    serde_json::from_str(&stored.metadata).map_err(|_| unavailable())?;
                if metadata.identity.state_root != self.root
                    || request.excluded.contains(&metadata.identity)
                    || request
                        .parameters
                        .structured
                        .as_ref()
                        .is_some_and(|q| q.filter.metadata_truth(&metadata) == Truth::False)
                {
                    continue;
                }
                let mut rank = SearchRelevance::new(&request.parameters);
                let mut observation = QueryObservation::default();
                let mut evidence = Vec::new();
                let invalid = std::cell::Cell::new(false);
                let mut current_ordinal = None;
                let mut text = String::new();
                let mut first_evidence: Option<ConversationEvidence> = None;
                let terms = request.parameters.ranking_terms();
                let mut flush = |text: &mut String, first: &mut Option<ConversationEvidence>| {
                    if let Some(mut e) = first.take() {
                        if e.position
                            .as_ref()
                            .is_none_or(|p| p.chunk_total != text.len())
                        {
                            invalid.set(true);
                        }
                        if invalid.get() {
                            text.clear();
                            return;
                        }
                        if let Some(filter) =
                            request.parameters.structured.as_ref().map(|q| &q.filter)
                        {
                            filter.observe(
                                &mut observation,
                                &QueryText {
                                    text,
                                    timestamp: e
                                        .position
                                        .as_ref()
                                        .and_then(|p| p.timestamp.as_deref())
                                        .and_then(parse_message_timestamp),
                                },
                            );
                        }
                        if rank.observe(&e.source, text) > 0 {
                            let (excerpt, truncated) = bounded_excerpt(text, &terms, 320);
                            e.excerpt = request.parameters.excerpts.then_some(excerpt);
                            e.truncated = truncated;
                            if evidence.len() < 2 {
                                evidence.push(e);
                            }
                        }
                    }
                    text.clear();
                };
                let finished = index
                    .visit_documents(&stored.source, |ordinal, body, reference| {
                        cached_bytes = cached_bytes.saturating_add(body.len());
                        if cached_bytes > SEARCH_BYTES
                            || std::time::Instant::now() >= cache_deadline
                        {
                            return false;
                        }
                        if current_ordinal != Some(ordinal) {
                            flush(&mut text, &mut first_evidence);
                            current_ordinal = Some(ordinal);
                        }
                        let decoded = serde_json::from_str::<ConversationEvidence>(reference);
                        let Ok(chunk) = decoded else {
                            invalid.set(true);
                            return true;
                        };
                        let valid = chunk.source_fingerprint == stored.revision
                            && chunk.record_index as u64 == ordinal
                            && chunk.position.as_ref().is_some_and(|p| {
                                p.ordinal as u64 == ordinal
                                    && p.chunk_offset == text.len()
                                    && p.chunk_total <= 8 * 1024 * 1024
                            })
                            && first_evidence.as_ref().is_none_or(|first| {
                                first.source == chunk.source
                                    && first.source_bytes == chunk.source_bytes
                                    && first
                                        .position
                                        .as_ref()
                                        .zip(chunk.position.as_ref())
                                        .is_some_and(|(a, b)| {
                                            a.byte_offset == b.byte_offset
                                                && a.byte_length == b.byte_length
                                                && a.chunk_total == b.chunk_total
                                        })
                            });
                        if !valid || text.len().saturating_add(body.len()) > 8 * 1024 * 1024 {
                            invalid.set(true);
                            return true;
                        }
                        text.push_str(body);
                        if first_evidence.is_none() {
                            first_evidence = Some(chunk);
                        }
                        true
                    })
                    .map_err(|_| unavailable())?;
                flush(&mut text, &mut first_evidence);
                if !finished {
                    break;
                }
                if invalid.get() {
                    continue;
                }
                let (score, matched_terms, match_sources) = rank.facts();
                request.cached.push(CachedSessionSearch {
                    native_session_id: metadata.native_session_id,
                    source_fingerprint: stored.revision,
                    query_scope: request.continuation.scope.clone(),
                    score,
                    matched_terms,
                    match_sources,
                    evidence,
                    query_observation: observation,
                    content_complete: stored.complete,
                });
                if request.cached.len() == 10 {
                    break;
                }
            }
        }
        self.reader.search_sessions(&request)
    }
    fn session_context(
        &self,
        r: &ContextReference,
        e: &[ArtifactId],
    ) -> Result<ConversationEvidence, ProviderError> {
        self.reader.session_context(r, e)
    }
    fn session_context_window(
        &self,
        r: &ContextReference,
        o: &ContextOptions,
        e: &[ArtifactId],
    ) -> Result<ContextWindow, ProviderError> {
        self.reader.session_context_window(r, o, e)
    }
    fn transcript_page(
        &self,
        r: &ContextReference,
        c: Option<TranscriptCheckpoint>,
        e: &[ArtifactId],
    ) -> Result<TranscriptPage, ProviderError> {
        self.reader.transcript_page(r, c, e)
    }
}

#[derive(Debug, Default, serde::Serialize)]
pub struct IndexBuildReport {
    pub sessions_observed: usize,
    pub sessions_reused: usize,
    pub pages_indexed: usize,
    pub bytes_read: usize,
    pub unsupported_sessions: usize,
    pub inventory_exhausted: bool,
    pub pending_sources: usize,
    pub partial_sources: usize,
}
/// Explicit CLI operation. It resumes source checkpoints and validates unchanged
/// sources; limited runs restart inventory enumeration but reuse completed files.
/// No model-provided path, SQL or caller identity enters this operation.
pub fn build_search_index(
    reader: &dyn SessionSearchReadPort,
    index: &mut SearchTextIndex,
    root: &Path,
    excluded: &[ArtifactId],
    max_pages: usize,
    control: QueryControl,
) -> Result<IndexBuildReport, ProviderError> {
    let mut report = IndexBuildReport::default();
    let mut continuation = SearchContinuation::default();
    let mut seen = Vec::new();
    loop {
        control.check().map_err(|_| unavailable())?;
        let inventory = reader.search_sessions(&SearchReadRequest {
            parameters: SearchParameters {
                query: String::new(),
                include_archived: true,
                include_content: false,
                project_label: None,
                purpose: SearchPurpose::Diagnostic,
                page_size: 20,
                excerpts: false,
                structured: None,
            },
            excluded: excluded.to_vec(),
            management_session: None,
            discovery_session: None,
            continuation: continuation.clone(),
            cached: vec![],
        })?;
        for hit in inventory.sessions {
            if hit.metadata.identity.state_root != root || excluded.contains(&hit.metadata.identity)
            {
                return Err(unavailable());
            }
            report.sessions_observed += 1;
            let source = source_key(&hit.metadata)?;
            seen.push(source.clone());
            let existing = index.source_state(&source).map_err(|_| unavailable())?;
            let mut checkpoint = existing
                .as_ref()
                .and_then(|s| s.checkpoint.as_ref())
                .and_then(|s| serde_json::from_str::<TranscriptCheckpoint>(s).ok());
            let mut reference = ContextReference {
                decoder_progress: None,
                native_session_id: hit.metadata.native_session_id.clone(),
                local_id: hit.metadata.identity.local_id.clone(),
                record_index: 1,
                source_fingerprint: String::new(),
                source_bytes: 0,
                purpose: SearchPurpose::Diagnostic,
                binding_revision: "diagnostic_no_caller_guarantee".into(),
                position: None,
            };
            if let Some(stored) = &existing {
                // An EOF checkpoint is retained even for incomplete formats so
                // freshness validation can avoid rereading unchanged content.
                reference.source_fingerprint = stored.revision.clone();
                if let Some(c) = &checkpoint {
                    reference.record_index = c.record_index;
                }
                if stored.exhausted {
                    match reader.transcript_page(&reference, checkpoint.clone(), excluded) {
                        Ok(page) if page.source_revision == stored.revision => {
                            report.sessions_reused += 1;
                            report.partial_sources += usize::from(!stored.complete);
                            report.bytes_read += page.bytes_read;
                            continue;
                        }
                        _ => {
                            checkpoint = None;
                            reference.source_fingerprint.clear();
                        }
                    }
                }
            }
            loop {
                control.check().map_err(|_| unavailable())?;
                if report.pages_indexed >= max_pages {
                    report.pending_sources += 1;
                    return Ok(report);
                }
                let start = checkpoint.as_ref().map_or(0, |c| c.byte_offset);
                let page = match reader.transcript_page(&reference, checkpoint.clone(), excluded) {
                    Ok(p) => p,
                    Err(ProviderError::InvalidScope(_)) if checkpoint.is_some() => {
                        checkpoint = None;
                        reference.source_fingerprint.clear();
                        continue;
                    }
                    Err(_) => {
                        index.remove_source(&source).map_err(|_| unavailable())?;
                        report.unsupported_sessions += 1;
                        break;
                    }
                };
                report.pages_indexed += 1;
                report.bytes_read += page.bytes_read;
                let revision = page.source_revision.clone();
                let refs = page
                    .messages
                    .iter()
                    .map(|m| encode(&m.evidence))
                    .collect::<Result<Vec<_>, _>>()?;
                let docs: Vec<_> = page
                    .messages
                    .iter()
                    .zip(&refs)
                    .map(|(m, r)| IndexDocument {
                        ordinal: m.evidence.record_index as u64,
                        chunk: m
                            .evidence
                            .position
                            .as_ref()
                            .map_or(0, |p| p.chunk_offset as u32),
                        text: &m.text,
                        reference: r,
                    })
                    .collect();
                let next_offset = page.end.byte_offset;
                index
                    .append_page(&source, &revision, start, next_offset, page.complete, &docs)
                    .map_err(|_| unavailable())?;
                let exhausted = page.next.is_none();
                checkpoint = Some(page.end);
                let serialized = checkpoint.as_ref().map(encode).transpose()?;
                index
                    .set_source_state(
                        &source,
                        &encode(&hit.metadata)?,
                        serialized.as_deref(),
                        exhausted,
                    )
                    .map_err(|_| unavailable())?;
                reference.source_fingerprint = revision;
                if exhausted {
                    report.partial_sources += usize::from(!page.complete);
                    break;
                }
            }
        }
        match inventory.next {
            Some(next) => continuation = next,
            None => {
                report.inventory_exhausted = !inventory
                    .coverage
                    .reasons
                    .iter()
                    .any(|r| r == "metadata_inventory_limit");
                if report.inventory_exhausted {
                    index
                        .retain_sources(&seen, true)
                        .map_err(|_| unavailable())?;
                }
                break;
            }
        }
    }
    Ok(report)
}
