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
        }, path
    assert not re.search(r"\b(session_find|SessionSearchReadPort|run_codex_delete|CREATE\s+VIRTUAL\s+TABLE)\b", contents), path
    if path.startswith("crates/provider-claude/"):
        assert not re.search(r"\b(?:(?:remove_file|remove_dir|remove_dir_all|rename)\s*\(|OpenOptions\b|run_confined_codex_delete\b)", contents), path
        if re.search(r"\b(renameat_with|unlinkat|mkdirat)\b", contents):
            assert path in {"crates/provider-claude/src/cleanup/mod.rs", "crates/provider-claude/src/cleanup/history.rs"}, path

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
require("crates/provider-codex/src/worker.rs", "Operation::ProjectCatalog", "projects::read")
require("crates/provider-codex/src/worker/projects.rs", "FROM projects", "FROM project_roots",
        ".codex-global-state.json", "filesystem::open_source_read", "MAX_DESKTOP_BYTES")
require("crates/provider-codex/src/lib.rs", "identity.state_root != root", "Uuid::parse_str", "process::codex_delete")
require("crates/provider-api/src/process.rs", '.args(["-c", sqlite_override, "delete"])',
        '.arg("--force")', '.env("CODEX_HOME", root)', "output.status.success()")
require("crates/cli/src/ui/browser.rs", "Ipv4Addr::LOCALHOST", "byte.is_ascii_hexdigit()")

require("crates/provider-claude/src/read.rs", "MAX_RECORD", "Revision::read", "filesystem::open_observed_read")
require("crates/provider-claude/src/local.rs", "desktop_workspace", "verify(root,&path,revision)",
        "saved.identity", "scan.foreign_rows")
require("crates/cli/src/ui/mod.rs", "/api/claude/sessions/{key}/storage",
        "/api/{provider}/sessions/{key}/delete", "/api/codex/projects/{key}/delete")
require("crates/cli/src/ui/removal.rs", "if !request.confirm", "app.inventory.invalidate(provider)",
        "plan.delete()", "app.codex.delete_session")
require("crates/provider-claude/src/local.rs", 'path.join("projects")', "desktop.reconcile",
        "Exclusion::UnknownEntrypoint")
require("crates/provider-claude/src/storage.rs", 'root.join("claude-code-sessions")',
        "projects_complete && !candidates.contains_key(&id)")

require("crates/provider-claude/src/usage/walk.rs", "OFlags::NOFOLLOW",
        "AtFlags::SYMLINK_NOFOLLOW", "MAX_DEPTH", "self.budget.take()")
require("crates/provider-claude/src/artifacts.rs", "verify(&file.root, &file.path, &file.revision)")
require("crates/provider-claude/src/cleanup/mod.rs", "OFlags::NOFOLLOW", "fs::unlinkat",
        "current.st_nlink != 1", "exact_sidecar", 'root.join("file-history")')
require("crates/provider-claude/src/cleanup/history.rs", '"history.jsonl.lock"',
        "fs::mkdirat", "writer.write_all(line)", "lock.held()", "fs::renameat")
require("crates/cli/src/ui/mod.rs", "/api/storage/claude", "/api/storage/claude/refresh")
assert "operations" not in source("crates/cli/src/ui/claude_usage.rs")

for path in ["crates/provider-codex/src/exports.rs", "crates/cli/src/ui/storage.rs"]:
    assert not re.search(r"\b(remove_file|remove_dir_all|run_confined_codex_delete|ReviewedRemoval)\b", source(path)), path
require("crates/cli/src/ui/mod.rs", "/api/storage/exports/preview", "/api/storage/exports/operations/{key}/confirm")
require("crates/cli/src/ui/storage.rs", "receipt: None", "snapshot(app, true)")

require(".cargo/config.toml", "-USQLITE_ENABLE_FTS5")
print("Workspace dependencies, confined provider reads, browser authorization and native boundary passed.")
