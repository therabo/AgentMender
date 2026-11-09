#!/usr/bin/env python3
"""Explicit read-only MCP driver. Emits aggregate receipt; never transcript/IDs.
This is NOT actual model/client conversational acceptance. No persistent index.
"""
import argparse
import hashlib
import json
import selectors
import subprocess
import time
from pathlib import Path


def fingerprints(root):
    result = []
    for name in ('state_5.sqlite', 'state_5.sqlite-wal', 'state_5.sqlite-shm'):
        path = root / name
        if not path.exists():
            result.append(None)
            continue
        before = path.lstat()
        if path.is_symlink() or before.st_size > 64 * 1024 * 1024:
            raise ValueError('source fingerprint refused')
        digest = hashlib.sha256()
        with path.open('rb') as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b''):
                digest.update(block)
        after = path.lstat()
        fields = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        result.append((fields(before), fields(after), digest.digest()))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--max-pages', type=int, default=12)
    args = parser.parse_args()
    if not 1 <= args.max_pages <= 30 or not all(p.is_absolute() for p in (args.binary, args.root, args.executable)):
        parser.error('absolute paths and 1..30 pages required')
    before = fingerprints(args.root)
    child = subprocess.Popen([str(args.binary), '--provider', 'codex', '--codex-root', str(args.root),
                              '--codex-executable', str(args.executable), 'mcp', '--session-search'],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    selector = selectors.DefaultSelector()
    selector.register(child.stdout, selectors.EVENT_READ)
    request_id = 0

    def rpc(method, params):
        nonlocal request_id
        request_id += 1
        child.stdin.write((json.dumps({'jsonrpc': '2.0', 'id': request_id, 'method': method, 'params': params}) + '\n').encode())
        child.stdin.flush()
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            if not selector.select(timeout=max(0.01, deadline - time.monotonic())):
                break
            line = child.stdout.readline(512 * 1024)
            if not line:
                raise RuntimeError('MCP connection ended')
            value = json.loads(line)
            if value.get('id') == request_id:
                return value
        raise TimeoutError('bounded MCP request timed out')

    def call(name, arguments):
        started = time.perf_counter()
        response = rpc('tools/call', {'name': name, 'arguments': arguments})
        elapsed = (time.perf_counter() - started) * 1000
        result = response.get('result', {})
        value = result.get('structuredContent')
        if value is None:
            texts = [b['text'] for b in result.get('content', []) if b.get('type') == 'text']
            value = json.loads(texts[0]) if texts else {'error': 'protocol_error'}
        return value, elapsed

    receipt = {'kind': 'real-provider-read-only-protocol-driver', 'actual_conversational_client': False,
               'persistent_index_enabled': False, 'source_writes_requested': False}
    try:
        rpc('initialize', {'protocolVersion': '2025-03-26', 'capabilities': {},
                           'clientInfo': {'name': 'agentmender-explicit-read-probe', 'version': '1'}})
        child.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        child.stdin.flush()
        tools = rpc('tools/list', {}).get('result', {}).get('tools', [])
        search = next(t for t in tools if t['name'] == 'session_search')
        context = next(t for t in tools if t['name'] == 'session_context')
        receipt['updated_schema_observed'] = 'structured' in search['inputSchema'].get('properties', {}) and 'window' in context['inputSchema'].get('properties', {})
        value, elapsed = call('session_search', {'purpose': 'management', 'include_content': True})
        receipt['missing_binding'] = {'error': value.get('error'), 'result_count': value.get('result_count'), 'latency_ms': elapsed}
        temporal, elapsed = call('session_search', {'structured': {'filter': {'op': 'created', 'range': {
            'start_date': '2026-10-10', 'end_date': '2026-10-20', 'timezone': 'Europe/Rome'}}, 'mode': 'exhaustive', 'sort': 'created'}, 'page_size': 5})
        receipt['calendar_filter'] = {'results': len(temporal.get('sessions', [])), 'unresolved': len(temporal.get('unresolved_candidates', [])),
                                      'query_interpretation_observed': temporal.get('query_interpretation') is not None, 'latency_ms': elapsed, 'error': temporal.get('error')}
        negative, elapsed = call('session_search', {'structured': {'filter': {'op': 'not', 'arg': {
            'op': 'project', 'values': ['MISSION1'], 'source': 'observed_label'}}, 'mode': 'exhaustive'}, 'page_size': 5})
        receipt['negative_project'] = {'results': len(negative.get('sessions', [])), 'unresolved': len(negative.get('unresolved_candidates', [])), 'latency_ms': elapsed, 'error': negative.get('error')}
        query = {'query': '', 'include_content': True, 'page_size': 20,
                 'structured': {'filter': {'op': 'text', 'value': 'AgentMender', 'alternatives': ['the', 'and', 'che', 'Cesium', 'session']}, 'mode': 'ranked', 'sort': 'relevance'}}
        latencies, reasons = [], set()
        totals = {'pages': 0, 'verified_results': 0, 'unresolved_candidates': 0, 'bytes_read': 0, 'fully_traversed': 0}
        context_receipt = {'observed': False}
        for _ in range(args.max_pages):
            value, elapsed = call('session_search', query)
            latencies.append(elapsed)
            if value.get('error'):
                receipt['content_error'] = value['error']
                break
            coverage = value.get('coverage', {})
            totals['pages'] += 1
            totals['verified_results'] += len(value.get('sessions', []))
            totals['unresolved_candidates'] += len(value.get('unresolved_candidates', []))
            totals['bytes_read'] += coverage.get('bytes_read', 0)
            totals['fully_traversed'] += coverage.get('fully_traversed_sessions', 0)
            reasons.update(coverage.get('reasons', []))
            candidate = next((s for s in value.get('sessions', []) if s.get('evidence')), None)
            if candidate and not context_receipt['observed']:
                window, latency = call('session_context', {'target': candidate['target'], 'evidence_ref': candidate['evidence'][0]['evidence_ref'],
                                                         'window': {'before': 1, 'after': 2, 'max_messages': 4, 'max_bytes': 4096}})
                context_receipt = {'observed': bool(window.get('messages')), 'messages': len(window.get('messages', [])),
                                   'partial': window.get('partial'), 'latency_ms': latency, 'error': window.get('error')}
                if not context_receipt['observed']:
                    for attempt in range(2):
                        window, latency = call('session_context', {'target': candidate['target'], 'evidence_ref': candidate['evidence'][0]['evidence_ref'], 'window': {'max_messages': 1, 'max_bytes': 2048}})
                        if window.get('messages'):
                            context_receipt = {'observed': True, 'messages': len(window['messages']), 'partial': window.get('partial'), 'latency_ms': latency, 'retry_attempt': attempt + 1}
                            break
                if context_receipt['observed']:
                    break
            if not value.get('next_cursor'):
                break
            query['cursor'] = value['next_cursor']
        receipt['content'] = dict(totals, query_latency_ms=latencies, coverage_reasons=sorted(reasons))
        receipt['context'] = context_receipt
        receipt['source_snapshot_stable'] = before == fingerprints(args.root)
    finally:
        child.stdin.close()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.terminate()  # Only this disposable AgentMender process, never Codex.
            child.wait(timeout=5)
        selector.close()
    print(json.dumps(receipt, indent=2))


if __name__ == '__main__':
    main()
