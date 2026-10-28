use crate::CodexAdapter;
use agentmender_domain::*;
use agentmender_provider_api::ProviderAdapter;
use std::path::Path;

#[test]
fn fixtures_remain_protected_and_do_not_prove_absence() {
    for name in [
        "known",
        "observed-snapshot",
        "observed-snapshot-0.165.0-alpha.1.2",
        "partial",
        "unknown",
    ] {
        let adapter = CodexAdapter::from_fixture(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/codex")
                .join(name),
        )
        .unwrap();
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
    }
}

fn fixture_graph(name: &str) -> ArtifactGraph {
    let adapter = CodexAdapter::from_fixture(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/codex")
            .join(name),
    )
    .unwrap();
    let snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
    adapter.build_graph(&snapshot).unwrap()
}

#[test]
fn duplicate_capture_is_concurrent_change_not_identity_corruption() {
    let graph = fixture_graph("duplicate");
    let report = evaluate_integrity(&graph, &IntegrityContext::default());
    assert!(
        report
            .blockers
            .iter()
            .any(|b| b.code == IntegrityBlockerCode::ConcurrentChange)
    );
    assert!(!report.findings.iter().any(|f| matches!(
        f.kind,
        IntegrityIssueKind::DuplicateSessionIdentifier
            | IntegrityIssueKind::ConflictingIdentityEvidence
    )));
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.disposition != IssueDisposition::EstablishedViolation)
    );
}

#[test]
fn active_roots_and_skipped_links_do_not_create_findings() {
    let adapter = CodexAdapter::from_fixture(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known"),
    )
    .unwrap();
    let mut snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
    let graph = adapter.build_graph(&snapshot).unwrap();
    for kind in [
        RootKind::ActiveSession,
        RootKind::ArchivedRetainedSession,
        RootKind::ProviderGlobal,
    ] {
        let root = graph.roots().iter().find(|r| r.kind == kind).unwrap();
        assert!(graph.node(&root.node).unwrap().is_protected());
    }
    let original = evaluate_integrity(&graph, &IntegrityContext::default());
    assert!(original.findings.is_empty());
    snapshot.diagnostics.push(Diagnostic {
        code: DiagnosticCode::LinkSkipped,
        message: "synthetic skipped link".into(),
        path: None,
    });
    let skipped = evaluate_integrity(
        &adapter.build_graph(&snapshot).unwrap(),
        &IntegrityContext::default(),
    );
    assert!(
        skipped
            .blockers
            .iter()
            .any(|b| b.code == IntegrityBlockerCode::LinkSkipped)
    );
    assert_eq!(skipped.findings, original.findings);
    assert!(skipped.residuals.is_empty());
}
