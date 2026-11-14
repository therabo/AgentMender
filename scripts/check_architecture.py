#!/usr/bin/env python3
"""Provider observation, approval and exact lifecycle fitness (ADRs 0001-0018)."""
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
    if re.match(r'\s*#!\[cfg\(test\)\]', source):
        return ''
    masked = mask_literals(source)
    result = list(source)
    pattern = r'#\s*\[\s*cfg\s*\(\s*(?:test|all\s*\(\s*test\s*,\s*unix\s*\))\s*\)\s*\]\s*(?:pub\s+)?mod\s+\w+\s*\{'
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


def declaration_has_raw_payload(source, name):
    """Audit an entire named Rust declaration, including nested variant fields."""
    code = mask_literals(source)
    matches = list(re.finditer(r'\b(?:enum|struct)\s+' + re.escape(name) + r'\b[^;{]*\{', code))
    if len(matches) != 1:
        raise ValueError(f'expected exactly one declaration of {name}')
    match = matches[0]
    index, depth = match.end(), 1
    while index < len(code) and depth:
        depth += (code[index] == '{') - (code[index] == '}')
        index += 1
    if depth:
        raise ValueError(f'unbalanced declaration of {name}')
    return bool(re.search(r'\b(?:PathBuf|Value|String|Vec\s*<\s*u8\b)', code[match.start():index]))


def impl_methods(source, name):
    """Audit inherent gate methods, including constructors spelled with Self."""
    code = mask_literals(production_source(source))
    result = []
    for match in re.finditer(r'\bimpl\s+' + re.escape(name) + r'\s*\{', code):
        index, depth = match.end(), 1
        while index < len(code) and depth:
            depth += (code[index] == '{') - (code[index] == '}')
            index += 1
        if depth:
            raise ValueError('unbalanced gate implementation')
        result.extend(re.findall(r'\bfn\s+(\w+)\s*\(', code[match.end():index]))
    return set(result)


def approval_boundary_violation(source, crate_name, module_name):
    """ADR-0012: authority cannot enter MCP/providers or an execution CLI."""
    code = mask_literals(production_source(source))
    authority = r'\b(?:ApprovalGrant|ApprovalAction|ApprovalScope|ApprovalId|AuthorizationAuthentication|TransactionStore|RecoveryStore|DurableIntent|grant_approval|revoke_approval|record_intent|authorization_signing_bytes)\b'
    if crate_name == 'provider-codex' and module_name in {'lifecycle.rs', 'scoped_cleanup.rs', 'scoped_unsupported.rs', 'desktop_cleanup.rs'}:
        # This adapter may consume handoff gates; it cannot mint authority or write history.
        forbidden = r'\b(?:ApprovalGrant|ApprovalAction|ApprovalScope|ApprovalId|AuthorizationAuthentication|TransactionStore|RecoveryStore|grant_approval|revoke_approval|record_intent|authorization_signing_bytes|AuthorityMac)\b'
        return bool(re.search(forbidden, code))
    if crate_name == 'mcp-server' or crate_name.startswith('provider-'):
        return bool(re.search(authority, code))
    if crate_name == 'cli':
        if re.search(r'\b(?:record_intent|DurableIntent|AuthorizationAuthentication|authorization_signing_bytes|AuthorityMac)\b', code):
            return True
        if module_name != 'approvals.rs' and re.search(r'\b(?:grant_approval|revoke_approval|create_transaction)\b', code):
            return True
        if module_name not in {'approvals.rs', 'lifecycle.rs'} and re.search(r'\bTransactionStore\b', code):
            return True
        if module_name == 'approvals.rs' and re.search(r'\b(?:yes|force|confirm|authorized|auto_approve|yes_really_delete)\s*:\s*bool\b', code):
            return True
    return False


def conversation_authority_violation(source):
    """ADR-0023: preparation/status only; no grants, intent or effects."""
    return bool(re.search(
        r'\b(?:grant_approval|revoke_approval|record_intent|DurableIntent|DurableCleanupPort|DurableEffectPort|DurableCompleteRemovalPort|record_compensation_intent)\b|\.dispatch\s*\(',
        mask_literals(production_source(source)),
    ))


def claude_lifecycle_violation(source):
    """ADR-0014: no Claude effects until project identity/scope/proofs are validated.

    Audit every production module, including code after test modules. Only the
    existing fixed read-only version/bundle probes may cross the process helper.
    This is source fitness, supplemented by compiled guard properties and review.
    """
    code = mask_literals(production_source(source))
    forbidden = (
        r'\b(?:Command|Child|Stdio|OpenOptions|DirBuilder|File|Write|DurableEffectPort|'
        r'DurableIntent|DurableObservation|EffectAttempt|NativeEffectBoundary|'
        r'LifecycleCapability|NativeLifecycleOperation)\b|'
        r'\b(?:remove_file|remove_dir|remove_dir_all|rename|copy|create_dir|'
        r'create_dir_all|set_len|set_permissions|write_all|write_fmt|truncate)\s*\(|'
        r'\b(?:std|tokio)\s*::\s*(?:process|net)\b|'
        r'\buse\s+std\s*::\s*fs\b|'
        r'\buse\s+std\s*::\s*\{[^;]*\bfs\b|'
        r'\.(?:write|write_vectored)\s*\(|'
        r'\bfn\s+(?:planning_context|dispatch|launch|execute|reconcile|'
        r'archive|unarchive|purge|delete|restore)\b'
    )
    if re.search(forbidden, code):
        return True
    return any(name not in {'probe_version', 'read_bundle_metadata', 'BundleMetadata'}
               for name in re.findall(r'\bprocess\s*::\s*(\w+)', code))


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

# Regression: inspect every nested variant, while ignoring literals and comments.
for raw_type in ['String', 'Value', 'Vec<u8>', 'PathBuf']:
    assert declaration_has_raw_payload(
        f'enum TransactionEvent {{ Safe {{ x: u64 }}, Nested {{ payload: {raw_type} }} }}',
        'TransactionEvent',
    ), f'nested raw payload fitness canary failed: {raw_type}'
assert not declaration_has_raw_payload(
    'enum TransactionEvent { Safe { x: u64 }, Nested { digest: Digest } } // String }',
    'TransactionEvent',
)
try:
    declaration_has_raw_payload('struct Other { value: u64 }', 'TransactionEvent')
except ValueError:
    pass
else:
    raise AssertionError('missing declaration fitness canary failed')
