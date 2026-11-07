#![cfg(test)]
use super::*;
use agentmender_application::{cleanup_inputs, lifecycle::execute_scoped_cleanup};
use agentmender_domain::{DiscoveryScope, TransactionState};
use agentmender_persistence::{OwnedStateRoot, TransactionStore};
use agentmender_provider_api::ProviderLocation;
use rusqlite::Connection;
use std::fs;

#[test]
fn approved_adapter_removes_only_frozen_session_and_survives_reopen() {
    run_acceptance(false, None, None, false);
}

#[test]
fn interrupted_after_unlink_keeps_row_and_never_retries() {
    run_acceptance(false, Some(TestFault::AfterUnlink), None, false);
}

#[test]
fn interrupted_after_commit_reconciles_read_only_without_retry() {
    run_acceptance(false, Some(TestFault::AfterCommit), None, false);
}

#[test]
#[ignore = "requires exact Codex 0.166 binary and built CLI evidence helper on macOS arm64"]
fn exact_binary_and_sandboxed_worker_on_disposable_fixture() {
    run_acceptance(true, None, None, false);
}

#[test]
#[ignore = "requires exact Codex 0.166 binary and built CLI evidence helper on macOS arm64"]
fn exact_binary_and_sandboxed_worker_expanded_metadata_on_disposable_fixture() {
    run_acceptance(true, None, None, true);
}

#[test]
fn expanded_profile_removes_target_metadata_and_preserves_foreign_metadata() {
    run_acceptance(false, None, None, true);
}

#[test]
fn expanded_unlink_interruption_is_unknown_and_reconciliation_is_read_only() {
    run_acceptance(false, Some(TestFault::AfterUnlink), None, true);
}

#[test]
fn expanded_database_interruption_is_unknown_and_reconciliation_is_read_only() {
    run_acceptance(false, Some(TestFault::AfterCommit), None, true);
}

#[test]
fn expanded_progress_failure_stops_before_database_commit() {
    run_acceptance(false, Some(TestFault::RejectProgress), None, true);
}

fn run_acceptance(
    live: bool,
    fault: Option<TestFault>,
    crash_base: Option<&std::path::Path>,
    expanded: bool,
) {
    run_acceptance_inner(live, fault, crash_base, expanded, false);
}

#[test]
fn conversational_protection_preparation_retry_and_verified_result() {
    run_acceptance_inner(false, None, None, false, true);
}

#[test]
#[ignore = "requires pinned stable Codex and built evidence helper; fixture approval only"]
fn exact_binary_conversational_primary_workflow() {
    run_acceptance_inner(true, None, None, false, true);
}

