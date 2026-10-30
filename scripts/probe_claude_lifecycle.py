#!/usr/bin/env python3
"""Opt-in investigation only. Never grants AgentMender lifecycle authority.

Only the exact installed executable is caller supplied. Every lifecycle selector/config/HOME is
fresh private synthetic state. No credentials, accounts, networks or real history.
Retains synthetic evidence under a new private temporary directory. Not run by CI.
"""
import argparse
import hashlib
import json
import os
import pathlib
import plistlib
import pwd
import re
import selectors
import stat
import subprocess
import sys
import tempfile
import time

PIN = '16ad925cea3d7752af589f8a63b0d5689b2ad3e95f01797847e6034715e84e12'
INFO_PIN = '1c4546041b57aabe93c3508d88bbc6c6b3e54bb0db29eda6a0cacb9142de1d83'
LIMIT = 262144
TIMEOUT = 10


def seatbelt_literal(path):
    text = str(path)
    if not text.isascii() or any(ord(c) < 32 or ord(c) == 127 for c in text):
        raise ValueError('unsupported Seatbelt path literal; ASCII without controls is required')
    return json.dumps(text)


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def snapshot(root):
    result = {}
    for p in sorted(root.rglob('*')):
        if p.is_symlink():
            raise RuntimeError('unexpected link in private fixture')
        if p.is_file():
            result[str(p.relative_to(root))] = digest(p)
    return result


