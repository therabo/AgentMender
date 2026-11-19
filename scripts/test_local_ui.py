#!/usr/bin/env python3
"""HTTP acceptance on disposable archives, with optional native Codex cascade tests."""
import argparse
from contextlib import closing, contextmanager
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
from urllib.parse import urlsplit

sys.dont_write_bytecode = True
import native_ui_fixture as probe

CLAUDE_ID = "eb000000-1111-4111-8111-000000000001"
CLAUDE_DESKTOP_ID = "eb000000-1111-4111-8111-000000000002"
CLAUDE_SIDECHAIN_ID = "eb000000-1111-4111-8111-000000000003"
CLAUDE_AGENT_ID = "a123badc"
PROJECTS = "/api/codex/projects"
RECENTS = "/api/codex/recents"


def scoped_key(provider, root, ident, namespace=None):
    identity = dict(provider=provider, state_root=str(root), namespace=namespace, native_id=ident)
    return hashlib.sha256(json.dumps(identity, separators=(",", ":")).encode()).hexdigest()


def claude_fixture(root):
    bucket = root / "projects" / "project"
    bucket.mkdir(parents=True)
    records = [
        {"type": "user", "sessionId": CLAUDE_ID, "entrypoint": "cli",
         "cwd": "/fixture/Atlas", "timestamp": "2026-11-10T09:00:00Z",
         "message": {"content": "First visible Claude request."}},
        {"type": "user", "sessionId": CLAUDE_ID,
         "message": {"content": [{"type": "tool_result", "content": "TOOL CANARY"}]}},
        {"type": "assistant", "sessionId": CLAUDE_ID,
         "message": {"content": "Authorization: bearer SECRET CANARY"}},
        *[{"type": "assistant", "sessionId": CLAUDE_ID,
           "message": {"content": [{"type": "text", "text": f"Claude answer {index}"}]}}
          for index in range(24)],
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


@contextmanager
def ui(binary, base, codex_root, claude_root, codex=None, max_entries=None, env_overrides=None, extra_args=()):
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
        # Use the executable and archive configured for this fixture.
        command += ["--codex-executable", str(codex), "--protect-codex-session", probe.S]
    else:
        command += ["--codex-executable", str(base / "missing-codex")]
    command.extend(extra_args)
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
    assert row["state"] == "unknown" and row["created_at"] == row["last_active_at"]
    assert row["created_at"] is not None and row["claude"]["modified_at"] is not None
    assert row["namespace"] == "project"
    assert client.call("GET", "/api/claude/sessions?search=project&min_bytes=1000")["total"] == 1
    assert client.call("GET", "/api/claude/sessions?min_bytes=10&max_bytes=1", code=400)
    assert client.call("GET", "/api/claude/sessions?since=2026-11-11")["total"] == 0
    detail = client.call("GET", f"/api/claude/sessions/{row['key']}")
    assert detail["conversation_preview"] and detail["session"]["removal_reason"]
    path = f"/api/claude/sessions/{row['key']}"
    page = client.call("GET", path + "/conversation")
    assert len(page["messages"]) == 20 and not page["complete"]
    assert page["messages"][0] == {"role": "user", "text": "First visible Claude request."}
    assert "CANARY" not in repr(page) and len(page["next"]) == 64
    second = client.call("GET", path + "/conversation?cursor=" + page["next"])
    assert len(second["messages"]) == 5 and second["next"] is None and second["complete"]
    storage = client.call("GET", path + "/storage")
    assert storage["subagents"] == 1
    client.call("GET", "/api/claude/sessions/../../config.toml", code=404)
    client.call("GET", "/api/claude/sessions?root=%2Ftmp", code=400)
    desktop = client.call("GET", f"/api/claude/sessions?search={CLAUDE_DESKTOP_ID}")
    assert desktop["total"] == 1 and desktop["rows"][0]["workspace"] is None
    foreign = f"/api/claude/sessions/{desktop['rows'][0]['key']}/conversation"
    client.call("GET", foreign + "?cursor=" + page["next"], code=400)
    sources = list(claude_root.glob("projects/*/*.jsonl"))
    assert len(sources) == 3 and "First visible" in next(path for path in sources if path.stem == CLAUDE_ID).read_text()
    info = client.call("GET", "/api/claude/sessions?limit=1")
    assert info["provider"]["total"] == 2 and info["total"] == 2
    assert client.call("GET", "/api/claude/sessions?search=Hidden")["total"] == 0
    agent_key = scoped_key("claude", claude_root, f"{CLAUDE_ID}/agent:{CLAUDE_AGENT_ID}", "project")
    client.call("GET", f"/api/claude/sessions/{agent_key}", code=404)
    return row["key"]



def claude_multiple_roots(binary, base, primary):
    additional = base / "additional-claude"
    claude_fixture(additional)
    with ui(binary, base, None, primary, extra_args=("--claude-additional-root", str(additional))) as (client, ticket):
        authorize(client, ticket)
        result = client.call("GET", f"/api/claude/sessions?search={CLAUDE_ID}")
        assert result["total"] == 2 and len({row["key"] for row in result["rows"]}) == 2
        assert all(row["claude"]["same_uuid_elsewhere"] == 1 for row in result["rows"])
        roots = result["provider"]["claude_storage"]["roots"]
        assert {item["source"] for item in roots} == {"explicit", "additional"}
        assert {Path(item["path"]) for item in roots} == {primary, additional}
        first, second = [f"/api/claude/sessions/{row['key']}" for row in result["rows"]]
        cursor = client.call("GET", first + "/conversation")["next"]
        client.call("GET", second + "/conversation?cursor=" + cursor, code=400)
        row = result["rows"][0]
        source = Path(row["archive_root"]) / "projects/project" / f"{CLAUDE_ID}.jsonl"
        original = source.read_bytes()
        try:
            source.write_bytes(original + b"{}\n")
            client.call("GET", first + "/conversation?cursor=" + cursor, code=409)
            client.call("GET", first + "/storage", code=409)
            if platform.system() == "Darwin":
                client.call("POST", first + "/removal-preview", code=409)
            assert source.read_bytes() == original + b"{}\n"
        finally:
            source.write_bytes(original)


def claude_reconciliation(binary, base):
    root = base / "claude-reconciliation"
    extra = base / "claude-reconciliation-extra"
    desktop = base / "claude-reconciliation-desktop"
    profile = desktop / "claude-code-sessions" / CLAUDE_ID / CLAUDE_DESKTOP_ID
    profile.mkdir(parents=True)
    title = "Titolo identico Chat e Code"
    orphan_id = "eb000000-1111-4111-8111-000000000010"
    unknown_id = "eb000000-1111-4111-8111-000000000011"
    cloud_id = "eb000000-1111-4111-8111-000000000012"
    no_uuid_id = "eb000000-1111-4111-8111-000000000013"
    invalid_id = "eb000000-1111-4111-8111-000000000014"

    def write_transcript(project, ident, source):
        project.mkdir(parents=True, exist_ok=True)
        file = project / f"{ident}.jsonl"
        rows = [
            {"type": "user", "sessionId": ident, "entrypoint": source, "cwd": "/fixture/Atlas",
             "message": {"content": "Fixture message"}},
            {"type": "custom-title", "sessionId": ident, "customTitle": title},
        ]
        file.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")

    write_transcript(root / "projects/atlas", CLAUDE_DESKTOP_ID, "claude-desktop")
    write_transcript(extra / "projects/another", CLAUDE_ID, "cli")
    write_transcript(extra / "projects/another", unknown_id, "unknown-surface")
    (extra / "projects/another" / f"{invalid_id}.jsonl").write_bytes(b"{malformed}\n")
    (extra / "projects/another" / "unsupported.jsonl").write_bytes(b"{}\n")
    # A Chat cache can have a similar title or shape. Its path is outside Code.
    write_transcript(desktop / "IndexedDB", cloud_id, "claude-desktop")
    for record_id, cli_id, archived in [
        (f"local_{CLAUDE_DESKTOP_ID}", CLAUDE_DESKTOP_ID, True),
        (f"local_{orphan_id}", orphan_id, False),
        (f"local_{no_uuid_id}", None, False),
        (f"cloud_{cloud_id}", cloud_id, False),
    ]:
        (profile / f"{record_id}.json").write_text(json.dumps({
            "sessionId": record_id, "cliSessionId": cli_id,
            "isArchived": archived, "cwd": "/fixture/Atlas", "title": title,
        }), encoding="utf-8")

    roots = ("--claude-additional-root", str(extra), "--claude-desktop-root", str(desktop))
    with ui(binary, base, None, root, extra_args=roots) as (client, ticket):
        authorize(client, ticket)
        path = "/api/claude/sessions"
        page = client.call("GET", path + "?search=Titolo%20identico")
        assert page["total"] == 2
        assert {row["native_id"] for row in page["rows"]} == {CLAUDE_ID, CLAUDE_DESKTOP_ID}
        assert page["provider"]["inventory_incomplete"]
        storage = page["provider"]["claude_storage"]
        assert [s["namespaces"] for s in storage["roots"]] == [1, 1]
        diagnostic = storage["reconciliation"]
        assert diagnostic["transcripts"]["main"] == 2
        assert diagnostic["transcripts"]["desktop_main"] == diagnostic["transcripts"]["cli_main"] == 1
        assert diagnostic["transcripts"]["archived_main"] == 1
        assert diagnostic["transcripts"]["unclassified"] == 3
        assert diagnostic["transcripts"]["exclusions"] == {
            "unrecognized_locator": 1, "unknown_entrypoint": 1, "invalid_records": 1,
        }
        assert not diagnostic["projects_incomplete"]
        registry = diagnostic["desktop"]
        assert registry["registrations"] == 3 and registry["associated"] == 1
        assert registry["without_transcript"] == registry["unresolved"] == registry["without_cli_uuid"] == 1
        assert not registry["incomplete"] and registry["unreadable_records"] == 0
        assert set(diagnostic["out_of_scope"]) == {"chat", "cowork", "code_cloud", "ssh", "wsl"}
        archived = client.call("GET", path + "?state=archived")
        assert archived["total"] == 1 and archived["rows"][0]["native_id"] == CLAUDE_DESKTOP_ID
        assert archived["provider"]["claude_storage"]["reconciliation"] == diagnostic
        excluded = client.call("GET", path + f"?search={unknown_id}")
        assert excluded["total"] == 0

    blocked = base / "claude-not-a-directory"
    blocked.write_bytes(b"fixture only")
    with ui(binary, base, None, root, extra_args=(*roots, "--claude-additional-root", str(blocked))) as (client, ticket):
        authorize(client, ticket)
        storage = client.call("GET", path)["provider"]["claude_storage"]
        assert storage["roots"][-1]["projects_access"] == "unavailable"
        assert storage["reconciliation"]["projects_incomplete"]
        registry = storage["reconciliation"]["desktop"]
        assert registry["registrations"] == registry["unresolved"] == 3
        assert registry["associated"] == registry["without_transcript"] == 0



def claude_storage_acceptance(binary, base):
    root = base / "claude-storage"
    desktop = base / "claude-storage-desktop"
    temp_base = base / "claude-storage-temp"
    claude_fixture(root)
    project = root / "projects/project"
    archive = project / CLAUDE_ID
    source = project / f"{CLAUDE_ID}.jsonl"
    desktop_source = project / f"{CLAUDE_DESKTOP_ID}.jsonl"
    history = root / "history.jsonl"
    other = base / "outsider-fixture.txt"
    registry = desktop / "claude-code-sessions" / CLAUDE_ID / CLAUDE_DESKTOP_ID
    desktop_registration = registry / f"local_{CLAUDE_DESKTOP_ID}.json"
    temp = temp_base / ("claude" if os.name == "nt" else f"claude-{os.getuid()}") / "project" / CLAUDE_ID
    files = {
        archive / "tool-results/result.txt": b"result" * 19,
        archive / "mcp-tasks/running.json": b'{"state":"running"}',
        archive / "mcp-tasks/completed.json": b'{"state":"completed"}',
        root / "file-history" / CLAUDE_ID / "checkpoint": b"checkpoint payload",
        root / "image-cache" / CLAUDE_ID / "image.png": b"legacy image",
        root / "uploads" / CLAUDE_ID / "upload.bin": b"batch upload",
        temp / "images/new.png": b"temporary image",
        temp / "scratchpad/output.txt": b"user output",
        project / f"{CLAUDE_ID}.precompact.json": b"observed sidecar",
        project / f"{CLAUDE_ID}.orphaned-1700000000-A1.jsonl": b"historical candidate",
        project / f"{CLAUDE_ID}.jsonl.superseded-1700000001": b"second historical candidate",
        project / f"{CLAUDE_ID}.other": b"unknown sibling",
        root / "unclassified-private-directory" / "payload": b"must not descend",
        history: (json.dumps({"sessionId": CLAUDE_ID}) + "\r\nmalformed\r\n" + json.dumps({"sessionId": CLAUDE_DESKTOP_ID})).encode(),
        desktop_registration: json.dumps({"sessionId": f"local_{CLAUDE_DESKTOP_ID}",
            "cliSessionId": CLAUDE_DESKTOP_ID, "isArchived": True, "cwd": "/fixture/First"}).encode(),
        desktop / "Cache/sample.bin": b"shared cache",
        other: b"outside all audited roots",
    }
    for file, data in files.items():
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(data)
    files[source] = source.read_bytes()
    files[desktop_source] = desktop_source.read_bytes()
    if os.name != "nt":
        (archive / "tool-results/external-link").symlink_to(other)
        os.link(root / "image-cache" / CLAUDE_ID / "image.png", root / "image-cache" / CLAUDE_ID / "image-hardlink.png")

    storage_path = "/api/storage/claude"
    with ui(binary, base, None, root, extra_args=("--claude-desktop-root", str(desktop)),
            env_overrides={"CLAUDE_CODE_TMPDIR": str(temp_base)}) as (client, ticket):
        client.call("GET", storage_path, code=401)
        authorize(client, ticket)
        usage = client.call("GET", storage_path)
        assert usage["writer_observation"] in {
            "claude_process_observed", "no_named_process_observed", "unavailable",
        }
        assert [row["kind"] for row in usage["roots"]].count("temporary") == 1
        code = next(row for row in usage["roots"] if row["kind"] == "code")
        skipped = next(row for row in code["groups"] if row["category"] == "unclassified")
        assert skipped["size"]["files"] == 0 and not skipped["size"]["complete"]
        assert all("path" not in row for row in usage["largest_sessions"])
        assert usage["associated_transcript_bytes"] > len(files[source])
        client.call("GET", storage_path + "?root=/tmp", code=400)
        client.call("POST", storage_path + "/refresh", code=204)

        rows = client.call("GET", "/api/claude/sessions")["rows"]
        keys = {row["native_id"]: row["key"] for row in rows}
        detail_path = f"/api/claude/sessions/{keys[CLAUDE_ID]}"
        detail = client.call("GET", detail_path + "/storage")
        categories = {row["key"]: row for row in detail["categories"]}
        assert categories["subagent"]["files"] == 1
        assert categories["variant"]["files"] == 2
        assert categories["mcp_tasks"]["files"] == 2
        assert categories["images"]["files"] == 2 + int(os.name != "nt")
        assert categories["scratchpad"]["files"] == categories["checkpoint"]["files"] == 1
        assert categories["uploads"]["files"] == categories["unknown"]["files"] == 1
        assert detail["history_matches"] == 1 and detail["history_observation"] == "partial_uuid_only"
        assert not detail["diagnostic_complete"]
        if os.name != "nt":
            result = next(row for row in detail["observed"] if row["category"] == "tool_results")
            assert result["size"]["links"] == 1 and result["size"]["files"] == 1
            assert result["size"]["logical_bytes"] == len(files[archive / "tool-results/result.txt"])
            assert categories["images"]["allocated_bytes"] is None
        archived = client.call("GET", f"/api/claude/sessions/{keys[CLAUDE_DESKTOP_ID]}/storage")
        registration = next(row for row in archived["observed"] if row["category"] == "desktop_record")
        assert registration["format"] == "desktop_local_registration"
        assert registration["owner"] == CLAUDE_DESKTOP_ID
        assert all(path.read_bytes() == data for path, data in files.items())
        # A stale correlated registration invalidates the scoped preflight as well.
        desktop_registration.write_bytes(files[desktop_registration] + b"\n")
        client.call("GET", f"/api/claude/sessions/{keys[CLAUDE_DESKTOP_ID]}/storage", code=409)
        # Matching content and mtime cannot substitute for the indexed file identity.
        original_stat = source.stat()
        replacement = base / "replacement.jsonl"
        replacement.write_bytes(files[source])
        os.utime(replacement, ns=(original_stat.st_atime_ns, original_stat.st_mtime_ns))
        replacement.replace(source)
        client.call("GET", detail_path + "/storage", code=409)
        assert files[source] == source.read_bytes() and files[history] == history.read_bytes()



def claude_cleanup_acceptance(binary, base):
    root = base / "claude-delete"
    desktop = base / "claude-delete-desktop"
    bucket = root / "projects" / "cli"
    agent = bucket / CLAUDE_ID / "subagents" / f"agent-{CLAUDE_AGENT_ID}.jsonl"
    stored = bucket / f"{CLAUDE_ID}.jsonl"
    desktop_session = root / "projects" / "desktop" / f"{CLAUDE_DESKTOP_ID}.jsonl"
    blocked_id = CLAUDE_SIDECHAIN_ID
    blocked = bucket / f"{blocked_id}.jsonl"
    registration = desktop / "claude-code-sessions" / CLAUDE_ID / CLAUDE_DESKTOP_ID / f"local_{CLAUDE_DESKTOP_ID}.json"
    history = root / "history.jsonl"
    sidecar = stored.with_suffix(".precompact.json")
    env = root / "session-env" / CLAUDE_ID / "env"
    task = root / "tasks" / CLAUDE_ID / "1.json"
    debug = root / "debug" / f"{CLAUDE_ID}.txt"
    checkpoint = root / "file-history" / CLAUDE_ID / "keep"
    desktop_checkpoint = root / "file-history" / CLAUDE_DESKTOP_ID / "delete"
    nested = bucket / CLAUDE_ID
    unknown_output = nested / "user-output.txt"
    deep_output = nested.joinpath("mcp-tasks", *[f"level-{n}" for n in range(20)], "result.json")
    wide = nested / "tool-results"
    empty = nested / "other" / "empty"
    outside = base / "outside-claude-session"
    outside.mkdir()
    sentinel = outside / "keep"
    sentinel.write_bytes(b"another archive")
    link = nested / "external"
    shared = registration.with_name("scheduled-sessions.json")
    large_row = json.dumps({"sessionId": blocked_id, "display": "x" * 65536}).encode() + b"\n"
    history_prefix = (large_row * 512 + json.dumps({"sessionId": blocked_id}).encode() + b"\r\n"
                      + b"malformed, preserve my bytes\r\n")
    desktop_row = json.dumps({"session_id": CLAUDE_DESKTOP_ID}).encode()
    retained_history = history_prefix + desktop_row
    initial_history = history_prefix + json.dumps({"sessionId": CLAUDE_ID}).encode() + b"\r\n" + desktop_row
    assert len(initial_history) > 32 * 1024 * 1024
    files = {
        stored: json.dumps({"type": "user", "entrypoint": "cli", "sessionId": CLAUDE_ID,
                            "version": "2.1.202", "cwd": "/fixture/CLI"}) + "\n",
        desktop_session: json.dumps({"type": "user", "entrypoint": "claude-desktop", "sessionId": CLAUDE_DESKTOP_ID,
                          "version": "2.1.222", "cwd": "/fixture/Desktop"}) + "\n",
        blocked: json.dumps({"type": "user", "entrypoint": "cli", "sessionId": blocked_id}) + "\n",
        agent: json.dumps({"type": "user", "sessionId": CLAUDE_ID, "agentId": CLAUDE_AGENT_ID,
                           "isSidechain": True}) + "\n",
        agent.with_suffix(".meta.json"): "{}",
        registration: json.dumps({"sessionId": f"local_{CLAUDE_DESKTOP_ID}",
                                   "cliSessionId": CLAUDE_DESKTOP_ID, "cwd": "/fixture/Desktop"}),
        shared: '{"preserve":"scheduler"}',
        sidecar: '{}',
        env: "environment fixture",
        task: "{}",
        debug: "debug fixture",
        checkpoint: "checkpoint fixture",
        desktop_checkpoint: "Desktop checkpoint fixture",
        unknown_output: "unknown session output",
        deep_output: "deeply nested output",
    }
    for path, value in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value)
    wide.mkdir()
    wide_files = {wide / f"result-{index}" for index in range(8200)}
    for path in wide_files:
        path.touch()
    empty.mkdir(parents=True)
    link.symlink_to(outside, target_is_directory=True)
    history.write_bytes(initial_history)
    legacy = base / "home" / ".agentmender" / "claude-backups" / "existing-backup"
    legacy.mkdir(parents=True)
    (legacy / "keep").write_bytes(b"existing copy")

    with ui(binary, base, None, root, extra_args=("--claude-desktop-root", str(desktop))) as (client, ticket):
        authorize(client, ticket)
        paths = {row["native_id"]: f"/api/claude/sessions/{row['key']}"
                 for row in client.call("GET", "/api/claude/sessions")["rows"]}
        path = paths[CLAUDE_ID]
        review = client.call("POST", path + "/removal-preview")
        assert review["plan"]["history_matches"] == 1
        listed = {Path(item["root"]) / item["relative_path"] for item in review["plan"]["files"]}
        assert "mode" not in review["plan"]
        assert {stored, agent, agent.with_suffix(".meta.json"), sidecar, env, task, debug,
                checkpoint, unknown_output, deep_output, link} | wide_files == listed
        directories = {Path(item["root"]) / item["relative_path"] for item in review["plan"]["directories"]}
        assert {nested, deep_output.parent, wide, empty, checkpoint.parent} <= directories
        assert review["plan"]["logical_bytes"] == sum(item.stat().st_size for item in listed if item != link)
        confirm = {"confirm": True, "review": review["review"]}
        client.call("POST", path + "/delete", confirm, code=403, Origin=None)
        client.call("POST", path + "/delete", {**confirm, "path": str(desktop_session)}, code=422)
        client.call("POST", path + "/delete", {"confirm": False, "review": review["review"]}, code=400)
        client.call("POST", paths[blocked_id] + "/delete", confirm, code=409)
        assert link.is_symlink() and all(item.is_file() for item in listed if item != link)
        result = client.call("POST", path + "/delete", confirm)
        assert result["status"] == "deleted" and not result["errors"], result
        assert all(not item.exists() for item in listed)
        assert not nested.exists() and all(not entry.exists() for entry in directories)
        assert sentinel.read_bytes() == b"another archive"
        assert history.read_bytes() == retained_history
        assert not (root / "history.jsonl.lock").exists() and not list(root.glob("history.jsonl.*.tmp"))
        assert not env.parent.exists() and not task.parent.exists() and not agent.parent.exists()
        client.call("POST", path + "/delete", confirm, code=409)
        assert client.call("GET", "/api/claude/sessions")["total"] == 2

        lock = root / "history.jsonl.lock"
        lock.mkdir()
        review = client.call("POST", paths[CLAUDE_DESKTOP_ID] + "/removal-preview")
        listed = {Path(item["root"]) / item["relative_path"] for item in review["plan"]["files"]}
        assert {desktop_session, registration, desktop_checkpoint} == listed, review
        result = client.call("POST", paths[CLAUDE_DESKTOP_ID] + "/delete", {"confirm": True, "review": review["review"]})
        assert result["status"] == "partial" and "occupata" in str(result["errors"]), result
        assert all(not item.exists() for item in listed) and not desktop_checkpoint.parent.exists()
        assert blocked.read_text() == files[blocked] and shared.read_text() == files[shared]
        assert history.read_bytes() == retained_history and lock.is_dir()
        assert client.call("GET", "/api/claude/sessions")["total"] == 1

        # History can be absent. A replaced transcript must be left in place.
        history.unlink()
        lock.rmdir()
        path = paths[blocked_id]
        review = client.call("POST", path + "/removal-preview")
        replacement = base / "replacement-transcript"
        replacement.write_bytes(blocked.read_bytes())
        replacement.replace(blocked)
        result = client.call("POST", path + "/delete", {"confirm": True, "review": review["review"]})
        assert result["status"] == "failed" and blocked.exists(), result
        review = client.call("POST", path + "/removal-preview")
        result = client.call("POST", path + "/delete", {"confirm": True, "review": review["review"]})
        assert result["status"] == "deleted" and not history.exists(), result
        assert client.call("GET", "/api/claude/sessions")["total"] == 0
        client.call("GET", "/api/claude/backups", code=404)
        assert set(legacy.parent.iterdir()) == {legacy}
        assert (legacy / "keep").read_bytes() == b"existing copy"