fn run_acceptance_inner(
    live: bool,
    fault: Option<TestFault>,
    crash_base: Option<&std::path::Path>,
    expanded: bool,
    conversational: bool,
) {
    let tmp = crash_base.is_none().then(|| tempfile::tempdir().unwrap());
    let base = crash_base
        .unwrap_or_else(|| tmp.as_ref().unwrap().path())
        .canonicalize()
        .unwrap();
    let root = base.join("provider");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("sessions")).unwrap();
    fs::create_dir(root.join(".tmp")).unwrap();
    fs::create_dir(root.join("thread-writer-locks")).unwrap();
    fs::write(root.join(".tmp/rollout-maintenance.lock"), b"").unwrap();
    fs::write(root.join("thread-writer-locks/.coordination.lock"), b"").unwrap();
    let target = "12345678-1234-1234-1234-123456789abc";
    let child = "12345678-1234-1234-1234-123456789abd";
    let payload = root.join(format!(
        "sessions/rollout-2026-01-01T00-00-00-{target}.jsonl"
    ));
    let other = root.join(format!(
        "sessions/rollout-2026-01-01T00-00-01-{child}.jsonl"
    ));
    fs::write(
        &payload,
        format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{target}\",\"cli_version\":\"0.166.0\"}}}}\n"),
    )
    .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&payload)
        .unwrap()
        .set_len(64 * 1024 * 1024)
        .unwrap();
    fs::write(
        &other,
        format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{child}\"}}}}\nchild\n"),
    )
    .unwrap();
    fs::write(root.join("sessions/unknown.cache"), b"preserve").unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.pragma_update(None, "journal_mode", "WAL").unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE fixture_sequence_seed(value INTEGER PRIMARY KEY AUTOINCREMENT); DROP TABLE fixture_sequence_seed;").unwrap();
    let schema: Vec<(String, String, String, Option<String>)> = serde_json::from_str(include_str!(
        "../../../compatibility/codex/0.165.0-alpha.1.2/state-schema-5.json"
    ))
    .unwrap();
    for kind in ["table", "index", "trigger", "view"] {
        for (entry_kind, name, _, sql) in &schema {
            if entry_kind == kind
                && name != "sqlite_sequence"
                && let Some(sql) = sql
            {
                db.execute_batch(sql).unwrap();
            }
        }
    }
    for (id, path) in [(target, &payload), (child, &other)] {
        db.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,cli_version) VALUES (?1,?2,1,1,'cli','openai',?3,'same','read-only','never','0.166.0')",(id,path.to_str().unwrap(),base.to_str().unwrap())).unwrap();
    }
    let shared_asset = base.join("shared-attachment.asset");
    if expanded {
        fs::write(&shared_asset, b"shared attachment must survive").unwrap();
        let attachment = serde_json::json!({"path":shared_asset}).to_string();
        for (id, thread) in [("target", target), ("foreign", child)] {
            db.execute("INSERT INTO thread_attachments(id,thread_id,attachment_type,identity_key,payload,created_at) VALUES (?1,?2,'fixture','shared-key',?3,1)", (id, thread, &attachment)).unwrap();
            db.execute(
                "INSERT INTO guardian_review_feedback(id,thread_id,record) VALUES (?1,?2,?3)",
                (id, thread, b"fixture blob".as_slice()),
            )
            .unwrap();
        }
    }
    let mut persist = 1;
    db.file_control(
        None::<&str>,
        rusqlite::FileControl::PersistWal(&mut persist),
    )
    .unwrap();
    drop(db);
    let executable = live.then(|| {
        std::path::PathBuf::from(
            std::env::var_os("AGENTMENDER_CODEX166_BINARY")
                .expect("AGENTMENDER_CODEX166_BINARY required"),
        )
        .canonicalize()
        .unwrap()
    });
    let mut reader = crate::CodexAdapter::new(ProviderLocation {
        state_root: root.clone(),
        executable,
    });
    if live {
        let helper = std::path::PathBuf::from(
            std::env::var_os("AGENTMENDER_HELPER").expect("AGENTMENDER_HELPER required"),
        )
        .canonicalize()
        .unwrap();
        let home = std::path::PathBuf::from(std::env::var_os("HOME").expect("HOME required"))
            .canonicalize()
            .unwrap();
        reader = reader.with_evidence_helper(helper, home);
    }
    let mut adapter = if live {
        CodexScopedCleanupAdapter::new(reader)
    } else {
        CodexScopedCleanupAdapter::for_owned_test(reader)
    };
    adapter.fault = fault;
    adapter.expanded = expanded;
    if conversational {
        conversation_acceptance(base.as_path(), &root, adapter, target, child, &payload);
        return;
    }
    if !live && fault.is_none() && crash_base.is_none() {
        let outside = base.join("foreign-rollout.jsonl");
        let contents = format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{child}\",\"history_base\":{{\"id\":\"{target}\"}}}}}}\n"
        );
        fs::write(&outside, &contents).unwrap();
        let db = Connection::open(root.join("state_5.sqlite")).unwrap();
        db.execute(
            "UPDATE threads SET rollout_path=?1 WHERE id=?2",
            (outside.to_str().unwrap(), child),
        )
        .unwrap();
        let mut persist = 1;
        db.file_control(
            None::<&str>,
            rusqlite::FileControl::PersistWal(&mut persist),
        )
        .unwrap();
        drop(db);
        assert!(cleanup_inputs(&adapter, target, &DiscoveryScope::default()).is_err());
        assert_eq!(fs::read_to_string(&outside).unwrap(), contents);
        assert!(payload.exists());
        let db = Connection::open(root.join("state_5.sqlite")).unwrap();
        db.execute(
            "UPDATE threads SET rollout_path=?1 WHERE id=?2",
            (other.to_str().unwrap(), child),
        )
        .unwrap();
        let mut persist = 1;
        db.file_control(
            None::<&str>,
            rusqlite::FileControl::PersistWal(&mut persist),
        )
        .unwrap();
        drop(db);
        fs::remove_file(outside).unwrap();
    }
    let capture = adapter.capture_target(target).unwrap();
    assert!(
        crate::scoped_config::validate(&root, capture.cwd.as_deref().unwrap()).is_ok(),
        "configuration rejected: {:?}",
        capture.cwd
    );
    assert!(
        crate::scoped_files::observe(&root, target, capture.target_path.as_deref().unwrap())
            .is_ok(),
        "filesystem observe failed"
    );
    let inputs = cleanup_inputs(&adapter, target, &DiscoveryScope::default()).unwrap();
    assert_eq!(
        inputs.plan.content().status,
        agentmender_domain::PlanStatus::Ready,
        "{:?}",
        inputs.plan.content().blockers
    );
    let owned = OwnedStateRoot::create(&base.join("owned"), std::slice::from_ref(&root)).unwrap();
    let mut store = TransactionStore::open(owned).unwrap();
    let id = store.create_transaction(&inputs.plan).unwrap();
    store.prepare_recovery(&id, 0).unwrap();
    store.prepare(&id, 1).unwrap();
    store
        .grant_approval(
            &id,
            2,
            &inputs.plan,
            &inputs.graph,
            &inputs.integrity,
            &inputs.context,
        )
        .unwrap();
    let root_mtime = fs::metadata(&root).unwrap().modified().unwrap();
    fs::write(root.join("config.toml"), b"sqlite_home = '/elsewhere'\n").unwrap();
    let rejected = if expanded {
        agentmender_application::lifecycle::execute_complete_removal(
            &mut store, &id, &adapter, &adapter,
        )
    } else {
        execute_scoped_cleanup(&mut store, &id, &adapter, &adapter)
    };
    assert!(rejected.is_err());
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::Prepared
    );
    assert!(payload.exists());
    fs::remove_file(root.join("config.toml")).unwrap();
    fs::File::open(&root)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(root_mtime))
        .unwrap();
    let execution = if expanded {
        agentmender_application::lifecycle::execute_complete_removal(
            &mut store, &id, &adapter, &adapter,
        )
    } else {
        execute_scoped_cleanup(&mut store, &id, &adapter, &adapter)
    };
    if expanded {
        assert_eq!(
            fs::read(&shared_asset).unwrap(),
            b"shared attachment must survive"
        );
    }
    if let Some(fault) = fault {
        assert!(execution.is_err());
        assert_eq!(
            store.inspect(&id).unwrap().state(),
            TransactionState::RecoveryRequired
        );
        assert!(!payload.exists());
        assert!(other.exists());
        assert_eq!(
            fs::read(root.join("sessions/unknown.cache")).unwrap(),
            b"preserve"
        );
        let db = Connection::open(root.join("state_5.sqlite")).unwrap();
        let target_count: i64 = db
            .query_row("SELECT count(*) FROM threads WHERE id=?1", [target], |r| {
                r.get(0)
            })
            .unwrap();
        let child_count: i64 = db
            .query_row("SELECT count(*) FROM threads WHERE id=?1", [child], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            target_count,
            i64::from(matches!(
                fault,
                TestFault::AfterUnlink | TestFault::RejectProgress
            ))
        );
        assert_eq!(child_count, 1);
        drop(db);
        drop(store);
        let owned = OwnedStateRoot::open(&base.join("owned"), std::slice::from_ref(&root)).unwrap();
        let mut store = TransactionStore::open(owned).unwrap();
        let reconciled = if expanded {
            agentmender_application::lifecycle::reconcile_complete_removal(
                &mut store, &id, &adapter,
            )
        } else {
            agentmender_application::lifecycle::reconcile_cleanup(&mut store, &id, &adapter)
        };
        assert_eq!(reconciled.unwrap(), TransactionState::RecoveryRequired);
        let retry = if expanded {
            agentmender_application::lifecycle::execute_complete_removal(
                &mut store, &id, &adapter, &adapter,
            )
        } else {
            execute_scoped_cleanup(&mut store, &id, &adapter, &adapter)
        };
        assert!(retry.is_err());
        assert!(!payload.exists());
        assert!(other.exists());
        return;
    }
    assert_eq!(execution.unwrap(), TransactionState::Succeeded);
    assert!(!payload.exists());
    assert!(other.exists());
    assert_eq!(
        fs::read(root.join("sessions/unknown.cache")).unwrap(),
        b"preserve"
    );
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM threads WHERE id=?1", [child], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
    if expanded {
        for table in ["thread_attachments", "guardian_review_feedback"] {
            let sql = format!("SELECT count(*) FROM {table} WHERE thread_id=?1");
            assert_eq!(
                db.query_row(&sql, [target], |r| r.get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                db.query_row(&sql, [child], |r| r.get::<_, i64>(0)).unwrap(),
                1
            );
        }
    }
    drop(db);
    drop(store);
    let owned = OwnedStateRoot::open(&base.join("owned"), std::slice::from_ref(&root)).unwrap();
    let mut store = TransactionStore::open(owned).unwrap();
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::Succeeded
    );
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &adapter).is_err());
}

