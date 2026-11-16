#!/usr/bin/env python3
"""HTTP acceptance on disposable archives; optionally verify the exact native binary."""
import argparse
from contextlib import contextmanager
from concurrent.futures import ThreadPoolExecutor
import http.client
import json
import os
from pathlib import Path
import platform
import selectors
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


def codex_metadata_fixture(root, count=3000):
    root.mkdir()
    with sqlite3.connect(root / "state_5.sqlite") as db:
        db.execute("CREATE TABLE threads(id TEXT PRIMARY KEY,title TEXT,cwd TEXT,created_at INTEGER,updated_at INTEGER,archived INTEGER,rollout_path TEXT)")
        db.executemany("INSERT INTO threads VALUES(?,?,?,?,?,?,?)", (
            (f"ab000000-1111-4111-8111-{index:012d}", f"Café session {index}" if index else None,
             "/fixture/Atlas", index, index + 86400, index % 2, None)
            for index in range(count)
        ))


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
            if value["status"] != "running":
                return value
            time.sleep(0.1)
        raise AssertionError("native operation did not finish")


@contextmanager
def ui(binary, base, codex_root, claude_root, codex=None):
    other = base / "empty-path"
    other.mkdir(exist_ok=True)
    home = base / "home"
    home.mkdir(exist_ok=True)
    environment = {"HOME": str(home), "USERPROFILE": str(home), "PATH": str(other)}
    if os.name == "nt":
        environment["SystemRoot"] = os.environ["SystemRoot"]
    command = [str(binary), "--codex-root", str(codex_root), "--claude-root", str(claude_root), "ui", "--no-open"]
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
        proc.terminate()
        proc.wait(timeout=15)
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
    assert len(sources) == 2 and "Never expose" in next(path for path in sources if path.stem == CLAUDE_ID).read_text()
    return row["key"]


