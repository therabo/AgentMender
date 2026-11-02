use agentmender_domain::*;
use proptest::{prelude::*, test_runner::RngSeed};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

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
struct Case {
    graph: ArtifactGraph,
    proof: IntegrityContext,
    planning: PlanningContext,
    target: ArtifactId,
}
impl Case {
    fn plan(&self, intent: LifecycleIntent) -> LifecyclePlan {
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
    fn change_graph(&mut self, change: impl FnOnce(&mut Value)) {
        let mut wire = serde_json::to_value(&self.graph).unwrap();
        change(&mut wire);
        self.graph = serde_json::from_value(wire).unwrap();
    }
    fn validate(&self, plan: &LifecyclePlan) -> PlanValidation {
        plan.validate_against(
            &self.graph,
            &evaluate_integrity(&self.graph, &self.proof),
            &self.planning,
        )
    }
}
fn case(extra: usize) -> Case {
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
        state: SessionState::Active,
        last_active: Some("2026-10-27T00:00:00Z".into()),
        evidence: ev.observation.clone(),
    }));
    b.add_node_evidence(&target, ev.clone()).unwrap();
    b.add_root(ProtectedRoot {
        node: target.clone(),
        kind: RootKind::ActiveSession,
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
        operation: NativeLifecycleOperation::ArchiveSession,
        protocol_evidence_digest: format!("sha256:{}", "a".repeat(64)),
        effect_scope: NativeEffectScope::TargetOnly,
    };
    let mut effect_evidence = ev.clone();
    effect_evidence.rule_id = id("synthetic-native-effects");
    effect_evidence.observation.rule = "synthetic exhaustive native effect proof".into();
    let planning = PlanningContext {
        complete_removal_evidence: None,
        cleanup_evidence: None,
        scope: DiscoveryScope::default(),
        capabilities: vec![capability.clone()],
        effects: Some(NativeEffectBoundary {
            contract_id: id("synthetic-archive-effects-v1"),
            capability,
            effect_rule_ids: vec![id("synthetic-native-effects")],
            target: target.clone(),
            operation: NativeLifecycleOperation::ArchiveSession,
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

fn reverse_sets(value: &mut Value) {
    match value {
        Value::Array(values) => {
            values.reverse();
            values.iter_mut().for_each(reverse_sets);
        }
        Value::Object(values) => values.values_mut().for_each(reverse_sets),
        _ => (),
    }
}
fn target_node(wire: &mut Value) -> &mut Value {
    wire["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"]["local_id"] == "session:target")
        .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d345f706c616e73), .. ProptestConfig::default() })]

    #[test]
    fn complete_synthetic_archive_is_ready_without_filesystem_exclusivity(extra in 0usize..8) {
        let c = case(extra);
        let report = evaluate_integrity(&c.graph, &c.proof);
        prop_assert_eq!(report.verification_status, VerificationStatus::Healthy);
        let plan = c.plan(LifecycleIntent::Archive);
        prop_assert_eq!(plan.content().status, PlanStatus::Ready, "{:?}", plan.content().blockers);
        prop_assert_eq!(c.validate(&plan), PlanValidation::Current);
        prop_assert!(plan.content().requires_human_approval);
        prop_assert!(plan.content().artifacts.iter().all(|a| a.ownership.classification != OwnershipClassification::Exclusive));
        prop_assert_eq!(plan.content().operations.len(), 1);
        prop_assert_eq!(&plan.content().operations[0].affected, &vec![c.target]);
    }

    #[test]
    fn independent_input_permutations_and_duplicates_are_canonical(extra in 1usize..8, duplicate in any::<bool>()) {
        let mut c = case(extra);
        let first = c.plan(LifecycleIntent::Archive);
        c.change_graph(|wire| {
            reverse_sets(wire);
            if duplicate {
                let node = wire["nodes"][0].clone();
                wire["nodes"].as_array_mut().unwrap().push(node);
                let rule = wire["rules"][0].clone();
                wire["rules"].as_array_mut().unwrap().push(rule);
            }
        });
        c.proof.node_coverage.reverse();
        c.proof.required_targets.reverse();
        c.planning.capabilities.reverse();
        if duplicate {
            c.proof.node_coverage.push(c.proof.node_coverage[0].clone());
            c.planning.capabilities.push(c.planning.capabilities[0].clone());
        }
        let second = c.plan(LifecycleIntent::Archive);
        prop_assert_eq!(first.digest(), second.digest());
        prop_assert_eq!(first.state_fingerprint(), second.state_fingerprint());
    }

    #[test]
    fn harmless_presentation_changes_do_not_change_identity(extra in 0usize..5, suffix in 0u32..10000) {
        let mut c = case(extra);
        let first = c.plan(LifecycleIntent::Archive);
        c.change_graph(|wire| {
            wire["context"]["coverage"] = json!(format!("wording-{suffix}"));
            wire["context"]["provider"]["compatibility"]["reason"] = json!(format!("reason-{suffix}"));
            wire["rules"][0]["description"] = json!(format!("description-{suffix}"));
        });
        let report = evaluate_integrity(&c.graph, &c.proof);
        let second = plan_lifecycle(&c.graph, &report, &c.planning, first.content().request.clone());
        prop_assert_eq!(first.digest(), second.digest());
    }

    #[test]
    fn material_state_changes_always_invalidate_the_old_plan(field in 0u8..12) {
        let mut c = case(2);
        let original = c.plan(LifecycleIntent::Archive);
        match field {
            0 => c.change_graph(|w| w["context"]["provider"]["fingerprint"]["executable_version"] = json!("new-version")),
            1 => c.change_graph(|w| w["context"]["provider"]["fingerprint"]["layout_id"] = json!("new-layout")),
            2 => c.change_graph(|w| target_node(w)["observations"][0]["value"]["provider_session_id"] = json!("replacement-native-id")),
            3 => c.change_graph(|w| target_node(w)["observations"][0]["value"]["last_active"] = json!("2026-10-28T01:00:00Z")),
            4 => c.change_graph(|w| w["rules"][0]["max_confidence"] = json!("heuristic")),
            5 => c.change_graph(|w| w["context"]["provider"]["compatibility"]["status"] = json!("audit_only_unknown")),
            6 => c.planning.scope.max_entries += 1,
            7 => c.planning.capabilities[0].protocol_evidence_digest = format!("sha256:{}", "b".repeat(64)),
            8 => c.proof.contract.as_mut().unwrap().coherent = false,
            9 => c.planning.effects.as_mut().unwrap().coherent = false,
            10 => c.change_graph(|w| w["context"]["evidence_status"] = json!("unverified")),
            _ => c.change_graph(|w| w["roots"].as_array_mut().unwrap().push(json!({"node":target_node(&mut serde_json::to_value(&case(0).graph).unwrap())["id"],"kind":"user_protected"}))),
        }
        let changed=c.plan(LifecycleIntent::Archive);
        prop_assert_ne!(original.state_fingerprint(), changed.state_fingerprint());
        prop_assert_eq!(c.validate(&original), PlanValidation::Stale);
        prop_assert_eq!(original.content().status, PlanStatus::Ready);
    }

    #[test]
    fn partial_unknown_stale_concurrent_and_conflicts_cannot_be_ready(mode in 0u8..8) {
        let mut c = case(1);
        match mode {
            0 => c.change_graph(|w| w["context"]["completeness"] = json!("partial")),
            1 => c.proof.node_coverage[0].completeness = Completeness::Partial,
            2 => c.change_graph(|w| w["context"]["provider"]["compatibility"]["status"] = json!("audit_only_unknown")),
            3 | 4 => c.change_graph(|w| w["context"]["diagnostics"] = json!([{"code":if mode == 3 {"snapshot_stale"} else {"concurrent_change"},"message":"presentation","path":null}])),
            5 => c.proof = IntegrityContext::default(),
            6 => c.planning.effects = None,
            _ => c.change_graph(|w| { let mut rule=w["rules"][0].clone();rule["description"]=json!("conflicting description only");w["rules"].as_array_mut().unwrap().push(rule); }),
        }
        let plan=c.plan(LifecycleIntent::Archive);
        prop_assert_ne!(plan.content().status, PlanStatus::Ready);
        prop_assert!(plan.content().operations.is_empty());
    }

    #[test]
    fn protected_targets_and_effect_members_never_enter_operations(kind in 0u8..3, descendant in any::<bool>()) {
        let mut c=case(0);
        let root_kind=match kind {0=>"user_protected",1=>"provider_global",_=>"unknown_state"};
        if descendant {
            c.planning.capabilities[0].effect_scope=NativeEffectScope::TargetAndAttemptedDescendants;
            c.planning.effects.as_mut().unwrap().capability=c.planning.capabilities[0].clone();
            let mut node=serde_json::to_value(c.graph.node(&c.target).unwrap()).unwrap();
            node["id"]["local_id"]=json!("session:child");
            node["observations"][0]["value"]["id"]=json!("session:child");
            node["observations"][0]["value"]["provider_session_id"]=json!("native-child");
            let child:ArtifactId=serde_json::from_value(node["id"].clone()).unwrap();
            c.change_graph(|w| {w["nodes"].as_array_mut().unwrap().push(node);w["roots"].as_array_mut().unwrap().push(json!({"node":child,"kind":root_kind}));});
            c.planning.effects.as_mut().unwrap().affected.push(child);
        } else {
            let target=c.target.clone();
            c.change_graph(|w| w["roots"].as_array_mut().unwrap().push(json!({"node":target,"kind":root_kind})));
        }
        let plan=c.plan(LifecycleIntent::Archive);
        prop_assert_ne!(plan.content().status,PlanStatus::Ready);
        prop_assert!(plan.content().operations.is_empty());
    }

    #[test]
    fn purge_never_turns_descriptive_ownership_into_exclusive_operations(extra in 0usize..6) {
        let mut c=case(extra);
        c.planning.capabilities[0].operation=NativeLifecycleOperation::DeleteSession;
        c.planning.effects.as_mut().unwrap().operation=NativeLifecycleOperation::DeleteSession;
        c.planning.effects.as_mut().unwrap().capability=c.planning.capabilities[0].clone();
        let plan=c.plan(LifecycleIntent::Purge);
        prop_assert_ne!(plan.content().status,PlanStatus::Ready);
        prop_assert!(plan.content().operations.is_empty());
        prop_assert!(plan.content().artifacts.iter().all(|a| a.ownership.classification != OwnershipClassification::Exclusive));
    }

    #[test]
    fn unsupported_intents_and_capabilities_cannot_produce_operations(intent in 0u8..3) {
        let mut c=case(0);
        let intent=match intent {0=>LifecycleIntent::SoftDelete,1=>LifecycleIntent::Repair,_=>{c.planning.capabilities.clear();LifecycleIntent::Purge}};
        let plan=c.plan(intent);
        prop_assert_eq!(plan.content().status,PlanStatus::Unsupported);
        prop_assert!(plan.content().operations.is_empty());
    }

    #[test]
    fn serialization_roundtrip_and_json_order_preserve_the_bound_plan(extra in 0usize..8) {
        let c=case(extra);
        let original=c.plan(LifecycleIntent::Archive);
        let mut wire=serde_json::to_value(&original).unwrap();
        reverse_sets(&mut wire);
        let round:LifecyclePlan=serde_json::from_value(wire).unwrap();
        prop_assert_eq!(original.digest(),round.digest());
        prop_assert_eq!(round.validate_against(&c.graph,&evaluate_integrity(&c.graph,&c.proof),&c.planning),PlanValidation::Current);
    }

    #[test]
    fn semantic_wire_tampering_is_rejected(field in 0u8..6) {
        let c=case(1);let original=c.plan(LifecycleIntent::Archive);let mut wire=serde_json::to_value(original).unwrap();
        match field {0=>wire["content"]["requires_human_approval"]=json!(false),1=>wire["content"]["request"]["intent"]=json!("purge"),2=>wire["content"]["provider"]["adapter_version"]=json!("changed"),3=>wire["content"]["operations"][0]["native_id"]=json!("different-native-id"),4=>wire["content"]["space"]["physical"]["bytes"]=json!(123),_=>wire["content"]["preconditions"]=json!([])}
        prop_assert!(serde_json::from_value::<LifecyclePlan>(wire).is_err());
    }

    #[test]
    fn conflicting_capabilities_are_order_independent_and_blocked(reverse in any::<bool>()) {
        let mut c=case(0);let mut conflicting=c.planning.capabilities[0].clone();conflicting.effect_scope=NativeEffectScope::TargetAndDescendants;c.planning.capabilities.push(conflicting);
        let first=c.plan(LifecycleIntent::Archive);
        if reverse {c.planning.capabilities.reverse();}
        let second=c.plan(LifecycleIntent::Archive);
        prop_assert_ne!(first.content().status,PlanStatus::Ready);
        prop_assert_eq!(first.digest(),second.digest());
    }

    #[test]
    fn exhaustive_effect_contract_cannot_be_replaced_by_identity_or_another_capability(mode in 0u8..7) {
        let mut c=case(0);
        let original=c.plan(LifecycleIntent::Archive);
        let boundary=c.planning.effects.as_mut().unwrap();
        match mode {
            0=>boundary.evidence=vec![evidence()],
            1=>boundary.effect_rule_ids.clear(),
            2=>boundary.capability.protocol_evidence_digest=format!("sha256:{}","c".repeat(64)),
            3=>boundary.capability.effect_scope=NativeEffectScope::TargetAndAttemptedDescendants,
            4=>boundary.evidence[0].observation.confidence=Confidence::Heuristic,
            5=>boundary.affected.push(c.target.clone()),
            _=>boundary.completeness=Completeness::Partial,
        }
        let plan=c.plan(LifecycleIntent::Archive);
        prop_assert_ne!(plan.content().status,PlanStatus::Ready);
        prop_assert!(plan.content().operations.is_empty());
        prop_assert_eq!(plan.content().space.physical.bytes,None);
        prop_assert_ne!(plan.state_fingerprint(),original.state_fingerprint());
    }

    #[test]
    fn omitted_observed_descendants_block_even_a_claimed_target_only_archive(protected in any::<bool>(), cascading in any::<bool>()) {
        let mut c=case(0);
        let mut child=serde_json::to_value(c.graph.node(&c.target).unwrap()).unwrap();
        child["id"]["local_id"]=json!("session:child");
        child["observations"][0]["value"]["id"]=json!("session:child");
        child["observations"][0]["value"]["provider_session_id"]=json!("native-child");
        let child_id:ArtifactId=serde_json::from_value(child["id"].clone()).unwrap();
        let target=c.target.clone();
        c.change_graph(|w| {
            w["nodes"].as_array_mut().unwrap().push(child);
            w["roots"].as_array_mut().unwrap().push(json!({"node":child_id,"kind":if protected {"user_protected"} else {"active_session"}}));
            w["edges"].as_array_mut().unwrap().push(json!({"source":child_id,"target":target,"kind":"spawned_by","evidence":[evidence()]}));
        });
        c.proof.relationship_coverage.push(RelationshipCoverage{source_class:GraphNodeKind::Session,target_class:GraphNodeKind::Session,kind:EdgeKind::SpawnedBy,completeness:Completeness::Complete,evidence:vec![evidence()]});
        if cascading {
            c.planning.capabilities[0].effect_scope=NativeEffectScope::TargetAndAttemptedDescendants;
            c.planning.effects.as_mut().unwrap().capability=c.planning.capabilities[0].clone();
        }
        prop_assert_eq!(evaluate_integrity(&c.graph,&c.proof).verification_status,VerificationStatus::Healthy);
        let plan=c.plan(LifecycleIntent::Archive);
        prop_assert_eq!(plan.content().status,PlanStatus::Blocked);
        prop_assert!(plan.content().blockers.contains(&PlanBlocker::EffectBoundaryInvalid));
        prop_assert!(plan.content().operations.is_empty());
    }
}

