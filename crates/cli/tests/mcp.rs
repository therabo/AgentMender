use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
};

#[test]
fn fixture_stdio_is_protocol_only_and_preserves_provider_bytes() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known");
    let manifest = fixture.join("manifest.json");
    let before = std::fs::read(&manifest).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "")
        .args([
            "--provider",
            "codex",
            "--fixture",
            fixture.to_str().unwrap(),
            "mcp",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    writeln!(stdin, "{}", serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let init: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(init["id"], 1);
    assert!(init.get("result").is_some());
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(stdin, "{}", serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"providers","arguments":{}}})).unwrap();
    line.clear();
    stdout.read_line(&mut line).unwrap();
    let response: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());
    assert!(!line.contains(fixture.to_str().unwrap()));
    assert!(!line.contains(home.path().to_str().unwrap()));
    drop(stdin);
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert_eq!(std::fs::read(manifest).unwrap(), before);
}

fn fixture_bytes(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort();
    files
}

struct Session {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
    next_id: u64,
    fixture: std::path::PathBuf,
    home: tempfile::TempDir,
    files: Vec<(std::path::PathBuf, Vec<u8>)>,
}
impl Session {
    fn new(provider: &str, relative: &str, desktop: bool) -> Self {
        let home = tempfile::tempdir().unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(relative);
        let files = fixture_bytes(&fixture);
        let mut command = Command::new(env!("CARGO_BIN_EXE_agentmender"));
        command
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "")
            .args([
                "--provider",
                provider,
                "--fixture",
                fixture.to_str().unwrap(),
            ]);
        if desktop {
            command.arg("--desktop-fixture");
        }
        let mut child = command
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        writeln!(input, "{}", serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(&line)
                .unwrap()
                .get("result")
                .is_some()
        );
        writeln!(
            input,
            "{}",
            serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        Self {
            child,
            input,
            output,
            next_id: 2,
            fixture,
            home,
            files,
        }
    }
    fn call(&mut self, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        writeln!(self.input, "{}", serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})).unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        assert!(!line.contains(self.fixture.to_str().unwrap()));
        assert!(!line.contains(self.home.path().to_str().unwrap()));
        let response: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], id);
        response
    }
    fn finish(self) {
        drop(self.input);
        let result = self.child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        assert_eq!(fixture_bytes(&self.fixture), self.files);
    }
}
fn view(response: &serde_json::Value) -> &serde_json::Value {
    assert_ne!(response["result"]["isError"], true, "{response}");
    &response["result"]["structuredContent"]
}

#[test]
fn fixture_queries_cover_graph_integrity_and_planning() {
    for (provider, fixture, desktop) in [
        ("codex", "codex/known", false),
        ("codex", "codex/unknown", false),
        ("codex", "codex/partial", false),
        ("claude", "claude/2.1.311-synthetic", false),
        ("claude", "claude/4.29.0-observed", true),
    ] {
        let mut s = Session::new(provider, fixture, desktop);
        let scan = s.call(
            "workspace_scan",
            serde_json::json!({"page":{"page_size":100}}),
        );
        assert!(view(&scan)["contexts"].is_array());
        let verify = s.call("verify_integrity", serde_json::json!({}));
        assert!(
            view(&verify)["statuses"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "unknown" || v == "degraded")
        );
        let graph = s.call("graph", serde_json::json!({"page":{"page_size":100}}));
        let records = view(&graph)["records"].as_array().unwrap();
        let selected = records
            .iter()
            .filter(|r| r["kind"] == "node")
            .find(|r| r["node"]["id"]["class"] == "session")
            .or_else(|| records.iter().find(|r| r["kind"] == "node"));
        {
            let node = selected.expect("fixture graph has a selectable node");
            let target = node["node"]["id"].clone();
            for tool in ["inspect", "explain"] {
                let response = s.call(tool, serde_json::json!({"target":target}));
                assert!(view(&response)["contexts"].is_array());
            }
            let plan = s.call(
                "plan_lifecycle",
                serde_json::json!({"target":target,"intent":"purge"}),
            );
            let status = &view(&plan)["plan"]["status"];
            assert!(status == "blocked" || status == "unsupported", "{plan}");
            if provider == "claude" && target["class"] == "session" {
                assert_eq!(status, "unsupported");
            }
        }
        let missing = s.call("explain", serde_json::json!({"target":{"provider":provider,"class":"session","token":"0".repeat(64)}}));
        assert_eq!(missing["result"]["isError"], true);
        if fixture == "codex/partial" || fixture == "claude/4.29.0-observed" {
            assert_eq!(
                missing["result"]["structuredContent"]["error"],
                "target_unavailable"
            );
        }
        let missing_ref = s.call("explain", serde_json::json!({}));
        assert!(missing_ref.get("error").is_some());
        let invalid = s.call(
            "explain",
            serde_json::json!({"target":{"provider":provider,"class":"session","token":"bad"}}),
        );
        assert!(invalid.get("error").is_some());
        let path = s.call("graph", serde_json::json!({"path":"/arbitrary/private"}));
        assert!(path.get("error").is_some());
        let other = if provider == "codex" {
            "claude"
        } else {
            "codex"
        };
        let mismatch = s.call("graph", serde_json::json!({"provider":other}));
        assert_eq!(mismatch["result"]["isError"], true);
        assert_eq!(
            mismatch["result"]["structuredContent"]["error"],
            "provider_unsupported"
        );
        s.finish();
    }
}
