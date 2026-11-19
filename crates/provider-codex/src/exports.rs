//! Bounded export inventory and removal of explicitly selected export groups.
use crate::discovery::{Access, Storage, StorageKind, access, identity};
use agentmender_domain::canonical_uuid;
use agentmender_provider_api::filesystem;
#[cfg(unix)]
use rustix::fs::{self as scoped, AtFlags, Mode, OFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::{collections::HashMap, os::unix::fs::MetadataExt, path::Component, sync::Arc};
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_ENTRIES: usize = 350_000;
const MAX_GROUPS: usize = 4_096;
const MAX_MANIFEST_BYTES: usize = 8_192;
const MARKER: &str = ".artifact-session-publication.json";
const LEASE: &str = ".artifact-session-publication-lease.json";
const CHANGED: &str = "Il gruppo o la sua root è cambiato dopo l’anteprima.";
#[cfg(unix)]
const OVERLAP: &str = "Il gruppo si sovrappone a un’altra area di storage rilevata.";

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
    #[cfg(unix)]
    #[serde(skip)]
    target: Option<RemovalTarget>,
}

#[cfg(unix)]
#[derive(Clone)]
struct RemovalTarget {
    source: Arc<(Storage, fs::Metadata)>,
    path: PathBuf,
    budget: Budget,
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
    in_use: bool,
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

#[derive(Clone, Copy)]
struct Budget {
    entries: usize,
}

impl Budget {
    fn new() -> Self {
        Self {
            entries: MAX_ENTRIES,
        }
    }

