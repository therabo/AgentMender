#!/usr/bin/env python3
"""Compare search continuation behavior on generated provider state only."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import uuid

from benchmark_progressive_resolution import Client, fixture

DB = "state_5.sqlite"
PROVIDER_FILES = (DB, f"{DB}-wal", f"{DB}-shm")


def fingerprints(root):
    result = {}
    for name in PROVIDER_FILES:
        path = root / name
        if path.exists():
            st = path.stat()
            result[name] = (st.st_ino, st.st_mtime_ns, st.st_ctime_ns, sha256(path.read_bytes()).hexdigest())
    return result


def append_message(path, text):
    item = {"type": "response_item", "payload": {"type": "message", "role": "assistant",
            "content": [{"type": "output_text", "text": text}]}}
    with path.open("a") as stream:
        stream.write(json.dumps(item) + "\n")


def call(client, **arguments):
    envelope = client.exchange("tools/call", {"name": "session_resolve",
                               "_meta": {"threadId": client.caller}, "arguments": arguments})
    return envelope["structuredContent"]


def advance(client, page, operation, **changes):
    return call(client, action="advance", run=page["run"], expected_revision=page["step_revision"],
                operation_id=str(operation), **changes)


def judge(page):
    return [{"native_session_id": row["native_session_id"], "disposition": "verified_relevant",
             "relationship": "direct", "evidence": [row["evidence"][0]["handle"]],
             "explanation": "The generated fixture contains a positive passage."}
            for row in page["candidates"]]


def probe(binary, case):
    with tempfile.TemporaryDirectory(prefix="agentmender-invalidation-") as directory:
        root = Path(directory).resolve() / "provider"
        truth, caller = fixture(root, 110 if case == "late_corpus_insert" else 16)
        ids = sorted(truth)
        db = sqlite3.connect(root / DB)
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("PRAGMA wal_autocheckpoint=0")
        db.execute("CREATE TABLE unrelated(id INTEGER)")
        db.execute("INSERT INTO unrelated VALUES (1)")
        db.commit()
        excluded_project = case == "excluded_title"
        archived = case == "archive_with_filter"
        stale_source = case in {"candidate_file_append", "persistent_index_stale_source"}
        unrelated = ids[-1]
        if case in {"irrelevant_title", "irrelevant_file_append", "excluded_title"}:
            source = root / "sessions" / f"rollout-{unrelated}.jsonl"
            with source.open() as stream:
                header = stream.readline()
            source.write_text(header)
            append_message(source, "A discussion about lunch.")
            if excluded_project:
                db.execute("UPDATE threads SET cwd='/fixture/Outside' WHERE id=?", (unrelated,))
                db.commit()
        index = None
        if case == "persistent_index_stale_source":
            index = root.parent / "index"
            subprocess.run([str(binary), "--provider", "codex", "--codex-root", str(root),
                            "search-index", "--root", str(index), "build", "--max-pages", "50"],
                           check=True, capture_output=True)
        intent = {"schema_version": "1", "subject": "Find FlightProbe implementations",
                  "hints": {"terms": ["FlightProbe", "Regression"]}}
        if archived:
            intent["constraints"] = {"archive_state": "active"}
        if excluded_project:
            intent["constraints"] = {"project_membership": {"mode": "include",
                                     "basis": "observed", "names": ["Fleet"]}}
        client = Client(binary, root, caller, index)
        try:
            page = call(client, action="start", search_intent=intent)
            staged = "phase" in page
            if staged and case != "late_corpus_insert":
                operation = 0
                while not page["candidates"]:
                    page = advance(client, page, operation)
                    assert page["execution_state"] == "in_progress", page
                    operation += 1
            elif not staged:
                for operation in range(8 if case == "late_corpus_insert" else 0):
                    page = advance(client, page, operation, assessments=judge(page))
            candidate = page["candidates"][0]["native_session_id"] if page["candidates"] else ids[0]
            before = fingerprints(root)
            modified = None
            if case in {"new_chat", "late_corpus_insert"}:
                new_id = str(uuid.UUID(int=len(ids) + 10))
                source = root / "sessions" / f"rollout-{new_id}.jsonl"
                source.write_text(json.dumps({"type": "session_meta", "payload": {"id": new_id,
                                  "cli_version": "0.166.0"}}) + "\n")
                append_message(source, "A different synthetic subject.")
                db.execute("INSERT INTO threads VALUES(?,?,?,1,2,0,'0.166.0',?)",
                           (new_id, "New fixture conversation", "/fixture/Fleet", str(source)))
            elif case in {"candidate_title", "irrelevant_title", "excluded_title"}:
                identity = candidate if case == "candidate_title" else unrelated
                db.execute("UPDATE threads SET title='New title without search terms' WHERE id=?", (identity,))
            elif case in {"timestamp", "caller_message"}:
                db.execute("UPDATE threads SET updated_at=updated_at+1 WHERE id=?",
                           (caller if case == "caller_message" else candidate,))
            elif case in {"archive_with_filter", "archive_without_filter"}:
                db.execute("UPDATE threads SET archived=1 WHERE id=?", (candidate,))
            elif case == "wal_only":
                db.execute("INSERT INTO unrelated VALUES(2)")
            if stale_source or case in {"caller_message", "irrelevant_file_append"}:
                identity = (caller if case == "caller_message" else
                            unrelated if case == "irrelevant_file_append" else candidate)
                modified = root / "sessions" / f"rollout-{identity}.jsonl"
                before_source = sha256(modified.read_bytes()).hexdigest()
                append_message(modified, "An additional synthetic message.")
            db.commit()
            if case == "wal_checkpoint":
                db.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchall()
            after = fingerprints(root)
            if staged and not page["candidates"]:
                operation = 20
                while not page["candidates"]:
                    page = advance(client, page, operation)
                    assert page["execution_state"] == "in_progress", page
                    operation += 1
            reply = advance(client, page, 9, assessments=judge(page))
            assert "error" not in reply, reply.get("error")
            work = reply["progress"].get("provider_work", {})
            print(json.dumps({
                "case": case,
                "physical_changed": [name for name in PROVIDER_FILES if before.get(name) != after.get(name)],
                "source_changed": bool(modified and sha256(modified.read_bytes()).hexdigest() != before_source),
                "execution": reply["execution_state"], "stop": reply["progress"]["stop_reason"],
                "freshness_failures": reply["progress"]["freshness_failures"],
                "source_reanchors": reply["progress"].get("source_reanchors", 0),
                "published": len(reply["matches"]), "prepared": len(reply.get("prepared_choices", [])),
                "next_candidates": len(reply["candidates"]),
                "snapshots_delta": work.get("sqlite_snapshots", 0) - page["progress"]["provider_work"]["sqlite_snapshots"],
                "fts_builds_delta": work.get("fts_builds", 0) - page["progress"]["provider_work"]["fts_builds"],
            }))
        finally:
            client.close()
            db.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    for case in ("fts_rerank", "new_chat", "candidate_title", "timestamp", "caller_message",
                 "candidate_file_append", "archive_with_filter", "archive_without_filter", "wal_only",
                 "wal_checkpoint", "irrelevant_title", "irrelevant_file_append", "excluded_title",
                 "late_corpus_insert", "persistent_index_stale_source"):
        probe(args.binary.resolve(), case)


if __name__ == "__main__":
    main()
