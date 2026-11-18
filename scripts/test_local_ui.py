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
def ui(binary, base, codex_root, claude_root, codex=None, max_entries=None, env_overrides=None):
    other = base / "empty-path"
    other.mkdir(exist_ok=True)
    home = base / "home"
    home.mkdir(exist_ok=True)
    environment = {"HOME": str(home), "USERPROFILE": str(home), "PATH": str(other)}
    if os.name == "nt":
        environment["SystemRoot"] = os.environ["SystemRoot"]
    environment.update(env_overrides or {})
    command = [str(binary), "--claude-root", str(claude_root), "ui", "--no-open"]
    if codex_root is not None:
        command += ["--codex-root", str(codex_root)]
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
    candidate = probe.seed(root.parent, probe.session_id(5))
    fuzzy_collision = candidate.with_name(candidate.stem + "_" + probe.session_id(17) + ".jsonl")
    fuzzy_collision.write_bytes(candidate.read_bytes())
    try:
        refuse(probe.R, "rollout estraneo")
    finally:
        fuzzy_collision.unlink()
        candidate.unlink()
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
    shared_history = probe.seed(root.parent, ref, archived=True, history_mode="paginated",
                                history_base={"thread_id": probe.C, "end_ordinal_exclusive": 1, "end_byte_offset": 10})
    refuse(probe.G, "fork esterno")
    shared_history.unlink()
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

    database("UPDATE thread_spawn_edges SET status='unrecognized' WHERE child_thread_id=?", (probe.C,))
    refuse(probe.G, "in uso")
    database("UPDATE thread_spawn_edges SET status='open' WHERE child_thread_id=?", (probe.C,))

    database("UPDATE threads SET archived=0 WHERE id=?", (probe.C,))
    refuse(probe.G, "in uso")
    database("UPDATE threads SET archived=1 WHERE id=?", (probe.C,))

    locks = root / "thread-writer-locks"
    locks.mkdir(exist_ok=True)
    with (locks / f"{probe.C}.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        refuse(probe.G, "in uso")
    with (locks / f"{probe.F_CHILD}.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        refuse(probe.F, "in uso")

    review = preview(probe.G, 5)
    database("INSERT INTO thread_spawn_edges VALUES(?,?,'closed')", (probe.C, ref))
    confirm(review, "changed")
    assert all(path.is_file() for path in rollouts.values())
    database("DELETE FROM thread_spawn_edges WHERE child_thread_id=?", (ref,))
    refresh()

    database("INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES(63,'fixture',1,x'',0)")
    refuse(probe.G, "formato")
    database("DELETE FROM _sqlx_migrations WHERE version=63")

    # Each successful operation preserves unrelated fixtures and exports.
    exports = root / "exports"
    exports.mkdir(exist_ok=True)
    independent = exports / "separate-backup.zip"
    independent.write_bytes(b"independent export fixture")
    variants = tuple(rollouts[probe.F].parent.glob(f"*{probe.F}_*.jsonl"))
    assert len(variants) == 2
    for ident, descendants in [(probe.R, set()), (probe.ONE, {probe.ONLY_CHILD}), (probe.F, {probe.F_CHILD}),
            (probe.G, {probe.C, probe.GRANDCHILD, probe.SIBLING, probe.ORPHAN, probe.session_id(99)})]:
        affected = {ident} | descendants
        untouched = {key: path.read_bytes() for key, path in rollouts.items()
                     if key not in affected and path.exists()}
        before = [row for row in database("SELECT * FROM threads ORDER BY id") if row[0] not in affected]
        review = preview(ident, len(descendants))
        confirm(review, "verified", concurrent=(ident == probe.G))
        assert all(not path.exists() for key, path in rollouts.items() if key in affected)
        if ident == probe.F:
            assert not any(path.exists() for path in variants)
        assert independent.read_bytes() == b"independent export fixture"
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
RECENTS = "/api/codex/recents"


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
    assert review["confirmation"] == "RIMUOVI SESSIONI DEL PROGETTO"
    phrase = {"confirmation": review["confirmation"]}
    client.call("POST", path, {"confirmation": "RIMUOVI"}, code=409)
    client.call("POST", path, {"confirmation": "ELIMINA PROGETTO"}, code=409)
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
    recent = probe.session_id(103)
    workspaces = {
        probe.R: "other/Target", probe.G: "workspace/Target", probe.ONE: "workspace/Target",
        probe.C: "different-subagent-cwd", probe.F: "active", probe.PROTECTED_PARENT: "protected",
        probe.M: "partial", probe.REPLACEMENT: "partial", extra: "partial", recent: "workspace/Target",
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

        with probe.FixtureServer(root.parent, executable, live, experimental=False) as server:
            rejected = server.call("project/list", {"limit": 1})
            assert "experimental" in rejected["error"]["message"].lower(), rejected
        with probe.FixtureServer(root.parent, executable, live) as server:
            imported = {}
            for index, (folder, members) in enumerate([
                (workspaces[probe.G], (probe.G, probe.ONE)),
                (workspaces[probe.R], (probe.R,)),
                ("active", (probe.F,)),
                ("protected", (probe.PROTECTED_PARENT,)),
                ("partial", (probe.M, probe.REPLACEMENT, extra)),
                ("temporary", (recent,)),
            ]):
                result = server.call("project/import", {
                    "name": Path(folder).name, "roots": [{"path": str(cwd / folder)}],
                    "threads": list(members), "idempotencyKey": f"fixture-project-{index}",
                })
                assert "result" in result, result
                imported[folder] = result["result"]["project"]["id"]
            listed = server.call("project/list", {"limit": 20})["result"]["data"]
            assert {project["id"] for project in listed} == set(imported.values())
            target_id = imported[workspaces[probe.G]]
            singleton_id = imported[workspaces[probe.R]]
            assert server.call("project/read", {"projectId": target_id})["result"]["project"]["name"] == "Target"
            assert "result" in server.call("project/delete", {"projectId": imported["temporary"]})
            assert database("SELECT project_id FROM threads WHERE id=?", (recent,)) == [(None,)]
            assert rollouts[recent].is_file()

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
            observed_recent = client.call("GET", RECENTS + f"?search={recent}")["rows"][0]
            assert observed_recent["project"] is None and observed_recent["workspace"] == str(target)
            client.call("POST", f"{PROJECTS}/{probe.G}/removal-preview", code=404)
            active = project_path(client, cwd / workspaces[probe.F])
            assert client.call("POST", active + "/removal-preview")["descendants"] == 1
            blocked = project_path(client, cwd / workspaces[probe.PROTECTED_PARENT])
            assert "protetta" in client.call("POST", blocked + "/removal-preview", code=409)["error"]

            database("UPDATE threads SET archived=0 WHERE id=?", (probe.C,))
            assert "percorso di rollout" in client.call("POST", path + "/removal-preview", code=409)["error"]
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
            assert review["complete"] and review["status"] == "ready" and review["scope"] == "sessions"
            assert (review["main_conversations"], review["descendants"], review["total_sessions"]) == (2, 6, 8), review

            # A newly imported main in the same group invalidates the entire receipt.
            new_rollout = probe.seed(root.parent, new_id, archived=True, cwd=str(target))
            with probe.FixtureServer(root.parent, executable, live) as server:
                assert "result" in server.call("thread/read", {"threadId": new_id, "includeTurns": False})
            database("UPDATE threads SET project_id=? WHERE id=?", (target_id, new_id))
            assert confirm_project(client, review, "changed")["removed"] == []
            database("DELETE FROM threads WHERE id=?", (new_id,))
            new_rollout.unlink()

            review = client.call("POST", path + "/removal-preview")
            database("UPDATE threads SET project_id=? WHERE id=?", (singleton_id, probe.ONE))
            assert not confirm_project(client, review, "changed")["removed"]
            database("UPDATE threads SET project_id=? WHERE id=?", (target_id, probe.ONE))

            database("UPDATE threads SET project_id=? WHERE id=?", (singleton_id, probe.C))
            client.call("POST", path + "/removal-preview", code=409)
            database("UPDATE threads SET project_id=NULL WHERE id=?", (probe.C,))

            review = client.call("POST", path + "/removal-preview")
            desktop_state = root / ".codex-global-state.json"
            desktop_state.write_text(json.dumps({
                "local-projects": {
                    target_id: {"id": target_id, "name": "Target", "rootPaths": [str(target)]},
                },
            }))
            original_desktop = desktop_state.read_bytes()
            assert not confirm_project(client, review, "refused")["removed"]
            assert desktop_state.read_bytes() == original_desktop
            desktop_state.unlink()

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
            assert client.call("GET", path)["main_conversations"] == 0
            assert database("SELECT id FROM projects WHERE id=?", (target_id,)) == [(target_id,)]
            assert client.call("GET", RECENTS + f"?search={recent}")["total"] == 1
            review = client.call("POST", singleton + "/removal-preview")
            assert review["main_conversations"] == review["total_sessions"] == 1
            confirm_project(client, review, "verified")
            assert client.call("GET", singleton)["main_conversations"] == 0
            recent_path = f"/api/codex/sessions/{observed_recent['key']}"
            individual = client.call("POST", recent_path + "/removal-preview")
            client.call("POST", f"/api/operations/{individual['id']}/confirm", {"confirmation": "RIMUOVI"}, code=202)
            assert client.operation(individual["id"])["status"] == "verified"

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


def codex_membership_acceptance(agentmender, base, claude):
    root = base / "codex-projects"
    codex_metadata_fixture(root, 18)
    atlas, empty, dangling, legacy_atlas, legacy_empty = map(probe.session_id, range(1000, 1005))
    sid = probe.session_id

    def database(sql, params=()):
        with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
            db.execute(sql, params)

    database("DELETE FROM threads WHERE title='Hidden Codex worker'")
    database("ALTER TABLE threads ADD COLUMN project_id TEXT")
    database("CREATE TABLE projects(id TEXT PRIMARY KEY, name TEXT)")
    database("CREATE TABLE project_roots(project_id TEXT, position INTEGER, path TEXT)")
    database("INSERT INTO projects VALUES(?,'Atlas'),(?,'Atlas')", (atlas, empty))
    database("INSERT INTO project_roots VALUES(?,0,'/fixture/Atlas'),(?,1,'/fixture/Extra'),(?,0,'/another/Atlas')",
             (atlas, atlas, empty))
    database("UPDATE threads SET cwd='/fixture/workspace-' || created_at WHERE created_at>=9")
    database("UPDATE threads SET project_id=? WHERE id IN (?,?)", (atlas, sid(0), sid(10)))
    database("UPDATE threads SET project_id=? WHERE id=?", (dangling, sid(3)))

    with ui(agentmender, base, root, claude) as (client, ticket):
        authorize(client, ticket)
        page = client.call("GET", PROJECTS)
        assert page["total"] == 2 and page["provider"]["projects_incomplete"]
        assert {row["name"] for row in page["rows"]} == {"Atlas"}
        assert page["provider"]["project_conversations"] == 2 and page["provider"]["recent_conversations"] == 16
        path = project_path(client, "/fixture/Atlas")
        empty_path = project_path(client, "/another/Atlas")
        assert path != empty_path and client.call("GET", empty_path)["main_conversations"] == 0
        assert client.call("GET", path)["roots"] == ["/fixture/Atlas", "/fixture/Extra"]
        assert client.call("GET", PROJECTS + "?search=Extra")["total"] == 1
        assert client.call("GET", path + "/sessions?search=Extra")["total"] == 2
        same_workspace = client.call("GET", RECENTS + f"?search={sid(1)}")["rows"][0]
        assert same_workspace["project"] is None and same_workspace["workspace"] == "/fixture/Atlas"
        assert client.call("GET", RECENTS + f"?search={sid(3)}")["total"] == 1
        old_scope = dict(provider="codex", state_root=str(root),
                         locator={"kind": "workspace", "value": "/fixture/Atlas"})
        old_key = hashlib.sha256(json.dumps(old_scope, separators=(",", ":")).encode()).hexdigest()
        client.call("POST", f"{PROJECTS}/{old_key}/removal-preview", code=404)
        database("UPDATE threads SET project_id=? WHERE id=?", (empty, sid(10)))
        client.call("POST", "/api/codex/refresh")
        assert client.call("GET", empty_path + "/sessions")["rows"][0]["workspace"] == "/fixture/workspace-10"

        state = root / ".codex-global-state.json"
        state.write_text(json.dumps({
            "local-projects": {
                legacy_atlas: {"id": legacy_atlas, "name": "Atlas", "rootPaths": ["/fixture/Atlas", "/fixture/Extra"]},
                legacy_empty: {"id": legacy_empty, "name": "Atlas", "rootPaths": ["/another/Atlas"]},
            },
            "app-server-project-id-by-legacy-project-id-by-host": {
                f"local:{root}": {legacy_atlas: atlas, legacy_empty: empty},
            },
            "thread-project-assignments": {
                sid(2): {"projectKind": "local", "projectId": legacy_atlas},
                sid(4): {"projectKind": "local", "projectId": legacy_atlas},
                sid(5): {"projectKind": "local", "projectId": legacy_atlas},
                sid(10): {"projectKind": "local", "projectId": legacy_atlas},
            },
            "thread-project-membership-host-ids": {sid(2): "local", sid(5): "another-host"},
            "projectless-thread-ids": [sid(4)],
        }))
        original = state.read_bytes()
        client.call("POST", "/api/codex/refresh")
        assert client.call("GET", PROJECTS)["total"] == 2
        assert client.call("GET", path)["registration"] == "native_and_desktop"
        assert {row["native_id"] for row in client.call("GET", path + "/sessions")["rows"]} == {sid(0), sid(2)}
        assert client.call("GET", empty_path)["main_conversations"] == 0
        for number in (4, 5, 10):
            assert client.call("GET", RECENTS + f"?search={sid(number)}")["total"] == 1
        client.call("POST", path + "/removal-preview", code=409)
        assert state.read_bytes() == original

        database("DELETE FROM projects WHERE id=?", (empty,))
        client.call("POST", "/api/codex/refresh")
        assert client.call("GET", PROJECTS)["total"] == 1

        desktop = json.loads(original)
        offline = sid(1005)
        desktop["local-projects"][offline] = {"id": offline, "name": "Legacy", "rootPaths": ["/legacy"]}
        desktop["thread-project-assignments"][sid(6)] = {"projectKind": "local", "projectId": offline}
        state.write_text(json.dumps(desktop))
        client.call("POST", "/api/codex/refresh")
        legacy = project_path(client, "/legacy")
        assert client.call("GET", legacy)["registration"] == "desktop"
        assert client.call("GET", legacy + "/sessions")["rows"][0]["native_id"] == sid(6)
        client.call("POST", legacy + "/removal-preview", code=409)

    with ui(agentmender, base, root, claude, max_entries=4) as (client, ticket):
        authorize(client, ticket)
        page = client.call("GET", PROJECTS)
        assert page["provider"]["inventory_incomplete"] and page["rows"][0]["incomplete"]
        assert page["provider"]["total"] == page["provider"]["project_conversations"] + page["provider"]["recent_conversations"]


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


def storage_acceptance(binary, base, claude):
    base.mkdir()
    archive = base / "árchivio"
    archive.mkdir()
    alternate = base / "external-sqlite"
    alternate.mkdir()
    # A catalog signature is enough for discovery, not for a worker or writer.
    (alternate / "state_5.sqlite").write_bytes(b"SQLite format 3\0")
    worktree = base / "linked-work"
    worktree.mkdir()
    (archive / "config.toml").write_text(
        "sqlite_home = " + json.dumps(str(alternate), ensure_ascii=False) +
        "\n[desktop]\ngit-worktree-root = " + json.dumps(str(worktree), ensure_ascii=False), encoding="utf-8")
    exports = archive / "exports"
    exports.mkdir()
    default_archive = base / "home" / ".codex"
    default_archive.mkdir(parents=True)
    (default_archive / "state_5.sqlite").write_bytes(b"SQLite format 3\0")
    user_output = base / "home" / "Documents" / "Codex"
    user_output.mkdir(parents=True)
    workspace_file = user_output / "keep.txt"
    workspace_file.write_bytes(b"user workspace")
    other = base / "fake-xdg"
    (other / "codex" / "exports").mkdir(parents=True)
    environment = {"CODEX_HOME": str(archive), "XDG_DATA_HOME": str(other)}
    with ui(binary, base, None, claude, env_overrides=environment) as (client, ticket):
        client.call("GET", "/api/storage/codex", code=401)
        authorize(client, ticket)
        roots = client.call("GET", "/api/storage/codex")
        assert Path(roots["selected_root"]) == archive
        kinds = roots["storage"]
        assert any(row["kind"] == "state_database" and Path(row["path"]).parent == alternate
                   for row in kinds), kinds
        assert any(row["kind"] == "worktrees" and Path(row["path"]) == worktree for row in kinds)
        assert any(row["kind"] == "home" and Path(row["path"]) == default_archive for row in kinds)
        assert not any(other in Path(row["path"]).parents for row in kinds)
        assert any(row["kind"] == "user_output" and not row["removal_available"] for row in kinds)
        empty = client.call("GET", "/api/storage/exports")
        assert empty["total"] == 0 and empty["complete"]
        backup = exports / "Backup Café"
        backup.mkdir()
        (backup / "file.zip").write_bytes(b"data")
        (backup / "manifest.json").write_text('{"type":"backup","producer":"Fixture"}')
        (exports / "unknown").mkdir()
        (exports / "unknown" / "manifest.json").write_bytes(b"not JSON")
        for index in range(45):
            (exports / f"data-{index:02}.zip").write_bytes(bytes(index))
        huge = exports / "sparse-fixture.zip"
        with huge.open("wb") as stream:
            stream.truncate(1_000_000)
        if os.name != "nt":
            os.link(workspace_file, exports / "hardlink-fixture")
            (exports / "outside").symlink_to(user_output, target_is_directory=True)
        else:
            # Directory junctions do not require symlink privileges.
            junction = exports / "outside"
            subprocess.run(["cmd.exe", "/d", "/c", "mklink", "/J", str(junction), str(user_output)],
                           check=True, capture_output=True)
        client.call("POST", "/api/storage/exports/refresh", code=403, Origin=None)
        client.call("POST", "/api/storage/exports/refresh", code=204)
        first = client.call("GET", "/api/storage/exports?limit=4")
        second = client.call("GET", "/api/storage/exports?limit=4&offset=4")
        assert first["rows"][0]["name"] == huge.name and first["total"] > 45
        assert not {row["key"] for row in first["rows"]} & {row["key"] for row in second["rows"]}
        assert not first["removal_available"]
        entry = client.call("GET", "/api/storage/exports?search=Caf%C3%A9")["rows"][0]
        assert entry["protection"] == "independent_backup" and entry["manifest"] == "manifest.json"
        links = client.call("GET", "/api/storage/exports?search=outside")["rows"][0]
        assert links["size"]["files"] == 0 and links["protection"] == "shared"
        if os.name != "nt":
            linked = client.call("GET", "/api/storage/exports?search=hardlink")["rows"][0]
            assert linked["shared_entries"] == 1 and not linked["removal_available"]
        preview = client.call("POST", "/api/storage/exports/preview", {"keys": [entry["key"]]})
        assert preview["removable"] == 0 and preview["protected"] == 1 and preview["receipt"] is None
        client.call("POST", "/api/storage/exports/preview", {"paths": [str(backup)]}, code=422)
        client.call("POST", "/api/storage/exports/operations/forged/confirm", code=409)
        client.call("POST", "/api/storage/exports/operations/forged/confirm", code=409)
        old = exports / "different"
        backup.rename(old)
        backup.mkdir()
        (backup / "file.zip").write_bytes(b"replacement")
        client.call("POST", "/api/storage/exports/preview", {"keys": [entry["key"]]}, code=409)
        assert workspace_file.read_bytes() == b"user workspace"
        assert (backup / "file.zip").read_bytes() == b"replacement"


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
            storage_acceptance(agentmender, base / "storage-suite", claude)
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
                    assert client.call("GET", PROJECTS)["total"] == 0
                    options = "search=CAFE%CC%81%20Atlas&state=archived&sort=created&ascending=true&limit=2"
                    p1 = client.call("GET", f"{RECENTS}?{options}")
                    p2 = client.call("GET", f"{RECENTS}?{options}&offset=2")
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
                claude_projects(client, other_claude)
                client.call("GET", f"/api/claude/sessions/{first_claude_key}", code=404)
                if first_codex_key:
                    other = client.call("GET", "/api/codex/sessions")["rows"][0]
                    assert other["native_id"] == "ab000000-1111-4111-8111-000000000000"
                    assert other["key"] != first_codex_key
                    assert client.call("GET", PROJECTS)["total"] == 0
                    assert client.call("GET", RECENTS)["rows"][0]["workspace"] == "/fixture/Atlas"
                    client.call("GET", f"/api/codex/sessions/{first_codex_key}", code=404)
            if platform.system() == "Darwin":
                with ui(agentmender, base, other_codex, other_claude, max_entries=6) as (client, ticket):
                    authorize(client, ticket)
                    truncated = client.call("GET", "/api/codex/sessions")
                    assert truncated["total"] == 0 and truncated["provider"]["total"] == 0
                codex_membership_acceptance(agentmender, base, other_claude)
                assert not (other_codex / "thread-writer-locks").exists()
            print("Local UI: scoped duplicate IDs, provider inventory, unsupported removal, origin and token guards, pagination and coverage.")


if __name__ == "__main__":
    main()
