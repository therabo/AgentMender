//! Actual stdio + protected worker + approved provider effects on disposable state.
//! The test grant is fixture authority, never independent human/client acceptance.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use agentmender_domain::*;
use agentmender_persistence::{OwnedStateRoot, TransactionStore};
use agentmender_provider_api::ProviderLocation;
use agentmender_provider_codex::{CodexAdapter, CodexScopedCleanupAdapter};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
const T: &str = "12345678-1234-1234-1234-123456789abc";
const M: &str = "12345678-1234-1234-1234-123456789abd";
const U: &str = "12345678-1234-1234-1234-123456789ab0";
struct Client {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}
impl Client {
    fn start(root: &Path, owned: &Path, binary: &Path, home: &Path) -> Self {
        Self::connect(
            base(root, binary, home)
                .args([
                    "--protect-codex-session",
                    M,
                    "mcp",
                    "--session-removal",
                    "--state-root",
                ])
                .arg(owned),
        )
    }
    fn connect(command: &mut Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self {
            child,
            input,
            output,
        };
        let init = client.exchange(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"disposable-protocol-fixture","version":"1"}}}));
        assert!(init.get("result").is_some(), "{init}");
        writeln!(
            client.input.as_mut().unwrap(),
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        client
    }
    fn exchange(&mut self, request: Value) -> Value {
        writeln!(self.input.as_mut().unwrap(), "{request}").unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        assert!(line.len() <= 256 * 1024);
        serde_json::from_str(&line).unwrap()
    }
    fn call(&mut self, id: u64, name: &str, mut args: Value) -> Value {
        if name == "session_search" {
            args.as_object_mut()
                .unwrap()
                .entry("purpose")
                .or_insert(json!("diagnostic"));
        }
        self.exchange(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}}))
    }
    fn close(mut self) {
        self.input.take();
        assert!(self.child.wait().unwrap().success());
    }
}

