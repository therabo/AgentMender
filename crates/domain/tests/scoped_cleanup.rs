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
        complete_removal_evidence: None,
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

/// Frozen by executing the pre-continuation d299ccb tests in a separate checkout.
/// This oracle is independent of the new schema-3 implementation.
#[test]
fn historical_v2_plan_binding_and_journal_retain_their_exact_encoding() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/historical-cleanup-v2.json")).unwrap();
    let historical: LifecyclePlan = serde_json::from_value(fixture["plan"].clone()).unwrap();
    let (current, _, _, _) = ready_cleanup();
    assert_eq!(current, historical);
    assert_eq!(
        serde_json::to_value(TransactionBinding::from_plan(&historical).unwrap()).unwrap(),
        fixture["binding"]
    );
    assert_eq!(
        serde_json::to_value(report(&manifest())).unwrap(),
        fixture["report"]
    );
    let wire = include_str!("fixtures/historical-cleanup-v2-journal.json").trim_end();
    let records: Vec<JournalRecord> = serde_json::from_str(wire).unwrap();
    assert_eq!(serde_json::to_string(&records).unwrap(), wire);
    let replay = TransactionSnapshot::replay(&records).unwrap();
    assert_eq!(replay.state(), TransactionState::Succeeded);
    assert_eq!(replay.review_plan(), Some(&historical));
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

