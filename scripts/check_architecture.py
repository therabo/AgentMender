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


# Regression: production code after a test module must still be audited.
example = '#[cfg(test)] mod tests { let x = r#"}"#; fs::write("fixture", b"x"); }\nfn later() { fs::write("unsafe", b"x"); }'
filtered = production_source(example)
assert 'fixture' not in filtered and 'fn later' in filtered and 'fs::write' in filtered

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
    if path.parent.parent.name == "domain" and re.search(r"std::(?:fs|process)|provider_(?:codex|claude|api)", source):
        errors.append(f"domain infrastructure coupling: {path.relative_to(ROOT)}")

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
if 'unsafe_code = "forbid"' not in (ROOT / "Cargo.toml").read_text():
    raise SystemExit("workspace unsafe-code prohibition changed")
print("Architecture and real-state read-only source checks passed (6 crates).")