#[test]
fn shared_unknown_global_and_referenced_state_is_retained_without_exclusivity() {
    let mut c = case(1);
    let artifact = c
        .graph
        .nodes()
        .iter()
        .find(|n| n.id.class == GraphNodeKind::Artifact)
        .unwrap()
        .id
        .clone();
    let first = c.plan(LifecycleIntent::Archive);
    let mut other = serde_json::to_value(c.graph.node(&c.target).unwrap()).unwrap();
    other["id"]["local_id"] = json!("session:other");
    other["observations"][0]["value"]["id"] = json!("session:other");
    other["observations"][0]["value"]["provider_session_id"] = json!("native-other");
    let other_id: ArtifactId = serde_json::from_value(other["id"].clone()).unwrap();
    let target = c.target.clone();
    c.change_graph(|w| {
        w["nodes"].as_array_mut().unwrap().push(other);
        w["roots"].as_array_mut().unwrap().push(json!({"node":other_id,"kind":"active_session"}));
        for source in [&target,&other_id] {
            w["edges"].as_array_mut().unwrap().push(json!({"source":source,"target":artifact,"kind":"references","evidence":[evidence()]}));
        }
    });
    c.proof.relationship_coverage.push(RelationshipCoverage {
        source_class: GraphNodeKind::Session,
        target_class: GraphNodeKind::Artifact,
        kind: EdgeKind::References,
        completeness: Completeness::Complete,
        evidence: vec![evidence()],
    });
    assert_eq!(
        c.graph
            .ownership_assessment(&artifact)
            .unwrap()
            .classification,
        OwnershipClassification::Shared
    );
    assert_eq!(c.validate(&first), PlanValidation::Stale);
    let current = c.plan(LifecycleIntent::Archive);
    assert_eq!(current.content().status, PlanStatus::Ready);
    let shared = current
        .content()
        .artifacts
        .iter()
        .find(|a| a.id == artifact)
        .unwrap();
    assert_eq!(shared.role, PlannedArtifactRole::Retained);
    assert!(
        current
            .content()
            .artifacts
            .iter()
            .any(
                |a| a.ownership.classification == OwnershipClassification::Global
                    && a.role == PlannedArtifactRole::Retained
            )
    );
    c.planning.capabilities[0].effect_scope = NativeEffectScope::TargetAndAttemptedDescendants;
    c.planning.effects.as_mut().unwrap().capability = c.planning.capabilities[0].clone();
    c.planning.effects.as_mut().unwrap().affected.push(artifact);
    let blocked = c.plan(LifecycleIntent::Archive);
    assert_eq!(blocked.content().status, PlanStatus::Blocked);
    assert!(
        blocked
            .content()
            .blockers
            .contains(&PlanBlocker::OwnershipNotExclusive)
    );
    assert!(
        blocked
            .content()
            .space
            .logical
            .unknown_reasons
            .contains(&EstimateUnknownReason::ProtectedOwnership)
    );
}

