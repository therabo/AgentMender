//! Read-only export inventory. Provider publication metadata does not grant
//! authority over publisher locks, leases, shared outputs or filesystem paths.
use crate::discovery::{Access, Storage, StorageKind, access, identity};
use agentmender_domain::canonical_uuid;
use agentmender_provider_api::filesystem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::collections::HashMap;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_ENTRIES: usize = 350_000;
const MAX_GROUPS: usize = 4_096;
const MAX_MANIFEST_BYTES: usize = 8_192;
const MARKER: &str = ".artifact-session-publication.json";
const LEASE: &str = ".artifact-session-publication-lease.json";
const MISSING_CONTRACT: &str = "La rimozione richiede un contratto verificato dell’esportatore per proprietà, riferimenti e writer.";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Protection {
    ProviderManaged,
    IndependentBackup,
    UserOutput,
    UnknownOwner,
    Shared,
    InUse,
    Incomplete,
}

#[derive(Clone, Default, Serialize)]
pub struct Size {
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub files: u64,
    pub directories: u64,
}

#[derive(Clone, Serialize)]
pub struct Group {
    pub key: String,
    pub name: String,
    pub path: String,
    pub root: String,
    pub size: Size,
    pub created_at: Option<u64>,
    pub modified_at: Option<u64>,
    pub format: String,
    manifest: Option<&'static str>,
    producer: Option<String>,
    session_id: Option<String>,
    shared_entries: u64,
    linked_entries: u64,
    protection: Protection,
    pub reason: &'static str,
    pub removal_available: bool,
    pub complete: bool,
}

impl Group {
    pub fn has_shared_entries(&self) -> bool {
        self.shared_entries > 0 || self.linked_entries > 0
    }
}

pub struct Inventory {
    pub groups: Vec<Group>,
    pub size: Size,
    pub complete: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    producer: Option<String>,
    session_id: Option<String>,
    publication_id: Option<String>,
    file_name: Option<String>,
    byte_length: Option<u64>,
    digest: Option<String>,
    created_at_ms: Option<u64>,
    minimum_retain_until_ms: Option<u64>,
    renewed_at_ms: Option<u64>,
}

#[derive(Default)]
struct Scan {
    size: Size,
    shared: u64,
    links: u64,
    incomplete: bool,
    revision: [u8; 32],
    #[cfg(unix)]
    linked_inodes: HashSet<(u64, u64)>,
}

#[derive(Default)]
struct Allocation {
    #[cfg(unix)]
    inodes: HashMap<(u64, u64), u64>,
    direct: u64,
}

struct Budget {
    entries: usize,
    deadline: Instant,
}

impl Budget {
    fn take(&mut self) -> bool {
        if self.entries == 0 || Instant::now() >= self.deadline {
            return false;
        }
        self.entries -= 1;
        true
    }
}

fn display(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .take(2048)
        .map(|c| if c.is_control() { '�' } else { c })
        .collect()
}

fn unix_time(result: io::Result<SystemTime>) -> Option<u64> {
    result
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|time| time.as_secs())
}

fn safe_label(value: String) -> Option<String> {
    (value.len() <= 80
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || " .+_-".contains(c)))
    .then_some(value)
}

fn manifest(root: &Path, path: &Path) -> Option<Manifest> {
    let bytes = filesystem::read_bounded(root, path, MAX_MANIFEST_BYTES).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn publication(group: &Path, value: &Manifest) -> bool {
    let id = group.file_name().and_then(|name| name.to_str());
    let digest_ok = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    };
    matches!(
        value.schema.as_deref(),
        Some(
            "openai.artifact-session.filesystem-publication-marker/v1"
                | "openai.artifact-session.filesystem-publication-marker/v2"
        )
    ) && id.is_some_and(digest_ok)
        && id == value.publication_id.as_deref()
        && value.digest.as_deref().is_some_and(digest_ok)
        && value.file_name.as_deref().is_some_and(|name| {
            let path = Path::new(name);
            matches!(
                path.components().next(),
                Some(std::path::Component::Normal(_))
            ) && path.components().count() == 1
                && !name.contains(['/', '\\', ':'])
                && fs::symlink_metadata(group.join(path)).is_ok_and(|meta| {
                    meta.is_file()
                        && !filesystem::is_link(&meta)
                        && Some(meta.len()) == value.byte_length
                })
        })
}

