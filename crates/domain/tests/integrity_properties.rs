use agentmender_domain::*;
use proptest::{prelude::*, test_runner::RngSeed};

fn id(s: &str) -> EntityId {
    EntityId::new(s).unwrap()
}
fn ev() -> GraphEvidence {
    GraphEvidence {
        observation: Evidence {
            source: EvidenceSource::Fixture,
            confidence: Confidence::Authoritative,
            provider_version: Some("v1".into()),
            rule: "raw".into(),
        },
        rule_id: id("rule"),
        nature: EvidenceNature::Observation,
        locator: None,
        observed_at: None,
    }
}
fn rule() -> EvidenceRule {
    EvidenceRule {
        id: id("rule"),
        provider: ProviderKind::Codex,
        description: "synthetic".into(),
        admissible_sources: vec![EvidenceSource::Fixture],
        max_confidence: Confidence::Authoritative,
        adapter_version: "a1".into(),
        interface_id: Some("i1".into()),
        layout_id: Some("l1".into()),
    }
}
fn graph_context() -> GraphContext {
    GraphContext {
        provider: Provider {
            kind: ProviderKind::Codex,
            presence: Presence::StateOnly,
            state_root: "/synthetic".into(),
            fingerprint: ProviderFingerprint {
                executable: None,
                executable_version: Some("v1".into()),
                adapter_version: "a1".into(),
                interface_id: Some("i1".into()),
                layout_id: Some("l1".into()),
            },
            capabilities: ProviderCapabilities::new(),
            compatibility: CompatibilityProfile {
                status: CompatibilityStatus::ReadOnlyKnown,
                interface_stability: InterfaceStability::OfficialStable,
                reason: "synthetic".into(),
            },
            installations: vec![],
            evidence_status: EvidenceStatus::SyntheticOnly,
            diagnostics: vec![],
        },
        evidence_status: EvidenceStatus::SyntheticOnly,
        completeness: Completeness::Complete,
        coverage: "synthetic full namespace".into(),
        diagnostics: vec![],
    }
}
fn session(name: &str) -> NodeObservation {
    NodeObservation::Session(Session {
        id: id(name),
        provider_session_id: name.into(),
        project_id: None,
        state: SessionState::Archived,
        last_active: None,
        evidence: ev().observation,
    })
}
fn artifact(name: &str, kind: ArtifactKind) -> NodeObservation {
    NodeObservation::Artifact(Artifact {
        id: id(name),
        kind,
        locator: format!("/synthetic/{name}").into(),
        size_bytes: None,
        evidence: ev().observation,
    })
}
fn aid(class: GraphNodeKind, name: &str) -> ArtifactId {
    ArtifactId {
        provider: ProviderKind::Codex,
        state_root: "/synthetic".into(),
        class,
        local_id: id(name),
    }
}
fn contract() -> IntegrityContract {
    IntegrityContract {
        id: id("contract"),
        provider_version: "v1".into(),
        adapter_version: "a1".into(),
        interface_id: Some("i1".into()),
        layout_id: Some("l1".into()),
        coherent: true,
        evidence: vec![ev()],
    }
}
fn coverage(class: GraphNodeKind) -> NodeCoverage {
    NodeCoverage {
        class,
        completeness: Completeness::Complete,
        evidence: vec![ev()],
    }
}
fn relation() -> RelationshipCoverage {
    RelationshipCoverage {
        source_class: GraphNodeKind::Session,
        target_class: GraphNodeKind::Artifact,
        kind: EdgeKind::References,
        completeness: Completeness::Complete,
        evidence: vec![ev()],
    }
}
fn build(
    owner: bool,
    target: bool,
    diagnostic: Option<DiagnosticCode>,
    wrong_class: bool,
) -> (ArtifactGraph, IntegrityContext) {
    build_with_owner_rule(owner, target, diagnostic, wrong_class, false)
}
fn build_with_owner_rule(
    owner: bool,
    target: bool,
    diagnostic: Option<DiagnosticCode>,
    wrong_class: bool,
    nonowner: bool,
) -> (ArtifactGraph, IntegrityContext) {
    let mut gc = graph_context();
    if let Some(code) = diagnostic {
        gc.diagnostics.push(Diagnostic {
            code,
            message: "synthetic".into(),
            path: None,
        });
    }
    let mut b = ArtifactGraph::builder(gc);
    b.add_rule(rule()).unwrap();
    let mut other_rule = rule();
    other_rule.id = id("other-rule");
    b.add_rule(other_rule).unwrap();
    let source = b.observe(session("source"));
    b.add_node_evidence(&source, ev()).unwrap();
    let orphan = b.observe(artifact("orphan", ArtifactKind::Plan));
    b.add_node_evidence(&orphan, ev()).unwrap();
    let missing = if target {
        let node = b.observe(artifact("missing", ArtifactKind::Transcript));
        b.add_node_evidence(&node, ev()).unwrap();
        node
    } else if wrong_class {
        let node = b.observe(session("missing"));
        b.add_node_evidence(&node, ev()).unwrap();
        b.observe(NodeObservation::Unresolved { id: id("missing") })
    } else {
        b.observe(NodeObservation::Unresolved { id: id("missing") })
    };
    b.add_edge(ReferenceEdge {
        source: source.clone(),
        target: missing,
        kind: EdgeKind::References,
        evidence: vec![ev()],
    })
    .unwrap();
    if owner {
        let mut owner_evidence = ev();
        if nonowner {
            owner_evidence.rule_id = id("other-rule");
        }
        b.add_edge(ReferenceEdge {
            source: source.clone(),
            target: orphan.clone(),
            kind: EdgeKind::References,
            evidence: vec![owner_evidence],
        })
        .unwrap();
    }
    let ctx = IntegrityContext {
        contract: Some(contract()),
        node_coverage: vec![
            coverage(GraphNodeKind::Session),
            coverage(GraphNodeKind::Artifact),
        ],
        relationship_coverage: vec![relation()],
        required_targets: vec![RequiredTarget {
            source,
            target: aid(GraphNodeKind::Artifact, "missing"),
            edge_kind: EdgeKind::References,
            kind: RequiredTargetKind::Transcript,
            evidence: vec![ev()],
        }],
        required_owners: vec![RequiredOwner {
            artifact: orphan,
            relations: vec![OwnerRelation {
                owner_class: GraphNodeKind::Session,
                edge_kind: EdgeKind::References,
                direction: OwnerDirection::OwnerToArtifact,
                rule_ids: vec![id("rule")],
            }],
            evidence: vec![ev()],
        }],
        residuals: vec![],
    };
    (b.finish().unwrap(), ctx)
}
fn has(r: &IntegrityReport, k: IntegrityIssueKind) -> bool {
    r.findings
        .iter()
        .any(|f| f.kind == k && f.disposition == IssueDisposition::EstablishedViolation)
}

