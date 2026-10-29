use agentmender_domain::*;
use proptest::{prelude::*, test_runner::RngSeed};
mod common;
use common::*;

fn approval_fixture(seed: [u8; 32], issued: u64) -> (Vec<JournalRecord>, ApprovalGrant) {
    let plan = case(0).plan(LifecycleIntent::Archive);
    let binding = TransactionBinding::from_plan(&plan).unwrap();
    let id = TransactionId::from_bytes(seed);
    let mut records = vec![JournalRecord::new_v2(
        id.clone(),
        0,
        binding.digest(),
        None,
        TransactionEvent::Created {
            binding: binding.clone(),
            plan: Some(plan),
        },
    )];
    prepare(&mut records, &binding);
    let recovery_digest = TransactionSnapshot::replay(&records)
        .unwrap()
        .recovery()
        .unwrap()
        .digest();
    let grant = ApprovalGrant {
        id: ApprovalId::from_bytes(seed),
        nonce: ApprovalNonce::from_bytes(seed.map(|x| x ^ 1)),
        scope: ApprovalScope {
            transaction_id: id,
            binding,
            recovery_digest,
            operation_index: 0,
            direction: ApprovalDirection::ForwardOnly,
        },
        principal: ApprovalPrincipal::LocalInteractiveUser,
        issued_at: issued,
        expires_at: issued + 300,
    };
    (records, grant)
}
fn authentication() -> AuthorizationAuthentication {
    AuthorizationAuthentication {
        key_id: ApprovalKeyId::from_bytes([1; 32]),
        tag: AuthenticationTag::from_bytes([2; 32]),
    }
}
fn authorize(action: ApprovalAction) -> TransactionEvent {
    TransactionEvent::Authorization {
        action,
        authentication: authentication(),
    }
}
fn grant_record(records: &mut Vec<JournalRecord>, grant: &ApprovalGrant) {
    push(
        records,
        authorize(ApprovalAction::Granted {
            grant: grant.clone(),
        }),
    );
}
fn consume(grant: &ApprovalGrant, at: u64) -> ApprovalAction {
    ApprovalAction::Consumed {
        approval_id: grant.id.clone(),
        at,
        plan: grant.scope.binding.plan_digest().clone(),
        state: grant.scope.binding.state_fingerprint().clone(),
        operation_index: 0,
    }
}

