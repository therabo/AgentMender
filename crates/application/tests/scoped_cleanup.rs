#[allow(dead_code)]
mod lifecycle_common;
use agentmender_application::lifecycle::{execute_scoped_cleanup, reconcile_cleanup};
use agentmender_domain::*;
use agentmender_persistence::*;
use agentmender_provider_api::{ProviderError, ScopedCleanupObservation, ScopedCleanupReadPort};
use std::cell::Cell;
const SCOPED_CODEX_VERSION: &str = "0.166.0";
const SCOPED_CODEX_EXECUTABLE_SHA256: &str =
    "39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1";
const SCOPED_CODEX_SCHEMA_SHA256: &str =
    "1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647";
const SCOPED_ADAPTER: &str = "scoped-cleanup-v1";

use std::path::PathBuf;

fn id(s: &str) -> EntityId {
    EntityId::new(s).unwrap()
}
fn artifact(s: &str, class: GraphNodeKind) -> ArtifactId {
    ArtifactId {
        provider: ProviderKind::Codex,
        state_root: "/owned".into(),
        class,
        local_id: id(s),
    }
}
fn file(path: &str, inode: u64, size_bytes: u64) -> CleanupFileIdentity {
    CleanupFileIdentity {
        path: PathBuf::from(path),
        device: 1,
        inode,
        links: 1,
        size_bytes,
        mtime_seconds: 1,
        mtime_nanoseconds: 0,
    }
}
fn manifest() -> CleanupManifest {
    let native = id("12345678-1234-1234-1234-123456789abc");
    CleanupManifest {
        target: artifact("target", GraphNodeKind::Session),
        native_session_id: native.clone(),
        root: file("/owned", 1, 0),
        profile: CleanupProfile {
            version: SCOPED_CODEX_VERSION.into(),
            executable_sha256: SCOPED_CODEX_EXECUTABLE_SHA256.into(),
            schema_sha256: SCOPED_CODEX_SCHEMA_SHA256.into(),
            adapter: SCOPED_ADAPTER.into(),
            platform: "darwin-arm64".into(),
            journal_mode: "wal".into(),
            history_layout: "legacy-plain".into(),
        },
        database: file("/owned/state_5.sqlite", 2, 100),
        rows: [
            "threads",
            "thread_dynamic_tools",
            "thread_read_receipts",
            "thread_attachments",
            "guardian_review_feedback",
            "thread_spawn_edges",
        ]
        .iter()
        .map(|name| CleanupRows {
            table: (*name).into(),
            expected_count: u64::from(*name == "threads"),
            preimage_sha256: "a".repeat(64),
        })
        .collect(),
        database_payload_paths: vec![CleanupDatabasePath {
            path: "/owned/sessions/rollout-2026-01-01-12345678-1234-1234-1234-123456789abc.jsonl"
                .into(),
        }],
        payloads: vec![CleanupPayload {
            artifact: artifact("payload", GraphNodeKind::Artifact),
            identity: file(
                "/owned/sessions/rollout-2026-01-01-12345678-1234-1234-1234-123456789abc.jsonl",
                3,
                123,
            ),
            header_session_id: native,
            header_sha256: "b".repeat(64),
        }],
        optional_index: None,
        retained_database_sha256: "c".repeat(64),
        retained_files: vec![file("/owned/sessions/child.jsonl", 4, 7)],
        residuals: vec![CleanupResidual {
            artifact: artifact("unknown-cache", GraphNodeKind::Artifact),
            kind: CleanupResidualKind::Unknown,
        }],
    }
}
fn report(m: &CleanupManifest) -> CleanupReport {
    CleanupReport {
        outcome: CleanupOutcome::RemovedWithResiduals,
        actions: vec![CleanupActionReport {
            artifact: m.payloads[0].artifact.clone(),
            outcome: CleanupActionOutcome::Removed,
            logical_unlinked_bytes: 123,
        }],
        canonical_row_absent: true,
        retained_primary_unchanged: true,
        optional_index: None,
        post_cutoff_residue: false,
        logical_planned_bytes: 123,
        logical_unlinked_bytes: 123,
        physical_reclaimed_bytes: None,
    }
}
fn ready_cleanup() -> (
    LifecyclePlan,
    ArtifactGraph,
    IntegrityContext,
    PlanningContext,
) {
    let m = manifest();
    let sample = lifecycle_common::case(0);
    let mut context = sample.graph.context().clone();
    context.provider.state_root = m.root.path.clone();
    context.provider.fingerprint.executable_version = Some(SCOPED_CODEX_VERSION.into());
    let mut builder = ArtifactGraph::builder(context);
    let rule = sample.graph.rules()[0].clone();
    builder.add_rule(rule.clone()).unwrap();
    let raw = Evidence {
        source: EvidenceSource::Fixture,
        confidence: Confidence::Authoritative,
        provider_version: Some(SCOPED_CODEX_VERSION.into()),
        rule: "synthetic".into(),
    };
    let evidence = GraphEvidence {
        observation: raw.clone(),
        rule_id: rule.id,
        nature: EvidenceNature::Observation,
        locator: None,
        observed_at: None,
    };
    let target = builder.observe(NodeObservation::Session(Session {
        id: m.target.local_id.clone(),
        provider_session_id: m.native_session_id.to_string(),
        project_id: None,
        state: SessionState::Active,
        last_active: None,
        evidence: raw.clone(),
    }));
    builder
        .add_node_evidence(&target, evidence.clone())
        .unwrap();
    let payload = builder.observe(NodeObservation::Artifact(Artifact {
        id: m.payloads[0].artifact.local_id.clone(),
        kind: ArtifactKind::Attachment,
        locator: m.payloads[0].identity.path.clone(),
        size_bytes: Some(m.payloads[0].identity.size_bytes),
        evidence: raw,
    }));
    builder.add_node_evidence(&payload, evidence).unwrap();
    let graph = builder.finish().unwrap();
    let proof = IntegrityContext::default();
    let planning = PlanningContext {
        scope: DiscoveryScope::default(),
        capabilities: vec![],
        effects: None,
        complete_removal_evidence: None,
        cleanup_evidence: Some(ScopedCleanupEvidence {
            manifest: m,
            root_boundary: CleanupBoundaryObservation::CompleteAndUnshared,
            references: CleanupBoundaryObservation::CompleteAndUnshared,
            writer_fence: CleanupBoundaryObservation::CompleteAndUnshared,
            schema: CleanupBoundaryObservation::CompleteAndUnshared,
        }),
    };
    let plan = plan_lifecycle(
        &graph,
        &evaluate_integrity(&graph, &proof),
        &planning,
        LifecycleRequest {
            target,
            intent: LifecycleIntent::CleanupSession,
        },
    );
    (plan, graph, proof, planning)
}

