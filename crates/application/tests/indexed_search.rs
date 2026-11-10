//! Deterministic read-port fixtures. No provider-owned or private user data.
use agentmender_application::{
    QueryControl,
    indexed_search::{IndexedSearchReader, build_search_index},
};
use agentmender_domain::*;
use agentmender_persistence::{IndexDocument, SearchTextIndex, open_search_index};
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct Chunk {
    ordinal: usize,
    offset: usize,
    text: String,
}
#[derive(Clone)]
struct Source {
    revision: String,
    pages: Vec<Vec<Chunk>>,
    complete: bool,
    readable: bool,
}
struct Reader {
    root: PathBuf,
    sources: Mutex<BTreeMap<String, Source>>,
    read_offsets: Mutex<Vec<(String, u64)>>,
    replacements: Mutex<BTreeMap<(String, u64), Source>>,
    searches: Mutex<Vec<SearchReadRequest>>,
}
impl Reader {
    fn metadata(&self, id: &str) -> SessionMetadata {
        SessionMetadata {
            identity: ArtifactId {
                provider: ProviderKind::Codex,
                state_root: self.root.clone(),
                class: GraphNodeKind::Session,
                local_id: EntityId::new(format!("codex:session:{id}")).unwrap(),
            },
            native_session_id: id.into(),
            title: Some("An unrelated title".into()),
            project_label: Some("AgentMender".into()),
            created_at: Some(100),
            last_active_at: Some(200),
            state: SessionState::Active,
            producer_version_matches: false,
            producer_version: Some("fixture".into()),
            parent_native_session_id: None,
        }
    }
    fn put(&self, id: &str, revision: &str, pages: &[&[(usize, usize, &str)]], complete: bool) {
        self.sources.lock().unwrap().insert(
            id.into(),
            Source {
                revision: revision.into(),
                pages: pages
                    .iter()
                    .map(|page| {
                        page.iter()
                            .map(|(ordinal, offset, text)| Chunk {
                                ordinal: *ordinal,
                                offset: *offset,
                                text: (*text).into(),
                            })
                            .collect()
                    })
                    .collect(),
                complete,
                readable: true,
            },
        );
    }
}
fn unavailable() -> ProviderError {
    ProviderError::CapabilityUnavailable("fixture unreadable source".into())
}
impl SessionSearchReadPort for Reader {
    fn search_sessions(
        &self,
        request: &SearchReadRequest,
    ) -> Result<SessionSearchPage, ProviderError> {
        self.searches.lock().unwrap().push(request.clone());
        let sessions: Vec<_> = self
            .sources
            .lock()
            .unwrap()
            .keys()
            .map(|id| self.metadata(id))
            .filter(|m| !request.excluded.contains(&m.identity))
            .map(|metadata| SessionSearchHit {
                source_coverage: None,
                metadata,
                score: 0,
                matched_terms: vec![],
                match_sources: vec![],
                evidence: vec![],
                content_status: "not_requested".into(),
                query_observation: QueryObservation::default(),
                content_complete: false,
            })
            .collect();
        Ok(SessionSearchPage {
            unresolved: vec![],
            coverage: SearchCoverage {
                eligible_observed: sessions.len(),
                sessions_inspected: sessions.len(),
                ..Default::default()
            },
            sessions,
            inventory_fingerprint: "fixture-inventory".into(),
            batch_fingerprint: "fixture-batch".into(),
            next: None,
        })
    }
    fn session_context(
        &self,
        _: &ContextReference,
        _: &[ArtifactId],
    ) -> Result<ConversationEvidence, ProviderError> {
        Err(unavailable())
    }
    fn transcript_page(
        &self,
        reference: &ContextReference,
        checkpoint: Option<TranscriptCheckpoint>,
        excluded: &[ArtifactId],
    ) -> Result<TranscriptPage, ProviderError> {
        if excluded.contains(&self.metadata(&reference.native_session_id).identity) {
            return Err(unavailable());
        }
        let requested_offset = checkpoint.as_ref().map_or(0, |c| c.byte_offset);
        if let Some(replacement) = self
            .replacements
            .lock()
            .unwrap()
            .remove(&(reference.native_session_id.clone(), requested_offset))
        {
            self.sources
                .lock()
                .unwrap()
                .insert(reference.native_session_id.clone(), replacement);
        }
        let sources = self.sources.lock().unwrap();
        let source = sources
            .get(&reference.native_session_id)
            .filter(|s| s.readable)
            .ok_or_else(unavailable)?;
        if (!reference.source_fingerprint.is_empty()
            && reference.source_fingerprint != source.revision)
            || checkpoint
                .as_ref()
                .is_some_and(|c| c.source_fingerprint != source.revision)
        {
            return Err(ProviderError::InvalidScope("stale_search_evidence".into()));
        }
        let offset = checkpoint.as_ref().map_or(0, |c| c.byte_offset);
        self.read_offsets
            .lock()
            .unwrap()
            .push((reference.native_session_id.clone(), offset));
        let page_index = (offset / 100) as usize;
        let chunks = source.pages.get(page_index).map_or(&[][..], Vec::as_slice);
        let total_bytes = source.pages.len() * 100;
        let messages = chunks
            .iter()
            .map(|chunk| TranscriptMessage {
                text: chunk.text.clone(),
                evidence: ConversationEvidence {
                    source: "user_message".into(),
                    record_index: chunk.ordinal,
                    position: Some(MessagePosition {
                        byte_offset: 1,
                        byte_length: total_bytes.saturating_sub(1) as u64,
                        ordinal: chunk.ordinal,
                        message_id: Some(format!("m{}", chunk.ordinal)),
                        timestamp: Some("2026-10-15T12:00:00Z".into()),
                        chunk_offset: chunk.offset,
                        chunk_total: source
                            .pages
                            .iter()
                            .flatten()
                            .filter(|c| c.ordinal == chunk.ordinal)
                            .map(|c| c.offset + c.text.len())
                            .max()
                            .unwrap_or(0),
                        ..Default::default()
                    }),
                    excerpt: Some(chunk.text.clone()),
                    truncated: false,
                    source_fingerprint: source.revision.clone(),
                    source_bytes: total_bytes,
                    format: "fixture-supported".into(),
                },
            })
            .collect();
        let exhausted = page_index + 1 >= source.pages.len();
        let end = TranscriptCheckpoint {
            native_session_id: reference.native_session_id.clone(),
            source_fingerprint: source.revision.clone(),
            byte_offset: if exhausted {
                total_bytes as u64
            } else {
                (page_index as u64 + 1) * 100
            },
            // Counts source records including ignored trailing records.
            record_index: if exhausted { 11 } else { page_index + 1 },
            history_mode: "fixture-supported".into(),
            partial: !source.complete,
            ..Default::default()
        };
        Ok(TranscriptPage {
            work: Default::default(),
            physical_eof: exhausted,
            end: end.clone(),
            messages,
            next: (!exhausted).then_some(end),
            complete: exhausted && source.complete,
            source_revision: source.revision.clone(),
            bytes_read: if chunks.is_empty() { 1 } else { 100 },
        })
    }
}
struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    index_path: PathBuf,
    reader: Reader,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("provider");
        std::fs::create_dir(&root).unwrap();
        let index_path = directory.path().join("private-index");
        Self {
            reader: Reader {
                root: root.clone(),
                sources: Mutex::new(BTreeMap::new()),
                read_offsets: Mutex::new(vec![]),
                replacements: Mutex::new(BTreeMap::new()),
                searches: Mutex::new(vec![]),
            },
            _directory: directory,
            root,
            index_path,
        }
    }
    fn index(&self) -> SearchTextIndex {
        open_search_index(&self.index_path, std::slice::from_ref(&self.root)).unwrap()
    }
    fn build(
        &self,
        index: &mut SearchTextIndex,
        pages: usize,
    ) -> agentmender_application::indexed_search::IndexBuildReport {
        build_search_index(
            &self.reader,
            index,
            &self.root,
            &[],
            pages,
            QueryControl::default(),
        )
        .unwrap()
    }
}
fn structured(filter: QueryExpr) -> SearchReadRequest {
    SearchReadRequest {
        parameters: SearchParameters {
            structured: Some(StructuredQuery {
                filter,
                mode: SearchMode::Exhaustive,
                sort: SearchSort::Identity,
                coverage: CoverageRequirement::BestEffort,
            }),
            query: String::new(),
            include_content: true,
            include_archived: true,
            project_label: None,
            purpose: SearchPurpose::Diagnostic,
            page_size: 20,
            excerpts: true,
        },
        continuation: SearchContinuation {
            scope: "bound-fixture-query".into(),
            ..Default::default()
        },
        excluded: vec![],
        management_session: None,
        discovery_session: None,
        cached: vec![],
    }
}
fn text(value: &str, kind: TextMatch) -> QueryExpr {
    QueryExpr::Text {
        value: value.into(),
        kind,
        alternatives: vec![],
    }
}
fn cached(
    fixture: &Fixture,
    index: Arc<Mutex<SearchTextIndex>>,
    request: &SearchReadRequest,
) -> Vec<CachedSessionSearch> {
    IndexedSearchReader {
        reader: &fixture.reader,
        index,
        root: &fixture.root,
    }
    .search_sessions(request)
    .unwrap();
    fixture
        .reader
        .searches
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .cached
        .clone()
}
fn source_key(reader: &Reader, id: &str) -> String {
    serde_json::to_string(&reader.metadata(id).identity).unwrap()
}