for crate_name in ['mcp-server', 'provider-api', 'provider-codex', 'provider-claude']:
    for canary in ['fn bypass() { store.grant_approval(); }', 'use agentmender_domain::ApprovalGrant;']:
        assert approval_boundary_violation(canary, crate_name, 'lib.rs')
for canary in ['fn bad() { store.grant_approval(); }', 'fn bad() { store.record_intent(); }', 'fn bad() { port.dispatch(receipt); }', 'use agentmender_persistence::DurableCleanupPort;']:
    assert conversation_authority_violation(canary), 'conversation authority canary failed'
assert not conversation_authority_violation('fn prepare() { store.create_transaction(plan); store.prepare_recovery(id, 0); }')
assert not conversation_authority_violation('#[cfg(test)] mod tests { fn fixture() { store.grant_approval(); } }')
assert approval_boundary_violation('fn consume() { store.record_intent(); }', 'cli', 'approvals.rs')
assert approval_boundary_violation('struct Flags { yes: bool }', 'cli', 'approvals.rs')
assert approval_boundary_violation('fn bypass() { store.grant_approval(); }', 'cli', 'main.rs')
assert not approval_boundary_violation('fn local() { store.grant_approval(); }', 'cli', 'approvals.rs')
assert not approval_boundary_violation('fn gated(intent: DurableIntent) {}', 'provider-codex', 'lifecycle.rs')
for canary in ['use agentmender_persistence::TransactionStore;', 'fn bypass() { store.grant_approval(); }']:
    assert approval_boundary_violation(canary, 'provider-codex', 'lifecycle.rs')
assert approval_boundary_violation('fn bypass() { store.record_intent(); }', 'cli', 'lifecycle.rs')
assert impl_methods('impl DurableIntent { pub fn bypass() -> Self { Self {} } }', 'DurableIntent') == {'bypass'}
for canary in [
    'fn later() { fs::remove_file(path); }',
    'fn later() { file.write_all(bytes); }',
    'use std::fs as storage; fn later() { storage::write(path, bytes); }',
    'use std::{fs as storage}; fn later() { storage::write(path, bytes); }',
    'use std::fs::OpenOptions as Options;',
    'fn later() { process::spawn_provider(args); }',
    'fn later() { let c = LifecycleCapability {}; }',
    'fn planning_context(&self) -> Context { context }',
    'fn dispatch(&self, intent: DurableIntent) {}',
    'fn later() { let observation = DurableObservation {}; }',
    'use std::process::Command as ClaudeCommand;',
    'fn later() { let effects = NativeEffectBoundary {}; }',
    'fn later() { File::options().truncate(true).open(history); }',
]:
    assert claude_lifecycle_violation(canary), 'Claude lifecycle fitness canary failed'
assert claude_lifecycle_violation('#[cfg(test)] mod tests {}\nfn later() { fs::rename(a, b); }')
assert not claude_lifecycle_violation('#[cfg(test)] mod tests { fn fixture() { fs::write(a, b); } }')
assert not claude_lifecycle_violation('fn read() { process::probe_version(kind, exe); }')
assert not claude_lifecycle_violation('fn read() { let text = "purge; fs::remove_file(path)"; }')

