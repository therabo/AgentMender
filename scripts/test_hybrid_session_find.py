#!/usr/bin/env python3
"""Exercise hybrid retrieval on disposable Codex rows, optionally using real Luna."""

import argparse
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unicodedata

from benchmark_progressive_resolution import Client, fixture
from test_autonomous_session_find import APP_SERVER, FindClient, auth, delete_outcomes

TITLE = "Trova conversazioni sulle carte Pokèmon"
TITLE_QUERY = "Utilizzando AgentMender trovami la conversazione con titolo " + TITLE
CONTENT_QUERY = ("Utilizzando AgentMender trovami le conversazioni in cui ho parlato "
                 "di carte pokemon per essere eliminate")


def populate(root):
    ids, caller = fixture(root, 7)
    exact, archived, discussion, uncertain, outside, unknown, rejected = sorted(ids)
    with sqlite3.connect(root / "state_5.sqlite") as db:
        def update(identity, title, *, project="Cards", created=1794000000, archive=0, text=None):
            path = Path(db.execute("SELECT rollout_path FROM threads WHERE id=?", (identity,)).fetchone()[0])
            if text is None:
                path.unlink()
            else:
                if identity == archived:
                    # A traversable prefix with an unreadable tail.
                    lines = path.read_text().splitlines()[:1] + ["{"]
                else:
                    message = {"type": "response_item", "payload": {"type": "message", "role": "user",
                               "content": [{"type": "input_text", "text": text}]}}
                    lines = path.read_text().splitlines()[:1] + [json.dumps(message, ensure_ascii=False)]
                path.write_text("\n".join(lines) + "\n")
            if identity == discussion:
                zstd = shutil.which("zstd")
                if zstd:
                    subprocess.run([zstd, "--quiet", "--rm", str(path)], check=True)
            db.execute("UPDATE threads SET title=?,cwd=?,created_at=?,archived=? WHERE id=?",
                       (title, f"/fixture/{project}" if project else None, created, archive, identity))

        update(exact, TITLE)
        update(archived, TITLE, archive=1, text="partial")
        update(discussion, "Catalogo raccoglitore",
               text="Ho parlato delle carte Poke\u0301mon: sto catalogando le mie carte Pokémon. "
                    "Ho due Charizard olografici e cerco un Pikachu Base Set.")
        update(uncertain, "Riferimento importato", text="Parole chiave da verificare: carte Pokemon.")
        update(outside, "Altro archivio", project="Elsewhere", created=1, text="Il set di carte Pokèmon.")
        update(unknown, None, project=None)
        update(rejected, "Esempio di filtro", text="Esempio per un test: carte Pokemon. Nessuna discussione.")
        update(caller, TITLE, text="Ho parlato delle carte Pokémon, per esempio la carta Bulbasaur.")
    return caller, exact, archived, discussion, uncertain, outside, unknown, rejected


def title(value=TITLE, kind="exact"):
    return {"op": "title", "value": value, "kind": kind}


TEXT = {"op": "text", "value": "carte Pokémon"}
DATE = {"op": "created", "range": {"start": 1793491200, "end": 1796083200}}
PROJECT = {"op": "project", "values": ["Cards"]}


def group(op, *args):
    return {"op": op, "args": list(args)}


def resolve(client, query, assess=lambda _: None, **intent):
    page = client.call(action="start", search_intent={
        "schema_version": "1", "subject": "Fixture Pokémon", "filter": query,
        "hints": {"terms": []}, **intent,
    })
    serial = 0
    matches, alternatives = {}, {}
    while page["execution_state"] == "in_progress":
        serial += 1
        assert serial < 200, page
        changes = {}
        if page["progress"]["publication_ready"] or page["phase"] == "publishing":
            changes["publish"] = True
        elif page["candidates"]:
            changes["assessments"] = [assess(row) for row in page["candidates"]]
            assert all(changes["assessments"]), page
        page = client.call(action="advance", run=page["run"], expected_revision=page["step_revision"],
                           operation_id=str(serial), **changes)
        matches.update((r["native_session_id"], r) for r in page.get("matches", []))
        alternatives.update((r["native_session_id"], r) for r in page.get("alternatives", []))
    return matches, alternatives, page