def claude_shared_uuid_cleanup(binary, base):
    root = base / "claude-shared-uuid"
    selected, duplicate = (root / "projects" / ns / f"{CLAUDE_ID}.jsonl" for ns in ("selected", "duplicate"))
    outside = duplicate.with_suffix("") / "keep.txt"
    selected_nested = selected.with_suffix("") / "remove.txt"
    history = root / "history.jsonl"
    checkpoint = root / "file-history" / CLAUDE_ID / "shared"
    row = json.dumps({"type": "user", "sessionId": CLAUDE_ID}).encode() + b"\n"
    for path in (selected, selected_nested, history, checkpoint):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(row)
    with ui(binary, base, None, root) as (client, ticket):
        authorize(client, ticket)
        row_key = client.call("GET", "/api/claude/sessions")["rows"][0]["key"]
        path = f"/api/claude/sessions/{row_key}"
        review = client.call("POST", path + "/removal-preview")
        assert review["plan"]["history_matches"] == 1
        outside.parent.mkdir(parents=True)
        duplicate.write_bytes(row)
        outside.write_bytes(b"other namespace")
        outcome = client.call("POST", path + "/delete", {"confirm": True, "review": review["review"]})
        assert outcome["status"] == "partial" and "stesso ID" in str(outcome["errors"]), outcome
        assert not selected.exists() and not selected_nested.parent.exists()
        assert duplicate.read_bytes() == history.read_bytes() == checkpoint.read_bytes() == row
        assert outside.read_bytes() == b"other namespace"



