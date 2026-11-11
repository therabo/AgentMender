#!/usr/bin/env python3
"""Benchmark actual AgentMender STDIO on generated, read-only provider fixtures."""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import resource
import sqlite3
import statistics
import subprocess
import tempfile
import threading
import time
import uuid


def fixture(root, count, hidden_bytes=0):
    root.mkdir()
    (root / "sessions").mkdir()
    with sqlite3.connect(root / "state_5.sqlite") as database:
        database.execute("""CREATE TABLE threads(
            id TEXT PRIMARY KEY, title TEXT, cwd TEXT, created_at INTEGER,
            updated_at INTEGER, archived INTEGER, cli_version TEXT, rollout_path TEXT
        )""")
        ids = [str(uuid.UUID(int=i)) for i in range(1, count + 1)]
        caller = str(uuid.UUID(int=count + 1))
        for identity in [*ids, caller]:
            path = root / "sessions" / f"rollout-{identity}.jsonl"
            rows = [{"type": "session_meta", "payload": {"id": identity, "cli_version": "0.166.0"}}]
            for text in (
                "FlightProbe camera limits were implemented in this session.",
                "Regression checks passed after the projection fix.",
                "A later test found a failure. The workaround was reverted.",
            ):
                rows.append({"type": "response_item", "payload": {"type": "message", "role": "assistant",
                             "content": [{"type": "output_text", "text": text}]}})
            for offset in range(0, hidden_bytes, 128 * 1024):
                rows.append({"type": "response_item", "payload": {"type": "message",
                             "role": "system", "content": [{"type": "input_text",
                             "text": "x" * min(128 * 1024, hidden_bytes - offset)}]}})
            path.write_text("".join(json.dumps(row) + "\n" for row in rows))
            database.execute("INSERT INTO threads VALUES(?,?,?,1,2,0,'0.166.0',?)",
                             (identity, "Duplicate fixture title", "/fixture/Fleet", str(path)))
    return set(ids), caller


