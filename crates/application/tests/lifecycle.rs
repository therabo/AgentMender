mod lifecycle_common;
use agentmender_application::{
    DiscoveryService,
    lifecycle::{LifecycleError, execute, reconcile},
};
use agentmender_domain::*;
use agentmender_persistence::*;
use agentmender_provider_api::{ProviderAdapter, ProviderError};
use std::cell::Cell;

struct Adapter<'a>(&'a lifecycle_common::Case);
impl ProviderAdapter for Adapter<'_> {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }
    fn detect(&self) -> Result<Provider, ProviderError> {
        Ok(self.0.graph.context().provider.clone())
    }
    fn discover(&self, _: &DiscoveryScope) -> Result<DiscoverySnapshot, ProviderError> {
        Ok(DiscoverySnapshot::empty(self.detect()?))
    }
    fn build_graph(&self, _: &DiscoverySnapshot) -> Result<ArtifactGraph, ProviderError> {
        Ok(self.0.graph.clone())
    }
    fn integrity_context(&self, _: &ArtifactGraph) -> Result<IntegrityContext, ProviderError> {
        Ok(self.0.proof.clone())
    }
    fn planning_context(
        &self,
        _: &ArtifactGraph,
        _: &DiscoveryScope,
    ) -> Result<PlanningContext, ProviderError> {
        Ok(self.0.planning.clone())
    }
}
struct FakePort {
    before: ArtifactGraph,
    after: ArtifactGraph,
    proof: IntegrityContext,
    mode: NativeOutcome,
    native_after: NativeObservation,
    count: Cell<usize>,
}
impl DurableEffectPort for FakePort {
    fn preflight(&self, _: &LifecyclePlan) -> Result<(), EffectError> {
        Ok(())
    }
    fn dispatch(&self, intent: DurableIntent, _: &LifecyclePlan) -> EffectAttempt {
        self.count.set(self.count.get() + 1);
        EffectAttempt {
            observation: intent.into_observation(),
            outcome: self.mode,
        }
    }
    fn observe(&self, _: &LifecyclePlan) -> Result<EffectObservation, EffectError> {
        let (native, graph) = if self.count.get() == 0 {
            (NativeObservation::Archived, &self.before)
        } else {
            (self.native_after, &self.after)
        };
        Ok(EffectObservation {
            native,
            graph: graph.clone(),
            integrity: evaluate_integrity(graph, &self.proof),
        })
    }
}
fn active_graph(graph: &ArtifactGraph) -> ArtifactGraph {
    let mut wire = serde_json::to_value(graph).unwrap();
    for node in wire["nodes"].as_array_mut().unwrap() {
        for observation in node["observations"].as_array_mut().unwrap() {
            if observation["kind"] == "session" && observation["value"]["id"] == "session:target" {
                observation["value"]["state"] = "active".into();
            }
        }
    }
    for root in wire["roots"].as_array_mut().unwrap() {
        if root["kind"] == "archived_retained_session"
            && root["node"]["local_id"] == "session:target"
        {
            root["kind"] = "active_session".into();
        }
    }
    serde_json::from_value(wire).unwrap()
}
fn child_case() -> lifecycle_common::Case {
    let mut c = lifecycle_common::case(0);
    let mut wire = serde_json::to_value(&c.graph).unwrap();
    let mut child = wire["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"]["local_id"] == "session:target")
        .unwrap()
        .clone();
    child["id"]["local_id"] = "session:child".into();
    child["observations"][0]["value"]["id"] = "session:child".into();
    child["observations"][0]["value"]["provider_session_id"] = "native-child".into();
    wire["nodes"].as_array_mut().unwrap().push(child.clone());
    let mut root = wire["roots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "archived_retained_session")
        .unwrap()
        .clone();
    root["node"] = child["id"].clone();
    wire["roots"].as_array_mut().unwrap().push(root);
    let mut edge = wire["edges"][0].clone();
    edge["source"] = child["id"].clone();
    edge["kind"] = "spawned_by".into();
    wire["edges"].as_array_mut().unwrap().push(edge);
    c.graph = serde_json::from_value(wire).unwrap();
    let mut coverage = c.proof.relationship_coverage[0].clone();
    coverage.kind = EdgeKind::SpawnedBy;
    c.proof.relationship_coverage.push(coverage);
    c
}
fn setup(
    c: &lifecycle_common::Case,
) -> (
    tempfile::TempDir,
    TransactionStore,
    TransactionId,
    LifecyclePlan,
) {
    let dir = tempfile::tempdir().unwrap();
    let root =
        OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")]).unwrap();
    let mut store = TransactionStore::open(root).unwrap();
    let plan = c.plan(LifecycleIntent::Unarchive);
    assert_eq!(
        plan.content().status,
        PlanStatus::Ready,
        "{:?}",
        plan.content().blockers
    );
    let id = store.create_transaction(&plan).unwrap();
    store.prepare_recovery(&id, 0).unwrap();
    store.prepare(&id, 1).unwrap();
    let report = evaluate_integrity(&c.graph, &c.proof);
    store
        .grant_approval(&id, 2, &plan, &c.graph, &report, &c.planning)
        .unwrap();
    (dir, store, id, plan)
}
fn fake(
    c: &lifecycle_common::Case,
    mode: NativeOutcome,
    native_after: NativeObservation,
) -> FakePort {
    FakePort {
        before: c.graph.clone(),
        after: active_graph(&c.graph),
        proof: c.proof.clone(),
        mode,
        native_after,
        count: Cell::new(0),
    }
}
#[test]
fn accepted_then_coherent_unchanged_retained_state_succeeds() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::Succeeded
    );
    assert_eq!(port.count.get(), 1);
    assert!(execute(&mut store, &id, &discovery, &port).is_err());
    assert_eq!(port.count.get(), 1);
}
#[test]
fn ambiguity_reconciles_without_redispatch_and_restart_cannot_succeed() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(
        &c,
        NativeOutcome::Unknown(EffectError::Timeout),
        NativeObservation::Active,
    );
    assert!(matches!(
        execute(&mut store, &id, &discovery, &port),
        Err(LifecycleError::Ambiguous(EffectError::Timeout))
    ));
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::OutcomeUnknown
    );
    assert_eq!(
        reconcile(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
    assert_eq!(port.count.get(), 1);
}
#[test]
fn accepted_response_without_proven_active_state_is_never_success() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(&c, NativeOutcome::Accepted, NativeObservation::Unknown);
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
    assert_eq!(port.count.get(), 1);
}