#[test]
fn forged_healthy_report_does_not_override_recomputed_integrity() {
    let c = case(0);
    let mut report = evaluate_integrity(&c.graph, &IntegrityContext::default());
    report.verification_status = VerificationStatus::Healthy;
    report.blockers.clear();
    let plan = plan_lifecycle(
        &c.graph,
        &report,
        &c.planning,
        LifecycleRequest {
            target: c.target.clone(),
            intent: LifecycleIntent::Archive,
        },
    );
    assert_eq!(plan.content().status, PlanStatus::Blocked);
    assert!(
        plan.content()
            .blockers
            .contains(&PlanBlocker::IntegrityMismatch)
    );
}

#[test]
fn native_id_alias_and_filtered_scope_cannot_claim_readiness() {
    for mode in 0..3 {
        let mut c = case(0);
        if mode == 0 {
            let mut alias = serde_json::to_value(c.graph.node(&c.target).unwrap()).unwrap();
            alias["id"]["local_id"] = json!("alias");
            alias["observations"][0]["value"]["id"] = json!("alias");
            c.change_graph(|w| w["nodes"].as_array_mut().unwrap().push(alias));
        } else if mode == 1 {
            c.planning.scope.include_archived = false;
        } else {
            c.planning.scope.project_path = Some("/synthetic/project".into());
        }
        assert_ne!(
            c.plan(LifecycleIntent::Archive).content().status,
            PlanStatus::Ready
        );
    }
}