ROOT = pathlib.Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version=1", "--no-deps", "--locked"], cwd=ROOT
))
allowed = {
    "agentmender-domain": set(),
    "agentmender-provider-api": {"agentmender-domain"},
    "agentmender-application": {"agentmender-domain", "agentmender-provider-api", "agentmender-persistence"},
    "agentmender-provider-codex": {"agentmender-domain", "agentmender-provider-api", "agentmender-persistence"},
    "agentmender-provider-claude": {"agentmender-domain", "agentmender-provider-api"},
    "agentmender-mcp-server": {"agentmender-domain", "agentmender-application"},
    "agentmender-persistence": {"agentmender-domain"},
    "agentmender": {"agentmender-domain", "agentmender-provider-api", "agentmender-application", "agentmender-provider-codex", "agentmender-provider-claude", "agentmender-mcp-server", "agentmender-persistence"},
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
    internal = {d["name"] for d in package["dependencies"] if d["name"].startswith("agentmender-") and d.get('kind') != 'dev'}
    # ADR-0017: full-chain disposable acceptance uses application only in tests.
    for dep in package['dependencies']:
        if dep.get('kind') == 'dev' and dep['name'].startswith('agentmender-'):
            if not (name == 'agentmender-provider-codex' and dep['name'] == 'agentmender-application') and dep['name'] not in allowed[name]:
                errors.append(f"unreviewed development dependency: {name}: {dep['name']}")
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
    r'\b(?:unlinkat|renameat|renameat2|mkdirat|ftruncate)\s*\(',
    r'"(?:DELETE|UPDATE|INSERT)\s+',
]
desktop_refresh_path = ROOT / "crates/provider-codex/src/desktop_reconciliation.rs"
desktop_refresh_sql = (
    "UPDATE local_thread_catalog_sync_state SET last_full_reconciled_at = NULL "
    "WHERE host_id COLLATE BINARY = 'local' AND last_full_reconciled_at IS NOT NULL"
)
for path in (ROOT / "crates").glob("*/src/**/*.rs"):
    source = production_source(path.read_text(encoding="utf-8"))
    code = mask_literals(source)
    reviewed_search_index = path == ROOT / "crates/persistence/src/search_index.rs"
    reviewed_capture = path == ROOT / "crates/provider-codex/src/capture.rs"
    reviewed_lifecycle = path == ROOT / "crates/provider-codex/src/lifecycle.rs"
    reviewed_scoped = path in {ROOT / f"crates/provider-codex/src/{name}.rs" for name in ["scoped_cleanup", "scoped_unsupported", "desktop_cleanup"]}
    reviewed_scoped_storage = path == ROOT / "crates/provider-codex/src/scoped_storage.rs"
    reviewed_scoped_files = path == ROOT / "crates/provider-codex/src/scoped_files.rs"
    reviewed_desktop_catalog = path == ROOT / "crates/provider-codex/src/desktop_catalog.rs"
    reviewed_desktop_shared = path == ROOT / "crates/provider-codex/src/desktop_shared.rs"
    crate_name = path.relative_to(ROOT / "crates").parts[0]
    if crate_name == 'provider-claude' and claude_lifecycle_violation(source):
        errors.append(f'Claude lifecycle authority before validated project contract: {path.relative_to(ROOT)}')
    if approval_boundary_violation(source, crate_name, path.name):
        errors.append(f"independent approval boundary violation: {path.relative_to(ROOT)}")
    owned_files = {ROOT / f"crates/persistence/src/{name}.rs" for name in ["owned", "recovery"]}
    reviewed_owned = path in owned_files
    reviewed_cohort = path == ROOT / "crates/provider-codex/src/content/cohort.rs"
    if reviewed_cohort:
        for marker in ['Connection::open("")', 'PRAGMA temp_store=FILE',
                       'open_provider_db(root)', 'scope.excluded', 'search_batch(']:
            if marker not in source:
                errors.append(f'UUID cohort boundary lost: {marker}')
        if re.search(r'\b(?:fingerprint_metadata|read_snapshot|inventory_cached|bounded_backup)\s*\(', code):
            errors.append('UUID cohort reintroduced a global snapshot dependency')
    find_host = path == ROOT / "crates/cli/src/session_find/host.rs"
    find_jobs = path == ROOT / "crates/cli/src/session_find.rs"
    process_boundary = path == ROOT / "crates/provider-api/src/process.rs"
    if find_host or find_jobs:
        if conversation_authority_violation(source):
            errors.append(f"search job acquired lifecycle authority: {path.relative_to(ROOT)}")
        if re.search(r'\b(?:agentmender_persistence|DurableEffectPort|Command::new|thread/(?:archive|unarchive|delete|resume))\b', source):
            errors.append(f"search job acquired provider dispatch authority: {path.relative_to(ROOT)}")
    if find_jobs and not all(marker in source for marker in [
        "EvidenceSandbox::new(&launch.root, &launch.home)",
        'Connection::open(directory.path().join("verified-results.sqlite"))',
        "PRAGMA max_page_count",
    ]):
        errors.append("search results must use the bounded private job spool")
    if process_boundary:
        for method, required in {
            "spawn_luna_coordinator": ["evidence_command(python, sandbox, false)", '"/usr/bin/python3"', '"--internal-coordinator"'],
            "spawn_codex_luna": ["sandbox.validate()?", '"app-server"', "deny process-fork",
                "deny file-write*", "deny file-read-data", ".env_clear()", '"mcp_servers={}"',
                '"features.shell_tool=false"', '"features.multi_agent_v2=false"'],
        }.items():
            bodies = list(method_bodies(source, method))
            if len(bodies) != 1 or not all(marker in bodies[0] for marker in required):
                errors.append(f"fixed Luna launch boundary lost: {method}")
    for pattern in patterns:
        inspected = source
        if reviewed_lifecycle:
            # Only fixed create-new pin copies under the opaque owned sandbox may write.
            # All other code in this module retains the ordinary mutation prohibition.
            for body in method_bodies(source, 'pin_image'):
                inspected = inspected.replace(body, '')
            if pattern == patterns[3]:
                inspected = inspected.replace('"thread/unarchive"', '')
        if path == ROOT / "crates/provider-codex/src/metadata.rs" and pattern == patterns[3]:
            # Advertised method names in schema inspection are data, not dispatch.
            for body in method_bodies(source, "schema_methods"):
                inspected = inspected.replace(body, "")
        if reviewed_search_index and pattern == patterns[0]:
            # ADR-0027: only explicit offline rebuild may unlink the two fixed
            # derived files inside a validated OwnedStateRoot; checked below.
            for body in method_bodies(source, 'rebuild_search_index'):
                inspected = inspected.replace(body, '')
        if find_host and pattern in {patterns[0], patterns[1]}:
            for body in method_bodies(source, "install_assets"):
                if 'sandbox.validate()' not in body or 'for (name, bytes) in ASSETS' not in body or '.create_new(true)' not in body:
                    errors.append("embedded Luna asset write escaped the private sandbox")
                inspected = inspected.replace(body, "")
        if find_host and pattern == patterns[3]:
            for body in method_bodies(source, "handle_packet"):
                allowlist = 'Some("initialize" | "initialized" | "thread/start" | "turn/start")'
                if allowlist not in body or '"app_server_method_refused"' not in body:
                    errors.append("Luna bridge lost its fixed App Server method allowlist")
                inspected = inspected.replace(body, body.replace(allowlist, 'Some("fixed_model_method")'))
        if process_boundary and pattern == patterns[0]:
            for body in method_bodies(source, "spawn_codex_luna"):
                private = body
                for call in ["std::fs::create_dir(&home)", "std::fs::create_dir(&temp)"]:
                    private = private.replace(call, "")
                inspected = inspected.replace(body, private)
        if find_jobs and pattern == patterns[-1]:
            for sql in ['"DELETE FROM results WHERE id=?1"',
                        '"INSERT OR REPLACE INTO results(id,kind,row) VALUES (?1,?2,?3)"']:
                inspected = inspected.replace(sql, "")
        if path == desktop_refresh_path and pattern == patterns[-1]:
            # ADR-0040 admits one exact marker reset after confirmed native delete.
            for body in method_bodies(source, "request_full_reconciliation"):
                writes = re.findall(r'"((?:DELETE|UPDATE|INSERT)\s+[^"]*)"', body)
                if [" ".join(sql.split()) for sql in writes] != [desktop_refresh_sql]:
                    errors.append("Desktop reconciliation escaped its fixed SQL update")
                else:
                    inspected = inspected.replace(body, body.replace(f'"{writes[0]}"', "", 1))
        if reviewed_cohort and pattern == patterns[-1]:
            # ADR-0035 admits only the fixed writes to an anonymous worker spool.
            for sql in ['"INSERT INTO cohort(id,metadata) VALUES (?1,?2)"',
                        '"UPDATE cohort SET candidate=?2 WHERE id=?1 AND candidate IS NULL"',
                        '"UPDATE cohort SET receipt=?2 WHERE id=?1 AND candidate IS NOT NULL AND (receipt IS NULL OR receipt=?2)"']:
                inspected = inspected.replace(sql, '')
        if (re.search(pattern, inspected)
            and not (reviewed_capture and pattern in {patterns[1], patterns[2], patterns[-1]})
            and not (reviewed_owned and pattern in patterns[:3])
            and not (reviewed_scoped_storage and pattern == patterns[-1])
            and not (reviewed_scoped_files and pattern == patterns[-2])
            and not (reviewed_desktop_catalog and pattern == patterns[-1])
            and not (reviewed_desktop_shared and pattern == patterns[-2])
            and not (crate_name == "persistence" and path.name == "store.rs" and pattern == patterns[-1])
            and not (reviewed_search_index and pattern == patterns[-1])):
            errors.append(f"possible provider mutation: {path.relative_to(ROOT)}: {pattern}")
    if path != ROOT / "crates/provider-api/src/macos_sandbox.rs" and (re.search(r"\bunsafe\b", code) or re.search(r"allow\s*\(\s*unsafe_code", code)):
        errors.append(f"unsafe code outside reviewed OS boundary: {path.relative_to(ROOT)}")
    if reviewed_capture and not all(marker in source for marker in ["SQLITE_OPEN_READ_ONLY", "PRAGMA query_only=ON", "verify_source_write_denial", "create_new(true)"]):
        errors.append("disposable capture safety markers missing; review required")
    if "Command::new" in source and path != ROOT / "crates/provider-api/src/process.rs" and not reviewed_lifecycle:
        errors.append(f"process dispatch outside sandbox boundary: {path.relative_to(ROOT)}")
    if reviewed_lifecycle:
        launch = list(method_bodies(source, 'launch'))
        pins = list(method_bodies(source, 'pin_image'))
        if not launch or not all('DurableObservation' in body.split('{', 1)[0] for body in launch):
            errors.append('Codex native launcher lacks the nonconstructible handoff gate')
        native_launch = [body for body in launch if '"thread/unarchive"' in body]
        if len(native_launch) != 1 or source.count('"thread/unarchive"') != 1:
            errors.append('Codex native lifecycle method escaped its private launcher')
        elif not (0 <= native_launch[0].find('self.validate_executable()') < native_launch[0].find('.spawn()')):
            errors.append('Codex executable identity is not checked before provider startup')
        if len(pins) != 1 or 'EvidenceSandbox' not in pins[0].split('{', 1)[0]:
            errors.append('executable pin writer lacks its opaque owned sandbox')
        elif re.search(r'\b(?:remove_file|remove_dir|remove_dir_all|rename|copy|set_len|truncate|set_permissions)\s*\(', mask_literals(pins[0])):
            errors.append('unreviewed mutation inside executable pin helper')
        remaining = source
        for body in launch:
            remaining = remaining.replace(body, '')
        if 'Command::new' in remaining or len(re.findall(r'Command::new', source)) != 1:
            errors.append('Codex process dispatch escaped its private gated launcher')
        no_pin = source
        for body in pins:
            no_pin = no_pin.replace(body, '')
        if re.search(r'\b(?:DirBuilder|OpenOptions)\b|\.write_all\s*\(', mask_literals(no_pin)):
            errors.append('Codex owned writes escaped the fixed executable pin helper')
        if re.search(r'\bpub(?:\([^)]*\))?\s+fn\s+(?:launch|pin_image|lifecycle_response)\b', code):
            errors.append('private Codex transport or owned write capability exposed publicly')
        for marker in ['into_observation', 'validate_executable', 'grant_valid', 'NativeOutcome::Unknown', 'env_clear', 'EvidenceSandbox']:
            if marker not in source:
                errors.append(f'Codex durable handoff contract missing: {marker}')
    if crate_name == "persistence":
        if re.search(r"\b(?:agentmender_provider_\w+|agentmender_application|agentmender_mcp_server|std::process|Command)\b", code):
            errors.append(f"persistence crosses provider/inbound boundary: {path.relative_to(ROOT)}")
        # ADR-0011: remove only the verified temporary second link after atomic
        # no-replace publication. Final manifests and journal are never removed.
        mutation_code = code
        if path.name == "owned.rs":
            stage_cleanup = r"\bfs::remove_file\s*\(\s*&stage\.path\s*\)"
            if len(re.findall(stage_cleanup, mutation_code)) != 1:
                errors.append("reviewed recovery staging cleanup contract changed")
            mutation_code = re.sub(stage_cleanup, "stage_cleanup", mutation_code)
        if reviewed_search_index:
            for body in method_bodies(source, 'rebuild_search_index'):
                mutation_code = mutation_code.replace(mask_literals(body), '')
        if re.search(r"\b(?:remove_file|remove_dir|remove_dir_all|rename|copy|set_len|truncate)\s*\(", mutation_code):
            errors.append(f"persistent history or recovery rewrite/removal: {path.relative_to(ROOT)}")
        if path.name == "recovery.rs" and not all(marker in source for marker in ["RecoveryManifest", "create_recovery_stage", "publish_recovery", "sync_all", "verify"]):
            errors.append("typed durable recovery storage contract missing")
        if path.name not in {"owned.rs", "recovery.rs", "store.rs", "windows_permissions.rs", "lib.rs", "handoff.rs", "search_index.rs"}:
            errors.append(f"unreviewed persistence module: {path.relative_to(ROOT)}")
        if "Connection::" in code and path.name not in {"store.rs", "search_index.rs"}:
            errors.append(f"SQLite outside owned journal module: {path.relative_to(ROOT)}")
    # ADR-0023 admits only durable preparation in the application handoff;
    # MCP still cannot access persistence, grants, keys or a dispatch gate.
    lifecycle_composition = {ROOT / 'crates/cli/src/lifecycle.rs', ROOT / 'crates/application/src/lifecycle.rs', ROOT / 'crates/application/src/conversation.rs'}
    if path == ROOT / 'crates/application/src/conversation.rs' and conversation_authority_violation(source):
        errors.append('conversational preparation acquired approval or effect authority')
    if crate_name in {"cli", "mcp-server", "application"} and path != ROOT / "crates/cli/src/approvals.rs" and path not in lifecycle_composition and re.search(
        r"\b(?:TransactionStore|RecoveryStore|DurableIntent|create_transaction|record_intent|record_compensation_intent)\b", code
    ):
        errors.append(f"inbound interface can advance durable transactions: {path.relative_to(ROOT)}")
    provider_authority = r"\b(?:TransactionStore|RecoveryStore|CleanupTransaction|TransactionEvent|JournalRecord)\b" if (reviewed_lifecycle or reviewed_scoped) else r"\b(?:TransactionStore|RecoveryStore|DurableIntent|CleanupTransaction|TransactionEvent|JournalRecord)\b"
    if crate_name.startswith("provider-") and re.search(provider_authority, code):
        errors.append(f"provider adapter contains transaction authority: {path.relative_to(ROOT)}")
    if crate_name == "mcp-server":
        if (re.search(r"\b(?:agentmender_provider_(?:api|codex|claude)|agentmender_persistence|lifecycle|DurableEffectPort|EffectObservation|std::(?:fs|process|net)|tokio::(?:fs|process|net)|rusqlite|sqlite|Command|File|OpenOptions)\b", code)
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
    if re.search(r'\b(?:fn|struct|enum|trait)\s+(?:execute_plan|execute_transaction|run_effect|provider_effect|approve_plan|ApprovalBroker|DeleteFile|RunSql|RemoveDirectory|ExecuteCommand)\b', code):
        errors.append(f"execution or low-level planning primitive before its milestone: {path.relative_to(ROOT)}")
    if re.search(r'\b(?:struct|enum|trait)\s+ApprovalGrant\b', code) and path != ROOT / 'crates/domain/src/approval.rs':
        errors.append(f"approval model outside pure domain: {path.relative_to(ROOT)}")
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

# ADR-0027 reviewed derived index: no journal/history names, authority, provider
# access, arbitrary filenames or SQL accepted from inbound tools.
def search_index_boundary_violation(source):
    source = production_source(source)
    code = mask_literals(source)
    if re.search(r'\b(?:TransactionStore|RecoveryStore|DurableIntent|ApprovalGrant|Command|ProviderAdapter|SessionSearchReadPort)\b', code):
        return True
    if any(name not in {'search_sources','search_documents','search_source_state','search_terms'}
           for name in re.findall(r'(?:INSERT INTO|DELETE FROM|UPDATE(?!\s+SET\b)|CREATE TABLE IF NOT EXISTS|CREATE VIRTUAL TABLE IF NOT EXISTS)\s+(\w+)', source)):
        return True
    return bool(re.search(r'"(?:transactions\.(?:sqlite3|journal|lock)|authority\.key|recovery-[^"\n]*)"', source))
assert search_index_boundary_violation('db.execute("DELETE FROM threads", []);')
assert search_index_boundary_violation('db.execute("UPDATE transactions SET state=1", []);')
assert search_index_boundary_violation('fn acquire() { store.record_intent(); let x: DurableIntent; }')
assert not search_index_boundary_violation('db.execute("DELETE FROM search_sources WHERE source=?1", [source]);')
index_source = production_source((ROOT / 'crates/persistence/src/search_index.rs').read_text(encoding='utf-8'))
if search_index_boundary_violation(index_source):
    errors.append('derived search index crosses provider/authority/history boundary')
rebuild = list(method_bodies(index_source, 'rebuild_search_index'))
if len(rebuild) != 1 or not all(marker in rebuild[0] for marker in [
        'OwnedStateRoot::open', 'validate_sidecars', 'try_lock()',
        '["search.sqlite3-journal", "search.sqlite3"]',
        'open_child_existing', 'same_file', 'root.revalidate()', 'remove_file(target)']):
    errors.append('derived index offline rebuild lost its fixed private scope')
if len(re.findall(r'\bremove_file\s*\(', mask_literals(index_source))) != 1:
    errors.append('derived index removal escaped explicit fixed rebuild')
for marker in ['SQLITE_OPEN_NOFOLLOW','search_index_path','PRAGMA secure_delete=ON',
               "contentless_delete=1", 'try_lock_shared()', '262144']:
    if marker not in index_source:
        errors.append('derived search index privacy/resource boundary lost: ' + marker)

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
if 'unsafe_code = "forbid"' not in (ROOT / "Cargo.toml").read_text(encoding="utf-8"):
    raise SystemExit("workspace unsafe-code prohibition changed")
store = production_source((ROOT / "crates/persistence/src/store.rs").read_text(encoding="utf-8"))
for marker in ["try_lock", "synchronous=EXTRA", "BEGIN IMMEDIATE", "events_no_update", "events_no_delete", "validate_against", "verify_manifest", "SQLITE_OPEN_READ_ONLY", "SQLITE_OPEN_NOFOLLOW"]:
    if marker not in store:
        raise SystemExit(f"ADR-0011 journal safety contract missing: {marker}")
transactions = production_source((ROOT / "crates/domain/src/transaction.rs").read_text(encoding="utf-8"))
for name in ["TransactionEvent", "TransactionBinding", "RecoveryManifest"]:
    try:
        raw_payload = declaration_has_raw_payload(transactions, name)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if raw_payload:
        raise SystemExit(f"raw content or operation payload in {name}")
approval = production_source((ROOT / 'crates/domain/src/approval.rs').read_text(encoding="utf-8"))
for name in ['ApprovalScope', 'ApprovalGrant', 'ApprovalAction', 'AuthorizationAuthentication']:
    if declaration_has_raw_payload(approval, name):
        raise SystemExit(f'raw content or operation payload in {name}')
for marker in ['approval_eligible', 'approval_valid', 'authorization_signing_bytes', 'scope_and_time_valid']:
    if marker not in approval:
        raise SystemExit(f'ADR-0012 pure approval contract missing: {marker}')
for marker in ['verify_slice', 'grant_approval', 'revoke_approval', 'ApprovalAction::Consumed', 'authority_metadata', 'migrate_v1']:
    if marker not in store:
        raise SystemExit(f'ADR-0012 authenticated persistence contract missing: {marker}')
handoff = production_source((ROOT / 'crates/persistence/src/handoff.rs').read_text(encoding="utf-8"))
for name in ['EffectError', 'NativeOutcome', 'EffectAttempt', 'NativeObservation', 'EffectObservation', 'CleanupAttempt']:
    if declaration_has_raw_payload(handoff, name):
        raise SystemExit(f'raw provider payload in lifecycle handoff: {name}')
for name in ['DurableIntent', 'DurableObservation']:
    declaration = re.search(r'pub\s+struct\s+' + name + r'\s*\{([^}]+)\}', mask_literals(store))
    if declaration is None or re.search(r'\bpub\b', declaration.group(1)):
        raise SystemExit(f'nonconstructible lifecycle gate exposed: {name}')
    if re.search(r'#\[derive\([^]]*(?:Clone|Copy|Default|Serialize|Deserialize)[^]]*\)\]\s*pub\s+struct\s+' + name, store):
        raise SystemExit(f'cloneable/serializable lifecycle gate: {name}')
    constructors = list(method_bodies(store, 'into_observation')) if name == 'DurableObservation' else []
    if name == 'DurableObservation' and (len(constructors) != 1 or '(self)' not in constructors[0].split('{', 1)[0]):
        raise SystemExit('dispatch gate must be consumed into observation-only authority')
    if re.search(r'\bimpl\b[^\{;]*\b(?:Clone|Copy|Default|Deserialize)\b[^\{;]*\bfor\s+' + name + r'\b', mask_literals(store)):
        raise SystemExit(f'gate reconstruction trait implementation: {name}')
gate_methods = {'into_observation', 'expires_at', 'approval_id', 'plan_digest', 'state_fingerprint', 'operation_index', 'transaction_id', 'sequence'}
if impl_methods(store, 'DurableIntent') != gate_methods or impl_methods(store, 'DurableObservation'):
    raise SystemExit('unreviewed lifecycle gate constructor or conversion method')
intents = list(method_bodies(store, 'record_intent'))
if len(intents) != 1 or not (0 <= intents[0].find('self.append(r)?') < intents[0].find('Ok(DurableIntent {')):
    raise SystemExit('durable receipt construction no longer follows committed intent')
workflow = production_source((ROOT / 'crates/application/src/lifecycle.rs').read_text(encoding="utf-8"))
reconciliation = list(method_bodies(workflow, 'reconcile'))
if len(reconciliation) != 1 or re.search(r'\.(?:dispatch|record_intent)\s*\(', mask_literals(reconciliation[0])):
    raise SystemExit('reconciliation can dispatch or recreate execution authority')
for marker in ['record_intent', '.dispatch(', 'record_observation', 'evaluate_integrity', 'record_verification']:
    if marker not in workflow:
        raise SystemExit(f'lifecycle orchestration contract missing: {marker}')
execution = list(method_bodies(workflow, 'execute'))
if len(execution) != 1 or not (0 <= execution[0].find('.record_intent(') < execution[0].find('.dispatch(')):
    raise SystemExit('native dispatch does not follow durable intent in the execution workflow')
if not re.search(r'fn\s+dispatch\s*\([^;{]*\bDurableIntent\b', mask_literals(handoff)):
    raise SystemExit('lifecycle effect port lost its owned durable gate')

# ADR-0017 permits two fixed SQL deletes; ADR-0019 adds two exact metadata
# deletes behind a separately approved schema-3 profile. One relative payload
# unlink, never a generic local mutation port. Keep these exceptions auditable.
scoped_base = ROOT / 'crates/provider-codex/src'
scoped_storage = production_source((scoped_base / 'scoped_storage.rs').read_text(encoding="utf-8"))
scoped_files = production_source((scoped_base / 'scoped_files.rs').read_text(encoding="utf-8"))
scoped_adapter = production_source((scoped_base / 'scoped_cleanup.rs').read_text(encoding="utf-8"))
scoped_profile = production_source((scoped_base / 'scoped_profile.rs').read_text(encoding="utf-8"))
deletes = re.findall(r'"((?:DELETE|UPDATE|INSERT)\s+[^"\n]*)"', scoped_storage)
if sorted(deletes) != sorted(['DELETE FROM thread_read_receipts WHERE thread_id=?1', 'DELETE FROM threads WHERE id=?1',
                             'DELETE FROM thread_attachments WHERE thread_id=?1',
                             'DELETE FROM guardian_review_feedback WHERE thread_id=?1']):
    raise SystemExit('unreviewed scoped SQL mutation')
apply = list(method_bodies(scoped_storage, 'apply'))
if len(apply) != 1 or not all(sql in apply[0] for sql in deletes):
    raise SystemExit('scoped deletes escaped the private fixed action')
if 'if crate::scoped_profile::is_expanded(m)' not in apply[0]:
    raise SystemExit('expanded metadata effects lost their distinct profile gate')
unlink = list(method_bodies(scoped_files, 'validate_and_unlink'))
relative_mutation = r'\b(?:unlinkat|renameat|renameat2|mkdirat|ftruncate)\s*\('
if len(unlink) != 1 or len(re.findall(relative_mutation, mask_literals(scoped_files))) != 1 or 'fs::unlinkat(&parent, &name, AtFlags::empty())' not in unlink[0]:
    raise SystemExit('scoped file mutation escaped the one frozen relative unlink')
for module in [scoped_storage, scoped_files]:
    if re.search(r'\bpub\s+(?:fn|struct|enum|trait)\b', mask_literals(module)):
        raise SystemExit('provider scoped implementation exposed a public primitive')
for marker in ['validate_manifest_profile', 'locking_mode=EXCLUSIVE', 'BEGIN EXCLUSIVE', 'retained_digest', 'ROLLBACK TO', 'COMMIT']:
    if marker not in scoped_storage:
        raise SystemExit(f'scoped database fence contract missing: {marker}')
if impure_graph_code(scoped_profile):
    raise SystemExit('provider profile predicate is not pure')
for marker in ['validate_manifest_profile', 'validate_executable', 'WriterFence::acquire', 'DatabaseFence::acquire', 'not_expired', 'validate_cleanup_report']:
    if marker not in scoped_adapter:
        raise SystemExit(f'scoped adapter contract missing: {marker}')
for marker in ['complete_removal_manifest', 'schema_version != 3', 'self.expanded',
               'progress.record', 'CompleteRemovalAction::PrimaryPayload',
               'CompleteRemovalAction::StateThreadRecords']:
    if marker not in scoped_adapter:
        raise SystemExit(f'expanded cleanup contract missing: {marker}')
for name in ['reconcile_cleanup', 'reconcile_complete_removal']:
    for body in method_bodies(workflow, name):
        if re.search(r'\.(?:dispatch|record_intent)\s*\(', mask_literals(body)):
            raise SystemExit('cleanup reconciliation can recreate authority')
for name in ['execute_scoped_cleanup', 'execute_complete_removal']:
    execution = list(method_bodies(workflow, name))
    if len(execution) != 1 or not (0 <= execution[0].find('.record_intent(') < execution[0].find('.dispatch(')):
        raise SystemExit('scoped dispatch does not follow durable intent')
assert approval_boundary_violation('fn bypass() { store.grant_approval(); }', 'provider-codex', 'scoped_cleanup.rs')
assert approval_boundary_violation('use agentmender_persistence::TransactionStore;', 'provider-codex', 'scoped_unsupported.rs')
assert production_source('#[cfg(all(test, unix))] mod tests { fs::write("test", b"x"); }\nfn later() {}').strip() == 'fn later() {}'

# ADR-0020 admits only candidate catalog SQL and private shared-file replacement
# within the exact Desktop handoff. A later admission must keep these boundaries.
desktop_catalog = production_source((scoped_base / 'desktop_catalog.rs').read_text(encoding="utf-8"))
desktop_catalog_sql = [
    "UPDATE local_thread_catalog_sync_state SET observation_sequence=?1 WHERE host_id='local' AND observation_sequence=?2",
    "INSERT INTO local_thread_catalog_scan_entries(host_id,thread_id,removed) VALUES('local',?1,1) ON CONFLICT(host_id,thread_id) DO UPDATE SET removed=1",
    "DELETE FROM local_thread_catalog WHERE host_id='local' AND thread_id=?1 AND source_kind=?2 RETURNING missing_candidate",
    "UPDATE local_thread_catalog_metadata SET catalog_revision=?1 WHERE id=1 AND catalog_revision=?2",
]
if sorted(re.findall(r'"((?:DELETE|UPDATE|INSERT)\s+[^"\n]*)"', desktop_catalog)) != sorted(desktop_catalog_sql):
    raise SystemExit('unreviewed Desktop catalog SQL mutation')
catalog_apply = list(method_bodies(desktop_catalog, 'apply'))
if len(catalog_apply) != 1 or not all(sql in catalog_apply[0] for sql in desktop_catalog_sql):
    raise SystemExit('Desktop catalog mutation escaped its fenced action')
desktop_shared = production_source((scoped_base / 'desktop_shared.rs').read_text(encoding="utf-8"))
desktop_publish = list(method_bodies(desktop_shared, 'publish'))
if (len(desktop_publish) != 1 or len(re.findall(relative_mutation, mask_literals(desktop_shared))) != 2
    or 'fs::renameat(&fence.fd, stage_name.as_str(), &fence.fd, name)' not in desktop_publish[0]
    or 'fs::unlinkat(&fence.fd, stage_name.as_str(), AtFlags::empty())' not in desktop_publish[0]):
    raise SystemExit('Desktop replacement escaped the private stage and exact destination')
for name in ['acquire', 'publish_global', 'publish_index']:
    bodies = list(method_bodies(desktop_shared, name))
    if len(bodies) != 1 or 'DesktopExecution' not in bodies[0].split('{', 1)[0]:
        raise SystemExit(f'Desktop shared effect lacks the consumed gate: {name}')
global_publication = list(method_bodies(desktop_shared, 'publish_global'))[0]
if not (0 <= global_publication.find('if expected.target_units > 0')
        < global_publication.find('return Err(EffectError::UnsupportedOperation)')
        < global_publication.find('publish(')):
    raise SystemExit('uncoordinated Desktop global publication was enabled')
desktop_catalog_capture = production_source((scoped_base / 'desktop_catalog_capture.rs').read_text(encoding="utf-8"))
catalog_boundary = list(method_bodies(desktop_catalog_capture, 'acquire_catalog'))
if len(catalog_boundary) != 1 or 'DesktopExecution' not in catalog_boundary[0].split('{', 1)[0]:
    raise SystemExit('Desktop catalog write boundary lacks the consumed gate')
desktop_cleanup = production_source((scoped_base / 'desktop_cleanup.rs').read_text(encoding="utf-8"))
if re.search(r'\bstruct\s+DesktopExecution\s*<[^>]*>\s*\{[^}]*\bpub\b', mask_literals(desktop_cleanup)):
    raise SystemExit('Desktop execution context can be constructed outside its adapter')
desktop_dispatch = list(method_bodies(desktop_cleanup, 'dispatch'))
if len(desktop_dispatch) != 1 or not (0 <= desktop_dispatch[0].find('intent.into_observation()') < desktop_dispatch[0].find('&DesktopExecution {')):
    raise SystemExit('Desktop sequencing escaped the durable intent handoff')
if any(re.search(r'\.(?:dispatch|record_intent)\s*\(', mask_literals(body)) for body in method_bodies(desktop_cleanup, 'reconcile')):
    raise SystemExit('Desktop reconciliation can recreate authority')

# ADR-0018 adds product assessment, not a general projection mutation port.
# These structural canaries supplement the exact SQL/unlink allowlist above and
# the behavioral assessment properties; they are not an ownership proof.
def projection_mutation_violation(source):
    source = production_source(source)
    code = mask_literals(source)
    generic = r'\bfn\s+(?:edit_json|delete_jsonl_where|delete_cache|execute_sql)\s*\('
    substring = r'\.contains\s*\(\s*(?:target|thread_id|session_id|native_id)(?:\b|\.)'
    whole_container = (
        r'\b(?:remove_file|unlink|unlinkat)\s*\([^;]*'
        r'"(?:[^"\n]*/)?(?:session_index\.jsonl|history\.jsonl|\.codex-global-state\.json(?:\.bak)?|codex\.db)"'
    )
    broad_cache = r'\bremove_dir_all\s*\([^;]*"[^"\n]*(?:Cache|IndexedDB|Local Storage)[^"\n]*"'
    broad_map = r'\.remove\s*\(\s*"(?:electron-persisted-atom-state|pinned-thread-ids|thread-projects)"'
    broad_process = r'Command\s*::\s*new\s*\(\s*"(?:killall|pkill)"'
    visual_restart = r'\bfn\s+(?:restart_codex|reload_codex|terminate_codex|restart_for_visual_verification)\s*\('
    return bool(re.search(generic, code) or re.search(substring, code)
                or re.search(whole_container, source) or re.search(broad_cache, source)
                or re.search(broad_map, source) or re.search(broad_process, source)
                or re.search(visual_restart, code))


for canary in [
    'fn edit_json(path: Path, pointer: String) {}',
    'fn delete_jsonl_where(predicate: Predicate) {}',
    'fn delete_cache(path: Path) {}',
    'fn execute_sql(sql: String) {}',
    'fn bad() { line.contains(thread_id.as_str()); }',
    'fn bad() { fs::remove_file(root.join("session_index.jsonl")); }',
    'fn bad() { fs::remove_file(root.join("history.jsonl")); }',
    'fn bad() { fs::remove_file(root.join(".codex-global-state.json.bak")); }',
    'fn bad() { fs::remove_file(root.join("codex.db")); }',
    'fn bad() { fs::remove_dir_all(root.join("Cache")); }',
    'fn bad() { map.remove("electron-persisted-atom-state"); }',
    'fn bad() { Command::new("killall"); }',
    'fn bad() { Command::new("pkill"); }',
    'fn restart_for_visual_verification(app: App) {}',
    'fn restart_codex() {}',
    'fn reload_codex() {}',
    'fn terminate_codex() {}',
]:
    assert projection_mutation_violation(canary), 'projection mutation canary failed'
assert not projection_mutation_violation('fn exact() { record.id == target; }')
for crate in ['provider-codex', 'provider-api', 'application', 'cli']:
    for path in (ROOT / 'crates' / crate / 'src').glob('*.rs'):
        if projection_mutation_violation(path.read_text(encoding="utf-8")):
            raise SystemExit(f'unreviewed broad/generic projection mutation: {path}')

assessment = production_source((ROOT / 'crates/domain/src/session_removal.rs').read_text(encoding="utf-8"))
if impure_graph_code(assessment) or re.search(r'\bDeserialize\b', mask_literals(assessment)):
    raise SystemExit('product assessment gained I/O or imported proof')
legacy_assessment = list(method_bodies(assessment, 'assess_legacy_session_removal'))
if len(legacy_assessment) != 1 or 'SessionRemovalOutcome::Removed' in legacy_assessment[0]:
    raise SystemExit('historical primary receipt can claim complete removal')
complete_contract = production_source((ROOT / 'crates/domain/src/complete_removal.rs').read_text(encoding="utf-8"))
if impure_graph_code(complete_contract):
    raise SystemExit('complete removal contract gained I/O')
for marker in ['ProfileAdmission::PrimaryOnly', 'WriterRequirement::CompatibleLifetimeFence',
               'codex-scoped-cleanup-v2', 'validate_complete_removal_report']:
    if marker not in complete_contract:
        raise SystemExit(f'complete profile admission contract missing: {marker}')
for marker in ['required_surfaces', 'RequiredPersistentProjection', 'EphemeralUiProjection',
               'UnknownReconstructionSurface', 'ExactProfileStructuralEvidence',
               'reconstruction_source_coverage', 'structural_profile_evidence',
               'ReconstructionRisk::None', 'Preservation::Unchanged',
               'MaximumSafeCleanup::Complete', 'checked_add']:
    if marker not in assessment:
        raise SystemExit(f'product verification contract missing: {marker}')
cli_cleanup = list(method_bodies((ROOT / 'crates/cli/src/lifecycle.rs').read_text(encoding="utf-8"), 'report_cleanup'))
if len(cli_cleanup) != 1 or any(marker not in cli_cleanup[0] for marker in [
    'assess_legacy_session_removal', 'primary_cleanup_outcome', 'product_complete', 'Ok(!product_complete)',
]):
    raise SystemExit('CLI cleanup lost primary/product separation')


# ADR-0024: adaptive metadata is a fixed read contract, never mutation authority.
adaptive = production_source((ROOT / 'crates/provider-codex/src/metadata.rs').read_text(encoding="utf-8"))
for marker in ['validate_worker_paths', 'SQLITE_OPEN_READ_ONLY', 'SQLITE_OPEN_NOFOLLOW', 'PRAGMA query_only=ON', 'bounded_backup', 'LIMIT 501']:
    if marker not in adaptive: raise SystemExit('adaptive metadata isolation/bounds lost: ' + marker)
if re.search(r'"(?:DELETE|UPDATE|INSERT|ATTACH|CREATE)\s+', adaptive, re.I):
    raise SystemExit('adaptive metadata must not change source SQL state')
probe = list(method_bodies(production_source((ROOT / 'crates/provider-api/src/process.rs').read_text(encoding="utf-8")), 'codex_app_server_schema'))
if len(probe) != 1 or 'generate-json-schema' not in probe[0] or re.search(r'\.(?:send|write_all)\s*\(', probe[0]):
    raise SystemExit('native capability inspection acquired dispatch authority')

# ADR-0026: bounded content and caller registration add no write/dispatch port.
def search_write_violation(source):
    code = mask_literals(production_source(source))
    return bool(re.search(
        r'\b(?:Command|OpenOptions|DurableIntent|TransactionStore|DurableEffectPort)\b|'
        r'\b(?:remove_file|remove_dir_all|rename|create_dir|write|grant_approval|record_intent)\s*\(|'
        r'\.dispatch\s*\(', code
    ) or re.search(r'"(?:DELETE|UPDATE|INSERT|ATTACH|CREATE)\s+', production_source(source), re.I))

for canary in ['fn bad() { fs::write(path, bytes); }',
               'fn bad() { Command::new(command); }',
               'fn bad() { db.execute("DELETE FROM threads", []); }',
               'fn bad() { port.dispatch(intent); }']:
    assert search_write_violation(canary), 'search write canary failed'
assert not search_write_violation('fn read() { source.read_to_end(&mut bytes); }')
content = production_source((ROOT / 'crates/provider-codex/src/content.rs').read_text(encoding="utf-8"))
for relative in ['provider-codex/src/content.rs', 'application/src/session_search.rs', 'cli/src/management.rs']:
    if search_write_violation((ROOT / 'crates' / relative).read_text(encoding="utf-8")):
        raise SystemExit('read-only search/registration gained write authority: ' + relative)
for marker in ['validate_worker_paths', 'SQLITE_OPEN_READ_ONLY', 'SQLITE_OPEN_NOFOLLOW',
               'PRAGMA query_only=ON', 'bounded_backup', 'open_source_read',
               'spawn_codex_content_capture', 'SEARCH_BYTES', 'SESSION_SLICE_BYTES', 'STREAM_RECORD_BYTES',
               'RECORD_BYTES', 'LIMIT 10001', 'WHERE id NOT IN', 'header_identity_mismatch']:
    if marker not in content:
        raise SystemExit('content isolation/identity/bounds lost: ' + marker)
search_service = (ROOT / 'crates/application/src/session_search.rs').read_text(encoding="utf-8")
search_body = list(method_bodies(search_service, 'search'))
if len(search_body) != 1 or not (0 <= search_body[0].find('self.binding(') < search_body[0].find('.search_sessions(')):
    raise SystemExit('management binding must precede provider search access')
resolver = (ROOT / 'crates/application/src/session_resolution.rs').read_text(encoding="utf-8")
scan_body = list(method_bodies(resolver, 'scan'))
if len(scan_body) != 1 or '.read_cohort(' not in scan_body[0] or '.search(' in scan_body[0]:
    raise SystemExit('progressive resolution must only acquire pages from its UUID cohort')
registration = (ROOT / 'crates/cli/src/management.rs').read_text(encoding="utf-8")
for marker in ['verify_operator_descriptor', 'root_device', 'root_inode', 'server_instance', 'expires_at', 'single_conversation_operator_verified']:
    if marker not in registration:
        raise SystemExit('independent registration boundary lost: ' + marker)

print("Architecture, independent approval, gated Codex and read-only Claude checks passed (8 crates).")

# Evolved transcript bounds remain explicit, not unlimited memory allocation.
assert "const STREAM_RECORD_BYTES: usize = 8 * 1024 * 1024;" in content
assert "const SESSION_SLICE_BYTES: usize = 12 * 1024 * 1024;" in content