#[test]
fn active_session_with_stale_archived_root_never_succeeds() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let mut port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    let mut wire = serde_json::to_value(&port.after).unwrap();
    for root in wire["roots"].as_array_mut().unwrap() {
        if root["kind"] == "active_session" {
            root["kind"] = "archived_retained_session".into();
        }
    }
    port.after = serde_json::from_value(wire).unwrap();
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
}

#[test]
fn never_started_is_failed_and_cannot_retry() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(
        &c,
        NativeOutcome::NeverDispatched(EffectError::Unavailable),
        NativeObservation::Unknown,
    );
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::Failed
    );
    assert!(execute(&mut store, &id, &discovery, &port).is_err());
    assert_eq!(port.count.get(), 1);
}
#[test]
fn stale_inputs_prevent_consumption_and_dispatch() {
    let c = lifecycle_common::case(0);
    let changed = lifecycle_common::case(1);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&changed);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    assert!(execute(&mut store, &id, &discovery, &port).is_err());
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::Prepared
    );
    assert_eq!(port.count.get(), 0);
}
#[test]
fn revoked_approval_never_dispatches() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let grant = store
        .inspect(&id)
        .unwrap()
        .approval()
        .grant
        .clone()
        .unwrap();
    store.revoke_approval(&id, 3, &grant.id).unwrap();
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    assert!(execute(&mut store, &id, &discovery, &port).is_err());
    assert_eq!(port.count.get(), 0);
}
#[test]
fn retained_metadata_change_is_a_failed_verification() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let mut port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    let mut wire = serde_json::to_value(&port.after).unwrap();
    for node in wire["nodes"].as_array_mut().unwrap() {
        for observation in node["observations"].as_array_mut().unwrap() {
            if observation["kind"] == "session" {
                observation["value"]["last_active"] = "different".into();
            }
        }
    }
    port.after = serde_json::from_value(wire).unwrap();
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
    assert!(matches!(
        store.records(&id).unwrap().last().unwrap().event(),
        TransactionEvent::Verified {
            verification: Verification::Failed
        }
    ));
}
#[test]
fn archived_target_after_ambiguous_dispatch_remains_pending() {
    let c = lifecycle_common::case(0);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let mut port = fake(
        &c,
        NativeOutcome::Unknown(EffectError::Transport),
        NativeObservation::Archived,
    );
    port.after = c.graph.clone();
    assert!(matches!(
        execute(&mut store, &id, &discovery, &port),
        Err(LifecycleError::Ambiguous(EffectError::Transport))
    ));
    assert_eq!(
        reconcile(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::OutcomeUnknown
    );
    assert_eq!(port.count.get(), 1);
}

#[test]
fn root_restoration_keeps_archived_child_and_relationship() {
    let c = child_case();
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::Succeeded
    );
}
#[test]
fn unexpectedly_restored_child_cannot_be_success() {
    let c = child_case();
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let mut port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    let mut wire = serde_json::to_value(&port.after).unwrap();
    for node in wire["nodes"].as_array_mut().unwrap() {
        if node["id"]["local_id"] == "session:child" {
            node["observations"][0]["value"]["state"] = "active".into();
        }
    }
    for root in wire["roots"].as_array_mut().unwrap() {
        if root["node"]["local_id"] == "session:child" {
            root["kind"] = "active_session".into();
        }
    }
    port.after = serde_json::from_value(wire).unwrap();
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
    assert!(matches!(
        store.records(&id).unwrap().last().unwrap().event(),
        TransactionEvent::Verified {
            verification: Verification::Failed
        }
    ));
}
#[test]
fn missing_child_relationship_cannot_be_success() {
    let c = child_case();
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let mut port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    let mut wire = serde_json::to_value(&port.after).unwrap();
    wire["edges"]
        .as_array_mut()
        .unwrap()
        .retain(|edge| edge["kind"] != "spawned_by");
    port.after = serde_json::from_value(wire).unwrap();
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
}