@contextmanager
def codex_desktop_catalog(root):
    directory = root / "sqlite"
    directory.mkdir()
    database = directory / "codex.db"
    schema = Path(__file__).resolve().parents[1] / "crates/provider-codex/test_data/desktop-reconciliation.sql"
    with closing(sqlite3.connect(database)) as db:
        db.execute("PRAGMA journal_mode=WAL").fetchone()
        db.executescript(schema.read_text())
        with db:
            db.execute("INSERT INTO local_thread_catalog_sync_state VALUES ('local',1,1,7,123), ('remote',2,1,8,456)")
            db.executemany("""
                INSERT INTO local_thread_catalog
                    (host_id, thread_id, display_title, source_created_at, source_updated_at, source_kind, observation_sequence)
                VALUES (?, ?, 'Disposable', 1, 1, 'cli', 7)
                """, [("local", probe.R), ("local", probe.REPLACEMENT), ("remote", probe.R)])
        yield db
    database.unlink()


def codex_reviewed_delete(client, path, review=None):
    review = review or client.call("POST", path + "/removal-preview")["review"]
    return client.call("POST", path + "/delete", {"confirm": True, "review": review})


def codex_fixture_executable(root, rules=None):
    """Native-command double that mutates only the isolated fixture archive."""
    rules = rules or {}
    (root / "fixture-rules.json").write_text(json.dumps(rules))
    executable = root / "codex-fixture"
    executable.write_text(f"""#!/bin/sh
if [ "$1" = --version ]; then printf 'codex-cli 99.0.0\n'; exit; fi
exec {shlex.quote(sys.executable)} {shlex.quote(str(root / "native-delete.py"))} "$@"
""")
    executable.chmod(0o700)
    (root / "native-delete.py").write_text('''
from pathlib import Path
import json
import os
import sqlite3
import sys

_, option, override, command, ident, flag = sys.argv
root = Path.cwd()
assert (option, command, flag) == ("-c", "delete", "--force")
assert Path(os.environ["CODEX_HOME"]) == root
assert override == "sqlite_home=" + json.dumps(str(root))
with (root / "native-calls.jsonl").open("a") as log:
    print(json.dumps(ident), file=log)
rules = json.loads((root / "fixture-rules.json").read_text())
files = rules.get("rollouts", {})
with sqlite3.connect(root / "state_5.sqlite") as db:
    def present(id):
        return db.execute("SELECT 1 FROM threads WHERE id=?", (id,)).fetchone() or (
            id in files and (root / files[id]).is_file())
    blocker = next((child for child in rules.get("blocks", {}).get(ident, []) if present(child)), None)
    if blocker:
        sys.exit(f"cannot delete thread while attached thread {blocker} exists; delete the attached thread first")
    for target in [ident, *rules.get("cascade", {}).get(ident, [])]:
        if target in rules.get("retain", []):
            continue
        db.execute("DELETE FROM threads WHERE id=?", (target,))
        db.execute("DELETE FROM thread_spawn_edges WHERE parent_thread_id=? OR child_thread_id=?", (target, target))
        if target in files:
            (root / files[target]).unlink(missing_ok=True)
print("Native deletion fixture succeeded")
''')
    return executable