// The child path is accepted only from a fresh private directory with a marker;
// it is test-only and never influences the executable pin or a production path.
fn crash_child(fault: TestFault, expanded: bool) {
    use std::os::unix::fs::MetadataExt;
    let Some(raw) = std::env::var_os("AGENTMENDER_TEST_CRASH_DIR") else {
        return;
    };
    let path = std::path::PathBuf::from(raw);
    let canonical = path.canonicalize().unwrap();
    assert_eq!(canonical.as_os_str(), path.as_os_str());
    assert_eq!(canonical.parent(), Some(crash_temp_base().as_path()));
    assert!(
        canonical
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("agentmender-m82-crash-")
    );
    let metadata = fs::symlink_metadata(&canonical).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    assert_eq!(metadata.mode() & 0o777, 0o700);
    assert_eq!(
        fs::read(canonical.join("sentinel")).unwrap(),
        b"agentmender disposable crash fixture"
    );
    assert!(!canonical.join("provider").exists() && !canonical.join("owned").exists());
    run_acceptance(false, Some(fault), Some(&canonical), expanded);
    panic!("injected child did not terminate at the expected cut");
}

fn crash_temp_base() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    let path = "/private/tmp";
    #[cfg(not(target_os = "macos"))]
    let path = "/tmp";
    std::path::Path::new(path).canonicalize().unwrap()
}