#[test]
fn logical_measurement_never_manufactures_physical_reclaim() {
    let mut c = case(2);
    let artifacts: Vec<_> = c
        .graph
        .nodes()
        .iter()
        .filter(|n| n.id.class == GraphNodeKind::Artifact)
        .map(|n| n.id.clone())
        .collect();
    c.planning.capabilities[0].effect_scope = NativeEffectScope::TargetAndAttemptedDescendants;
    c.planning.effects.as_mut().unwrap().capability = c.planning.capabilities[0].clone();
    c.planning
        .effects
        .as_mut()
        .unwrap()
        .affected
        .extend(artifacts);
    let plan = c.plan(LifecycleIntent::Archive);
    assert_eq!(plan.content().status, PlanStatus::Blocked);
    assert_eq!(plan.content().space.physical.bytes, None);
    assert_eq!(plan.content().space.logical.total_bytes, None);
    assert!(
        plan.content()
            .space
            .logical
            .unknown_reasons
            .contains(&EstimateUnknownReason::ProtectedOwnership)
    );
}

#[test]
fn blocked_explanation_and_diagnostic_wording_do_not_change_identity() {
    let mut c = case(0);
    c.change_graph(|w| {
        w["context"]["diagnostics"] =
            json!([{"code":"snapshot_stale","message":"old words","path":null}])
    });
    let original = c.plan(LifecycleIntent::Archive);
    c.change_graph(|w| w["context"]["diagnostics"][0]["message"] = json!("different words"));
    let mut report = evaluate_integrity(&c.graph, &c.proof);
    for blocker in &mut report.blockers {
        blocker.explanation = "different explanation".into();
    }
    let changed = plan_lifecycle(
        &c.graph,
        &report,
        &c.planning,
        original.content().request.clone(),
    );
    assert_eq!(original.digest(), changed.digest());
    assert!(changed.content().operations.is_empty());
}

