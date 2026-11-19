//! Remove auxiliary references after the native Codex command succeeds.
use crate::desktop_reconciliation;
use agentmender_provider_api::rewrite::{self, directory};
use serde_json::Value;
use std::{io, path::Path};
use uuid::Uuid;

pub(super) fn after_native_delete(root: &Path, id: Uuid) -> Vec<String> {
    let mut errors = Vec::new();
    let id = id.hyphenated().to_string();
    match directory(root) {
        Ok(archive) => {
            for (name, field) in [
                ("history.jsonl", "session_id"),
                ("session_index.jsonl", "id"),
            ] {
                if let Err(error) = rewrite::replace(
                    &archive,
                    name,
                    |input, output| {
                        rewrite::filter_jsonl(input, output, |line| {
                            serde_json::from_slice::<Value>(line).is_ok_and(|row| {
                                row.get(field).and_then(Value::as_str) == Some(&id)
                            })
                        })
                    },
                    || Ok(()),
                ) {
                    errors.push(format!("{name}: {error}"));
                }
            }

            let catalogs = desktop_reconciliation::remove_local_thread(root, &id, &mut errors);

            for name in [".codex-global-state.json", ".codex-global-state.json.bak"] {
                if let Err(error) = rewrite::replace(
                    &archive,
                    name,
                    |input, output| {
                        let mut state: Value = serde_json::from_reader(io::BufReader::new(input))?;
                        if !prune_desktop_state(&mut state, &id)? {
                            return Ok(0);
                        }
                        serde_json::to_writer_pretty(&mut *output, &state)?;
                        output.write_all(b"\n")?;
                        Ok(1)
                    },
                    || Ok(()),
                ) {
                    errors.push(format!("{name}: {error}"));
                }
            }

            desktop_reconciliation::schedule_desktop_catalog_reconciliation(
                root,
                catalogs,
                &mut errors,
            );
        }
        Err(error) => errors.push(format!("Archivio Codex: {error}")),
    }
    errors
}

fn prune_desktop_state(state: &mut Value, id: &str) -> io::Result<bool> {
    let state = state.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Lo stato Desktop deve essere un oggetto JSON.",
        )
    })?;
    let mut changed = false;
    for key in [
        "thread-project-assignments",
        "thread-projectless-output-directories",
        "thread-writable-roots",
        "thread-workspace-root-hints",
    ] {
        if let Some(map) = state.get_mut(key).and_then(Value::as_object_mut) {
            changed |= map.remove(id).is_some();
        }
    }
    if let Some(items) = state
        .get_mut("projectless-thread-ids")
        .and_then(Value::as_array_mut)
    {
        let len = items.len();
        items.retain(|item| item.as_str() != Some(id));
        changed |= items.len() != len;
    }
    Ok(changed)
}
