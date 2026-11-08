use agentmender_application::{ApplicationError, ProviderReport, ReadOnlyResponse};
use agentmender_mcp_server::{Executor, serve_io};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

async fn exchange(
    reader: &mut BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    writer: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>,
    request: Value,
) -> Value {
    writer
        .write_all(format!("{request}\n").as_bytes())
        .await
        .unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    assert!(line.len() <= 256 * 1024, "response exceeds wire limit");
    let response: Value = serde_json::from_str(&line).unwrap();
    if let Some(structured) = response
        .get("result")
        .and_then(|r| r.get("structuredContent"))
    {
        let text: Value =
            serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(
            &text, structured,
            "text fallback must preserve the typed result"
        );
    }
    response
}
fn executor(calls: Arc<AtomicUsize>) -> Executor {
    Arc::new(move |_, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(ReadOnlyResponse::Providers(ProviderReport::default()))
    })
}
#[tokio::test]
async fn modern_registry_and_strict_validation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (client, server) = tokio::io::duplex(128 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let task = tokio::spawn(serve_io(sr, sw, executor(calls.clone()), [1; 32]));
    let (cr, mut cw) = tokio::io::split(client);
    let mut cr = BufReader::new(cr);
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    let discover = exchange(
        &mut cr,
        &mut cw,
        json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":meta}}),
    )
    .await;
    assert_eq!(
        discover["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], "agentmender",
        "{discover}"
    );
    let list = exchange(
        &mut cr,
        &mut cw,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta}}),
    )
    .await;
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 8);
    assert_eq!(
        tools
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "providers",
            "workspace_scan",
            "inspect",
            "graph",
            "explain",
            "verify_integrity",
            "plan_lifecycle",
            "repair_plan"
        ]
    );
    assert!(
        serde_json::to_vec(&list).unwrap().len() + 64 * 1024 + 128 <= 256 * 1024,
        "tool schemas must fit even with the largest admitted request ID"
    );
    for tool in tools {
        assert!(tool["inputSchema"].is_object());
        assert!(tool["outputSchema"].is_object());
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
    }
    let bad = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"_meta":meta,"name":"workspace_scan","arguments":{"scope":{"max_entries":10001},"private_canary":"secret"}}})).await;
    assert_eq!(bad["error"]["code"], -32602);
    assert!(!bad.to_string().contains("private_canary"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let bad_cursor = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"_meta":meta,"name":"graph","arguments":{"page":{"cursor":"invalid"}}}})).await;
    assert_eq!(bad_cursor["error"]["code"], -32602);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for (name, arguments) in [
        ("unknown_tool", json!({})),
        ("graph", json!({"provider":"other"})),
        ("graph", json!({"scope":{"max_entries":"10"}})),
        ("graph", json!({"scope":{"project_path":"/secret"}})),
        (
            "inspect",
            json!({"target":{"provider":"codex","class":"session","token":"a".repeat(65)}}),
        ),
        (
            "inspect",
            json!({"provider":"claude","target":{"provider":"codex","class":"session","token":"a".repeat(64)}}),
        ),
        (
            "plan_lifecycle",
            json!({"target":{"provider":"codex","class":"session","token":"a".repeat(64)},"intent":"repair"}),
        ),
    ] {
        let bad = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"_meta":meta,"name":name,"arguments":arguments}})).await;
        assert!(bad.get("error").is_some(), "{bad}");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    let good = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"_meta":meta,"name":"providers","arguments":{}}})).await;
    assert!(good["result"]["structuredContent"].is_object(), "{good}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(cw);
    drop(cr);
    task.await.unwrap().unwrap();
}
#[tokio::test]
async fn legacy_initialize_and_sanitized_errors() {
    let (client, server) = tokio::io::duplex(128 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let exec: Executor = Arc::new(|_, _| Err(ApplicationError::Cancelled));
    let task = tokio::spawn(serve_io(sr, sw, exec, [2; 32]));
    let (cr, mut cw) = tokio::io::split(client);
    let mut cr = BufReader::new(cr);
    let init = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "agentmender");
    cw.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    let list = exchange(
        &mut cr,
        &mut cw,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    )
    .await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 8);
    let result = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"providers","arguments":{}}})).await;
    assert_eq!(result["result"]["structuredContent"]["error"], "cancelled");
    drop(cw);
    drop(cr);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn busy_cancellation_and_shutdown_wait_for_worker() {
    use std::sync::atomic::AtomicBool;
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let (s, r, c) = (started.clone(), release.clone(), cancelled.clone());
    let exec: Executor = Arc::new(move |_, control| {
        s.store(true, Ordering::SeqCst);
        while !r.load(Ordering::SeqCst) {
            if control.check().is_err() {
                c.store(true, Ordering::SeqCst);
            }
            std::thread::yield_now();
        }
        Ok(ReadOnlyResponse::Providers(ProviderReport::default()))
    });
    let (client, server) = tokio::io::duplex(128 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let task = tokio::spawn(serve_io(sr, sw, exec, [3; 32]));
    let (cr, mut cw) = tokio::io::split(client);
    let mut cr = BufReader::new(cr);
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    let first = json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"_meta":meta,"name":"providers","arguments":{}}});
    cw.write_all(format!("{first}\n").as_bytes()).await.unwrap();
    while !started.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    let busy = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"_meta":meta,"name":"providers","arguments":{}}})).await;
    assert_eq!(busy["result"]["structuredContent"]["error"], "busy");
    cw.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{\"requestId\":10}}\n").await.unwrap();
    while !cancelled.load(Ordering::SeqCst) {
        tokio::task::yield_now().await;
    }
    let busy_after_cancel = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"_meta":meta,"name":"providers","arguments":{}}})).await;
    assert_eq!(busy_after_cancel["id"], 12);
    assert_eq!(
        busy_after_cancel["result"]["structuredContent"]["error"],
        "busy"
    );
    release.store(true, Ordering::SeqCst);
    let mut late = String::new();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            cr.read_line(&mut late)
        )
        .await
        .is_err(),
        "late result: {late}"
    );
    drop(cw);
    drop(cr);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn oversized_frame_never_reaches_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (client, server) = tokio::io::duplex(256 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let task = tokio::spawn(serve_io(sr, sw, executor(calls.clone()), [4; 32]));
    let (cr, mut cw) = tokio::io::split(client);
    let message = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"padding\":\"{}\"}}\n",
        "x".repeat(70_000)
    );
    let _ = cw.write_all(message.as_bytes()).await;
    drop(cw);
    drop(cr);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .expect("oversized frame must terminate transport");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn duplicate_keys_do_not_reach_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (client, server) = tokio::io::duplex(128 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let task = tokio::spawn(serve_io(sr, sw, executor(calls.clone()), [8; 32]));
    let (cr, mut cw) = tokio::io::split(client);
    let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}},"name":"graph","arguments":{"scope":{"max_entries":10001,"max_entries":1}}}}"#;
    cw.write_all(format!("{raw}\n").as_bytes()).await.unwrap();
    drop(cw);
    drop(cr);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .expect("duplicate frame must terminate transport");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn authority_calls_and_confirmation_arguments_never_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let recorded = calls.clone();
    let exec: Executor = Arc::new(move |_, _| {
        recorded.fetch_add(1, Ordering::SeqCst);
        Err(ApplicationError::Cancelled)
    });
    let (client, server) = tokio::io::duplex(128 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let task = tokio::spawn(serve_io(sr, sw, exec, [9; 32]));
    let (cr, mut cw) = tokio::io::split(client);
    let mut cr = BufReader::new(cr);
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});

    for name in [
        "approve_plan",
        "authorize_transaction",
        "confirm_delete",
        "grant_permission",
        "sign_plan",
        "execute_plan",
    ] {
        let response = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"_meta":meta,"name":name,"arguments":{}}})).await;
        assert_eq!(response["error"]["code"], -32602, "{name}: {response}");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{name} dispatched");
    }

    let target = json!({"provider":"codex","class":"session","token":"a".repeat(64)});
    for (name, arguments) in [
        ("providers", json!({})),
        ("plan_lifecycle", json!({"target":target,"intent":"purge"})),
        ("repair_plan", json!({"target":target})),
    ] {
        // A valid control proves the extra field causes each rejection.
        let before = calls.load(Ordering::SeqCst);
        let response = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"_meta":meta,"name":name,"arguments":arguments}})).await;
        assert_eq!(
            response["result"]["structuredContent"]["error"], "cancelled",
            "{name}: {response}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), before + 1);
        for field in ["confirm", "authorized", "yes_really_delete"] {
            let mut extra = arguments.clone();
            extra[field] = json!(true);
            let response = exchange(&mut cr, &mut cw, json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"_meta":meta,"name":name,"arguments":extra}})).await;
            assert_eq!(
                response["error"]["code"], -32602,
                "{name} {field}: {response}"
            );
            assert_eq!(
                calls.load(Ordering::SeqCst),
                before + 1,
                "{name} {field} dispatched"
            );
        }
    }
    drop(cw);
    drop(cr);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn opt_in_conversation_registry_metadata_privacy_and_authority_rejection() {
    use agentmender_application::conversation::{
        ConversationRequest, ConversationResponse, SessionMatch,
    };
    use agentmender_domain::*;
    let calls = Arc::new(AtomicUsize::new(0));
    let workflow_calls = calls.clone();
    let workflow: agentmender_mcp_server::ConversationExecutor = Arc::new(move |request, _| {
        workflow_calls.fetch_add(1, Ordering::SeqCst);
        match request {
            ConversationRequest::Search { .. } => Ok(ConversationResponse::Search {
                read_compatibility: None,
                completeness: Completeness::Partial,
                sessions: vec![SessionMatch {
                    metadata: SessionMetadata {
                        identity: ArtifactId {
                            provider: ProviderKind::Codex,
                            state_root: "/PRIVATE/ROOT".into(),
                            class: GraphNodeKind::Session,
                            local_id: EntityId::new(
                                "codex:session:12345678-1234-1234-1234-123456789abc",
                            )
                            .unwrap(),
                        },
                        native_session_id: "12345678-1234-1234-1234-123456789abc".into(),
                        title: Some("MISSION1 README\u{1b}\u{202e}".into()),
                        project_label: Some("MISSION1".into()),
                        created_at: Some(1),
                        last_active_at: Some(2),
                        state: SessionState::Archived,
                        producer_version_matches: true,
                        producer_version: None,
                        parent_native_session_id: None,
                    },
                    can_request_preview: true,
                }],
            }),
            ConversationRequest::Status { .. } | ConversationRequest::RequestExecution { .. } => {
                Ok(ConversationResponse::Transactions(vec![]))
            }
            _ => Err(ApplicationError::Cancelled),
        }
    });
    let (client, server) = tokio::io::duplex(256 * 1024);
    let (sr, sw) = tokio::io::split(server);
    let task = tokio::spawn(agentmender_mcp_server::serve_io_with_conversation(
        sr,
        sw,
        executor(Arc::new(AtomicUsize::new(0))),
        workflow,
        [42; 32],
    ));
    let (cr, mut cw) = tokio::io::split(client);
    let mut cr = BufReader::new(cr);
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"fixture","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    let list = exchange(
        &mut cr,
        &mut cw,
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":meta}}),
    )
    .await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 13);
    assert!(serde_json::to_vec(&list).unwrap().len() + 64 * 1024 + 128 <= 256 * 1024);
    let call = |id, name, args| json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"_meta":meta,"name":name,"arguments":args}});
    let result = exchange(
        &mut cr,
        &mut cw,
        call(2, "session_search", json!({"query":"MISSION1 README"})),
    )
    .await;
    let view = &result["result"]["structuredContent"];
    assert_eq!(view["completeness"], "partial");
    assert_eq!(view["sessions"][0]["title"], "MISSION1 README");
    assert_eq!(
        view["sessions"][0]["native_session_id"],
        "12345678-1234-1234-1234-123456789abc"
    );
    assert!(!result.to_string().contains("PRIVATE"));
    let target = view["sessions"][0]["target"].clone();
    for (id, name, args) in [
        (
            3,
            "prepare_removal",
            json!({"target":target,"expected_plan_digest":"a".repeat(64),"confirm":true}),
        ),
        (
            4,
            "preview_removal",
            json!({"target":target,"caller_session_id":"attacker"}),
        ),
        (
            5,
            "request_removal_execution",
            json!({"transaction_id":"a".repeat(64),"approval":true}),
        ),
        (6, "removal_status", json!({"transaction_id":"forged"})),
        (
            7,
            "session_search",
            json!({"query":"x","root":"/arbitrary"}),
        ),
    ] {
        let rejected = exchange(&mut cr, &mut cw, call(id, name, args)).await;
        assert_eq!(rejected["error"]["code"], -32602, "{rejected}");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let parsed_digest = exchange(
        &mut cr,
        &mut cw,
        call(
            12,
            "prepare_removal",
            json!({"target":target,"expected_plan_digest":format!("sha256:{}", "a".repeat(64))}),
        ),
    )
    .await;
    assert_eq!(
        parsed_digest["result"]["structuredContent"]["error"],
        "cancelled"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let handoff = exchange(
        &mut cr,
        &mut cw,
        call(
            8,
            "request_removal_execution",
            json!({"transaction_id":"a".repeat(64)}),
        ),
    )
    .await;
    assert!(handoff["result"]["structuredContent"]["transactions"].is_array());
    drop(cw);
    drop(cr);
    task.await.unwrap().unwrap();
}
