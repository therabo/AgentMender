use agentmender_domain::*;
use proptest::{prelude::*, test_runner::RngSeed};

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

fn initial(extra: usize, seed: [u8; 32]) -> (Vec<JournalRecord>, TransactionBinding) {
    let plan = case(extra).plan(LifecycleIntent::Archive);
    let binding = TransactionBinding::from_plan(&plan).unwrap();
    let id = TransactionId::from_bytes(seed);
    let record = JournalRecord::new(
        id,
        0,
        binding.digest(),
        None,
        TransactionEvent::Created {
            binding: binding.clone(),
        },
    );
    (vec![record], binding)
}
fn push(records: &mut Vec<JournalRecord>, event: TransactionEvent) {
    let snapshot = TransactionSnapshot::replay(records).unwrap();
    records.push(snapshot.next_record(event).unwrap());
}
fn prepare(records: &mut Vec<JournalRecord>, binding: &TransactionBinding) {
    let id = records[0].transaction_id().clone();
    push(
        records,
        TransactionEvent::RecoveryPrepared {
            manifest: RecoveryManifest::new(id, binding),
        },
    );
    push(records, TransactionEvent::Prepared);
}
fn intent(binding: &TransactionBinding) -> TransactionEvent {
    TransactionEvent::Intent {
        plan: binding.plan_digest().clone(),
        state: binding.state_fingerprint().clone(),
        operation_index: 0,
    }
}
proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d365f74786e7331), .. ProptestConfig::default() })]
    #[test]
    fn replay_preserves_order_and_unknown(extra in 0usize..8, seed in any::<[u8;32]>(), choice in 0u8..3) {
        let (mut records, binding) = initial(extra, seed);
        let first = TransactionSnapshot::replay(&records).unwrap();
        prop_assert!(first.next_record(intent(&binding)).is_err());
        let premature = TransactionEvent::Observed { observation: Observation::Applied };
        prop_assert!(first.next_record(premature).is_err());
        prepare(&mut records, &binding);
        push(&mut records, intent(&binding));
        prop_assert_eq!(TransactionSnapshot::replay(&records).unwrap().state(), TransactionState::OutcomeUnknown);
        let observation = [Observation::Applied, Observation::NoEffect, Observation::Unknown][choice as usize];
        push(&mut records, TransactionEvent::Observed { observation });
        let state = TransactionSnapshot::replay(&records).unwrap().state();
        prop_assert_eq!(state, [TransactionState::EffectObserved, TransactionState::Failed, TransactionState::RecoveryRequired][choice as usize]);
        prop_assert_eq!(TransactionSnapshot::replay(&records).unwrap(), TransactionSnapshot::replay(&records).unwrap());
        let mut duplicate = records.clone(); duplicate.push(records.last().unwrap().clone());
        prop_assert!(TransactionSnapshot::replay(&duplicate).is_err());
        let mut reordered = records.clone(); reordered.swap(1,2);
        prop_assert!(TransactionSnapshot::replay(&reordered).is_err());
        let mut wire = serde_json::to_value(&records).unwrap();
        wire[1]["sequence"] = serde_json::json!(99);
        let corrupted: Vec<JournalRecord> = serde_json::from_value(wire).unwrap();
        prop_assert!(TransactionSnapshot::replay(&corrupted).is_err());
    }
    #[test]
    fn reordered_plan_sets_have_same_binding(extra in 0usize..8) {
        let plan = case(extra).plan(LifecycleIntent::Archive);
        let mut wire = serde_json::to_value(&plan).unwrap();
        for path in ["preconditions", "postconditions"] {
            wire["content"][path].as_array_mut().unwrap().reverse();
        }
        for path in ["affected", "rule_ids"] {
            wire["content"]["effect_contract"][path].as_array_mut().unwrap().reverse();
        }
        wire["content"]["operations"][0]["affected"].as_array_mut().unwrap().reverse();
        let reordered: LifecyclePlan = serde_json::from_value(wire).unwrap();
        prop_assert_eq!(plan.digest(), reordered.digest());
        prop_assert_eq!(TransactionBinding::from_plan(&plan).unwrap(), TransactionBinding::from_plan(&reordered).unwrap());
    }
    #[test]
    fn success_requires_independent_verification(extra in 0usize..8, seed in any::<[u8;32]>(), passed in any::<bool>()) {
        let (mut records, binding) = initial(extra, seed);
        prepare(&mut records, &binding); push(&mut records, intent(&binding));
        push(&mut records, TransactionEvent::Observed { observation: Observation::Applied });
        push(&mut records, TransactionEvent::VerificationStarted);
        push(&mut records, TransactionEvent::Verified { verification: if passed { Verification::Passed } else { Verification::Unknown } });
        let snapshot = TransactionSnapshot::replay(&records).unwrap();
        prop_assert_eq!(snapshot.state(), if passed { TransactionState::Succeeded } else { TransactionState::RecoveryRequired });
        prop_assert!(snapshot.next_record(TransactionEvent::VerificationStarted).is_err());
        prop_assert_eq!(snapshot.binding(), &binding);
    }
}

