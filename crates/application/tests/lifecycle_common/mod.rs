use agentmender_domain::*;

fn id(value: &str) -> EntityId {
    EntityId::new(value).unwrap()
}
fn evidence() -> GraphEvidence {
    GraphEvidence {
        observation: Evidence {
            source: EvidenceSource::Fixture,
            confidence: Confidence::Authoritative,
            provider_version: Some("synthetic-v1".into()),
            rule: "synthetic explicit identity".into(),
        },
        rule_id: id("synthetic-identity"),
        nature: EvidenceNature::Observation,
        locator: None,
        observed_at: Some("2026-10-28T00:00:00Z".into()),
    }
}
pub struct Case {
    pub graph: ArtifactGraph,
    pub proof: IntegrityContext,
    pub planning: PlanningContext,
    target: ArtifactId,
}
impl Case {
    pub fn plan(&self, intent: LifecycleIntent) -> LifecyclePlan {
        plan_lifecycle(
            &self.graph,
            &evaluate_integrity(&self.graph, &self.proof),
            &self.planning,
            LifecycleRequest {
                target: self.target.clone(),
                intent,
            },
        )
    }
}
pub fn case(extra: usize) -> Case {
    let ev = evidence();
    let fingerprint = ProviderFingerprint {
        executable: None,
        executable_version: Some("synthetic-v1".into()),
        adapter_version: "synthetic-adapter-v1".into(),
        interface_id: Some("synthetic-interface-v1".into()),
        layout_id: Some("synthetic-layout-v1".into()),
    };
    let provider = Provider {
        kind: ProviderKind::Codex,
        presence: Presence::StateOnly,
        state_root: "/synthetic-planning".into(),
        fingerprint: fingerprint.clone(),
        capabilities: ProviderCapabilities::new(),
        compatibility: CompatibilityProfile {
            status: CompatibilityStatus::ReadOnlyKnown,
            interface_stability: InterfaceStability::OfficialStable,
            reason: "synthetic only".into(),
        },
        installations: vec![],
        evidence_status: EvidenceStatus::SyntheticOnly,
        diagnostics: vec![],
    };
    let mut b = ArtifactGraph::builder(GraphContext {
        provider,
        evidence_status: EvidenceStatus::SyntheticOnly,
        completeness: Completeness::Complete,
        coverage: "synthetic full boundary".into(),
        diagnostics: vec![],
    });
    let identity_rule = EvidenceRule {
        id: ev.rule_id.clone(),
        provider: ProviderKind::Codex,
        description: "synthetic authoritative identity".into(),
        admissible_sources: vec![EvidenceSource::Fixture, EvidenceSource::ProviderApi],
        max_confidence: Confidence::Authoritative,
        adapter_version: fingerprint.adapter_version.clone(),
        interface_id: fingerprint.interface_id.clone(),
        layout_id: fingerprint.layout_id.clone(),
    };
    b.add_rule(identity_rule.clone()).unwrap();
    let mut effect_rule = identity_rule;
    effect_rule.id = id("synthetic-native-effects");
    effect_rule.description = "synthetic exact exhaustive native effects contract".into();
    b.add_rule(effect_rule).unwrap();
    let global = b.observe(NodeObservation::ProviderState);
    b.add_root(ProtectedRoot {
        node: global,
        kind: RootKind::ProviderGlobal,
    })
    .unwrap();
    let target = b.observe(NodeObservation::Session(Session {
        id: id("session:target"),
        provider_session_id: "native-target".into(),
        project_id: None,
        state: SessionState::Archived,
        last_active: Some("2026-10-27T00:00:00Z".into()),
        evidence: ev.observation.clone(),
    }));
    b.add_node_evidence(&target, ev.clone()).unwrap();
    b.add_root(ProtectedRoot {
        node: target.clone(),
        kind: RootKind::ArchivedRetainedSession,
    })
    .unwrap();
    b.add_edge(ReferenceEdge {
        source: target.clone(),
        target: target.clone(),
        kind: EdgeKind::References,
        evidence: vec![ev.clone()],
    })
    .unwrap();
    for n in 0..extra {
        let node = b.observe(NodeObservation::Artifact(Artifact {
            id: id(&format!("artifact:{n}")),
            kind: ArtifactKind::Attachment,
            locator: format!("/synthetic-planning/artifact-{n}").into(),
            size_bytes: Some(n as u64 + 1),
            evidence: ev.observation.clone(),
        }));
        b.add_node_evidence(&node, ev.clone()).unwrap();
    }
    let mut classes = vec![GraphNodeKind::ProviderState, GraphNodeKind::Session];
    if extra > 0 {
        classes.push(GraphNodeKind::Artifact);
    }
    let proof = IntegrityContext {
        contract: Some(IntegrityContract {
            id: id("synthetic-closed-contract"),
            provider_version: "synthetic-v1".into(),
            adapter_version: fingerprint.adapter_version.clone(),
            interface_id: fingerprint.interface_id.clone(),
            layout_id: fingerprint.layout_id.clone(),
            coherent: true,
            evidence: vec![ev.clone()],
        }),
        node_coverage: classes
            .into_iter()
            .map(|class| NodeCoverage {
                class,
                completeness: Completeness::Complete,
                evidence: vec![ev.clone()],
            })
            .collect(),
        relationship_coverage: vec![RelationshipCoverage {
            source_class: GraphNodeKind::Session,
            target_class: GraphNodeKind::Session,
            kind: EdgeKind::References,
            completeness: Completeness::Complete,
            evidence: vec![ev.clone()],
        }],
        required_targets: vec![RequiredTarget {
            source: target.clone(),
            target: target.clone(),
            edge_kind: EdgeKind::References,
            kind: RequiredTargetKind::Reference,
            evidence: vec![ev.clone()],
        }],
        ..Default::default()
    };
    let capability = LifecycleCapability {
        provider: ProviderKind::Codex,
        fingerprint,
        operation: NativeLifecycleOperation::UnarchiveSession,
        protocol_evidence_digest: format!("sha256:{}", "a".repeat(64)),
        effect_scope: NativeEffectScope::TargetRestoreWithProviderManagedState,
    };
    let mut effect_evidence = ev.clone();
    effect_evidence.rule_id = id("synthetic-native-effects");
    effect_evidence.observation.rule = "synthetic exhaustive native effect proof".into();
    let planning = PlanningContext {
        scope: DiscoveryScope::default(),
        capabilities: vec![capability.clone()],
        cleanup_evidence: None,
        effects: Some(NativeEffectBoundary {
            contract_id: id("synthetic-archive-effects-v1"),
            capability,
            effect_rule_ids: vec![id("synthetic-native-effects")],
            target: target.clone(),
            operation: NativeLifecycleOperation::UnarchiveSession,
            affected: vec![target.clone()],
            completeness: Completeness::Complete,
            coherent: true,
            evidence: vec![effect_evidence],
        }),
    };
    Case {
        graph: b.finish().unwrap(),
        proof,
        planning,
        target,
    }
}