proptest! {
 #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d335f696e746567), .. ProptestConfig::default() })]
 #[test]
 fn closed_witness_and_weakening(mode in 0u8..6) {
    let (g, mut c) = build(false, false, None, false);
    let baseline = evaluate_integrity(&g, &c);
    prop_assert!(has(&baseline, IntegrityIssueKind::OrphanPlan));
    prop_assert!(has(&baseline, IntegrityIssueKind::MissingTranscript));
    let mut wire = serde_json::to_value(&g).unwrap();
    match mode { 0 => wire["context"]["completeness"] = serde_json::json!("partial"), 1 => c.node_coverage[1].completeness = Completeness::Partial, 2 => c.relationship_coverage[0].completeness = Completeness::Partial, 3 => wire["context"]["provider"]["compatibility"]["status"] = serde_json::json!("audit_only_unknown"), 4 => c.contract.as_mut().unwrap().evidence[0].observation.provider_version = Some("other".into()), _ => c.contract.as_mut().unwrap().coherent = false }
    let weakened: ArtifactGraph = serde_json::from_value(wire).unwrap(); let r = evaluate_integrity(&weakened, &c);
    if mode != 2 { prop_assert!(!has(&r, IntegrityIssueKind::MissingTranscript)); }
    prop_assert!(!has(&r, IntegrityIssueKind::OrphanPlan));
    prop_assert_ne!(r.verification_status, VerificationStatus::Healthy);
 }
 #[test]
 fn added_owner_and_target_defeat_absence(owner in any::<bool>(), target in any::<bool>()) {
    let (g,c) = build(owner,target,None,false); let r = evaluate_integrity(&g,&c);
    prop_assert_eq!(has(&r, IntegrityIssueKind::OrphanPlan), !owner);
    prop_assert_eq!(has(&r, IntegrityIssueKind::MissingTranscript), !target);
    prop_assert!(g.nodes().iter().all(ArtifactNode::is_protected));
 }
 #[test]
 fn permutations_and_exact_duplicates_are_canonical(reverse in any::<bool>(), duplicates in any::<bool>()) {
    let (g, mut c) = build(false,false,None,false); let expected = serde_json::to_vec(&evaluate_integrity(&g,&c)).unwrap();
    if reverse { c.node_coverage.reverse(); c.required_owners[0].relations.reverse(); }
    if duplicates { c.node_coverage.push(c.node_coverage[0].clone()); c.required_targets.push(c.required_targets[0].clone()); c.contract.as_mut().unwrap().evidence.push(ev()); }
    prop_assert_eq!(expected, serde_json::to_vec(&evaluate_integrity(&g,&c)).unwrap());
 }
 #[test]
 fn stale_and_concurrent_are_not_negative_proof(stale in any::<bool>()) {
    let (g,c) = build(false,false,Some(if stale { DiagnosticCode::SnapshotStale } else { DiagnosticCode::ConcurrentChange }),false); let r=evaluate_integrity(&g,&c);
    prop_assert!(!has(&r, IntegrityIssueKind::OrphanPlan)); prop_assert!(!has(&r, IntegrityIssueKind::MissingTranscript));
    prop_assert!(r.findings.iter().all(|f| f.disposition != IssueDisposition::EstablishedViolation));
 }
 #[test]
 fn unknown_residual_is_never_safe(which in 0u8..2) {
    let (g,mut c) = build(false,false,None,false); c.contract = None;
    c.residuals.push(ResidualObservation { artifact: aid(GraphNodeKind::Artifact,"orphan"), expectation_evidence: if which == 0 { vec![] } else { vec![ev()] } });
    let r=evaluate_integrity(&g,&c); prop_assert_eq!(r.residuals[0].classification, ResidualClassification::UnknownResidual);
 }
}

