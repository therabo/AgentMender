use agentmender_domain::*;
use proptest::{prelude::*, test_runner::RngSeed};

mod common;
use common::*;

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
            plan: None,
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