macro_rules! approval_property {
    ($name:ident, $n:expr) => {
        proptest! {
            #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d375f6170707276), .. ProptestConfig::default() })]
            #[test]
            fn $name(seed in any::<[u8;32]>(), issued in 1u64..1_000_000) {
                let (mut records, mut grant) = approval_fixture(seed, issued);
                let snapshot = TransactionSnapshot::replay(&records).unwrap();
                let key = ApprovalKeyId::from_bytes(seed);
                let action = ApprovalAction::Granted { grant: grant.clone() };
                let signed = authorization_signing_bytes(&key, snapshot.id(), 3, &snapshot.binding().digest(), &Some(snapshot.last_digest().clone()), &action);
                match $n {
                    0 => { prop_assert!(grant.scope_and_time_valid(&snapshot, issued)); prop_assert!(!grant.scope_and_time_valid(&snapshot, issued - 1)); }
                    1 => { prop_assert!(grant.scope_and_time_valid(&snapshot, issued + 299)); prop_assert!(!grant.scope_and_time_valid(&snapshot, issued + 300)); }
                    2 => { grant.expires_at = issued; prop_assert!(!grant.scope_and_time_valid(&snapshot, issued)); }
                    3 => { grant.expires_at = issued + 301; prop_assert!(!grant.scope_and_time_valid(&snapshot, issued)); }
                    4 => { grant.scope.transaction_id = TransactionId::from_bytes(seed.map(|x| x ^ 1)); prop_assert!(!grant.scope_and_time_valid(&snapshot, issued)); }
                    5 => { grant.scope.recovery_digest = "a".repeat(64).parse().unwrap(); if grant.scope.recovery_digest == snapshot.recovery().unwrap().digest() { grant.scope.recovery_digest = "b".repeat(64).parse().unwrap(); } prop_assert!(!grant.scope_and_time_valid(&snapshot, issued)); }
                    6 => { grant.scope.operation_index = 1; prop_assert!(!grant.scope_and_time_valid(&snapshot, issued)); }
                    7 => { grant_record(&mut records, &grant); let s = TransactionSnapshot::replay(&records).unwrap(); prop_assert!(s.next_record(authorize(action)).is_err()); }
                    8 => { grant_record(&mut records, &grant); push(&mut records, authorize(ApprovalAction::Revoked { approval_id: grant.id.clone(), at: issued })); let s = TransactionSnapshot::replay(&records).unwrap(); prop_assert_eq!(s.approval().state, ApprovalState::Revoked); prop_assert!(s.next_record(authorize(consume(&grant, issued))).is_err()); }
                    9 => { grant_record(&mut records, &grant); push(&mut records, authorize(consume(&grant, issued))); let s = TransactionSnapshot::replay(&records).unwrap(); prop_assert_eq!(s.approval().state, ApprovalState::Consumed); prop_assert_eq!(s.state(), TransactionState::OutcomeUnknown); prop_assert!(s.next_record(authorize(consume(&grant, issued))).is_err()); }
                    10 => { grant_record(&mut records, &grant); let s = TransactionSnapshot::replay(&records).unwrap(); prop_assert!(s.next_record(authorize(consume(&grant, issued + 300))).is_err()); }
                    11 => { grant_record(&mut records, &grant); let s = TransactionSnapshot::replay(&records).unwrap(); prop_assert!(s.next_record(authorize(consume(&grant, issued - 1))).is_err()); }
                    12 => { let other = ApprovalKeyId::from_bytes(seed.map(|x| x ^ 1)); prop_assert_ne!(signed, authorization_signing_bytes(&other, snapshot.id(), 3, &snapshot.binding().digest(), &Some(snapshot.last_digest().clone()), &action)); }
                    13 => { prop_assert_ne!(signed, authorization_signing_bytes(&key, snapshot.id(), 4, &snapshot.binding().digest(), &Some(snapshot.last_digest().clone()), &action)); }
                    14 => { grant.nonce = ApprovalNonce::from_bytes(seed.map(|x| x ^ 2)); prop_assert_ne!(signed, authorization_signing_bytes(&key, snapshot.id(), 3, &snapshot.binding().digest(), &Some(snapshot.last_digest().clone()), &ApprovalAction::Granted { grant })); }
                    _ => { let c = case(0); let report = evaluate_integrity(&c.graph, &c.proof); prop_assert!(approval_eligible(&snapshot, &c.graph, &report, &c.planning, true)); prop_assert!(!approval_eligible(&snapshot, &c.graph, &report, &c.planning, false)); }
                }
            }
        }
    }
}
approval_property!(before_issuance_is_invalid, 0);
approval_property!(expiry_boundary_is_exclusive, 1);
approval_property!(zero_lifetime_is_invalid, 2);
approval_property!(excess_lifetime_is_invalid, 3);
approval_property!(wrong_transaction_is_invalid, 4);
approval_property!(wrong_recovery_is_invalid, 5);
approval_property!(wrong_operation_is_invalid, 6);
approval_property!(second_grant_is_rejected, 7);
approval_property!(revocation_is_terminal, 8);
approval_property!(consumption_is_atomic_and_terminal, 9);
approval_property!(expired_consumption_is_rejected, 10);
approval_property!(early_consumption_is_rejected, 11);
approval_property!(key_id_changes_signing_bytes, 12);
approval_property!(sequence_changes_signing_bytes, 13);
approval_property!(nonce_changes_signing_bytes, 14);
approval_property!(eligibility_requires_verified_recovery, 15);

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d375f6170707276), .. ProptestConfig::default() })]
    #[test]
    fn all_envelope_and_grant_fields_are_signed(seed in any::<[u8;32]>(), issued in 1u64..1_000_000) {
        let (records, grant) = approval_fixture(seed, issued);
        let s = TransactionSnapshot::replay(&records).unwrap();
        let key = ApprovalKeyId::from_bytes(seed);
        let action = ApprovalAction::Granted { grant: grant.clone() };
        let previous = Some(s.last_digest().clone());
        let sign = |k: &ApprovalKeyId, id: &TransactionId, seq, binding: &BindingDigest, prev: &Option<JournalDigest>, a: &ApprovalAction| authorization_signing_bytes(k, id, seq, binding, prev, a);
        let original = sign(&key, s.id(), 3, &s.binding().digest(), &previous, &action);
        let other_id = TransactionId::from_bytes(seed.map(|x| x ^ 255));
        prop_assert_ne!(&original, &sign(&key, &other_id, 3, &s.binding().digest(), &previous, &action));
        prop_assert_ne!(&original, &sign(&key, s.id(), 3, &s.binding().digest(), &None, &action));
        let other_binding = TransactionBinding::from_plan(&case(1).plan(LifecycleIntent::Archive)).unwrap();
        prop_assert_ne!(&original, &sign(&key, s.id(), 3, &other_binding.digest(), &previous, &action));
        let mut mutations = Vec::new();
        let mut g = grant.clone(); g.id = ApprovalId::from_bytes(seed.map(|x| x ^ 1)); mutations.push(g);
        let mut g = grant.clone(); g.nonce = ApprovalNonce::from_bytes(seed.map(|x| x ^ 2)); mutations.push(g);
        let mut g = grant.clone(); g.scope.transaction_id = other_id; mutations.push(g);
        let mut g = grant.clone(); g.scope.binding = other_binding; mutations.push(g);
        let mut g = grant.clone(); g.scope.recovery_digest = "a".repeat(64).parse().unwrap(); if g.scope.recovery_digest == grant.scope.recovery_digest { g.scope.recovery_digest = "b".repeat(64).parse().unwrap(); } mutations.push(g);
        let mut g = grant.clone(); g.scope.operation_index = 1; mutations.push(g);
        let mut g = grant.clone(); g.issued_at += 1; mutations.push(g);
        let mut g = grant.clone(); g.expires_at += 1; mutations.push(g);
        for g in mutations { prop_assert_ne!(&original, &sign(&key, s.id(), 3, &s.binding().digest(), &previous, &ApprovalAction::Granted { grant: g })); }
        prop_assert_ne!(&original, &sign(&key, s.id(), 3, &s.binding().digest(), &previous, &consume(&grant, issued)));
    }
    #[test]
    fn v2_rejects_mixed_versions_and_legacy_intent(seed in any::<[u8;32]>(), issued in 1u64..1_000_000) {
        let (records, grant) = approval_fixture(seed, issued);
        let s = TransactionSnapshot::replay(&records).unwrap();
        prop_assert!(s.next_record(intent(&grant.scope.binding)).is_err());
        let mixed = JournalRecord::new(s.id().clone(), s.sequence() + 1, s.binding().digest(), Some(s.last_digest().clone()), intent(&grant.scope.binding));
        prop_assert!(s.advanced(&mixed).is_err());
        if let TransactionEvent::Created { plan: Some(plan), .. } = records[0].event() { prop_assert_eq!(s.review_plan(), Some(plan)); } else { prop_assert!(false); }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d375f6170707276), .. ProptestConfig::default() })]
    #[test]
    fn eligibility_rejects_stale_or_already_issued_and_consumption_is_exact(seed in any::<[u8;32]>(), issued in 1u64..1_000_000) {
        let (mut records, grant) = approval_fixture(seed, issued);
        let s = TransactionSnapshot::replay(&records).unwrap();
        let changed = case(1);
        let changed_report = evaluate_integrity(&changed.graph, &changed.proof);
        prop_assert!(!approval_eligible(&s, &changed.graph, &changed_report, &changed.planning, true));
        let c = case(0);
        let report = evaluate_integrity(&c.graph, &c.proof);
        prop_assert!(!approval_valid(&s, &grant, issued));
        grant_record(&mut records, &grant);
        let granted = TransactionSnapshot::replay(&records).unwrap();
        prop_assert!(!approval_eligible(&granted, &c.graph, &report, &c.planning, true));
        prop_assert!(approval_valid(&granted, &grant, issued));
        prop_assert_eq!(granted.approval_status_at(issued - 1), ApprovalStatus::Invalid);
        prop_assert_eq!(granted.approval_status_at(issued), ApprovalStatus::Granted);
        prop_assert_eq!(granted.approval_status_at(issued + 300), ApprovalStatus::Expired);
        let mut forged = grant.clone();
        forged.nonce = ApprovalNonce::from_bytes(seed.map(|x| x ^ 2));
        prop_assert!(!approval_valid(&granted, &forged, issued));
        forged = grant.clone();
        forged.scope.binding = TransactionBinding::from_plan(&changed.plan(LifecycleIntent::Archive)).unwrap();
        prop_assert!(!forged.scope_and_time_valid(&granted, issued));
        let mut revoked_records = records.clone();
        push(&mut revoked_records, authorize(ApprovalAction::Revoked { approval_id: grant.id.clone(), at: issued }));
        let revoked = TransactionSnapshot::replay(&revoked_records).unwrap();
        prop_assert!(!approval_eligible(&revoked, &c.graph, &report, &c.planning, true));
        prop_assert!(!approval_valid(&revoked, &grant, issued));
        prop_assert_eq!(revoked.approval_status_at(issued), ApprovalStatus::Revoked);
        prop_assert!(revoked.next_record(authorize(ApprovalAction::Granted { grant: grant.clone() })).is_err(), "revoked regrant");
        push(&mut records, authorize(consume(&grant, issued)));
        let consumed = TransactionSnapshot::replay(&records).unwrap();
        prop_assert!(!approval_eligible(&consumed, &c.graph, &report, &c.planning, true));
        prop_assert!(!approval_valid(&consumed, &grant, issued));
        prop_assert_eq!(consumed.approval_status_at(issued), ApprovalStatus::Consumed);
        prop_assert!(consumed.next_record(authorize(ApprovalAction::Granted { grant })).is_err(), "consumed regrant");
    }
    #[test]
    fn v1_cannot_issue_and_v2_requires_original_plan(seed in any::<[u8;32]>(), issued in 1u64..1_000_000) {
        let (records, binding) = initial(0, seed);
        let mut legacy = records;
        prepare(&mut legacy, &binding);
        let s = TransactionSnapshot::replay(&legacy).unwrap();
        let c = case(0);
        let report = evaluate_integrity(&c.graph, &c.proof);
        prop_assert!(!approval_eligible(&s, &c.graph, &report, &c.planning, true));
        let (_, grant) = approval_fixture(seed, issued);
        prop_assert!(s.next_record(authorize(ApprovalAction::Granted { grant })).is_err(), "v1 authorization");
        let missing = JournalRecord::new_v2(s.id().clone(), 0, binding.digest(), None, TransactionEvent::Created { binding, plan: None });
        prop_assert!(TransactionSnapshot::replay(&[missing]).is_err());
    }
}

