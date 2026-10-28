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

#[test]
fn observed_claude_graph_explains_mapping_without_spawn_claim() {
    let home = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/claude/4.29.0-observed");
    let prefix = [
        "--json",
        "--provider",
        "claude",
        "--fixture",
        fixture.to_str().unwrap(),
        "--desktop-fixture",
    ];
    let mut args = prefix.to_vec();
    args.push("graph");
    let output = run(home.path(), &args);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let graph = &value["report"]["graphs"][0];
    let edges = graph["edges"].as_array().unwrap();
    assert!(!edges.iter().any(|edge| edge["kind"] == "spawned_by"));
    let mapping = edges.iter().find(|edge| edge["kind"] == "maps_to").unwrap();
    let id = mapping["source"]["local_id"].as_str().unwrap();
    let mut args = prefix.to_vec();
    args.extend(["explain", id]);
    let explained = run(home.path(), &args);
    let report: serde_json::Value = serde_json::from_slice(&explained.stdout).unwrap();
    let rule = report["report"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["id"] == "explicit_cliSessionId")
        .unwrap();
    assert!(!rule["description"].as_str().unwrap().is_empty());
    assert_eq!(rule["max_confidence"], "heuristic");
    let evidence = report["report"]["explanation"]["evidence"]
        .as_array()
        .unwrap();
    let mapping = evidence
        .iter()
        .find(|item| item["evidence"]["rule_id"] == "explicit_cliSessionId")
        .unwrap();
    assert_eq!(mapping["confidence"], "heuristic");
    assert!(mapping["evidence"]["observed_at"].is_null());
    assert!(mapping["evidence"]["observation"]["provider_version"].is_null());
}