#[test]
fn wrong_class_is_ambiguous_and_not_absence() {
    let (g, c) = build(false, false, None, true);
    assert!(!has(
        &evaluate_integrity(&g, &c),
        IntegrityIssueKind::MissingTranscript
    ));
}

#[test]
fn required_target_kinds_have_nonvacuous_closed_witnesses() {
    for (kind, class, expected) in [
        (
            RequiredTargetKind::Reference,
            GraphNodeKind::Artifact,
            IntegrityIssueKind::DanglingReference,
        ),
        (
            RequiredTargetKind::Transcript,
            GraphNodeKind::Artifact,
            IntegrityIssueKind::MissingTranscript,
        ),
        (
            RequiredTargetKind::Child,
            GraphNodeKind::Session,
            IntegrityIssueKind::MissingChild,
        ),
        (
            RequiredTargetKind::DatabaseReference,
            GraphNodeKind::Artifact,
            IntegrityIssueKind::BrokenDatabaseReference,
        ),
    ] {
        let (g, mut c) = build(false, false, None, false);
        c.required_targets[0].kind = kind;
        c.required_targets[0].target.class = class;
        assert!(
            has(&evaluate_integrity(&g, &c), expected),
            "missing {expected:?} witness"
        );
        c.node_coverage
            .iter_mut()
            .find(|v| v.class == class)
            .unwrap()
            .completeness = Completeness::Partial;
        assert!(!has(&evaluate_integrity(&g, &c), expected));
    }
}

