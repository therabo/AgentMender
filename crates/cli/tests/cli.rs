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
fn providers_reports_isolated_missing_state_as_json() {
    let home = tempfile::tempdir().unwrap();
    let result = run(home.path(), &["--json", "providers"]);
    assert_eq!(result.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["mode"], "live");
    assert_eq!(json["report"]["providers"].as_array().unwrap().len(), 2);
    assert_eq!(json["report"]["providers"][0]["presence"], "missing");
    assert!(result.stderr.is_empty());
}

#[test]
fn malformed_arguments_are_structured_on_stderr() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["--json", "--fixture", "/missing", "scan"],
        vec!["--json", "--max-entries", "0", "scan"],
        vec!["--json", "--codex-executable", "relative", "providers"],
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