#[test]
fn durable_restart_cut_points_never_reissue_a_dispatch_gate() {
    for cut in 0..4 {
        let c = lifecycle_common::case(0);
        let (dir, mut store, id, plan) = setup(&c);
        let report = evaluate_integrity(&c.graph, &c.proof);
        let gate = store
            .record_intent(&id, 3, &plan, &c.graph, &report, &c.planning)
            .unwrap();
        if cut >= 2 {
            let receipt = gate.into_observation();
            store
                .record_observation(&id, 4, &receipt, Observation::Applied)
                .unwrap();
            if cut == 3 {
                store.start_verification(&id, 5).unwrap();
            }
        } else {
            drop(gate);
        }
        drop(store);
        let root = OwnedStateRoot::open(&dir.path().join("owned"), &[dir.path().join("provider")])
            .unwrap();
        let mut store = TransactionStore::open(root).unwrap();
        let adapter = Adapter(&c);
        let discovery = DiscoveryService::new(vec![&adapter]);
        let port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
        if cut > 0 {
            port.count.set(1);
        }
        let expected = if cut == 0 {
            TransactionState::OutcomeUnknown
        } else {
            TransactionState::RecoveryRequired
        };
        assert_eq!(
            reconcile(&mut store, &id, &discovery, &port).unwrap(),
            expected
        );
        assert_eq!(port.count.get(), usize::from(cut > 0));
        assert!(execute(&mut store, &id, &discovery, &port).is_err());
        assert_eq!(port.count.get(), usize::from(cut > 0));
    }
}

#[test]
fn added_or_missing_descendant_cannot_be_success() {
    for added in [false, true] {
        let c = child_case();
        let (_dir, mut store, id, _) = setup(&c);
        let adapter = Adapter(&c);
        let discovery = DiscoveryService::new(vec![&adapter]);
        let mut port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
        let mut wire = serde_json::to_value(&port.after).unwrap();
        if added {
            let mut child = wire["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"]["local_id"] == "session:child")
                .unwrap()
                .clone();
            child["id"]["local_id"] = "session:new-child".into();
            child["observations"][0]["value"]["id"] = "session:new-child".into();
            child["observations"][0]["value"]["provider_session_id"] = "native-new-child".into();
            let mut edge = wire["edges"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["kind"] == "spawned_by")
                .unwrap()
                .clone();
            edge["source"] = child["id"].clone();
            wire["nodes"].as_array_mut().unwrap().push(child.clone());
            wire["edges"].as_array_mut().unwrap().push(edge);
            let mut root = wire["roots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["node"]["local_id"] == "session:child")
                .unwrap()
                .clone();
            root["node"] = child["id"].clone();
            wire["roots"].as_array_mut().unwrap().push(root);
        } else {
            wire["nodes"]
                .as_array_mut()
                .unwrap()
                .retain(|n| n["id"]["local_id"] != "session:child");
            wire["edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["source"]["local_id"] != "session:child");
            wire["roots"]
                .as_array_mut()
                .unwrap()
                .retain(|r| r["node"]["local_id"] != "session:child");
        }
        port.after = serde_json::from_value(wire).unwrap();
        assert_eq!(
            execute(&mut store, &id, &discovery, &port).unwrap(),
            TransactionState::RecoveryRequired
        );
    }
}

#[test]
fn retained_locator_spelling_is_compared_losslessly() {
    let c = lifecycle_common::case(1);
    let (_dir, mut store, id, _) = setup(&c);
    let adapter = Adapter(&c);
    let discovery = DiscoveryService::new(vec![&adapter]);
    let mut port = fake(&c, NativeOutcome::Accepted, NativeObservation::Active);
    let mut wire = serde_json::to_value(&port.after).unwrap();
    for node in wire["nodes"].as_array_mut().unwrap() {
        for observation in node["observations"].as_array_mut().unwrap() {
            if observation["kind"] == "artifact" {
                observation["value"]["locator"] = "/synthetic-planning/./artifact-0".into();
            }
        }
    }
    port.after = serde_json::from_value(wire).unwrap();
    assert_eq!(
        evaluate_integrity(&port.after, &c.proof).verification_status,
        VerificationStatus::Healthy
    );
    assert_eq!(
        execute(&mut store, &id, &discovery, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
    assert!(matches!(
        store.records(&id).unwrap().last().unwrap().event(),
        TransactionEvent::Verified {
            verification: Verification::Failed
        }
    ));
}
