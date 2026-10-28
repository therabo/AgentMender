use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SECRET: &str = "SYNTHETIC_PRIVATE_BODY_MUST_NEVER_APPEAR_IN_OUTPUT";

fn manifest(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for item in fs::read_dir(path).unwrap() {
            let item = item.unwrap();
            let path = item.path();
            let kind = item.file_type().unwrap();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if kind.is_dir() {
                result.insert(relative, None);
                visit(root, &path, result);
            } else if kind.is_file() {
                result.insert(relative, Some(fs::read(&path).unwrap()));
            } else {
                panic!("unexpected fixture entry: {}", path.display());
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home)
        .env("PATH", "")
        .args(args)
        .output()
        .unwrap()
}
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}
fn no_secret(output: &Output) {
    assert!(!String::from_utf8_lossy(&output.stdout).contains(SECRET));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(SECRET));
}
fn fixture(provider: &str, dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(provider)
        .join(dir)
}

#[test]
fn fixture_scans_are_read_only_and_report_limits() {
    let home = tempfile::tempdir().unwrap();
    for (provider, dir, expected_diagnostic, desktop) in [
        ("codex", "known", None, false),
        ("codex", "observed-snapshot", None, false),
        ("codex", "unknown", Some("provider_version_unknown"), false),
        (
            "codex",
            "malformed",
            Some("malformed_provider_state"),
            false,
        ),
        (
            "codex",
            "missing-result",
            Some("malformed_provider_state"),
            false,
        ),
        ("codex", "partial", Some("incomplete_discovery"), false),
        ("claude", "2.1.311-synthetic", None, false),
        ("claude", "4.29.0-observed", None, true),
    ] {
        let root = fixture(provider, dir);
        let before = manifest(&root);
        let mut args = vec![
            "--json",
            "--provider",
            provider,
            "--fixture",
            root.to_str().unwrap(),
        ];
        if desktop {
            args.push("--desktop-fixture");
        }
        args.push("scan");
        let output = run(home.path(), &args);
        let value = json(&output);
        assert_eq!(value["mode"], "fixture");
        let snapshot = &value["report"]["snapshots"][0];
        assert_eq!(snapshot["provider"]["kind"], provider);
        if let Some(code) = expected_diagnostic {
            assert_eq!(output.status.code(), Some(2));
            assert_eq!(snapshot["completeness"], "partial");
            assert!(
                snapshot["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["code"] == code)
            );
        }
        if let Some(session) = snapshot["sessions"].as_array().unwrap().first() {
            let id = session["id"].as_str().unwrap();
            args.pop();
            args.extend(["inspect", id]);
            let inspect = run(home.path(), &args);
            assert_eq!(json(&inspect)["report"]["entity"]["entity"]["id"], id);
            no_secret(&inspect);
        }
        no_secret(&output);
        assert_eq!(
            manifest(&root),
            before,
            "fixture changed: {}",
            root.display()
        );
    }
}

#[test]
fn live_unknown_state_is_metadata_only_and_unmodified() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("arbitrary.json"),
        format!("{{ malformed {SECRET}"),
    )
    .unwrap();
    let before = manifest(root.path());
    for provider in ["codex", "claude"] {
        let flag = if provider == "codex" {
            "--codex-root"
        } else {
            "--claude-root"
        };
        let output = run(
            home.path(),
            &[
                "--json",
                "--provider",
                provider,
                flag,
                root.path().to_str().unwrap(),
                "scan",
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        let report = json(&output);
        let snapshot = &report["report"]["snapshots"][0];
        assert_eq!(report["mode"], "live");
        assert_eq!(snapshot["completeness"], "partial");
        assert_eq!(
            snapshot["provider"]["compatibility"]["status"],
            "audit_only_unknown"
        );
        assert!(snapshot["sessions"].as_array().unwrap().is_empty());
        assert!(
            snapshot["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["locator"].as_str().unwrap().ends_with("arbitrary.json"))
        );
        no_secret(&output);
    }
    assert_eq!(manifest(root.path()), before);
}

#[test]
fn missing_provider_and_incomplete_inspection_are_not_success() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("missing");
    let output = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--codex-root",
            missing.to_str().unwrap(),
            "providers",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        json(&output)["report"]["providers"][0]["presence"],
        "not_detected"
    );
    let root = fixture("codex", "partial");
    let before = manifest(&root);
    let output = run(
        home.path(),
        &[
            "--json",
            "--provider",
            "codex",
            "--fixture",
            root.to_str().unwrap(),
            "inspect",
            "codex:session:missing",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["code"], "inspection_unavailable");
    assert!(output.stdout.is_empty());
    assert_eq!(manifest(&root), before);
}

// The outer sandbox independently denies writes for the entire CLI process tree.
#[cfg(target_os = "macos")]
fn live_smoke(provider: &str, root_var: &str, executable_var: &str) {
    use std::ffi::OsString;
    let root = PathBuf::from(std::env::var_os(root_var).expect("explicit live root required"));
    let executable =
        PathBuf::from(std::env::var_os(executable_var).expect("explicit executable required"));
    assert!(root.is_absolute() && root.is_dir());
    assert!(executable.is_absolute() && executable.is_file());
    let home = tempfile::tempdir().unwrap();
    let canary = home.path().join("write-canary");
    let profile = "(version 1)(allow default)(deny file-write*)(deny network*)";
    let check = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", profile, "/usr/bin/touch"])
        .arg(&canary)
        .output()
        .unwrap();
    assert!(
        !check.status.success() && !canary.exists(),
        "sandbox write denial could not be verified"
    );
    let (root_flag, executable_flag) = if provider == "codex" {
        ("--codex-root", "--codex-executable")
    } else {
        ("--claude-root", "--claude-executable")
    };
    let args: Vec<OsString> = vec![
        "--json".into(),
        "--provider".into(),
        provider.into(),
        root_flag.into(),
        root.into_os_string(),
        executable_flag.into(),
        executable.into_os_string(),
        "scan".into(),
    ];
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", profile, env!("CARGO_BIN_EXE_agentmender")])
        .args(args)
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "")
        .output()
        .unwrap();
    let value = json(&output);
    assert_eq!(value["mode"], "live");
    let snapshot = &value["report"]["snapshots"][0];
    assert!(snapshot["provider"]["compatibility"]["status"].is_string());
    let capabilities = snapshot["provider"]["capabilities"].as_array().unwrap();
    let read_only = [
        "metadata_audit",
        "session_discovery",
        "archived_session_discovery",
        "parent_session_discovery",
        "project_discovery",
        "artifact_discovery",
    ];
    assert!(
        capabilities
            .iter()
            .all(|c| read_only.contains(&c.as_str().unwrap()))
    );
    fn no_content_fields(value: &Value) {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    assert!(
                        !["content", "message_body", "transcript", "messages"]
                            .contains(&key.as_str())
                    );
                    no_content_fields(value);
                }
            }
            Value::Array(items) => {
                for item in items {
                    no_content_fields(item);
                }
            }
            _ => {}
        }
    }
    no_content_fields(&value);
    no_secret(&output);
}

#[test]
#[ignore = "requires explicit live roots and executables; macOS write-denied sandbox"]
fn live_codex_read_only() {
    #[cfg(target_os = "macos")]
    live_smoke(
        "codex",
        "AGENTMENDER_CODEX_ROOT",
        "AGENTMENDER_CODEX_EXECUTABLE",
    );
    #[cfg(not(target_os = "macos"))]
    panic!("live smoke requires macOS sandbox-exec");
}
#[test]
#[ignore = "requires explicit live roots and executables; macOS write-denied sandbox"]
fn live_claude_read_only() {
    #[cfg(target_os = "macos")]
    live_smoke(
        "claude",
        "AGENTMENDER_CLAUDE_ROOT",
        "AGENTMENDER_CLAUDE_EXECUTABLE",
    );
    #[cfg(not(target_os = "macos"))]
    panic!("live smoke requires macOS sandbox-exec");
}