#[test]
fn child_terminate_after_unlink() {
    crash_child(TestFault::CrashAfterUnlink, false);
}
#[test]
fn child_terminate_after_commit() {
    crash_child(TestFault::CrashAfterCommit, false);
}

#[test]
fn expanded_child_terminate_after_unlink() {
    crash_child(TestFault::CrashAfterUnlink, true);
}
#[test]
fn expanded_child_terminate_after_commit() {
    crash_child(TestFault::CrashAfterCommit, true);
}

fn process_crash_case(child_name: &str, code: i32, target_remains: bool, expanded: bool) {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::Builder::new()
        .prefix("agentmender-m82-crash-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in(crash_temp_base())
        .unwrap();
    let base = temp.path().canonicalize().unwrap();
    fs::write(
        base.join("sentinel"),
        b"agentmender disposable crash fixture",
    )
    .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", child_name])
        .env("AGENTMENDER_TEST_CRASH_DIR", &base)
        .output()
        .unwrap();
    assert_eq!(
        status.status.code(),
        Some(code),
        "child output: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let root = base.join("provider");
    let target = "12345678-1234-1234-1234-123456789abc";
    let child = "12345678-1234-1234-1234-123456789abd";
    let payload = root.join(format!(
        "sessions/rollout-2026-01-01T00-00-00-{target}.jsonl"
    ));
    let other = root.join(format!(
        "sessions/rollout-2026-01-01T00-00-01-{child}.jsonl"
    ));
    assert!(!payload.exists());
    assert_eq!(
        fs::read(&other).unwrap(),
        format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{child}\"}}}}\nchild\n")
            .as_bytes()
    );
    assert_eq!(
        fs::read(root.join("sessions/unknown.cache")).unwrap(),
        b"preserve"
    );
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    let target_count: i64 = db
        .query_row("SELECT count(*) FROM threads WHERE id=?1", [target], |r| {
            r.get(0)
        })
        .unwrap();
    let child_count: i64 = db
        .query_row("SELECT count(*) FROM threads WHERE id=?1", [child], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(target_count, i64::from(target_remains));
    assert_eq!(child_count, 1);
    if expanded {
        for table in ["thread_attachments", "guardian_review_feedback"] {
            let sql = format!("SELECT count(*) FROM {table} WHERE thread_id=?1");
            assert_eq!(
                db.query_row(&sql, [target], |r| r.get::<_, i64>(0))
                    .unwrap(),
                i64::from(target_remains)
            );
            assert_eq!(
                db.query_row(&sql, [child], |r| r.get::<_, i64>(0)).unwrap(),
                1
            );
        }
        assert_eq!(
            fs::read(base.join("shared-attachment.asset")).unwrap(),
            b"shared attachment must survive"
        );
    }
    drop(db);
    let owned = OwnedStateRoot::open(&base.join("owned"), std::slice::from_ref(&root)).unwrap();
    let mut store = TransactionStore::open(owned).unwrap();
    let entries = store.list().unwrap();
    assert_eq!(entries.len(), 1);
    let id = entries[0].id().clone();
    assert!(matches!(
        entries[0].state(),
        TransactionState::OutcomeUnknown | TransactionState::RecoveryRequired
    ));
    let mut adapter =
        CodexScopedCleanupAdapter::for_owned_test(crate::CodexAdapter::new(ProviderLocation {
            state_root: root.clone(),
            executable: None,
        }));
    adapter.expanded = expanded;
    let committed_before = store
        .records(&id)
        .unwrap()
        .iter()
        .filter(|r| {
            matches!(
                r.event(),
                agentmender_domain::TransactionEvent::CompleteRemovalActionObserved { .. }
            )
        })
        .count();
    if expanded {
        assert_eq!(committed_before, if target_remains { 1 } else { 2 });
    }
    let reconciled = if expanded {
        agentmender_application::lifecycle::reconcile_complete_removal(&mut store, &id, &adapter)
            .unwrap()
    } else {
        agentmender_application::lifecycle::reconcile_cleanup(&mut store, &id, &adapter).unwrap()
    };
    assert!(matches!(
        reconciled,
        TransactionState::OutcomeUnknown | TransactionState::RecoveryRequired
    ));
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &adapter).is_err());
    if expanded {
        assert!(
            agentmender_application::lifecycle::execute_complete_removal(
                &mut store, &id, &adapter, &adapter
            )
            .is_err()
        );
        assert_eq!(
            store
                .records(&id)
                .unwrap()
                .iter()
                .filter(|r| matches!(
                    r.event(),
                    agentmender_domain::TransactionEvent::CompleteRemovalActionObserved { .. }
                ))
                .count(),
            committed_before
        );
    }
    assert!(!payload.exists());
    assert!(other.exists());
}

