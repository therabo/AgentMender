//! Version-independent, validated fixed-column read contract (ADR-0024).
//! Source SQLite is opened only in the existing kernel-attested worker. No
//! configuration, transcript, preview, arbitrary SQL or mutation authority crosses it.
use crate::{CodexAdapter, Guard, capture};
use agentmender_domain::*;
use agentmender_provider_api::{
    ProviderError, SessionMetadataReadPort, evidence::EvidenceSandbox, process,
};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
const MAX_OUTPUT: usize = 128 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    source_root: PathBuf,
    mirror_root: PathBuf,
    probe_path: PathBuf,
}
fn unavailable() -> ProviderError {
    ProviderError::CapabilityUnavailable("validated metadata read contract unavailable".into())
}
pub(super) fn uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
pub(super) fn columns(
    db: &Connection,
    table: &str,
) -> Result<BTreeMap<String, String>, ProviderError> {
    // Only these fixed table names can enter the statement, never tool input.
    let sql = match table {
        "threads" => "PRAGMA table_info(threads)",
        "thread_spawn_edges" => "PRAGMA table_info(thread_spawn_edges)",
        _ => return Err(unavailable()),
    };
    let mut statement = db.prepare(sql).map_err(|_| unavailable())?;
    let result: BTreeMap<String, String> = statement
        .query_map([], |r| {
            Ok((r.get(1)?, r.get::<_, String>(2)?.to_ascii_uppercase()))
        })
        .map_err(|_| unavailable())?
        .collect::<Result<_, _>>()
        .map_err(|_| unavailable())?;
    if result.len() > 128 {
        return Err(unavailable());
    }
    Ok(result)
}
fn inventory(db: &Connection, root: &std::path::Path) -> Result<SessionInventory, ProviderError> {
    let kind: Option<String> = db
        .query_row(
            "SELECT type FROM sqlite_master WHERE name='threads'",
            [],
            |r| r.get(0),
        )
        .ok();
    if kind.as_deref() != Some("table") {
        return Err(unavailable());
    }
    let cols = columns(db, "threads")?;
    for (name, ty) in [
        ("id", "TEXT"),
        ("title", "TEXT"),
        ("cwd", "TEXT"),
        ("created_at", "INTEGER"),
        ("updated_at", "INTEGER"),
        ("archived", "INTEGER"),
    ] {
        if cols.get(name).map(String::as_str) != Some(ty) {
            return Err(unavailable());
        }
    }
    let producer = cols.get("cli_version").is_some_and(|v| v == "TEXT");
    let parents = columns(db, "thread_spawn_edges")?;
    let parent_columns = parents.get("parent_thread_id").is_some_and(|v| v == "TEXT")
        && parents.get("child_thread_id").is_some_and(|v| v == "TEXT");
    let named = cols.get("name").is_some_and(|v| v == "TEXT");
    let sql = match (producer, named) {
        (true, true) => {
            "SELECT id,COALESCE(NULLIF(name,''),title),cwd,created_at,updated_at,archived,cli_version FROM threads ORDER BY updated_at DESC,id LIMIT 501"
        }
        (false, true) => {
            "SELECT id,COALESCE(NULLIF(name,''),title),cwd,created_at,updated_at,archived,NULL FROM threads ORDER BY updated_at DESC,id LIMIT 501"
        }
        (true, false) => {
            "SELECT id,title,cwd,created_at,updated_at,archived,cli_version FROM threads ORDER BY updated_at DESC,id LIMIT 501"
        }
        (false, false) => {
            "SELECT id,title,cwd,created_at,updated_at,archived,NULL FROM threads ORDER BY updated_at DESC,id LIMIT 501"
        }
    };
    let mut stmt = db.prepare(sql).map_err(|_| unavailable())?;
    let mut rows = stmt.query([]).map_err(|_| unavailable())?;
    let mut sessions = vec![];
    let mut seen = BTreeSet::new();
    let mut total = 0;
    while let Some(row) = rows.next().map_err(|_| unavailable())? {
        if sessions.len() == 500 {
            break;
        }
        let id: String = row.get(0).map_err(|_| unavailable())?;
        if !uuid(&id) || !seen.insert(id.clone()) {
            return Err(unavailable());
        }
        let title: Option<String> = row.get(1).map_err(|_| unavailable())?;
        let cwd: Option<String> = row.get(2).map_err(|_| unavailable())?;
        // Oversized optional labels are explicitly unavailable, not invented or
        // silently truncated. One long first-prompt title must not hide all peers.
        let title = title.filter(|v| v.len() <= 4096);
        let cwd = cwd.filter(|v| v.len() <= 4096);
        let timestamp = |index| -> Result<Option<i64>, ProviderError> {
            Ok(row
                .get::<_, Option<i64>>(index)
                .map_err(|_| unavailable())?
                .filter(|v| *v >= 0))
        };
        let state = match row.get::<_, i64>(5).map_err(|_| unavailable())? {
            0 => SessionState::Active,
            1 => SessionState::Archived,
            _ => SessionState::Unknown,
        };
        let version: Option<String> = row.get(6).map_err(|_| unavailable())?;
        if version.as_ref().is_some_and(|v| {
            v.len() > 128
                || !v
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
        }) {
            return Err(unavailable());
        }
        let parent = if parent_columns {
            let mut q=db.prepare("SELECT parent_thread_id FROM thread_spawn_edges WHERE child_thread_id=?1 ORDER BY parent_thread_id LIMIT 2").map_err(|_|unavailable())?;
            let values = q
                .query_map([&id], |r| r.get::<_, String>(0))
                .map_err(|_| unavailable())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| unavailable())?;
            if values.len() == 1 && uuid(&values[0]) {
                values.into_iter().next()
            } else {
                None
            }
        } else {
            None
        };
        let metadata = SessionMetadata {
            identity: ArtifactId {
                provider: ProviderKind::Codex,
                state_root: root.into(),
                class: GraphNodeKind::Session,
                local_id: crate::scoped_id("session", &id)?,
            },
            native_session_id: id,
            title: title.filter(|v| !v.trim().is_empty()),
            project_label: cwd.and_then(|v| {
                std::path::Path::new(&v)
                    .file_name()
                    .and_then(|v| v.to_str())
                    .map(str::to_owned)
            }),
            created_at: timestamp(3)?,
            last_active_at: timestamp(4)?,
            state,
            // This historical flag concerns the stable fallback only. Readability
            // and the installed version are reported independently below.
            producer_version_matches: version.as_deref() == Some("0.166.0"),
            producer_version: version,
            parent_native_session_id: parent,
        };
        let size = serde_json::to_vec(&metadata)
            .map_err(|_| unavailable())?
            .len();
        if total + size > 96 * 1024 {
            break;
        }
        total += size;
        sessions.push(metadata);
    }
    Ok(SessionInventory { sessions, completeness:Completeness::Partial,
        read_compatibility:Some(SessionReadCompatibility {
            metadata_contract:"codex-state-threads-columns-v1".into(),provider_version:None,app_server_methods:vec![],native_interface_status:"not_probed".into(),
            native_delete_admission:"unverified_effect_contract".into(),scoped_fallback_admission:"not_assessed_by_metadata".into(),additional_cleanup_admission:"unavailable".into(),
            coverage:"selected_root_state_database_snapshot_only; cwd grouping is not ownership; labels over 4096 bytes are unavailable; optional relationships are not descendant closure".into(),relationship_coverage:Completeness::Partial,source_changed_during_capture:false,
        }) })
}