def bounded(argv, cwd, env):
    proc = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    streams = {'stdout': bytearray(), 'stderr': bytearray()}
    sel = selectors.DefaultSelector()
    for name in streams:
        pipe = getattr(proc, name)
        os.set_blocking(pipe.fileno(), False)
        sel.register(pipe, selectors.EVENT_READ, name)
    deadline = time.monotonic() + TIMEOUT
    reason = None
    try:
        while sel.get_map():
            if time.monotonic() >= deadline:
                reason = 'timeout'
                break
            for key, _ in sel.select(min(0.1, max(0, deadline-time.monotonic()))):
                data = os.read(key.fileobj.fileno(), 65536)
                if not data:
                    sel.unregister(key.fileobj)
                    continue
                if sum(map(len, streams.values())) + len(data) > LIMIT:
                    reason = 'output-limit'
                    break
                streams[key.data].extend(data)
            if reason:
                break
        if reason:
            proc.kill()
        remaining = max(0.1, deadline-time.monotonic())
        try:
            proc.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            reason = 'timeout'
            proc.kill()
            proc.wait()
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        sel.close()
        proc.stdout.close()
        proc.stderr.close()
    return {'returncode': proc.returncode, 'bounded_failure': reason,
            **{k: bytes(v).decode('utf8', errors='replace') for k, v in streams.items()}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=pathlib.Path)
    parser.add_argument('--allow-isolated-native', action='store_true')
    args = parser.parse_args()
    if not args.allow_isolated_native:
        parser.error('explicit --allow-isolated-native is required')
    if sys.platform != 'darwin':
        parser.error('verified sandbox requires macOS')
    real_home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir).resolve()
    expected_bundle = real_home/'Library/Application Support/Claude/claude-code/2.1.222/claude.app'
    try:
        seatbelt_literal(real_home)
        seatbelt_literal(expected_bundle)
    except ValueError as error:
        parser.error(str(error))
    exe = args.executable
    # The read exception below must never be derived from an arbitrary relocation
    # of the pinned binary (for example, a fake bundle rooted at the real HOME).
    if exe != expected_bundle/'Contents/MacOS/claude':
        parser.error('only the exact installed Desktop-owned Code bundle is accepted')
    if not exe.is_absolute() or exe.is_symlink() or exe.resolve() != exe:
        parser.error('executable must be an absolute canonical regular file, without links')
    metadata = exe.stat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_size != 271289792 or digest(exe) != PIN:
        parser.error('executable differs from the exact reviewed Code2.1.222 SHA256 pin')
    bundle = exe.parents[2]
    info_path = bundle/'Contents/Info.plist'
    info_metadata = info_path.lstat()
    if not stat.S_ISREG(info_metadata.st_mode) or info_metadata.st_size > 65536 or digest(info_path) != INFO_PIN:
        parser.error('bounded exact bundle Info.plist pin mismatch')
    info = plistlib.loads(info_path.read_bytes())
    if (info.get('CFBundleIdentifier'), info.get('CFBundleShortVersionString'),
        info.get('CFBundleExecutable')) != ('com.anthropic.claude-code', '2.1.222', 'claude'):
        parser.error('exact bundle metadata mismatch')
    if bundle != expected_bundle or bundle == real_home or bundle in real_home.parents:
        parser.error('bundle must be the exact narrow installed read exception')
    root = pathlib.Path(tempfile.mkdtemp(prefix='agentmender-m9-native-', dir='/tmp')).resolve()
    os.chmod(root, 0o700)
    q = seatbelt_literal
    policy = ('(version 1)(allow default)'
              '(deny file-write* (require-not (subpath ' + q(root) + ')))'
              '(deny network*)(deny process-fork)(deny mach-lookup)'
              '(deny file-read-data (require-all (subpath ' + q(real_home) + ') '
              '(require-not (subpath ' + q(bundle) + '))))')
    (root/'sandbox.sb').write_text(policy)
    observations = []

    def environment(case):
        return {'HOME': str(case/'home'), 'CLAUDE_CONFIG_DIR': str(case/'config'),
                'TMPDIR': str(case/'tmp'), 'PATH': '/usr/bin:/bin',
                'DISABLE_AUTOUPDATER': '1', 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC': '1',
                'TERM': 'dumb', 'NO_COLOR': '1'}

    def fixture(name, shape='basic'):
        case = root/name
        case.mkdir(mode=0o700)
        for d in ['home', 'config', 'cwd', 'tmp']:
            (case/d).mkdir(mode=0o700)
        a = case/'cwd'/('a-b' if shape == 'collision' else 'target')
        b = case/'cwd'/'a'/'b' if shape == 'collision' else (
            a/'nested' if shape == 'nested' else case/'cwd'/'other')
        c = case/'cwd'/'retained'
        project_map = {}
        history = []
        for n, p in enumerate([a, b, c], 1):
            p.mkdir(parents=True, exist_ok=True)
            sid = f'{n}'*8 + '-' + f'{n}'*4 + '-4' + f'{n}'*3 + '-8' + f'{n}'*3 + '-' + f'{n}'*12
            key = re.sub('[^a-zA-Z0-9]', '-', str(p))
            files = {f'config/projects/{key}/{sid}.jsonl': json.dumps({
                'type': 'user', 'sessionId': sid, 'cwd': str(p), 'version': '2.1.222',
                'message': {'role': 'user', 'content': 'synthetic disposable marker'}})+'\n',
                f'config/tasks/{sid}/1.json': '{"synthetic":true}\n',
                f'config/debug/{sid}.txt': 'synthetic\n',
                f'config/file-history/{sid}/snapshot': 'synthetic\n',
                f'config/plans/{sid}.md': 'synthetic retained marker\n'}
            for name, value in files.items():
                f = case/name
                f.parent.mkdir(parents=True, exist_ok=True)
                f.write_text(value)
            (p/'source.txt').write_text('synthetic source retained\n')
            project_map[str(p)] = {'hasTrustDialogAccepted': True}
            history.append({'project': str(p), 'sessionId': sid, 'display': 'synthetic'})
        (case/'config/.claude.json').write_text(json.dumps({'projects': project_map}))
        (case/'home/.claude.json').write_text(json.dumps({'projects': project_map}))
        (case/'config/history.jsonl').write_text('\n'.join(map(json.dumps, history))+'\n')
        for name in ['config/shell-snapshots/retained.sh', 'config/backups/retained.json']:
            f = case/name
            f.parent.mkdir(parents=True, exist_ok=True)
            f.write_text('synthetic retained marker\n')
        mem = case/'config/projects'/re.sub('[^a-zA-Z0-9]', '-', str(a))/'memory/MEMORY.md'
        mem.parent.mkdir(parents=True, exist_ok=True)
        mem.write_text('synthetic disposable memory\n')
        return case, a

    def probe(label, case, native_args, extra_denial=None):
        rules = policy
        if extra_denial is not None:
            rules += '(deny file-write* (subpath '+q(extra_denial)+'))'
        profile = root/(label+'.sb')
        profile.write_text(rules)
        before = snapshot(case)
        result = bounded(['/usr/bin/sandbox-exec', '-f', str(profile), str(exe), *native_args],
                         case/'cwd', environment(case))
        after = snapshot(case)
        result.update(label=label, args=native_args,
                      removed=sorted(before.keys()-after.keys()),
                      created=sorted(after.keys()-before.keys()),
                      changed=sorted(k for k in before.keys() & after.keys() if before[k] != after[k]))
        (root/(label+'.json')).write_text(json.dumps(result, indent=2))
        observations.append(result)
        if result['bounded_failure']:
            raise RuntimeError('bounded native probe failed: '+label)
        return result

    # An external sibling owned canary tests real outside-root denial. No real provider path.
    canary_dir = pathlib.Path(tempfile.mkdtemp(prefix='agentmender-m9-native-canary-', dir='/tmp')).resolve()
    os.chmod(canary_dir, 0o700)
    canary = canary_dir/'untouched'
    canary.write_text('AgentMender-owned denial canary\n')
    prior = canary.stat().st_mtime_ns
    denial = bounded(['/usr/bin/sandbox-exec', '-f', str(root/'sandbox.sb'),
                      '/usr/bin/touch', str(canary)], root, {})
    denial['unchanged_mtime'] = prior == canary.stat().st_mtime_ns
    (root/'write-denial.json').write_text(json.dumps(denial, indent=2))
    if denial['returncode'] == 0 or not denial['unchanged_mtime']:
        raise RuntimeError('OS outside-write denial was not confirmed')
    canary.unlink()
    canary_dir.rmdir()

    meta, unused = fixture('metadata')
    for label, command in [('version', ['--version']), ('help', ['--help']),
                           ('project-help', ['project', '--help']),
                           ('purge-help', ['project', 'purge', '--help'])]:
        r = probe(label, meta, command)
        if r['returncode'] != 0:
            raise RuntimeError('metadata probe failed')
    if observations[0]['stdout'].strip() != '2.1.222 (Claude Code)':
        raise RuntimeError('version mismatch')
    basic, target = fixture('basic')
    probe('nonbare-startup', basic, ['project', 'purge', str(target), '--yes'])
    for label, flag in [('basic-dry', '--dry-run'), ('basic-apply', '--yes'),
                        ('basic-post-dry', '--dry-run')]:
        probe(label, basic, ['--bare', 'project', 'purge', str(target), flag])
    for shape in ['collision', 'nested', 'denied']:
        case, target = fixture(shape, 'nested' if shape == 'denied' else shape)
        first = case/'config/tasks/11111111-1111-4111-8111-111111111111'
        extra = first if shape == 'denied' else None
        for label, flag in [('dry', '--dry-run'), ('apply', '--yes')]:
            probe(shape+'-'+label, case, ['--bare', 'project', 'purge', str(target), flag], extra)
        if shape == 'denied':
            project_dir = case/'config/projects'/re.sub('[^a-zA-Z0-9]', '-', str(target))
            probe('denied-late-apply', case,
                  ['--bare', 'project', 'purge', str(target), '--yes'], project_dir)
    rechecked = digest(exe)
    by_label = {item['label']: item for item in observations}
    dry = by_label['basic-dry']
    first = by_label['denied-apply']
    late = by_label['denied-late-apply']
    findings = {
        'dry_run_changed_config': 'config/.claude.json' in dry['changed'],
        'dry_run_created_backup': any(p.startswith('config/backups/') for p in dry['created']),
        'collision_deleted_two_transcripts': sum(p.endswith('.jsonl') for p in by_label['collision-apply']['removed']) == 2,
        'nested_deleted_two_transcripts': sum(p.endswith('.jsonl') for p in by_label['nested-apply']['removed']) == 2,
        'denied_first_exit_zero_without_deletions_or_summary': first['returncode'] == 0
            and not first['removed'] and 'Deleted' not in first['stdout'],
        'denied_late_exit_zero_after_six_session_store_deletions_without_summary': late['returncode'] == 0
            and len(late['removed']) == 6 and 'Deleted' not in late['stdout']
            and all(p.startswith(('config/tasks/', 'config/debug/', 'config/file-history/')) for p in late['removed']),
    }
    (root/'findings-checks.json').write_text(json.dumps(findings, indent=2))
    summary = {'provenance': 'ActualInstalledNativeBehaviorOnSyntheticFixturesOnly',
               'production_execution_supported': False, 'executable_sha256': PIN,
               'source_executable_unchanged': rechecked == PIN,
               'native_probe_count': len(observations),
               'yes_invocations': sum('--yes' in o['args'] for o in observations),
               'checks_passed': sum(findings.values()), 'checks_failed': sum(not v for v in findings.values()),
               'root': str(root), 'labels': [o['label'] for o in observations]}
    (root/'summary.json').write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary, indent=2))
    if rechecked != PIN or not all(findings.values()):
        raise SystemExit('native evidence changed; no lifecycle authority may be inferred')


if __name__ == '__main__':
    main()
