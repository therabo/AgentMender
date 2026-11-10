use agentmender_application::{
    ApplicationError, QueryControl, session_search::SessionSearchService,
};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

fn artifact(provider: ProviderKind, root: &str, id: &str) -> ArtifactId {
    ArtifactId {
        provider,
        state_root: root.into(),
        class: GraphNodeKind::Session,
        local_id: EntityId::new(id).unwrap(),
    }
}
fn parameters(purpose: SearchPurpose) -> SearchParameters {
    SearchParameters {
        structured: None,
        query: "Cesium camera".into(),
        include_archived: true,
        include_content: true,
        project_label: None,
        purpose,
        page_size: 20,
        excerpts: true,
    }
}
#[derive(Default)]
struct Reader {
    requests: Mutex<Vec<SearchReadRequest>>,
    contexts: Mutex<usize>,
    returned: Mutex<Option<ArtifactId>>,
}
impl SessionSearchReadPort for Reader {
    fn search_sessions(
        &self,
        request: &SearchReadRequest,
    ) -> Result<SessionSearchPage, ProviderError> {
        self.requests.lock().unwrap().push(request.clone());
        let sessions = self
            .returned
            .lock()
            .unwrap()
            .clone()
            .map(|id| SessionSearchHit {
                source_coverage: None,
                query_observation: Default::default(),
                content_complete: false,
                metadata: SessionMetadata {
                    identity: id,
                    native_session_id: "candidate".into(),
                    title: None,
                    project_label: None,
                    created_at: None,
                    last_active_at: None,
                    state: SessionState::Archived,
                    producer_version_matches: false,
                    producer_version: None,
                    parent_native_session_id: None,
                },
                score: 1,
                matched_terms: vec![],
                match_sources: vec![],
                evidence: vec![],
                content_status: "supported".into(),
            })
            .into_iter()
            .collect();
        Ok(SessionSearchPage {
            unresolved: vec![],
            sessions,
            coverage: Default::default(),
            inventory_fingerprint: "snapshot".into(),
            batch_fingerprint: "batch".into(),
            next: Some(SearchContinuation {
                inventory_fingerprint: "snapshot".into(),
                start: 100,
                ..Default::default()
            }),
        })
    }
    fn session_context(
        &self,
        _: &ContextReference,
        _: &[ArtifactId],
    ) -> Result<ConversationEvidence, ProviderError> {
        *self.contexts.lock().unwrap() += 1;
        Err(ProviderError::CapabilityUnavailable("fixture".into()))
    }
}
fn binding() -> ManagementIdentity {
    ManagementIdentity {
        identity: artifact(ProviderKind::Codex, "/provider", "codex:session:management"),
        native_session_id: "management".into(),
        revision: "operator-revision".into(),
    }
}
#[test]
fn missing_identity_refuses_before_any_reader_or_counts_for_every_management_query() {
    let reader = Reader::default();
    let service = SessionSearchService {
        reader: &reader,
        provider: ProviderKind::Codex,
        provider_root: Path::new("/provider"),
        management: None,
        apparent_caller: None,
        registered: &[],
        instance: "fixture-instance",
    };
    for query in ["", "Cesium", "management"] {
        for content in [false, true] {
            for archived in [false, true] {
                let mut p = parameters(SearchPurpose::Management);
                p.query = query.into();
                p.include_content = content;
                p.include_archived = archived;
                p.project_label = Some("project".into());
                assert!(matches!(
                    service.search(p, None, QueryControl::default()),
                    Err(ApplicationError::CallerIdentityRequired { .. })
                ));
            }
        }
    }
    assert!(reader.requests.lock().unwrap().is_empty());
    let result = service
        .search(
            parameters(SearchPurpose::Diagnostic),
            None,
            QueryControl::default(),
        )
        .unwrap();
    assert_eq!(result.binding_revision, "diagnostic_no_caller_guarantee");
    assert_eq!(reader.requests.lock().unwrap().len(), 1);
}
#[test]
fn exact_binding_is_sent_before_provider_limits_and_faulty_adapter_cannot_leak_it() {
    let reader = Reader::default();
    let binding = binding();
    let service = SessionSearchService {
        reader: &reader,
        provider: ProviderKind::Codex,
        provider_root: Path::new("/provider"),
        management: Some(&binding),
        apparent_caller: None,
        registered: &[],
        instance: "fixture",
    };
    service
        .search(
            parameters(SearchPurpose::Management),
            None,
            QueryControl::default(),
        )
        .unwrap();
    let requests = reader.requests.lock().unwrap();
    assert_eq!(
        requests[0].excluded.as_slice(),
        std::slice::from_ref(&binding.identity)
    );
    assert_eq!(
        requests[0].management_session.as_deref(),
        Some("management")
    );
    drop(requests);
    *reader.returned.lock().unwrap() = Some(binding.identity.clone());
    assert!(matches!(
        service.search(
            parameters(SearchPurpose::Management),
            None,
            QueryControl::default()
        ),
        Err(ApplicationError::InvalidSearch)
    ));
    let foreign = ManagementIdentity {
        identity: artifact(ProviderKind::Codex, "/foreign", "management"),
        ..binding.clone()
    };
    let foreign_service = SessionSearchService {
        management: Some(&foreign),
        ..service
    };
    assert!(matches!(
        foreign_service.search(
            parameters(SearchPurpose::Management),
            None,
            QueryControl::default()
        ),
        Err(ApplicationError::CallerIdentityRequired { .. })
    ));
}
#[test]
fn query_filter_binding_changes_and_cancellation_invalidate_before_source_access() {
    let reader = Reader::default();
    let binding = binding();
    let service = SessionSearchService {
        reader: &reader,
        provider: ProviderKind::Codex,
        provider_root: Path::new("/provider"),
        management: Some(&binding),
        apparent_caller: None,
        registered: &[],
        instance: "fixture",
    };
    let page = service
        .search(
            parameters(SearchPurpose::Management),
            None,
            QueryControl::default(),
        )
        .unwrap();
    let next = page.page.next.unwrap();
    let mut changed = parameters(SearchPurpose::Management);
    changed.project_label = Some("other".into());
    assert!(matches!(
        service.search(changed, Some(next.clone()), QueryControl::default()),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    let new_binding = ManagementIdentity {
        revision: "new operator registration".into(),
        ..binding.clone()
    };
    assert!(matches!(
        SessionSearchService {
            management: Some(&new_binding),
            ..service
        }
        .search(
            parameters(SearchPurpose::Management),
            Some(next),
            QueryControl::default()
        ),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    let control = QueryControl::default();
    control.cancel();
    assert!(matches!(
        service.search(parameters(SearchPurpose::Management), None, control),
        Err(ApplicationError::Cancelled)
    ));
    assert_eq!(reader.requests.lock().unwrap().len(), 1);
}
#[test]
fn protected_or_stale_context_never_reaches_reader_and_provider_boundaries_are_neutral() {
    let reader = Reader::default();
    let binding = binding();
    let service = SessionSearchService {
        reader: &reader,
        provider: ProviderKind::Codex,
        provider_root: Path::new("/provider"),
        management: Some(&binding),
        apparent_caller: None,
        registered: &[],
        instance: "fixture",
    };
    let reference = ContextReference {
        decoder_progress: None,
        position: None,
        native_session_id: "management".into(),
        local_id: binding.identity.local_id.clone(),
        record_index: 2,
        source_fingerprint: "fingerprint".into(),
        source_bytes: 1024,
        purpose: SearchPurpose::Management,
        binding_revision: binding.revision.clone(),
    };
    assert!(matches!(
        service.context(
            reference.clone(),
            Arc::new(|_| true),
            QueryControl::default()
        ),
        Err(ApplicationError::ProtectedManagementSession)
    ));
    let mut stale = reference;
    stale.binding_revision = "stale".into();
    assert!(matches!(
        service.context(stale, Arc::new(|_| true), QueryControl::default()),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    assert_eq!(*reader.contexts.lock().unwrap(), 0);
    // Core orchestration consumes provider-neutral identities/ports. This fake
    // Claude port is not admission of any real Claude transcript format.
    let claude = ManagementIdentity {
        identity: artifact(ProviderKind::Claude, "/claude", "claude-fixture-host"),
        ..binding.clone()
    };
    let neutral = SessionSearchService {
        provider: ProviderKind::Claude,
        provider_root: Path::new("/claude"),
        management: Some(&claude),
        ..service
    };
    neutral
        .search(
            parameters(SearchPurpose::Management),
            None,
            QueryControl::default(),
        )
        .unwrap();
    assert_eq!(
        reader.requests.lock().unwrap().last().unwrap().excluded[0].provider,
        ProviderKind::Claude
    );
}

#[test]
fn two_trusted_fixture_callers_share_reader_without_identity_bleed_and_switches_refuse_old_cursors()
{
    let reader = Reader::default();
    let first = binding();
    let second = ManagementIdentity {
        identity: artifact(ProviderKind::Codex, "/provider", "codex:session:second"),
        native_session_id: "second".into(),
        revision: "second-connection".into(),
    };
    std::thread::scope(|scope| {
        for binding in [&first, &second] {
            let reader = &reader;
            scope.spawn(move || {
                let service = SessionSearchService {
                    reader,
                    provider_root: Path::new("/provider"),
                    provider: ProviderKind::Codex,
                    management: Some(binding),
                    apparent_caller: None,
                    registered: &[],
                    instance: "shared-server-fixture",
                };
                let page = service
                    .search(
                        parameters(SearchPurpose::Management),
                        None,
                        QueryControl::default(),
                    )
                    .unwrap();
                assert_eq!(page.binding_revision, binding.revision);
            });
        }
    });
    let requests = reader.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        assert_eq!(request.excluded.len(), 1);
        assert!(
            request.excluded[0]
                .local_id
                .as_str()
                .ends_with(request.management_session.as_ref().unwrap())
        );
    }
    drop(requests);
    let service = SessionSearchService {
        reader: &reader,
        provider_root: Path::new("/provider"),
        provider: ProviderKind::Codex,
        management: Some(&first),
        apparent_caller: None,
        registered: &[],
        instance: "shared-server-fixture",
    };
    let next = service
        .search(
            parameters(SearchPurpose::Management),
            None,
            QueryControl::default(),
        )
        .unwrap()
        .page
        .next;
    let switched = SessionSearchService {
        management: Some(&second),
        ..service
    };
    let count = reader.requests.lock().unwrap().len();
    assert!(matches!(
        switched.search(
            parameters(SearchPurpose::Management),
            next,
            QueryControl::default()
        ),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    assert_eq!(reader.requests.lock().unwrap().len(), count);
}