struct Adapter {
    graph: ArtifactGraph,
    proof: IntegrityContext,
    planning: PlanningContext,
}
impl ScopedCleanupReadPort for Adapter {
    fn observe_cleanup(
        &self,
        _: &str,
        _: &DiscoveryScope,
    ) -> Result<ScopedCleanupObservation, ProviderError> {
        Ok(ScopedCleanupObservation {
            graph: self.graph.clone(),
            context: self.planning.clone(),
        })
    }
}
struct Port {
    report: CleanupReport,
    count: Cell<usize>,
    unknown: bool,
}
impl DurableCleanupPort for Port {
    fn preflight(&self, _: &LifecyclePlan) -> Result<(), EffectError> {
        Ok(())
    }
    fn dispatch(&self, intent: DurableIntent, _: &LifecyclePlan) -> CleanupAttempt {
        self.count.set(self.count.get() + 1);
        CleanupAttempt {
            observation: intent.into_observation(),
            outcome: if self.unknown {
                NativeOutcome::Unknown(EffectError::Timeout)
            } else {
                NativeOutcome::Accepted
            },
            report: (!self.unknown).then(|| self.report.clone()),
        }
    }
    fn observe(&self, _: &LifecyclePlan) -> Result<CleanupReport, EffectError> {
        Ok(self.report.clone())
    }
}
fn setup() -> (
    tempfile::TempDir,
    TransactionStore,
    TransactionId,
    Adapter,
    CleanupReport,
) {
    let (plan, graph, proof, planning) = ready_cleanup();
    assert_eq!(plan.content().status, PlanStatus::Ready);
    let dir = tempfile::tempdir().unwrap();
    let root =
        OwnedStateRoot::create(&dir.path().join("owned"), &[dir.path().join("provider")]).unwrap();
    let mut store = TransactionStore::open(root).unwrap();
    let id = store.create_transaction(&plan).unwrap();
    store.prepare_recovery(&id, 0).unwrap();
    store.prepare(&id, 1).unwrap();
    store
        .grant_approval(
            &id,
            2,
            &plan,
            &graph,
            &evaluate_integrity(&graph, &proof),
            &planning,
        )
        .unwrap();
    let report = report(plan.content().cleanup_manifest.as_ref().unwrap());
    (
        dir,
        store,
        id,
        Adapter {
            graph,
            proof,
            planning,
        },
        report,
    )
}
#[test]
fn consumed_cleanup_records_only_manifest_bound_report() {
    let (_dir, mut store, id, adapter, mut report) = setup();
    report.actions[0].artifact.local_id = EntityId::new("foreign").unwrap();
    let port = Port {
        report,
        count: Cell::new(0),
        unknown: false,
    };
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &port).is_err());
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::OutcomeUnknown
    );
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &port).is_err());
    assert_eq!(port.count.get(), 1);
}
#[test]
fn ambiguous_cleanup_reconciles_without_redispatch() {
    let (dir, mut store, id, adapter, mut report) = setup();
    report.actions[0].outcome = CleanupActionOutcome::AlreadyAbsent;
    report.actions[0].logical_unlinked_bytes = 0;
    report.logical_unlinked_bytes = 0;
    let port = Port {
        report,
        count: Cell::new(0),
        unknown: true,
    };
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &port).is_err());
    drop(store);
    let root =
        OwnedStateRoot::open(&dir.path().join("owned"), &[dir.path().join("provider")]).unwrap();
    let mut store = TransactionStore::open(root).unwrap();
    assert_eq!(
        reconcile_cleanup(&mut store, &id, &port).unwrap(),
        TransactionState::Succeeded
    );
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &port).is_err());
    assert_eq!(port.count.get(), 1);
}

