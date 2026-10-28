use crate::ClaudeAdapter;
use agentmender_domain::*;
use agentmender_provider_api::ProviderAdapter;
use std::path::Path;

#[test]
fn fixture_graphs_do_not_infer_orphans_or_residuals() {
    for desktop in [false, true] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(if desktop {
            "../../fixtures/claude/4.29.0-observed"
        } else {
            "../../fixtures/claude/2.1.311-synthetic"
        });
        let adapter = if desktop {
            ClaudeAdapter::from_desktop_fixture(path).unwrap()
        } else {
            ClaudeAdapter::from_fixture(path).unwrap()
        };
        let snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
        let graph = adapter.build_graph(&snapshot).unwrap();
        let report = evaluate_integrity(&graph, &IntegrityContext::default());
        assert!(matches!(
            report.verification_status,
            VerificationStatus::Unknown | VerificationStatus::Degraded
        ));
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.disposition != IssueDisposition::EstablishedViolation)
        );
        assert!(report.residuals.is_empty());
        assert!(graph.nodes().iter().all(ArtifactNode::is_protected));
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.code == IntegrityBlockerCode::MissingIntegrityContract)
        );
        if desktop {
            assert!(graph.edges().iter().any(|e| e.kind == EdgeKind::References));
            assert!(!graph.edges().iter().any(|e| e.kind == EdgeKind::SpawnedBy));
        }
    }
}