def codex_graph_fixture(base, name, *, cascade=False, refusal=False, retain=False):
    root = base / name
    codex_metadata_fixture(root, 180)
    parent = probe.session_id(0)
    a, b, c, d, fork = map(probe.session_id, range(1000, 1005))
    files = {
        a: Path("archived_sessions/relocated") / f"rollout-{a}.jsonl",
        c: Path("sessions/2026/11/18") / f"rollout-{c}.jsonl",
        d: Path("archived_sessions/deep/nested") / f"rollout-{d}.jsonl",
    }
    for child, related in [(a, parent), (c, a), (d, c)]:
        path = root / files[child]
        path.parent.mkdir(parents=True, exist_ok=True)
        source = {"subagent": {"thread_spawn": {"parent_thread_id": related}}}
        path.write_text(json.dumps({"type": "session_meta", "payload": {
            "id": child, "cli_version": "fixture", "source": source}}) + "\n")
    with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
        db.executemany("""INSERT INTO threads
            (id, title, cwd, created_at, updated_at, archived, rollout_path, source, attached_thread_parent_id, forked_from_id)
            VALUES (?, 'Generic fixture', '/fixture', 0, 1, ?, ?, ?, ?, ?)""", [
            (a, 1, str(root / files[a]), "cli", None, None),
            (b, 1, None, "cli", parent, None),
            (c, 0, str(root / files[c]), json.dumps({"subagent": {"thread_spawn": {"parent_thread_id": a}}}), None, None),
            (fork, 1, None, "cli", None, parent),
        ])
        db.executemany("INSERT INTO thread_spawn_edges VALUES (?,?)", [(parent, a), (a, c)])
    rules = {"rollouts": {ident: str(path) for ident, path in files.items()}}
    if cascade:
        rules["cascade"] = {parent: [a, c, d]}
    if refusal:
        rules["blocks"] = {parent: [b]}
    if retain:
        rules["retain"] = [d]
    executable = codex_fixture_executable(root, rules)
    return root, parent, (a, b, c, d), fork, executable


