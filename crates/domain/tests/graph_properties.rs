use agentmender_domain::*;
use proptest::{prelude::*, test_runner::RngSeed};

fn id(s: impl Into<String>) -> EntityId {
    EntityId::new(s.into()).unwrap()
}
fn context(completeness: Completeness, status: EvidenceStatus) -> GraphContext {
    GraphContext {
        provider: Provider {
            kind: ProviderKind::Codex,
            presence: Presence::StateOnly,
            state_root: "/synthetic/state".into(),
            fingerprint: ProviderFingerprint {
                executable: None,
                executable_version: None,
                adapter_version: "test-1".into(),
                interface_id: Some("interface".into()),
                layout_id: Some("layout".into()),
            },
            capabilities: ProviderCapabilities::new(),
            compatibility: CompatibilityProfile {
                status: CompatibilityStatus::ReadOnlyKnown,
                interface_stability: InterfaceStability::OfficialStable,
                reason: "synthetic".into(),
            },
            installations: vec![],
            evidence_status: status,
            diagnostics: vec![],
        },
        evidence_status: status,
        completeness,
        coverage: "declared synthetic interface".into(),
        diagnostics: vec![],
    }
}
fn evidence(confidence: Confidence) -> GraphEvidence {
    GraphEvidence {
        observation: Evidence {
            source: EvidenceSource::Fixture,
            confidence,
            provider_version: Some("producer-1".into()),
            rule: "raw-rule".into(),
        },
        rule_id: id("rule"),
        nature: EvidenceNature::Observation,
        locator: Some("/synthetic/record".into()),
        observed_at: None,
    }
}
fn rule(cap: Confidence) -> EvidenceRule {
    EvidenceRule {
        id: id("rule"),
        provider: ProviderKind::Codex,
        description: "synthetic descriptor".into(),
        admissible_sources: vec![EvidenceSource::Fixture],
        max_confidence: cap,
        adapter_version: "test-1".into(),
        interface_id: Some("interface".into()),
        layout_id: Some("layout".into()),
    }
}
fn artifact(name: &str, locator: &str, ev: &GraphEvidence) -> NodeObservation {
    NodeObservation::Artifact(Artifact {
        id: id(name),
        kind: ArtifactKind::Plan,
        locator: locator.into(),
        size_bytes: None,
        evidence: ev.observation.clone(),
    })
}
fn session(n: usize, ev: &GraphEvidence) -> NodeObservation {
    NodeObservation::Session(Session {
        id: id(format!("session-{n}")),
        provider_session_id: format!("session-{n}"),
        project_id: None,
        state: SessionState::Archived,
        last_active: None,
        evidence: ev.observation.clone(),
    })
}
fn graph(
    count: usize,
    completeness: Completeness,
    reverse: bool,
    duplicate: bool,
) -> (ArtifactGraph, ArtifactId) {
    let ev = evidence(Confidence::Strong);
    let mut b = ArtifactGraph::builder(context(completeness, EvidenceStatus::SyntheticOnly));
    b.add_rule(rule(Confidence::Authoritative)).unwrap();
    let target = b.observe(artifact("target", "/synthetic/target", &ev));
    b.add_node_evidence(&target, ev.clone()).unwrap();
    let mut indices: Vec<_> = (0..count).collect();
    if reverse {
        indices.reverse();
    }
    for n in indices {
        let root = b.observe(session(n, &ev));
        b.add_node_evidence(&root, ev.clone()).unwrap();
        let r = ProtectedRoot {
            node: root.clone(),
            kind: RootKind::ArchivedRetainedSession,
        };
        let edge = ReferenceEdge {
            source: root,
            target: target.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev.clone()],
        };
        b.add_root(r.clone()).unwrap();
        b.add_edge(edge.clone()).unwrap();
        if duplicate {
            b.add_root(r).unwrap();
            b.add_edge(edge).unwrap();
            b.add_node_evidence(&target, ev.clone()).unwrap();
            b.observe(session(n, &ev));
        }
    }
    (b.finish().unwrap(), target)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d325f6772617068), .. ProptestConfig::default() })]
    #[test]
    fn retained_roots_classify_and_never_unprotect(count in 0usize..7, mode in 0u8..3) {
        let completeness = [Completeness::Complete, Completeness::Partial, Completeness::Unavailable][mode as usize];
        let (g, target) = graph(count, completeness, false, false);
        let class = g.ownership_assessment(&target).unwrap().classification;
        prop_assert_eq!(class, match count { 0 => OwnershipClassification::Unknown, 1 => OwnershipClassification::Referenced, _ => OwnershipClassification::Shared });
        prop_assert!(g.nodes().iter().all(ArtifactNode::is_protected));
        let (added, _) = graph(count + 1, completeness, false, false);
        prop_assert!(added.node(&target).unwrap().is_protected());
        prop_assert!(!matches!(added.ownership_assessment(&target).unwrap().classification, OwnershipClassification::Exclusive | OwnershipClassification::Orphaned));
    }
    #[test]
    fn insertion_permutation_and_exact_duplicates_are_canonical(count in 0usize..7, mode in 0u8..3) {
        let c = [Completeness::Complete, Completeness::Partial, Completeness::Unavailable][mode as usize];
        let (a, _) = graph(count, c, false, false); let (b, _) = graph(count, c, true, true);
        prop_assert_eq!(serde_json::to_vec(&a).unwrap(), serde_json::to_vec(&b).unwrap());
    }
    #[test]
    fn roundtrip_preserves_provenance_and_reclassifies(count in 0usize..7, mode in 0u8..3) {
        let c = [Completeness::Complete, Completeness::Partial, Completeness::Unavailable][mode as usize];
        let (g, target) = graph(count, c, false, false);
        let round: ArtifactGraph = serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
        prop_assert_eq!(&g, &round);
        prop_assert_eq!(g.ownership_assessment(&target).unwrap(), round.ownership_assessment(&target).unwrap());
        prop_assert_eq!(round.node(&target).unwrap().evidence[0].clone(), evidence(Confidence::Strong));
    }
    #[test]
    fn weak_raw_evidence_never_promotes(raw in 0u8..2, cap in 0u8..4, status in 0u8..3, inferred in any::<bool>()) {
        let raw = [Confidence::Unknown, Confidence::Heuristic][raw as usize];
        let cap = [Confidence::Unknown, Confidence::Heuristic, Confidence::Strong, Confidence::Authoritative][cap as usize];
        let status = [EvidenceStatus::Unverified, EvidenceStatus::SyntheticOnly, EvidenceStatus::RealVerified][status as usize];
        let mut ev = evidence(raw); if inferred { ev.nature = EvidenceNature::Inference; }
        let mut b = ArtifactGraph::builder(context(Completeness::Complete, status)); b.add_rule(rule(cap)).unwrap();
        let target = b.observe(artifact("target", "/synthetic/target", &ev)); b.add_node_evidence(&target, ev.clone()).unwrap();
        let g = b.finish().unwrap(); let effective = g.effective_confidence(&ev);
        prop_assert!(matches!(effective, Confidence::Unknown | Confidence::Heuristic));
        if raw == Confidence::Unknown { prop_assert_eq!(effective, Confidence::Unknown); }
        prop_assert_eq!(&g.node(&target).unwrap().evidence[0], &ev);
    }
    #[test]
    fn differing_duplicate_payloads_remain_conflicting(suffix in 1u16..1000) {
        let ev = evidence(Confidence::Strong); let mut b = ArtifactGraph::builder(context(Completeness::Complete, EvidenceStatus::SyntheticOnly));
        b.add_rule(rule(Confidence::Authoritative)).unwrap();
        let target = b.observe(artifact("target", "/synthetic/a", &ev)); b.observe(artifact("target", &format!("/synthetic/{suffix}"), &ev)); b.add_node_evidence(&target, ev).unwrap();
        let g = b.finish().unwrap();
        prop_assert_eq!(g.node(&target).unwrap().observations.len(), 2);
        prop_assert!(g.diagnostics().contains(&GraphDiagnostic::ConflictingNodeObservations { node: target.clone() }), "missing conflict diagnostic");
        prop_assert_eq!(g.ownership_assessment(&target).unwrap().classification, OwnershipClassification::Unknown);
    }
    #[test]
    fn equivalent_path_spellings_preserve_distinct_raw_facts(suffix in 1u16..1000, reverse in any::<bool>()) {
        let direct = format!("/synthetic/record-{suffix}");
        let dotted = format!("/synthetic/./record-{suffix}");
        let mut first = evidence(Confidence::Strong);
        first.locator = Some(direct.clone().into());
        let mut second = first.clone();
        second.locator = Some(dotted.clone().into());
        prop_assert_eq!(&first.locator, &second.locator);
        prop_assert_ne!(serde_json::to_vec(&first).unwrap(), serde_json::to_vec(&second).unwrap());
        let mut b = ArtifactGraph::builder(context(Completeness::Complete, EvidenceStatus::SyntheticOnly));
        b.add_rule(rule(Confidence::Strong)).unwrap();
        let target = b.observe(artifact("target", &direct, &first));
        b.observe(artifact("target", &dotted, &second));
        let facts = if reverse { [second, first] } else { [first, second] };
        for fact in facts {
            b.add_node_evidence(&target, fact).unwrap();
        }
        let g = b.finish().unwrap();
        prop_assert_eq!(g.node(&target).unwrap().observations.len(), 2);
        prop_assert_eq!(g.node(&target).unwrap().evidence.len(), 2);
        let assessment = g.ownership_assessment(&target).unwrap();
        prop_assert_eq!(assessment.classification, OwnershipClassification::Unknown);
        prop_assert!(assessment.blockers.contains(&ProtectionReason::ConflictingEvidence));
        let bytes = serde_json::to_vec(&g).unwrap();
        let round: ArtifactGraph = serde_json::from_slice(&bytes).unwrap();
        prop_assert_eq!(bytes, serde_json::to_vec(&round).unwrap());
    }
}