#[test]
fn schema_three_binds_expanded_scope_and_never_claims_desktop_acceptance() {
    let (_, graph, proof, mut context) = ready_cleanup();
    let mut primary_evidence = context.cleanup_evidence.clone().unwrap();
    primary_evidence.manifest.profile.adapter = "scoped-cleanup-v2".into();
    let primary = primary_evidence.manifest.clone();
    let manifest = CompleteRemovalManifest {
        desktop: None,
        primary: primary.clone(),
        actions: vec![
            CompleteRemovalAction::PrimaryPayload,
            CompleteRemovalAction::StateThreadRecords,
        ],
        profile: CompleteRemovalProfile {
            identity: id("codex-scoped-cleanup-v2"),
            admission: ProfileAdmission::PrimaryOnly,
            writer_requirement: WriterRequirement::CompatibleLifetimeFence,
        },
        required_surfaces: vec![PersistentSurface {
            id: id("codex-desktop-reconstruction-unverified"),
        }],
        technical_residuals: primary.residuals.clone(),
    };
    context.complete_removal_evidence = Some(CompleteRemovalEvidence {
        desktop_boundary: None,
        manifest: manifest.clone(),
        primary_evidence,
    });
    let conflicting = plan_lifecycle(
        &graph,
        &evaluate_integrity(&graph, &proof),
        &context,
        LifecycleRequest {
            target: primary.target.clone(),
            intent: LifecycleIntent::CleanupSession,
        },
    );
    assert_eq!(conflicting.content().status, PlanStatus::Blocked);
    assert!(
        conflicting
            .content()
            .blockers
            .contains(&PlanBlocker::EffectBoundaryInvalid)
    );
    context.cleanup_evidence = None;
    let plan = plan_lifecycle(
        &graph,
        &evaluate_integrity(&graph, &proof),
        &context,
        LifecycleRequest {
            target: primary.target.clone(),
            intent: LifecycleIntent::CleanupSession,
        },
    );
    assert_ne!(conflicting.state_fingerprint(), plan.state_fingerprint());
    // Historical schema 3 wire content omits the new optional field entirely.
    let wire = serde_json::to_value(&plan).unwrap();
    assert!(
        wire["content"]["complete_removal_manifest"]
            .get("desktop")
            .is_none()
    );
    let absent = DesktopFileCommitment {
        identity: None,
        preimage_sha256: None,
        retained_sha256: None,
        metadata_sha256: None,
        target_units: 0,
    };
    let digest = "0".repeat(64);
    let mut unadmitted = manifest.clone();
    unadmitted.desktop = Some(DesktopRemovalManifest {
        desktop_root: primary.root.clone(),
        user_data: primary.root.clone(),
        bundle_path: "/Applications/Codex.app".into(),
        bundle_id: "com.openai.codex".into(),
        bundle_version: "26.1027.51404".into(),
        bundle_build: "23880".into(),
        asar_sha256: digest.clone(),
        stable_worker_sha256: digest.clone(),
        bundle_sha256: digest.clone(),
        cli_sha256: digest.clone(),
        local_host: "local".into(),
        source_kind: "local".into(),
        target_title: "target".into(),
        legacy: DesktopLegacyCommitment {
            files: vec![],
            layout_id: "absent".into(),
            origin: "app://-".into(),
            logical_legacy_entries: 0,
        },
        catalog: DesktopCatalogCommitment {
            identity: primary.database.clone(),
            schema_sha256: digest.clone(),
            migration_sha256: digest.clone(),
            preimage_sha256: digest.clone(),
            retained_sha256: digest.clone(),
            target_rows: 1,
            target_sequence_sha256: digest.clone(),
            revision_sha256: digest,
        },
        global_backup: absent.clone(),
        global_main: absent.clone(),
        session_index: absent,
    });
    unadmitted.profile.identity = id("codex-desktop-complete-removal-v1");
    unadmitted.profile.admission = ProfileAdmission::DesktopUnaccepted;
    unadmitted.profile.writer_requirement = WriterRequirement::RequiresProviderClosed;
    unadmitted.actions = vec![
        CompleteRemovalAction::DesktopCatalog,
        CompleteRemovalAction::GlobalBackup,
        CompleteRemovalAction::GlobalMain,
        CompleteRemovalAction::SessionIndex,
        CompleteRemovalAction::PrimaryPayload,
        CompleteRemovalAction::StateThreadRecords,
    ];
    assert!(!validate_complete_removal_manifest(&unadmitted));
    let mut desktop_context = context.clone();
    desktop_context
        .complete_removal_evidence
        .as_mut()
        .unwrap()
        .manifest = unadmitted;
    let desktop_plan = plan_lifecycle(
        &graph,
        &evaluate_integrity(&graph, &proof),
        &desktop_context,
        LifecycleRequest {
            target: primary.target.clone(),
            intent: LifecycleIntent::CleanupSession,
        },
    );
    assert_eq!(desktop_plan.content().schema_version, 4);
    assert_eq!(desktop_plan.content().status, PlanStatus::Blocked);
    assert!(TransactionBinding::from_plan(&desktop_plan).is_err());
    assert_ne!(desktop_plan.digest(), plan.digest());
    assert_eq!(plan.content().schema_version, 3);
    assert_eq!(plan.content().status, PlanStatus::Ready);
    assert!(TransactionBinding::from_plan(&plan).is_ok());
    assert_ne!(
        TransactionBinding::from_plan(&plan).unwrap(),
        TransactionBinding::from_plan(&ready_cleanup().0).unwrap()
    );
    assert_eq!(
        serde_json::from_slice::<LifecyclePlan>(&serde_json::to_vec(&plan).unwrap()).unwrap(),
        plan
    );
    let actions: Vec<_> = manifest
        .actions
        .iter()
        .map(|action| CompleteRemovalActionProgress {
            action: *action,
            outcome: CleanupActionOutcome::Removed,
        })
        .collect();
    for i in 0..256 {
        let result = CompleteRemovalReport {
            desktop_collateral_unchanged: None,
            primary: report(&primary),
            actions: actions.clone(),
            surfaces: vec![PersistentSurfaceObservation {
                id: manifest.required_surfaces[0].id.clone(),
                observation: if i % 3 == 0 {
                    PersistentObservation::Absent
                } else if i % 3 == 1 {
                    PersistentObservation::Present
                } else {
                    PersistentObservation::Unknown
                },
            }],
        };
        assert_eq!(
            assess_complete_removal(&manifest, &result).outcome,
            SessionRemovalOutcome::ProjectionIncomplete
        );
        let mut mismatch = result.clone();
        mismatch.actions[0].outcome = CleanupActionOutcome::AlreadyAbsent;
        assert!(!validate_complete_removal_report(&manifest, &mismatch));
        let mut bad = result.clone();
        bad.actions.reverse();
        assert!(!validate_complete_removal_report(&manifest, &bad));
        let mut bad = result;
        bad.surfaces.clear();
        assert!(!validate_complete_removal_report(&manifest, &bad));
    }
    for terminal in [CleanupOutcome::Blocked, CleanupOutcome::Failed] {
        let mut primary_report = report(&primary);
        primary_report.outcome = terminal;
        primary_report.canonical_row_absent = false;
        let mut observed = CompleteRemovalReport {
            desktop_collateral_unchanged: None,
            primary: primary_report,
            actions: actions[..1].to_vec(),
            surfaces: vec![PersistentSurfaceObservation {
                id: manifest.required_surfaces[0].id.clone(),
                observation: PersistentObservation::Unknown,
            }],
        };
        assert!(validate_cleanup_report(&primary, &observed.primary));
        assert!(!validate_complete_removal_report(&manifest, &observed));
        observed.actions = actions.clone();
        assert!(!validate_complete_removal_report(&manifest, &observed));
        observed.actions.clear();
        assert!(!validate_complete_removal_report(&manifest, &observed));
        observed.primary.actions[0].outcome = CleanupActionOutcome::Unknown;
        observed.primary.actions[0].logical_unlinked_bytes = 0;
        observed.primary.logical_unlinked_bytes = 0;
        observed.actions.push(actions[0].clone());
        assert!(!validate_complete_removal_report(&manifest, &observed));
        observed.actions.clear();
        assert!(!validate_complete_removal_report(&manifest, &observed));
        observed.primary.actions[0].outcome = CleanupActionOutcome::Retained;
        observed.actions.push(CompleteRemovalActionProgress {
            action: CompleteRemovalAction::PrimaryPayload,
            outcome: CleanupActionOutcome::Unknown,
        });
        assert!(!validate_complete_removal_report(&manifest, &observed));
        observed.actions.clear();
        assert!(validate_complete_removal_report(&manifest, &observed));
    }
    let binding = TransactionBinding::from_plan(&plan).unwrap();
    let tx = TransactionId::from_bytes([41; 32]);
    let mut records = vec![JournalRecord::new_v2(
        tx.clone(),
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
        id: ApprovalId::from_bytes([42; 32]),
        nonce: ApprovalNonce::from_bytes([43; 32]),
        scope: ApprovalScope {
            transaction_id: tx,
            binding: binding.clone(),
            recovery_digest,
            operation_index: 0,
            direction: ApprovalDirection::ForwardOnly,
        },
        principal: ApprovalPrincipal::LocalInteractiveUser,
        issued_at: 100,
        expires_at: 200,
    };
    let authentication = AuthorizationAuthentication {
        key_id: ApprovalKeyId::from_bytes([44; 32]),
        tag: AuthenticationTag::from_bytes([45; 32]),
    };
    common::push(
        &mut records,
        TransactionEvent::Authorization {
            action: ApprovalAction::Granted {
                grant: grant.clone(),
            },
            authentication: authentication.clone(),
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
            authentication,
        },
    );
    let snapshot = TransactionSnapshot::replay(&records).unwrap();
    assert!(
        snapshot
            .next_record(TransactionEvent::CompleteRemovalActionObserved {
                progress: CompleteRemovalActionProgress {
                    action: CompleteRemovalAction::PrimaryPayload,
                    outcome: CleanupActionOutcome::Unknown
                }
            })
            .is_err()
    );
    assert!(
        snapshot
            .next_record(TransactionEvent::CompleteRemovalActionObserved {
                progress: actions[1].clone()
            })
            .is_err()
    );
    common::push(
        &mut records,
        TransactionEvent::CompleteRemovalActionObserved {
            progress: actions[0].clone(),
        },
    );
    common::push(
        &mut records,
        TransactionEvent::CompleteRemovalActionObserved {
            progress: actions[1].clone(),
        },
    );
    let snapshot = TransactionSnapshot::replay(&records).unwrap();
    let mut final_report = CompleteRemovalReport {
        desktop_collateral_unchanged: None,
        primary: report(&primary),
        actions: actions.clone(),
        surfaces: vec![PersistentSurfaceObservation {
            id: manifest.required_surfaces[0].id.clone(),
            observation: PersistentObservation::Unknown,
        }],
    };
    for terminal in [CleanupOutcome::Blocked, CleanupOutcome::Failed] {
        let mut contradicted = final_report.clone();
        contradicted.primary.outcome = terminal;
        contradicted.primary.canonical_row_absent = false;
        assert!(
            snapshot
                .next_record(TransactionEvent::CompleteRemovalObserved {
                    report: contradicted
                })
                .is_err()
        );
    }
    let mut omitted = final_report.clone();
    omitted.actions.pop();
    assert!(
        snapshot
            .next_record(TransactionEvent::CompleteRemovalObserved { report: omitted })
            .is_err()
    );
    final_report.actions[0].outcome = CleanupActionOutcome::AlreadyAbsent;
    assert!(
        snapshot
            .next_record(TransactionEvent::CompleteRemovalObserved {
                report: final_report
            })
            .is_err()
    );
    assert!(
        snapshot
            .next_record(TransactionEvent::CleanupObserved {
                report: report(&primary)
            })
            .is_err()
    );
    let mut wrong = manifest.clone();
    wrong.profile.identity = id("uncompiled");
    assert!(!validate_complete_removal_manifest(&wrong));
    let mut wrong = manifest;
    wrong.profile.admission = ProfileAdmission::CompleteAccepted;
    assert!(!validate_complete_removal_manifest(&wrong));
}