def codex_graph_acceptance(binary, base, claude):
    for name, options in [
        ("residuals", {}), ("cascade", {"cascade": True}), ("refusal", {"refusal": True}),
        ("still-present", {"retain": True}),
    ]:
        root, parent, (a, b, c, d), fork, executable = codex_graph_fixture(base, name, **options)
        ids = [parent, a, b, c, d, fork]
        for filename, field in [("history.jsonl", "session_id"), ("session_index.jsonl", "id")]:
            (root / filename).write_text("".join(json.dumps({field: ident}) + "\n" for ident in ids))
        state_path = root / ".codex-global-state.json"
        state_path.write_text(json.dumps({"projectless-thread-ids": ids}))
        with codex_desktop_catalog(root) as desktop, ui(
            binary, base, root, claude, executable, max_entries=12,
        ) as (client, ticket):
            with desktop:
                desktop.executemany("""INSERT INTO local_thread_catalog
                    (host_id, thread_id, display_title, source_created_at, source_updated_at, source_kind, observation_sequence)
                    VALUES ('local', ?, 'Fixture', 1, 1, 'cli', 7)""", [(ident,) for ident in ids])
            authorize(client, ticket)
            path = f"/api/codex/sessions/{scoped_key('codex', root, parent)}"
            client.call("POST", path + "/delete", {"confirm": True}, code=409)
            review = client.call("POST", path + "/removal-preview")
            tree, = review["plan"]["trees"]
            assert tree["id"] == parent
            assert [(node["id"], node["parent"], node["depth"], node["archived"]) for node in tree["descendants"]] == [
                (a, parent, 1, True), (b, parent, 1, True), (c, a, 2, False), (d, c, 3, True)]
            outcome = codex_reviewed_delete(client, path, review["review"])
            expected = [parent, b] if name == "cascade" else [parent, d, c, b, a]
            if name == "refusal":
                expected += [parent]
            assert [json.loads(line) for line in (root / "native-calls.jsonl").read_text().splitlines()] == expected
            retained = [d, fork] if name == "still-present" else [fork]
            assert outcome["status"] == ("partial" if name == "still-present" else "deleted"), outcome
            if name == "still-present":
                assert any(d in error and "ancora presente" in error for error in outcome["errors"]), outcome
            for filename, field in [("history.jsonl", "session_id"), ("session_index.jsonl", "id")]:
                assert [json.loads(line)[field] for line in (root / filename).read_text().splitlines()] == retained
            assert json.loads(state_path.read_text())["projectless-thread-ids"] == retained
            assert {row[0] for row in desktop.execute(
                "SELECT thread_id FROM local_thread_catalog WHERE host_id='local'"
            )} == {*retained, probe.R, probe.REPLACEMENT}
            assert {row[0] for row in desktop.execute(
                "SELECT thread_id FROM local_thread_catalog_scan_entries WHERE host_id='local' AND removed=1"
            )} == set(ids) - set(retained)
            with closing(sqlite3.connect(root / "state_5.sqlite")) as db:
                assert db.execute("SELECT 1 FROM threads WHERE id=?", (fork,)).fetchone()
                assert not db.execute("SELECT 1 FROM threads WHERE id=?", (parent,)).fetchone()
            assert not list(root.glob("*.agentmender-*.tmp"))

    root, parent, (a, _b, c, d), fork, executable = codex_graph_fixture(base, "unreviewed")
    path = f"/api/codex/sessions/{scoped_key('codex', root, parent)}"
    with ui(binary, base, root, claude, executable, extra_args=("--protect-codex-session", d)) as (client, ticket):
        authorize(client, ticket)
        client.call("POST", path + "/removal-preview", code=409)
    with ui(binary, base, root, claude, executable) as (client, ticket):
        authorize(client, ticket)
        review = client.call("POST", path + "/removal-preview")
        with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
            db.execute("INSERT INTO thread_spawn_edges VALUES (?,?)", (c, fork))
        client.call("POST", path + "/delete", {"confirm": True, "review": review["review"]}, code=409)
        with closing(sqlite3.connect(root / "state_5.sqlite")) as db, db:
            db.execute("INSERT INTO thread_spawn_edges VALUES (?,?)", (c, a))
        client.call("POST", path + "/removal-preview", code=409)
    assert not (root / "native-calls.jsonl").exists()
    print("Codex graph: verified spawn, attachment and rollout relations, native order, stale reviews and residuals.")


