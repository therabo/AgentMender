//! Independent local lifecycle orchestration. Reconciliation never dispatches.
use crate::DiscoveryService;
use agentmender_domain::{
    ApprovalState, ArtifactGraph, Completeness, LifecycleIntent, LifecyclePlan,
    NativeLifecycleOperation, NodeObservation, Observation, PlanStatus, PlanValidation, RootKind,
    SessionState, TransactionId, TransactionState, Verification, VerificationStatus,
    evaluate_integrity,
};
use agentmender_persistence::{
    DurableEffectPort, EffectError, EffectObservation, NativeObservation, NativeOutcome,
    StoreError, TransactionStore,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("durable transaction error: {0}")]
    Store(#[from] StoreError),
    #[error("lifecycle preconditions unavailable or changed")]
    Preconditions,
    #[error("native lifecycle preflight failed: {0:?}")]
    Native(EffectError),
    #[error("native lifecycle outcome is unknown: {0:?}")]
    Ambiguous(EffectError),
}

fn is_unarchive_plan(plan: &LifecyclePlan) -> bool {
    let c = plan.content();
    c.status == PlanStatus::Ready
        && c.request.intent == LifecycleIntent::Unarchive
        && c.operations.len() == 1
        && c.operations[0].operation == NativeLifecycleOperation::UnarchiveSession
        && c.capability
            .as_ref()
            .is_some_and(|cap| cap.operation == NativeLifecycleOperation::UnarchiveSession)
}

/// The plan, approval, recovery, and fresh inputs are all rechecked before a
/// one-shot durable intent is handed to the provider. Ambiguity remains pending.
pub fn execute(
    store: &mut TransactionStore,
    id: &TransactionId,
    discovery: &DiscoveryService<'_>,
    port: &dyn DurableEffectPort,
) -> Result<TransactionState, LifecycleError> {
    let snapshot = store.inspect(id)?;
    let plan = snapshot
        .review_plan()
        .ok_or(LifecycleError::Preconditions)?
        .clone();
    if snapshot.schema_version() != 2
        || snapshot.state() != TransactionState::Prepared
        || snapshot.approval().state != ApprovalState::Granted
        || !is_unarchive_plan(&plan)
    {
        return Err(LifecycleError::Preconditions);
    }
    port.preflight(&plan).map_err(LifecycleError::Native)?;
    let pre = port.observe(&plan).map_err(LifecycleError::Native)?;
    if !native_matches(&plan, &pre, NativeObservation::Archived) {
        return Err(LifecycleError::Preconditions);
    }
    let c = plan.content();
    let inputs = discovery
        .approval_inputs(
            &|target| same_serialized(target, &c.request.target),
            c.request.intent,
            &c.observation_scope,
        )
        .map_err(|_| LifecycleError::Preconditions)?;
    if !same_serialized(&pre.graph, &inputs.graph)
        || !inputs.failures.is_empty()
        || !same_serialized(&inputs.plan, &plan)
        || plan.validate_against(&inputs.graph, &inputs.integrity, &inputs.context)
            != PlanValidation::Current
    {
        return Err(LifecycleError::Preconditions);
    }
    let receipt = store.record_intent(
        id,
        snapshot.sequence(),
        &plan,
        &inputs.graph,
        &inputs.integrity,
        &inputs.context,
    )?;
    let seq = receipt.sequence();
    let attempt = port.dispatch(receipt, &plan);
    match attempt.outcome {
        NativeOutcome::NeverDispatched(_) => {
            store.record_observation(id, seq, &attempt.observation, Observation::NoEffect)?
        }
        NativeOutcome::Unknown(reason) => return Err(LifecycleError::Ambiguous(reason)),
        NativeOutcome::Accepted => {
            store.record_observation(id, seq, &attempt.observation, Observation::Applied)?;
            verify(store, id, port, &plan, Some(&inputs.graph))?;
        }
    }
    Ok(store.inspect(id)?.state())
}