#[test]
fn unknown_rule_global_cycles_and_unresolved_are_protected() {
    let mut ev = evidence(Confidence::Strong);
    ev.rule_id = id("unknown-rule");
    let mut b = ArtifactGraph::builder(context(Completeness::Partial, EvidenceStatus::Unverified));
    let global = b.observe(NodeObservation::ProviderState);
    let unresolved = b.observe(NodeObservation::Unresolved { id: id("missing") });
    let target = b.observe(artifact("target", "/synthetic/target", &ev));
    b.add_node_evidence(&target, ev.clone()).unwrap();
    for node in [&global, &unresolved] {
        b.add_root(ProtectedRoot {
            node: node.clone(),
            kind: if node == &global {
                RootKind::ProviderGlobal
            } else {
                RootKind::UnknownState
            },
        })
        .unwrap();
    }
    for (source, target) in [
        (&global, &unresolved),
        (&unresolved, &global),
        (&unresolved, &target),
    ] {
        b.add_edge(ReferenceEdge {
            source: source.clone(),
            target: target.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev.clone()],
        })
        .unwrap();
    }
    let g = b.finish().unwrap();
    assert_eq!(g.effective_confidence(&ev), Confidence::Unknown);
    assert_eq!(g.node(&target).unwrap().evidence[0], ev);
    assert_eq!(
        g.ownership_assessment(&global).unwrap().classification,
        OwnershipClassification::Global
    );
    assert_eq!(
        g.ownership_assessment(&target).unwrap().classification,
        OwnershipClassification::Unknown
    );
    assert_eq!(g.roots().len(), 2);
    assert!(g.nodes().iter().all(ArtifactNode::is_protected));
    assert_eq!(g.explain(&global).unwrap(), g.explain(&global).unwrap());
}

