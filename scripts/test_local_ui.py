#!/usr/bin/env python3
"""HTTP acceptance on disposable archives, with optional native Codex cascade tests."""
import argparse
from contextlib import closing, contextmanager
from concurrent.futures import ThreadPoolExecutor
import hashlib
import http.client
import json
import os
from pathlib import Path
import platform
import selectors
import shlex
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlsplit

sys.dont_write_bytecode = True
import native_ui_fixture as probe

CLAUDE_ID = "eb000000-1111-4111-8111-000000000001"
CLAUDE_DESKTOP_ID = "eb000000-1111-4111-8111-000000000002"
CLAUDE_SIDECHAIN_ID = "eb000000-1111-4111-8111-000000000003"
CLAUDE_AGENT_ID = "a123badc"


def scoped_key(provider, root, ident, namespace=None):
    identity = dict(provider=provider, state_root=str(root), namespace=namespace, native_id=ident)
    return hashlib.sha256(json.dumps(identity, separators=(",", ":")).encode()).hexdigest()


def claude_fixture(root):
    bucket = root / "projects" / "project"
    bucket.mkdir(parents=True)
    records = [
        {"type": "user", "sessionId": CLAUDE_ID, "entrypoint": "cli",
         "cwd": "/fixture/Atlas", "timestamp": "2026-11-10T09:00:00Z",
         "message": {"content": "Never expose full Claude messages."}},
        {"type": "custom-title", "sessionId": CLAUDE_ID, "customTitle": "Atlas Café"},
    ]
    (bucket / f"{CLAUDE_ID}.jsonl").write_text("".join(json.dumps(row) + "\n" for row in records))
    desktop = [
        {"type": "user", "sessionId": CLAUDE_DESKTOP_ID, "entrypoint": "claude-desktop", "cwd": "/fixture/First"},
        {"type": "assistant", "sessionId": CLAUDE_DESKTOP_ID, "entrypoint": "claude-desktop", "cwd": "/fixture/Second"},
    ]
    (bucket / f"{CLAUDE_DESKTOP_ID}.jsonl").write_text("".join(json.dumps(row) + "\n" for row in desktop))

    child = bucket / CLAUDE_ID / "subagents" / f"agent-{CLAUDE_AGENT_ID}.jsonl"
    child.parent.mkdir(parents=True)
    child.write_text(json.dumps({"type": "user", "sessionId": CLAUDE_ID,
        "agentId": CLAUDE_AGENT_ID, "isSidechain": True, "customTitle": "Hidden worker"}) + "\n")
    (bucket / f"{CLAUDE_SIDECHAIN_ID}.jsonl").write_text(json.dumps({
        "type": "user", "sessionId": CLAUDE_SIDECHAIN_ID, "isSidechain": True}) + "\n")


def codex_metadata_fixture(root, count=3000):
    root.mkdir()
    with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
        db.execute("""CREATE TABLE threads(id TEXT PRIMARY KEY,title TEXT,cwd TEXT,
            created_at INTEGER,updated_at INTEGER,archived INTEGER,rollout_path TEXT,
            source TEXT,forked_from_id TEXT,attached_thread_parent_id TEXT)""")
        db.execute("CREATE TABLE thread_spawn_edges(parent_thread_id TEXT,child_thread_id TEXT)")
        db.executemany("INSERT INTO threads VALUES(?,?,?,?,?,?,?,?,?,?)", (
            (probe.session_id(index), f"Café session {index}" if index else None,
             "/fixture/Atlas", index, index + 86400, index % 2, None, "cli", None, None)
            for index in range(count)
        ))
        hidden = [f"00000000-1111-4111-8111-{index:012d}" for index in range(6)]
        parent = probe.session_id(1)
        db.executemany("INSERT INTO threads VALUES(?,?,?,?,?,?,?,?,?,?)", (
            (ident, "Hidden Codex worker", "/fixture/Atlas", 90000, 90000, 1, None,
             "cli", fork, attached)
            for ident, fork, attached in zip(hidden,
                (None, None, None, parent, None, "malformed fork"),
                (None, None, None, None, parent, None), strict=True)
        ))
        db.executemany("INSERT INTO thread_spawn_edges VALUES(?,?)", [
            (parent, hidden[0]), ("malformed", hidden[1]),
            (parent, hidden[2]), (probe.session_id(4000), hidden[2]),
        ])