#[test]
fn approval_nested_unknown_and_duplicate_fields_are_rejected() {
    let (_, grant) = approval_fixture([9; 32], 100);
    let action = ApprovalAction::Granted { grant };
    let mut value = serde_json::to_value(&action).unwrap();
    value["grant"]["scope"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ApprovalAction>(value).is_err());
    let text = serde_json::to_string(&action).unwrap();
    let duplicate = text.replace(
        "\"operation_index\":0",
        "\"operation_index\":0,\"operation_index\":0",
    );
    assert_ne!(text, duplicate);
    assert!(serde_json::from_str::<ApprovalAction>(&duplicate).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d375f6170707276), .. ProptestConfig::default() })]
    #[test]
    fn each_binding_field_is_independently_scoped_and_signed(seed in any::<[u8;32]>(), issued in 1u64..1_000_000) {
        let (mut records, grant) = approval_fixture(seed, issued);
        grant_record(&mut records, &grant);
        let snapshot = TransactionSnapshot::replay(&records).unwrap();
        let key = ApprovalKeyId::from_bytes(seed);
        let previous = Some(snapshot.last_digest().clone());
        let sign = |g: ApprovalGrant| authorization_signing_bytes(&key, snapshot.id(), 4, &snapshot.binding().digest(), &previous, &ApprovalAction::Granted { grant: g });
        let original = sign(grant.clone());
        let serialized = serde_json::to_value(&grant).unwrap();
        for field in ["plan_digest", "state_fingerprint", "provider", "intent", "operation", "target", "native_identity", "profile", "capability_effect", "conditions", "operation_commitment"] {
            let mut changed = serialized.clone();
            let slot = &mut changed["scope"]["binding"][field];
            let replacement = match field {
                "provider" => if slot == "codex" { "claude".to_owned() } else { "codex".to_owned() },
                "intent" => if slot == "archive" { "unarchive".to_owned() } else { "archive".to_owned() },
                "operation" => if slot == "archive_session" { "unarchive_session".to_owned() } else { "archive_session".to_owned() },
                _ => { let prefix = if slot.as_str().unwrap().starts_with("sha256:") { "sha256:" } else { "" }; let a = format!("{prefix}{}", "a".repeat(64)); if slot == &serde_json::json!(a) { format!("{prefix}{}", "b".repeat(64)) } else { a } },
            };
            *slot = serde_json::json!(replacement);
            let mutated: ApprovalGrant = serde_json::from_value(changed).unwrap();
            prop_assert!(!mutated.scope_and_time_valid(&snapshot, issued), "{field}");
            prop_assert!(!approval_valid(&snapshot, &mutated, issued), "{field}");
            prop_assert_ne!(&original, &sign(mutated), "{}", field);
        }
        for (container, field) in [("scope", "direction"), ("", "principal")] {
            let mut changed = serialized.clone();
            if container.is_empty() { changed[field] = serde_json::json!("invalid"); }
            else { changed[container][field] = serde_json::json!("invalid"); }
            prop_assert!(serde_json::from_value::<ApprovalGrant>(changed).is_err());
        }
    }
}
