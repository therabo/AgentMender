//! Fixed, lossless reducers for reviewed Desktop shared projections.
//!
//! These functions do not grant authority or perform filesystem effects. Callers
//! must bind observations to the approved manifest and hold the transaction's
//! complete writer and filesystem fences before publishing anything.

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::Value;
use std::fmt;

const MAX_INDEX_BYTES: usize = 64 * 1024 * 1024;
const MAX_INDEX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SharedReductionError {
    TooLarge,
    UnsafeTargetRow,
}

/// An index row retains all top-level entries, including duplicate keys. This
/// avoids serde_json's normal last-key-wins interpretation for target rows.
struct Row(Vec<(String, Value)>);
impl<'de> Deserialize<'de> for Row {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RowVisitor;
        impl<'de> Visitor<'de> for RowVisitor {
            type Value = Row;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a session index object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Row, M::Error> {
                let mut fields = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    fields.push((key, value));
                }
                Ok(Row(fields))
            }
        }
        deserializer.deserialize_map(RowVisitor)
    }
}

/// Remove all exact, structurally valid target rows while preserving every byte
/// of every other line, including malformed lines and their original delimiters.
/// A parseable object which claims the target but has extra, duplicate, missing,
/// or non-string fields is ambiguous and cannot be reduced.
pub(crate) fn reduce_index(
    bytes: &[u8],
    target: &str,
) -> Result<(Vec<u8>, u64), SharedReductionError> {
    if bytes.len() > MAX_INDEX_BYTES {
        return Err(SharedReductionError::TooLarge);
    }
    let mut output = Vec::with_capacity(bytes.len());
    let mut count = 0_u64;
    let target_uuid = uuid::Uuid::try_parse(target).ok();
    for line in bytes.split_inclusive(|b| *b == b'\n') {
        if line.len() > MAX_INDEX_LINE_BYTES {
            return Err(SharedReductionError::TooLarge);
        }
        let body = line.strip_suffix(b"\n").unwrap_or(line);
        let body = body.strip_suffix(b"\r").unwrap_or(body);
        // The provider trims Unicode whitespace before parsing each line.
        let row = std::str::from_utf8(body)
            .ok()
            .and_then(|line| serde_json::from_str::<Row>(line.trim()).ok());
        let Some(Row(fields)) = row else {
            output.extend_from_slice(line);
            continue;
        };
        let mut claims_target = false;
        for (_, value) in fields.iter().filter(|(key, _)| key == "id") {
            let Some(id) = value.as_str() else { continue };
            if id == target {
                claims_target = true;
            } else if target_uuid.is_some() && uuid::Uuid::try_parse(id).ok() == target_uuid {
                // The provider also resolves noncanonical UUID text. Treat it
                // as unresolved evidence without widening the reducer's edits.
                return Err(SharedReductionError::UnsafeTargetRow);
            }
        }
        if !claims_target {
            output.extend_from_slice(line);
            continue;
        }
        let valid = fields.len() == 3
            && ["id", "thread_name", "updated_at"].iter().all(|name| {
                let mut values = fields.iter().filter(|(key, _)| key == name);
                values.next().is_some_and(|(_, value)| value.is_string()) && values.next().is_none()
            });
        if !valid {
            return Err(SharedReductionError::UnsafeTargetRow);
        }
        count += 1;
    }
    Ok((output, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_every_exact_row_and_preserves_other_bytes() {
        let input = b"{\"id\":\"T\",\"thread_name\":\"one\",\"updated_at\":\"now\"}\r\nmalformed\xff\n{ \"id\" : \"U\", \"x\":1 }\r\n{\"updated_at\":\"then\",\"id\":\"T\",\"thread_name\":\"two\"}";
        let (retained, count) = reduce_index(input, "T").unwrap();
        assert_eq!(count, 2);
        assert_eq!(retained, b"malformed\xff\n{ \"id\" : \"U\", \"x\":1 }\r\n");
    }

    #[test]
    fn rejects_ambiguous_target_objects() {
        for line in [
            r#"{"id":"T","id":"U","thread_name":"a","updated_at":"b"}"#,
            r#"{"id":"T","thread_name":"a","updated_at":"b","extra":0}"#,
            r#"{"id":"T","thread_name":null,"updated_at":"b"}"#,
        ] {
            assert_eq!(
                reduce_index(line.as_bytes(), "T"),
                Err(SharedReductionError::UnsafeTargetRow)
            );
        }
    }

    #[test]
    fn preserves_similar_and_unrelated_rows() {
        let input = b"{\"id\":\"TT\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\n{}";
        assert_eq!(reduce_index(input, "T").unwrap(), (input.to_vec(), 0));
    }

    #[test]
    fn refuses_provider_equivalent_noncanonical_ids() {
        let target = "12345678-1234-1234-1234-123456789abc";
        let peer = br#"{"id":"12345678-1234-1234-1234-123456789ABD","thread_name":"peer","updated_at":"now"}"#;
        assert_eq!(reduce_index(peer, target).unwrap(), (peer.to_vec(), 0));
        for id in [
            "12345678-1234-1234-1234-123456789ABC",
            "12345678123412341234123456789abc",
            "urn:uuid:12345678-1234-1234-1234-123456789abc",
            "{12345678-1234-1234-1234-123456789abc}",
        ] {
            let row = format!(r#"{{"id":"{id}","thread_name":"target","updated_at":"now"}}"#);
            assert_eq!(
                reduce_index(row.as_bytes(), target),
                Err(SharedReductionError::UnsafeTargetRow)
            );
        }
    }
}

use super::desktop_json::{Node, parse, remove_runs};

fn remove_array(
    node: &Node,
    bytes: &[u8],
    wanted: &str,
    edits: &mut Vec<(usize, usize)>,
) -> Result<u64, SharedReductionError> {
    remove_array_values(node, bytes, &[wanted], edits)
}
fn remove_array_values(
    node: &Node,
    bytes: &[u8],
    wanted: &[&str],
    edits: &mut Vec<(usize, usize)>,
) -> Result<u64, SharedReductionError> {
    let items = node
        .array()
        .map_err(|_| SharedReductionError::UnsafeTargetRow)?;
    let mut selected = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .string(bytes)
            .ok_or(SharedReductionError::UnsafeTargetRow)?;
        selected.push(wanted.contains(&value.as_str()));
    }
    let count = selected.iter().filter(|v| **v).count() as u64;
    let spans: Vec<_> = items.iter().map(|n| (n.start, n.end)).collect();
    remove_runs(&spans, &selected, edits);
    Ok(count)
}
fn remove_key(
    node: &Node,
    key: &str,
    edits: &mut Vec<(usize, usize)>,
) -> Result<u64, SharedReductionError> {
    remove_keys(node, &[key], edits)
}
fn remove_keys(
    node: &Node,
    keys: &[&str],
    edits: &mut Vec<(usize, usize)>,
) -> Result<u64, SharedReductionError> {
    let fields = node
        .object()
        .map_err(|_| SharedReductionError::UnsafeTargetRow)?;
    let selected: Vec<_> = fields
        .iter()
        .map(|(k, _, _)| keys.contains(&k.as_str()))
        .collect();
    let spans: Vec<_> = fields.iter().map(|(_, s, n)| (*s, n.end)).collect();
    let count = selected.iter().filter(|v| **v).count() as u64;
    remove_runs(&spans, &selected, edits);
    Ok(count)
}
fn field<'a>(node: &'a Node, key: &str) -> Result<Option<&'a Node>, SharedReductionError> {
    node.field(key)
        .map_err(|_| SharedReductionError::UnsafeTargetRow)
}
fn local_array(
    node: &Node,
    bytes: &[u8],
    target: &str,
    allowed: bool,
    edits: &mut Vec<(usize, usize)>,
) -> Result<u64, SharedReductionError> {
    let mut pending = Vec::new();
    let count = remove_array(node, bytes, target, &mut pending)?;
    if count > 0 && !allowed {
        return Err(SharedReductionError::UnsafeTargetRow);
    }
    edits.extend(pending);
    Ok(count)
}