#[test]
fn structured_ids_and_invalid_imports() {
    let (g, target) = graph(1, Completeness::Complete, false, false);
    let mut other_provider = target.clone();
    other_provider.provider = ProviderKind::Claude;
    let mut other_root = target.clone();
    other_root.state_root = "/synthetic/other".into();
    let mut other_class = target.clone();
    other_class.class = GraphNodeKind::Session;
    let ids: std::collections::BTreeSet<_> = [
        target.clone(),
        other_provider.clone(),
        other_root.clone(),
        other_class,
    ]
    .into_iter()
    .collect();
    assert_eq!(ids.len(), 4);
    for foreign in [other_provider, other_root] {
        let mut wire = serde_json::to_value(&g).unwrap();
        wire["nodes"][0]["id"] = serde_json::to_value(foreign).unwrap();
        assert!(serde_json::from_value::<ArtifactGraph>(wire).is_err());
    }
    let mut wire = serde_json::to_value(&g).unwrap();
    wire["edges"][0]["target"]["local_id"] = serde_json::to_value(id("missing")).unwrap();
    assert!(serde_json::from_value::<ArtifactGraph>(wire).is_err());
}

#[test]
fn stale_and_concurrent_snapshots_cannot_assert_reference_class() {
    for code in [
        DiagnosticCode::SnapshotStale,
        DiagnosticCode::ConcurrentChange,
    ] {
        let (g, target) = graph(2, Completeness::Complete, false, false);
        let mut wire = serde_json::to_value(g).unwrap();
        wire["context"]["diagnostics"] = serde_json::to_value(vec![Diagnostic {
            code,
            message: "synthetic".into(),
            path: None,
        }])
        .unwrap();
        let imported: ArtifactGraph = serde_json::from_value(wire).unwrap();
        assert_eq!(
            imported
                .ownership_assessment(&target)
                .unwrap()
                .classification,
            OwnershipClassification::Unknown
        );
        assert!(imported.node(&target).unwrap().is_protected());
    }
}