#[test]
fn bounded_build_resumes_chunks_and_reuses_canonical_eof_without_duplicate_documents() {
    let f = Fixture::new();
    f.reader
        .put("a", "r1", &[&[(2, 0, "Ces")], &[(2, 3, "ium")]], true);
    let mut index = f.index();
    let first = f.build(&mut index, 1);
    assert_eq!(first.pages_indexed, 1);
    assert_eq!(first.pending_sources, 1);
    assert!(!first.inventory_exhausted);
    assert_eq!(index.status().unwrap().documents, 1);
    let second = f.build(&mut index, 5);
    assert_eq!(second.pages_indexed, 1);
    assert!(second.inventory_exhausted);
    assert_eq!(index.status().unwrap().documents, 2);
    let state = index
        .source_state(&source_key(&f.reader, "a"))
        .unwrap()
        .unwrap();
    assert!(state.complete && state.exhausted);
    let end: TranscriptCheckpoint =
        serde_json::from_str(state.checkpoint.as_deref().unwrap()).unwrap();
    assert_eq!(end.byte_offset, 200);
    assert_eq!(end.record_index, 11);
    assert_eq!(end.history_mode, "fixture-supported");
    let third = f.build(&mut index, 5);
    assert_eq!(third.pages_indexed, 0);
    assert_eq!(third.sessions_reused, 1);
    assert_eq!(index.status().unwrap().documents, 2);
    assert_eq!(
        *f.reader.read_offsets.lock().unwrap(),
        [("a".into(), 0), ("a".into(), 100), ("a".into(), 200)]
    );
}

