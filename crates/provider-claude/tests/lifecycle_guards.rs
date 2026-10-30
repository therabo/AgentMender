use agentmender_domain::*;
use agentmender_provider_api::ProviderAdapter;
use agentmender_provider_claude::ClaudeAdapter;
use proptest::{prelude::*, test_runner::RngSeed};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn fixture(desktop: bool) -> (PathBuf, ClaudeAdapter) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(if desktop {
        "../../fixtures/claude/4.29.0-observed"
    } else {
        "../../fixtures/claude/2.1.311-synthetic"
    });
    let adapter = if desktop {
        ClaudeAdapter::from_desktop_fixture(path.clone())
    } else {
        ClaudeAdapter::from_fixture(path.clone())
    }
    .unwrap();
    (path, adapter)
}
fn assert_guards(
    adapter: &ClaudeAdapter,
    snapshot: &DiscoverySnapshot,
    all_targets: bool,
    selected_intent: Option<LifecycleIntent>,
) {
    let scope = DiscoveryScope::default();
    let graph = adapter.build_graph(snapshot).unwrap();
    let planning = adapter.planning_context(&graph, &scope).unwrap();
    assert!(planning.capabilities.is_empty());
    assert!(planning.effects.is_none());
    let integrity = evaluate_integrity(&graph, &adapter.integrity_context(&graph).unwrap());
    for class in [GraphNodeKind::Project, GraphNodeKind::Session] {
        let targets: Vec<_> = graph
            .nodes()
            .iter()
            .filter(|node| node.id.class == class)
            .collect();
        assert!(!targets.is_empty(), "fixture needs {class:?}");
        for node in targets
            .into_iter()
            .take(if all_targets { usize::MAX } else { 1 })
        {
            for intent in [
                LifecycleIntent::Archive,
                LifecycleIntent::Unarchive,
                LifecycleIntent::SoftDelete,
                LifecycleIntent::Purge,
                LifecycleIntent::Repair,
            ]
            .into_iter()
            .filter(|intent| selected_intent.is_none_or(|selected| selected == *intent))
            {
                let plan = plan_lifecycle(
                    &graph,
                    &integrity,
                    &planning,
                    LifecycleRequest {
                        target: node.id.clone(),
                        intent,
                    },
                );
                let content = plan.content();
                assert_eq!(content.request.target, node.id);
                if all_targets {
                    assert_eq!(content.status, PlanStatus::Unsupported);
                } else {
                    assert_ne!(content.status, PlanStatus::Ready);
                }
                assert!(content.operations.is_empty());
                assert!(content.capability.is_none());
                assert!(content.effect_contract.is_none());
            }
        }
    }
}
#[test]
fn fixtures_have_no_lifecycle_authority_and_are_unchanged() {
    for desktop in [false, true] {
        let (path, adapter) = fixture(desktop);
        let before = files(&path);
        let snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
        assert_guards(&adapter, &snapshot, true, None);
        assert_eq!(files(&path), before);
    }
}
#[test]
fn missing_copied_transcripts_cannot_create_authority() {
    for desktop in [false, true] {
        let (source, _) = fixture(desktop);
        let original_files = files(&source);
        let temp = tempfile::tempdir().unwrap();
        for (relative, bytes) in &original_files {
            let destination = temp.path().join(relative);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::write(destination, bytes).unwrap();
        }
        let adapter = if desktop {
            ClaudeAdapter::from_desktop_fixture(temp.path().into())
        } else {
            ClaudeAdapter::from_fixture(temp.path().into())
        }
        .unwrap();
        let scope = DiscoveryScope::default();
        let baseline = adapter.discover(&scope).unwrap();
        let graph = adapter.build_graph(&baseline).unwrap();
        let targets: Vec<_> = graph
            .nodes()
            .iter()
            .filter(|node| {
                matches!(
                    node.id.class,
                    GraphNodeKind::Project | GraphNodeKind::Session
                )
            })
            .map(|node| node.id.clone())
            .collect();
        assert!(targets.iter().any(|id| id.class == GraphNodeKind::Project));
        assert!(targets.iter().any(|id| id.class == GraphNodeKind::Session));
        let transcripts: Vec<_> = original_files
            .keys()
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        assert!(!transcripts.is_empty());
        for path in transcripts {
            fs::remove_file(temp.path().join(path)).unwrap();
        }
        let after = adapter.discover(&scope).unwrap();
        let graph = adapter.build_graph(&after).unwrap();
        let planning = adapter.planning_context(&graph, &scope).unwrap();
        assert!(planning.capabilities.is_empty());
        assert!(planning.effects.is_none());
        let integrity = evaluate_integrity(&graph, &adapter.integrity_context(&graph).unwrap());
        for target in targets {
            for intent in [
                LifecycleIntent::Archive,
                LifecycleIntent::Unarchive,
                LifecycleIntent::SoftDelete,
                LifecycleIntent::Purge,
                LifecycleIntent::Repair,
            ] {
                let request = LifecycleRequest {
                    target: target.clone(),
                    intent,
                };
                let plan = plan_lifecycle(&graph, &integrity, &planning, request.clone());
                let content = plan.content();
                assert_eq!(content.request, request);
                assert_ne!(content.status, PlanStatus::Ready);
                assert!(content.operations.is_empty());
                assert!(content.capability.is_none());
                assert!(content.effect_contract.is_none());
            }
        }
        assert_eq!(files(&source), original_files);
    }
}
proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: RngSeed::Fixed(0x4d395f636c617564), .. ProptestConfig::default() })]
    #[test]
    fn claimed_profile_and_identity_never_create_authority(desktop in any::<bool>(), known in any::<bool>(), project_id in any::<bool>(), path in any::<bool>(), session_id in any::<bool>(), choice in 0usize..5, version in prop_oneof![Just("2.1.222".to_owned()), Just("2.1.308".to_owned()), Just("2.1.314".to_owned()), "[a-z0-9]{1,20}"]) {
        let (fixture_path, adapter) = fixture(desktop);
        let before = files(&fixture_path);
        let mut snapshot = adapter.discover(&DiscoveryScope::default()).unwrap();
        snapshot.provider.compatibility.status = if known { CompatibilityStatus::ReadOnlyKnown } else { CompatibilityStatus::AuditOnlyUnknown };
        snapshot.provider.fingerprint.executable_version = Some(version.clone());
        for project in &mut snapshot.projects {
            project.provider_project_id = project_id.then(|| version.clone());
            project.path = path.then(|| PathBuf::from("/synthetic/cwd"));
            project.evidence.provider_version = Some(version.clone());
        }
        for session in &mut snapshot.sessions {
            session.provider_session_id = if session_id { version.clone() } else { String::new() };
            session.evidence.provider_version = Some(version.clone());
        }
        for artifact in &mut snapshot.artifacts {
            artifact.locator = if path { PathBuf::from("/synthetic/present") } else { PathBuf::new() };
        }
        assert_guards(&adapter, &snapshot, false, Some([LifecycleIntent::Archive, LifecycleIntent::Unarchive, LifecycleIntent::SoftDelete, LifecycleIntent::Purge, LifecycleIntent::Repair][choice]));
        prop_assert_eq!(files(&fixture_path), before);
    }
}