class Client:
    def __init__(self, binary, root, caller, index_root=None):
        options = [] if index_root is None else ["--search-index-root", str(index_root)]
        self.child = subprocess.Popen(
            [str(binary), "--provider", "codex", "--codex-root", str(root),
             "--codex-executable", str(root / "unused"), *options, "mcp", "--session-search"],
            text=True, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self.caller = caller
        self.calls = Counter()
        self.serial = 0
        self.exchange("initialize", {"protocolVersion": "2025-11-25", "capabilities": {},
                                      "clientInfo": {"name": "disposable-retrieval-benchmark", "version": "1"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def send(self, payload):
        self.child.stdin.write(json.dumps(payload) + "\n")
        self.child.stdin.flush()

    def exchange(self, method, params):
        self.serial += 1
        self.send({"jsonrpc": "2.0", "id": self.serial, "method": method, "params": params})
        line = self.child.stdout.readline()
        assert line, "MCP closed before replying"
        message = json.loads(line)
        assert message.get("id") == self.serial and "result" in message, message
        return message["result"]

    def call(self, tool="session_resolve", **arguments):
        self.calls[arguments.get("action", tool)] += 1
        response = self.exchange("tools/call", {"name": tool, "arguments": arguments,
                                 "_meta": {"threadId": self.caller}})
        result = response.get("structuredContent")
        assert result is not None and "error" not in result, response
        return result

    def close(self):
        self.child.stdin.close()
        assert self.child.wait(timeout=10) == 0


class RssSampler:
    """Sample the fixture MCP process and its direct reader separately."""

    def __init__(self, pid):
        self.pid = pid
        self.stop = threading.Event()
        self.parent_kib = 0
        self.worker_kib = 0
        self.thread = threading.Thread(target=self.sample)

    def sample(self):
        while not self.stop.is_set():
            table = subprocess.check_output(["/bin/ps", "-axo", "pid=,ppid=,rss="], text=True)
            for row in table.splitlines():
                pid, parent, rss = map(int, row.split())
                if pid == self.pid:
                    self.parent_kib = max(self.parent_kib, rss)
                elif parent == self.pid:
                    self.worker_kib = max(self.worker_kib, rss)
            self.stop.wait(0.05)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_):
        self.stop.set()
        self.thread.join()


def assessments(page, progressive, context_handles=None):
    return [
        {"native_session_id": candidate["native_session_id"],
         "disposition": "verified_relevant", "outcomes": ["implemented"],
         "evidence": context_handles or [candidate["evidence"][0]["handle"]],
         "explanation": "Scripted fixture ground truth.",
         **({"relationship": "direct"} if progressive else {})}
        for candidate in page["candidates"]
    ]


def run(client, truth, progressive, contextual, exact):
    started = time.perf_counter()
    first_verified = None
    seen = set()
    terminal = None
    progress = {}
    citations = 0

    def collect(page):
        nonlocal first_verified, citations
        for match in page["matches"]:
            identity = match["native_session_id"]
            assert identity in truth and identity not in seen, identity
            refs = [json.loads(bytes.fromhex(token.split(".", 1)[0]))
                    for token in match["evidence_refs"]]
            assert all(ref["native_session_id"] == identity for ref in refs)
            assert {ref["record_index"] for ref in refs} == ({2, 3, 4} if contextual else {2}), refs
            seen.add(identity)
            citations += len(match["evidence_refs"])
        if seen and first_verified is None:
            first_verified = time.perf_counter() - started

    if exact:
        page = client.call("session_discover", structured={"filter": {"op": "native_id", "values": sorted(truth)}})
        seen.update(row["native_session_id"] for row in page["sessions"])
        first_verified = time.perf_counter() - started
        terminal = "metadata"
    else:
        request = {"schema_version": "1", "subject": "Find the FlightProbe implementation",
                   "activity": "implementation", "hints": {"terms": ["FlightProbe"]}}
        if len(truth) > 1:
            request["coverage_goal"] = "exhaustive"
        page = client.call(action="start", search_intent=request,
                           **({"progressive": True} if progressive else {}))
        run_id = page["run"]
        operation = 0

        def advance(page, **changes):
            nonlocal operation
            operation += 1
            return client.call(action="advance", run=run_id, expected_revision=page["step_revision"],
                               operation_id=str(operation), **changes)

        handles = None
        if contextual:
            anchor = page["candidates"][0]["evidence"][0]["handle"]
            windows = [{"anchor": anchor, "window": {"direction": "occurrences",
                         "query": query, "max_bytes": 4096, "max_messages": 5}}
                       for query in ("Regression", "reverted")]
            if progressive:
                response = advance(page, context_requests=windows)
                results = response["windows"]
                page["step_revision"] = response["step_revision"]
            else:
                results = [client.call(action="context", run=run_id, **window) for window in windows]
            handles = [anchor, *[result["messages"][0]["handle"] for result in results]]

        if progressive:
            while terminal is None:
                page = advance(page, assessments=assessments(page, True, handles))
                collect(page)
                terminal = page["outcome"]
        else:
            while True:
                updates = assessments(page, False, handles)
                if updates:
                    client.call(action="assess", run=run_id, assessments=updates)
                if not page["progress"]["more_candidates"]:
                    break
                page = client.call(action="next", run=run_id)
            page = client.call(action="finish", run=run_id)
            collect(page)
            while page.get("next_offset") is not None:
                page = client.call(action="finish", run=run_id, offset=page["next_offset"],
                                   revision=page["revision"])
                collect(page)
            terminal = page["outcome"]
        progress = page["progress"]
    return {"terminal": terminal, "mcp_calls": sum(client.calls.values()),
            "calls": dict(client.calls), "recall": len(seen) / len(truth), "citations": citations,
            "p5": min(5, len(seen)) / 5, "r10": min(10, len(seen)) / len(truth),
            "provider_pages": progress.get("provider_pages"),
            "publication_checks": progress.get("publication_checks"),
            "worker": progress.get("provider_work"),
            "parsed_bytes": progress.get("bytes_read"),
            "decoded_bytes": progress.get("decoded_bytes"),
            "elapsed_ms": (time.perf_counter() - started) * 1000,
            "first_verified_ms": None if first_verified is None else first_verified * 1000}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--mode", choices=("legacy", "progressive"), required=True)
    parser.add_argument("--scenario", choices=("exact", "simple", "context", "exhaustive"), required=True)
    parser.add_argument("--count", type=int, default=90)
    parser.add_argument("--repeats", type=int, default=20)
    parser.add_argument("--sample-rss", action="store_true", help="sample fixture process RSS separately from latency runs")
    parser.add_argument("--index", action="store_true", help="build an opt-in index of the generated fixture")
    parser.add_argument("--hidden-bytes", type=int, default=0, help="ignored event bytes per fixture source")
    args = parser.parse_args()
    if args.count < 1 or args.repeats < 1 or not 0 <= args.hidden_bytes <= 8 * 1024 * 1024:
        parser.error("count and repeats must be positive, hidden bytes must fit 8 MiB")
    with tempfile.TemporaryDirectory(prefix="agentmender-progressive-benchmark-") as temporary:
        root = Path(temporary).resolve() / "provider"
        truth, caller = fixture(root, args.count if args.scenario == "exhaustive" else 1,
                                args.hidden_bytes)
        index_root = None
        if args.index:
            index_root = root.parent / "index"
            subprocess.run([str(args.binary.resolve()), "--provider", "codex", "--codex-root", str(root),
                            "--codex-executable", str(root / "unused"), "search-index", "--root",
                            str(index_root), "build", "--max-pages", str(len(truth) + 10)],
                           check=True, capture_output=True, text=True)
        results = []
        for _ in range(args.repeats):
            usage_before = resource.getrusage(resource.RUSAGE_CHILDREN)
            client = Client(args.binary.resolve(), root, caller, index_root)
            try:
                def run_once():
                    return run(client, truth, args.mode == "progressive",
                               args.scenario == "context", args.scenario == "exact")

                if args.sample_rss:
                    with RssSampler(client.child.pid) as sampler:
                        result = run_once()
                    result.update(parent_peak_rss_kib=sampler.parent_kib, worker_peak_rss_kib=sampler.worker_kib)
                else:
                    result = run_once()
            finally:
                client.close()
            usage_after = resource.getrusage(resource.RUSAGE_CHILDREN)
            result.update(child_cpu_user_s=usage_after.ru_utime - usage_before.ru_utime,
                          child_cpu_system_s=usage_after.ru_stime - usage_before.ru_stime)
            results.append(result)
        summary = results[-1].copy()
        for metric in ("elapsed_ms", "first_verified_ms"):
            values = sorted(row[metric] for row in results if row[metric] is not None)
            summary.pop(metric)
            summary[f"{metric}_p50"] = statistics.median(values) if values else None
            summary[f"{metric}_p95"] = values[math.ceil(len(values) * .95) - 1] if len(values) >= 20 else None
        assert all(row["recall"] == summary["recall"] for row in results)
        for metric in ("child_cpu_user_s", "child_cpu_system_s"):
            summary[metric] = statistics.median(row[metric] for row in results)
        print(json.dumps({"scenario": args.scenario, "mode": args.mode,
                          "count": len(truth), "replicates": args.repeats,
                          "private_index": args.index, "hidden_bytes": args.hidden_bytes, **summary}))


if __name__ == "__main__":
    main()