/// Reconcile consumed v2 history through observation only. Never reissue intent.
pub fn reconcile(
    store: &mut TransactionStore,
    id: &TransactionId,
    discovery: &DiscoveryService<'_>,
    port: &dyn DurableEffectPort,
) -> Result<TransactionState, LifecycleError> {
    let snapshot = store.inspect_consumed(id)?;
    let plan = snapshot
        .review_plan()
        .ok_or(LifecycleError::Preconditions)?
        .clone();
    if snapshot.schema_version() != 2
        || snapshot.approval().state != ApprovalState::Consumed
        || !is_unarchive_plan(&plan)
        || !matches!(
            snapshot.state(),
            TransactionState::OutcomeUnknown
                | TransactionState::EffectObserved
                | TransactionState::Verifying
        )
    {
        return Err(LifecycleError::Preconditions);
    }
    if snapshot.state() == TransactionState::OutcomeUnknown {
        if !port
            .observe(&plan)
            .is_ok_and(|obs| native_matches(&plan, &obs, NativeObservation::Active))
        {
            return Ok(TransactionState::OutcomeUnknown);
        }
        store.record_reconciliation(id, snapshot.sequence(), Observation::Applied)?;
    }
    let _ = discovery; // No retained baseline is available across restart.
    verify(store, id, port, &plan, None)?;
    Ok(store.inspect(id)?.state())
}

fn verify(
    store: &mut TransactionStore,
    id: &TransactionId,
    port: &dyn DurableEffectPort,
    plan: &LifecyclePlan,
    baseline: Option<&ArtifactGraph>,
) -> Result<(), LifecycleError> {
    let snapshot = store.inspect_consumed(id)?;
    if snapshot.state() == TransactionState::EffectObserved {
        store.start_verification(id, snapshot.sequence())?;
    }
    let result = port
        .observe(plan)
        .ok()
        .map_or(Verification::Unknown, |obs| {
            evaluate_post(plan, baseline, &obs)
        });
    let seq = store.inspect_consumed(id)?.sequence();
    store.record_verification(id, seq, result)?;
    Ok(())
}

fn same_serialized<T: serde::Serialize + ?Sized>(a: &T, b: &T) -> bool {
    matches!((serde_json::to_vec(a), serde_json::to_vec(b)), (Ok(a), Ok(b)) if a == b)
}

fn native_matches(
    plan: &LifecyclePlan,
    obs: &EffectObservation,
    expected: NativeObservation,
) -> bool {
    let c = plan.content();
    if obs.native != expected
        || obs.graph.context().provider.kind != c.request.target.provider
        || !same_serialized(&obs.graph.context().provider.fingerprint, &c.provider)
        || obs.graph.context().completeness != Completeness::Complete
        || !obs.graph.diagnostics().is_empty()
    {
        return false;
    }
    let mut scoped_target = c.request.target.clone();
    scoped_target.state_root = obs.graph.context().provider.state_root.clone();
    if !same_serialized(&scoped_target, &c.request.target) {
        return false;
    }
    let (wanted, required_root, forbidden_root) = match expected {
        NativeObservation::Active => (
            SessionState::Active,
            RootKind::ActiveSession,
            RootKind::ArchivedRetainedSession,
        ),
        NativeObservation::Archived => (
            SessionState::Archived,
            RootKind::ArchivedRetainedSession,
            RootKind::ActiveSession,
        ),
        NativeObservation::Unknown => return false,
    };
    if obs
        .graph
        .roots()
        .iter()
        .filter(|root| same_serialized(&root.node, &c.request.target) && root.kind == required_root)
        .count()
        != 1
        || obs.graph.roots().iter().any(|root| {
            same_serialized(&root.node, &c.request.target) && root.kind == forbidden_root
        })
    {
        return false;
    }
    let Some(id) = c.native_target_id.as_ref() else {
        return false;
    };
    let identity_count = obs.graph.nodes().iter().flat_map(|node| &node.observations)
        .filter(|observation| matches!(observation, NodeObservation::Session(session) if session.provider_session_id == id.as_str())).count();
    if identity_count != 1 {
        return false;
    }
    let mut nodes = obs
        .graph
        .nodes()
        .iter()
        .filter(|n| same_serialized(&n.id, &c.request.target));
    let Some(node) = nodes.next() else {
        return false;
    };
    nodes.next().is_none()
        && node.observations.len() == 1
        && matches!(&node.observations[0],
        NodeObservation::Session(session) if session.provider_session_id == id.as_str() && session.state == wanted)
}

