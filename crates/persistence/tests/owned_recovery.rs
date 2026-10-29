use agentmender_persistence::OwnedStateRoot;
use std::{fs, path::PathBuf};

fn roots(base: &std::path::Path) -> Vec<PathBuf> {
    vec![base.join("provider")]
}
#[test]
fn root_is_explicit_private_and_reopens() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("owned");
    let protected = roots(temp.path());
    assert!(OwnedStateRoot::create(&path, &[]).is_err());
    OwnedStateRoot::create(&path, &protected).unwrap();
    OwnedStateRoot::open(&path, &protected).unwrap();
    assert!(OwnedStateRoot::create(&path, &protected).is_err());
}
#[test]
fn overlap_and_missing_parent_fail_before_creation() {
    let temp = tempfile::tempdir().unwrap();
    let provider = temp.path().join("provider");
    fs::create_dir(&provider).unwrap();
    assert!(
        OwnedStateRoot::create(&provider.join("owned"), std::slice::from_ref(&provider)).is_err()
    );
    assert!(OwnedStateRoot::create(temp.path(), std::slice::from_ref(&provider)).is_err());
    let path = temp.path().join("missing").join("owned");
    assert!(OwnedStateRoot::create(&path, std::slice::from_ref(&provider)).is_err());
    assert!(!path.exists());
    assert!(
        OwnedStateRoot::create(
            &temp.path().join(".. ").join("..").join("owned"),
            std::slice::from_ref(&provider)
        )
        .is_err()
    );
}
#[cfg(unix)]
#[test]
fn root_rejects_links_and_existing_broad_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let provider = temp.path().join("provider");
    fs::create_dir(&provider).unwrap();
    let alias = temp.path().join("alias");
    symlink(&provider, &alias).unwrap();
    assert!(OwnedStateRoot::create(&alias.join("owned"), std::slice::from_ref(&provider)).is_err());
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(OwnedStateRoot::open(&root, std::slice::from_ref(&provider)).is_err());
    let link = temp.path().join("rootlink");
    symlink(&root, &link).unwrap();
    assert!(OwnedStateRoot::open(&link, std::slice::from_ref(&provider)).is_err());
}

mod common;
use agentmender_domain::{LifecycleIntent, RecoveryManifest, TransactionBinding, TransactionId};
use agentmender_persistence::RecoveryStore;

fn manifest() -> RecoveryManifest {
    let plan = common::case(0).plan(LifecycleIntent::Archive);
    let binding = TransactionBinding::from_plan(&plan).unwrap();
    RecoveryManifest::new(TransactionId::from_bytes([9; 32]), &binding)
}

#[test]
fn future_recovery_schema_is_rejected_before_any_write() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("owned");
    let root = OwnedStateRoot::create(&path, &roots(temp.path())).unwrap();
    let mut value = serde_json::to_value(manifest()).unwrap();
    value["schema_version"] = serde_json::json!(2);
    let future: RecoveryManifest = serde_json::from_value(value).unwrap();
    assert!(RecoveryStore::new(root).prepare(&future).is_err());
    assert_eq!(fs::read_dir(path).unwrap().count(), 0);
}

#[test]
fn torn_existing_recovery_is_retained_and_never_replaced() {
    let temp = tempfile::tempdir().unwrap();
    let root = OwnedStateRoot::create(&temp.path().join("owned"), &roots(temp.path())).unwrap();
    let store = RecoveryStore::new(root);
    let manifest = manifest();
    let point = store.prepare(&manifest).unwrap();
    let path = temp
        .path()
        .join("owned")
        .join(format!("recovery-{}.json", point.digest()));
    fs::write(&path, b"{torn").unwrap();
    assert!(store.prepare(&manifest).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"{torn");
}

#[test]
fn concurrent_prepare_reuses_a_verified_durable_artifact() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("owned");
    let root = OwnedStateRoot::create(&path, &roots(temp.path())).unwrap();
    let m = manifest();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let store = RecoveryStore::new(root.clone());
            let m = m.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store.prepare(&m)
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert!(results.iter().any(Result::is_ok));
    let reopened = RecoveryStore::new(OwnedStateRoot::open(&path, &roots(temp.path())).unwrap());
    reopened.prepare(&m).unwrap();
    reopened.verify_manifest(&m).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn case_insensitive_provider_alias_is_rejected_before_creation() {
    let temp = tempfile::tempdir().unwrap();
    let provider = temp.path().join("providerstate");
    fs::create_dir(&provider).unwrap();
    let alias = temp.path().join("PROVIDERSTATE");
    if !alias.exists() {
        // This particular volume is case-sensitive; there is no case alias.
        return;
    }
    let child = alias.join("new-owned");
    assert!(OwnedStateRoot::create(&child, std::slice::from_ref(&provider)).is_err());
    assert!(!child.exists());
    // Neither endpoint exists, but their missing names still alias.
    let protected_missing = temp.path().join("missing-provider");
    let missing_alias_child = temp.path().join("MISSING-PROVIDER/new-owned");
    assert!(OwnedStateRoot::create(&missing_alias_child, &[protected_missing]).is_err());
    assert!(!missing_alias_child.exists());
}

#[cfg(target_os = "macos")]
#[test]
fn absent_endpoints_under_unicode_aliases_are_rejected_before_creation() {
    use std::os::unix::fs::MetadataExt;
    let temp = tempfile::tempdir().unwrap();
    let nfc = temp.path().join("\u{e9}parent");
    let nfd = temp.path().join("e\u{301}parent");
    fs::create_dir(&nfc).unwrap();
    let Ok(alias) = fs::metadata(&nfd) else {
        return;
    };
    let actual = fs::metadata(&nfc).unwrap();
    if (actual.dev(), actual.ino()) != (alias.dev(), alias.ino()) {
        return;
    }
    let protected = nfc.join("provider");
    let target = nfd.join("PROVIDER");
    assert!(OwnedStateRoot::create(&target, std::slice::from_ref(&protected)).is_err());
    assert!(!protected.exists());
    assert!(!target.exists());
}

#[cfg(unix)]
#[test]
fn dangling_protected_alias_never_creates_its_target() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("provider-target");
    let protected = temp.path().join("provider-link");
    symlink(&target, &protected).unwrap();
    assert!(OwnedStateRoot::create(&target, std::slice::from_ref(&protected)).is_err());
    assert!(!target.exists());
}
