const SCOPED_CODEX_VERSION: &str = "synthetic-v1";
const SCOPED_CODEX_EXECUTABLE_SHA256: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SCOPED_CODEX_SCHEMA_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SCOPED_ADAPTER: &str = "synthetic-adapter";
use agentmender_domain::*;
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
#[test]
fn exact_identity_and_retained_boundaries() {
    let m = manifest();
    assert!(validate_cleanup_manifest(&m));
    let mut drift = m.clone();
    drift.database_payload_paths[0].path = "/owned/sessions/foreign.jsonl".into();
    assert!(!validate_cleanup_manifest(&drift));
    drift = m.clone();
    drift.payloads[0].header_session_id = id("12345678-1234-1234-1234-123456789abd");
    assert!(!validate_cleanup_manifest(&drift));
    drift = m.clone();
    drift.payloads[0].identity.links = 2;
    assert!(!validate_cleanup_manifest(&drift));
    drift = m.clone();
    drift.payloads[0].identity.inode = drift.database.inode;
    assert!(!validate_cleanup_manifest(&drift));
    drift = m.clone();
    drift.residuals[0].artifact = drift.payloads[0].artifact.clone();
    assert!(!validate_cleanup_manifest(&drift));
    drift = m.clone();
    drift.retained_files[0] = drift.payloads[0].identity.clone();
    assert!(!validate_cleanup_manifest(&drift));
}
#[test]
fn required_absence_and_optional_failure_are_truthful() {
    let mut m = manifest();
    let mut r = report(&m);
    assert!(validate_cleanup_report(&m, &r));
    r.canonical_row_absent = false;
    assert!(!validate_cleanup_report(&m, &r));
    r = report(&m);
    r.actions[0].outcome = CleanupActionOutcome::Failed;
    r.actions[0].logical_unlinked_bytes = 0;
    r.logical_unlinked_bytes = 0;
    assert!(!validate_cleanup_report(&m, &r));
    m.optional_index = Some(CleanupIndexAction {
        identity: file("/owned/session_index.jsonl", 5, 8),
        preimage_sha256: "d".repeat(64),
        record_sha256: "e".repeat(64),
    });
    r = report(&m);
    r.optional_index = Some(CleanupActionOutcome::Failed);
    assert!(validate_cleanup_report(&m, &r));
    r.outcome = CleanupOutcome::Removed;
    assert!(!validate_cleanup_report(&m, &r));
    r = report(&m);
    r.optional_index = Some(CleanupActionOutcome::Removed);
    r.actions[0].artifact = artifact("unknown-cache", GraphNodeKind::Artifact);
    assert!(!validate_cleanup_report(&m, &r));
}

#[test]
fn historical_primary_reports_never_prove_complete_session_removal() {
    let mut m = manifest();
    let original_manifest = serde_json::to_vec(&m).unwrap();
    let r = report(&m);
    let original_report = serde_json::to_vec(&r).unwrap();
    let assessment = assess_legacy_session_removal(&m, &r);
    assert!(assessment.primary_removed);
    assert!(!assessment.projection_clean);
    assert!(!assessment.product_complete());
    assert_eq!(
        assessment.outcome,
        SessionRemovalOutcome::ProjectionIncomplete
    );
    assert_eq!(assessment.maximum_safe_cleanup, MaximumSafeCleanup::Unknown);
    assert!(assessment.coverage.is_none());
    assert_eq!(serde_json::to_vec(&m).unwrap(), original_manifest);
    assert_eq!(serde_json::to_vec(&r).unwrap(), original_report);

    // Even a residue-free historical receipt lacks GUI and restart evidence.
    m.residuals.clear();
    let mut r = report(&m);
    r.outcome = CleanupOutcome::Removed;
    assert!(validate_cleanup_report(&m, &r));
    assert!(!assess_legacy_session_removal(&m, &r).product_complete());

    r.canonical_row_absent = false;
    let assessment = assess_legacy_session_removal(&m, &r);
    assert!(!assessment.primary_removed);
    assert!(!assessment.product_complete());
}
#[test]
fn deterministic_model_4096_cases() {
    let mut seed = 0x4d382e3253434f50u64;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut m = manifest();
        let size = seed >> 8;
        m.payloads[0].identity.size_bytes = size;
        let mut r = report(&m);
        r.logical_planned_bytes = size;
        r.logical_unlinked_bytes = size;
        r.actions[0].logical_unlinked_bytes = size;
        let forged = seed & 1 != 0;
        let missing = seed & 2 != 0;
        let overcount = seed & 4 != 0;
        let unknown_action = seed & 8 != 0;
        if forged {
            r.actions[0].artifact =
                artifact("foreign-same-cwd-prefix-child", GraphNodeKind::Artifact);
        }
        if missing {
            r.canonical_row_absent = false;
        }
        if overcount {
            r.logical_unlinked_bytes = size + 1;
        }
        if unknown_action {
            r.actions[0].outcome = CleanupActionOutcome::Unknown;
            r.actions[0].logical_unlinked_bytes = 0;
            if !overcount {
                r.logical_unlinked_bytes = 0;
            }
        }
        assert_eq!(
            validate_cleanup_report(&m, &r),
            !(forged || missing || overcount || unknown_action)
        );
    }
}