#[test]
fn compensation_requires_explicit_contract_and_its_own_verified_outcome() {
    let (mut records, binding) = initial(0, [7; 32]);
    prepare(&mut records, &binding);
    push(&mut records, intent(&binding));
    push(
        &mut records,
        TransactionEvent::Observed {
            observation: Observation::Applied,
        },
    );
    let compensation = TransactionEvent::CompensationIntent {
        plan: binding.plan_digest().clone(),
        state: binding.state_fingerprint().clone(),
        operation_index: 0,
    };
    assert!(
        TransactionSnapshot::replay(&records)
            .unwrap()
            .next_record(compensation.clone())
            .is_err()
    );
    let (mut records, binding) = initial(0, [8; 32]);
    let manifest = RecoveryManifest::new(records[0].transaction_id().clone(), &binding)
        .with_compensation(CompensationContract::ReviewedSemantic {
            operation: NativeLifecycleOperation::UnarchiveSession,
            evidence: "a".repeat(64).parse().unwrap(),
        });
    push(
        &mut records,
        TransactionEvent::RecoveryPrepared { manifest },
    );
    push(&mut records, TransactionEvent::Prepared);
    push(&mut records, intent(&binding));
    push(
        &mut records,
        TransactionEvent::Observed {
            observation: Observation::Applied,
        },
    );
    push(&mut records, compensation);
    assert_eq!(
        TransactionSnapshot::replay(&records).unwrap().state(),
        TransactionState::CompensationOutcomeUnknown
    );
    push(
        &mut records,
        TransactionEvent::CompensationObserved {
            observation: Observation::Applied,
        },
    );
    push(
        &mut records,
        TransactionEvent::CompensationVerificationStarted,
    );
    push(
        &mut records,
        TransactionEvent::CompensationVerified {
            verification: Verification::Passed,
        },
    );
    let snapshot = TransactionSnapshot::replay(&records).unwrap();
    assert_eq!(snapshot.state(), TransactionState::Compensated);
    assert!(snapshot.next_record(TransactionEvent::Prepared).is_err());
}

#[test]
fn malformed_identity_and_binding_fail_closed() {
    for value in ["", "A".repeat(64).as_str(), "0".repeat(63).as_str()] {
        assert!(value.parse::<TransactionId>().is_err());
    }
    let (records, _) = initial(0, [1; 32]);
    let mut value = serde_json::to_value(&records[0]).unwrap();
    value["event"]["binding"]["operation"] = serde_json::json!("delete_session");
    let record: JournalRecord = serde_json::from_value(value).unwrap();
    assert!(TransactionSnapshot::replay(&[record]).is_err());
}

