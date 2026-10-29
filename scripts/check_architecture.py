#!/usr/bin/env python3
"""Read-only architecture fitness checks; complements review and sandbox canaries."""
import json
import pathlib
import re
import subprocess
import sys

def mask_literals(source):
    """Keep offsets/braces in code; mask comments and Rust string/char literals."""
    chars = list(source)
    index = 0
    while index < len(source):
        start = index
        if source.startswith('//', index):
            index = source.find('\n', index)
            if index < 0:
                index = len(source)
        elif source.startswith('/*', index):
            index += 2
            depth = 1
            while index < len(source) and depth:
                if source.startswith('/*', index):
                    depth += 1
                    index += 2
                elif source.startswith('*/', index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
        else:
            raw = re.match(r'(?:br|cr|r)(#*)"', source[index:])
            if raw:
                delimiter = '"' + raw.group(1)
                end = source.find(delimiter, index + len(raw.group()))
                index = len(source) if end < 0 else end + len(delimiter)
            elif source[index] == '"':
                index += 1
                while index < len(source):
                    if source[index] == '\\':
                        index += 2
                    elif source[index] == '"':
                        index += 1
                        break
                    else:
                        index += 1
            elif source[index] == "'" and (char := re.match(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'", source[index:])):
                index += len(char.group())
            else:
                index += 1
                continue
        chars[start:index] = ['\n' if c == '\n' else ' ' for c in source[start:index]]
    return ''.join(chars)


def production_source(source):
    """Remove complete test modules, including when production follows them."""
    masked = mask_literals(source)
    result = list(source)
    pattern = r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*(?:pub\s+)?mod\s+\w+\s*\{'
    for match in re.finditer(pattern, masked):
        index = match.end()
        depth = 1
        while index < len(masked) and depth:
            depth += (masked[index] == '{') - (masked[index] == '}')
            index += 1
        if depth:
            raise ValueError('unbalanced Rust test module')
        result[match.start():index] = ['\n' if c == '\n' else ' ' for c in source[match.start():index]]
    return ''.join(result)


def impure_graph_code(source):
    code = mask_literals(production_source(source))
    return bool(re.search(
        r"\b(?:filesystem|process|fs)::|\bCommand\b|\b(?:SystemTime|Instant)\s*::\s*now|\.(?:detect|discover)\s*\(", code
    ) or re.search(
        r"\bstd\s*::\s*(?:fs|process|time|net)\b|\buse\s+std\s*::\s*\{[^;]*(?:\bfs\b|\bprocess\b|\btime\b|\bnet\b)", code
    ))


def method_bodies(source, name):
    """Inspect pure-port overrides even when declared in an adapter lib.rs."""
    masked = mask_literals(production_source(source))
    for match in re.finditer(r'\bfn\s+' + re.escape(name) + r'\b[^;{]*\{', masked):
        index, depth = match.end(), 1
        while index < len(masked) and depth:
            depth += (masked[index] == '{') - (masked[index] == '}')
            index += 1
        if depth:
            raise ValueError('unbalanced pure port method')
        yield source[match.start():index]


# Regression: production code after a test module must still be audited.
example = '#[cfg(test)] mod tests { let x = r#"}"#; fs::write("fixture", b"x"); }\nfn later() { fs::write("unsafe", b"x"); }'
filtered = production_source(example)
assert 'fixture' not in filtered and 'fn later' in filtered and 'fs::write' in filtered
for canary in [
    'fn build() { std::fs::read("provider"); }',
    'use std::{fs as files}; fn build() { files::read("provider"); }',
    'fn build() { SystemTime::now(); }',
    'fn build() { adapter.discover(scope); }',
]:
    assert impure_graph_code(canary), 'graph purity fitness canary failed'
assert not impure_graph_code('fn build() { let text = "std::fs::read"; }')
assert not impure_graph_code('#[cfg(test)] mod tests { fn fixture() { std::fs::read("fixture"); } }')
override = 'impl Adapter { fn integrity_context(&self) { std::fs::read("provider"); } }'
assert any(impure_graph_code(body) for body in method_bodies(override, 'integrity_context'))
planning_override = 'impl Adapter { fn planning_context(&self) { std::fs::read("provider"); } }'
assert any(impure_graph_code(body) for body in method_bodies(planning_override, 'planning_context'))
assert impure_graph_code('fn planning_helper() { std::fs::read("provider"); }')

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
    "agentmender-mcp-server": {"agentmender-domain", "agentmender-application"},
    "agentmender": {"agentmender-domain", "agentmender-provider-api", "agentmender-application", "agentmender-provider-codex", "agentmender-provider-claude", "agentmender-mcp-server"},
}
errors = []
actual_packages = {p["name"] for p in metadata["packages"]}
if actual_packages != set(allowed):
    errors.append(f"workspace crate set changed: {sorted(actual_packages ^ set(allowed))}")
for package in metadata["packages"]:
    name = package["name"]
    if name not in allowed:
        errors.append(f"unreviewed crate: {name}")
        continue
    internal = {d["name"] for d in package["dependencies"] if d["name"].startswith("agentmender-")}
    if internal - allowed[name]:
        errors.append(f"dependency direction violation: {name}: {sorted(internal - allowed[name])}")
    for dep in package["dependencies"]:
        if dep["name"] in {"rmcp", "schemars"} and name != "agentmender-mcp-server":
            errors.append(f"MCP SDK outside adapter: {name}: {dep['name']}")
        if dep["name"] in {"tokio", "tokio-util"} and name not in {"agentmender", "agentmender-mcp-server"}:
            errors.append(f"async runtime outside inbound boundary: {name}: {dep['name']}")

patterns = [
    r"(?:std::)?fs::(?:write|remove_file|remove_dir|remove_dir_all|rename|copy|create_dir|create_dir_all|set_permissions)\s*\(",
    r"\.(?:write|append|create|create_new|truncate)\s*\(\s*true\s*\)",
    r"File::create\s*\(",
    r'"(?:thread/(?:archive|unarchive|delete|start|resume)|turn/start)"',
    r'"(?:DELETE|UPDATE|INSERT)\s+',
]
for path in (ROOT / "crates").glob("*/src/**/*.rs"):
    source = production_source(path.read_text())
    code = mask_literals(source)
    reviewed_capture = path == ROOT / "crates/provider-codex/src/capture.rs"
    for pattern in patterns:
        if re.search(pattern, source) and not (reviewed_capture and pattern in patterns[1:]):
            errors.append(f"possible provider mutation: {path.relative_to(ROOT)}: {pattern}")
    if path != ROOT / "crates/provider-api/src/macos_sandbox.rs" and (re.search(r"\bunsafe\b", code) or re.search(r"allow\s*\(\s*unsafe_code", code)):
        errors.append(f"unsafe code outside reviewed OS boundary: {path.relative_to(ROOT)}")
    if reviewed_capture and not all(marker in source for marker in ["SQLITE_OPEN_READ_ONLY", "PRAGMA query_only=ON", "verify_source_write_denial", "create_new(true)"]):
        errors.append("disposable capture safety markers missing; review required")
    if "Command::new" in source and path != ROOT / "crates/provider-api/src/process.rs":
        errors.append(f"process dispatch outside sandbox boundary: {path.relative_to(ROOT)}")
    crate_name = path.relative_to(ROOT / "crates").parts[0]
    if crate_name == "mcp-server":
        if (re.search(r"\b(?:agentmender_provider_(?:api|codex|claude)|std::(?:fs|process|net)|tokio::(?:fs|process|net)|rusqlite|sqlite|Command|File|OpenOptions)\b", code)
            or re.search(r"\buse\s+(?:std|tokio)\s*::\s*\{[^;]*(?:\bfs\b|\bprocess\b|\bnet\b)", code)
            or re.search(r"\b(?:fn|struct|enum|trait)\s+(?:execute|approve|execute_\w+|approve_\w+|\w+_execute|\w+_approval)\b", code)):
            errors.append(f"MCP infrastructure or execution access: {path.relative_to(ROOT)}")
    if crate_name == "domain" and re.search(r"std::(?:fs|process)|provider_(?:codex|claude|api)", source):
        errors.append(f"domain infrastructure coupling: {path.relative_to(ROOT)}")
    # ADR-0006: graph interpretation consumes observations and cannot acquire new
    # evidence through filesystem/process/time access. Audit executable code after
    # masking strings and removing complete test modules, not broad prose matches.
    # ADR-0008: integrity consumes normalized graph facts under the same purity
    # boundary. Audit all domain modules, including any future nested modules.
    if path.name in {"graph.rs", "integrity.rs", "planning.rs"} or crate_name == "domain":
        if impure_graph_code(source):
            errors.append(f"impure graph transformation: {path.relative_to(ROOT)}")
    # ADR-0009: planning descriptions interpret supplied normalized evidence only.
    for name in ["build_graph", "integrity_context", "planning_context"]:
        if any(impure_graph_code(body) for body in method_bodies(source, name)):
            errors.append(f"impure {name} port override: {path.relative_to(ROOT)}")
    if re.search(r'\bfn\s+(?:archive|unarchive|purge|repair|delete)\b', code):
        errors.append(f"lifecycle or repair method before its milestone: {path.relative_to(ROOT)}")
    if re.search(r'\b(?:fn|struct|enum|trait)\s+(?:execute_plan|approve_plan|ApprovalGrant|ApprovalBroker|RecoveryStore|RecoveryCoordinator|CleanupTransaction|TransactionJournal|DeleteFile|RunSql|RemoveDirectory|ExecuteCommand)\b', code):
        errors.append(f"execution/approval/recovery or low-level planning primitive before its milestone: {path.relative_to(ROOT)}")
    if crate_name == "domain" and any(marker in source for marker in [
        "sqlite_master", "state_5.sqlite", "claude-code-sessions", "cliSessionId",
        "forkedFromSessionId", "thread/list", "attached_thread_parent_id",
    ]):
        errors.append(f"provider storage knowledge in domain: {path.relative_to(ROOT)}")
    if path == ROOT / "crates/provider-api/src/lib.rs":
        trait = re.search(r"pub\s+trait\s+ProviderAdapter\s*\{", code)
        if trait is None:
            errors.append("read-only provider port missing")
        else:
            index, depth = trait.end(), 1
            while index < len(code) and depth:
                depth += (code[index] == '{') - (code[index] == '}')
                index += 1
            if re.search(r"ProviderAdapter\s*:\s*[^\{]*(?:Send|Sync)", code[trait.start():trait.end()]):
                errors.append("provider port must not require Send or Sync")
            methods = set(re.findall(r"\bfn\s+(\w+)\s*\(", code[trait.end():index]))
            if methods != {"kind", "detect", "discover", "build_graph", "integrity_context", "planning_context"}:
                errors.append(f"unreviewed provider port methods: {sorted(methods)}")

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
if 'unsafe_code = "forbid"' not in (ROOT / "Cargo.toml").read_text():
    raise SystemExit("workspace unsafe-code prohibition changed")
print("Architecture and real-state read-only source checks passed (7 crates).")