def codex_acceptance(client, root, rollouts):
    page = client.call("GET", "/api/codex/sessions")
    assert page["provider"]["removal_available"], page["provider"]
    keys = {item["native_id"]: item["key"] for item in page["rows"]}
    target = f"/api/codex/sessions/{keys[probe.R]}"
    protected = f"/api/codex/sessions/{keys[probe.S]}"
    active = f"/api/codex/sessions/{keys[probe.F]}"
    parent = f"/api/codex/sessions/{keys[probe.G]}"
    assert client.call("GET", parent)["children"] == [probe.C]
    orphan_parent = f"/api/codex/sessions/{keys[probe.C]}"
    assert not client.call("GET", orphan_parent)["children"]
    for refused in (protected, active, parent, orphan_parent):
        client.call("POST", refused + "/removal-preview", code=409)
    client.call("POST", target + "/removal-preview", code=401, Authorization=None)
    client.call("POST", target + "/removal-preview", code=403, Origin="null")
    client.call("POST", target + "/removal-preview", code=403, Origin=None)
    transcript = client.call("GET", target + "/conversation")
    assert any("Visible disposable" in message["text"] for message in transcript["messages"])
    assert "secret-marker" not in json.dumps(transcript)
    cursor = transcript["next"]
    assert cursor and all(len(message["text"].encode()) <= 16384 for message in transcript["messages"])
    client.call("GET", protected + f"/conversation?cursor={cursor}", code=400)
    continuation = client.call("GET", target + f"/conversation?cursor={cursor}")
    assert continuation["messages"] and "secret-marker" not in json.dumps(continuation)
    frozen = {ident: path.read_bytes() for ident, path in rollouts.items() if ident != probe.R}
    preview = client.call("POST", target + "/removal-preview")
    path = f"/api/operations/{preview['id']}/confirm"
    phrase = {"confirmation": preview["confirmation"]}
    client.call("POST", "/api/operations/forged/confirm", phrase, code=409)
    client.call("POST", path, {"confirmed": True}, code=422)
    client.call("POST", path, {"confirmation": "wrong"}, code=409)
    # A same-identity append after preview must prevent the native command.
    with rollouts[probe.R].open("a") as out:
        out.write("\n")
    client.call("POST", path, phrase, code=202)
    assert client.operation(preview["id"])["status"] == "changed"
    assert rollouts[probe.R].is_file()
    client.call("POST", path, phrase, code=409)
    client.call("POST", "/api/codex/refresh")
    preview = client.call("POST", target + "/removal-preview")
    path = f"/api/operations/{preview['id']}/confirm"
    with ThreadPoolExecutor(max_workers=2) as pool:
        replies = [pool.submit(client.call, "POST", path, phrase, code=(202, 409)) for _ in range(2)]
        assert sorted(reply.result() for reply in replies) == [202, 409]
    assert client.operation(preview["id"])["status"] == "command_succeeded"
    assert not rollouts[probe.R].exists()
    assert all(rollouts[ident].read_bytes() == content for ident, content in frozen.items())
    with sqlite3.connect(root / "state_5.sqlite") as db:
        assert db.execute("SELECT 1 FROM threads WHERE id=?", (probe.R,)).fetchone() is None
        assert db.execute("SELECT 1 FROM threads WHERE id=?", (probe.S,)).fetchone() == (1,)
    remaining = client.call("GET", "/api/codex/sessions")["rows"]
    assert {item["native_id"] for item in remaining} == set(frozen)

    uncertain = f"/api/codex/sessions/{keys[probe.M]}"
    preview = client.call("POST", uncertain + "/removal-preview")
    # An invalid provider config causes the native command to exit nonzero.
    config = root / "config.toml"
    config.write_text("invalid = [")
    try:
        client.call("POST", f"/api/operations/{preview['id']}/confirm", phrase, code=202)
        assert client.operation(preview["id"])["status"] == "outcome_unknown"
    finally:
        config.unlink()
    client.call("POST", uncertain + "/removal-preview", code=409)
    assert rollouts[probe.M].read_bytes() == frozen[probe.M]


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
    parser.add_argument("--native-codex", type=Path, help="exact reviewed Darwin arm64 binary; only disposable writes")
    args = parser.parse_args()
    agentmender = args.agentmender.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="am-ui-") as directory:
        base = Path(directory).resolve()
        claude = base / "claude"
        claude_fixture(claude)
        if args.native_codex:
            binary = args.native_codex.resolve(strict=True)
            assert platform.system() == "Darwin" and platform.machine() == "arm64"
            assert probe.digest(binary) == probe.DIGEST, "binary does not match the validated profile"
            with probe.native_fixture(base, binary) as (root, rollouts):
                with ui(agentmender, base, root, claude, binary) as (client, ticket):
                    authorize(client, ticket)
                    inventory_acceptance(client, claude)
                    codex_acceptance(client, root, rollouts)
            print("Native Codex UI: stale preview refused, one-shot delete, controls preserved, nonzero exit held for review.")
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
                    second = client.call("GET", "/api/codex/sessions?limit=10&offset=10")
                    assert not {item["key"] for item in first["rows"]} & {item["key"] for item in second["rows"]}
                    selected = client.call("GET", "/api/codex/sessions?search=CAFE%CC%81%20Atlas&state=archived&since=1970-01-02&until=1970-01-02&sort=created&ascending=true&limit=2")
                    assert selected["total"] == 1500 and selected["rows"][0]["created_at"] == 1
                    assert not first["provider"]["removal_available"]
                    client.call("POST", f"/api/codex/sessions/{selected['rows'][0]['key']}/removal-preview", code=409)
                    missing_title = client.call("GET", "/api/codex/sessions?offset=2999")["rows"][0]
                    assert missing_title["title"] is None and missing_title["transcript_bytes"] is None
                    first_codex_key = missing_title["key"]
                    with sqlite3.connect(root / "state_5.sqlite") as db:
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
                client.call("GET", f"/api/claude/sessions/{first_claude_key}", code=404)
                if first_codex_key:
                    other = client.call("GET", "/api/codex/sessions")["rows"][0]
                    assert other["native_id"] == "ab000000-1111-4111-8111-000000000000"
                    assert other["key"] != first_codex_key
                    client.call("GET", f"/api/codex/sessions/{first_codex_key}", code=404)
            print("Local UI: scoped duplicate IDs, provider inventory, unsupported removal, origin and token guards, pagination and coverage.")


if __name__ == "__main__":
    main()