fn foreign_reference(
    bytes: &[u8],
    target: &str,
    migration_scope: &str,
) -> Result<bool, SharedReductionError> {
    let root = parse(bytes).map_err(|_| SharedReductionError::UnsafeTargetRow)?;
    let atoms = field(&root, "electron-persisted-atom-state")?;
    let unread = match atoms {
        Some(atoms) => field(atoms, "unread-thread-ids-by-host-v1")?,
        None => None,
    };
    let migrated = field(&root, "app-server-migrated-pinned-thread-ids-by-host")?;
    for (map, local) in [(migrated, migration_scope), (unread, "local")] {
        let Some(map) = map else { continue };
        for (host, _, ids) in map
            .object()
            .map_err(|_| SharedReductionError::UnsafeTargetRow)?
        {
            if host != local
                && ids
                    .array()
                    .map_err(|_| SharedReductionError::UnsafeTargetRow)?
                    .iter()
                    .any(|id| id.string(bytes).as_deref() == Some(target))
            {
                return Ok(true);
            }
        }
    }
    if let Some(atoms) = atoms {
        if let Some(tabs) = field(atoms, "pinned-content-tabs-v1")? {
            for (_, _, tab) in tabs
                .object()
                .map_err(|_| SharedReductionError::UnsafeTargetRow)?
            {
                if field(tab, "threadId")?
                    .and_then(|n| n.string(bytes))
                    .as_deref()
                    == Some(target)
                    && field(tab, "hostId")?
                        .and_then(|n| n.string(bytes))
                        .as_deref()
                        != Some("local")
                {
                    return Ok(true);
                }
            }
        }
        if let Some(accounts) = field(atoms, "sidebar-custom-sections-v3")? {
            for (_, _, account) in accounts
                .object()
                .map_err(|_| SharedReductionError::UnsafeTargetRow)?
            {
                if let Some(hosts) = field(account, "threadHostIds")?
                    && let Some(host) = field(hosts, target)?
                    && host.string(bytes).as_deref() != Some("local")
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Fixed reducer for the local Desktop global-state contract. Unknown values
/// are retained verbatim. Some complex atom variants remain explicitly
/// unsupported, so a caller cannot mistake an unreviewed shape for absence.
pub(crate) fn reduce_global(
    bytes: &[u8],
    target: &str,
    local_host: &str,
    ownership_proven: bool,
) -> Result<(Vec<u8>, u64), SharedReductionError> {
    let root = parse(bytes).map_err(|_| SharedReductionError::UnsafeTargetRow)?;
    let mut edits = Vec::new();
    let mut count = 0;
    // The provider's update-resume state is a top-level global key.
    // Its target semantics have not yet been admitted for this profile.
    if field(&root, "update-resume-state-v1")?.is_some() {
        return Err(SharedReductionError::UnsafeTargetRow);
    }
    for key in ["pinned-thread-ids", "projectless-thread-ids"] {
        if let Some(n) = field(&root, key)? {
            count += local_array(n, bytes, target, ownership_proven, &mut edits)?;
        }
    }
    if let Some(hosts) = field(&root, "app-server-migrated-pinned-thread-ids-by-host")?
        && let Some(n) = field(hosts, local_host)?
    {
        count += remove_array(n, bytes, target, &mut edits)?;
    }
    if let Some(hosts) = field(&root, "thread-project-membership-host-ids")?
        && let Some(n) = field(hosts, target)?
        && n.string(bytes).as_deref() != Some("local")
    {
        return Err(SharedReductionError::UnsafeTargetRow);
    }
    for key in [
        "thread-project-assignments",
        "thread-project-membership-host-ids",
        "thread-writable-roots",
        "thread-workspace-root-hints",
        "thread-projectless-output-directories",
    ] {
        if let Some(map) = field(&root, key)?
            && let Some(value) = field(map, target)?
        {
            if key == "thread-project-assignments" {
                let kind = field(value, "projectKind")?.and_then(|n| n.string(bytes));
                let project = field(value, "projectId")?.and_then(|n| n.string(bytes));
                if kind.as_deref() != Some("local") || project.is_none() {
                    return Err(SharedReductionError::UnsafeTargetRow);
                }
            }
            if !ownership_proven {
                return Err(SharedReductionError::UnsafeTargetRow);
            }
            count += remove_key(map, target, &mut edits)?;
        }
    }
    if let Some(map) = field(&root, "sidebar-thread-metadata")?
        && let Some(record) = field(map, target)?
        && field(record, "labelColor")?.is_some()
    {
        if !ownership_proven {
            return Err(SharedReductionError::UnsafeTargetRow);
        }
        count += remove_key(record, "labelColor", &mut edits)?;
    }
    if let Some(projects) = field(&root, "sidebar-project-thread-orders")? {
        for (_, _, project) in projects
            .object()
            .map_err(|_| SharedReductionError::UnsafeTargetRow)?
        {
            if let Some(ids) = field(project, "threadIds")? {
                let mut pending = Vec::new();
                let qualified = format!("codex:thread:local:{target}");
                let removed = remove_array_values(ids, bytes, &[target, &qualified], &mut pending)?;
                if removed > 0 && !ownership_proven {
                    return Err(SharedReductionError::UnsafeTargetRow);
                }
                count += removed;
                edits.extend(pending);
            }
        }
    }
    if let Some(atoms) = field(&root, "electron-persisted-atom-state")? {
        if field(atoms, "update-resume-state-v1")?.is_some() {
            return Err(SharedReductionError::UnsafeTargetRow);
        }
        let qualified = format!("codex:thread:local:{target}");
        let mut tab_ids = Vec::new();
        if let Some(tabs) = field(atoms, "pinned-content-tabs-v1")? {
            for (key, _, tab) in tabs
                .object()
                .map_err(|_| SharedReductionError::UnsafeTargetRow)?
            {
                let thread = field(tab, "threadId")?.and_then(|n| n.string(bytes));
                if thread.as_deref() != Some(target) {
                    continue;
                }
                let host = field(tab, "hostId")?
                    .and_then(|n| n.string(bytes))
                    .ok_or(SharedReductionError::UnsafeTargetRow)?;
                if host != "local" {
                    continue;
                }
                let id = field(tab, "id")?.and_then(|n| n.string(bytes));
                let account =
                    field(tab, "accountId")?.ok_or(SharedReductionError::UnsafeTargetRow)?;
                if id.as_deref() != Some(key.as_str())
                    || &bytes[account.start..account.end] != b"null"
                {
                    return Err(SharedReductionError::UnsafeTargetRow);
                }
                for name in [
                    "clientThreadId",
                    "tabId",
                    "tabKind",
                    "title",
                    "routePath",
                    "routeSearch",
                ] {
                    if field(tab, name)?.and_then(|n| n.string(bytes)).is_none() {
                        return Err(SharedReductionError::UnsafeTargetRow);
                    }
                }
                let order = field(tab, "order")?.ok_or(SharedReductionError::UnsafeTargetRow)?;
                if !serde_json::from_slice::<serde_json::Value>(&bytes[order.start..order.end])
                    .map_err(|_| SharedReductionError::UnsafeTargetRow)?
                    .is_number()
                {
                    return Err(SharedReductionError::UnsafeTargetRow);
                }
                tab_ids.push(key.as_str());
            }
            count += remove_keys(tabs, &tab_ids, &mut edits)?;
        }
        if !tab_ids.is_empty()
            && let Some(presentation) = field(atoms, "pinned-content-tab-presentation-v1")?
        {
            count += remove_keys(presentation, &tab_ids, &mut edits)?;
        }
        let mut owned_orders = vec![qualified];
        owned_orders.extend(tab_ids.iter().map(|id| format!("content-tab:{id}")));
        let order_refs: Vec<_> = owned_orders.iter().map(String::as_str).collect();
        for key in [
            "unified-sidebar-chat-order-v1",
            "unified-sidebar-pinned-order-v1",
            "codex-sidebar-chat-order-v1",
        ] {
            if let Some(n) = field(atoms, key)? {
                count += remove_array_values(n, bytes, &order_refs, &mut edits)?;
            }
        }
        let qualified = &owned_orders[0];
        if let Some(n) = field(atoms, "app-server-pinned-thread-order-v1")? {
            count += local_array(n, bytes, target, ownership_proven, &mut edits)?;
        }
        if let Some(hosts) = field(atoms, "unread-thread-ids-by-host-v1")?
            && let Some(n) = field(hosts, "local")?
        {
            count += remove_array(n, bytes, target, &mut edits)?;
        }
        if let Some(accounts) = field(atoms, "sidebar-custom-sections-v3")? {
            for (account_id, _, account) in accounts
                .object()
                .map_err(|_| SharedReductionError::UnsafeTargetRow)?
            {
                let Some(sections) = field(account, "sections")? else {
                    return Err(SharedReductionError::UnsafeTargetRow);
                };
                let mut section_edits = Vec::new();
                let mut removed = 0;
                for section in sections
                    .array()
                    .map_err(|_| SharedReductionError::UnsafeTargetRow)?
                {
                    let keys =
                        field(section, "itemKeys")?.ok_or(SharedReductionError::UnsafeTargetRow)?;
                    let section_count = remove_array(keys, bytes, qualified, &mut section_edits)?;
                    if let Some(pending) = field(section, "pendingDeletionHostIdsByItemKey")?
                        && let Some(hosts) = field(pending, qualified)?
                    {
                        let hosts = hosts
                            .array()
                            .map_err(|_| SharedReductionError::UnsafeTargetRow)?;
                        if section_count == 0
                            || hosts
                                .iter()
                                .any(|host| host.string(bytes).as_deref() != Some("local"))
                        {
                            return Err(SharedReductionError::UnsafeTargetRow);
                        }
                        removed += remove_key(pending, qualified, &mut section_edits)?;
                    }
                    removed += section_count;
                }
                let hosts = field(account, "threadHostIds")?;
                let host = match hosts {
                    Some(map) => field(map, target)?.and_then(|n| n.string(bytes)),
                    None => None,
                };
                if removed > 0 && host.as_deref() != Some("local") {
                    return Err(SharedReductionError::UnsafeTargetRow);
                }
                if removed > 0 || (host.as_deref() == Some("local") && ownership_proven) {
                    // The candidate is the accountless local profile. A target
                    // reference in another principal must be resolved separately.
                    if !account_id.is_empty() {
                        return Err(SharedReductionError::UnsafeTargetRow);
                    }
                    let hosts = hosts.ok_or(SharedReductionError::UnsafeTargetRow)?;
                    edits.extend(section_edits);
                    count += removed;
                    count += remove_key(hosts, target, &mut edits)?;
                }
            }
        }
        // Do not silently ignore reconstruction-capable composite atoms whose
        // exact target semantics are not yet established.
        if field(atoms, "update-resume-v1")?.is_some() {
            return Err(SharedReductionError::UnsafeTargetRow);
        }
    }
    edits.sort_unstable();
    let mut output = Vec::with_capacity(bytes.len());
    let mut pos = 0;
    for (start, end) in edits {
        if start < pos || end > bytes.len() {
            return Err(SharedReductionError::UnsafeTargetRow);
        }
        output.extend_from_slice(&bytes[pos..start]);
        pos = end;
    }
    output.extend_from_slice(&bytes[pos..]);
    // Ensure the span edits left a valid document.
    let _: serde_json::Value =
        serde_json::from_slice(&output).map_err(|_| SharedReductionError::UnsafeTargetRow)?;
    Ok((output, count))
}

#[cfg(unix)]
mod files {
    use super::{foreign_reference, reduce_global, reduce_index};
    use agentmender_domain::{CleanupFileIdentity, DesktopFileCommitment};
    use agentmender_persistence::EffectError;
    use rustix::fs::{self, AtFlags, FlockOperation, Mode, OFlags};
    use sha2::{Digest, Sha256};
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom, Write},
        os::{fd::OwnedFd, unix::fs::MetadataExt},
        path::{Component, Path},
        sync::atomic::{AtomicU64, Ordering},
    };

    static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    #[cfg(test)]
    thread_local! {
        static BEFORE_FINAL_COMPARE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = std::cell::RefCell::new(None);
    }

    #[derive(Clone, Copy)]
    pub(crate) enum GlobalKind {
        Main,
        Backup,
    }
    impl GlobalKind {
        fn name(self) -> &'static str {
            match self {
                Self::Main => ".codex-global-state.json",
                Self::Backup => ".codex-global-state.json.bak",
            }
        }
    }
    fn bad<T>() -> Result<T, EffectError> {
        Err(EffectError::InvalidEvidence)
    }
    fn root(identity: &CleanupFileIdentity) -> Result<OwnedFd, EffectError> {
        if !identity.path.is_absolute() {
            return bad();
        }
        let mut fd = fs::open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| EffectError::InvalidEvidence)?;
        for part in identity.path.components() {
            match part {
                Component::RootDir => (),
                Component::Normal(name) => {
                    fd = fs::openat(
                        &fd,
                        name,
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(|_| EffectError::InvalidEvidence)?
                }
                _ => return bad(),
            }
        }
        let check = File::from(
            fs::openat(
                &fd,
                ".",
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| EffectError::InvalidEvidence)?,
        );
        let m = check.metadata().map_err(|_| EffectError::InvalidEvidence)?;
        if m.dev() != identity.device || m.ino() != identity.inode {
            return bad();
        }
        Ok(fd)
    }
    fn digest(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    #[derive(PartialEq, Eq)]
    struct FileMetadata {
        mode: u32,
        uid: u32,
        gid: u32,
        provenance: Option<Vec<u8>>,
    }
    type FileRead = (CleanupFileIdentity, Vec<u8>, FileMetadata, (i64, i64));
    impl FileMetadata {
        fn observe(file: &File, m: &std::fs::Metadata) -> Result<Self, EffectError> {
            let provenance =
                agentmender_provider_api::filesystem::plain_replacement_provenance(file)
                    .map_err(|_| EffectError::InvalidEvidence)?;
            Ok(Self {
                mode: m.mode(),
                uid: m.uid(),
                gid: m.gid(),
                provenance,
            })
        }

        fn digest(&self) -> String {
            let mut bytes = Vec::with_capacity(76);
            bytes.extend(self.mode.to_le_bytes());
            bytes.extend(self.uid.to_le_bytes());
            bytes.extend(self.gid.to_le_bytes());
            if let Some(value) = &self.provenance {
                bytes.extend(value);
            }
            digest(&bytes)
        }

        fn preserve_provenance(&self, stage: &File, current: &Self) -> Result<(), EffectError> {
            if self.provenance == current.provenance {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            {
                const NAME: &str = "com.apple.provenance";
                match &self.provenance {
                    Some(value) => fs::fsetxattr(stage, NAME, value, fs::XattrFlags::empty()),
                    None => fs::fremovexattr(stage, NAME),
                }
                .map_err(|_| EffectError::InvalidEvidence)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = stage;
                bad()
            }
        }
    }

    fn read(fd: &OwnedFd, rootpath: &Path, name: &str) -> Result<Option<FileRead>, EffectError> {
        let opened = match fs::openat(
            fd,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(v) => v,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(_) => return bad(),
        };
        let mut file = File::from(opened);
        let m = file.metadata().map_err(|_| EffectError::InvalidEvidence)?;
        let metadata = FileMetadata::observe(&file, &m)?;
        if !m.is_file() || m.nlink() != 1 || m.len() > 64 * 1024 * 1024 {
            return bad();
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| EffectError::InvalidEvidence)?;
        let after = file.metadata().map_err(|_| EffectError::InvalidEvidence)?;
        if m.dev() != after.dev()
            || m.ino() != after.ino()
            || m.len() != after.len()
            || m.mtime() != after.mtime()
            || m.mtime_nsec() != after.mtime_nsec()
            || m.ctime() != after.ctime()
            || m.ctime_nsec() != after.ctime_nsec()
            || bytes.len() as u64 != m.len()
        {
            return bad();
        }
        Ok(Some((
            CleanupFileIdentity {
                path: rootpath.join(name),
                device: m.dev(),
                inode: m.ino(),
                links: m.nlink(),
                size_bytes: m.len(),
                mtime_seconds: m.mtime(),
                mtime_nanoseconds: m.mtime_nsec() as u32,
            },
            bytes,
            metadata,
            (m.ctime(), m.ctime_nsec()),
        )))
    }
    type Reducer<'a> = dyn Fn(&[u8]) -> Result<(Vec<u8>, u64), super::SharedReductionError> + 'a;
    fn observation(
        fd: &OwnedFd,
        rootpath: &Path,
        name: &str,
        reducer: &Reducer<'_>,
    ) -> Result<(DesktopFileCommitment, Option<Vec<u8>>), EffectError> {
        commitment(read(fd, rootpath, name)?, reducer)
    }
    fn commitment(
        file: Option<FileRead>,
        reducer: &Reducer<'_>,
    ) -> Result<(DesktopFileCommitment, Option<Vec<u8>>), EffectError> {
        let Some((identity, bytes, metadata, _)) = file else {
            return Ok((
                DesktopFileCommitment {
                    identity: None,
                    preimage_sha256: None,
                    retained_sha256: None,
                    metadata_sha256: None,
                    target_units: 0,
                },
                None,
            ));
        };
        let (retained, target_units) = reducer(&bytes).map_err(|_| EffectError::InvalidEvidence)?;
        Ok((
            DesktopFileCommitment {
                identity: Some(identity),
                preimage_sha256: Some(digest(&bytes)),
                retained_sha256: Some(digest(&retained)),
                metadata_sha256: Some(metadata.digest()),
                target_units,
            },
            Some(retained),
        ))
    }
    pub(crate) fn observe_globals(
        root_id: &CleanupFileIdentity,
        target: &str,
        host: &str,
        ownership: bool,
    ) -> Result<[DesktopFileCommitment; 2], EffectError> {
        let fd = root(root_id)?;
        let backup = read(&fd, &root_id.path, GlobalKind::Backup.name())?;
        let main = read(&fd, &root_id.path, GlobalKind::Main.name())?;
        let mut bare_owned = ownership;
        for (_, bytes, _, _) in [&backup, &main].into_iter().flatten() {
            if foreign_reference(bytes, target, host).map_err(|_| EffectError::InvalidEvidence)? {
                bare_owned = false;
            }
        }
        let reducer = |bytes: &[u8]| reduce_global(bytes, target, host, bare_owned);
        Ok([
            commitment(backup, &reducer)?.0,
            commitment(main, &reducer)?.0,
        ])
    }
    pub(crate) fn observe_index(
        root_id: &CleanupFileIdentity,
        target: &str,
    ) -> Result<DesktopFileCommitment, EffectError> {
        let fd = root(root_id)?;
        observation(&fd, &root_id.path, "session_index.jsonl", &|b| {
            reduce_index(b, target)
        })
        .map(|v| v.0)
    }
    pub(crate) struct SharedFence {
        fd: OwnedFd,
        lock: File,
        root: CleanupFileIdentity,
    }
    impl SharedFence {
        pub(crate) fn acquire(
            execution: &crate::desktop_cleanup::DesktopExecution<'_>,
        ) -> Result<Self, EffectError> {
            let root_id = &execution.manifest().primary.root;
            if execution.manifest().desktop.is_none() {
                return bad();
            }
            let fd = root(root_id)?;
            let lock = File::from(
                fs::openat(
                    &fd,
                    "session_index.jsonl.lock",
                    OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::from_raw_mode(0o600),
                )
                .map_err(|_| EffectError::Unavailable)?,
            );
            let m = lock.metadata().map_err(|_| EffectError::InvalidEvidence)?;
            if !m.is_file() || m.nlink() != 1 || m.dev() != root_id.device {
                return bad();
            }
            fs::flock(&lock, FlockOperation::NonBlockingLockExclusive)
                .map_err(|_| EffectError::Unavailable)?;
            fs::fsync(&fd).map_err(|_| EffectError::Transport)?;
            let result = Self {
                fd,
                lock,
                root: root_id.clone(),
            };
            result.validate()?;
            Ok(result)
        }
        // These stat fields have different native widths on Darwin and Linux.
        #[allow(clippy::unnecessary_cast)]
        pub(crate) fn validate(&self) -> Result<(), EffectError> {
            let m = self
                .lock
                .metadata()
                .map_err(|_| EffectError::InvalidEvidence)?;
            let s = fs::statat(
                &self.fd,
                "session_index.jsonl.lock",
                AtFlags::SYMLINK_NOFOLLOW,
            )
            .map_err(|_| EffectError::InvalidEvidence)?;
            if m.nlink() != 1
                || s.st_nlink as u64 != 1
                || m.dev() != s.st_dev as u64
                || m.ino() != s.st_ino as u64
            {
                return bad();
            }
            let reopened = root(&self.root)?;
            let rm = File::from(reopened)
                .metadata()
                .map_err(|_| EffectError::InvalidEvidence)?;
            if rm.dev() != self.root.device || rm.ino() != self.root.inode {
                return bad();
            }
            Ok(())
        }
    }
    #[allow(clippy::unnecessary_cast)]
    fn publish(
        fence: &SharedFence,
        name: &str,
        expected: &DesktopFileCommitment,
        reducer: &Reducer<'_>,
    ) -> Result<DesktopFileCommitment, EffectError> {
        fence.validate()?;
        let (current, retained) = observation(&fence.fd, &fence.root.path, name, reducer)?;
        if &current != expected {
            return bad();
        }
        if current.target_units == 0 {
            return Ok(current);
        }
        let retained = retained.ok_or(EffectError::InvalidEvidence)?;
        let (_, _, metadata, _) =
            read(&fence.fd, &fence.root.path, name)?.ok_or(EffectError::InvalidEvidence)?;
        if Some(metadata.digest()) != expected.metadata_sha256 {
            return bad();
        }
        let stage_name = format!(
            ".agentmender-stage-{}-{}",
            std::process::id(),
            STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let stage_fd = fs::openat(
            &fence.fd,
            stage_name.as_str(),
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|_| EffectError::Unavailable)?;
        let mut stage = File::from(stage_fd);
        let outcome = (|| {
            let meta = stage.metadata().map_err(|_| EffectError::Transport)?;
            let stage_metadata = FileMetadata::observe(&stage, &meta)?;
            if meta.uid() != metadata.uid
                || meta.gid() != metadata.gid
                || meta.dev() != fence.root.device
                || meta.nlink() != 1
            {
                return bad();
            }
            fs::fchmod(&stage, Mode::from_raw_mode((metadata.mode & 0o7777) as _))
                .map_err(|_| EffectError::Transport)?;
            stage
                .write_all(&retained)
                .map_err(|_| EffectError::Transport)?;
            metadata.preserve_provenance(&stage, &stage_metadata)?;
            stage.sync_all().map_err(|_| EffectError::Transport)?;
            let staged = stage.metadata().map_err(|_| EffectError::InvalidEvidence)?;
            stage
                .seek(SeekFrom::Start(0))
                .map_err(|_| EffectError::Transport)?;
            let mut chunk = [0; 65536];
            for expected in retained.chunks(chunk.len()) {
                let actual = &mut chunk[..expected.len()];
                stage
                    .read_exact(actual)
                    .map_err(|_| EffectError::Transport)?;
                if actual != expected {
                    return bad();
                }
            }
            fence.validate()?;
            #[cfg(test)]
            BEFORE_FINAL_COMPARE.with(|hook| {
                if let Some(f) = hook.borrow_mut().take() {
                    f();
                }
            });
            let (source, fresh, fresh_metadata, ctime) =
                read(&fence.fd, &fence.root.path, name)?.ok_or(EffectError::InvalidEvidence)?;
            if Some(&source) != expected.identity.as_ref()
                || Some(digest(&fresh)) != expected.preimage_sha256
                || fresh_metadata != metadata
            {
                return bad();
            }
            let stage_meta = stage.metadata().map_err(|_| EffectError::InvalidEvidence)?;
            let stage_path = fs::statat(&fence.fd, stage_name.as_str(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| EffectError::InvalidEvidence)?;
            if !stage_meta.is_file()
                || stage_meta.nlink() != 1
                || stage_path.st_nlink as u64 != 1
                || stage_meta.dev() != stage_path.st_dev as u64
                || stage_meta.ino() != stage_path.st_ino as u64
                || stage_meta.len() != retained.len() as u64
                || (stage_meta.ctime(), stage_meta.ctime_nsec())
                    != (staged.ctime(), staged.ctime_nsec())
                || (stage_path.st_ctime as i64, stage_path.st_ctime_nsec as i64)
                    != (staged.ctime(), staged.ctime_nsec())
                || FileMetadata::observe(&stage, &stage_meta)? != metadata
            {
                return bad();
            }
            let source_now = fs::statat(&fence.fd, name, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| EffectError::InvalidEvidence)?;
            if source_now.st_dev as u64 != source.device
                || source_now.st_ino as u64 != source.inode
                || source_now.st_nlink as u64 != source.links
                || source_now.st_size as u64 != source.size_bytes
                || source_now.st_mtime as i64 != source.mtime_seconds
                || source_now.st_mtime_nsec as u32 != source.mtime_nanoseconds
                || (source_now.st_ctime as i64, source_now.st_ctime_nsec as i64) != ctime
            {
                return bad();
            }
            fs::renameat(&fence.fd, stage_name.as_str(), &fence.fd, name)
                .map_err(|_| EffectError::Transport)?;
            fs::fsync(&fence.fd).map_err(|_| EffectError::Transport)?;
            let (post_identity, post, post_metadata, _) =
                read(&fence.fd, &fence.root.path, name)?.ok_or(EffectError::InvalidEvidence)?;
            if Some(digest(&post)) != expected.retained_sha256
                || post != retained
                || post_metadata != metadata
                || post_identity.device != stage_meta.dev()
                || post_identity.inode != stage_meta.ino()
            {
                return bad();
            }
            fence.validate()?;
            Ok(DesktopFileCommitment {
                identity: Some(post_identity),
                preimage_sha256: expected.retained_sha256.clone(),
                retained_sha256: expected.retained_sha256.clone(),
                metadata_sha256: expected.metadata_sha256.clone(),
                target_units: 0,
            })
        })();
        if outcome.is_err() {
            // Never unlink a name that no longer denotes our staged inode.
            if let (Ok(meta), Ok(path)) = (
                stage.metadata(),
                fs::statat(&fence.fd, stage_name.as_str(), AtFlags::SYMLINK_NOFOLLOW),
            ) && meta.nlink() == 1
                && path.st_nlink as u64 == 1
                && meta.dev() == path.st_dev as u64
                && meta.ino() == path.st_ino
            {
                let _ = fs::unlinkat(&fence.fd, stage_name.as_str(), AtFlags::empty());
            }
        }
        outcome
    }
    pub(crate) fn publish_global(
        execution: &crate::desktop_cleanup::DesktopExecution<'_>,
        fence: &SharedFence,
        kind: GlobalKind,
        ownership: bool,
    ) -> Result<DesktopFileCommitment, EffectError> {
        let manifest = execution.manifest();
        if fence.root != manifest.primary.root {
            return bad();
        }
        let desktop = manifest
            .desktop
            .as_ref()
            .ok_or(EffectError::InvalidEvidence)?;
        let expected = match kind {
            GlobalKind::Main => &desktop.global_main,
            GlobalKind::Backup => &desktop.global_backup,
        };
        let target = manifest.primary.native_session_id.as_str();
        let host = local_host_scope(&desktop.desktop_root, &desktop.local_host)?;
        publish(fence, kind.name(), expected, &|b| {
            reduce_global(b, target, &host, ownership)
        })
    }
    pub(crate) fn publish_index(
        execution: &crate::desktop_cleanup::DesktopExecution<'_>,
        fence: &SharedFence,
    ) -> Result<DesktopFileCommitment, EffectError> {
        let manifest = execution.manifest();
        if fence.root != manifest.primary.root {
            return bad();
        }
        let desktop = manifest
            .desktop
            .as_ref()
            .ok_or(EffectError::InvalidEvidence)?;
        let target = manifest.primary.native_session_id.as_str();
        publish(fence, "session_index.jsonl", &desktop.session_index, &|b| {
            reduce_index(b, target)
        })
    }
    pub(crate) fn local_host_scope(
        root: &CleanupFileIdentity,
        host: &str,
    ) -> Result<String, EffectError> {
        let path = root.path.to_str().ok_or(EffectError::InvalidEvidence)?;
        Ok(format!("{host}:{path}"))
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::fs::MetadataExt;

        fn fixture() -> (tempfile::TempDir, SharedFence) {
            let dir = tempfile::tempdir().unwrap();
            let m = std::fs::metadata(dir.path()).unwrap();
            let identity = CleanupFileIdentity {
                path: dir.path().canonicalize().unwrap(),
                device: m.dev(),
                inode: m.ino(),
                links: m.nlink(),
                size_bytes: m.len(),
                mtime_seconds: m.mtime(),
                mtime_nanoseconds: m.mtime_nsec() as u32,
            };
            let fd = root(&identity).unwrap();
            let lock = File::from(
                fs::openat(
                    &fd,
                    "session_index.jsonl.lock",
                    OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::from_raw_mode(0o600),
                )
                .unwrap(),
            );
            fs::flock(&lock, FlockOperation::NonBlockingLockExclusive).unwrap();
            (
                dir,
                SharedFence {
                    fd,
                    lock,
                    root: identity,
                },
            )
        }
        #[test]
        fn changed_preimage_is_preserved_and_refused() {
            let (dir, fence) = fixture();
            let path = dir.path().join("session_index.jsonl");
            let first = b"{\"id\":\"T\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\n";
            std::fs::write(&path, first).unwrap();
            let expected = observation(&fence.fd, &fence.root.path, "session_index.jsonl", &|b| {
                reduce_index(b, "T")
            })
            .unwrap()
            .0;
            let concurrent = b"unrelated concurrent bytes\n";
            std::fs::write(&path, concurrent).unwrap();
            assert!(
                publish(&fence, "session_index.jsonl", &expected, &|b| reduce_index(
                    b, "T"
                ))
                .is_err()
            );
            assert_eq!(std::fs::read(path).unwrap(), concurrent);
        }
        #[test]
        fn replaced_root_symlink_refuses_before_publication() {
            use std::os::unix::fs::symlink;
            let parent = tempfile::tempdir().unwrap();
            let path = parent.path().canonicalize().unwrap().join("codex");
            std::fs::create_dir(&path).unwrap();
            let m = std::fs::metadata(&path).unwrap();
            let identity = CleanupFileIdentity {
                path: path.clone(),
                device: m.dev(),
                inode: m.ino(),
                links: m.nlink(),
                size_bytes: m.len(),
                mtime_seconds: m.mtime(),
                mtime_nanoseconds: m.mtime_nsec() as u32,
            };
            let fd = root(&identity).unwrap();
            let lock = File::from(
                fs::openat(
                    &fd,
                    "session_index.jsonl.lock",
                    OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::from_raw_mode(0o600),
                )
                .unwrap(),
            );
            fs::flock(&lock, FlockOperation::NonBlockingLockExclusive).unwrap();
            let fence = SharedFence {
                fd,
                lock,
                root: identity,
            };
            let before = b"{\"id\":\"T\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\n";
            std::fs::write(path.join("session_index.jsonl"), before).unwrap();
            let expected = observation(&fence.fd, &path, "session_index.jsonl", &|b| {
                reduce_index(b, "T")
            })
            .unwrap()
            .0;
            let moved = parent.path().join("moved");
            std::fs::rename(&path, &moved).unwrap();
            symlink(&moved, &path).unwrap();
            assert!(
                publish(&fence, "session_index.jsonl", &expected, &|b| reduce_index(
                    b, "T"
                ))
                .is_err()
            );
            assert_eq!(
                std::fs::read(moved.join("session_index.jsonl")).unwrap(),
                before
            );
        }
        #[test]
        fn change_during_staging_refuses_and_preserves_concurrent_bytes() {
            let (dir, fence) = fixture();
            let path = dir.path().join("session_index.jsonl");
            std::fs::write(
                &path,
                b"{\"id\":\"T\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\n",
            )
            .unwrap();
            let expected = observation(&fence.fd, &fence.root.path, "session_index.jsonl", &|b| {
                reduce_index(b, "T")
            })
            .unwrap()
            .0;
            let changed_path = path.clone();
            BEFORE_FINAL_COMPARE.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    std::fs::write(changed_path, b"concurrent update\n").unwrap()
                }))
            });
            assert!(
                publish(&fence, "session_index.jsonl", &expected, &|b| reduce_index(
                    b, "T"
                ))
                .is_err()
            );
            assert_eq!(std::fs::read(path).unwrap(), b"concurrent update\n");
        }
        #[test]
        fn same_length_stage_edit_cannot_replace_destination() {
            let (dir, fence) = fixture();
            let path = dir.path().join("session_index.jsonl");
            let original = b"{\"id\":\"T\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\nkeep\n";
            std::fs::write(&path, original).unwrap();
            let expected = observation(&fence.fd, &fence.root.path, "session_index.jsonl", &|b| {
                reduce_index(b, "T")
            })
            .unwrap()
            .0;
            let root = dir.path().to_owned();
            BEFORE_FINAL_COMPARE.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    let stage = std::fs::read_dir(root)
                        .unwrap()
                        .map(Result::unwrap)
                        .find(|entry| {
                            entry
                                .file_name()
                                .to_string_lossy()
                                .starts_with(".agentmender-stage-")
                        })
                        .unwrap();
                    std::fs::write(stage.path(), b"edit\n").unwrap();
                }));
            });
            assert!(
                publish(&fence, "session_index.jsonl", &expected, &|b| reduce_index(
                    b, "T"
                ))
                .is_err()
            );
            assert_eq!(std::fs::read(path).unwrap(), original);
        }
        #[test]
        fn independent_global_main_and_backup_publication() {
            let (dir, fence) = fixture();
            let input = br#"{"pinned-thread-ids":["T","U"],"unknown":{"n":1e+09}}"#;
            for name in [".codex-global-state.json", ".codex-global-state.json.bak"] {
                std::fs::write(dir.path().join(name), input).unwrap();
                let expected = observation(&fence.fd, &fence.root.path, name, &|b| {
                    reduce_global(b, "T", "local:/x", true)
                })
                .unwrap()
                .0;
                publish(&fence, name, &expected, &|b| {
                    reduce_global(b, "T", "local:/x", true)
                })
                .unwrap();
                assert_eq!(
                    std::fs::read(dir.path().join(name)).unwrap(),
                    br#"{"pinned-thread-ids":["U"],"unknown":{"n":1e+09}}"#
                );
            }
        }
        #[test]
        fn foreign_reference_in_either_global_copy_blocks_bare_pin_removal() {
            let (dir, fence) = fixture();
            let main = dir.path().join(GlobalKind::Main.name());
            let backup = dir.path().join(GlobalKind::Backup.name());
            let bare = br#"{"pinned-thread-ids":["T","U"]}"#;
            for foreign in [
                br#"{"app-server-migrated-pinned-thread-ids-by-host":{"remote:/x":["T"]}}"#.as_slice(),
                br#"{"electron-persisted-atom-state":{"unread-thread-ids-by-host-v1":{"remote":["T"]}}}"#,
                br#"{"electron-persisted-atom-state":{"pinned-content-tabs-v1":{"a":{"threadId":"T","hostId":"remote"}}}}"#,
            ] {
                for (main_data, backup_data) in [(bare.as_slice(), foreign), (foreign, bare.as_slice())] {
                    std::fs::write(&main, main_data).unwrap();
                    std::fs::write(&backup, backup_data).unwrap();
                    assert!(observe_globals(&fence.root, "T", "local:/x", true).is_err());
                    assert_eq!(std::fs::read(&main).unwrap(), main_data);
                    assert_eq!(std::fs::read(&backup).unwrap(), backup_data);
                }
            }
            std::fs::write(
                &backup,
                br#"{"app-server-migrated-pinned-thread-ids-by-host":{"local:/x":["T"]}}"#,
            )
            .unwrap();
            std::fs::write(&main, bare).unwrap();
            assert_eq!(
                observe_globals(&fence.root, "T", "local:/x", true)
                    .unwrap()
                    .map(|file| file.target_units),
                [1, 1]
            );
        }
        #[test]
        fn publication_preserves_other_lines_and_lock_inode() {
            let (dir, fence) = fixture();
            let path = dir.path().join("session_index.jsonl");
            std::fs::write(
                &path,
                b"{\"id\":\"T\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\r\nother\n",
            )
            .unwrap();
            let before = fence.lock.metadata().unwrap().ino();
            let expected = observation(&fence.fd, &fence.root.path, "session_index.jsonl", &|b| {
                reduce_index(b, "T")
            })
            .unwrap()
            .0;
            publish(&fence, "session_index.jsonl", &expected, &|b| {
                reduce_index(b, "T")
            })
            .unwrap();
            assert_eq!(std::fs::read(path).unwrap(), b"other\n");
            assert_eq!(
                std::fs::metadata(dir.path().join("session_index.jsonl.lock"))
                    .unwrap()
                    .ino(),
                before
            );
        }
    }
}
#[cfg(unix)]
pub(crate) use files::{
    GlobalKind, SharedFence, local_host_scope, observe_globals, observe_index, publish_global,
    publish_index,
};

