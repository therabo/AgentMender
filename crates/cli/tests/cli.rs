use std::{
    path::Path,
    process::{Command, Output},
};

fn run(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home)
        .env("PATH", "")
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn version_needs_no_provider_environment() {
    let result = Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .arg("--version")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("agentmender")
    );
}

#[test]
fn providers_reports_isolated_undetected_state_as_json() {
    let home = tempfile::tempdir().unwrap();
    let result = run(
        home.path(),
        &[
            "--json",
            "--claude-root",
            home.path().to_str().unwrap(),
            "providers",
        ],
    );
    assert_eq!(result.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["mode"], "live");
    assert_eq!(json["report"]["providers"].as_array().unwrap().len(), 2);
    assert_eq!(json["report"]["providers"][0]["presence"], "not_detected");
    assert!(json["report"]["providers"][0]["installations"].is_array());
    assert!(json["report"]["providers"][0]["evidence_status"].is_string());
    assert!(json["report"]["providers"][0]["diagnostics"].is_array());
    assert!(result.stderr.is_empty());
}

#[test]
fn malformed_arguments_are_structured_on_stderr() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["--json", "--fixture", "/missing", "scan"],
        vec!["--json", "--max-entries", "0", "scan"],
        vec!["--json", "--codex-executable", "relative", "providers"],
        vec!["--json", "--claude-desktop-root", "relative", "providers"],
        vec!["--json", "--claude-app-bundle", "relative", "providers"],
    ] {
        let result = run(home.path(), &args);
        assert_eq!(result.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["code"], "invalid_arguments");
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn fixture_errors_are_structured_without_reading_live_homes() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("missing-fixture");
    let result = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            missing.to_str().unwrap(),
            "scan",
        ],
    );
    assert_eq!(result.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert!(error["code"].is_string());
    assert!(result.stdout.is_empty());
}

#[test]
fn fixture_scan_and_inspect_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known");
    let fixture = fixture.to_str().unwrap();
    let scan = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            fixture,
            "scan",
        ],
    );
    assert_eq!(scan.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&scan.stdout).unwrap();
    assert_eq!(report["mode"], "fixture");
    let sessions = report["report"]["snapshots"][0]["sessions"]
        .as_array()
        .unwrap();
    assert!(!sessions.is_empty());
    let id = sessions[0]["id"].as_str().unwrap();
    let inspected = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            fixture,
            "inspect",
            id,
        ],
    );
    assert_eq!(inspected.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(report["report"]["entity"]["entity"]["id"], id);
}

#[test]
fn explicit_claude_root_does_not_infer_desktop_context() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("custom");
    std::fs::create_dir(&config).unwrap();
    let desktop = home
        .path()
        .join("Library/Application Support/Claude/claude-code-sessions");
    std::fs::create_dir_all(&desktop).unwrap();
    let result = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "claude",
            "--claude-root",
            config.to_str().unwrap(),
            "providers",
        ],
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        report["report"]["providers"][0]["installations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        report["report"]["providers"][0]["state_root"],
        config.to_str().unwrap()
    );
}

#[test]
fn observed_desktop_fixture_maps_seven_sessions() {
    let home = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/claude/4.29.0-observed");
    let result = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "claude",
            "--fixture",
            fixture.to_str().unwrap(),
            "--desktop-fixture",
            "scan",
        ],
    );
    assert_eq!(result.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let snapshot = &report["report"]["snapshots"][0];
    assert_eq!(report["mode"], "fixture");
    assert_eq!(snapshot["sessions"].as_array().unwrap().len(), 7);
    assert_eq!(
        snapshot["references"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "maps_to")
            .count(),
        7
    );
    assert_eq!(snapshot["evidence_status"], "fixture_verified");
    assert_eq!(snapshot["completeness"], "partial");
}

#[test]
fn desktop_fixture_requires_claude_provider() {
    let home = tempfile::tempdir().unwrap();
    let result = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            "/unused",
            "--desktop-fixture",
            "scan",
        ],
    );
    assert_eq!(result.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["code"], "invalid_arguments");
}