fn all_events(binding: &TransactionBinding, id: &TransactionId) -> Vec<TransactionEvent> {
    let mut events = vec![
        TransactionEvent::Created {
            binding: binding.clone(),
        },
        TransactionEvent::RecoveryPrepared {
            manifest: RecoveryManifest::new(id.clone(), binding),
        },
        TransactionEvent::Prepared,
        intent(binding),
        TransactionEvent::VerificationStarted,
        TransactionEvent::CompensationIntent {
            plan: binding.plan_digest().clone(),
            state: binding.state_fingerprint().clone(),
            operation_index: 0,
        },
        TransactionEvent::CompensationVerificationStarted,
    ];
    for observation in [
        Observation::Applied,
        Observation::NoEffect,
        Observation::Unknown,
    ] {
        events.push(TransactionEvent::Observed { observation });
        events.push(TransactionEvent::CompensationObserved { observation });
    }
    for verification in [
        Verification::Passed,
        Verification::Failed,
        Verification::Unknown,
    ] {
        events.push(TransactionEvent::Verified { verification });
        events.push(TransactionEvent::CompensationVerified { verification });
    }
    events
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d365f74786e7331), .. ProptestConfig::default() })]
    #[test]
    fn every_success_history_cut_has_only_recorded_knowledge(extra in 0usize..8, seed in any::<[u8;32]>()) {
        let (mut records, binding) = initial(extra, seed);
        prepare(&mut records, &binding);
        push(&mut records, intent(&binding));
        push(&mut records, TransactionEvent::Observed { observation: Observation::Applied });
        push(&mut records, TransactionEvent::VerificationStarted);
        push(&mut records, TransactionEvent::Verified { verification: Verification::Passed });
        let expected = [TransactionState::Created, TransactionState::RecoveryPrepared, TransactionState::Prepared, TransactionState::OutcomeUnknown, TransactionState::EffectObserved, TransactionState::Verifying, TransactionState::Succeeded];
        for (i, state) in expected.into_iter().enumerate() {
            let snapshot = TransactionSnapshot::replay(&records[..=i]).unwrap();
            prop_assert_eq!(snapshot.state(), state);
            prop_assert_eq!(snapshot.binding(), &binding);
        }
    }
    #[test]
    fn early_events_cannot_skip_recovery_preparation_or_intent(extra in 0usize..8, seed in any::<[u8;32]>()) {
        let (mut records, binding) = initial(extra, seed);
        let id = records[0].transaction_id().clone();
        let created = TransactionSnapshot::replay(&records).unwrap();
        for event in all_events(&binding, &id) {
            if !matches!(event, TransactionEvent::RecoveryPrepared { .. }) {
                prop_assert!(created.next_record(event).is_err());
            }
        }
        push(&mut records, TransactionEvent::RecoveryPrepared { manifest: RecoveryManifest::new(id.clone(), &binding) });
        let recovered = TransactionSnapshot::replay(&records).unwrap();
        for event in all_events(&binding, &id) {
            if !matches!(event, TransactionEvent::Prepared) { prop_assert!(recovered.next_record(event).is_err()); }
        }
        push(&mut records, TransactionEvent::Prepared);
        let prepared = TransactionSnapshot::replay(&records).unwrap();
        for event in all_events(&binding, &id) {
            if !matches!(event, TransactionEvent::Intent { .. }) { prop_assert!(prepared.next_record(event).is_err()); }
        }
    }
    #[test]
    fn every_terminal_state_rejects_every_event(extra in 0usize..8, seed in any::<[u8;32]>(), terminal in 0u8..5) {
        let (mut records, binding) = initial(extra, seed);
        if terminal == 4 {
            let manifest = RecoveryManifest::new(records[0].transaction_id().clone(), &binding).with_compensation(CompensationContract::ReviewedSemantic { operation: NativeLifecycleOperation::UnarchiveSession, evidence: "a".repeat(64).parse().unwrap() });
            push(&mut records, TransactionEvent::RecoveryPrepared { manifest }); push(&mut records, TransactionEvent::Prepared);
        } else { prepare(&mut records, &binding); }
        push(&mut records, intent(&binding));
        let observation = if terminal == 0 { Observation::NoEffect } else if terminal == 1 { Observation::Unknown } else { Observation::Applied };
        push(&mut records, TransactionEvent::Observed { observation });
        if terminal == 2 || terminal == 3 {
            push(&mut records, TransactionEvent::VerificationStarted);
            push(&mut records, TransactionEvent::Verified { verification: if terminal == 2 { Verification::Passed } else { Verification::Failed } });
        }
        if terminal == 4 {
            push(&mut records, TransactionEvent::CompensationIntent { plan: binding.plan_digest().clone(), state: binding.state_fingerprint().clone(), operation_index: 0 });
            push(&mut records, TransactionEvent::CompensationObserved { observation: Observation::Applied });
            push(&mut records, TransactionEvent::CompensationVerificationStarted);
            push(&mut records, TransactionEvent::CompensationVerified { verification: Verification::Passed });
        }
        let snapshot = TransactionSnapshot::replay(&records).unwrap();
        prop_assert_eq!(snapshot.state(), [TransactionState::Failed, TransactionState::RecoveryRequired, TransactionState::Succeeded, TransactionState::RecoveryRequired, TransactionState::Compensated][terminal as usize]);
        for event in all_events(&binding, snapshot.id()) { prop_assert!(snapshot.next_record(event).is_err()); }
    }
    #[test]
    fn regenerated_bad_envelopes_fail_closed(extra in 0usize..8, seed in any::<[u8;32]>()) {
        let (records, binding) = initial(extra, seed);
        let snapshot = TransactionSnapshot::replay(&records).unwrap();
        let manifest = RecoveryManifest::new(snapshot.id().clone(), &binding);
        let event = TransactionEvent::RecoveryPrepared { manifest };
        let other_id = TransactionId::from_bytes(seed.map(|b| b ^ 0xff));
        let bad_binding: BindingDigest = "a".repeat(64).parse().unwrap();
        let bad_previous: JournalDigest = "b".repeat(64).parse().unwrap();
        let malformed = [
            JournalRecord::new(other_id, 1, binding.digest(), Some(snapshot.last_digest().clone()), event.clone()),
            JournalRecord::new(snapshot.id().clone(), 2, binding.digest(), Some(snapshot.last_digest().clone()), event.clone()),
            JournalRecord::new(snapshot.id().clone(), 1, bad_binding, Some(snapshot.last_digest().clone()), event.clone()),
            JournalRecord::new(snapshot.id().clone(), 1, binding.digest(), Some(bad_previous), event.clone()),
            JournalRecord::new(snapshot.id().clone(), 1, binding.digest(), None, event),
        ];
        for record in malformed { let mut history = records.clone(); history.push(record); prop_assert!(TransactionSnapshot::replay(&history).is_err()); }
        let valid = snapshot.next_record(TransactionEvent::RecoveryPrepared { manifest: RecoveryManifest::new(snapshot.id().clone(), &binding) }).unwrap();
        for field in ["schema_version", "digest"] {
            let mut value = serde_json::to_value(&valid).unwrap();
            value[field] = if field == "schema_version" { serde_json::json!(2) } else { serde_json::json!("c".repeat(64)) };
            let bad: JournalRecord = serde_json::from_value(value).unwrap();
            let mut history = records.clone(); history.push(bad); prop_assert!(TransactionSnapshot::replay(&history).is_err());
        }
    }
    #[test]
    fn attempts_have_distinct_ids_but_identical_immutable_binding(extra in 0usize..8, seed in any::<[u8;32]>()) {
        let (first, binding) = initial(extra, seed);
        let (second, other_binding) = initial(extra, seed.map(|b| b ^ 0xff));
        prop_assert_ne!(first[0].transaction_id(), second[0].transaction_id());
        prop_assert_eq!(&binding, &other_binding);
        let first_snapshot = TransactionSnapshot::replay(&first).unwrap();
        let second_snapshot = TransactionSnapshot::replay(&second).unwrap();
        prop_assert_eq!(first_snapshot.binding(), &binding);
        prop_assert_eq!(second_snapshot.binding(), &binding);
    }
    #[test]
    fn compensation_intent_crash_remains_unknown(extra in 0usize..8, seed in any::<[u8;32]>()) {
        let (mut records, binding) = initial(extra, seed);
        let manifest = RecoveryManifest::new(records[0].transaction_id().clone(), &binding).with_compensation(CompensationContract::ReviewedSemantic { operation: NativeLifecycleOperation::UnarchiveSession, evidence: "a".repeat(64).parse().unwrap() });
        push(&mut records, TransactionEvent::RecoveryPrepared { manifest }); push(&mut records, TransactionEvent::Prepared); push(&mut records, intent(&binding));
        push(&mut records, TransactionEvent::Observed { observation: Observation::Applied });
        push(&mut records, TransactionEvent::CompensationIntent { plan: binding.plan_digest().clone(), state: binding.state_fingerprint().clone(), operation_index: 0 });
        let snapshot = TransactionSnapshot::replay(&records).unwrap();
        prop_assert_eq!(snapshot.state(), TransactionState::CompensationOutcomeUnknown);
        prop_assert!(snapshot.next_record(TransactionEvent::CompensationVerificationStarted).is_err());
    }
}