pub fn run_metadata_capture_worker() -> Result<(), ProviderError> {
    #[cfg(target_os = "macos")]
    {
        let mut bytes = vec![];
        std::io::stdin()
            .take(8193)
            .read_to_end(&mut bytes)
            .map_err(|_| unavailable())?;
        if bytes.len() > 8192 {
            return Err(unavailable());
        }
        let request: Request = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        capture::validate_worker_paths(
            &request.source_root,
            &request.mirror_root,
            &request.probe_path,
        )?;
        let before = capture::fingerprints(&request.source_root)?;
        let source = Connection::open_with_flags(
            request.source_root.join("state_5.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| unavailable())?;
        capture::configure(&source)?;
        source
            .execute_batch("PRAGMA query_only=ON")
            .map_err(|_| unavailable())?;
        let mut memory = Connection::open_in_memory().map_err(|_| unavailable())?;
        capture::configure(&memory)?;
        capture::bounded_backup(
            &source,
            &mut memory,
            Instant::now() + Duration::from_secs(3),
        )?;
        drop(source);
        let mut result = inventory(&memory, &request.source_root)?;
        result
            .read_compatibility
            .as_mut()
            .expect("contract")
            .source_changed_during_capture =
            capture::fingerprints_changed(&before, &capture::fingerprints(&request.source_root)?);
        let bytes = serde_json::to_vec(&result).map_err(|_| unavailable())?;
        if bytes.len() > MAX_OUTPUT {
            return Err(unavailable());
        }
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|_| unavailable())?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(unavailable())
    }
}

impl SessionMetadataReadPort for CodexAdapter {
    fn session_metadata(&self) -> Result<SessionInventory, ProviderError> {
        if self.fixture.is_some() {
            return Err(unavailable());
        }
        agentmender_provider_api::validate_location(&self.location)?;
        let (helper, home) = self.evidence_helper.as_ref().ok_or_else(unavailable)?;
        let sandbox = EvidenceSandbox::new(&self.location.state_root, home)?;
        let request = Request {
            source_root: sandbox.source_root().into(),
            mirror_root: sandbox.path().into(),
            probe_path: sandbox.probe_path().into(),
        };
        let mut child = Guard(process::spawn_codex_metadata_capture(helper, &sandbox)?);
        let mut input = child.0.stdin.take().ok_or_else(unavailable)?;
        let mut output = child.0.stdout.take().ok_or_else(unavailable)?;
        let encoded = serde_json::to_vec(&request).map_err(|_| unavailable())?;
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut bytes = vec![];
            let result = input
                .write_all(&encoded)
                .and_then(|_| {
                    drop(input);
                    output
                        .by_ref()
                        .take((MAX_OUTPUT + 1) as u64)
                        .read_to_end(&mut bytes)
                })
                .map(|_| bytes);
            let _ = sent.send(result);
        });
        let bytes = received
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| unavailable())?
            .map_err(|_| unavailable())?;
        if bytes.len() > MAX_OUTPUT {
            return Err(unavailable());
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.0.try_wait() {
                Ok(Some(s)) if s.success() => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                _ => return Err(unavailable()),
            }
        }
        let mut result: SessionInventory =
            serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
        if result.sessions.len() > 500
            || result.sessions.iter().any(|s| {
                s.identity.state_root != sandbox.source_root()
                    || s.identity.provider != ProviderKind::Codex
                    || s.identity.class != GraphNodeKind::Session
                    || s.identity.local_id.as_str()
                        != format!("codex:session:{}", s.native_session_id)
                    || !uuid(&s.native_session_id)
            })
        {
            return Err(unavailable());
        }
        if let Some(contract) = result.read_compatibility.as_mut() {
            if let Some(executable) = &self.location.executable {
                contract.provider_version =
                    process::probe_version(ProviderKind::Codex, executable).ok();
                if let Ok(methods) = process::codex_app_server_schema(executable, &sandbox)
                    .and_then(|bytes| schema_methods(&bytes))
                {
                    contract.app_server_methods = methods;
                    contract.native_interface_status =
                        "advertised_by_installed_schema_not_effect_admission".into();
                } else {
                    contract.native_interface_status = "unknown".into();
                }
            }
        } else {
            return Err(unavailable());
        }
        Ok(result)
    }
}

