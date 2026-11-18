use agentmender_domain::SessionKind;
use agentmender_provider_claude::{ClaudeAdapter, DesktopContext, RootSource};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const MAIN: &str = "eb000000-1111-4111-8111-000000000001";
const FORK: &str = "eb000000-1111-4111-8111-000000000002";
const PROFILE: &str = "eb000000-1111-4111-8111-000000000003";
const ACCOUNT: &str = "eb000000-1111-4111-8111-000000000004";

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().canonicalize().unwrap();
    (fixture, path)
}

fn adapter(root: &Path) -> ClaudeAdapter {
    ClaudeAdapter::new(
        vec![(root.into(), RootSource::Explicit)],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
}

fn transcript(root: &Path, namespace: &str, id: &str) -> PathBuf {
    let directory = root.join("projects").join(namespace);
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{id}.jsonl"))
}

fn user(id: &str, content: Value) -> Value {
    json!({"type":"user","sessionId":id,"entrypoint":"sdk-cli","version":"2.1.325",
        "cwd":"/fixture/Atlas","timestamp":"2026-11-01T10:00:00Z",
        "message":{"role":"user","content":content}})
}

fn write(mut file: &File, row: &Value) {
    serde_json::to_writer(&mut file, row).unwrap();
    file.write_all(b"\n").unwrap();
}

#[test]
fn late_titles_dates_and_opaque_pages_ignore_sensitive_and_non_conversational_payloads() {
    let (_fixture, root) = fixture();
    let path = transcript(&root, "Città-日本", MAIN);
    let mut file = File::create(&path).unwrap();
    write(
        &file,
        &user(
            MAIN,
            json!("<local-command-stdout>LOCAL CANARY</local-command-stdout>"),
        ),
    );
    write(
        &file,
        &user(
            MAIN,
            json!([{"type":"tool_result","content":"TOOL CANARY"}]),
        ),
    );
    write(
        &file,
        &user(MAIN, json!("Authorization: bearer SECRET CANARY")),
    );
    write(&file, &user(MAIN, json!("First visible request")));
    for index in 0..100 {
        write(
            &file,
            &user(MAIN, json!(format!("{index:03} {}", "*".repeat(1000)))),
        );
    }
    write(
        &file,
        &json!({"type":"ai-title","sessionId":MAIN,"aiTitle":"Generated title"}),
    );
    write(
        &file,
        &json!({"type":"custom-title","sessionId":MAIN,"customTitle":"Late custom title"}),
    );
    write(
        &file,
        &json!({"type":"summary","summary":"Legacy summary after rename"}),
    );
    write(
        &file,
        &json!({"type":"assistant","sessionId":MAIN,"timestamp":"2026-11-17T20:04:00Z",
        "message":{"role":"assistant","content":[{"type":"thinking","thinking":"THINKING CANARY"},
            {"type":"text","text":"Final answer"}]}}),
    );
    file.write_all(b"{\"type\":\"unclosed").unwrap();
    drop(file);

    let reader = adapter(&root);
    let catalog = reader.local_catalog(100);
    assert_eq!(catalog.inventory.sessions.len(), 1);
    let session = &catalog.inventory.sessions[0];
    let evidence = &catalog.evidence[&session.identity];
    assert_eq!(session.title.as_deref(), Some("Late custom title"));
    assert_eq!(session.created_at, Some(1793527200));
    assert_eq!(session.last_active_at, Some(1794945840));
    assert!(evidence.modified_at.is_some());
    assert!(!evidence.metadata_complete && catalog.inventory.incomplete);
    let mut cursor = None;
    let mut bodies = Vec::new();
    let mut finished = false;
    for _ in 0..10 {
        let page = reader
            .transcript_page(&session.identity, &evidence.revision, cursor)
            .unwrap();
        assert!(page.messages.len() <= 20);
        bodies.extend(page.messages.into_iter().map(|message| message.text));
        cursor = page.next;
        if cursor.is_none() {
            assert!(!page.complete);
            finished = true;
            break;
        }
    }
    assert!(finished);
    assert_eq!(bodies.first().unwrap(), "First visible request");
    assert_eq!(bodies.last().unwrap(), "Final answer");
    assert!(!bodies.iter().any(|text| text.contains("CANARY")));
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    assert!(
        reader
            .transcript_page(&session.identity, &evidence.revision, None)
            .is_err()
    );
}

#[test]
fn sparse_oversized_line_retains_a_bounded_tail_and_resumable_preview() {
    let (_fixture, root) = fixture();
    let path = transcript(&root, "atlas", MAIN);
    let mut file = File::create(path).unwrap();
    write(&file, &user(MAIN, json!("Initial request")));
    file.seek(SeekFrom::Start(26 * 1024 * 1024)).unwrap();
    file.write_all(b"\n").unwrap();
    write(
        &file,
        &json!({"type":"custom-title","sessionId":MAIN,"customTitle":"Tail title"}),
    );
    write(&file, &user(MAIN, json!("Tail message")));
    drop(file);
    let reader = adapter(&root);
    let catalog = reader.local_catalog(100);
    let session = &catalog.inventory.sessions[0];
    let evidence = &catalog.evidence[&session.identity];
    assert_eq!(session.title.as_deref(), Some("Tail title"));
    assert!(!evidence.metadata_complete);
    let mut cursor = None;
    let mut bodies = Vec::new();
    for _ in 0..10 {
        let page = reader
            .transcript_page(&session.identity, &evidence.revision, cursor)
            .unwrap();
        bodies.extend(page.messages.into_iter().map(|message| message.text));
        cursor = page.next;
        if cursor.is_none() {
            assert!(!page.complete);
            break;
        }
    }
    assert_eq!(bodies, ["Initial request", "Tail message"]);
}

#[test]
fn corroborated_desktop_fork_and_subagent_stay_outside_main_inventory() {
    let (_fixture, root) = fixture();
    let desktop = root.join("desktop");
    let state = desktop
        .join("claude-code-sessions")
        .join(PROFILE)
        .join(ACCOUNT);
    fs::create_dir_all(&state).unwrap();
    let state_record = |id, parent: Option<String>| {
        let record_id = format!("local_{id}");
        let path = state.join(format!("{record_id}.json"));
        fs::write(
            path,
            json!({"sessionId":record_id,"cliSessionId":id,
            "createdAt":1793527200000_u64,"cwd":"/fixture/Atlas","isArchived":true,"forkedFromSessionId":parent})
            .to_string(),
        )
        .unwrap();
    };
    state_record(MAIN, None);
    state_record(FORK, Some(format!("local_{MAIN}")));
    let parent = user(MAIN, json!("Parent request"));
    let mut fork = user(FORK, json!("Fork request"));
    fork["entrypoint"] = "claude-desktop".into();
    let mut parent = parent;
    parent["entrypoint"] = "claude-desktop".into();
    write(
        &File::create(transcript(&root, "atlas", MAIN)).unwrap(),
        &parent,
    );
    let fork_file = File::create(transcript(&root, "atlas", FORK)).unwrap();
    write(&fork_file, &parent);
    write(&fork_file, &fork);
    let agents = root.join("projects/atlas").join(MAIN).join("subagents");
    fs::create_dir_all(&agents).unwrap();
    write(
        &File::create(agents.join("agent-a1.jsonl")).unwrap(),
        &json!({
            "type":"user","sessionId":MAIN,"agentId":"a1","isSidechain":true,
            "entrypoint":"claude-desktop","message":{"content":"Worker request"}
        }),
    );
    let reader = ClaudeAdapter::new(
        vec![(root.clone(), RootSource::Explicit)],
        vec![DesktopContext {
            data_root: desktop,
            app_bundles: vec![],
        }],
        vec![],
        vec![],
    )
    .unwrap();
    let catalog = reader.local_catalog(100);
    let main: Vec<_> = catalog
        .inventory
        .sessions
        .iter()
        .filter(|row| row.is_main())
        .collect();
    assert_eq!(main.len(), 1);
    assert_eq!(main[0].identity.native_id, MAIN);
    assert_eq!(catalog.evidence[&main[0].identity].subagents, 1);
    assert_eq!(catalog.evidence[&main[0].identity].forks, 1);
    assert!(
        catalog
            .inventory
            .sessions
            .iter()
            .any(|session| session.kind == SessionKind::Fork)
    );
}

#[test]
fn duplicate_uuid_uses_scoped_namespace_and_root_and_candidates_cannot_grant_removal() {
    let (_fixture, base) = fixture();
    let root = base.join("primary");
    let alternate = base.join("additional");
    for (archive, namespace) in [(&root, "a"), (&root, "b"), (&alternate, "a")] {
        write(
            &File::create(transcript(archive, namespace, MAIN)).unwrap(),
            &user(MAIN, json!(namespace)),
        );
    }
    let selected = root.join("projects/a");
    fs::write(
        selected.join(format!("{MAIN}.precompact.json")),
        "metadata candidate",
    )
    .unwrap();
    fs::create_dir(selected.join(format!("{MAIN}.dir-sync.json"))).unwrap();
    fs::write(
        selected.join(format!("{MAIN}.fake.cast")),
        "preserve unknown",
    )
    .unwrap();
    let history = root.join("history.jsonl");
    let original =
        format!("{{\"sessionId\":\"{MAIN}\"}}\r\nmalformed\r\n{{\"sessionId\":\"{FORK}\"}}\n");
    fs::write(&history, &original).unwrap();
    let reader = ClaudeAdapter::new(
        vec![
            (root.clone(), RootSource::Explicit),
            (alternate, RootSource::Additional),
        ],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    let catalog = reader.local_catalog(100);
    assert_eq!(catalog.inventory.sessions.len(), 3);
    assert_eq!(catalog.inventory.projects.len(), 3);
    let row = catalog
        .inventory
        .sessions
        .iter()
        .find(|session| {
            session.identity.state_root == root
                && session.identity.namespace.as_deref() == Some("a")
        })
        .unwrap();
    let evidence = &catalog.evidence[&row.identity];
    assert_eq!(evidence.same_uuid_elsewhere, 2);
    let storage = reader.storage_detail(&row.identity, evidence).unwrap();
    assert_eq!(storage.sidecar_candidates, 2);
    assert_eq!(storage.history_matches, Some(1));
    assert_eq!(storage.history_observation, "partial_uuid_only");
    assert!(!storage.standard_available && !storage.deep_available);
    assert_eq!(fs::read_to_string(history).unwrap(), original);
}

#[cfg(any(unix, windows))]
#[test]
fn links_and_hardlinks_are_never_admitted_as_transcripts() {
    let (_fixture, root) = fixture();
    let real = transcript(&root, "original", MAIN);
    write(
        &File::create(&real).unwrap(),
        &user(MAIN, json!("Keep content")),
    );
    let alias = transcript(&root, "aliased", MAIN);
    let reader = adapter(&root);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        assert_eq!(reader.local_catalog(100).inventory.sessions.len(), 1);
        fs::remove_file(&alias).unwrap();
    }
    fs::hard_link(&real, &alias).unwrap();
    let observation = reader.local_catalog(100).inventory;
    assert!(observation.sessions.is_empty());
    assert!(observation.incomplete);
}