#[test]
fn affected_unknown_and_global_state_cannot_be_native_operations() {
    for global in [false, true] {
        let mut c = case(1);
        c.planning.capabilities[0].effect_scope = NativeEffectScope::TargetAndAttemptedDescendants;
        c.planning.effects.as_mut().unwrap().capability = c.planning.capabilities[0].clone();
        if global {
            let id = c
                .graph
                .nodes()
                .iter()
                .find(|n| n.id.class == GraphNodeKind::ProviderState)
                .unwrap()
                .id
                .clone();
            c.planning.effects.as_mut().unwrap().affected.push(id);
        } else {
            c.change_graph(|w| {
                for node in w["nodes"].as_array_mut().unwrap() {
                    if node["id"]["class"] == "artifact" {
                        node["observations"][0]["value"]["kind"] = json!("unknown");
                    }
                }
            });
            let id = c
                .graph
                .nodes()
                .iter()
                .find(|n| n.id.class == GraphNodeKind::Artifact)
                .unwrap()
                .id
                .clone();
            c.planning.effects.as_mut().unwrap().affected.push(id);
        }
        let plan = c.plan(LifecycleIntent::Archive);
        assert_ne!(plan.content().status, PlanStatus::Ready);
        assert!(plan.content().operations.is_empty());
        assert_eq!(plan.content().space.physical.bytes, None);
    }
}