    fn take(&mut self) -> bool {
        if self.entries == 0 {
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
        Protection::ProviderManaged => "Pubblicazione Artifact Session.",
        Protection::IndependentBackup => "Backup indipendente.",
        Protection::UserOutput => "Output dell’utente.",
        Protection::UnknownOwner => "Gruppo di una root export con proprietà sconosciuta.",
        Protection::Shared => "Il gruppo contiene collegamenti o riferimenti condivisi.",
        Protection::InUse => "Il gruppo contiene un lease di pubblicazione attivo.",
        Protection::Incomplete => {
            "Inventario parziale per accesso, budget o modifica del contenuto."
        }
    }
}

type ObservedEntries = Vec<(PathBuf, fs::Metadata)>;

fn collect(
    path: &Path,
    meta: &fs::Metadata,
    budget: &mut Budget,
    allocation: &mut Allocation,
    mut observed: Option<&mut ObservedEntries>,
) -> Scan {
    #[cfg(not(unix))]
    let _ = &allocation;
    let mut result = Scan::default();
    let mut queue = vec![(path.to_owned(), 0usize, meta.clone())];
    let mut revision = Sha256::new();
    let root_device = device(meta);
    while let Some((entry, depth, metadata)) = queue.pop() {
        let encoded = entry.as_os_str().as_encoded_bytes();
        revision.update(encoded.len().to_le_bytes());
        revision.update(encoded);
        revision.update(format!("{:?}", fingerprint(&metadata)));
        if let Some(entries) = observed.as_deref_mut() {
            entries.push((entry.clone(), metadata.clone()));
        }
        if filesystem::is_link(&metadata) {
            result.links += 1;
            continue;
        }
        if device(&metadata) != root_device {
            result.incomplete = true;
            continue;
        }
        if metadata.is_file() {
            if entry.file_name().is_some_and(|name| name == LEASE) {
                match manifest(path, &entry) {
                    Some(lease) if lease.publication_id.is_some() => {
                        let now = unix_time(Ok(SystemTime::now())).unwrap_or(0);
                        result.in_use |= lease
                            .minimum_retain_until_ms
                            .is_some_and(|until| until / 1000 > now)
                            || lease
                                .renewed_at_ms
                                .is_some_and(|time| time / 1000 + 25 * 3600 > now);
                    }
                    _ => result.incomplete = true,
                }
            }
            result.size.files += 1;
            result.size.logical_bytes = result.size.logical_bytes.saturating_add(metadata.len());
            #[cfg(unix)]
            {
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
        result.size.directories += 1;
        let before = fingerprint(&metadata);
        if !fs::symlink_metadata(&entry).is_ok_and(|current| fingerprint(&current) == before)
            || access(&entry) != Access::Accessible
        {
            result.incomplete = true;
            continue;
        }
        let Ok(entries) = fs::read_dir(&entry) else {
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
        if !fs::symlink_metadata(&entry).is_ok_and(|current| fingerprint(&current) == before) {
            result.incomplete = true;
        }
    }
    result.revision.copy_from_slice(&revision.finalize());
    result
}

fn device(meta: &fs::Metadata) -> Option<u64> {
    #[cfg(unix)]
    {
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
    let identity = Some((meta.dev(), meta.ino()));
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
    path: &Path,
    budget: &mut Budget,
    allocation: &mut Allocation,
    observed: Option<&mut ObservedEntries>,
) -> Group {
    let name = path
        .file_name()
        .map(Path::new)
        .map(display)
        .unwrap_or_default();
    let metadata = fs::symlink_metadata(path).ok();
    let mut scan = metadata
        .as_ref()
        .map(|meta| collect(path, meta, budget, allocation, observed))
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
    let mut protection = if storage.kind == StorageKind::ExternalExports {
        Protection::UserOutput
    } else {
        Protection::UnknownOwner
    };
    let mut producer = None;
    let mut seen_manifest = None;
    let mut session_id = None;
    let mut created_at = metadata.as_ref().and_then(|meta| unix_time(meta.created()));
    if dir && !scan.incomplete && scan.links == 0 && scan.shared == 0 {
        if storage.kind == StorageKind::ArtifactSessionExports
            && storage.owned
            && let Some(marker) =
                manifest(&storage.path, &path.join(MARKER)).filter(|value| publication(path, value))
        {
            format = "Pubblicazione Artifact Session".into();
            producer = Some("Codex Desktop Artifact Session".into());
            seen_manifest = Some(MARKER);
            protection = Protection::ProviderManaged;
            created_at = marker.created_at_ms.map(|time| time / 1000).or(created_at);
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
    if scan.in_use {
        protection = Protection::InUse;
    }
    if scan.links > 0
        || scan.shared > 0
        || storage.kind == StorageKind::ArtifactSessionExports
            && name.eq_ignore_ascii_case("latest")
    {
        protection = Protection::Shared;
    }
    if !fs::symlink_metadata(path)
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
    hash.update(identity(path).unwrap_or_default());
    let root = display(&storage.path);
    Group {
        key: hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        name,
        path: display(path),
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
        removal_available: cfg!(unix),
        complete: !scan.incomplete,
        #[cfg(unix)]
        target: None,
    }
}

impl Inventory {
    pub(crate) fn scan(storage: &[Storage]) -> Self {
        let mut budget = Budget::new();
        let mut groups = Vec::new();
        let mut size = Size::default();
        let mut complete = true;
        let mut seen = HashSet::new();
        let mut allocation = Allocation::default();
        for root in storage.iter().filter(|item| export_kind(item.kind)) {
            let Some(id) = &root.identity else {
                complete &= !root.observed;
                continue;
            };
            if !root.inventory_available {
                complete = false;
                continue;
            }
            if root.kind == StorageKind::ExternalExports
                && storage.iter().any(|other| {
                    other.kind != StorageKind::ExternalExports
                        && export_kind(other.kind)
                        && other.inventory_available
                        && other.identity.as_ref() == Some(id)
                })
            {
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
            let first = groups.len();
            #[cfg(unix)]
            let source = Arc::new((root.clone(), meta));
            #[cfg(unix)]
            let scope = removal_scope(storage, root);
            for item in entries {
                if groups.len() >= MAX_GROUPS || !budget.take() {
                    complete = false;
                    break;
                }
                let Ok(item) = item else {
                    complete = false;
                    continue;
                };
                let path = item.path();
                #[cfg(unix)]
                let group_budget = budget;
                let mut row = group(root, &path, &mut budget, &mut allocation, None);
                #[cfg(unix)]
                {
                    let reason = match &scope {
                        Err(reason) => Some(*reason),
                        Ok(protected)
                            if !protected.is_empty()
                                && identity(&path).is_some_and(|id| protected.contains(&id)) =>
                        {
                            Some(OVERLAP)
                        }
                        Ok(_)
                            if !fs::symlink_metadata(&path)
                                .is_ok_and(|entry| entry.dev() == source.1.dev()) =>
                        {
                            Some("Il gruppo è inaccessibile o su un altro volume.")
                        }
                        Ok(_) => None,
                    };
                    if let Some(reason) = reason {
                        row.removal_available = false;
                        row.reason = reason;
                    } else {
                        row.target = Some(RemovalTarget {
                            source: source.clone(),
                            path,
                            budget: group_budget,
                        });
                    }
                }
                #[cfg(not(unix))]
                {
                    row.reason = "Eliminazione export non disponibile su questa piattaforma.";
                }
                size.files = size.files.saturating_add(row.size.files);
                size.directories = size.directories.saturating_add(row.size.directories);
                size.logical_bytes = size.logical_bytes.saturating_add(row.size.logical_bytes);
                complete &= row.complete;
                groups.push(row);
            }
            if identity(&root.path).as_ref() != Some(id)
                || !fs::symlink_metadata(&root.path).is_ok_and(|meta| fingerprint(&meta) == before)
            {
                complete = false;
                for row in &mut groups[first..] {
                    row.complete = false;
                    row.removal_available = false;
                    row.reason = CHANGED;
                }
            }
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
        Self {
            groups,
            size,
            complete,
        }
    }
}

fn export_kind(kind: StorageKind) -> bool {
    matches!(
        kind,
        StorageKind::ExportCandidate
            | StorageKind::ArtifactSessionExports
            | StorageKind::ExternalExports
    )
}

#[cfg(unix)]
fn removal_scope(storage: &[Storage], root: &Storage) -> Result<HashSet<String>, &'static str> {
    if root.kind == StorageKind::ArtifactSessionExports && !root.recognized {
        return Err("La root Artifact Session non è verificata.");
    }
    let others = storage
        .iter()
        .filter(|other| other.observed && !std::ptr::eq(*other, root));
    if root.kind == StorageKind::ExternalExports {
        let ancestors: HashSet<_> = root.path.ancestors().filter_map(identity).collect();
        if others.clone().any(|other| match other.kind {
            StorageKind::Home => root.identity == other.identity,
            StorageKind::UserOutput => false,
            _ => other
                .identity
                .as_ref()
                .is_some_and(|id| ancestors.contains(id)),
        }) {
            return Err(OVERLAP);
        }
    }

    // Find first-level groups containing another storage root by identity,
    // including aliases with different casing on case-insensitive volumes.
    let child_depth = root.path.components().count() + 1;
    Ok(others
        .filter_map(|other| {
            let skip = other.path.components().count().checked_sub(child_depth)?;
            let mut paths = other.path.ancestors().skip(skip);
            let group = paths.next()?;
            if identity(paths.next()?) != root.identity {
                return None;
            }
            identity(group)
        })
        .collect())
}

impl Group {
    /// Re-observe only this group and remove only the observed entries.
    pub fn remove(&self) -> Result<(), &'static str> {
        if !self.removal_available {
            return Err(self.reason);
        }
        #[cfg(unix)]
        {
            let target = self.target.as_ref().ok_or(CHANGED)?;
            let (root, expected) = target.source.as_ref();
            let dir = open_root(&root.path).map_err(removal_error)?;
            let current = dir.metadata().map_err(removal_error)?;
            if (current.dev(), current.ino()) != (expected.dev(), expected.ino()) {
                return Err(CHANGED);
            }
            let mut entries = Vec::new();
            // Reuse the preview's remaining entries so a truncated scan has the same scope.
            let mut budget = target.budget;
            let current = group(
                root,
                &target.path,
                &mut budget,
                &mut Allocation::default(),
                Some(&mut entries),
            );
            if current.key != self.key || identity(&root.path) != root.identity {
                return Err(CHANGED);
            }
            remove_observed(&dir, &mut entries.into_iter().peekable()).map_err(removal_error)
        }
        #[cfg(not(unix))]
        Err(self.reason)
    }
}

#[cfg(unix)]
const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

#[cfg(unix)]
fn open_root(path: &Path) -> io::Result<fs::File> {
    let mut parts = path.components();
    if parts.next() != Some(Component::RootDir) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut dir = scoped::open("/", DIRECTORY, Mode::empty())?;
    for part in parts {
        let Component::Normal(name) = part else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        dir = scoped::openat(dir, name, DIRECTORY, Mode::empty())?;
    }
    Ok(dir.into())
}

#[cfg(unix)]
fn same_object(current: &scoped::Stat, expected: &fs::Metadata) -> bool {
    i128::from(current.st_dev) == i128::from(expected.dev()) && current.st_ino == expected.ino()
}

#[cfg(unix)]
fn remove_observed(
    parent: &fs::File,
    entries: &mut std::iter::Peekable<std::vec::IntoIter<(PathBuf, fs::Metadata)>>,
) -> io::Result<()> {
    let (path, expected) = entries.next().ok_or(io::ErrorKind::InvalidData)?;
    let name = path.file_name().ok_or(io::ErrorKind::InvalidData)?;
    let changed = io::ErrorKind::InvalidData;
    if expected.is_dir() {
        // Reject bind mounts as well as symlinks when descending on Linux.
        #[cfg(target_os = "linux")]
        let fd = scoped::openat2(
            parent,
            name,
            DIRECTORY,
            Mode::empty(),
            scoped::ResolveFlags::NO_XDEV,
        )?;
        #[cfg(not(target_os = "linux"))]
        let fd = scoped::openat(parent, name, DIRECTORY, Mode::empty())?;
        let dir = fs::File::from(fd);
        if fingerprint(&dir.metadata()?) != fingerprint(&expected) {
            return Err(changed.into());
        }
        while entries
            .peek()
            .is_some_and(|(child, _)| child.parent() == Some(&path))
        {
            remove_observed(&dir, entries)?;
        }
        let current = scoped::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if !same_object(&current, &expected) {
            return Err(changed.into());
        }
        scoped::unlinkat(parent, name, AtFlags::REMOVEDIR)?;
    } else {
        let current = scoped::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)?;
        if !same_object(&current, &expected)
            || i128::from(current.st_size) != i128::from(expected.len())
            || i128::from(current.st_mtime) != i128::from(expected.mtime())
            || i128::from(current.st_mtime_nsec) != i128::from(expected.mtime_nsec())
        {
            return Err(changed.into());
        }
        // Unlink the directory entry itself, including a symlink or a hardlink name.
        scoped::unlinkat(parent, name, AtFlags::empty())?;
    }
    Ok(())
}

#[cfg(unix)]
fn removal_error(error: io::Error) -> &'static str {
    match error.kind() {
        io::ErrorKind::PermissionDenied => "Permesso negato durante l’eliminazione.",
        io::ErrorKind::NotFound | io::ErrorKind::InvalidData | io::ErrorKind::DirectoryNotEmpty => {
            CHANGED
        }
        _ => "Il filesystem ha impedito l’eliminazione.",
    }
}