#[test]
fn provenance_only_variants_are_not_identity_corruption() {
    let mut b = ArtifactGraph::builder(graph_context());
    b.add_rule(rule()).unwrap();
    let first = artifact("same", ArtifactKind::Plan);
    let node = b.observe(first.clone());
    let mut second = first;
    if let NodeObservation::Artifact(ref mut a) = second {
        a.evidence.rule = "other provenance".into();
    }
    b.observe(second);
    b.add_node_evidence(&node, ev()).unwrap();
    let mut provenance = ev();
    provenance.observation.rule = "other provenance".into();
    b.add_node_evidence(&node, provenance).unwrap();
    let g = b.finish().unwrap();
    let r = evaluate_integrity(&g, &IntegrityContext::default());
    assert!(
        !r.findings
            .iter()
            .any(|f| f.kind == IntegrityIssueKind::ConflictingIdentityEvidence)
    );

    let mut b = ArtifactGraph::builder(graph_context());
    b.add_rule(rule()).unwrap();
    let node = b.observe(artifact("same", ArtifactKind::Plan));
    b.observe(artifact("same", ArtifactKind::Transcript));
    b.add_node_evidence(&node, ev()).unwrap();
    let r = evaluate_integrity(&b.finish().unwrap(), &IntegrityContext::default());
    assert!(r.findings.iter().any(
        |f| f.kind == IntegrityIssueKind::ConflictingIdentityEvidence
            && f.disposition == IssueDisposition::SuspiciousInconsistency
    ));
}

#[test]
fn conflicting_parent_claims_preserve_all_authoritative_evidence() {
    let mut b = ArtifactGraph::builder(graph_context());
    b.add_rule(rule()).unwrap();
    let child = b.observe(session("child"));
    b.add_node_evidence(&child, ev()).unwrap();
    let mut parents = Vec::new();
    for name in ["parent-a", "parent-b"] {
        let parent = b.observe(session(name));
        b.add_node_evidence(&parent, ev()).unwrap();
        let mut evidence = ev();
        evidence.locator = Some(format!("/synthetic/{name}").into());
        b.add_edge(ReferenceEdge {
            source: child.clone(),
            target: parent.clone(),
            kind: EdgeKind::SpawnedBy,
            evidence: vec![evidence],
        })
        .unwrap();
        parents.push(parent);
    }
    let report = evaluate_integrity(&b.finish().unwrap(), &IntegrityContext::default());
    let finding = report
        .findings
        .iter()
        .find(|f| f.kind == IntegrityIssueKind::ConflictingRelationshipEvidence)
        .unwrap();
    assert!(parents.iter().all(|p| finding.affected.contains(p)));
    assert_eq!(finding.evidence.len(), 2);
    assert!(
        finding
            .evidence
            .iter()
            .all(|e| e.confidence == Confidence::Authoritative)
    );
}