#[test]
fn unknown_wire_fields_and_rehashed_inconsistent_roles_are_rejected() {
    let c = case(1);
    for path in ["top", "nested", "role"] {
        let mut wire = serde_json::to_value(c.plan(LifecycleIntent::Archive)).unwrap();
        match path {
            "top" => wire["unknown"] = json!(true),
            "nested" => wire["content"]["provider"]["unknown"] = json!(true),
            _ => {
                for a in wire["content"]["artifacts"].as_array_mut().unwrap() {
                    if a["role"] == "target" {
                        a["role"] = json!("retained");
                    }
                }
                rehash_wire(&mut wire);
            }
        }
        assert!(serde_json::from_value::<LifecyclePlan>(wire).is_err());
    }
}

#[test]
fn duplicate_json_keys_are_rejected_before_normalization() {
    let c = case(0);
    let text = serde_json::to_string(&c.plan(LifecycleIntent::Archive)).unwrap();
    for repeated in [
        text.replacen(
            "\"status\":\"ready\"",
            "\"status\":\"blocked\",\"status\":\"ready\"",
            1,
        ),
        text.replacen(
            "\"status\":\"ready\"",
            "\"\\u0073tatus\":\"blocked\",\"status\":\"ready\"",
            1,
        ),
        text.replacen(
            "\"schema_version\":1",
            "\"schema_version\":0,\"schema_version\":1",
            1,
        ),
        text.replacen(
            "\"native_id\":\"native-target\"",
            "\"native_id\":\"other\",\"native_id\":\"native-target\"",
            1,
        ),
    ] {
        assert_ne!(repeated, text);
        assert!(serde_json::from_str::<LifecyclePlan>(&repeated).is_err());
        assert!(serde_json::from_slice::<LifecyclePlan>(repeated.as_bytes()).is_err());
    }
}