#[test]
fn journal_and_manifest_do_not_persist_synthetic_native_data() {
    let (mut records, binding) = initial(1, [42; 32]);
    let manifest = RecoveryManifest::new(records[0].transaction_id().clone(), &binding);
    push(
        &mut records,
        TransactionEvent::RecoveryPrepared {
            manifest: manifest.clone(),
        },
    );
    let json = serde_json::to_string(&(records, manifest)).unwrap();
    for canary in [
        "native-target",
        "session:target",
        "/synthetic-planning",
        "synthetic-adapter-v1",
        "synthetic authoritative identity",
        "synthetic full boundary",
    ] {
        assert!(!json.contains(canary), "leaked canary {canary}");
    }
}

#[test]
fn nested_compensation_contract_rejects_unknown_and_duplicate_fields() {
    let (records, binding) = initial(0, [43; 32]);
    let manifest = RecoveryManifest::new(records[0].transaction_id().clone(), &binding)
        .with_compensation(CompensationContract::ReviewedSemantic {
            operation: NativeLifecycleOperation::UnarchiveSession,
            evidence: "a".repeat(64).parse().unwrap(),
        });
    let mut value = serde_json::to_value(&manifest).unwrap();
    value["compensation"]["reviewed_semantic"]["unrecognized"] = serde_json::json!(true);
    assert!(serde_json::from_value::<RecoveryManifest>(value).is_err());
    let text = serde_json::to_string(&manifest).unwrap();
    let duplicate = text.replace(
        "\"operation\":\"unarchive_session\"",
        "\"operation\":\"unarchive_session\",\"operation\":\"unarchive_session\"",
    );
    assert_ne!(text, duplicate);
    assert!(serde_json::from_str::<RecoveryManifest>(&duplicate).is_err());
}