def rust_cases(binary, root):
    caller, exact, archived, discussion, uncertain, outside, unknown, rejected = populate(root)
    client = Client(binary, root, caller)
    seen = set()

    def assess(row):
        identity = row["native_session_id"]
        assert identity not in seen, row
        seen.add(identity)
        assert row["evidence"], row
        handle = row["evidence"][0]["handle"]
        judgment = {"native_session_id": identity, "evidence": [handle], "outcomes": [],
                    "explanation": "Disposable fixture judgment."}
        if identity == discussion:
            assert "Charizard" in row["evidence"][0]["excerpt"]
            return dict(judgment, disposition="verified_relevant", relationship="direct", activity="discussion")
        if identity == rejected:
            return dict(judgment, disposition="related")
        if identity == outside:
            return dict(judgment, disposition="insufficient", plausible_alternative=True,
                        missing_evidence="Only a single passing reference is available.")
        assert identity == uncertain, row
        return dict(judgment, disposition="insufficient", evidence=[],
                    missing_evidence="An imported tag cannot verify a discussion.")

    try:
        rows, _, page = resolve(client, title())
        assert set(rows) == {exact, archived}, rows
        assert rows[archived]["state"] == "archived", rows
        assert rows[exact]["title_match"] == "original_exact"
        assert page["progress"]["bytes_read"] == page["progress"]["candidates_evaluated"] == 0, page
        assert page["progress"]["sessions_examined"] == 7
        assert page["progress"]["sources_inspected"] == 3
        assert page["coverage"]["eligible_evaluation_complete"] is False
        for variant in ("TROVA CONVERSAZIONI SULLE CARTE POKEMON", unicodedata.normalize("NFD", TITLE)):
            rows, _, _ = resolve(client, title(variant))
            assert set(rows) == {exact, archived}, rows
            assert all(row["title_match"] == "normalized_exact" and row["title"] == TITLE
                       for row in rows.values())
        rows, _, _ = resolve(client, group("and", title("pokémon", "contains"), DATE))
        assert set(rows) == {exact, archived}
        assert all(row["title_match"] == "partial" for row in rows.values())

        rows, alternatives, page = resolve(client, group("or", title("Pokémon", "contains"), TEXT), assess)
        assert set(rows) == {exact, archived, discussion}, rows
        assert seen == {discussion, uncertain, outside, rejected}
        assert page["progress"]["candidates_evaluated"] == 4
        assert page["progress"]["published"] == 5
        assert page["progressive_coverage"]["not_published_reasons"] == {"semantic_rejection": 1}
        assert page["progressive_coverage"]["unresolved_predicates"] == 1
        assert rows[discussion]["match_basis"] == "semantic_evidence"
        assert alternatives[outside]["match_basis"] == "plausible_evidence"
        assert alternatives[uncertain]["match_basis"] == "retrieved_unverified"
        assert alternatives[uncertain]["title"] == "Riferimento importato"
        assert all(row["evidence_refs"] for row in (*rows.values(), *alternatives.values()))

        seen.clear()
        rows, alternatives, _ = resolve(client, group("and", PROJECT, DATE, TEXT), assess, activity="discussion")
        assert seen == {discussion, uncertain, rejected}
        assert set(rows) == {discussion} and set(alternatives) == {uncertain}
        rows, _, page = resolve(client, {"op": "not", "arg": PROJECT})
        assert set(rows) == {outside} and unknown not in rows
        assert page["progressive_coverage"]["unresolved_predicates"] == 1
        rows, _, page = resolve(client, {"op": "not", "arg": TEXT})
        assert exact not in rows and unknown not in rows and archived not in rows
        assert page["progressive_coverage"]["unresolved_predicates"] == 3
    finally:
        client.close()
    print("Hybrid Rust fixtures passed", flush=True)


def invalid_token(client, selection):
    client.serial += 1
    client.send({"jsonrpc": "2.0", "id": client.serial, "method": "tools/call",
                 "params": {"name": "session_delete", "arguments": selection,
                            "_meta": {"threadId": client.caller}}})
    reply = json.loads(client.child.stdout.readline())
    assert reply.get("id") == client.serial and reply.get("error", {}).get("code") == -32602, reply


def simulated_model_cases(binary, root, fake):
    caller, exact, archived, discussion, uncertain, *_ = populate(root)
    auth(root)
    client = FindClient(binary, root, caller, fake, elicit=True)
    try:
        first = client.terminal(client.tool("session_find", query=TITLE_QUERY))
        assert first["reason"] == "query_predicate_unknown", first
        assert set(first["metrics"]["luna"]) == {"planner"}, first
        rows = client.rows(first)["confirmed"]
        assert {row["native_session_id"] for row in rows} == {exact, archived}, rows
        assert all(row["match_basis"] == "verified_metadata" for row in rows)
        row = next(row for row in rows if row["native_session_id"] == exact)
        selection = {"native_session_id": exact, "evidence_ref": row["evidence_refs"][0]}
        token = selection["evidence_ref"]
        body = json.loads(bytes.fromhex(token.split(".")[0]))
        assert body["native_session_id"] == exact and body["provider"] == "codex"
        assert "position" not in body and "metadata_fingerprint" in body and str(root) not in str(body), body
        invalid_token(client, dict(selection, evidence_ref=token[:-1] + ("a" if token[-1] != "a" else "b")))
        invalid_token(client, dict(selection, native_session_id=archived))
        assert "error" in client.tool("session_delete", archived, **selection)
        assert "error" in client.tool("session_delete", exact, **selection)
        delete_outcomes(client, root, fake, caller, row, selection)

        second = client.terminal(client.tool("session_find", query=CONTENT_QUERY))
        assert second["reason"] == "source_unavailable", second
        assert set(second["metrics"]["luna"]) == {"planner", "evaluator"}, second
        rows = client.rows(second)
        assert discussion in {row["native_session_id"] for row in rows["confirmed"]}, rows
        assert uncertain in {row["native_session_id"] for row in rows["possible_unconfirmed"]}, rows
        with sqlite3.connect(root / "state_5.sqlite") as db:
            db.execute("UPDATE threads SET archived=1 WHERE id=?", (exact,))
        assert "error" in client.tool("session_delete", **selection)
    finally:
        client.close()
    restarted = FindClient(binary, root, caller, fake)
    try:
        invalid_token(restarted, selection)
    finally:
        restarted.close()
    print("Autonomous hybrid fixtures and metadata selection passed", flush=True)