#[test]
fn adaptive_metadata_search_needs_no_supported_binary_or_approval_root() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap();
    let root = path.join("provider");
    let project = path.join("MISSION1");
    fs::create_dir(&project).unwrap();
    seed(&root, &project);
    let fake = root.join("not-an-executable");
    fs::write(&fake, b"unsupported provider image; no mutation authority").unwrap();
    let before = tree_bytes(&root);
    let home = PathBuf::from(std::env::var_os("HOME").unwrap())
        .canonicalize()
        .unwrap();
    let mut client = Client::connect(base(&root, &fake, &home).args(["mcp", "--session-search"]));
    let search = client.call(1, "session_search", json!({"query":"MISSION1 README"}));
    let view = &search["result"]["structuredContent"];
    let sessions = view["sessions"].as_array().expect("adaptive search");
    assert_eq!(sessions.len(), 2, "{search}");
    assert!(
        sessions
            .iter()
            .all(|s| s["exact_profile"] == "no_mutation_profile_selected")
    );
    assert!(view["read_compatibility"]["provider_version"].is_null());
    assert_eq!(
        view["read_compatibility"]["authenticated_caller_binding"],
        false
    );
    let target = sessions[0]["target"].clone();
    let preview = client.call(2, "preview_removal", json!({"target":target}));
    let view = &preview["result"]["structuredContent"];
    assert_eq!(view["status"], "blocked");
    assert_eq!(view["execution_permitted"], false);
    assert!(view.get("review").is_none());
    let prepared = client.call(
        3,
        "prepare_removal",
        json!({"target":target,"expected_plan_digest":format!("sha256:{}","0".repeat(64))}),
    );
    assert_eq!(
        prepared["result"]["structuredContent"]["error"],
        "read_only_removal_unavailable"
    );
    assert_eq!(
        client.call(4, "removal_status", json!({}))["result"]["structuredContent"]["error"],
        "read_only_removal_unavailable"
    );
    client.close();
    let mut registered = Client::connect(base(&root, &fake, &home).args([
        "--protect-codex-session",
        M,
        "mcp",
        "--session-search",
    ]));
    for query in [M, "Management"] {
        let found = registered.call(1, "session_search", json!({"query":query}));
        assert_eq!(found["result"]["structuredContent"]["sessions"], json!([]));
    }
    registered.close();
    assert_eq!(tree_bytes(&root), before);
    assert!(!path.join("owned").exists());
}
#[test]
fn content_search_stdio_is_grounded_and_management_requires_independent_binding() {
    use std::os::unix::fs::MetadataExt;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap();
    let root = path.join("provider");
    seed(&root, &path.join("MISSION1"));
    let fake = root.join("not-an-executable");
    fs::write(&fake, b"no executable mutation profile").unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").unwrap())
        .canonicalize()
        .unwrap();
    let registration = path.join("untrusted-registration.json");
    fs::write(&registration, b"{}").unwrap();
    let before = tree_bytes(&root);
    let mut client = Client::connect(
        base(&root, &fake, &home)
            .args([
                "--protect-codex-session",
                M,
                "mcp",
                "--session-search",
                "--management-registration",
            ])
            .arg(&registration),
    );
    let args = json!({"query":"Earth rendering Cesium camera","include_content":true,"purpose":"diagnostic","page_size":20});
    let found = client.call(1, "session_search", args);
    let view = &found["result"]["structuredContent"];
    assert_eq!(view["result_count"], 1, "{found}");
    assert_eq!(view["coverage"]["eligible_observed"], 2);
    assert_eq!(view["management_exclusion"], "not_guaranteed_diagnostic");
    let hit = &view["sessions"][0];
    assert_eq!(hit["native_session_id"], T);
    assert_eq!(hit["title"], "MISSION1 README");
    assert!(
        hit["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["untrusted_data"] == true
                && e["excerpt"].as_str().unwrap().chars().count() <= 320)
    );
    assert!(!found.to_string().contains("PRIVATE"));
    let target = hit["target"].clone();
    let evidence = hit["evidence"][0]["evidence_ref"].clone();
    let context = client.call(
        2,
        "session_context",
        json!({"target":target,"evidence_ref":evidence}),
    );
    assert!(
        context["result"]["structuredContent"]["evidence"]["excerpt"].is_string(),
        "{context}"
    );
    assert!(
        serde_json::to_vec(&context["result"]["structuredContent"])
            .unwrap()
            .len()
            <= 2048
    );
    let window = client.call(
        20,
        "session_context",
        json!({
            "target":target,"evidence_ref":evidence,
            "window":{"before":1,"after":2,"max_messages":10,"max_bytes":4096}
        }),
    );
    let w = &window["result"]["structuredContent"];
    assert!(
        w["messages"].as_array().is_some_and(|m| !m.is_empty()),
        "{window}"
    );
    assert!(w["messages"].as_array().unwrap().iter().all(|e| e["untrusted_data"] == true && !e["evidence_ref"].as_str().unwrap().is_empty()));
    assert!(!window.to_string().contains("PRIVATE"));
    let occurrence = client.call(
        21,
        "session_context",
        json!({
            "target":target,"evidence_ref":evidence,
            "window":{"direction":"occurrences","query":"tuning","max_messages":10}
        }),
    );
    assert!(
        occurrence["result"]["structuredContent"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["excerpt"].as_str().unwrap().contains("tuning")),
        "{occurrence}"
    );
    let temporal = client.call(
        22,
        "session_search",
        json!({"page_size":1,"structured":{"filter":{"op":"created","range":{"start":0,"end":2}}}}),
    );
    let tv = &temporal["result"]["structuredContent"];
    assert_eq!(tv["result_count"], 1, "{temporal}");
    assert!(tv["query_interpretation"].is_object());
    assert!(tv["next_cursor"].is_string(), "{temporal}");
    let next = client.call(23,"session_search",json!({"page_size":1,"structured":{"filter":{"op":"created","range":{"start":0,"end":2}}},"cursor":tv["next_cursor"]}));
    assert_eq!(
        next["result"]["structuredContent"]["result_count"], 1,
        "{next}"
    );
    assert_ne!(
        next["result"]["structuredContent"]["sessions"][0]["native_session_id"],
        tv["sessions"][0]["native_session_id"]
    );
    let negative = client.call(
        24,
        "session_search",
        json!({"structured":{"filter":{"op":"not","arg":{"op":"project","values":["MISSION1"]}}}}),
    );
    assert_eq!(
        negative["result"]["structuredContent"]["result_count"], 0,
        "{negative}"
    );
    let unknown = client.call(25,"session_search",json!({"structured":{"filter":{"op":"not","arg":{"op":"project","values":["MISSION1"],"source":"verified_label"}}}}));
    assert_eq!(
        unknown["result"]["structuredContent"]["result_count"], 0,
        "{unknown}"
    );
    assert_eq!(
        unknown["result"]["structuredContent"]["coverage"]["unresolved_sessions"], 2,
        "{unknown}"
    );
    let unresolved = unknown["result"]["structuredContent"]["unresolved_candidates"]
        .as_array()
        .unwrap();
    assert_eq!(unresolved.len(), 2);
    assert!(
        unresolved
            .iter()
            .all(|candidate| candidate["hard_filters_verified"] == false
                && candidate["can_request_preview"] == false)
    );
    let legacy = client.call(3, "session_search", json!({"query":"README"}));
    let other = legacy["result"]["structuredContent"]["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["native_session_id"] == U)
        .unwrap()["target"]
        .clone();
    assert_eq!(
        client.call(
            4,
            "session_context",
            json!({"target":other,"evidence_ref":evidence})
        )["result"]["isError"],
        true
    );
    let forged = format!("{}0", evidence.as_str().unwrap());
    assert!(
        client
            .call(
                5,
                "session_context",
                json!({"target":target,"evidence_ref":forged})
            )
            .get("error")
            .is_some()
    );
    let denied = client.call(
        6,
        "session_search",
        json!({"query":"","purpose":"management"}),
    );
    assert_eq!(
        denied["result"]["structuredContent"]["error"],
        "caller_identity_required"
    );
    assert_eq!(denied["result"]["structuredContent"]["sessions"], json!([]));
    // Even a well-formed file using the observed instance challenge cannot be
    // registered by this same-user model-controlled fixture process.
    let m = fs::metadata(&root).unwrap();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 300;
    fs::write(&registration,json!({"schema":1,"server_instance":denied["result"]["structuredContent"]["registration_instance"],"provider":"codex","state_root":root,"root_device":m.dev(),"root_inode":m.ino(),"native_session_id":M,"expires_at":expires,"connection_scope":"single_conversation_operator_verified"}).to_string()).unwrap();
    assert_eq!(
        client.call(
            7,
            "session_search",
            json!({"query":"Earth","purpose":"management","include_content":true})
        )["result"]["structuredContent"]["error"],
        "caller_identity_required"
    );
    assert!(
        client
            .call(
                8,
                "session_search",
                json!({"purpose":"management","caller_session_id":M})
            )
            .get("error")
            .is_some()
    );
    client.close();
    assert_eq!(before, tree_bytes(&root));
    assert!(!path.join("owned").exists());
}

#[test]
fn default_management_and_forged_request_metadata_fail_closed_across_connections() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap();
    let root = path.join("provider");
    let project = path.join("MISSION1");
    fs::create_dir(&project).unwrap();
    seed(&root, &project);
    let fake = root.join("not-an-executable");
    fs::write(&fake, b"no provider startup").unwrap();
    let before = tree_bytes(&root);
    let home = PathBuf::from(std::env::var_os("HOME").unwrap())
        .canonicalize()
        .unwrap();
    let mut first = Client::connect(base(&root, &fake, &home).args(["mcp", "--session-search"]));
    let mut second = Client::connect(base(&root, &fake, &home).args(["mcp", "--session-search"]));
    // One process also receives switches/forged UUIDs on consecutive requests.
    for (index, caller) in [M, T, "forged", M].into_iter().enumerate() {
        for client in [&mut first, &mut second] {
            let result = client.exchange(json!({"jsonrpc":"2.0","id":index+1,"method":"tools/call","params":{"name":"session_search","_meta":{"threadId":caller,"sessionId":T},"arguments":{"query":caller,"include_content":true}}}));
            let value = &result["result"]["structuredContent"];
            assert_eq!(value["error"], "caller_identity_required", "{result}");
            assert_eq!(value["result_count"], 0);
            assert_eq!(value["sessions"], json!([]));
            assert_eq!(
                value["caller_identity_evidence"]["thread_id_metadata_present"], true,
                "{result}"
            );
            assert!(!serde_json::to_string(value).unwrap().contains(caller));
            assert_eq!(
                value["caller_identity_evidence"]["authenticated_host_route"],
                false
            );
        }
    }
    first.close();
    second.close();
    let mut reconnected =
        Client::connect(base(&root, &fake, &home).args(["mcp", "--session-search"]));
    let result = reconnected.exchange(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"session_search","arguments":{}}}));
    assert_eq!(
        result["result"]["structuredContent"]["error"],
        "caller_identity_required"
    );
    assert_eq!(
        result["result"]["structuredContent"]["caller_identity_evidence"]["thread_id_metadata_present"],
        false
    );
    reconnected.close();
    assert_eq!(before, tree_bytes(&root));
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn base(root: &Path, binary: &Path, home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_agentmender"));
    c.env_clear()
        .env("HOME", home)
        .env("PATH", "")
        .args(["--provider", "codex", "--codex-root"])
        .arg(root)
        .arg("--codex-executable")
        .arg(binary);
    c
}
fn seed(root: &Path, cwd: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fs::create_dir(root).unwrap();
    for name in ["sessions", ".tmp", "thread-writer-locks"] {
        fs::create_dir(root.join(name)).unwrap();
    }
    fs::write(root.join(".tmp/rollout-maintenance.lock"), b"").unwrap();
    fs::write(root.join("thread-writer-locks/.coordination.lock"), b"").unwrap();
    let db = Connection::open(root.join("state_5.sqlite")).unwrap();
    db.pragma_update(None, "journal_mode", "WAL").unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE fixture_sequence_seed(value INTEGER PRIMARY KEY AUTOINCREMENT); DROP TABLE fixture_sequence_seed;").unwrap();
    let schema: Vec<(String, String, String, Option<String>)> = serde_json::from_str(include_str!(
        "../../../compatibility/codex/0.165.0-alpha.1.2/state-schema-5.json"
    ))
    .unwrap();
    for kind in ["table", "index", "trigger", "view"] {
        for (entry, name, _, sql) in &schema {
            if entry == kind
                && name != "sqlite_sequence"
                && let Some(sql) = sql
            {
                db.execute_batch(sql).unwrap();
            }
        }
    }
    let mut files = vec![];
    for (id, title) in [
        (T, "MISSION1 README"),
        (M, "Management"),
        (U, "MISSION1 README"),
    ] {
        let path = root.join(format!("sessions/rollout-2026-01-01T00-00-00-{id}.jsonl"));
        let mut body = format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"cli_version\":\"0.166.0\"}}}}\nPRIVATE TRANSCRIPT\n"
        );
        let texts = if id == U {
            vec![("user", "Discuss lunch plans")]
        } else if id == M {
            vec![
                ("user", "Earth rendering Cesium camera"),
                ("assistant", "Earth rendering Cesium camera"),
                ("user", "Earth rendering Cesium camera"),
            ]
        } else {
            vec![
                ("user", "Earth rendering performance is slow"),
                ("assistant", "Cesium camera behavior needs tuning"),
            ]
        };
        for (role, text) in texts {
            body.push_str(&json!({"type":"response_item","payload":{"type":"message","role":role,"content":[{"type":if role=="assistant"{"output_text"}else{"input_text"},"text":text}]}}).to_string());
            body.push('\n');
        }
        let bytes = body.into_bytes();
        fs::write(&path, &bytes).unwrap();
        db.execute("INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,cli_version,preview,first_user_message) VALUES (?1,?2,1,2,'cli','openai',?3,?4,'read-only','never','0.166.0','PRIVATE PREVIEW','PRIVATE MESSAGE')",(id,path.to_str().unwrap(),cwd.to_str().unwrap(),title)).unwrap();
        files.push((path, bytes));
    }
    let mut persist = 1;
    db.file_control(
        None::<&str>,
        rusqlite::FileControl::PersistWal(&mut persist),
    )
    .unwrap();
    files
}
fn tree_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
fn retained_thread_rows(root: &Path) -> Vec<Vec<rusqlite::types::Value>> {
    let db = Connection::open_with_flags(
        root.join("state_5.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut statement = db
        .prepare("SELECT * FROM threads WHERE id<>?1 ORDER BY id")
        .unwrap();
    let columns = statement.column_count();
    statement
        .query_map([T], |row| (0..columns).map(|i| row.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
fn assert_primary_cut(root: &Path, retained: &[Vec<rusqlite::types::Value>]) {
    let db = Connection::open_with_flags(
        root.join("state_5.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let target_rows: i64 = db
        .query_row("SELECT count(*) FROM threads WHERE id=?1", [T], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(target_rows, 0);
    drop(db);
    assert_eq!(retained_thread_rows(root), retained);
}
#[test]
#[ignore = "explicit pinned-binary disposable MCP integration; fixture authority, no human approval claim"]
fn exact_profile_stdio_prepare_external_execution_restart_and_status() {
    let binary = PathBuf::from(
        std::env::var_os("AGENTMENDER_CODEX166_BINARY").expect("exact binary required"),
    )
    .canonicalize()
    .unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").unwrap())
        .canonicalize()
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let base_path = temp.path().canonicalize().unwrap();
    let root = base_path.join("provider");
    let other_root = base_path.join("other-provider");
    let owned_path = base_path.join("owned");
    let files = seed(&root, &base_path);
    let foreign_files = seed(&other_root, &base_path);
    let foreign_before = tree_bytes(&other_root);
    let retained_before = retained_thread_rows(&root);
    let mut client = Client::start(&root, &owned_path, &binary, &home);
    let tools = client.exchange(json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}));
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 14);
    let found = client.call(2, "session_search", json!({"query":"MISSION1 README"}));
    let sessions = found["result"]["structuredContent"]["sessions"]
        .as_array()
        .expect("search");
    assert_eq!(sessions.len(), 2, "{found}");
    assert!(!found.to_string().contains("PRIVATE"));
    let target = sessions
        .iter()
        .find(|s| s["native_session_id"] == T)
        .unwrap()["target"]
        .clone();
    let old_unrelated = sessions
        .iter()
        .find(|s| s["native_session_id"] == U)
        .unwrap()["target"]
        .clone();
    let all = client.call(3, "session_search", json!({"query":"Management"}));
    assert_eq!(all["result"]["structuredContent"]["sessions"], json!([]));
    let preview = client.call(5, "preview_removal", json!({"target":target}));
    // The retained-plan reference cannot bypass protection even though search
    // no longer presents the management session as a candidate.
    let management =
        preview["result"]["structuredContent"]["review"]["protected_sessions"][0].clone();
    assert_eq!(
        client.call(4, "preview_removal", json!({"target":management}))["result"]["isError"],
        true
    );
    let digest = preview["result"]["structuredContent"]["review"]["plan_digest"]
        .as_str()
        .expect("preview");
    let prepared = client.call(
        6,
        "prepare_removal",
        json!({"target":target,"expected_plan_digest":digest}),
    );
    assert!(
        prepared["result"]["structuredContent"]["transactions"].is_array(),
        "preview={preview}; prepare={prepared}"
    );
    let transaction = prepared["result"]["structuredContent"]["transactions"][0]["transaction_id"]
        .as_str()
        .expect("prepare")
        .to_owned();
    let retried = client.call(
        7,
        "prepare_removal",
        json!({"target":target,"expected_plan_digest":digest}),
    );
    assert_eq!(
        retried["result"]["structuredContent"]["transactions"][0]["transaction_id"],
        transaction
    );
    let handoff = client.call(
        8,
        "request_removal_execution",
        json!({"transaction_id":transaction}),
    );
    assert_eq!(
        handoff["result"]["structuredContent"]["transactions"][0]["execution_channel"],
        "trusted_external_terminal"
    );
    assert_eq!(
        handoff["result"]["structuredContent"]["transactions"][0]["approval_status"],
        "None"
    );
    client.close();
    assert!(files[0].0.exists());
    let adapter = CodexScopedCleanupAdapter::new(
        CodexAdapter::new(ProviderLocation {
            state_root: root.clone(),
            executable: Some(binary.clone()),
        })
        .with_evidence_helper(
            PathBuf::from(env!("CARGO_BIN_EXE_agentmender")),
            home.clone(),
        ),
    )
    .with_protected_sessions(vec![M.into()]);
    let inputs =
        agentmender_application::cleanup_inputs(&adapter, T, &DiscoveryScope::default()).unwrap();
    assert_eq!(inputs.plan.digest().to_string(), digest);
    let owned = OwnedStateRoot::open(&owned_path, std::slice::from_ref(&root)).unwrap();
    let id: TransactionId = transaction.parse().unwrap();
    let mut store = TransactionStore::open(owned).unwrap();
    // A test fixture supplies authority here. A model must never perform this
    // step in an actual human acceptance run.
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
    drop(store);
    let executed = base(&root, &binary, &home)
        .arg("--json")
        .args(["transactions", "--state-root"])
        .arg(&owned_path)
        .args(["execute", &transaction])
        .output()
        .unwrap();
    assert!(
        executed.status.success(),
        "{} {}",
        String::from_utf8_lossy(&executed.stdout),
        String::from_utf8_lossy(&executed.stderr)
    );
    let result: Value = serde_json::from_slice(&executed.stdout).unwrap();
    assert_eq!(result["report"]["primary_verified"], true);
    assert!(!files[0].0.exists());
    // Independent inspection, not an assertion based only on the adapter report.
    assert_primary_cut(&root, &retained_before);
    assert_eq!(tree_bytes(&other_root), foreign_before);
    for (path, bytes) in files.iter().skip(1).chain(&foreign_files) {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    let mut reopened = Client::start(&root, &owned_path, &binary, &home);
    let status = reopened.call(9, "removal_status", json!({"transaction_id":transaction}));
    let result = &status["result"]["structuredContent"]["transactions"][0];
    assert_eq!(result["primary_verified"], true);
    assert_eq!(result["other_sessions_preserved"], true);
    assert_eq!(result["approval_status"], "Consumed");
    assert_eq!(result["retry_permitted"], false);
    assert_eq!(
        reopened.call(10, "preview_removal", json!({"target":old_unrelated}))["result"]["isError"],
        true
    );
    reopened.call(
        11,
        "request_removal_execution",
        json!({"transaction_id":transaction}),
    );
    reopened.close();
    let repeated = base(&root, &binary, &home)
        .args(["transactions", "--state-root"])
        .arg(&owned_path)
        .args(["execute", &transaction])
        .output()
        .unwrap();
    assert!(!repeated.status.success());
    assert_primary_cut(&root, &retained_before);
    assert_eq!(tree_bytes(&other_root), foreign_before);
    for (path, bytes) in files.iter().skip(1).chain(&foreign_files) {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
}