class Client:
    def __init__(self, address):
        self.origin = f"http://{address.netloc}"
        self.host = address.netloc
        self.authorization = None

    def call(self, method, path, body=None, code=200, **overrides):
        headers = {"Host": self.host}
        if method != "GET":
            headers["Origin"] = self.origin
        if self.authorization:
            headers["Authorization"] = self.authorization
        if body is not None:
            headers["Content-Type"] = "application/json"
            body = json.dumps(body)
        for key, value in overrides.items():
            if value is None:
                headers.pop(key, None)
            else:
                headers[key] = value
        conn = http.client.HTTPConnection(self.host, timeout=35)
        try:
            conn.request(method, path, body=body, headers=headers)
            response = conn.getresponse()
            raw = response.read()
            allowed = code if isinstance(code, tuple) else (code,)
            assert response.status in allowed, (method, path, response.status, raw[:350])
            assert response.getheader("Access-Control-Allow-Origin") is None
            assert response.getheader("Cache-Control") == "no-store"
            if isinstance(code, tuple):
                return response.status
            if (response.getheader("Content-Type") or "").startswith("application/json"):
                return json.loads(raw)
            return raw
        finally:
            conn.close()

    def operation(self, token):
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            value = self.call("GET", f"/api/operations/{token}")
            if value["status"] not in ("preparing", "running"):
                return value
            time.sleep(0.1)
        raise AssertionError("native operation did not finish")


@contextmanager
def ui(binary, base, codex_root, claude_root, codex=None, max_entries=None):
    other = base / "empty-path"
    other.mkdir(exist_ok=True)
    home = base / "home"
    home.mkdir(exist_ok=True)
    environment = {"HOME": str(home), "USERPROFILE": str(home), "PATH": str(other)}
    if os.name == "nt":
        environment["SystemRoot"] = os.environ["SystemRoot"]
    command = [str(binary), "--codex-root", str(codex_root), "--claude-root", str(claude_root), "ui", "--no-open"]
    if max_entries is not None:
        command += ["--max-entries", str(max_entries)]
    if codex:
        # The fixture setup and the HTTP-dispatched native command both use
        # write-denying macOS profiles, each scoped to its disposable archive.
        command += ["--codex-executable", str(codex), "--protect-codex-session", probe.S]
    else:
        command += ["--codex-executable", str(base / "missing-codex")]
    proc = subprocess.Popen(command, env=environment, stdout=subprocess.PIPE, text=True)
    try:
        if os.name != "nt":
            with selectors.DefaultSelector() as ready:
                ready.register(proc.stdout, selectors.EVENT_READ)
                assert ready.select(15), "UI server did not publish a launch URL"
        address = urlsplit(proc.stdout.readline().strip())
        assert address.scheme == "http" and address.hostname == "127.0.0.1", address._replace(fragment="")
        client = Client(address)
        yield client, address.fragment.removeprefix("ticket=")
    finally:
        try:
            proc.terminate()
            try:
                proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
                raise
        finally:
            proc.stdout.close()


def authorize(client, ticket):
    assert b'<html lang="it">' in client.call("GET", "/")
    assert b"AgentMender" in client.call("GET", "/app.js")
    client.call("GET", "/api/codex/sessions", code=401)
    client.call("GET", "/", code=403, Host="attacker.test")
    client.call("POST", "/api/authorize", {"ticket": ticket}, code=403, Origin=None)
    client.call("POST", "/api/authorize", {"ticket": ticket}, code=403, Origin="http://attacker.test")
    client.call("POST", "/api/authorize", {"ticket": "invalid"}, code=403)
    identity = client.call("POST", "/api/authorize", {"ticket": ticket})
    client.authorization = identity["authorization"]
    client.call("POST", "/api/authorize", {"ticket": ticket}, code=403)


def inventory_acceptance(client, claude_root):
    first = client.call("GET", "/api/claude/sessions?search=CAFE%CC%81")
    assert first["total"] == 1 and first["provider"]["coverage"] == "partial"
    row = first["rows"][0]
    assert row["title"] == "Atlas Café" and row["workspace"] == "/fixture/Atlas"
    assert row["state"] == "unknown" and row["created_at"] is None
    detail = client.call("GET", f"/api/claude/sessions/{row['key']}")
    assert not detail["conversation_preview"] and detail["session"]["removal_reason"]
    client.call("POST", f"/api/claude/sessions/{row['key']}/removal-preview", code=409)
    client.call("GET", f"/api/claude/sessions/{row['key']}/conversation", code=409)
    client.call("GET", "/api/claude/sessions/../../config.toml", code=404)
    client.call("GET", "/api/claude/sessions?root=%2Ftmp", code=400)
    desktop = client.call("GET", f"/api/claude/sessions?search={CLAUDE_DESKTOP_ID}")
    assert desktop["total"] == 1 and desktop["rows"][0]["workspace"] is None
    sources = list(claude_root.glob("projects/*/*.jsonl"))
    assert len(sources) == 3 and "Never expose" in next(path for path in sources if path.stem == CLAUDE_ID).read_text()
    info = client.call("GET", "/api/claude/sessions?limit=1")
    assert info["provider"]["total"] == 2 and info["total"] == 2
    assert client.call("GET", "/api/claude/sessions?search=Hidden")["total"] == 0
    agent_key = scoped_key("claude", claude_root, f"{CLAUDE_ID}/agent:{CLAUDE_AGENT_ID}", "project")
    client.call("GET", f"/api/claude/sessions/{agent_key}", code=404)
    return row["key"]


