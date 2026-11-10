use agentmender_application::{
    ApplicationError, QueryControl, session_search::SessionSearchService,
};
use agentmender_domain::*;
use agentmender_provider_api::{ProviderError, SessionSearchReadPort};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Default)]
struct Reader {
    calls: AtomicUsize,
    window: Mutex<Option<ContextWindow>>,
}
impl SessionSearchReadPort for Reader {
    fn search_sessions(&self, _: &SearchReadRequest) -> Result<SessionSearchPage, ProviderError> {
        unreachable!()
    }
    fn session_context(
        &self,
        _: &ContextReference,
        _: &[ArtifactId],
    ) -> Result<ConversationEvidence, ProviderError> {
        unreachable!()
    }
    fn session_context_window(
        &self,
        _: &ContextReference,
        _: &ContextOptions,
        _: &[ArtifactId],
    ) -> Result<ContextWindow, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.window.lock().unwrap().clone().unwrap())
    }
}
fn identity() -> ArtifactId {
    ArtifactId {
        provider: ProviderKind::Codex,
        state_root: "/provider".into(),
        class: GraphNodeKind::Session,
        local_id: EntityId::new("codex:session:candidate").unwrap(),
    }
}
fn reference() -> ContextReference {
    ContextReference {
        decoder_progress: None,
        native_session_id: "candidate".into(),
        local_id: identity().local_id,
        record_index: 42,
        source_fingerprint: "revision".into(),
        source_bytes: 1000,
        purpose: SearchPurpose::Diagnostic,
        binding_revision: "diagnostic_no_caller_guarantee".into(),
        position: Some(MessagePosition {
            ordinal: 42,
            ..Default::default()
        }),
    }
}
fn window() -> ContextWindow {
    ContextWindow {
        source_coverage: Default::default(),
        work: Default::default(),
        messages: vec![ConversationEvidence {
            source: "assistant_message".into(),
            record_index: 42,
            position: reference().position,
            excerpt: Some("The proposed fix was rejected; the final resolution follows.".into()),
            truncated: false,
            source_fingerprint: "revision".into(),
            source_bytes: 1000,
            format: "fixture".into(),
        }],
        previous: Some(reference()),
        next: Some(reference()),
        at_start: false,
        at_end: false,
        partial: false,
        bytes_read: 1000,
    }
}
fn service(reader: &Reader) -> SessionSearchService<'_> {
    SessionSearchService {
        reader,
        provider_root: Path::new("/provider"),
        provider: ProviderKind::Codex,
        management: None,
        apparent_caller: None,
        registered: &[],
        instance: "fixture",
    }
}
#[test]
fn context_windows_preserve_binding_exclusions_and_selector_before_access() {
    let reader = Reader {
        window: Mutex::new(Some(window())),
        ..Default::default()
    };
    let service = service(&reader);
    let mut management = reference();
    management.purpose = SearchPurpose::Management;
    assert!(matches!(
        service.context_window(
            management,
            Arc::new(|_| true),
            ContextOptions::default(),
            QueryControl::default()
        ),
        Err(ApplicationError::CallerIdentityRequired { .. })
    ));
    let mut stale = reference();
    stale.binding_revision = "another binding".into();
    assert!(matches!(
        service.context_window(
            stale,
            Arc::new(|_| true),
            ContextOptions::default(),
            QueryControl::default()
        ),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    assert!(matches!(
        service.context_window(
            reference(),
            Arc::new(|_| false),
            ContextOptions::default(),
            QueryControl::default()
        ),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    let excluded = [identity()];
    let protected = SessionSearchService {
        registered: &excluded,
        ..service
    };
    assert!(matches!(
        protected.context_window(
            reference(),
            Arc::new(|_| true),
            ContextOptions::default(),
            QueryControl::default()
        ),
        Err(ApplicationError::ProtectedManagementSession)
    ));
    assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
    let result = service
        .context_window(
            reference(),
            Arc::new(|id| *id == identity()),
            ContextOptions::default(),
            QueryControl::default(),
        )
        .unwrap();
    assert_eq!(result.window.messages[0].record_index, 42);
    assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn discovery_context_cannot_cross_provider_roots_or_read_the_apparent_caller() {
    let reader = Reader::default();
    let caller =
        ApparentCodexCaller::from_metadata("12345678-1234-1234-1234-123456789abc", None).unwrap();
    let original = SessionSearchService {
        apparent_caller: Some(&caller),
        ..service(&reader)
    };
    let id = caller.identity(original.provider_root);
    let discovery = ContextReference {
        purpose: SearchPurpose::Discovery,
        binding_revision: discovery_binding(&id),
        ..reference()
    };
    let foreign_root = SessionSearchService {
        provider_root: Path::new("/different-root"),
        ..original
    };
    let check = |service: &SessionSearchService<'_>, reference| {
        service.context_window(
            reference,
            Arc::new(|_| true),
            ContextOptions::default(),
            QueryControl::default(),
        )
    };
    assert!(matches!(
        check(&foreign_root, discovery.clone()),
        Err(ApplicationError::StaleSearchEvidence)
    ));
    let foreign_provider = SessionSearchService {
        provider: ProviderKind::Claude,
        ..original
    };
    assert!(matches!(
        check(&foreign_provider, discovery.clone()),
        Err(ApplicationError::DiscoveryCallerUnavailable)
    ));
    assert!(matches!(
        check(
            &original,
            ContextReference {
                local_id: id.local_id,
                native_session_id: caller.native_id().into(),
                ..discovery
            }
        ),
        Err(ApplicationError::ProtectedManagementSession)
    ));
    assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn faulty_adapter_cannot_change_source_neighbors_or_exceed_context_budgets() {
    let reader = Reader {
        window: Mutex::new(Some(window())),
        ..Default::default()
    };
    let service = service(&reader);
    for violation in 0..5 {
        let mut w = window();
        match violation {
            0 => w.messages[0].source_fingerprint = "unrelated".into(),
            1 => w.next.as_mut().unwrap().native_session_id = "foreign".into(),
            2 => w.previous.as_mut().unwrap().binding_revision = "foreign binding".into(),
            3 => w.messages[0].excerpt = Some("a".repeat(16385)),
            _ => w.messages[0].excerpt = Some("password=private".into()),
        }
        *reader.window.lock().unwrap() = Some(w);
        assert!(
            matches!(
                service.context_window(
                    reference(),
                    Arc::new(|_| true),
                    ContextOptions::default(),
                    QueryControl::default()
                ),
                Err(ApplicationError::InvalidSearch)
            ),
            "violation {violation}"
        );
    }
    let cancelled = QueryControl::default();
    cancelled.cancel();
    assert!(matches!(
        service.context_window(
            reference(),
            Arc::new(|_| true),
            ContextOptions::default(),
            cancelled
        ),
        Err(ApplicationError::Cancelled)
    ));
}