#[test]
fn cached_exact_phrase_and_negation_reassemble_message_chunks_before_evaluation() {
    let f = Fixture::new();
    f.reader.put(
        "a",
        "r1",
        &[&[(2, 0, "Ces")], &[(2, 3, "ium rendering")]],
        true,
    );
    let mut index = f.index();
    f.build(&mut index, 10);
    let index = Arc::new(Mutex::new(index));
    for (filter, expected) in [
        (text("Cesium rendering", TextMatch::Exact), Truth::True),
        (text("Cesium rendering", TextMatch::Phrase), Truth::True),
        (
            QueryExpr::Not {
                arg: Box::new(text("Cesium", TextMatch::Contains)),
            },
            Truth::False,
        ),
        (
            QueryExpr::Not {
                arg: Box::new(text("Docker", TextMatch::Contains)),
            },
            Truth::True,
        ),
    ] {
        let request = structured(filter.clone());
        let cached = cached(&f, index.clone(), &request);
        assert_eq!(cached.len(), 1);
        assert!(cached[0].content_complete);
        assert_eq!(
            filter.evaluate_observed(
                &f.reader.metadata("a"),
                &cached[0].query_observation,
                cached[0].content_complete
            ),
            expected
        );
    }
}

#[test]
fn incomplete_source_cache_never_proves_negative_content() {
    let f = Fixture::new();
    f.reader
        .put("a", "r1", &[&[(2, 0, "Database indexing")]], false);
    let mut index = f.index();
    f.build(&mut index, 10);
    let filter = QueryExpr::Not {
        arg: Box::new(text("Cesium", TextMatch::Contains)),
    };
    let cached = cached(&f, Arc::new(Mutex::new(index)), &structured(filter.clone()));
    assert_eq!(cached.len(), 1);
    assert!(!cached[0].content_complete);
    assert_eq!(
        filter.evaluate_observed(
            &f.reader.metadata("a"),
            &cached[0].query_observation,
            cached[0].content_complete
        ),
        Truth::Unknown
    );
}