def codex_partial_cleanup(binary, base, claude):
    root = base / "codex-partial"
    codex_metadata_fixture(root, 2)
    ident = probe.session_id(0)
    executable = codex_fixture_executable(root)
    index = root / "session_index.jsonl"
    kept = json.dumps({"id": probe.session_id(1)}).encode()
    index.write_bytes(json.dumps({"id": ident}).encode() + b"\r\n" + kept)
    state = root / ".codex-global-state.json"
    state.write_bytes(b"malformed JSON")
    with ui(binary, base, root, claude, executable) as (client, ticket):
        authorize(client, ticket)
        row = next(row for row in client.call("GET", "/api/codex/sessions")["rows"]
                   if row["native_id"] == ident)
        outcome = codex_reviewed_delete(client, f"/api/codex/sessions/{row['key']}")
        assert outcome["status"] == "partial", outcome
        assert "Native deletion fixture succeeded" in outcome["message"] and "incompleta" in outcome["message"]
        assert len(outcome["errors"]) == 1 and state.name in outcome["errors"][0]
        assert index.read_bytes() == kept and state.read_bytes() == b"malformed JSON"
        assert not state.with_suffix(".json.bak").exists()



def codex_acceptance(client, root, rollouts):
    query = "/api/codex/sessions"
    page = client.call("GET", query)
    keys = {item["native_id"]: item["key"] for item in page["rows"]}
    assert page["provider"]["removal_available"] and len(keys) > 2
    assert probe.C not in keys
    target = f"{query}/{keys[probe.R]}"
    other = f"{query}/{keys[probe.REPLACEMENT]}"
    transcript = client.call("GET", target + "/conversation")
    assert any("Visible disposable" in message["text"] for message in transcript["messages"])
    assert "secret-marker" not in json.dumps(transcript)
    cursor = transcript["next"]
    assert cursor
    client.call("GET", other + f"/conversation?cursor={cursor}", code=400)
    assert client.call("GET", target + f"/conversation?cursor={cursor}")["messages"]
    execute = target + "/delete"
    client.call("POST", execute, {"confirm": True}, code=401, Authorization=None)
    client.call("POST", execute, {"confirm": True}, code=403, Origin="null")
    client.call("POST", execute, {"confirm": False}, code=400)
    client.call("POST", execute, {"confirm": True, "args": ["--other"]}, code=422)
    client.call("POST", f"{query}/{keys[probe.S]}/delete", {"confirm": True}, code=409)
    child = f"{query}/{scoped_key('codex', root, probe.C)}"
    client.call("POST", child + "/removal-preview", code=404)
    unrelated = rollouts[probe.REPLACEMENT].read_bytes()
    result = codex_reviewed_delete(client, target)
    assert result["status"] == "deleted", result
    assert not rollouts[probe.R].exists()
    assert rollouts[probe.REPLACEMENT].read_bytes() == unrelated
    assert probe.R not in {row["native_id"] for row in client.call("GET", query)["rows"]}
    client.call("POST", target + "/removal-preview", code=404)


def codex_project_deletion(binary, base, root, claude, executable, rollouts):
    workspace = root.parent / "cwd" / "bulk-project"
    workspace.mkdir()
    source = workspace / "keep.txt"
    source.write_bytes(b"project source is outside session deletion")
    with probe.FixtureServer(root.parent, executable, base / "private-canary") as server:
        response = server.call("project/import", {
            "name": "Bulk fixture", "roots": [{"path": str(workspace)}],
            "threads": [probe.M, probe.REPLACEMENT, probe.G], "idempotencyKey": "deletion-refactor",
        })
        assert "result" in response, response
    unrelated = rollouts[probe.S].read_bytes()
    with ui(binary, base, root, claude, executable) as (client, ticket):
        authorize(client, ticket)
        path = project_path(client, workspace)
        before = client.call("GET", path)
        assert before["main_conversations"] == 3 and before["removal_available"], before
        review = client.call("POST", path + "/removal-preview")
        descendants = [child for tree in review["plan"]["trees"] for child in tree["descendants"]]
        assert [(child["id"], child["parent"]) for child in descendants] == [(probe.C, probe.G)]
        result = codex_reviewed_delete(client, path, review["review"])
        assert result["status"] == "deleted", result
        assert client.call("GET", path)["main_conversations"] == 0
        assert all(not rollouts[ident].exists() for ident in (probe.M, probe.REPLACEMENT, probe.G, probe.C))
        assert rollouts[probe.S].read_bytes() == unrelated
        assert source.read_bytes() == b"project source is outside session deletion"


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
        client.call("POST", scoped + "/delete", {"confirm": True}, code=404)
    assert unknown.read_bytes() == b"outside transcript contract" and same.is_dir()
    observed = client.call("GET", query + "?search=observed-empty")["rows"]
    assert len(observed) == 1 and observed[0]["main_conversations"] == 0
    assert duplicate.read_bytes() == original.read_bytes()
    return projects