#[test]
fn process_termination_after_unlink_preserves_other_session() {
    process_crash_case(
        "scoped_cleanup::acceptance::child_terminate_after_unlink",
        86,
        true,
        false,
    );
}
#[test]
fn process_termination_after_commit_preserves_other_session() {
    process_crash_case(
        "scoped_cleanup::acceptance::child_terminate_after_commit",
        87,
        false,
        false,
    );
}

#[test]
fn expanded_process_termination_after_unlink_preserves_commit_prefix() {
    process_crash_case(
        "scoped_cleanup::acceptance::expanded_child_terminate_after_unlink",
        86,
        true,
        true,
    );
}
#[test]
fn expanded_process_termination_after_commit_preserves_commit_prefix() {
    process_crash_case(
        "scoped_cleanup::acceptance::expanded_child_terminate_after_commit",
        87,
        false,
        true,
    );
}

fn conversation_acceptance(
    base: &std::path::Path,
    root: &std::path::Path,
    adapter: CodexScopedCleanupAdapter,
    target: &str,
    management: &str,
    payload: &std::path::Path,
) {
    use agentmender_application::{
        QueryControl,
        conversation::{
            ConversationRequest as Request, ConversationResponse as Response, ConversationService,
        },
    };
    use agentmender_provider_api::SessionMetadataReadPort;
    let adapter = adapter.with_protected_sessions(vec![management.into()]);
    let protected = vec![management.to_owned()];
    let service = ConversationService {
        metadata: &adapter,
        cleanup: &adapter,
        protected_sessions: &protected,
        provider_root: root,
    };
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    let mut sentinels = Vec::new();
    for (suffix, title, archived, version) in [
        ("ab0", "MISSION1 README", 0, "0.166.0"),
        ("ab1", "MISSION1 README notes", 1, "0.166.0"),
        ("ab2", "", 0, "0.166.0"),
        ("ab3", "MISSION1 README", 0, "unknown"),
    ] {
        let id = format!("12345678-1234-1234-1234-123456789{suffix}");
        let path = root.join(format!("sessions/rollout-2026-01-01T00-00-03-{id}.jsonl"));
        let bytes = format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\"}}}}\nunrelated transcript secret\n"
        );
        fs::write(&path, &bytes).unwrap();
        db.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,cli_version,archived,preview,first_user_message) VALUES (?1,?2,1,2,'cli','openai',?3,?4,'read-only','never',?5,?6,'PRIVATE PREVIEW','PRIVATE MESSAGE')", (&id,path.to_str().unwrap(),base.to_str().unwrap(),title,version,archived)).unwrap();
        sentinels.push((id, path, bytes));
    }
    db.execute(
        "UPDATE threads SET title='MISSION1 README' WHERE id=?1",
        [target],
    )
    .unwrap();
    let mut persist = 1;
    db.file_control(
        None::<&str>,
        rusqlite::FileControl::PersistWal(&mut persist),
    )
    .unwrap();
    drop(db);
    let search = service
        .dispatch(
            Request::Search {
                query: "MISSION1 README".into(),
                include_archived: true,
            },
            None,
            QueryControl::default(),
        )
        .unwrap();
    let Response::Search { sessions, .. } = search else {
        panic!("search")
    };
    assert_eq!(sessions.len(), 4);
    assert!(
        sessions
            .iter()
            .any(|s| s.metadata.state == agentmender_domain::SessionState::Archived)
    );
    assert!(
        sessions
            .iter()
            .any(|s| !s.metadata.producer_version_matches && !s.can_request_preview)
    );
    let metadata = adapter.session_metadata().unwrap();
    let encoded = serde_json::to_string(&metadata).unwrap();
    assert!(
        !encoded.contains("PRIVATE PREVIEW")
            && !encoded.contains("PRIVATE MESSAGE")
            && !encoded.contains("unrelated transcript secret")
    );
    assert!(metadata.sessions.iter().any(|s| s.title.is_none()));
    let select = |id: &str| {
        let identity = metadata
            .sessions
            .iter()
            .find(|s| s.native_session_id == id)
            .unwrap()
            .identity
            .clone();
        std::sync::Arc::new(move |candidate: &agentmender_domain::ArtifactId| {
            *candidate == identity
        }) as agentmender_application::TargetSelector
    };
    assert!(
        service
            .dispatch(
                Request::Preview {
                    target: select(management)
                },
                None,
                QueryControl::default()
            )
            .is_err()
    );
    assert!(
        service
            .dispatch(
                Request::Preview {
                    target: select(&sentinels[3].0)
                },
                None,
                QueryControl::default()
            )
            .is_err()
    );
    let Response::Preview { inputs, .. } = service
        .dispatch(
            Request::Preview {
                target: select(target),
            },
            None,
            QueryControl::default(),
        )
        .unwrap()
    else {
        panic!("preview")
    };
    assert!(
        inputs
            .plan
            .content()
            .artifacts
            .iter()
            .any(|a| a.explicitly_protected && a.id.local_id.as_str().ends_with(management))
    );
    let owned_path = base.join("conversation-owned");
    let owned = OwnedStateRoot::create(&owned_path, &[root.into()]).unwrap();
    let digest = inputs.plan.digest().to_string();
    assert!(
        service
            .dispatch(
                Request::Prepare {
                    target: select(target),
                    expected_digest: "0".repeat(64)
                },
                Some(owned.clone()),
                QueryControl::default()
            )
            .is_err()
    );
    assert!(!owned_path.join("authority.key").exists());
    let prepared = service
        .dispatch(
            Request::Prepare {
                target: select(target),
                expected_digest: digest.clone(),
            },
            Some(owned.clone()),
            QueryControl::default(),
        )
        .unwrap();
    let Response::Transactions(statuses) = prepared else {
        panic!("prepare")
    };
    let id: agentmender_domain::TransactionId = statuses[0].transaction_id.parse().unwrap();
    assert!(!statuses[0].primary_verified);
    let Response::Transactions(retried) = service
        .dispatch(
            Request::Prepare {
                target: select(target),
                expected_digest: digest,
            },
            Some(owned.clone()),
            QueryControl::default(),
        )
        .unwrap()
    else {
        panic!("retry")
    };
    assert_eq!(retried[0].transaction_id, id.to_string());
    service
        .dispatch(
            Request::RequestExecution {
                transaction_id: id.to_string(),
            },
            Some(owned.clone()),
            QueryControl::default(),
        )
        .unwrap();
    let mut store = TransactionStore::open(owned.clone()).unwrap();
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &adapter).is_err());
    assert!(payload.exists());
    // Fixture authority is not actual independent human acceptance.
    store
        .grant_approval(
            &id,
            2,
            &inputs.plan,
            &inputs.graph,
            &inputs.integrity,
            &inputs.context,
        )
        .unwrap();
    let mut now_protected = protected.clone();
    now_protected.push(target.into());
    let protected_adapter =
        CodexScopedCleanupAdapter::for_owned_test(CodexAdapter::new(ProviderLocation {
            state_root: root.into(),
            executable: None,
        }))
        .with_protected_sessions(now_protected);
    assert!(
        execute_scoped_cleanup(&mut store, &id, &protected_adapter, &protected_adapter).is_err()
    );
    assert_eq!(
        store.inspect(&id).unwrap().state(),
        TransactionState::Prepared
    );
    assert!(payload.exists());
    assert_eq!(
        execute_scoped_cleanup(&mut store, &id, &adapter, &adapter).unwrap(),
        TransactionState::Succeeded
    );
    assert!(!payload.exists());
    assert!(execute_scoped_cleanup(&mut store, &id, &adapter, &adapter).is_err());
    drop(store);
    let Response::Transactions(statuses) = service
        .dispatch(
            Request::Status {
                transaction_id: Some(id.to_string()),
            },
            Some(owned),
            QueryControl::default(),
        )
        .unwrap()
    else {
        panic!("status")
    };
    assert!(statuses[0].primary_verified && statuses[0].other_sessions_preserved);
    assert!(!statuses[0].additional_artifacts_fully_cleaned && !statuses[0].retry_permitted);
    for (_, path, bytes) in sentinels {
        assert_eq!(fs::read_to_string(path).unwrap(), bytes);
    }
    assert!(
        root.join(format!(
            "sessions/rollout-2026-01-01T00-00-01-{management}.jsonl"
        ))
        .exists()
    );
}