#[allow(dead_code)]
mod common;
#[test]
fn wire_versions_are_strict_and_native_fields_stay_absent() {
    let case = common::case(0);
    let native = case.plan(LifecycleIntent::Archive);
    let wire = serde_json::to_value(&native).unwrap();
    assert_eq!(wire["content"]["schema_version"], 1);
    assert!(wire["content"].get("cleanup_manifest").is_none());
    let round: LifecyclePlan = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        serde_json::to_vec(&round).unwrap(),
        serde_json::to_vec(&native).unwrap()
    );
    let mut forged = wire;
    forged["content"]["request"]["intent"] = serde_json::json!("cleanup_session");
    assert!(serde_json::from_value::<LifecyclePlan>(forged).is_err());
    let scoped = case.plan(LifecycleIntent::CleanupSession);
    assert_eq!(scoped.content().schema_version, 2);
    assert_eq!(scoped.content().status, PlanStatus::Blocked);
    let mut wire = serde_json::to_value(&scoped).unwrap();
    assert!(serde_json::from_value::<LifecyclePlan>(wire.clone()).is_ok());
    wire["content"]["schema_version"] = serde_json::json!(1);
    assert!(serde_json::from_value::<LifecyclePlan>(wire).is_err());
}

fn ready_cleanup() -> (
    LifecyclePlan,
    ArtifactGraph,
    IntegrityContext,
    PlanningContext,
) {
    let m = manifest();
    let sample = common::case(0);
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

#[test]
fn ready_v2_plan_binds_manifest_and_revalidates() {
    let (plan, graph, proof, mut planning) = ready_cleanup();
    assert_eq!(plan.content().status, PlanStatus::Ready);
    assert_eq!(plan.content().integrity.status, VerificationStatus::Unknown);
    assert!(TransactionBinding::from_plan(&plan).is_ok());
    let wire = serde_json::to_vec(&plan).unwrap();
    let round: LifecyclePlan = serde_json::from_slice(&wire).unwrap();
    assert_eq!(round, plan);
    planning
        .cleanup_evidence
        .as_mut()
        .unwrap()
        .manifest
        .payloads[0]
        .identity
        .inode += 1;
    assert_eq!(
        plan.validate_against(&graph, &evaluate_integrity(&graph, &proof), &planning),
        PlanValidation::Stale
    );
}

#[test]
fn consumed_cleanup_replay_requires_approved_observations_and_allows_reconciliation() {
    let (plan, _, _, _) = ready_cleanup();
    let binding = TransactionBinding::from_plan(&plan).unwrap();
    let id = TransactionId::from_bytes([11; 32]);
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
    common::prepare(&mut records, &binding);
    let recovery_digest = TransactionSnapshot::replay(&records)
        .unwrap()
        .recovery()
        .unwrap()
        .digest();
    let grant = ApprovalGrant {
        id: ApprovalId::from_bytes([12; 32]),
        nonce: ApprovalNonce::from_bytes([13; 32]),
        scope: ApprovalScope {
            transaction_id: id,
            binding: binding.clone(),
            recovery_digest,
            operation_index: 0,
            direction: ApprovalDirection::ForwardOnly,
        },
        principal: ApprovalPrincipal::LocalInteractiveUser,
        issued_at: 100,
        expires_at: 200,
    };
    let auth = AuthorizationAuthentication {
        key_id: ApprovalKeyId::from_bytes([14; 32]),
        tag: AuthenticationTag::from_bytes([15; 32]),
    };
    common::push(
        &mut records,
        TransactionEvent::Authorization {
            action: ApprovalAction::Granted {
                grant: grant.clone(),
            },
            authentication: auth.clone(),
        },
    );
    common::push(
        &mut records,
        TransactionEvent::Authorization {
            action: ApprovalAction::Consumed {
                approval_id: grant.id,
                at: 101,
                plan: binding.plan_digest().clone(),
                state: binding.state_fingerprint().clone(),
                operation_index: 0,
            },
            authentication: auth,
        },
    );
    let snapshot = TransactionSnapshot::replay(&records).unwrap();
    assert!(
        snapshot
            .next_record(TransactionEvent::Observed {
                observation: Observation::Applied
            })
            .is_err()
    );
    let mut forged = report(&manifest());
    forged.actions[0].artifact = artifact("foreign", GraphNodeKind::Artifact);
    assert!(
        snapshot
            .next_record(TransactionEvent::CleanupObserved { report: forged })
            .is_err()
    );
    let mut unknown = report(&manifest());
    unknown.outcome = CleanupOutcome::OutcomeUnknown;
    unknown.canonical_row_absent = false;
    common::push(
        &mut records,
        TransactionEvent::CleanupObserved { report: unknown },
    );
    assert_eq!(
        TransactionSnapshot::replay(&records).unwrap().state(),
        TransactionState::RecoveryRequired
    );
    common::push(
        &mut records,
        TransactionEvent::CleanupObserved {
            report: report(&manifest()),
        },
    );
    assert_eq!(
        TransactionSnapshot::replay(&records).unwrap().state(),
        TransactionState::Succeeded
    );
}
