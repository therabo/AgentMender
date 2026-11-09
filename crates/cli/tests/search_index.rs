//! Explicit derived-index controls on disposable roots. No provider is started.
use agentmender_persistence::open_search_index;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn run(home: &Path, provider: &Path, index: &Path, action: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agentmender"))
        .env_clear()
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PATH", "")
        .args(["--json", "--provider", "codex", "--codex-root"])
        .arg(provider)
        .args(["search-index", "--root"])
        .arg(index)
        .arg(action)
        .output()
        .unwrap()
}
fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().canonicalize().unwrap();
    let provider = home.join("provider");
    fs::create_dir(&provider).unwrap();
    fs::write(
        provider.join("source-canary"),
        b"provider bytes must remain unchanged",
    )
    .unwrap();
    let index = home.join("private-search");
    (temp, home, provider, index)
}
fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "status={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn refused(output: Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["code"], "invalid_configuration");
}
#[test]
fn status_reset_and_corruption_rebuild_touch_only_explicit_derived_storage() {
    let (_temp, home, provider, index) = setup();
    let before = fs::read(provider.join("source-canary")).unwrap();
    let status = successful(run(&home, &provider, &index, "status"));
    assert_eq!(status["mode"], "search_index");
    assert_eq!(status["report"]["status"]["persistent"], true);
    assert_eq!(status["report"]["status"]["documents"], 0);
    assert_eq!(status["report"]["absence_proof"], false);
    {
        let mut derived = open_search_index(&index, std::slice::from_ref(&provider)).unwrap();
        derived
            .add(1, "synthetic Cesium frame-drop discussion")
            .unwrap();
        assert_eq!(derived.status().unwrap().documents, 1);
    }
    let status = successful(run(&home, &provider, &index, "status"));
    assert_eq!(status["report"]["status"]["documents"], 1);
    let reset = successful(run(&home, &provider, &index, "reset"));
    assert_eq!(reset["report"]["status"]["documents"], 0);
    assert_eq!(reset["report"]["provider_sources_modified"], false);
    fs::write(
        index.join("search.sqlite3"),
        b"synthetic corrupt derived database",
    )
    .unwrap();
    refused(run(&home, &provider, &index, "status"));
    successful(run(&home, &provider, &index, "rebuild"));
    let status = successful(run(&home, &provider, &index, "status"));
    assert_eq!(status["report"]["status"]["documents"], 0);
    assert_eq!(fs::read(provider.join("source-canary")).unwrap(), before);
    assert!(!provider.join("search.sqlite3").exists());
    assert!(!index.join("authority.key").exists());
    assert!(!index.join("transactions.sqlite3").exists());
}
#[test]
fn provider_overlap_refuses_before_creating_derived_files() {
    let (_temp, home, provider, _index) = setup();
    for index in [&provider, &provider.join("search"), &home] {
        for action in ["status", "reset", "rebuild"] {
            refused(run(&home, &provider, index, action));
        }
    }
    assert_eq!(fs::read_dir(&provider).unwrap().count(), 1);
    assert!(!home.join("search.sqlite3").exists());
}
#[cfg(unix)]
#[test]
fn owner_only_permissions_are_created_and_insecure_existing_state_refuses() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, home, provider, index) = setup();
    successful(run(&home, &provider, &index, "status"));
    assert_eq!(
        fs::metadata(&index).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let db = index.join("search.sqlite3");
    assert_eq!(
        fs::metadata(&db).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&index, fs::Permissions::from_mode(0o755)).unwrap();
    refused(run(&home, &provider, &index, "status"));
    assert_eq!(
        fs::metadata(&index).unwrap().permissions().mode() & 0o777,
        0o755
    );
    fs::set_permissions(&index, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&db, fs::Permissions::from_mode(0o644)).unwrap();
    let before = fs::read(&db).unwrap();
    refused(run(&home, &provider, &index, "status"));
    assert_eq!(fs::read(&db).unwrap(), before);
    assert_eq!(
        fs::metadata(&db).unwrap().permissions().mode() & 0o777,
        0o644
    );
}
#[cfg(unix)]
#[test]
fn linked_roots_and_database_files_refuse_without_touching_the_target() {
    use std::os::unix::fs::symlink;
    let (_temp, home, provider, index) = setup();
    successful(run(&home, &provider, &index, "status"));
    let alias = home.join("alias");
    symlink(&index, &alias).unwrap();
    for action in ["status", "reset", "rebuild"] {
        refused(run(&home, &provider, &alias, action));
    }
    let target = home.join("retained-file");
    fs::write(&target, b"unrelated retained bytes").unwrap();
    fs::remove_file(index.join("search.sqlite3")).unwrap();
    symlink(&target, index.join("search.sqlite3")).unwrap();
    for action in ["status", "reset", "rebuild"] {
        refused(run(&home, &provider, &index, action));
    }
    assert_eq!(fs::read(&target).unwrap(), b"unrelated retained bytes");
}