fn status(protection: Protection) -> &'static str {
    match protection {
        Protection::ProviderManaged => {
            "Pubblicazione Artifact Session protetta dal protocollo di quote e lease del servizio."
        }
        Protection::IndependentBackup => "Backup indipendente da preservare.",
        Protection::UserOutput => {
            "Output dell’utente. La pubblicazione può avere riferimenti esterni."
        }
        Protection::UnknownOwner => MISSING_CONTRACT,
        Protection::Shared => {
            "Sono presenti hardlink o collegamenti. Gli altri riferimenti restano protetti."
        }
        Protection::InUse => {
            "Il servizio ha registrato un lease o un riferimento di pubblicazione attivo."
        }
        Protection::Incomplete => {
            "Inventario parziale per accesso, budget o modifica del contenuto."
        }
    }
}

fn collect(
    path: &Path,
    meta: &fs::Metadata,
    budget: &mut Budget,
    allocation: &mut Allocation,
) -> Scan {
    #[cfg(not(unix))]
    let _ = &allocation;
    let mut result = Scan::default();
    let mut queue = vec![(path.to_owned(), 0usize, meta.clone())];
    let mut revision = Sha256::new();
    let root_device = device(meta);
    while let Some((path, depth, metadata)) = queue.pop() {
        let encoded = path.as_os_str().as_encoded_bytes();
        revision.update(encoded.len().to_le_bytes());
        revision.update(encoded);
        revision.update(format!("{:?}", fingerprint(&metadata)));
        if filesystem::is_link(&metadata) {
            result.links += 1;
            continue;
        }
        if device(&metadata) != root_device {
            result.incomplete = true;
            continue;
        }
        if metadata.is_file() {
            result.size.files += 1;
            result.size.logical_bytes = result.size.logical_bytes.saturating_add(metadata.len());
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let blocks = metadata.blocks().saturating_mul(512);
                let total = result.size.allocated_bytes.get_or_insert(0);
                if metadata.nlink() > 1 {
                    result.shared += 1;
                    let ident = (metadata.dev(), metadata.ino());
                    allocation.inodes.entry(ident).or_insert(blocks);
                    if !result.linked_inodes.insert(ident) {
                        continue;
                    }
                } else {
                    allocation.direct = allocation.direct.saturating_add(blocks);
                }
                *total = total.saturating_add(blocks);
            }
            continue;
        }
        if !metadata.is_dir() || depth >= 64 {
            result.incomplete = true;
            continue;
        }
        result.size.directories += u64::from(depth > 0);
        let before = fingerprint(&metadata);
        if !fs::symlink_metadata(&path).is_ok_and(|current| fingerprint(&current) == before)
            || access(&path) != Access::Accessible
        {
            result.incomplete = true;
            continue;
        }
        let Ok(entries) = fs::read_dir(&path) else {
            result.incomplete = true;
            continue;
        };
        let mut children = Vec::new();
        for entry in entries {
            if !budget.take() {
                result.incomplete = true;
                break;
            }
            let result_entry = entry.and_then(|entry| {
                let child = entry.path();
                fs::symlink_metadata(&child).map(|meta| (child, depth + 1, meta))
            });
            match result_entry {
                Ok(entry) => children.push(entry),
                Err(_) => result.incomplete = true,
            }
        }
        children.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        queue.extend(children);
        if !fs::symlink_metadata(&path).is_ok_and(|current| fingerprint(&current) == before) {
            result.incomplete = true;
        }
    }
    result.revision.copy_from_slice(&revision.finalize());
    result
}

fn device(meta: &fs::Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(meta.dev())
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        None
    }
}

fn fingerprint(meta: &fs::Metadata) -> (bool, bool, u64, Option<SystemTime>, Option<(u64, u64)>) {
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        Some((meta.dev(), meta.ino()))
    };
    #[cfg(not(unix))]
    let identity = None;
    (
        meta.is_dir(),
        filesystem::is_link(meta),
        meta.len(),
        meta.modified().ok(),
        identity,
    )
}

