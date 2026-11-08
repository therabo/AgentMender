use agentmender_application::{QueryControl, conversation::*};
use agentmender_domain::*;
use agentmender_provider_api::*;
struct Metadata;
const M: &str = "12345678-1234-1234-1234-123456789abc";
impl SessionMetadataReadPort for Metadata {
    fn session_metadata(&self) -> Result<SessionInventory, ProviderError> {
        Ok(SessionInventory {
            completeness: Completeness::Partial,
            read_compatibility: None,
            sessions: [
                (
                    "/a",
                    M,
                    Some("README"),
                    "MISSION1",
                    SessionState::Active,
                    true,
                ),
                (
                    "/a",
                    "12345678-1234-1234-1234-123456789abd",
                    Some("README"),
                    "MISSION1",
                    SessionState::Archived,
                    true,
                ),
                (
                    "/b",
                    M,
                    Some("README"),
                    "MISSION1",
                    SessionState::Active,
                    true,
                ),
                (
                    "/a",
                    "12345678-1234-1234-1234-123456789ab0",
                    Some("README notes"),
                    "MISSION1",
                    SessionState::Active,
                    false,
                ),
                (
                    "/a",
                    "12345678-1234-1234-1234-123456789ab1",
                    None,
                    "MISSION1",
                    SessionState::Unknown,
                    true,
                ),
            ]
            .into_iter()
            .map(
                |(root, id, title, project, state, version)| SessionMetadata {
                    identity: ArtifactId {
                        provider: ProviderKind::Codex,
                        state_root: root.into(),
                        class: GraphNodeKind::Session,
                        local_id: EntityId::new(format!("codex:session:{id}")).unwrap(),
                    },
                    native_session_id: id.into(),
                    title: title.map(str::to_owned),
                    project_label: Some(project.into()),
                    created_at: None,
                    last_active_at: None,
                    state,
                    producer_version_matches: version,
                    producer_version: None,
                    parent_native_session_id: None,
                },
            )
            .collect(),
        })
    }
}
impl ScopedCleanupReadPort for Metadata {
    fn observe_cleanup(
        &self,
        _: &str,
        _: &DiscoveryScope,
    ) -> Result<ScopedCleanupObservation, ProviderError> {
        panic!("unsafe identity reached cleanup")
    }
}
#[test]
fn ambiguous_metadata_partial_coverage_and_root_identity_never_authorize() {
    let protected = vec![M.into()];
    let service = ConversationService {
        metadata: &Metadata,
        cleanup: &Metadata,
        protected_sessions: &protected,
        provider_root: std::path::Path::new("/a"),
    };
    let ConversationResponse::Search {
        sessions,
        completeness,
        ..
    } = service
        .dispatch(
            ConversationRequest::Search {
                query: "MISSION1 README".into(),
                include_archived: true,
            },
            None,
            QueryControl::default(),
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(completeness, Completeness::Partial);
    assert_eq!(sessions.len(), 3);
    assert!(sessions.iter().all(|s| {
        s.metadata.identity.state_root != std::path::Path::new("/a")
            || s.metadata.native_session_id != M
    }));
    assert_ne!(sessions[0].metadata.identity, sessions[2].metadata.identity);
    assert!(
        sessions
            .iter()
            .any(|s| !s.metadata.producer_version_matches && !s.can_request_preview)
    );
    for identity in [
        sessions[0].metadata.identity.clone(),
        sessions[2].metadata.identity.clone(),
        ArtifactId {
            provider: ProviderKind::Claude,
            ..sessions[0].metadata.identity.clone()
        },
    ] {
        let selector = std::sync::Arc::new(move |candidate: &ArtifactId| *candidate == identity);
        assert!(
            service
                .dispatch(
                    ConversationRequest::Preview { target: selector },
                    None,
                    QueryControl::default()
                )
                .is_err()
        );
    }
    let unknown = std::sync::Arc::new(|_: &ArtifactId| false);
    assert!(
        service
            .dispatch(
                ConversationRequest::Preview { target: unknown },
                None,
                QueryControl::default()
            )
            .is_err()
    );
    let unregistered = ConversationService {
        protected_sessions: &[],
        ..service
    };
    assert!(
        unregistered
            .dispatch(
                ConversationRequest::Preview {
                    target: std::sync::Arc::new(|_: &ArtifactId| true)
                },
                None,
                QueryControl::default()
            )
            .is_err()
    );
}

#[test]
fn registered_management_is_excluded_before_matching_in_both_search_modes() {
    let protected = vec![M.into()];
    let root = std::path::Path::new("/a");
    let service = ConversationService {
        metadata: &Metadata,
        cleanup: &Metadata,
        protected_sessions: &protected,
        provider_root: root,
    };
    // Blank listings, exact UUID, keywords and archive filtering use one policy.
    for query in ["", M, "README", "MISSION1"] {
        for include_archived in [false, true] {
            for primary in [false, true] {
                let request = ConversationRequest::Search {
                    query: query.into(),
                    include_archived,
                };
                let response = if primary {
                    service.dispatch(request, None, QueryControl::default())
                } else {
                    read_only_dispatch(
                        &Metadata,
                        root,
                        &protected,
                        request,
                        QueryControl::default(),
                    )
                };
                let ConversationResponse::Search { sessions, .. } = response.unwrap() else {
                    panic!("expected search")
                };
                assert!(sessions.iter().all(|s| {
                    s.metadata.identity.state_root != root || s.metadata.native_session_id != M
                }));
                // A same-UUID session in another root is independent evidence.
                let foreign = sessions
                    .iter()
                    .find(|s| {
                        s.metadata.identity.state_root == std::path::Path::new("/b")
                            && s.metadata.native_session_id == M
                    })
                    .expect("do not erase the foreign namespace");
                assert!(!foreign.can_request_preview);
            }
        }
    }
}

#[test]
fn protected_preview_prepare_and_unregistered_management_search_refuse() {
    use agentmender_application::ApplicationError;
    let protected = vec![M.into()];
    let root = std::path::Path::new("/a");
    let service = ConversationService {
        metadata: &Metadata,
        cleanup: &Metadata,
        protected_sessions: &protected,
        provider_root: root,
    };
    let target: agentmender_application::TargetSelector = std::sync::Arc::new(|id| {
        id.state_root == std::path::Path::new("/a")
            && id.local_id.as_str() == format!("codex:session:{M}")
    });
    assert!(matches!(
        read_only_dispatch(
            &Metadata,
            root,
            &protected,
            ConversationRequest::Preview {
                target: target.clone()
            },
            QueryControl::default()
        ),
        Err(ApplicationError::ProtectedManagementSession)
    ));
    for request in [
        ConversationRequest::Preview {
            target: target.clone(),
        },
        ConversationRequest::Prepare {
            target,
            expected_digest: "untrusted".into(),
        },
    ] {
        assert!(matches!(
            service.dispatch(request, None, QueryControl::default()),
            Err(ApplicationError::ProtectedManagementSession)
        ));
    }
    let unregistered = ConversationService {
        protected_sessions: &[],
        ..service
    };
    assert!(matches!(
        unregistered.dispatch(
            ConversationRequest::Search {
                query: "MISSION1".into(),
                include_archived: true
            },
            None,
            QueryControl::default()
        ),
        Err(ApplicationError::ManagementProtectionUnavailable)
    ));
    // Legacy read-only browsing is still available, without claiming caller binding.
    let ConversationResponse::Search { sessions, .. } = read_only_dispatch(
        &Metadata,
        root,
        &[],
        ConversationRequest::Search {
            query: M.into(),
            include_archived: true,
        },
        QueryControl::default(),
    )
    .unwrap() else {
        panic!("expected metadata")
    };
    assert_eq!(sessions.len(), 2);
}