fn ordered_graph(order: &[usize], edge_order: &[usize], root_order: &[usize]) -> ArtifactGraph {
    let ev = evidence(Confidence::Strong);
    let mut b = ArtifactGraph::builder(context(
        Completeness::Complete,
        EvidenceStatus::SyntheticOnly,
    ));
    b.add_rule(rule(Confidence::Authoritative)).unwrap();
    let target = b.observe(artifact("target", "/synthetic/target", &ev));
    b.add_node_evidence(&target, ev.clone()).unwrap();
    let ids: Vec<_> = (0..order.len())
        .map(|n| ArtifactId {
            provider: ProviderKind::Codex,
            state_root: "/synthetic/state".into(),
            class: GraphNodeKind::Session,
            local_id: id(format!("session-{n}")),
        })
        .collect();
    // Edges and roots are deliberately inserted before their endpoints.
    for &n in edge_order {
        b.add_edge(ReferenceEdge {
            source: ids[n].clone(),
            target: target.clone(),
            kind: EdgeKind::References,
            evidence: vec![ev.clone(), ev.clone()],
        })
        .unwrap();
    }
    for &n in root_order {
        b.add_root(ProtectedRoot {
            node: ids[n].clone(),
            kind: RootKind::ArchivedRetainedSession,
        })
        .unwrap();
    }
    for &n in order {
        b.add_node_evidence(&ids[n], ev.clone()).unwrap();
        assert_eq!(b.observe(session(n, &ev)), ids[n]);
    }
    b.finish().unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d325f6772617068), .. ProptestConfig::default() })]
    #[test]
    fn independent_generated_insertion_orders_match(keys in proptest::collection::vec((any::<u8>(), any::<u8>(), any::<u8>()), 0..7)) {
        let base: Vec<_> = (0..keys.len()).collect();
        let mut nodes = base.clone(); nodes.sort_by_key(|&n| (keys[n].0, n));
        let mut edges = base.clone(); edges.sort_by_key(|&n| (keys[n].1, n));
        let mut roots = base.clone(); roots.sort_by_key(|&n| (keys[n].2, n));
        prop_assert_eq!(serde_json::to_vec(&ordered_graph(&base, &base, &base)).unwrap(), serde_json::to_vec(&ordered_graph(&nodes, &edges, &roots)).unwrap());
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d325f6772617068), .. ProptestConfig::default() })]
    #[test]
    fn conflicting_parent_claims_remain_visible_and_uncertain(parents in 2usize..7) {
        let (g, _) = graph(parents + 1, Completeness::Complete, false, false);
        let sessions: Vec<_> = g.nodes().iter().filter(|n| n.id.class == GraphNodeKind::Session).map(|n| n.id.clone()).collect();
        let child = sessions[0].clone();
        let edges: Vec<_> = sessions[1..].iter().map(|parent| ReferenceEdge {
            source: child.clone(), target: parent.clone(), kind: EdgeKind::SpawnedBy,
            evidence: vec![evidence(Confidence::Strong)],
        }).collect();
        let mut wire = serde_json::to_value(&g).unwrap();
        wire["edges"] = serde_json::to_value(edges).unwrap();
        let conflict: ArtifactGraph = serde_json::from_value(wire).unwrap();
        prop_assert_eq!(conflict.edges().len(), parents);
        prop_assert!(conflict.diagnostics().contains(&GraphDiagnostic::ConflictingParentage { child }), "missing parentage conflict");
        for node in sessions {
            let assessment = conflict.ownership_assessment(&node).unwrap();
            prop_assert_eq!(assessment.classification, OwnershipClassification::Unknown);
            prop_assert!(assessment.blockers.contains(&ProtectionReason::ConflictingEvidence));
            prop_assert!(conflict.node(&node).unwrap().is_protected());
        }
    }
    #[test]
    fn global_and_unknown_artifacts_remain_protected_with_arbitrary_references(count in 0usize..7, raw in 0usize..4) {
        let raw = [Confidence::Unknown, Confidence::Heuristic, Confidence::Strong, Confidence::Authoritative][raw];
        let ev = evidence(raw);
        let mut b = ArtifactGraph::builder(context(Completeness::Partial, EvidenceStatus::SyntheticOnly));
        b.add_rule(rule(Confidence::Authoritative)).unwrap();
        let global = b.observe(NodeObservation::ProviderState);
        b.add_root(ProtectedRoot { node: global.clone(), kind: RootKind::ProviderGlobal }).unwrap();
        let unknown = b.observe(NodeObservation::Artifact(Artifact {
            id: id("unknown"), kind: ArtifactKind::Unknown, locator: "/synthetic/unknown".into(), size_bytes: None, evidence: ev.observation.clone(),
        }));
        b.add_node_evidence(&unknown, ev.clone()).unwrap();
        for n in 0..count {
            let source = b.observe(session(n, &ev));
            b.add_node_evidence(&source, ev.clone()).unwrap();
            b.add_root(ProtectedRoot { node: source.clone(), kind: RootKind::ArchivedRetainedSession }).unwrap();
            for target in [&unknown, &global] {
                b.add_edge(ReferenceEdge { source: source.clone(), target: target.clone(), kind: EdgeKind::References, evidence: vec![ev.clone()] }).unwrap();
            }
        }
        let graph = b.finish().unwrap();
        prop_assert_eq!(graph.ownership_assessment(&global).unwrap().classification, OwnershipClassification::Global);
        prop_assert_eq!(graph.ownership_assessment(&unknown).unwrap().classification, OwnershipClassification::Unknown);
        prop_assert!(graph.node(&global).unwrap().is_protected());
        prop_assert!(graph.node(&unknown).unwrap().is_protected());
    }
}

