#!/usr/bin/env python3
"""Provider observation, independent approval and gated lifecycle fitness (ADRs 0001-0014)."""
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
    if crate_name == 'provider-codex' and module_name == 'lifecycle.rs':
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
    reviewed_lifecycle = path == ROOT / "crates/provider-codex/src/lifecycle.rs"
    crate_name = path.relative_to(ROOT / "crates").parts[0]
    if crate_name == 'provider-claude' and claude_lifecycle_violation(source):
        errors.append(f'Claude lifecycle authority before validated project contract: {path.relative_to(ROOT)}')
    if approval_boundary_violation(source, crate_name, path.name):
        errors.append(f"independent approval boundary violation: {path.relative_to(ROOT)}")
    owned_files = {ROOT / f"crates/persistence/src/{name}.rs" for name in ["owned", "recovery"]}
    reviewed_owned = path in owned_files
    for pattern in patterns:
        inspected = source
        if reviewed_lifecycle:
            # Only fixed create-new pin copies under the opaque owned sandbox may write.
            # All other code in this module retains the ordinary mutation prohibition.
            for body in method_bodies(source, 'pin_image'):
                inspected = inspected.replace(body, '')
            if pattern == patterns[3]:
                inspected = inspected.replace('"thread/unarchive"', '')
        if (re.search(pattern, inspected)
            and not (reviewed_capture and pattern in patterns[1:])
            and not (reviewed_owned and pattern in patterns[:3])
            and not (crate_name == "persistence" and path.name == "store.rs" and pattern == patterns[-1])):
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
        if re.search(r"\b(?:remove_file|remove_dir|remove_dir_all|rename|copy|set_len|truncate)\s*\(", mutation_code):
            errors.append(f"persistent history or recovery rewrite/removal: {path.relative_to(ROOT)}")
        if path.name == "recovery.rs" and not all(marker in source for marker in ["RecoveryManifest", "create_recovery_stage", "publish_recovery", "sync_all", "verify"]):
            errors.append("typed durable recovery storage contract missing")
        if path.name not in {"owned.rs", "recovery.rs", "store.rs", "windows_permissions.rs", "lib.rs", "handoff.rs"}:
            errors.append(f"unreviewed persistence module: {path.relative_to(ROOT)}")
        if "Connection::" in code and path.name != "store.rs":
            errors.append(f"SQLite outside owned journal module: {path.relative_to(ROOT)}")
    lifecycle_composition = {ROOT / 'crates/cli/src/lifecycle.rs', ROOT / 'crates/application/src/lifecycle.rs'}
    if crate_name in {"cli", "mcp-server", "application"} and path != ROOT / "crates/cli/src/approvals.rs" and path not in lifecycle_composition and re.search(
        r"\b(?:TransactionStore|RecoveryStore|DurableIntent|create_transaction|record_intent|record_compensation_intent)\b", code
    ):
        errors.append(f"inbound interface can advance durable transactions: {path.relative_to(ROOT)}")
    provider_authority = r"\b(?:TransactionStore|RecoveryStore|CleanupTransaction|TransactionEvent|JournalRecord)\b" if reviewed_lifecycle else r"\b(?:TransactionStore|RecoveryStore|DurableIntent|CleanupTransaction|TransactionEvent|JournalRecord)\b"
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

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
if 'unsafe_code = "forbid"' not in (ROOT / "Cargo.toml").read_text():
    raise SystemExit("workspace unsafe-code prohibition changed")
store = production_source((ROOT / "crates/persistence/src/store.rs").read_text())
for marker in ["try_lock", "synchronous=EXTRA", "BEGIN IMMEDIATE", "events_no_update", "events_no_delete", "validate_against", "verify_manifest", "SQLITE_OPEN_READ_ONLY", "SQLITE_OPEN_NOFOLLOW"]:
    if marker not in store:
        raise SystemExit(f"ADR-0011 journal safety contract missing: {marker}")
transactions = production_source((ROOT / "crates/domain/src/transaction.rs").read_text())
for name in ["TransactionEvent", "TransactionBinding", "RecoveryManifest"]:
    try:
        raw_payload = declaration_has_raw_payload(transactions, name)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if raw_payload:
        raise SystemExit(f"raw content or operation payload in {name}")
approval = production_source((ROOT / 'crates/domain/src/approval.rs').read_text())
for name in ['ApprovalScope', 'ApprovalGrant', 'ApprovalAction', 'AuthorizationAuthentication']:
    if declaration_has_raw_payload(approval, name):
        raise SystemExit(f'raw content or operation payload in {name}')
for marker in ['approval_eligible', 'approval_valid', 'authorization_signing_bytes', 'scope_and_time_valid']:
    if marker not in approval:
        raise SystemExit(f'ADR-0012 pure approval contract missing: {marker}')
for marker in ['verify_slice', 'grant_approval', 'revoke_approval', 'ApprovalAction::Consumed', 'authority_metadata', 'migrate_v1']:
    if marker not in store:
        raise SystemExit(f'ADR-0012 authenticated persistence contract missing: {marker}')
handoff = production_source((ROOT / 'crates/persistence/src/handoff.rs').read_text())
for name in ['EffectError', 'NativeOutcome', 'EffectAttempt', 'NativeObservation', 'EffectObservation']:
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
workflow = production_source((ROOT / 'crates/application/src/lifecycle.rs').read_text())
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
print("Architecture, independent approval, gated Codex and read-only Claude checks passed (8 crates).")