def live(binary, root, codex, credential_file):
    caller, exact, archived, discussion, *_ = populate(root)
    # The existing sandbox requires a regular private auth.json in its source root.
    # The copy is scoped to this temporary fixture and removed by TemporaryDirectory.
    (root / "auth.json").touch(mode=0o600)
    shutil.copyfile(credential_file, root / "auth.json")
    client = FindClient(binary, root, caller, codex)
    try:
        for case, query in (("A", TITLE_QUERY), ("B", CONTENT_QUERY)):
            terminal = client.terminal(client.tool("session_find", query=query))
            result = client.tool("session_find_result", job_id=terminal["job_id"], offset=0)
            plan = result.get("search_intent")
            rows = client.rows(terminal)
            print(json.dumps({"case": case, "plan": plan, "state": terminal["state"],
                              "reason": terminal["reason"],
                              "luna": {role: {key: metrics[key] for key in ("model", "effort", "closed", "turns")
                                              if key in metrics}
                                       for role, metrics in terminal["metrics"]["luna"].items()},
                              "progress": {key: terminal["progress"][key] for key in
                                           ("sessions_examined", "sources_inspected", "candidates_staged", "candidates_evaluated",
                                            "bytes_read", "compressed_bytes", "published")
                                           if terminal["progress"] and key in terminal["progress"]},
                              "rows": {kind: [{key: row[key] for key in ("native_session_id", "title", "match_basis")}
                                              for row in group] for kind, group in rows.items()}},
                             ensure_ascii=False), flush=True)
            assert plan, terminal
            assert all((r["model"], r["effort"], r["closed"]) == ("gpt-6-luna", "medium", True)
                       for r in terminal["metrics"]["luna"].values()), terminal
            ids = {row["native_session_id"] for row in rows["confirmed"]}
            if case == "A":
                assert plan["filter"] == title(), plan
                assert set(terminal["metrics"]["luna"]) == {"planner"}, terminal
                assert ids == {exact, archived} and terminal["progress"]["bytes_read"] == 0, rows
            else:
                assert plan.get("activity") == "discussion", plan
                assert not plan.get("required_outcome") and not plan.get("excluded_outcome"), plan
                search_terms = json.dumps([plan["hints"], plan.get("filter")], ensure_ascii=False).lower()
                unaccented = "".join(c for c in unicodedata.normalize("NFD", search_terms)
                                     if not unicodedata.combining(c))
                assert "pokemon" in unaccented and "elimin" not in unaccented and "delet" not in unaccented, plan
                assert set(terminal["metrics"]["luna"]) == {"planner", "evaluator"}, terminal
                assert discussion in ids, rows
                assert all(row["native_session_id"] and row["title"] for group in rows.values() for row in group)
    finally:
        client.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agentmender", required=True, type=Path)
    parser.add_argument("--live-codex", type=Path)
    parser.add_argument("--credentials", type=Path)
    args = parser.parse_args()
    if bool(args.live_codex) != bool(args.credentials):
        parser.error("--live-codex and --credentials must be supplied together")
    if sys.platform != "darwin":
        parser.error("disposable MCP fixtures require the macOS isolation worker")
    binary = args.agentmender.resolve()
    with tempfile.TemporaryDirectory(dir="/private/tmp", prefix="agentmender-hybrid-") as tmp:
        base = Path(tmp)
        if args.live_codex:
            live(binary, base / "live", args.live_codex.resolve(), args.credentials)
        else:
            fake = base / "fake-codex"
            fake.write_text(APP_SERVER)
            fake.chmod(0o700)
            rust_cases(binary, base / "rust")
            simulated_model_cases(binary, base / "double", fake)


if __name__ == "__main__":
    main()
