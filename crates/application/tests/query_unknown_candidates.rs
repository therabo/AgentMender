//! Unknown negative-query candidates are evidence for inspection, not matches.
use agentmender_application::{
    ApplicationError, QueryControl, session_search::SessionSearchService,
};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::{path::Path, sync::Mutex};

struct Reader(Mutex<SessionSearchPage>);
impl SessionSearchReadPort for Reader {
    fn search_sessions(&self, _: &SearchReadRequest) -> Result<SessionSearchPage, ProviderError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn session_context(
        &self,
        _: &ContextReference,
        _: &[ArtifactId],
    ) -> Result<ConversationEvidence, ProviderError> {
        unreachable!()
    }
}
fn hit() -> SessionSearchHit {
    SessionSearchHit {
        source_coverage: None,
        metadata: SessionMetadata {
            identity: ArtifactId {
                provider: ProviderKind::Codex,
                state_root: "/provider".into(),
                class: GraphNodeKind::Session,
                local_id: EntityId::new("codex:session:unknown").unwrap(),
            },
            native_session_id: "unknown".into(),
            title: None,
            project_label: None,
            created_at: None,
            last_active_at: None,
            state: SessionState::Active,
            producer_version_matches: false,
            producer_version: None,
            parent_native_session_id: None,
        },
        score: 0,
        matched_terms: vec![],
        match_sources: vec![],
        evidence: vec![],
        content_status: "unknown".into(),
        content_complete: false,
        query_observation: Default::default(),
    }
}
fn parameters() -> SearchParameters {
    SearchParameters {
        query: String::new(),
        include_archived: true,
        include_content: false,
        project_label: None,
        purpose: SearchPurpose::Diagnostic,
        page_size: 20,
        excerpts: false,
        structured: Some(StructuredQuery {
            filter: QueryExpr::Not {
                arg: Box::new(QueryExpr::Project {
                    values: vec!["MISSION1".into()],
                    source: ProjectSource::ObservedLabel,
                }),
            },
            mode: SearchMode::Exhaustive,
            sort: SearchSort::Identity,
            coverage: CoverageRequirement::BestEffort,
        }),
    }
}
#[test]
fn unknown_project_is_returned_separately_and_cannot_be_promoted_or_leak_exclusions() {
    let reader = Reader(Mutex::new(SessionSearchPage {
        sessions: vec![],
        unresolved: vec![hit()],
        coverage: SearchCoverage {
            eligible_observed: 1,
            sessions_inspected: 1,
            unresolved_sessions: 1,
            ..Default::default()
        },
        inventory_fingerprint: "inventory".into(),
        batch_fingerprint: "batch".into(),
        next: None,
    }));
    let service = SessionSearchService {
        reader: &reader,
        provider_root: Path::new("/provider"),
        provider: ProviderKind::Codex,
        management: None,
        registered: &[],
        instance: "fixture",
    };
    let result = service
        .search(parameters(), None, QueryControl::default())
        .unwrap();
    assert!(result.page.sessions.is_empty());
    assert_eq!(result.page.unresolved.len(), 1);
    let excluded = [hit().metadata.identity];
    let protected = SessionSearchService {
        registered: &excluded,
        ..service
    };
    assert!(matches!(
        protected.search(parameters(), None, QueryControl::default()),
        Err(ApplicationError::InvalidSearch)
    ));
    {
        let mut page = reader.0.lock().unwrap();
        page.sessions = std::mem::take(&mut page.unresolved);
    }
    assert!(matches!(
        service.search(parameters(), None, QueryControl::default()),
        Err(ApplicationError::InvalidSearch)
    ));
    {
        let mut page = reader.0.lock().unwrap();
        page.sessions.clear();
        let mut known = hit();
        known.metadata.project_label = Some("Other".into());
        page.unresolved = vec![known];
    }
    assert!(matches!(
        service.search(parameters(), None, QueryControl::default()),
        Err(ApplicationError::InvalidSearch)
    ));
    let mut legacy_unknown = parameters();
    legacy_unknown.query = "Cesium".into();
    legacy_unknown.include_content = true;
    legacy_unknown.structured.as_mut().unwrap().filter = QueryExpr::Provider {
        values: vec![ProviderKind::Codex],
    };
    {
        let mut page = reader.0.lock().unwrap();
        page.unresolved = vec![hit()];
    }
    assert_eq!(
        service
            .search(legacy_unknown.clone(), None, QueryControl::default())
            .unwrap()
            .page
            .unresolved
            .len(),
        1
    );
    reader.0.lock().unwrap().unresolved[0].content_complete = true;
    assert!(matches!(
        service.search(legacy_unknown, None, QueryControl::default()),
        Err(ApplicationError::InvalidSearch)
    ));
}