#[test]
fn unknown_version_artifact_remains_protected() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/unknown");
    let output = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            fixture.to_str().unwrap(),
            "explain",
            "codex:artifact:manifest.json",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let explanation = &value["report"]["explanation"];
    assert_eq!(explanation["ownership"]["classification"], "unknown");
    assert!(
        explanation["ownership"]["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b == "unknown_compatibility")
    );
    assert!(
        explanation["roots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "unknown_state")
    );
}

#[test]
fn verify_reports_unknown_and_partial_proof_boundaries() {
    let home = tempfile::tempdir().unwrap();
    for (provider, fixture, desktop) in [
        ("codex", "../../fixtures/codex/unknown", false),
        ("codex", "../../fixtures/codex/partial", false),
        ("claude", "../../fixtures/claude/4.29.0-observed", true),
    ] {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
        let mut args = vec![
            "--json",
            "--provider",
            provider,
            "--fixture",
            fixture.to_str().unwrap(),
        ];
        if desktop {
            args.push("--desktop-fixture");
        }
        args.push("verify");
        let output = run(home.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &value["report"]["reports"][0];
        assert_eq!(report["context"]["completeness"], "partial");
        assert!(matches!(
            report["verification_status"].as_str(),
            Some("unknown" | "degraded")
        ));
        assert!(report["proof_context"]["contract"].is_null());
        assert!(
            report["blockers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|b| b["code"] == "missing_integrity_contract")
        );
        assert!(report["residuals"].as_array().unwrap().is_empty());
        assert!(value["report"]["failures"].as_array().unwrap().is_empty());
    }
}

#[test]
fn scoped_verify_is_unknown_and_invalid_scope_is_structured() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known");
    let prefix = [
        "--json",
        "--provider",
        "codex",
        "--fixture",
        fixture.to_str().unwrap(),
    ];
    let mut args = prefix.to_vec();
    args.extend([
        "--project",
        "/example/project",
        "--active-only",
        "--max-entries",
        "1",
        "verify",
    ]);
    let output = run(home.path(), &args);
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let report = &value["report"]["reports"][0];
    assert!(matches!(
        report["verification_status"].as_str(),
        Some("unknown" | "degraded")
    ));
    assert!(report["proof_context"]["contract"].is_null());
    for invalid in [
        vec!["--max-entries", "0", "verify"],
        vec!["--project", "relative", "verify"],
    ] {
        let mut args = prefix.to_vec();
        args.extend(invalid);
        let output = run(home.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["code"], "invalid_arguments");
    }
}

#[test]
fn planning_reports_intention_and_rejects_invalid_targets() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/codex/observed-snapshot-0.165.0-alpha.1.2");
    let prefix = [
        "--json",
        "--provider",
        "codex",
        "--fixture",
        fixture.to_str().unwrap(),
    ];
    let mut args = prefix.to_vec();
    args.push("scan");
    let scan = run(home.path(), &args);
    let value: serde_json::Value = serde_json::from_slice(&scan.stdout).unwrap();
    let id = value["report"]["snapshots"][0]["sessions"][0]["id"]
        .as_str()
        .unwrap();
    let mut args = prefix.to_vec();
    args.extend(["plan", "archive", id]);
    let output = run(home.path(), &args);
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["mode"], "planning_only");
    let plan = &value["report"]["plan"];
    assert_eq!(plan["content"]["status"], "unsupported");
    assert!(
        plan["content"]["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b == "integrity_unverified")
    );
    assert!(plan["content"]["operations"].as_array().unwrap().is_empty());
    assert!(plan["digest"].as_str().unwrap().starts_with("sha256:"));
    args.remove(0);
    let output = run(home.path(), &args);
    let text = String::from_utf8(output.stdout).unwrap();
    for fact in [
        "Target:",
        "Intent:",
        "Digest:",
        "State fingerprint:",
        "Native capability:",
        "Blockers:",
        "Preconditions:",
        "Postconditions:",
        "Space:",
        "Execution: NOT PERFORMED",
    ] {
        assert!(text.contains(fact), "missing {fact}");
    }
    for tail in [
        vec!["plan", "invalid", id],
        vec!["plan", "archive", ""],
        vec!["execute"],
        vec!["approve"],
    ] {
        let mut args = prefix.to_vec();
        args.extend(tail);
        let output = run(home.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn claude_purge_is_unsupported_and_unknown_version_remains_protected() {
    let home = tempfile::tempdir().unwrap();
    for (provider, fixture, target, expected) in [
        (
            "claude",
            "../../fixtures/claude/2.1.311-synthetic",
            None,
            "unsupported",
        ),
        (
            "codex",
            "../../fixtures/codex/unknown",
            Some("codex:artifact:manifest.json"),
            "blocked",
        ),
    ] {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
        let prefix = [
            "--json",
            "--provider",
            provider,
            "--fixture",
            fixture.to_str().unwrap(),
        ];
        let id = if let Some(target) = target {
            target.to_string()
        } else {
            let mut args = prefix.to_vec();
            args.push("scan");
            let scan = run(home.path(), &args);
            let value: serde_json::Value = serde_json::from_slice(&scan.stdout).unwrap();
            value["report"]["snapshots"][0]["sessions"][0]["id"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let mut args = prefix.to_vec();
        args.extend(["plan", "purge", &id]);
        let output = run(home.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["report"]["plan"]["content"]["status"], expected);
        assert!(
            !value["report"]["plan"]["content"]["blockers"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn plan_rejects_ambiguous_ids_and_incomplete_absence() {
    let home = tempfile::tempdir().unwrap();
    let ambiguous = run(
        home.path(),
        &[
            "--json",
            "--codex-root",
            home.path().join("codex").to_str().unwrap(),
            "--claude-root",
            home.path().join("claude").to_str().unwrap(),
            "plan",
            "archive",
            "provider-state",
        ],
    );
    assert_eq!(ambiguous.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&ambiguous.stderr).unwrap();
    assert_eq!(error["code"], "ambiguous_entity");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/partial");
    let missing = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            fixture.to_str().unwrap(),
            "plan",
            "archive",
            "absent",
        ],
    );
    assert_eq!(missing.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&missing.stderr).unwrap();
    assert_eq!(error["code"], "inspection_unavailable");
}
