//! Pure Claude interpretation of already collected discovery observations.
use agentmender_domain::{
    ArtifactGraph, Confidence, DiscoverySnapshot, EntityId, EvidenceRule, EvidenceSource,
    ProviderKind,
};
use agentmender_provider_api::{ProviderError, graph::build_observation_graph};

/// Build a graph without accessing provider state or interpreting additional metadata.
pub fn build_graph(snapshot: &DiscoverySnapshot) -> Result<ArtifactGraph, ProviderError> {
    if snapshot.provider.kind != ProviderKind::Claude {
        return Err(ProviderError::ProviderUnsupported(snapshot.provider.kind));
    }
    let fingerprint = &snapshot.provider.fingerprint;
    let mut rules = Vec::new();
    for (names, sources, cap, description) in [
        (
            &[
                "explicit_cliSessionId",
                "explicit_forkedFromSessionId",
                "explicit_desktop_cwd",
                "exact_fields_observed_with_desktop_4_29_unknown_producer",
            ][..],
            vec![EvidenceSource::ProviderMetadata, EvidenceSource::Fixture],
            Confidence::Heuristic,
            "Observed Desktop metadata field; historical producer is not established",
        ),
        (
            &[
                "explicit_cwd_workspace_observation_no_existence_claim",
                "explicit_transcript_cwd_not_decoded_bucket",
                "synthetic_explicit_session_metadata",
                "explicit_sidecar_parentAgentId",
                "explicit_subagent_sessionId_groups_root",
            ][..],
            vec![EvidenceSource::Fixture],
            Confidence::Heuristic,
            "Explicit metadata under the synthetic fixture contract",
        ),
        (
            &[
                "regular_file_metadata_protected",
                "desktop_regular_file_metadata_protected",
            ][..],
            vec![EvidenceSource::FilesystemMetadata, EvidenceSource::Fixture],
            Confidence::Unknown,
            "Regular file metadata does not establish ownership",
        ),
    ] {
        for name in names {
            for (id, confidence, detail) in [
                ((*name).to_owned(), cap, description.to_owned()),
                (
                    format!("{name}.association"),
                    Confidence::Heuristic,
                    format!("Grouping inferred from {name}; no ownership claim"),
                ),
            ] {
                rules.push(EvidenceRule {
                    id: EntityId::new(id).expect("static rule identity"),
                    provider: ProviderKind::Claude,
                    description: detail,
                    admissible_sources: sources.clone(),
                    max_confidence: confidence,
                    adapter_version: fingerprint.adapter_version.clone(),
                    interface_id: fingerprint.interface_id.clone(),
                    layout_id: fingerprint.layout_id.clone(),
                });
            }
        }
    }
    build_observation_graph(snapshot, rules).map_err(ProviderError::Graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClaudeAdapter;
    use agentmender_domain::{
        ArtifactKind, Completeness, DiscoveryScope, EdgeKind, EvidenceNature, GraphNodeKind,
        OwnershipClassification, Reference, ReferenceKind,
    };
    use agentmender_provider_api::ProviderAdapter;
    use std::path::Path;

    fn fixture(desktop: bool) -> DiscoverySnapshot {
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
        adapter.discover(&DiscoveryScope::default()).unwrap()
    }

    #[test]
    fn observed_desktop_references_and_raw_evidence_survive() {
        let snapshot = fixture(true);
        let graph = build_graph(&snapshot).unwrap();
        for (kind, count) in [
            (EdgeKind::MapsTo, 7),
            (EdgeKind::References, 1),
            (EdgeKind::SpawnedBy, 0),
        ] {
            assert_eq!(
                graph
                    .edges()
                    .iter()
                    .filter(|edge| edge.kind == kind)
                    .count(),
                count
            );
        }
        assert_eq!(graph.context().completeness, snapshot.completeness);
        assert_eq!(graph.context().coverage, snapshot.coverage);
        for reference in &snapshot.references {
            assert!(graph.edges().iter().any(|edge| {
                edge.evidence.iter().any(|e| {
                    e.observation == reference.evidence && e.nature == EvidenceNature::Observation
                })
            }));
        }
        for session in &snapshot.sessions {
            let node = graph
                .nodes()
                .iter()
                .find(|node| {
                    node.id.class == GraphNodeKind::Session && node.id.local_id == session.id
                })
                .unwrap();
            assert!(
                node.evidence
                    .iter()
                    .any(|e| e.observation == session.evidence
                        && e.observation.provider_version.is_none())
            );
            assert!(node.is_protected());
        }
        assert!(graph.nodes().iter().all(|node| node.is_protected()));
    }

    #[test]
    fn synthetic_relationships_are_heuristic_and_associations_are_inferences() {
        let snapshot = fixture(false);
        let graph = build_graph(&snapshot).unwrap();
        assert_eq!(
            graph
                .edges()
                .iter()
                .filter(|edge| edge.kind == EdgeKind::SpawnedBy)
                .count(),
            1
        );
        assert_eq!(
            graph
                .edges()
                .iter()
                .filter(|edge| edge.kind == EdgeKind::MapsTo)
                .count(),
            2
        );
        for edge in graph.edges() {
            for evidence in &edge.evidence {
                assert_eq!(graph.effective_confidence(evidence), Confidence::Heuristic);
                if edge.kind == EdgeKind::AssociatedWith {
                    assert_eq!(evidence.nature, EvidenceNature::Inference);
                    assert!(evidence.rule_id.as_str().ends_with(".association"));
                }
            }
        }
    }

    #[test]
    fn unknown_rules_and_unattributed_files_remain_unknown_and_protected() {
        let mut snapshot = fixture(false);
        snapshot.sessions[0].evidence.rule = "unrecognized_rule".into();
        snapshot.sessions[0].evidence.confidence = Confidence::Authoritative;
        snapshot.artifacts[0].kind = ArtifactKind::Unknown;
        let graph = build_graph(&snapshot).unwrap();
        let session = graph
            .nodes()
            .iter()
            .find(|n| {
                n.id.class == GraphNodeKind::Session && n.id.local_id == snapshot.sessions[0].id
            })
            .unwrap();
        assert_eq!(
            graph.effective_confidence(&session.evidence[0]),
            Confidence::Unknown
        );
        assert_eq!(
            session.evidence[0].observation,
            snapshot.sessions[0].evidence
        );
        let artifact = graph
            .nodes()
            .iter()
            .find(|n| {
                n.id.class == GraphNodeKind::Artifact && n.id.local_id == snapshot.artifacts[0].id
            })
            .unwrap();
        assert!(artifact.is_protected());
        assert_eq!(
            graph
                .ownership_assessment(&artifact.id)
                .unwrap()
                .classification,
            OwnershipClassification::Unknown
        );
    }

    #[test]
    fn unattributed_transcript_has_unknown_protected_ownership() {
        let mut snapshot = fixture(true);
        let id = snapshot.artifacts[0].id.clone();
        snapshot.artifacts[0].kind = ArtifactKind::Transcript;
        snapshot
            .references
            .retain(|r| r.target != id && r.source != id);
        let graph = build_graph(&snapshot).unwrap();
        let artifact = graph
            .nodes()
            .iter()
            .find(|node| node.id.class == GraphNodeKind::Artifact && node.id.local_id == id)
            .unwrap();
        assert_eq!(
            graph
                .ownership_assessment(&artifact.id)
                .unwrap()
                .classification,
            OwnershipClassification::Unknown
        );
        assert!(artifact.is_protected());
    }

    #[test]
    fn rule_caps_cannot_promote_or_authorize_other_sources() {
        let mut snapshot = fixture(false);
        snapshot.sessions[0].evidence.confidence = Confidence::Authoritative;
        snapshot.sessions[1].evidence.confidence = Confidence::Unknown;
        snapshot.sessions[2].evidence.source = EvidenceSource::ProviderMetadata;
        let graph = build_graph(&snapshot).unwrap();
        for (index, expected) in [
            (0, Confidence::Heuristic),
            (1, Confidence::Unknown),
            (2, Confidence::Unknown),
        ] {
            let node = graph
                .nodes()
                .iter()
                .find(|n| {
                    n.id.class == GraphNodeKind::Session
                        && n.id.local_id == snapshot.sessions[index].id
                })
                .unwrap();
            assert_eq!(
                node.evidence[0].observation,
                snapshot.sessions[index].evidence
            );
            assert_eq!(graph.effective_confidence(&node.evidence[0]), expected);
        }
    }

    #[test]
    fn unknown_compatibility_and_wrong_provider_are_not_interpreted() {
        let mut snapshot = fixture(false);
        snapshot.provider.compatibility.status =
            agentmender_domain::CompatibilityStatus::AuditOnlyUnknown;
        let graph = build_graph(&snapshot).unwrap();
        assert!(
            graph
                .nodes()
                .iter()
                .flat_map(|n| &n.evidence)
                .all(|e| graph.effective_confidence(e) == Confidence::Unknown)
        );
        snapshot.provider.kind = ProviderKind::Codex;
        assert!(build_graph(&snapshot).is_err());
    }

    #[test]
    fn input_permutations_and_partial_snapshot_are_preserved() {
        let mut snapshot = fixture(true);
        assert_eq!(snapshot.completeness, Completeness::Partial);
        let expected = serde_json::to_string(&build_graph(&snapshot).unwrap()).unwrap();
        snapshot.workspaces.reverse();
        snapshot.projects.reverse();
        snapshot.sessions.reverse();
        snapshot.artifacts.reverse();
        snapshot.references.reverse();
        snapshot.diagnostics.reverse();
        assert_eq!(
            expected,
            serde_json::to_string(&build_graph(&snapshot).unwrap()).unwrap()
        );
    }

    #[test]
    fn two_explicit_desktop_mappings_retain_shared_transcript() {
        let mut snapshot = fixture(true);
        snapshot.completeness = Completeness::Complete;
        snapshot.diagnostics.clear();
        let original = snapshot
            .references
            .iter()
            .find(|r| r.kind == ReferenceKind::MapsTo)
            .unwrap()
            .clone();
        let other = snapshot
            .sessions
            .iter()
            .find(|s| s.id != original.source)
            .unwrap()
            .id
            .clone();
        snapshot
            .references
            .retain(|r| !(r.kind == ReferenceKind::MapsTo && r.target == original.target));
        snapshot.references.push(original.clone());
        snapshot.references.push(Reference {
            source: other,
            ..original.clone()
        });
        let graph = build_graph(&snapshot).unwrap();
        let artifact = graph
            .nodes()
            .iter()
            .find(|n| n.id.class == GraphNodeKind::Artifact && n.id.local_id == original.target)
            .unwrap();
        assert_eq!(
            graph
                .ownership_assessment(&artifact.id)
                .unwrap()
                .classification,
            OwnershipClassification::Shared
        );
        assert!(artifact.is_protected());
    }
}