#[test]
fn unknown_native_lifecycle_state_is_blocked_even_with_healthy_integrity() {
    let mut c = case(0);
    c.change_graph(|w| target_node(w)["observations"][0]["value"]["state"] = json!("unknown"));
    assert_eq!(
        evaluate_integrity(&c.graph, &c.proof).verification_status,
        VerificationStatus::Healthy
    );
    let plan = c.plan(LifecycleIntent::Archive);
    assert_eq!(plan.content().status, PlanStatus::Blocked);
    assert!(
        plan.content()
            .blockers
            .contains(&PlanBlocker::NativeIdentityUnverified)
    );
    assert!(plan.content().operations.is_empty());
}

#[cfg(unix)]
#[test]
fn native_path_bytes_survive_plan_roundtrip_and_fingerprint_without_collisions() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let mut c = case(0);
    let original = c.plan(LifecycleIntent::Archive);
    c.planning.scope.project_path = Some(OsString::from_vec(vec![b'/', b'x', 255]).into());
    let changed = c.plan(LifecycleIntent::Archive);
    assert_ne!(original.state_fingerprint(), changed.state_fingerprint());
    let wire = serde_json::to_value(&changed).unwrap();
    assert_eq!(
        wire["content"]["observation_scope"]["project_path"]["hex"],
        "2f78ff"
    );
    let round: LifecyclePlan = serde_json::from_value(wire).unwrap();
    assert_eq!(round.digest(), changed.digest());
    assert_eq!(
        round.content().observation_scope.project_path,
        changed.content().observation_scope.project_path
    );
}

fn rehash_wire(wire: &mut Value) {
    fn canonical(v: &mut Value) {
        match v {
            Value::Array(a) => {
                a.iter_mut().for_each(canonical);
                a.sort_by_key(|x| serde_json::to_vec(x).unwrap());
                a.dedup();
            }
            Value::Object(o) => o.values_mut().for_each(canonical),
            _ => (),
        }
    }
    let mut content = wire["content"].clone();
    canonical(&mut content);
    let mut hasher = Sha256::new();
    hasher.update(b"agentmender/plan/v1\0");
    hasher.update(serde_json::to_vec(&content).unwrap());
    wire["digest"] = json!(format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ));
}

#[test]
fn rehashed_forged_content_is_not_valid_against_fresh_inputs() {
    let c = case(0);
    let plan = c.plan(LifecycleIntent::Archive);
    let mut wire = serde_json::to_value(plan).unwrap();
    wire["content"]["state_fingerprint"] = json!(format!("sha256:{}", "b".repeat(64)));
    rehash_wire(&mut wire);
    let forged: LifecyclePlan = serde_json::from_value(wire).unwrap();
    assert_eq!(c.validate(&forged), PlanValidation::Stale);
}