/// Compare all logical graph facts except the one reviewed target transition.
fn evaluate_post(
    plan: &LifecyclePlan,
    baseline: Option<&ArtifactGraph>,
    obs: &EffectObservation,
) -> Verification {
    if !native_matches(plan, obs, NativeObservation::Active) {
        return Verification::Unknown;
    }
    let report = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
    if !same_serialized(&report, &obs.integrity)
        || report
            .proof_context
            .contract
            .as_ref()
            .is_none_or(|c| !c.coherent)
        || !report.graph_diagnostics.is_empty()
    {
        return Verification::Unknown;
    }
    match report.verification_status {
        VerificationStatus::Degraded | VerificationStatus::Failed => return Verification::Failed,
        VerificationStatus::Unknown => return Verification::Unknown,
        VerificationStatus::Healthy | VerificationStatus::HealthyWithExpectedResiduals => {}
    }
    if !report.findings.is_empty() || !report.blockers.is_empty() {
        return Verification::Failed;
    }
    let Some(pre) = baseline else {
        return Verification::Unknown;
    };
    let target = &plan.content().request.target;
    if !same_serialized(pre.context(), obs.graph.context())
        || !same_serialized(pre.edges(), obs.graph.edges())
        || !same_serialized(pre.rules(), obs.graph.rules())
        || !same_serialized(pre.diagnostics(), obs.graph.diagnostics())
    {
        return Verification::Failed;
    }
    let mut post_nodes = obs.graph.nodes().to_vec();
    for node in &mut post_nodes {
        if same_serialized(&node.id, target) {
            for observation in &mut node.observations {
                if let NodeObservation::Session(session) = observation {
                    session.state = SessionState::Archived;
                }
            }
        }
    }
    if !same_serialized(pre.nodes(), post_nodes.as_slice()) {
        return Verification::Failed;
    }
    let mut post_roots = obs.graph.roots().to_vec();
    for root in &mut post_roots {
        if same_serialized(&root.node, target) && root.kind == RootKind::ActiveSession {
            root.kind = RootKind::ArchivedRetainedSession;
        }
    }
    let mut a: Vec<_> = pre
        .roots()
        .iter()
        .map(|x| serde_json::to_string(x).expect("root serializes"))
        .collect();
    let mut b: Vec<_> = post_roots
        .iter()
        .map(|x| serde_json::to_string(x).expect("root serializes"))
        .collect();
    a.sort();
    b.sort();
    if a != b {
        return Verification::Failed;
    }
    Verification::Passed
}

#[cfg(test)]
#[path = "../tests/lifecycle_common/mod.rs"]
mod property_fixture;
#[cfg(test)]
mod properties {
    use super::*;
    use proptest::{prelude::*, test_runner::RngSeed};