#[cfg(test)]
mod global_tests {
    use super::*;

    #[test]
    fn preserves_unknown_lexemes_and_unrelated_records() {
        let input = br#"{
  "pinned-thread-ids" : [ "T", "U" ],
  "thread-project-membership-host-ids":{"T":"local","U":"remote"},
  "thread-project-assignments":{"T":{"projectKind":"local","projectId":"/p"},"U":{"projectKind":"local","projectId":"/p"}},
  "sidebar-thread-metadata":{"T":{"labelColor":"red","other":1e+09}},
  "unknown":{"n":1.000,"s":"T"}
}"#;
        let input = String::from_utf8_lossy(input).replace("\n", "\r\n");
        let (out, count) = reduce_global(input.as_bytes(), "T", "local:/x", true).unwrap();
        assert_eq!(count, 4);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(r#""unknown":{"n":1.000,"s":"T"}"#));
        assert!(text.contains(r#""U":{"projectKind":"local","projectId":"/p"}"#));
        assert!(text.contains(r#""other":1e+09"#));
        assert!(text.contains("\r\n"));
    }
    #[test]
    fn target_bare_ids_require_proven_ownership() {
        assert_eq!(
            reduce_global(
                br#"{"pinned-thread-ids":["T","U"]}"#,
                "T",
                "local:/x",
                false
            ),
            Err(SharedReductionError::UnsafeTargetRow)
        );
    }
    #[test]
    fn section_removal_preserves_section_and_foreign_host() {
        let input = br#"{"electron-persisted-atom-state":{"sidebar-custom-sections-v3":{"":{"threadHostIds":{"T":"local","U":"remote"},"sections":[{"itemKeys":["codex:thread:local:T","codex:thread:remote:U"],"pendingDeletionHostIdsByItemKey":{"codex:thread:local:T":["local"]},"collapsed":true}]}}}}"#;
        let (out, _) = reduce_global(input, "T", "local:/x", true).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("codex:thread:remote:U"));
        assert!(text.contains(r#""collapsed":true"#));
        assert!(!text.contains("codex:thread:local:T"));
    }
    #[test]
    fn custom_section_refuses_unbound_account_and_mixed_pending_hosts() {
        for (account, pending) in [("other", r#"["local"]"#), ("", r#"["local","remote"]"#)] {
            let input = format!(
                r#"{{"electron-persisted-atom-state":{{"sidebar-custom-sections-v3":{{"{account}":{{"threadHostIds":{{"T":"local"}},"sections":[{{"itemKeys":["codex:thread:local:T"],"pendingDeletionHostIdsByItemKey":{{"codex:thread:local:T":{pending}}}}}]}}}}}}}}"#
            );
            assert_eq!(
                reduce_global(input.as_bytes(), "T", "local:/x", true),
                Err(SharedReductionError::UnsafeTargetRow)
            );
        }
    }
    #[test]
    fn generated_retained_index_lines_are_exact() {
        let mut state = 0x4d3832315f763431_u64;
        for case in 0..256 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unrelated = format!(
                "{{\"id\":\"U{case}\",\"thread_name\":\"{}\",\"updated_at\":\"x\"}}\r\n",
                state
            );
            let target = b"{\"id\":\"T\",\"thread_name\":\"a\",\"updated_at\":\"b\"}\n";
            let mut bytes = unrelated.as_bytes().to_vec();
            bytes.extend_from_slice(target);
            bytes.extend_from_slice(b"malformed\xff");
            let (actual, count) = reduce_index(&bytes, "T").unwrap();
            let mut expected = unrelated.into_bytes();
            expected.extend_from_slice(b"malformed\xff");
            assert_eq!(actual, expected);
            assert_eq!(count, 1);
        }
    }
    #[test]
    fn exact_local_null_account_tab_and_presentation_are_removed() {
        let input = br#"{"electron-persisted-atom-state":{"pinned-content-tabs-v1":{"a":{"id":"a","accountId":null,"hostId":"local","threadId":"T","clientThreadId":"T","tabId":"x","tabKind":"chat","title":"target","routePath":"/x","routeSearch":"","order":1},"b":{"id":"b","accountId":null,"hostId":"remote","threadId":"T","clientThreadId":"T","tabId":"x","tabKind":"chat","title":"foreign","routePath":"/x","routeSearch":"","order":2}},"pinned-content-tab-presentation-v1":{"a":{"title":"target"},"b":{"title":"foreign"}},"unified-sidebar-pinned-order-v1":["content-tab:a","content-tab:b","codex:thread:local:T","other"]}}"#;
        let (out, count) = reduce_global(input, "T", "local:/x", true).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(count, 4);
        assert!(!text.contains("content-tab:a"));
        assert!(!text.contains("codex:thread:local:T"));
        assert!(!text.contains("target"));
        assert!(text.contains("content-tab:b"));
        assert!(text.contains("foreign"));
    }
    #[test]
    fn local_tab_with_unbound_account_refuses() {
        let input = br#"{"electron-persisted-atom-state":{"pinned-content-tabs-v1":{"a":{"id":"a","accountId":"other","hostId":"local","threadId":"T"}}}}"#;
        assert_eq!(
            reduce_global(input, "T", "local:/x", true),
            Err(SharedReductionError::UnsafeTargetRow)
        );
    }
    #[test]
    fn duplicate_keys_and_unsupported_composite_atom_refuse() {
        assert!(
            reduce_global(
                br#"{"pinned-thread-ids":[],"pinned-thread-ids":[]}"#,
                "T",
                "local:/x",
                true
            )
            .is_err()
        );
        assert!(reduce_global(br#"{"update-resume-state-v1":{}}"#, "T", "local:/x", true).is_err());
    }
}