proptest! {
 #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d335f696e746567), .. ProptestConfig::default() })]
 #[test]
 fn excluded_known_rule_is_not_an_owner(nonowner in any::<bool>()) {
    let (g,c) = build_with_owner_rule(true,false,None,false,nonowner);
    prop_assert_eq!(has(&evaluate_integrity(&g,&c), IntegrityIssueKind::OrphanPlan), nonowner);
 }
 #[test]
 fn contradictory_coverage_never_proves_absence(relation_conflict in any::<bool>()) {
    let (g,mut c)=build(false,false,None,false);
    let baseline=evaluate_integrity(&g,&c);
    prop_assert!(has(&baseline, IntegrityIssueKind::OrphanPlan));
    prop_assert!(has(&baseline, IntegrityIssueKind::MissingTranscript));
    if relation_conflict { let mut v=c.relationship_coverage[0].clone(); v.completeness=Completeness::Partial; c.relationship_coverage.push(v); }
    else { let mut v=c.node_coverage[1].clone(); v.completeness=Completeness::Partial; c.node_coverage.push(v); }
    let report=evaluate_integrity(&g,&c);
    prop_assert!(!has(&report, IntegrityIssueKind::OrphanPlan));
    if !relation_conflict { prop_assert!(!has(&report, IntegrityIssueKind::MissingTranscript)); }
 }
 #[test]
 fn global_and_unknown_nodes_remain_protected(count in 0usize..8) {
    let mut b=ArtifactGraph::builder(graph_context()); b.add_rule(rule()).unwrap();
    let global=b.observe(NodeObservation::ProviderState);
    b.add_root(ProtectedRoot { node: global.clone(), kind: RootKind::ProviderGlobal }).unwrap();
    let unknown=b.observe(artifact("unknown",ArtifactKind::Unknown)); b.add_node_evidence(&unknown,ev()).unwrap();
    for n in 0..count { let source=b.observe(session(&format!("source-{n}"))); b.add_node_evidence(&source,ev()).unwrap(); b.add_edge(ReferenceEdge { source, target: unknown.clone(), kind: EdgeKind::References, evidence: vec![ev()] }).unwrap(); }
    let g=b.finish().unwrap(); let report=evaluate_integrity(&g,&IntegrityContext::default());
    prop_assert!(g.node(&global).unwrap().is_protected()); prop_assert!(g.node(&unknown).unwrap().is_protected());
    prop_assert_eq!(g.ownership_assessment(&global).unwrap().classification,OwnershipClassification::Global);
    prop_assert_eq!(g.ownership_assessment(&unknown).unwrap().classification,OwnershipClassification::Unknown);
    prop_assert!(report.findings.iter().all(|f| f.disposition != IssueDisposition::EstablishedViolation));
    let value=serde_json::to_value(report).unwrap();
    prop_assert!(value.get("removal_permission").is_none());
 }
 #[test]
 fn authoritative_conflicting_parents_keep_every_claim(count in 2usize..7) {
    let mut b=ArtifactGraph::builder(graph_context()); b.add_rule(rule()).unwrap();
    let child=b.observe(session("child")); b.add_node_evidence(&child,ev()).unwrap();
    let mut parents=Vec::new();
    for n in 0..count { let name=format!("parent-{n}"); let parent=b.observe(session(&name)); b.add_node_evidence(&parent,ev()).unwrap(); let mut evidence=ev(); evidence.locator=Some(format!("/synthetic/{name}").into()); b.add_edge(ReferenceEdge { source:child.clone(), target:parent.clone(), kind:EdgeKind::SpawnedBy, evidence:vec![evidence] }).unwrap(); parents.push(parent); }
    let r=evaluate_integrity(&b.finish().unwrap(),&IntegrityContext::default());
    let f=r.findings.iter().find(|f| f.kind == IntegrityIssueKind::ConflictingRelationshipEvidence).unwrap();
    prop_assert!(parents.iter().all(|p| f.affected.contains(p))); prop_assert_eq!(f.evidence.len(),count);
    prop_assert!(f.evidence.iter().all(|e| e.confidence == Confidence::Authoritative));
 }
 #[test]
 fn unrelated_authoritative_records_do_not_repair_wrong_producer(suffix in 0u16..1000) {
    let (g,mut c)=build(false,false,None,false); c.contract.as_mut().unwrap().evidence[0].observation.provider_version=Some("wrong".into());
    let initial=evaluate_integrity(&g,&c); prop_assert!(!has(&initial, IntegrityIssueKind::OrphanPlan)); prop_assert!(!has(&initial, IntegrityIssueKind::MissingTranscript));
    let mut unrelated=coverage(GraphNodeKind::Project); unrelated.evidence[0].locator=Some(format!("/synthetic/extra-{suffix}").into()); c.node_coverage.push(unrelated);
    let r=evaluate_integrity(&g,&c); prop_assert!(!has(&r, IntegrityIssueKind::OrphanPlan)); prop_assert!(!has(&r, IntegrityIssueKind::MissingTranscript));
 }
}
