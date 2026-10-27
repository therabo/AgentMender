#!/usr/bin/env python3
"""Read-only architecture fitness checks; complements review and sandbox canaries."""
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version=1", "--no-deps", "--locked"], cwd=ROOT
))
allowed = {
    "agentmender-domain": set(),
    "agentmender-provider-api": {"agentmender-domain"},
    "agentmender-application": {"agentmender-domain", "agentmender-provider-api"},
    "agentmender-provider-codex": {"agentmender-domain", "agentmender-provider-api"},
    "agentmender-provider-claude": {"agentmender-domain", "agentmender-provider-api"},
    "agentmender": {"agentmender-domain", "agentmender-provider-api", "agentmender-application", "agentmender-provider-codex", "agentmender-provider-claude"},
}
errors = []
for package in metadata["packages"]:
    name = package["name"]
    if name not in allowed:
        errors.append(f"unreviewed crate: {name}")
        continue
    internal = {d["name"] for d in package["dependencies"] if d["name"].startswith("agentmender-")}
    if internal - allowed[name]:
        errors.append(f"dependency direction violation: {name}: {sorted(internal - allowed[name])}")

patterns = [
    r"(?:std::)?fs::(?:write|remove_file|remove_dir|remove_dir_all|rename|copy|create_dir|create_dir_all|set_permissions)\s*\(",
    r"\.(?:write|append|create|create_new|truncate)\s*\(\s*true\s*\)",
    r"File::create\s*\(",
    r'"(?:thread/(?:archive|unarchive|delete|start|resume)|turn/start)"',
    r'"(?:DELETE|UPDATE|INSERT)\s+',
]
for path in (ROOT / "crates").glob("*/src/**/*.rs"):
    # Unit-test modules are at the bottom of files. Fixture writes are test-only.
    source = path.read_text().split("#[cfg(test)]", 1)[0]
    for pattern in patterns:
        if re.search(pattern, source):
            errors.append(f"possible provider mutation: {path.relative_to(ROOT)}: {pattern}")
    if "Command::new" in source and path != ROOT / "crates/provider-api/src/process.rs":
        errors.append(f"process dispatch outside sandbox boundary: {path.relative_to(ROOT)}")
    if path.parent.parent.name == "domain" and re.search(r"std::(?:fs|process)|provider_(?:codex|claude|api)", source):
        errors.append(f"domain infrastructure coupling: {path.relative_to(ROOT)}")

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print("Architecture and read-only source checks passed (6 crates).")