def project_path(client, workspace):
    page = client.call("GET", PROJECTS)
    return next(f"{PROJECTS}/{row['key']}" for row in page["rows"]
                if row["workspace"] == str(workspace))


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
        client.call("POST", path + "/delete", {"confirm": True}, code=409)
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
        client.call("POST", legacy + "/delete", {"confirm": True}, code=409)

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
    external = base / "explicit-outputs"
    external.mkdir()
    environment = {"CODEX_HOME": str(archive), "XDG_DATA_HOME": str(other),
                   "ARTIFACT_SESSION_HOST_EXPORT_ROOT": str(external)}
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
        unknown = exports / "unknown"
        (unknown / "nested").mkdir(parents=True)
        (unknown / "nested" / "data.txt").write_bytes(b"nested")
        output = external / "explicit-report.txt"
        output.write_bytes(b"report")
        replaced = exports / "replace.zip"
        replaced.write_bytes(b"old")
        denied = exports / "denied-bundle"
        denied.mkdir()
        (denied / "report.zip").write_bytes(b"denied")
        incomplete = exports / "incomplete-bundle"
        incomplete.mkdir()
        (incomplete / ".artifact-session-publication-lease.json").write_bytes(b"invalid lease")
        leased = exports / "active-lease"
        leased.mkdir()
        (leased / ".artifact-session-publication-lease.json").write_text(json.dumps({
            "publicationId": "a" * 64, "minimumRetainUntilMs": 9_000_000_000_000,
        }))
        for index in range(45):
            (exports / f"data-{index:02}.zip").write_bytes(bytes(index))
        huge = exports / "sparse-fixture.zip"
        with huge.open("wb") as stream:
            stream.truncate(1_000_000)
        outside = exports / "outside"
        hardlink = exports / "hardlink-fixture"
        can_delete = os.name != "nt"
        if can_delete:
            os.link(workspace_file, hardlink)
            outside.symlink_to(user_output, target_is_directory=True)
            (unknown / "nested" / "linked-directory").symlink_to(user_output, target_is_directory=True)
            os.mkfifo(incomplete / "pipe")
        else:
            # Directory junctions do not require symlink privileges.
            subprocess.run(["cmd.exe", "/d", "/c", "mklink", "/J", str(outside), str(user_output)],
                           check=True, capture_output=True)
        client.call("POST", "/api/storage/exports/refresh", code=403, Origin=None)
        client.call("POST", "/api/storage/exports/refresh", code=204)
        first = client.call("GET", "/api/storage/exports?limit=4")
        second = client.call("GET", "/api/storage/exports?limit=4&offset=4")
        assert first["rows"][0]["name"] == huge.name and first["total"] > 45
        assert not {row["key"] for row in first["rows"]} & {row["key"] for row in second["rows"]}
        rows = {row["name"]: row for row in client.call("GET", "/api/storage/exports?limit=100")["rows"]}
        assert first["removal_available"] == can_delete
        entry = client.call("GET", "/api/storage/exports?search=Caf%C3%A9")["rows"][0]
        assert entry["protection"] == "independent_backup" and entry["manifest"] == "manifest.json"
        assert rows[output.name]["protection"] == "user_output"
        links = rows[outside.name]
        assert links["size"]["files"] == 0 and links["protection"] == "shared"
        assert links["removal_available"] == can_delete
        assert rows[incomplete.name]["protection"] == "incomplete"
        assert rows[incomplete.name]["removal_available"] == can_delete
        assert rows[leased.name]["protection"] == "in_use"
        assert rows[leased.name]["removal_available"] == can_delete
        if can_delete:
            linked = rows[hardlink.name]
            assert linked["shared_entries"] == 1 and linked["removal_available"]

        def review(*names):
            return client.call("POST", "/api/storage/exports/preview", {"keys": [rows[name]["key"] for name in names]})

        def confirmation(receipt):
            return f"/api/storage/exports/operations/{receipt}/confirm"

        preview_route = "/api/storage/exports/preview"
        client.call("POST", preview_route, {"paths": [str(backup)]}, code=422)
        client.call("POST", preview_route, {"keys": ["f" * 64]}, code=409)
        client.call("POST", preview_route, {"keys": [str(backup)]}, code=400)
        client.call("POST", preview_route, {"keys": [entry["key"]] * 2}, code=400)
        client.call("POST", preview_route, {"keys": [row["key"] for row in list(rows.values())[:33]]}, code=400)
        client.call("POST", confirmation("forged"), {"confirm": True}, code=409)
        if not can_delete:
            read_only = review(outside.name, incomplete.name, leased.name, backup.name)
            assert read_only["removable"] == 0 and read_only["protected"] == 4 and read_only["receipt"] is None
            return

        before = set(exports.iterdir())
        single = review(huge.name)
        assert single["removable"] == 1 and single["protected"] == 0
        assert single["size"]["logical_bytes"] == 1_000_000 and single["size"]["allocated_bytes"] is not None
        confirm = confirmation(single["receipt"])
        client.call("POST", confirm, {"confirm": False}, code=400)
        client.call("POST", confirm, {"confirm": True, "keys": [entry["key"]]}, code=422)
        client.call("POST", confirm, {"confirm": True}, code=403, Origin=None)
        client.call("POST", confirm, {"confirm": True}, code=401, Authorization=None)
        assert huge.exists()
        outcome = client.call("POST", confirm, {"confirm": True})
        assert [row["name"] for row in outcome["deleted"]] == [huge.name] and not outcome["not_deleted"]
        client.call("POST", confirm, {"confirm": True}, code=409)
        assert not huge.exists() and set(exports.iterdir()) == before - {huge}
        assert client.call("GET", "/api/storage/exports?search=sparse-fixture")["total"] == 0

        superseded = review(backup.name)
        deleted_exports = [backup, outside, hardlink, unknown, incomplete, leased]
        deleted_names = [path.name for path in deleted_exports] + [output.name]
        selected = [replaced.name, *deleted_names[:-1]]
        permission_test = os.geteuid() != 0
        if permission_test:
            selected.append(denied.name)
        selected.append(output.name)
        preview = review(*selected)
        assert preview["removable"] == len(selected) and preview["protected"] == 0 and not preview["complete"]
        assert preview["size"]["files"] >= 6 and preview["size"]["directories"] >= 4
        client.call("POST", confirmation(superseded["receipt"]), {"confirm": True}, code=409)
        old = base / "original.zip"
        replaced.rename(old)
        replaced.symlink_to(workspace_file)
        if permission_test:
            denied.chmod(0o555)
        try:
            outcome = client.call("POST", confirmation(preview["receipt"]), {"confirm": True})
        finally:
            denied.chmod(0o755)
        assert [row["name"] for row in outcome["deleted"]] == deleted_names
        preserved = {row["name"]: row for row in outcome["not_deleted"]}
        assert set(preserved) == set(selected) - set(deleted_names)
        assert preserved[replaced.name]["error"]
        if permission_test:
            assert preserved[denied.name]["error"] and "Permesso negato" in preserved[denied.name]["reason"]
        client.call("POST", preview_route, {"keys": [rows[replaced.name]["key"]]}, code=409)
        remaining = {row["name"] for row in client.call("GET", "/api/storage/exports?limit=100")["rows"]}
        assert remaining == set(rows) - {huge.name} - set(deleted_names)
        assert set(exports.iterdir()) == before - {huge, *deleted_exports} and not list(external.iterdir())
        assert replaced.is_symlink() and old.read_bytes() == b"old"
        assert (denied / "report.zip").read_bytes() == b"denied"
        assert (exports / "data-01.zip").read_bytes() == b"\x00"
        assert workspace_file.read_bytes() == b"user workspace"

        (external / workspace_file.name).write_bytes(b"export")
        client.call("POST", "/api/storage/exports/refresh", code=204)
        external_row = client.call("GET", "/api/storage/exports?search=keep.txt")["rows"][0]
        root_review = client.call("POST", preview_route, {"keys": [external_row["key"]]})
        moved = base / "moved-exports"
        external.rename(moved)
        external.symlink_to(user_output, target_is_directory=True)
        outcome = client.call("POST", confirmation(root_review["receipt"]), {"confirm": True})
        assert not outcome["deleted"] and outcome["not_deleted"][0]["error"]
        assert (moved / workspace_file.name).read_bytes() == b"export"
        assert workspace_file.read_bytes() == b"user workspace"

    sessions = archive / "sessions"
    sessions.mkdir()
    rollout = sessions / "rollout-canary.jsonl"
    rollout.write_bytes(b"fixture session")
    alias = archive / "SESSIONS"
    environment["ARTIFACT_SESSION_HOST_EXPORT_ROOT"] = str(alias if alias.is_dir() else sessions)
    with ui(binary, base, None, claude, env_overrides=environment) as (client, ticket):
        authorize(client, ticket)
        row = client.call("GET", "/api/storage/exports?search=rollout-canary")["rows"][0]
        assert not row["removal_available"] and "si sovrappone" in row["reason"]
        assert client.call("POST", preview_route, {"keys": [row["key"]]})["receipt"] is None
    assert rollout.read_bytes() == b"fixture session"
    print("Codex exports: permanent scoped deletion across classifications, stale selections, partial results and inventory refresh.")


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
        claude_multiple_roots(agentmender, base, claude)
        claude_reconciliation(agentmender, base)
        claude_storage_acceptance(agentmender, base)
        storage_acceptance(agentmender, base / "storage-suite", claude)
        if platform.system() == "Darwin":
            claude_cleanup_acceptance(agentmender, base)
            claude_shared_uuid_cleanup(agentmender, base)
            codex_partial_cleanup(agentmender, base, claude)
            codex_graph_acceptance(agentmender, base, claude)
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
                failing = base / "codex-failing"
                failing.write_text("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'codex-cli 99.0.0\\n'; exit; fi\nprintf 'native fixture refusal\\n' >&2\nexit 1\n")
                failing.chmod(0o700)
                with codex_desktop_catalog(root) as desktop:
                    sync = "SELECT * FROM local_thread_catalog_sync_state ORDER BY host_id"
                    catalog = "SELECT * FROM local_thread_catalog ORDER BY host_id, thread_id"
                    entries = desktop.execute(catalog).fetchall()
                    projections = {}
                    for filename, field in [("history.jsonl", "session_id"), ("session_index.jsonl", "id")]:
                        path = root / filename
                        kept = b"malformed fixture row\r\n" + json.dumps({field: probe.REPLACEMENT}).encode()
                        original = json.dumps({field: probe.R}).encode() + b"\r\n" + kept
                        path.write_bytes(original)
                        projections[path] = (original, kept)
                    maps = ("thread-project-assignments", "thread-projectless-output-directories",
                            "thread-writable-roots", "thread-workspace-root-hints")
                    state = {name: {probe.R: {"fixture": name}, probe.REPLACEMENT: ["keep"]}
                             for name in maps}
                    state.update({"projectless-thread-ids": [probe.R, probe.REPLACEMENT, probe.R],
                                  "local-projects": [{"id": "keep-project", "path": "/fixture/workspace"}],
                                  "unrelated": {"large": 987654321012345678901234567890, "reference": probe.R}})
                    retained_state = {name: {probe.REPLACEMENT: ["keep"]} for name in maps}
                    retained_state.update({"projectless-thread-ids": [probe.REPLACEMENT],
                                           "local-projects": state["local-projects"], "unrelated": state["unrelated"]})
                    state_paths = [root / name for name in (".codex-global-state.json", ".codex-global-state.json.bak")]
                    for path in state_paths:
                        path.write_text(json.dumps(state))
                    with ui(agentmender, base, root, claude, failing) as (client, ticket):
                        authorize(client, ticket)
                        page = client.call("GET", "/api/codex/sessions")
                        assert page["provider"]["removal_available"]
                        row = next(row for row in page["rows"] if row["native_id"] == probe.REPLACEMENT)
                        result = codex_reviewed_delete(client, f"/api/codex/sessions/{row['key']}")
                        assert result["status"] == "failed" and result["errors"] == [f"{probe.REPLACEMENT}: native fixture refusal"], result
                        assert rollouts[probe.REPLACEMENT].is_file()
                    assert desktop.execute(sync).fetchall() == [("local", 1, 1, 7, 123), ("remote", 2, 1, 8, 456)]
                    assert desktop.execute(catalog).fetchall() == entries
                    assert all(path.read_bytes() == original for path, (original, _) in projections.items())
                    assert all(json.loads(path.read_bytes()) == state for path in state_paths)
                    with ui(agentmender, base, root, claude, executable_copy) as (client, ticket):
                        authorize(client, ticket)
                        inventory_acceptance(client, claude)
                        codex_acceptance(client, root, rollouts)
                    assert desktop.execute(sync).fetchall() == [("local", 1, 1, 8, None), ("remote", 2, 1, 8, 456)]
                    assert desktop.execute(catalog).fetchall() == [row for row in entries if row[:2] != ("local", probe.R)]
                    assert desktop.execute("SELECT * FROM local_thread_catalog_scan_entries").fetchall() == [("local", probe.R, 1)]
                    assert desktop.execute("SELECT catalog_revision FROM local_thread_catalog_metadata").fetchone() == (26,)
                    assert all(path.read_bytes() == kept for path, (_, kept) in projections.items())
                    assert all(json.loads(path.read_bytes()) == retained_state for path in state_paths)
                    assert not list(root.glob("*.agentmender-*.tmp"))
                assert not (root / "sqlite/codex.db").exists()
                for path in state_paths:
                    path.unlink()
                codex_project_deletion(agentmender, base, root, claude, executable_copy, rollouts)
            print("Native Codex fixture: command result, success-only auxiliary cleanup, Desktop reconciliation, and deletion without a Desktop catalog.")
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
            print("Local UI: scoped duplicate IDs, provider inventory, permanent Claude deletion, origin and token guards, pagination and coverage.")


if __name__ == "__main__":
    main()