#[test]
fn accepted_cleanup_requires_and_records_valid_postconditions() {
    let (_dir, mut store, id, adapter, report) = setup();
    let port = Port {
        report,
        count: Cell::new(0),
        unknown: false,
    };
    assert_eq!(
        execute_scoped_cleanup(&mut store, &id, &adapter, &port).unwrap(),
        TransactionState::Succeeded
    );
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &port).is_err());
    assert_eq!(port.count.get(), 1);
}

#[test]
fn partial_cleanup_is_journaled_and_reconciles_read_only_after_reopen() {
    let (dir, mut store, id, adapter, mut report) = setup();
    let plan = store.inspect(&id).unwrap().review_plan().unwrap().clone();
    let intent = store
        .record_intent(
            &id,
            3,
            &plan,
            &adapter.graph,
            &evaluate_integrity(&adapter.graph, &adapter.proof),
            &adapter.planning,
        )
        .unwrap();
    let seq = intent.sequence();
    let receipt = intent.into_observation();
    report.outcome = CleanupOutcome::OutcomeUnknown;
    report.canonical_row_absent = false;
    report.retained_primary_unchanged = false;
    store
        .record_cleanup_observation(&id, seq, &receipt, report.clone())
        .unwrap();
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::RecoveryRequired
    );
    drop(store);
    let root =
        OwnedStateRoot::open(&dir.path().join("owned"), &[dir.path().join("provider")]).unwrap();
    let mut store = TransactionStore::open(root).unwrap();
    report.actions[0].outcome = CleanupActionOutcome::AlreadyAbsent;
    report.actions[0].logical_unlinked_bytes = 0;
    report.logical_unlinked_bytes = 0;
    let port = Port {
        report,
        count: Cell::new(0),
        unknown: true,
    };
    assert_eq!(
        reconcile_cleanup(&mut store, &id, &port).unwrap(),
        TransactionState::RecoveryRequired
    );
    assert_eq!(port.count.get(), 0);
}