// Schema names establish interface availability only. No request is sent here.
fn schema_methods(bytes: &[u8]) -> Result<Vec<String>, ProviderError> {
    let schema: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| unavailable())?;
    fn visit(value: &serde_json::Value, found: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(method) = value.pointer("/properties/method") {
                    for name in method
                        .get("enum")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(serde_json::Value::as_str)
                        .chain(method.get("const").and_then(serde_json::Value::as_str))
                    {
                        if matches!(
                            name,
                            "thread/list"
                                | "thread/read"
                                | "thread/archive"
                                | "thread/delete"
                                | "thread/loaded/list"
                        ) {
                            found.insert(name.into());
                        }
                    }
                }
                for value in object.values() {
                    visit(value, found);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    visit(value, found);
                }
            }
            _ => {}
        }
    }
    let mut found = BTreeSet::new();
    visit(&schema, &mut found);
    if !found.contains("thread/list") {
        return Err(unavailable());
    }
    Ok(found.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE threads(id TEXT,title TEXT,cwd TEXT,created_at INTEGER,updated_at INTEGER,archived INTEGER,cli_version TEXT,future_column BLOB,preview TEXT,first_user_message TEXT);").unwrap();
        c
    }
    fn seed(c: &Connection, id: &str, title: &str, version: &str) {
        c.execute("INSERT INTO threads VALUES(?1,?2,'/private/project/MISSION1',1,2,0,?3,X'00','PRIVATE PREVIEW','PRIVATE MESSAGE')",(id,title,version)).unwrap();
    }
    #[test]
    fn additive_schema_and_unknown_producer_are_readable_without_mutation_authority() {
        let c = database();
        seed(
            &c,
            "12345678-1234-1234-1234-123456789abc",
            "same title",
            "9.0.0",
        );
        seed(
            &c,
            "12345678-1234-1234-1234-123456789abd",
            "same title",
            "0.166.0-alpha.6",
        );
        let inventory = inventory(&c, std::path::Path::new("/root")).unwrap();
        assert_eq!(inventory.sessions.len(), 2);
        assert!(
            inventory
                .sessions
                .iter()
                .all(|s| !s.producer_version_matches)
        );
        let encoded = serde_json::to_string(&inventory).unwrap();
        assert!(!encoded.contains("PRIVATE") && !encoded.contains("/private/project"));
        assert_eq!(
            inventory
                .read_compatibility
                .unwrap()
                .native_delete_admission,
            "unverified_effect_contract"
        );
    }
    #[test]
    fn changed_required_affinity_and_duplicate_uuid_refuse() {
        let c = database();
        seed(&c, "12345678-1234-1234-1234-123456789abc", "same", "9.0.0");
        seed(&c, "12345678-1234-1234-1234-123456789abc", "same", "9.0.0");
        assert!(inventory(&c, std::path::Path::new("/root")).is_err());
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE threads(id BLOB,title TEXT,cwd TEXT,created_at INTEGER,updated_at INTEGER,archived INTEGER)").unwrap();
        assert!(inventory(&c, std::path::Path::new("/root")).is_err());
    }
    #[test]
    fn display_name_optional_parent_archive_and_oversized_title_are_explicit() {
        let c = database();
        c.execute_batch("ALTER TABLE threads ADD COLUMN name TEXT; CREATE TABLE thread_spawn_edges(parent_thread_id TEXT,child_thread_id TEXT);").unwrap();
        // The pre-name fixture helper needs a fixed column list after the added field.
        let id = "12345678-1234-1234-1234-123456789abc";
        c.execute("INSERT INTO threads(id,title,cwd,created_at,updated_at,archived,cli_version,name) VALUES(?1,?2,'/MISSION1',1,2,1,'next-release','Friendly title')",(id,"x".repeat(8192))).unwrap();
        c.execute(
            "INSERT INTO thread_spawn_edges VALUES('12345678-1234-1234-1234-123456789abd',?1)",
            [id],
        )
        .unwrap();
        let found = inventory(&c, std::path::Path::new("/root")).unwrap();
        let row = &found.sessions[0];
        assert_eq!(row.title.as_deref(), Some("Friendly title"));
        assert_eq!(row.state, SessionState::Archived);
        assert_eq!(
            row.parent_native_session_id.as_deref(),
            Some("12345678-1234-1234-1234-123456789abd")
        );
        c.execute("UPDATE threads SET name=NULL", []).unwrap();
        assert!(
            inventory(&c, std::path::Path::new("/root"))
                .unwrap()
                .sessions[0]
                .title
                .is_none()
        );
    }
    #[test]
    fn inventory_bounds_and_schema_capabilities_never_admit_effects() {
        let c = database();
        for i in 0..501 {
            seed(
                &c,
                &format!("00000000-0000-0000-0000-{i:012x}"),
                "bounded",
                "unknown",
            );
        }
        let found = inventory(&c, std::path::Path::new("/root")).unwrap();
        assert!(found.sessions.len() <= 500);
        assert_eq!(found.completeness, Completeness::Partial);
        let methods=schema_methods(br#"{"oneOf":[{"properties":{"method":{"enum":["thread/list"]}}},{"properties":{"method":{"const":"thread/delete"}}},{"properties":{"method":{"const":"future/method"}}}]}"#).unwrap();
        assert_eq!(methods, vec!["thread/delete", "thread/list"]);
        assert_eq!(
            found.read_compatibility.unwrap().native_delete_admission,
            "unverified_effect_contract"
        );
        assert!(schema_methods(br#"{"oneOf":[]}"#).is_err());
    }
}
