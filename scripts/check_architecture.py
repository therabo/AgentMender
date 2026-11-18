#!/usr/bin/env python3
"""Check the dependency and I/O boundaries of the local manager."""
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
DEPS = {
    "agentmender": {"application", "domain", "provider-api", "provider-claude", "provider-codex"},
    "agentmender-application": {"domain"},
    "agentmender-domain": set(),
    "agentmender-provider-api": {"domain"},
    "agentmender-provider-claude": {"domain", "provider-api"},
    "agentmender-provider-codex": {"domain", "provider-api"},
}
packages = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
    cwd=ROOT,
))["packages"]
assert {package["name"] for package in packages} == set(DEPS)
for package in packages:
    deps = {dependency["name"] for dependency in package["dependencies"]}
    internal = {name.removeprefix("agentmender-") for name in deps if name.startswith("agentmender-")}
    assert internal == DEPS[package["name"]], (package["name"], internal)
    assert deps.isdisjoint({"rmcp", "schemars", "hmac", "chrono-tz", "snap", "rusty-leveldb"}), deps

def source(path):
    return (ROOT / path).read_text()

def require(path, *markers):
    contents = re.sub(r"\s+", "", source(path))
    assert all(re.sub(r"\s+", "", marker) in contents for marker in markers), path

def boundaries(path, contents):
    if "Command::new" in contents:
        assert path in {"crates/provider-api/src/process.rs", "crates/cli/src/ui/browser.rs"}, path
    if "allow(unsafe_code)" in contents:
        assert path == "crates/provider-api/src/macos_sandbox.rs", path
    if "rusqlite::" in contents:
        assert path in {
            "crates/provider-codex/src/snapshot.rs",
            "crates/provider-codex/src/worker.rs",
            "crates/provider-codex/src/worker/inventory.rs",
            "crates/provider-codex/src/worker/projects.rs",
            "crates/provider-codex/src/worker/cascade.rs",
            "crates/provider-codex/src/desktop_reconciliation.rs",
        }, path
    assert not re.search(r"\b(session_find|SessionSearchReadPort|run_codex_delete|CREATE\s+VIRTUAL\s+TABLE)\b", contents), path
    if path.startswith("crates/provider-claude/"):
        assert not re.search(r"\b(?:(?:remove_file|remove_dir|remove_dir_all|rename)\s*\(|OpenOptions\b|run_confined_codex_delete\b)", contents), path

for path in ROOT.glob("crates/*/src/**/*.rs"):
    production = path.read_text().split("#[cfg(test)]" + chr(10) + "mod tests", 1)[0]
    boundaries(path.relative_to(ROOT).as_posix(), production)

require("crates/cli/src/ui/mod.rs", "Ipv4Addr::LOCALHOST", "header::HOST",
        "header::ORIGIN", "header::AUTHORIZATION", "DefaultBodyLimit::max(4096)",
        "frame-ancestors 'none'")
require("crates/provider-api/src/filesystem.rs", "OFlags::NOFOLLOW", "metadata.nlink() != 1")
require("crates/provider-codex/src/snapshot.rs", "SQLITE_OPEN_READ_ONLY", "SQLITE_OPEN_NOFOLLOW",
        "PRAGMA query_only=ON", "verify_source_write_denial", "create_new(true)")
require("crates/application/src/local_inventory.rs", ".filter(Session::is_main)")
require("crates/provider-codex/src/worker/inventory.rs", "thread_spawn_edges",
        "reader::relationship_header", "attached_thread_parent_id")
require("crates/provider-codex/src/worker/cascade.rs", "SessionState::Archived", "thread_spawn_edges",
        "archived_sessions", "reader::relationship_header", "strict_source_files", "unrelated_revision",
        "check_native_lookup", "rollout.meta", "history_base")
require("crates/provider-codex/src/removal.rs", "self.prepare(adapter, review.session())",
        "self.is_current()", "probe_version", "probe_codex_delete", "check_writer_locks",
        "run_confined_codex_delete", "adapter.verify_removal(scope)", "Duration::from_secs(180)")
require("crates/provider-codex/src/worker.rs", "Operation::ProjectRemoval(project)", "cascade::project_plan",
        "Operation::ProjectCatalog", "projects::read")
require("crates/provider-codex/src/worker/projects.rs", "FROM projects", "FROM project_roots",
        ".codex-global-state.json", "filesystem::open_source_read", "MAX_DESKTOP_BYTES")
require("crates/provider-codex/src/worker/cascade.rs", "ProjectLocator::Codex(project_id)",
        "inventory::membership", "assigned.is_subset(&members)")
assert "ProjectLocator::Workspace" not in source("crates/provider-codex/src/worker/cascade.rs")
require("crates/provider-codex/src/removal.rs", "self.observe_project(adapter, &review.project)",
        "fresh.evidence.archive_revision", "tree.scope.unrelated_revision")
require("crates/cli/src/ui/removal.rs", "RIMUOVI SESSIONI DEL PROGETTO", 'scope: "sessions"', "self.running", "self.previews.remove")
require("crates/provider-api/src/process.rs", "sqlite_home={scope}", "(deny network*)",
        "(deny file-write* (require-not (subpath {scope})))", '.arg("--force")',
        'process.args(["delete", "--help"])', "if !is_current()")
require("crates/cli/src/ui/browser.rs", "Ipv4Addr::LOCALHOST", "byte.is_ascii_hexdigit()")

require("crates/provider-claude/src/read.rs", "MAX_RECORD", "Revision::read", "filesystem::open_observed_read")
require("crates/provider-claude/src/local.rs", "desktop_workspace", "verify(root,&path,revision)",
        "saved.identity", "scan.foreign_rows")
require("crates/cli/src/ui/mod.rs", "/api/claude/sessions/{key}/storage",
        "return Err(ApiError::conflict(agentmender_provider_claude::REMOVAL_BLOCKER,))")

require("crates/provider-claude/src/local.rs", 'path.join("projects")', "desktop.reconcile",
        "Exclusion::UnknownEntrypoint")
require("crates/provider-claude/src/storage.rs", 'root.join("claude-code-sessions")',
        "projects_complete && !candidates.contains_key(&id)")

for path in ["crates/provider-codex/src/exports.rs", "crates/cli/src/ui/storage.rs"]:
    assert not re.search(r"\b(remove_file|remove_dir_all|run_confined_codex_delete|ReviewedRemoval)\b", source(path)), path
require("crates/cli/src/ui/mod.rs", "/api/storage/exports/preview", "/api/storage/exports/operations/{key}/confirm")
require("crates/cli/src/ui/storage.rs", "receipt: None", "snapshot(app, true)")

native = source("crates/provider-codex/src/removal.rs")
assert not re.search(r"\b[a-f0-9]{64}\b", native)
assert native.count("run_confined_codex_delete(") == 1
require(".cargo/config.toml", "-USQLITE_ENABLE_FTS5")
print("Workspace dependencies, confined provider reads, browser authorization and native boundary passed.")