#[test]
fn source_replacement_rebuilds_postings_and_replaces_old_private_text() {
    let f = Fixture::new();
    f.reader
        .put("a", "r1", &[&[(2, 0, "Cesium old proposal")]], true);
    let mut index = f.index();
    f.build(&mut index, 10);
    assert_eq!(index.search("Cesium", 10).unwrap().len(), 1);
    f.reader
        .put("a", "r2", &[&[(2, 0, "PostgreSQL final decision")]], true);
    let result = f.build(&mut index, 10);
    assert_eq!(result.sessions_reused, 0);
    assert_eq!(result.pages_indexed, 1);
    assert!(index.search("Cesium", 10).unwrap().is_empty());
    assert_eq!(index.search("PostgreSQL", 10).unwrap().len(), 1);
    let mut bodies = Vec::new();
    index
        .visit_documents(&source_key(&f.reader, "a"), |_, text, _| {
            bodies.push(text.to_owned());
            true
        })
        .unwrap();
    assert_eq!(bodies, ["PostgreSQL final decision"]);
}

#[test]
fn missing_or_unreadable_source_invalidates_private_derived_text() {
    let f = Fixture::new();
    f.reader
        .put("a", "r1", &[&[(2, 0, "Cesium old proposal")]], true);
    let mut index = f.index();
    f.build(&mut index, 10);
    f.reader
        .sources
        .lock()
        .unwrap()
        .get_mut("a")
        .unwrap()
        .readable = false;
    let result = f.build(&mut index, 10);
    assert_eq!(result.unsupported_sessions, 1);
    assert!(index.search("Cesium", 10).unwrap().is_empty());
    assert!(
        index
            .source_state(&source_key(&f.reader, "a"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn complete_inventory_removes_deleted_sessions_and_preserves_other_sources() {
    let f = Fixture::new();
    f.reader.put("a", "r1", &[&[(2, 0, "Cesium")]], true);
    f.reader.put("b", "r1", &[&[(2, 0, "Orekit")]], true);
    let mut index = f.index();
    f.build(&mut index, 10);
    f.reader.sources.lock().unwrap().remove("a");
    f.build(&mut index, 10);
    assert!(index.search("Cesium", 10).unwrap().is_empty());
    assert_eq!(index.search("Orekit", 10).unwrap().len(), 1);
    assert_eq!(index.status().unwrap().sources, 1);
}

#[test]
fn memory_contentless_index_cannot_supply_empty_complete_cache_for_negative_query() {
    let f = Fixture::new();
    f.reader.put("a", "r1", &[&[(2, 0, "Cesium")]], true);
    let mut index = SearchTextIndex::memory().unwrap();
    f.build(&mut index, 10);
    let request = structured(QueryExpr::Not {
        arg: Box::new(text("Cesium", TextMatch::Contains)),
    });
    assert!(cached(&f, Arc::new(Mutex::new(index)), &request).is_empty());
}

#[test]
fn malformed_cached_reference_cannot_turn_unobserved_text_into_verified_negative() {
    let f = Fixture::new();
    f.reader.put("a", "r1", &[&[(2, 0, "Cesium")]], true);
    let mut index = f.index();
    let key = source_key(&f.reader, "a");
    index
        .append_page(
            &key,
            "r1",
            0,
            100,
            true,
            &[IndexDocument {
                ordinal: 2,
                chunk: 0,
                text: "Cesium",
                reference: "corrupt-json",
            }],
        )
        .unwrap();
    index
        .set_source_state(
            &key,
            &serde_json::to_string(&f.reader.metadata("a")).unwrap(),
            None,
            true,
        )
        .unwrap();
    let shared = Arc::new(Mutex::new(index));
    let result = IndexedSearchReader {
        reader: &f.reader,
        index: shared,
        root: &f.root,
    }
    .search_sessions(&structured(QueryExpr::Not {
        arg: Box::new(text("Cesium", TextMatch::Contains)),
    }));
    // Refusal or a deliberate cache miss/fallback is safe; empty complete
    // observations would incorrectly establish a verified negative.
    if result.is_ok() {
        assert!(
            f.reader
                .searches
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .cached
                .is_empty()
        );
    }
}

#[test]
fn protected_sources_never_enter_index_or_cache() {
    let f = Fixture::new();
    f.reader.put("a", "r1", &[&[(2, 0, "Cesium")]], true);
    let protected = f.reader.metadata("a").identity;
    let mut index = f.index();
    let result = build_search_index(
        &f.reader,
        &mut index,
        &f.root,
        std::slice::from_ref(&protected),
        10,
        QueryControl::default(),
    )
    .unwrap();
    assert_eq!(result.sessions_observed, 0);
    assert_eq!(index.status().unwrap().sources, 0);
    f.build(&mut index, 10);
    let mut request = structured(text("Cesium", TextMatch::Contains));
    request.excluded.push(protected);
    assert!(cached(&f, Arc::new(Mutex::new(index)), &request).is_empty());
}

#[test]
fn cancelled_build_performs_no_provider_read_or_private_write() {
    let f = Fixture::new();
    f.reader.put("a", "r1", &[&[(2, 0, "Cesium")]], true);
    let mut index = f.index();
    let control = QueryControl::default();
    control.cancel();
    assert!(build_search_index(&f.reader, &mut index, &f.root, &[], 10, control).is_err());
    assert!(f.reader.searches.lock().unwrap().is_empty());
    assert_eq!(index.status().unwrap().documents, 0);
}

#[test]
fn missing_message_chunk_cannot_establish_a_cached_negative() {
    let f = Fixture::new();
    f.reader
        .put("a", "r1", &[&[(2, 0, "Ces"), (2, 3, "ium")]], true);
    let mut index = f.index();
    f.build(&mut index, 10);
    let source = source_key(&f.reader, "a");
    let metadata = serde_json::to_string(&f.reader.metadata("a")).unwrap();
    let reference = ConversationEvidence {
        source: "user_message".into(),
        record_index: 2,
        position: Some(MessagePosition {
            byte_offset: 1,
            byte_length: 99,
            ordinal: 2,
            chunk_offset: 3,
            chunk_total: 6,
            ..Default::default()
        }),
        excerpt: Some("ium".into()),
        truncated: false,
        source_fingerprint: "r2".into(),
        source_bytes: 100,
        format: "fixture-supported".into(),
    };
    let serialized = serde_json::to_string(&reference).unwrap();
    // A fresh index revision containing only a suffix simulates lost derived
    // chunk data. Its complete bit must not substitute for chunk continuity.
    index
        .append_page(
            &source,
            "r2",
            0,
            100,
            true,
            &[IndexDocument {
                ordinal: 2,
                chunk: 3,
                text: "ium",
                reference: &serialized,
            }],
        )
        .unwrap();
    index
        .set_source_state(&source, &metadata, None, true)
        .unwrap();
    let request = structured(QueryExpr::Not {
        arg: Box::new(text("Cesium", TextMatch::Contains)),
    });
    assert!(cached(&f, Arc::new(Mutex::new(index)), &request).is_empty());
}

#[test]
fn corrupt_later_chunk_reference_cannot_be_hidden_by_a_valid_first_chunk() {
    let f = Fixture::new();
    f.reader
        .put("a", "r1", &[&[(2, 0, "Ces"), (2, 3, "ium")]], true);
    let mut index = f.index();
    let source = source_key(&f.reader, "a");
    let reference = ConversationEvidence {
        source: "user_message".into(),
        record_index: 2,
        position: Some(MessagePosition {
            byte_offset: 1,
            byte_length: 99,
            ordinal: 2,
            chunk_offset: 0,
            chunk_total: 6,
            ..Default::default()
        }),
        excerpt: Some("Ces".into()),
        truncated: true,
        source_fingerprint: "r1".into(),
        source_bytes: 100,
        format: "fixture-supported".into(),
    };
    let serialized = serde_json::to_string(&reference).unwrap();
    index
        .append_page(
            &source,
            "r1",
            0,
            100,
            true,
            &[
                IndexDocument {
                    ordinal: 2,
                    chunk: 0,
                    text: "Ces",
                    reference: &serialized,
                },
                IndexDocument {
                    ordinal: 2,
                    chunk: 3,
                    text: "ium",
                    reference: "broken-later-reference",
                },
            ],
        )
        .unwrap();
    index
        .set_source_state(
            &source,
            &serde_json::to_string(&f.reader.metadata("a")).unwrap(),
            None,
            true,
        )
        .unwrap();
    let request = structured(QueryExpr::Not {
        arg: Box::new(text("Cesium", TextMatch::Contains)),
    });
    assert!(cached(&f, Arc::new(Mutex::new(index)), &request).is_empty());
}

#[test]
fn live_replacement_during_incremental_build_restarts_without_mixing_revisions() {
    let f = Fixture::new();
    f.reader.put(
        "a",
        "r1",
        &[
            &[(2, 0, "Cesium proposal")],
            &[(3, 0, "Old rejected solution")],
        ],
        true,
    );
    let original = f.reader.sources.lock().unwrap().get("a").unwrap().clone();
    f.reader
        .put("a", "r2", &[&[(2, 0, "PostgreSQL final decision")]], true);
    let replacement = f
        .reader
        .sources
        .lock()
        .unwrap()
        .insert("a".into(), original)
        .unwrap();
    f.reader
        .replacements
        .lock()
        .unwrap()
        .insert(("a".into(), 100), replacement);
    let mut index = f.index();
    let result = f.build(&mut index, 10);
    assert_eq!(result.pages_indexed, 2);
    assert!(result.inventory_exhausted);
    assert!(index.search("Cesium", 10).unwrap().is_empty());
    assert!(index.search("rejected", 10).unwrap().is_empty());
    assert_eq!(index.search("PostgreSQL", 10).unwrap().len(), 1);
    let stored = index
        .source_state(&source_key(&f.reader, "a"))
        .unwrap()
        .unwrap();
    assert_eq!(stored.revision, "r2");
    assert!(stored.complete && stored.exhausted);
}

#[derive(Clone)]
struct UnresolvedReader(SessionSearchPage);
impl SessionSearchReadPort for UnresolvedReader {
    fn search_sessions(&self, _: &SearchReadRequest) -> Result<SessionSearchPage, ProviderError> {
        Ok(self.0.clone())
    }
    fn session_context(
        &self,
        _: &ContextReference,
        _: &[ArtifactId],
    ) -> Result<ConversationEvidence, ProviderError> {
        Err(unavailable())
    }
}

#[test]
fn application_revalidates_combined_legacy_and_structured_unknown_partition() {
    let f = Fixture::new();
    let hit = SessionSearchHit {
        source_coverage: None,
        metadata: f.reader.metadata("a"),
        score: 0,
        matched_terms: vec![],
        match_sources: vec![],
        evidence: vec![],
        content_status: "unsupported_or_unreadable".into(),
        query_observation: Default::default(),
        content_complete: false,
    };
    let page = SessionSearchPage {
        sessions: vec![],
        unresolved: vec![hit],
        coverage: SearchCoverage {
            eligible_observed: 1,
            sessions_inspected: 1,
            unresolved_sessions: 1,
            unsupported_sessions: 1,
            ..Default::default()
        },
        inventory_fingerprint: "fixture".into(),
        batch_fingerprint: "fixture".into(),
        next: None,
    };
    let reader = UnresolvedReader(page);
    let service = agentmender_application::session_search::SessionSearchService {
        reader: &reader,
        provider_root: &f.root,
        provider: ProviderKind::Codex,
        management: None,
        apparent_caller: None,
        registered: &[],
        instance: "fixture",
    };
    for filter in [
        None,
        Some(QueryExpr::Provider {
            values: vec![ProviderKind::Codex],
        }),
    ] {
        let mut parameters = structured(text("Cesium", TextMatch::Contains)).parameters;
        parameters.query = "Cesium".into();
        parameters.structured = filter.map(|filter| StructuredQuery {
            filter,
            mode: SearchMode::Exhaustive,
            sort: SearchSort::Identity,
            coverage: CoverageRequirement::BestEffort,
        });
        let result = service
            .search(parameters, None, QueryControl::default())
            .unwrap();
        assert!(result.page.sessions.is_empty());
        assert_eq!(result.page.unresolved.len(), 1);
    }
}