    fn fixture() -> (LifecyclePlan, ArtifactGraph, EffectObservation) {
        let c = super::property_fixture::case(0);
        let plan = c.plan(LifecycleIntent::Unarchive);
        assert_eq!(plan.content().status, PlanStatus::Ready);
        let mut v = serde_json::to_value(&c.graph).unwrap();
        for node in v["nodes"].as_array_mut().unwrap() {
            for observation in node["observations"].as_array_mut().unwrap() {
                if observation["kind"] == "session" {
                    observation["value"]["state"] = "active".into();
                }
            }
        }
        for root in v["roots"].as_array_mut().unwrap() {
            if root["kind"] == "archived_retained_session" {
                root["kind"] = "active_session".into();
            }
        }
        let graph: ArtifactGraph = serde_json::from_value(v).unwrap();
        let integrity = evaluate_integrity(&graph, &c.proof);
        (
            plan,
            c.graph,
            EffectObservation {
                native: NativeObservation::Active,
                graph,
                integrity,
            },
        )
    }
    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d385f6c69666531), .. ProptestConfig::default() })]
        #[test]
        fn changed_target_metadata_never_passes(value in 0u64..u64::MAX) {
            let (plan, pre, mut obs) = fixture();
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            for node in v["nodes"].as_array_mut().unwrap() {
                for item in node["observations"].as_array_mut().unwrap() {
                    if item["kind"] == "session" { item["value"]["last_active"] = format!("changed-{value}").into(); }
                }
            }
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Failed);
        }
        #[test]
        fn wrong_native_identity_never_passes(value in 0u64..u64::MAX) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            for node in v["nodes"].as_array_mut().unwrap() {
                for item in node["observations"].as_array_mut().unwrap() {
                    if item["kind"] == "session" { item["value"]["provider_session_id"] = format!("wrong-{value}").into(); }
                }
            }
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
        #[test]
        fn forged_integrity_report_never_succeeds(failed in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
            obs.integrity.verification_status = if failed { VerificationStatus::Failed } else { VerificationStatus::Unknown };
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
        #[test]
        fn changed_profile_never_succeeds(value in 0u64..u64::MAX) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            v["context"]["provider"]["fingerprint"]["adapter_version"] = format!("other-{value}").into();
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
        #[test]
        fn absent_or_stale_target_root_never_succeeds(stale in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            if stale {
                for root in v["roots"].as_array_mut().unwrap() {
                    if root["kind"] == "active_session" { root["kind"] = "archived_retained_session".into(); }
                }
            } else { v["roots"] = serde_json::json!([]); }
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
        #[test]
        fn changed_coverage_is_not_a_retained_match(value in 0u64..u64::MAX) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            v["context"]["coverage"] = format!("different-{value}").into();
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Failed);
        }
        #[test]
        fn incomplete_graph_never_proves_completion(partial in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            v["context"]["completeness"] = if partial { "partial" } else { "unavailable" }.into();
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
        #[test]
        fn missing_or_incoherent_integrity_contract_never_passes(missing in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            let mut proof = obs.integrity.proof_context.clone();
            if missing { proof.contract = None; } else { proof.contract.as_mut().unwrap().coherent = false; }
            obs.integrity = evaluate_integrity(&obs.graph, &proof);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
        #[test]
        fn project_identity_change_is_not_retained(value in 0u64..u64::MAX) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            for node in v["nodes"].as_array_mut().unwrap() {
                for item in node["observations"].as_array_mut().unwrap() {
                    if item["kind"] == "session" { item["value"]["project_id"] = format!("project-{value}").into(); }
                }
            }
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Failed);
        }
        #[test]
        fn changed_observation_evidence_status_never_passes(real in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            v["context"]["evidence_status"] = if real { "real_partial" } else { "unverified" }.into();
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_ne!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
        }
        #[test]
        fn missing_required_relationship_never_passes(value in 0u16..u16::MAX) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            if value % 2 == 0 { v["edges"] = serde_json::json!([]); }
            else { v["edges"][0]["kind"] = "associated_with".into(); }
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_ne!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
        }
        #[test]
        fn altered_integrity_rule_list_never_passes(value in 0usize..20) {
            let (plan, pre, mut obs) = fixture();
            obs.integrity.rules.truncate(value % (obs.integrity.rules.len() + 1));
            if obs.integrity.rules != evaluate_integrity(&obs.graph, &obs.integrity.proof_context).rules {
                prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
            } else {
                prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
            }
        }
        #[test]
        fn partial_node_coverage_never_passes(index in 0usize..4) {
            let (plan, pre, mut obs) = fixture();
            let mut proof = obs.integrity.proof_context.clone();
            let len = proof.node_coverage.len();
            if let Some(coverage) = proof.node_coverage.get_mut(index % len) { coverage.completeness = Completeness::Partial; }
            obs.integrity = evaluate_integrity(&obs.graph, &proof);
            prop_assert_ne!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
        }
        #[test]
        fn absent_relationship_coverage_never_passes(value in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            let mut proof = obs.integrity.proof_context.clone();
            if value { proof.relationship_coverage.clear(); } else { proof.relationship_coverage[0].completeness = Completeness::Partial; }
            obs.integrity = evaluate_integrity(&obs.graph, &proof);
            prop_assert_ne!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
        }
        #[test]
        fn additional_protection_root_cannot_be_ignored(value in 0u16..u16::MAX) {
            let (plan, pre, mut obs) = fixture();
            let mut v = serde_json::to_value(&obs.graph).unwrap();
            let mut root = v["roots"].as_array().unwrap().iter().find(|root| root["kind"] == "active_session").unwrap().clone();
            root["kind"] = if value % 2 == 0 { "user_protected" } else { "provider_global" }.into();
            v["roots"].as_array_mut().unwrap().push(root);
            obs.graph = serde_json::from_value(v).unwrap();
            obs.integrity = evaluate_integrity(&obs.graph, &obs.integrity.proof_context);
            prop_assert_ne!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
        }
        #[test]
        fn unproven_native_state_never_succeeds(archived in any::<bool>()) {
            let (plan, pre, mut obs) = fixture();
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Passed);
            obs.native = if archived { NativeObservation::Archived } else { NativeObservation::Unknown };
            prop_assert_eq!(evaluate_post(&plan, Some(&pre), &obs), Verification::Unknown);
        }
    }
}
