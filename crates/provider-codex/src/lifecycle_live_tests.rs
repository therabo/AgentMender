// Explicit opt-in integration against a disposable, synthetic provider home.
// The fixture proof below is deliberately synthetic and does not establish real discovery readiness.
#[cfg(test)]
mod tests {
    use super::super::*;
    use agentmender_persistence::{OwnedStateRoot, TransactionStore};
    use std::{
        fs,
        process::{Command, Stdio},
    };
    const TARGET: &str = "70000000-1111-4111-8111-000000000001";
    const CHILD: &str = "80000000-1111-4111-8111-000000000002";
    mod fixture {
        use agentmender_domain::*;
        use std::path::Path;

        fn id(value: &str) -> EntityId {
            EntityId::new(value).unwrap()
        }
        fn evidence() -> GraphEvidence {
            GraphEvidence {
                observation: Evidence {
                    source: EvidenceSource::Fixture,
                    confidence: Confidence::Authoritative,
                    provider_version: Some(super::VERSION.into()),
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
        pub fn case(root: &Path, executable: &Path) -> Case {
            let extra = 0;
            let ev = evidence();
            let fingerprint = ProviderFingerprint {
                executable: Some(executable.into()),
                executable_version: Some(super::VERSION.into()),
                adapter_version: env!("CARGO_PKG_VERSION").into(),
                interface_id: Some("codex-app-server-0.165.0-alpha.1.2".into()),
                layout_id: Some(super::LAYOUT.into()),
            };
            let provider = Provider {
                kind: ProviderKind::Codex,
                presence: Presence::StateOnly,
                state_root: root.into(),
                fingerprint: fingerprint.clone(),
                capabilities: ProviderCapabilities::new(),
                compatibility: CompatibilityProfile {
                    status: CompatibilityStatus::ReadOnlyKnown,
                    interface_stability: InterfaceStability::OfficialStable,
                    reason: "synthetic only".into(),
                },
                installations: vec![],
                evidence_status: EvidenceStatus::SnapshotVerified,
                diagnostics: vec![],
            };
            let mut b = ArtifactGraph::builder(GraphContext {
                provider,
                evidence_status: EvidenceStatus::SnapshotVerified,
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
                id: id(&format!("codex:session:{}", super::TARGET)),
                provider_session_id: super::TARGET.into(),
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
            let child = b.observe(NodeObservation::Session(Session {
                id: id(&format!("codex:session:{}", super::CHILD)),
                provider_session_id: super::CHILD.into(),
                project_id: None,
                state: SessionState::Archived,
                last_active: Some("2026-10-27T00:00:00Z".into()),
                evidence: ev.observation.clone(),
            }));
            b.add_node_evidence(&child, ev.clone()).unwrap();
            b.add_root(ProtectedRoot {
                node: child.clone(),
                kind: RootKind::ArchivedRetainedSession,
            })
            .unwrap();
            b.add_edge(ReferenceEdge {
                source: child,
                target: target.clone(),
                kind: EdgeKind::SpawnedBy,
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
                    provider_version: super::VERSION.into(),
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
                relationship_coverage: vec![
                    RelationshipCoverage {
                        source_class: GraphNodeKind::Session,
                        target_class: GraphNodeKind::Session,
                        kind: EdgeKind::References,
                        completeness: Completeness::Complete,
                        evidence: vec![ev.clone()],
                    },
                    RelationshipCoverage {
                        source_class: GraphNodeKind::Session,
                        target_class: GraphNodeKind::Session,
                        kind: EdgeKind::SpawnedBy,
                        completeness: Completeness::Complete,
                        evidence: vec![ev.clone()],
                    },
                ],
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
                protocol_evidence_digest: super::PROTOCOL.into(),
                effect_scope: NativeEffectScope::TargetRestoreWithProviderManagedState,
            };
            let mut effect_evidence = ev.clone();
            effect_evidence.rule_id = id("synthetic-native-effects");
            effect_evidence.observation.rule = "synthetic exhaustive native effect proof".into();
            let planning = PlanningContext {
                scope: DiscoveryScope::default(),
                capabilities: vec![capability.clone()],
                effects: Some(NativeEffectBoundary {
                    contract_id: id(super::CONTRACT),
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
    }

    fn seed(root: &Path, id: &str, source: Value) {
        let folder = root.join("archived_sessions");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("rollout-2026-10-29T10-00-00-{id}.jsonl"));
        let meta = json!({"id":id,"timestamp":"2026-10-29T10:00:00.000Z","cwd":root,"originator":"codex_cli_rs","cli_version":VERSION,"source":source,"model_provider":"openai","base_instructions":{"text":"Synthetic AgentMender lifecycle fixture only."}});
        let records = [
            json!({"timestamp":"2026-10-29T10:00:00.000Z","type":"session_meta","payload":meta}),
            json!({"timestamp":"2026-10-29T10:00:01.000Z","type":"event_msg","payload":{"type":"user_message","message":"Synthetic lifecycle fixture; no model call.","images":[],"local_images":[],"text_elements":[]}}),
        ];
        fs::write(
            path,
            records.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
    }

    // Setup and independent observation use only fixed read methods, and only a fresh test-owned root.
    fn test_server(root: &Path, binary: &Path, real_home: &Path) -> Transport {
        fn lit(p: &Path) -> String {
            format!(
                "\"{}\"",
                p.to_str()
                    .unwrap()
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
            )
        }
        let root_rule = lit(root);
        let home_rule = lit(real_home);
        let exe_rule = lit(binary);
        let policy = format!(
            "(version 1)(allow default)(deny file-write* (require-not (subpath {root_rule})))(deny network*)(deny process-fork)(deny mach-lookup)(deny file-read-data (require-all (subpath {home_rule}) (require-not (literal {exe_rule}))))"
        );
        let child = Command::new("/usr/bin/sandbox-exec")
            .args(["-p", &policy])
            .arg(binary)
            .args(["app-server", "--listen", "stdio://"])
            .env_clear()
            .env("HOME", root)
            .env("TMPDIR", root)
            .env("CODEX_HOME", root)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut t = Transport::from_child(child, Duration::from_secs(15)).unwrap();
        t.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"agentmender-isolated-test","version":"0.1.0"},"capabilities":{"experimentalApi":false}}})).unwrap();
        validate_initialize(&lifecycle_response(&mut t, 1).unwrap(), root).unwrap();
        t.send(json!({"method":"initialized","params":{}})).unwrap();
        t
    }
    fn query_list(t: &mut Transport, request: u64, archived: bool) -> Vec<String> {
        t.send(json!({"id":request,"method":"thread/list","params":{"limit":100,"archived":archived,"useStateDbOnly":true,"sourceKinds":["cli","subAgentThreadSpawn","unknown"]}})).unwrap();
        let result = lifecycle_response(t, request).unwrap();
        assert!(result["result"]["nextCursor"].is_null());
        result["result"]["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["id"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn exact_synthetic_profile_fields_are_independently_required() {
        let root = Path::new("/synthetic-profile");
        let executable = Path::new("/synthetic-profile/Codex.app/Contents/MacOS/codex");
        let case = fixture::case(root, executable);
        let plan = case.plan(LifecycleIntent::Unarchive);
        assert_eq!(plan.content().status, PlanStatus::Ready);
        let adapter = CodexLifecycleAdapter::new(
            CodexAdapter::new(ProviderLocation {
                state_root: root.into(),
                executable: Some(executable.into()),
            })
            .with_evidence_helper(executable.into(), "/synthetic-home".into()),
        );
        let original = plan.content().clone();
        assert_eq!(adapter.validate_plan_content(&original), Ok(()));
        let mut variations = Vec::new();
        let mut p = original.clone();
        p.provider.executable_version = Some("wrong".into());
        p.capability.as_mut().unwrap().fingerprint = p.provider.clone();
        variations.push((p, EffectError::ProfileMismatch));
        let mut p = original.clone();
        p.provider.interface_id = Some("wrong".into());
        p.capability.as_mut().unwrap().fingerprint = p.provider.clone();
        variations.push((p, EffectError::ProfileMismatch));
        let mut p = original.clone();
        p.provider.layout_id = Some("wrong".into());
        p.capability.as_mut().unwrap().fingerprint = p.provider.clone();
        variations.push((p, EffectError::ProfileMismatch));
        let mut p = original.clone();
        p.capability.as_mut().unwrap().protocol_evidence_digest =
            format!("sha256:{}", "a".repeat(64));
        variations.push((p, EffectError::ProfileMismatch));
        let mut p = original.clone();
        p.effect_contract.as_mut().unwrap().id = EntityId::new("wrong").unwrap();
        variations.push((p, EffectError::ProfileMismatch));
        let mut p = original.clone();
        p.native_target_id = Some(EntityId::new("wrong").unwrap());
        variations.push((p, EffectError::NativeIdentityMismatch));
        let mut p = original.clone();
        p.request.target.state_root = "/wrong".into();
        variations.push((p, EffectError::NativeIdentityMismatch));
        let mut p = original.clone();
        p.operations[0].affected.clear();
        variations.push((p, EffectError::NativeIdentityMismatch));
        for (p, error) in variations {
            assert_eq!(adapter.validate_plan_content(&p), Err(error));
        }
    }

    #[test]
    #[ignore = "explicit synthetic native integration; requires opt-in and exact installed binary"]
    fn isolated_durable_unarchive_restores_only_target() {
        assert_eq!(
            std::env::var("AGENTMENDER_ALLOW_ISOLATED_NATIVE").as_deref(),
            Ok("1")
        );
        let binary = PathBuf::from(
            std::env::var_os("AGENTMENDER_CODEX_BINARY").expect("explicit binary path"),
        );
        assert!(binary.is_absolute());
        // Never launch caller-selected code until it matches the complete attested bundle.
        let bundle = bundle_root(&binary).expect("attested bundle layout");
        read_pinned_source(&binary, 256 * 1024 * 1024, HASH, None).expect("exact binary digest");
        for (name, size, digest) in RESOURCES {
            read_pinned_source(&bundle.join(name), size, digest, Some(size))
                .expect("exact bundle resource");
        }
        let real_home = PathBuf::from(std::env::var_os("HOME").expect("HOME"))
            .canonicalize()
            .unwrap();
        let temp = tempfile::Builder::new()
            .prefix("agentmender-native-test-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let base = temp.path().canonicalize().unwrap();
        let provider = base.join("provider");
        fs::create_dir(&provider).unwrap();
        let provider = provider.canonicalize().unwrap();
        seed(&provider, TARGET, json!("cli"));
        seed(
            &provider,
            CHILD,
            json!({"subagent":{"thread_spawn":{"parent_thread_id":TARGET,"depth":1}}}),
        );
        let mut setup = test_server(&provider, &binary, &real_home);
        for (id, target) in [(2, TARGET), (3, CHILD)] {
            setup.send(json!({"id":id,"method":"thread/read","params":{"threadId":target,"includeTurns":false}})).unwrap();
            let r = lifecycle_response(&mut setup, id).unwrap();
            assert_eq!(r["result"]["thread"]["id"], target);
        }
        let archived = query_list(&mut setup, 4, true);
        assert!(archived.iter().any(|v| v == TARGET));
        assert!(archived.iter().any(|v| v == CHILD));
        drop(setup);

        let case = fixture::case(&provider, &binary);
        let plan = case.plan(LifecycleIntent::Unarchive);
        assert_eq!(
            plan.content().status,
            PlanStatus::Ready,
            "{:?}",
            plan.content().blockers
        );
        assert!(
            plan.content()
                .artifacts
                .iter()
                .any(
                    |entry| entry.id.local_id.as_str() == format!("codex:session:{CHILD}")
                        && entry.role == PlannedArtifactRole::Retained
                )
        );
        let report = evaluate_integrity(&case.graph, &case.proof);
        let state =
            OwnedStateRoot::create(&base.join("authority"), std::slice::from_ref(&provider))
                .unwrap();
        let mut store = TransactionStore::open(state).unwrap();
        let id = store.create_transaction(&plan).unwrap();
        store.prepare_recovery(&id, 0).unwrap();
        store.prepare(&id, 1).unwrap();
        store
            .grant_approval(&id, 2, &plan, &case.graph, &report, &case.planning)
            .unwrap();
        let reader = CodexAdapter::new(ProviderLocation {
            state_root: provider.clone(),
            executable: Some(binary.clone()),
        })
        .with_evidence_helper(binary.clone(), real_home.clone());
        let adapter = CodexLifecycleAdapter::new(reader);
        adapter.preflight(&plan).unwrap();
        let intent = store
            .record_intent(&id, 3, &plan, &case.graph, &report, &case.planning)
            .unwrap();
        assert_eq!(store.inspect(&id).unwrap().sequence(), 4);
        assert_eq!(
            store.inspect(&id).unwrap().state(),
            TransactionState::OutcomeUnknown
        );
        let attempt = adapter.dispatch(intent, &plan);
        assert_eq!(attempt.outcome, NativeOutcome::Accepted);
        store
            .record_observation(&id, 4, &attempt.observation, Observation::Applied)
            .unwrap();
        assert_eq!(
            store.inspect(&id).unwrap().state(),
            TransactionState::EffectObserved
        );
        let mut observer = test_server(&provider, &binary, &real_home);
        let active = query_list(&mut observer, 2, false);
        let archived = query_list(&mut observer, 3, true);
        assert!(active.iter().any(|v| v == TARGET));
        assert!(!archived.iter().any(|v| v == TARGET));
        assert!(archived.iter().any(|v| v == CHILD));
        assert!(!active.iter().any(|v| v == CHILD));
    }
}