#[cfg(unix)]
proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d325f6772617068), .. ProptestConfig::default() })]
    #[test]
    fn non_utf8_provenance_locator_round_trips(bytes in proptest::collection::vec(any::<u8>(), 0..32)) {
        use std::os::unix::ffi::OsStringExt;
        let mut native = b"/synthetic/".to_vec(); native.push(255); native.extend(bytes);
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(native));
        let mut ev = evidence(Confidence::Heuristic); ev.locator = Some(path.clone());
        let mut b = ArtifactGraph::builder(context(Completeness::Partial, EvidenceStatus::FixtureVerified));
        b.add_rule(rule(Confidence::Heuristic)).unwrap();
        let node = b.observe(NodeObservation::Artifact(Artifact { id: id("native"), kind: ArtifactKind::Transcript, locator: path.clone(), size_bytes: None, evidence: ev.observation.clone() }));
        b.add_node_evidence(&node, ev).unwrap();
        let g = b.finish().unwrap();
        let round: ArtifactGraph = serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
        prop_assert_eq!(round.node(&node).unwrap().evidence[0].locator.as_ref(), Some(&path));
        prop_assert_eq!(g, round);
    }
}

#[test]
fn user_protected_project_root_survives_round_trip_and_conflicts() {
    let ev = evidence(Confidence::Heuristic);
    let mut b = ArtifactGraph::builder(context(
        Completeness::Partial,
        EvidenceStatus::FixtureVerified,
    ));
    b.add_rule(rule(Confidence::Heuristic)).unwrap();
    let project = Project {
        id: id("protected-project"),
        workspace_id: None,
        provider_project_id: None,
        path: Some("/synthetic/project".into()),
        evidence: ev.observation.clone(),
    };
    let node = b.observe(NodeObservation::Project(project.clone()));
    b.observe(NodeObservation::Project(Project {
        path: Some("/synthetic/other".into()),
        ..project
    }));
    b.add_node_evidence(&node, ev).unwrap();
    let root = ProtectedRoot {
        node: node.clone(),
        kind: RootKind::UserProtected,
    };
    b.add_root(root.clone()).unwrap();
    let g = b.finish().unwrap();
    let round: ArtifactGraph = serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
    assert!(round.roots().contains(&root));
    let assessment = round.ownership_assessment(&node).unwrap();
    assert!(
        assessment
            .blockers
            .contains(&ProtectionReason::ProtectedRoot)
    );
    assert!(
        assessment
            .blockers
            .contains(&ProtectionReason::ConflictingEvidence)
    );
    assert_eq!(assessment.classification, OwnershipClassification::Unknown);
}
