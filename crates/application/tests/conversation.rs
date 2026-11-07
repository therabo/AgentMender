use agentmender_application::{QueryControl, conversation::*};
use agentmender_domain::*;
use agentmender_provider_api::*;
struct Metadata;
const M: &str = "12345678-1234-1234-1234-123456789abc";
impl SessionMetadataReadPort for Metadata {
    fn session_metadata(&self) -> Result<SessionInventory, ProviderError> {
        Ok(SessionInventory {
            completeness: Completeness::Partial,
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
    assert_eq!(sessions.len(), 4);
    assert_ne!(sessions[0].metadata.identity, sessions[3].metadata.identity);
    assert!(
        sessions
            .iter()
            .any(|s| !s.metadata.producer_version_matches && !s.can_request_preview)
    );
    for identity in [
        sessions[0].metadata.identity.clone(),
        sessions[3].metadata.identity.clone(),
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
