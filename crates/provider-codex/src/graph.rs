//! Pure interpretation of Codex snapshot evidence for the provider-neutral graph.
use agentmender_domain::{
    ArtifactGraph, Confidence, DiscoverySnapshot, EntityId, EvidenceRule, EvidenceSource,
    ProviderKind,
};
use agentmender_provider_api::ProviderError;

/// Interpret only the evidence already present in a snapshot. No provider state is read.
pub fn build_graph(snapshot: &DiscoverySnapshot) -> Result<ArtifactGraph, ProviderError> {
    if snapshot.provider.kind != ProviderKind::Codex {
        return Err(ProviderError::ProviderUnsupported(snapshot.provider.kind));
    }
    let fingerprint = &snapshot.provider.fingerprint;
    let definitions = [
        (
            "official_thread_list_metadata_use_state_db_only",
            "Official thread list metadata from the bounded state database snapshot",
            vec![EvidenceSource::ProviderSnapshot, EvidenceSource::Fixture],
            Confidence::Strong,
        ),
        (
            "cwd_group_without_native_project_assignment",
            "Cwd grouping without native project assignment",
            vec![EvidenceSource::ProviderSnapshot, EvidenceSource::Fixture],
            Confidence::Heuristic,
        ),
        (
            "protected_regular_file_metadata_only",
            "Protected regular file metadata only",
            vec![EvidenceSource::FilesystemMetadata],
            Confidence::Unknown,
        ),
    ];
    let mut rules = Vec::new();
    for (name, description, sources, maximum) in definitions {
        for (id, cap) in [
            (name.to_owned(), maximum),
            (format!("{name}.association"), Confidence::Heuristic),
        ] {
            rules.push(EvidenceRule {
                id: EntityId::new(id).expect("fixed rule identity"),
                provider: ProviderKind::Codex,
                description: description.into(),
                admissible_sources: sources.clone(),
                max_confidence: cap,
                adapter_version: fingerprint.adapter_version.clone(),
                interface_id: fingerprint.interface_id.clone(),
                layout_id: fingerprint.layout_id.clone(),
            });
        }
    }
    agentmender_provider_api::graph::build_observation_graph(snapshot, rules)
        .map_err(ProviderError::Graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodexAdapter;
    use agentmender_domain::{
        CompatibilityStatus, Completeness, Diagnostic, DiagnosticCode, DiscoveryScope, EdgeKind,
        EvidenceNature, GraphNodeKind, RootKind,
    };
    use agentmender_provider_api::ProviderAdapter;
    use std::path::Path;

    fn snapshot(name: &str) -> DiscoverySnapshot {
        CodexAdapter::from_fixture(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/codex")
                .join(name),
        )
        .unwrap()
        .discover(&DiscoveryScope::default())
        .unwrap()
    }

    #[test]
    fn fixture_graphs_preserve_context_evidence_and_are_deterministic() {
        for name in [
            "known",
            "observed-snapshot",
            "observed-snapshot-0.165.0-alpha.1.2",
            "partial",
            "unknown",
        ] {
            let mut s = snapshot(name);
            s.diagnostics.push(Diagnostic {
                code: DiagnosticCode::SnapshotStale,
                message: "stale".into(),
                path: None,
            });
            s.diagnostics.push(Diagnostic {
                code: DiagnosticCode::ConcurrentChange,
                message: "concurrent".into(),
                path: None,
            });
            let g = build_graph(&s).unwrap();
            assert_eq!(g.context().evidence_status, s.evidence_status);
            assert_eq!(g.context().completeness, s.completeness);
            assert_eq!(g.context().coverage, s.coverage);
            for diagnostic in &s.diagnostics {
                assert!(g.context().diagnostics.contains(diagnostic));
            }
            for node in g.nodes() {
                assert!(node.is_protected());
            }
            for evidence in s
                .workspaces
                .iter()
                .map(|x| &x.evidence)
                .chain(s.projects.iter().map(|x| &x.evidence))
                .chain(s.sessions.iter().map(|x| &x.evidence))
                .chain(s.artifacts.iter().map(|x| &x.evidence))
            {
                assert!(
                    g.nodes()
                        .iter()
                        .flat_map(|n| &n.evidence)
                        .any(|e| &e.observation == evidence)
                );
            }
            let first = serde_json::to_string(&g).unwrap();
            assert_eq!(
                first,
                serde_json::to_string(&build_graph(&s).unwrap()).unwrap()
            );
            s.workspaces.reverse();
            s.projects.reverse();
            s.sessions.reverse();
            s.artifacts.reverse();
            s.references.reverse();
            s.diagnostics.reverse();
            assert_eq!(
                first,
                serde_json::to_string(&build_graph(&s).unwrap()).unwrap()
            );
        }
    }

    #[test]
    fn synthetic_roots_references_and_grouping_retain_their_strength() {
        let s = snapshot("known");
        let g = build_graph(&s).unwrap();
        assert!(g.roots().iter().any(|r| r.kind == RootKind::ActiveSession));
        assert!(
            g.roots()
                .iter()
                .any(|r| r.kind == RootKind::ArchivedRetainedSession)
        );
        assert!(g.roots().iter().any(|r| r.kind == RootKind::ProviderGlobal));
        for reference in &s.references {
            assert!(
                g.edges()
                    .iter()
                    .flat_map(|e| &e.evidence)
                    .any(|e| e.observation == reference.evidence
                        && e.nature == EvidenceNature::Observation
                        && g.effective_confidence(e) == Confidence::Heuristic)
            );
        }
        let associations: Vec<_> = g
            .edges()
            .iter()
            .filter(|e| e.kind == EdgeKind::AssociatedWith)
            .collect();
        assert!(!associations.is_empty());
        assert!(
            associations
                .iter()
                .flat_map(|e| &e.evidence)
                .all(|e| e.nature == EvidenceNature::Inference
                    && g.effective_confidence(e) == Confidence::Heuristic)
        );
        let cwd_graph = build_graph(&snapshot("observed-snapshot")).unwrap();
        assert!(
            cwd_graph
                .nodes()
                .iter()
                .filter(|n| n.id.class == GraphNodeKind::Project)
                .flat_map(|n| &n.evidence)
                .any(
                    |e| e.observation.rule == "cwd_group_without_native_project_assignment"
                        && g.effective_confidence(e) == Confidence::Heuristic
                )
        );
    }

    #[test]
    fn observed_strength_and_source_admissibility_are_bounded() {
        let mut observed = snapshot("observed-snapshot");
        let graph = build_graph(&observed).unwrap();
        assert!(graph.nodes().iter().flat_map(|n| &n.evidence).any(|e| {
            e.observation.source == EvidenceSource::Fixture
                && e.observation.confidence == Confidence::Strong
                && graph.effective_confidence(e) == Confidence::Strong
        }));
        observed.sessions[0].evidence.source = EvidenceSource::ProviderApi;
        let graph = build_graph(&observed).unwrap();
        assert!(graph.nodes().iter().flat_map(|n| &n.evidence).any(|e| {
            e.observation.source == EvidenceSource::ProviderApi
                && graph.effective_confidence(e) == Confidence::Unknown
        }));
    }

    #[test]
    fn unknown_and_partial_context_remain_conservative() {
        let unknown = snapshot("unknown");
        assert_eq!(
            unknown.provider.compatibility.status,
            CompatibilityStatus::AuditOnlyUnknown
        );
        let graph = build_graph(&unknown).unwrap();
        assert!(
            graph
                .nodes()
                .iter()
                .flat_map(|n| &n.evidence)
                .all(|e| graph.effective_confidence(e) == Confidence::Unknown)
        );
        assert_eq!(
            build_graph(&snapshot("partial"))
                .unwrap()
                .context()
                .completeness,
            Completeness::Partial
        );
        let mut foreign = unknown;
        foreign.provider.kind = ProviderKind::Claude;
        assert!(matches!(
            build_graph(&foreign),
            Err(ProviderError::ProviderUnsupported(ProviderKind::Claude))
        ));
    }
}