def codex_acceptance(client, root, rollouts, binary):
    import fcntl

    query = "/api/codex/sessions"
    page = client.call("GET", query)
    assert page["provider"]["removal_available"], page["provider"]
    visible = {probe.R, probe.G, probe.F, probe.M, probe.ONE, probe.PROTECTED_PARENT, probe.REPLACEMENT}
    keys = {item["native_id"]: item["key"] for item in page["rows"]}
    assert set(keys) == visible and page["provider"]["total"] == len(visible)
    assert client.call("GET", f"{query}?limit=1&offset=1")["total"] == len(visible)
    for hidden in (probe.C, probe.ORPHAN, probe.SURVIVING_CHILD, probe.S):
        assert client.call("GET", f"{query}?search={hidden}")["total"] == 0
        child = f"{query}/{scoped_key('codex', root, hidden)}"
        client.call("GET", child, code=404)
        client.call("POST", child + "/removal-preview", code=404)

    def session(ident):
        return f"{query}/{keys[ident]}"

    def preview(ident, descendants=0):
        result = client.call("POST", session(ident) + "/removal-preview")
        assert result["main_id"] == ident
        assert result["spawned_descendants"] == descendants, result
        assert result["total_sessions"] == descendants + 1
        return result

    def refuse(ident, phrase):
        reply = client.call("POST", session(ident) + "/removal-preview", code=409)
        assert phrase in reply["error"], reply

    def confirm(review, expected, concurrent=False):
        path = f"/api/operations/{review['id']}/confirm"
        phrase = {"confirmation": review["confirmation"]}
        if concurrent:
            with ThreadPoolExecutor(max_workers=2) as pool:
                replies = [pool.submit(client.call, "POST", path, phrase, code=(202, 409))
                           for _ in range(2)]
                assert sorted(reply.result() for reply in replies) == [202, 409]
        else:
            client.call("POST", path, phrase, code=202)
        outcome = client.operation(review["id"])
        assert outcome["status"] == expected, outcome
        client.call("POST", path, phrase, code=409)
        return outcome

    def database(sql, params=()):
        with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
            return db.execute(sql, params).fetchall()

    def refresh():
        client.call("POST", "/api/codex/refresh")

    refuse(probe.PROTECTED_PARENT, "protetta")
    refuse(probe.F, "archiviate")
    target = session(probe.R)
    client.call("POST", target + "/removal-preview", code=401, Authorization=None)
    client.call("POST", target + "/removal-preview", code=403, Origin="null")
    client.call("POST", target + "/removal-preview", code=403, Origin=None)
    transcript = client.call("GET", target + "/conversation")
    assert any("Visible disposable" in message["text"] for message in transcript["messages"])
    assert "secret-marker" not in json.dumps(transcript)
    cursor = transcript["next"]
    assert cursor and all(len(message["text"].encode()) <= 16384 for message in transcript["messages"])
    client.call("GET", session(probe.G) + f"/conversation?cursor={cursor}", code=400)
    continuation = client.call("GET", target + f"/conversation?cursor={cursor}")
    assert continuation["messages"] and "secret-marker" not in json.dumps(continuation)
    review = preview(probe.R)
    path = f"/api/operations/{review['id']}/confirm"
    client.call("POST", "/api/operations/forged/confirm", {"confirmation": review["confirmation"]}, code=409)
    client.call("POST", path, {"confirmed": True}, code=422)
    client.call("POST", path, {"confirmation": "wrong"}, code=409)
    with rollouts[probe.R].open("a") as out:
        out.write("\n")
    confirm(review, "changed")
    assert rollouts[probe.R].is_file()
    refresh()

    # These references exist only in the rollout archive, beyond the canonical rows.
    ref = probe.session_id(100)
    fork = probe.seed(root.parent, ref, archived=True, forked_from_id=probe.C)
    refuse(probe.G, "fork esterno")
    fork.unlink()
    attached = probe.seed(root.parent, ref, archived=True, forked_from_id=probe.C, attached_thread=True)
    refuse(probe.G, "sessione agganciata")
    attached.unlink()
    ghost = probe.seed(root.parent, ref, parent=probe.C, archived=True)
    refuse(probe.G, "relazioni")
    ghost.unlink()

    database("UPDATE thread_spawn_edges SET parent_thread_id=? WHERE child_thread_id=?", ("bad-parent", probe.C))
    refuse(probe.G, "relazioni")
    database("UPDATE thread_spawn_edges SET parent_thread_id=? WHERE child_thread_id=?", (probe.G, probe.C))
    database("INSERT INTO thread_spawn_edges VALUES(?,?,'closed')", (probe.GRANDCHILD, probe.G))
    refuse(probe.G, "relazioni")
    refresh()
    assert client.call("GET", f"{query}?search={probe.G}")["total"] == 0
    database("DELETE FROM thread_spawn_edges WHERE child_thread_id=?", (probe.G,))
    refresh()

    database("UPDATE thread_spawn_edges SET status='open' WHERE child_thread_id=?", (probe.C,))
    refuse(probe.G, "attiva")
    database("UPDATE thread_spawn_edges SET status='closed' WHERE child_thread_id=?", (probe.C,))

    database("UPDATE threads SET archived=0 WHERE id=?", (probe.C,))
    refuse(probe.G, "attiva")
    database("UPDATE threads SET archived=1 WHERE id=?", (probe.C,))

    locks = root / "thread-writer-locks"
    locks.mkdir(exist_ok=True)
    with (locks / f"{probe.C}.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        refuse(probe.G, "in uso")

    review = preview(probe.G, 5)
    database("INSERT INTO thread_spawn_edges VALUES(?,?,'closed')", (probe.C, ref))
    confirm(review, "changed")
    assert all(path.is_file() for path in rollouts.values())
    database("DELETE FROM thread_spawn_edges WHERE child_thread_id=?", (ref,))
    refresh()

    database("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(63,'fixture',1,x'',0)")
    refuse(probe.G, "formato")
    database("DELETE FROM _sqlx_migrations WHERE version=63")

    # Each successful operation preserves the full canonical rows and exact bytes
    # of unrelated fixtures. This includes hidden workers and the active main.
    for ident, descendants in [(probe.R, set()), (probe.ONE, {probe.ONLY_CHILD}),
            (probe.G, {probe.C, probe.GRANDCHILD, probe.SIBLING, probe.ORPHAN, probe.session_id(99)})]:
        affected = {ident} | descendants
        untouched = {key: path.read_bytes() for key, path in rollouts.items()
                     if key not in affected and path.exists()}
        before = [row for row in database("SELECT * FROM threads ORDER BY id") if row[0] not in affected]
        review = preview(ident, len(descendants))
        confirm(review, "verified", concurrent=(ident == probe.G))
        assert all(not path.exists() for key, path in rollouts.items() if key in affected)
        assert all(rollouts[key].read_bytes() == original for key, original in untouched.items())
        assert database("SELECT * FROM threads ORDER BY id") == before
        assert all(parent not in affected and child not in affected
                   for parent, child in database("SELECT parent_thread_id,child_thread_id FROM thread_spawn_edges"))
        visible.remove(ident)
        remaining = client.call("GET", query)
        assert {row["native_id"] for row in remaining["rows"]} == visible

    # Invalid config is already present at review. The native process then exits
    # nonzero and the read-only check independently reports the remaining evidence.
    config = root / "config.toml"
    config.write_text("invalid = [")
    try:
        outcome = confirm(preview(probe.M), "native_uncertain")
        assert "Righe di sessione ancora presenti: 1" in outcome["message"], outcome
    finally:
        config.unlink()
    refuse(probe.M, "già un esito")

    review = preview(probe.REPLACEMENT)
    displaced = root.with_name("provider-before-replacement")
    root.rename(displaced)
    root.mkdir()
    try:
        confirm(review, "changed")
        assert not any(root.iterdir())
    finally:
        root.rmdir()
        displaced.rename(root)
    refresh()

    # Replace the executable at the same name with byte-identical contents. The
    # operation binds file identity through confirmation, regardless of its digest.
    review = preview(probe.REPLACEMENT)
    prior = binary.with_suffix(".prior")
    binary.rename(prior)
    shutil.copy2(prior, binary)
    assert probe.digest(prior) == probe.digest(binary)
    confirm(review, "changed")
    assert rollouts[probe.REPLACEMENT].exists()


PROJECTS = "/api/codex/projects"


def claude_projects(client, root):
    same = root / "projects" / "namespace-b"
    same.mkdir()
    original = root / "projects" / "project" / f"{CLAUDE_ID}.jsonl"
    duplicate = same / original.name
    duplicate.write_bytes(original.read_bytes())
    unknown = same / "unknown-sidecar"
    unknown.write_bytes(b"outside transcript contract")
    empty = root / "projects" / "observed-empty"
    empty.mkdir()
    client.call("POST", "/api/claude/refresh")
    query = "/api/claude/projects"
    first = client.call("GET", f"{query}?search=Atlas&limit=1")
    second = client.call("GET", f"{query}?search=Atlas&limit=1&offset=1")
    assert first["total"] == second["total"] == 2
    projects = first["rows"] + second["rows"]
    assert {project["namespace"] for project in projects} == {"project", "namespace-b"}
    assert len({project["key"] for project in projects}) == 2
    for project in projects:
        assert project["workspace"] == "/fixture/Atlas"
        assert project["persistent_instance"] and not project["removal_available"]
        scoped = f"{query}/{project['key']}"
        assert client.call("GET", scoped) == project
        filtered = client.call("GET", scoped + "/sessions?search=CAFE%CC%81")
        assert filtered["total"] == 1 and filtered["rows"][0]["native_id"] == CLAUDE_ID
        client.call("GET", f"{PROJECTS}/{project['key']}", code=404)
        message = client.call("POST", scoped + "/removal-preview", code=409)["error"]
        assert "sidecar" in message
    assert unknown.read_bytes() == b"outside transcript contract" and same.is_dir()
    observed = client.call("GET", query + "?search=observed-empty")["rows"]
    assert len(observed) == 1 and observed[0]["main_conversations"] == 0
    assert duplicate.read_bytes() == original.read_bytes()
    return projects


def project_path(client, workspace):
    page = client.call("GET", PROJECTS)
    return next(f"{PROJECTS}/{row['key']}" for row in page["rows"]
                if row["workspace"] == str(workspace))


def confirm_project(client, review, expected, concurrent=False):
    path = f"/api/operations/{review['id']}/confirm"
    assert review["confirmation"] == "ELIMINA PROGETTO"
    phrase = {"confirmation": review["confirmation"]}
    client.call("POST", path, {"confirmation": "RIMUOVI"}, code=409)
    client.call("POST", path, {**phrase, "ids": [probe.F]}, code=422)
    if concurrent:
        with ThreadPoolExecutor(max_workers=2) as pool:
            calls = [pool.submit(client.call, "POST", path, phrase, code=(202, 409)) for _ in range(2)]
            assert sorted(call.result() for call in calls) == [202, 409]
    else:
        client.call("POST", path, phrase, code=202)
    result = client.operation(review["id"])
    assert result["status"] == expected, result
    client.call("POST", path, phrase, code=409)
    return result["project"]


def codex_projects(agentmender, base, claude, executable):
    import fcntl

    extra = probe.session_id(102)
    workspaces = {
        probe.R: "other/Target", probe.G: "workspace/Target", probe.ONE: "workspace/Target",
        probe.C: "different-subagent-cwd", probe.F: "active", probe.PROTECTED_PARENT: "protected",
        probe.M: "partial", probe.REPLACEMENT: "partial", extra: "partial",
    }
    with probe.native_fixture(base, executable, workspaces) as (root, rollouts):
        cwd = root.parent / "cwd"
        target = cwd / workspaces[probe.G]
        target.mkdir(parents=True)
        (target / ".git").mkdir()
        source = target / "main.rs"
        source.write_bytes(b"fn main() {}\n")
        git_config = target / ".git" / "config"
        git_config.write_bytes(b"[core]\n")
        live = base / "private-canary"
        new_id = probe.session_id(101)

        def database(sql, params=()):
            with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
                return db.execute(sql, params).fetchall()

        with ui(agentmender, base, root, claude, executable, max_entries=3) as (client, ticket):
            authorize(client, ticket)
            path = project_path(client, target)
            observed = client.call("GET", path)
            assert observed["main_conversations"] == 1 and observed["incomplete"]
            review = client.call("POST", path + "/removal-preview")
            assert review["main_conversations"] == 2 and review["complete"]

        with ui(agentmender, base, root, claude, executable) as (client, ticket):
            authorize(client, ticket)
            path = project_path(client, target)
            singleton = project_path(client, cwd / workspaces[probe.R])
            assert path != singleton and not (cwd / workspaces[probe.R]).exists()
            assert client.call("GET", singleton)["name"] == client.call("GET", path)["name"] == "Target"
            assert client.call("GET", path + "/sessions?limit=1")["total"] == 2
            assert client.call("GET", path + f"/sessions?search={probe.C}")["total"] == 0
            client.call("POST", f"{PROJECTS}/{probe.G}/removal-preview", code=404)
            for key, reason in [(probe.F, "archiviate"), (probe.PROTECTED_PARENT, "protetta")]:
                blocked = project_path(client, cwd / workspaces[key])
                result = client.call("POST", blocked + "/removal-preview", code=409)
                assert reason in result["error"], result

            database("UPDATE threads SET archived=0 WHERE id=?", (probe.C,))
            assert "attiva" in client.call("POST", path + "/removal-preview", code=409)["error"]
            database("UPDATE threads SET archived=1 WHERE id=?", (probe.C,))
            locks = root / "thread-writer-locks"
            locks.mkdir(exist_ok=True)
            with (locks / f"{probe.ONLY_CHILD}.lock").open("w") as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                assert "in uso" in client.call("POST", path + "/removal-preview", code=409)["error"]
            fork = probe.seed(root.parent, new_id, archived=True,
                              cwd=str(cwd / "other"), forked_from_id=probe.C)
            assert "fork esterno" in client.call("POST", path + "/removal-preview", code=409)["error"]
            fork.unlink()
            orphan = probe.seed(root.parent, new_id, archived=True, cwd=str(target))
            assert "incompleta" in client.call("POST", path + "/removal-preview", code=409)["error"]
            orphan.unlink()

            review = client.call("POST", path + "/removal-preview")
            assert review["complete"] and review["status"] == "ready" and not review["persistent_items"]
            assert (review["main_conversations"], review["descendants"], review["total_sessions"]) == (2, 6, 8), review

            # A newly imported main in the same group invalidates the entire receipt.
            new_rollout = probe.seed(root.parent, new_id, archived=True, cwd=str(target))
            with probe.FixtureServer(root.parent, executable, live) as server:
                assert "result" in server.call("thread/read", {"threadId": new_id, "includeTurns": False})
            assert confirm_project(client, review, "changed")["removed"] == []
            database("DELETE FROM threads WHERE id=?", (new_id,))
            new_rollout.unlink()

            review = client.call("POST", path + "/removal-preview")
            database("UPDATE threads SET cwd=? WHERE id=?", (str(cwd / "different"), probe.ONE))
            assert not confirm_project(client, review, "refused")["removed"]
            database("UPDATE threads SET cwd=? WHERE id=?", (str(target), probe.ONE))

            before = {ident: path.read_bytes() for ident, path in rollouts.items()}
            deleted = {probe.G, probe.ONE, probe.C, probe.GRANDCHILD, probe.SIBLING, probe.ORPHAN, probe.ONLY_CHILD}
            unrelated = [row for row in database("SELECT * FROM threads ORDER BY id") if row[0] not in deleted]
            review = client.call("POST", path + "/removal-preview")
            progress = confirm_project(client, review, "verified", concurrent=True)
            assert progress["removed"] == review["subtrees"] and not progress["remaining"] and progress["uncertain"] is None
            assert database("SELECT * FROM threads ORDER BY id") == unrelated
            for ident, rollout in rollouts.items():
                if ident in deleted:
                    assert not rollout.exists()
                else:
                    assert rollout.read_bytes() == before[ident]
            assert source.read_bytes() == b"fn main() {}\n" and git_config.read_bytes() == b"[core]\n"
            client.call("GET", path, code=404)
            review = client.call("POST", singleton + "/removal-preview")
            assert review["main_conversations"] == review["total_sessions"] == 1
            confirm_project(client, review, "verified")
            client.call("GET", singleton, code=404)

        # Exercise a second dispatch failure with a fixture-only executable wrapper.
        # The first command is the real verified release. No production hook is used.
        wrapper = base / "fixture-fail-second"
        wrapper.write_text(f'''#!/bin/sh
for arg do
  [ "$arg" = "{probe.REPLACEMENT}" ] && exit 29
done
exec {shlex.quote(str(executable))} "$@"
''')
        wrapper.chmod(0o700)
        with ui(agentmender, base, root, claude, wrapper) as (client, ticket):
            authorize(client, ticket)
            path = project_path(client, cwd / "partial")
            review = client.call("POST", path + "/removal-preview")
            progress = confirm_project(client, review, "partial")
            assert [tree["main_id"] for tree in progress["removed"]] == [probe.M], progress
            assert progress["uncertain"]["main_id"] == probe.REPLACEMENT
            assert [tree["main_id"] for tree in progress["remaining"]] == [extra]
            assert rollouts[extra].is_file()
            assert rollouts[probe.REPLACEMENT].is_file() and not rollouts[probe.M].exists()
            client.call("POST", path + "/removal-preview", code=409)

        with ui(agentmender, base, root, claude, executable) as (client, ticket):
            authorize(client, ticket)
            client.call("GET", f"/api/operations/{review['id']}", code=404)
            assert client.call("GET", path)["main_conversations"] == 2
            fresh = client.call("POST", path + "/removal-preview")
            assert fresh["id"] != review["id"] and fresh["main_conversations"] == 2
            assert rollouts[probe.REPLACEMENT].is_file()


def rejects_unconfined_reader(binary, source):
    with tempfile.TemporaryDirectory(prefix="agentmender-evidence-", dir="/private/tmp") as private, \
         tempfile.TemporaryDirectory(prefix="agentmender-denial-", dir="/private/tmp") as denial:
        probe = str(Path(denial) / "must-remain-absent")
        request = {"source_root": str(source), "private_root": private, "probe_path": probe,
                   "operation": {"Inventory": {"after": "", "limit": 1}}}
        result = subprocess.run(
            [str(binary), "--internal-codex-content-capture"],
            input=json.dumps(request) + "\n", text=True, capture_output=True, timeout=5,
            env={"HOME": private, "CODEX_HOME": private, "TMPDIR": private},
        )
        assert result.returncode != 0 and not result.stdout and not Path(probe).exists()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agentmender", required=True, type=Path)
    parser.add_argument("--native-codex", type=Path, help="supported Darwin arm64 binary; only disposable writes")
    args = parser.parse_args()
    agentmender = args.agentmender.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="am-ui-") as directory:
        base = Path(directory).resolve()
        claude = base / "claude"
        claude_fixture(claude)
        if args.native_codex:
            binary = args.native_codex.resolve(strict=True)
            assert platform.system() == "Darwin" and platform.machine() == "arm64"
            if binary.parent.name == "MacOS" and binary.parents[2].suffix == ".app":
                # A signed app binary needs its original bundle metadata.
                bundle = base / binary.parents[2].name
                shutil.copytree(binary.parents[2], bundle, symlinks=True)
                executable_copy = bundle / binary.relative_to(binary.parents[2])
            else:
                executable_copy = base / "executable" / "codex"
                executable_copy.parent.mkdir()
                shutil.copy2(binary, executable_copy)
            with probe.native_fixture(base, executable_copy) as (root, rollouts):
                with ui(agentmender, base, root, claude, executable_copy) as (client, ticket):
                    authorize(client, ticket)
                    inventory_acceptance(client, claude)
                    codex_acceptance(client, root, rollouts, executable_copy)
                unknown = base / "unsupported-executable"
                unknown.write_text("#!/bin/sh\nprintf 'codex-cli 99.0.0\\n'\n")
                unknown.chmod(0o700)
                with ui(agentmender, base, root, claude, unknown) as (client, ticket):
                    authorize(client, ticket)
                    provider = client.call("GET", "/api/codex/sessions")["provider"]
                    assert not provider["removal_available"] and "versione" in provider["removal_reason"]
                banner_only = base / "missing-delete-executable"
                banner_only.write_text("#!/bin/sh\nprintf 'codex-cli 0.169.0\\n'\n")
                banner_only.chmod(0o700)
                with ui(agentmender, base, root, claude, banner_only) as (client, ticket):
                    authorize(client, ticket)
                    provider = client.call("GET", "/api/codex/sessions")["provider"]
                    assert not provider["removal_available"] and "comando" in provider["removal_reason"], provider
            project_base = base / "project-suite"
            project_base.mkdir()
            codex_projects(agentmender, project_base, claude, executable_copy)
            print("Native Codex projects: full preflight, serial verification, partial completion and fresh review after restart.")
            print(f"Native Codex UI ({probe.digest(binary)[:12]}): verified singleton and subtree removal, hidden descendants, refusal and one-shot guards, unchanged unrelated rows and rollouts.")
        else:
            root = base / "codex"
            with ui(agentmender, base, root, claude) as (client, ticket):
                authorize(client, ticket)
                first_claude_key = inventory_acceptance(client, claude)
                first_codex_key = None
                missing = client.call("GET", "/api/codex/sessions")
                assert missing["provider"]["coverage"] == "not_detected"
                assert missing["provider"]["presence"] == "not_detected"
                assert client.call("GET", "/api/claude/sessions?limit=101", code=400)
                codex_metadata_fixture(root)
                if platform.system() == "Darwin":
                    rejects_unconfined_reader(agentmender, root)
                client.call("POST", "/api/codex/refresh")
                first = client.call("GET", "/api/codex/sessions?limit=10")
                if platform.system() == "Darwin":
                    assert first["provider"]["total"] == 3000
                    atlas = project_path(client, "/fixture/Atlas")
                    options = "search=CAFE%CC%81%20Atlas&state=archived&sort=created&ascending=true&limit=2"
                    p1 = client.call("GET", f"{atlas}/sessions?{options}")
                    p2 = client.call("GET", f"{atlas}/sessions?{options}&offset=2")
                    assert p1["total"] == p2["total"] == 1500
                    assert [row["created_at"] for row in p1["rows"] + p2["rows"]] == [1, 3, 5, 7]
                    second = client.call("GET", "/api/codex/sessions?limit=10&offset=10")
                    assert not {item["key"] for item in first["rows"]} & {item["key"] for item in second["rows"]}
                    selected = client.call("GET", "/api/codex/sessions?search=CAFE%CC%81%20Atlas&state=archived&since=1970-01-02&until=1970-01-02&sort=created&ascending=true&limit=2")
                    assert selected["total"] == 1500 and selected["rows"][0]["created_at"] == 1
                    assert not first["provider"]["removal_available"]
                    client.call("POST", f"/api/codex/sessions/{selected['rows'][0]['key']}/removal-preview", code=409)
                    missing_title = client.call("GET", "/api/codex/sessions?offset=2999")["rows"][0]
                    assert missing_title["title"] is None and missing_title["transcript_bytes"] is None
                    first_codex_key = missing_title["key"]
                    with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
                        db.execute("ALTER TABLE threads RENAME TO unsupported")
                    refreshed = client.call("POST", "/api/codex/refresh")
                    assert refreshed["coverage"] == "unavailable"
                else:
                    assert first["provider"]["coverage"] == "unavailable"
            other_claude = base / "another-claude"
            other_codex = base / "another-codex"
            claude_fixture(other_claude)
            codex_metadata_fixture(other_codex, 1)
            with ui(agentmender, base, other_codex, other_claude) as (client, ticket):
                authorize(client, ticket)
                assert inventory_acceptance(client, other_claude) != first_claude_key
                projects = claude_projects(client, other_claude)
                client.call("GET", f"/api/claude/sessions/{first_claude_key}", code=404)
                if first_codex_key:
                    other = client.call("GET", "/api/codex/sessions")["rows"][0]
                    assert other["native_id"] == "ab000000-1111-4111-8111-000000000000"
                    assert other["key"] != first_codex_key
                    codex = client.call("GET", PROJECTS)["rows"][0]
                    assert codex["key"] not in {project["key"] for project in projects}
                    assert codex["workspace"] == "/fixture/Atlas"
                    client.call("GET", f"/api/codex/sessions/{first_codex_key}", code=404)
            if platform.system() == "Darwin":
                with ui(agentmender, base, other_codex, other_claude, max_entries=6) as (client, ticket):
                    authorize(client, ticket)
                    truncated = client.call("GET", "/api/codex/sessions")
                    assert truncated["total"] == 0 and truncated["provider"]["total"] == 0
                project_budget = base / "project-budget"
                codex_metadata_fixture(project_budget, 12)
                with closing(sqlite3.connect(project_budget / "state_5.sqlite")) as db, db:
                    db.execute("DELETE FROM threads WHERE title='Hidden Codex worker'")
                    db.execute("UPDATE threads SET cwd=NULL WHERE id=?", (probe.session_id(0),))
                with ui(agentmender, base, project_budget, other_claude, max_entries=4) as (client, ticket):
                    authorize(client, ticket)
                    page = client.call("GET", PROJECTS)
                    rows = {row["name"]: row for row in page["rows"]}
                    assert rows["Atlas"]["main_conversations"] == 3 and rows["Atlas"]["incomplete"]
                    assert rows["Senza progetto"]["main_conversations"] == 1 and rows["Senza progetto"]["unassigned"]
                    path = f"{PROJECTS}/{rows['Senza progetto']['key']}"
                    assert client.call("GET", path + "/sessions")["total"] == 1
                    client.call("POST", path + "/removal-preview", code=409)
                assert not (other_codex / "thread-writer-locks").exists()
            print("Local UI: scoped duplicate IDs, provider inventory, unsupported removal, origin and token guards, pagination and coverage.")


if __name__ == "__main__":
    main()