fn group(
    storage: &Storage,
    path: PathBuf,
    budget: &mut Budget,
    allocation: &mut Allocation,
) -> Group {
    let name = path
        .file_name()
        .map(Path::new)
        .map(display)
        .unwrap_or_default();
    let metadata = fs::symlink_metadata(&path).ok();
    let mut scan = metadata
        .as_ref()
        .map(|meta| collect(&path, meta, budget, allocation))
        .unwrap_or(Scan {
            incomplete: true,
            ..Scan::default()
        });
    let dir = metadata
        .as_ref()
        .is_some_and(|meta| meta.is_dir() && !filesystem::is_link(meta));
    let mut format = if dir {
        "Gruppo filesystem".into()
    } else {
        path.extension()
            .and_then(|e| safe_label(e.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "File".into())
    };
    let mut protection = Protection::UnknownOwner;
    let mut producer = None;
    let mut seen_manifest = None;
    let mut session_id = None;
    let mut created_at = metadata.as_ref().and_then(|meta| unix_time(meta.created()));
    if storage.kind == StorageKind::ExternalExports {
        protection = Protection::UserOutput;
    } else if dir && !scan.incomplete && scan.links == 0 && scan.shared == 0 {
        if storage.kind == StorageKind::ArtifactSessionExports && storage.owned {
            if name == "latest" {
                protection = Protection::Shared;
            }
            if let Some(marker) = manifest(&storage.path, &path.join(MARKER))
                .filter(|value| publication(&path, value))
            {
                format = "Pubblicazione Artifact Session".into();
                producer = Some("Codex Desktop Artifact Session".into());
                seen_manifest = Some(MARKER);
                protection = Protection::ProviderManaged;
                created_at = marker.created_at_ms.map(|time| time / 1000).or(created_at);
                let active = manifest(&storage.path, &path.join(LEASE)).is_some_and(|lease| {
                    let now = unix_time(Ok(SystemTime::now())).unwrap_or(0);
                    lease.publication_id == marker.publication_id
                        && (lease
                            .minimum_retain_until_ms
                            .is_some_and(|until| until / 1000 > now)
                            || lease
                                .renewed_at_ms
                                .is_some_and(|time| time / 1000 + 25 * 3600 > now))
                });
                if active {
                    protection = Protection::InUse;
                }
            }
        }
        if seen_manifest.is_none()
            && let Some(declared) = manifest(&storage.path, &path.join("manifest.json"))
        {
            seen_manifest = Some("manifest.json");
            producer = declared.producer.and_then(safe_label);
            session_id = declared.session_id.filter(|id| canonical_uuid(id));
            if declared.kind.as_deref() == Some("backup") {
                protection = Protection::IndependentBackup;
            }
        }
    }
    if scan.links > 0 || scan.shared > 0 {
        protection = Protection::Shared;
    }
    if !fs::symlink_metadata(&path)
        .ok()
        .zip(metadata.as_ref())
        .is_some_and(|(current, before)| fingerprint(&current) == fingerprint(before))
    {
        scan.incomplete = true;
    }
    if scan.incomplete {
        protection = Protection::Incomplete;
    }
    let mut hash = Sha256::new();
    hash.update(storage.identity.as_deref().unwrap_or_default());
    hash.update(path.as_os_str().as_encoded_bytes());
    hash.update(scan.revision);
    hash.update(identity(&path).unwrap_or_default());
    let root = display(&storage.path);
    Group {
        key: hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        name,
        path: display(&path),
        root,
        size: scan.size,
        created_at,
        modified_at: metadata
            .as_ref()
            .and_then(|meta| unix_time(meta.modified())),
        format,
        manifest: seen_manifest,
        producer,
        session_id,
        shared_entries: scan.shared,
        linked_entries: scan.links,
        protection,
        reason: status(protection),
        removal_available: false,
        complete: !scan.incomplete,
    }
}

impl Inventory {
    pub(crate) fn scan(storage: &[Storage]) -> io::Result<Self> {
        let mut budget = Budget {
            entries: MAX_ENTRIES,
            deadline: Instant::now() + Duration::from_secs(7),
        };
        let mut groups = Vec::new();
        let mut size = Size::default();
        let mut complete = true;
        let mut seen = HashSet::new();
        let mut allocation = Allocation::default();
        for root in storage.iter().filter(|item| {
            matches!(
                item.kind,
                StorageKind::ExportCandidate
                    | StorageKind::ArtifactSessionExports
                    | StorageKind::ExternalExports
            )
        }) {
            let Some(id) = &root.identity else {
                complete &= !root.observed;
                continue;
            };
            if !root.inventory_available {
                complete = false;
                continue;
            }
            if !seen.insert(id) {
                continue;
            }
            if identity(&root.path).as_ref() != Some(id) {
                complete = false;
                continue;
            }
            let Ok(meta) = fs::symlink_metadata(&root.path) else {
                complete = false;
                continue;
            };
            let before = fingerprint(&meta);
            let Ok(entries) = fs::read_dir(&root.path) else {
                complete = false;
                continue;
            };
            for item in entries {
                if groups.len() >= MAX_GROUPS || !budget.take() {
                    complete = false;
                    break;
                }
                let Ok(item) = item else {
                    complete = false;
                    continue;
                };
                let row = group(root, item.path(), &mut budget, &mut allocation);
                size.files = size.files.saturating_add(row.size.files);
                size.directories = size.directories.saturating_add(row.size.directories);
                size.logical_bytes = size.logical_bytes.saturating_add(row.size.logical_bytes);
                complete &= row.complete;
                groups.push(row);
            }
            complete &= identity(&root.path).as_ref() == Some(id)
                && fs::symlink_metadata(&root.path).is_ok_and(|meta| fingerprint(&meta) == before);
        }
        #[cfg(unix)]
        {
            size.allocated_bytes = Some(
                allocation
                    .inodes
                    .values()
                    .fold(allocation.direct, |sum, bytes| sum.saturating_add(*bytes)),
            );
        }
        #[cfg(not(unix))]
        let _ = allocation.direct;
        groups.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        Ok(Self {
            groups,
            size,
            complete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::access;

    #[test]
    fn recognizes_a_publication_and_lease_without_authorizing_removal() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().canonicalize().unwrap();
        let publication_id = "a".repeat(64);
        let bundle = root.join(&publication_id);
        fs::create_dir(&bundle).unwrap();
        fs::write(bundle.join("report.pdf"), b"example").unwrap();
        let marker = serde_json::json!({
            "schema": "openai.artifact-session.filesystem-publication-marker/v2",
            "publicationId": publication_id,
            "fileName": "report.pdf",
            "byteLength": 7,
            "digest": "b".repeat(64),
            "createdAtMs": 1500000000000u64,
        });
        fs::write(bundle.join(MARKER), serde_json::to_vec(&marker).unwrap()).unwrap();
        let storage = |owned| Storage {
            provider: "codex",
            platform: std::env::consts::OS,
            root: root.clone(),
            kind: StorageKind::ArtifactSessionExports,
            path: root.clone(),
            source: "fixture",
            identity: identity(&root),
            observed: true,
            recognized: owned,
            owned,
            access: access(&root),
            contract: None,
            inventory_available: true,
            removal_available: false,
            reason: "fixture",
        };
        let source = storage(true);
        let first = Inventory::scan(std::slice::from_ref(&source)).unwrap();
        assert!(first.complete);
        assert_eq!(first.groups[0].protection, Protection::ProviderManaged);
        assert_eq!(first.groups[0].created_at, Some(1_500_000_000));
        assert!(!first.groups[0].removal_available);
        let lease = serde_json::json!({
            "schema": "openai.artifact-session.filesystem-publication-lease/v2",
            "publicationId": publication_id,
            "minimumRetainUntilMs": 9_000_000_000_000u64,
        });
        fs::write(bundle.join(LEASE), serde_json::to_vec(&lease).unwrap()).unwrap();
        let second = Inventory::scan(&[source]).unwrap();
        assert_eq!(second.groups[0].protection, Protection::InUse);
        assert_ne!(first.groups[0].key, second.groups[0].key);
        let unverified_root = Inventory::scan(&[storage(false)]).unwrap();
        assert_eq!(
            unverified_root.groups[0].protection,
            Protection::UnknownOwner
        );
    }
}
