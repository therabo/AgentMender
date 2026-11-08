#!/usr/bin/env python3
"""Create fresh synthetic P0-C1 search data, never a real provider root.

No path, credential, approval or execution input is accepted. The protected UUID
is a fixture peer, NOT the identity of the calling Codex conversation. This helper
does not establish caller protection, client acceptance or deletion acceptance.
"""
import json
import os
from pathlib import Path
import platform
import sqlite3
import sys
import tempfile


MANAGEMENT_PEER = "12345678-1234-1234-1234-123456789abd"


def seed(root, project):
    for name in ("sessions", ".tmp", "thread-writer-locks"):
        (root / name).mkdir(mode=0o700)
    (root / ".tmp/rollout-maintenance.lock").write_bytes(b"")
    (root / "thread-writer-locks/.coordination.lock").write_bytes(b"")
    db = sqlite3.connect(root / "state_5.sqlite")
    db.execute("PRAGMA journal_mode=WAL")
    db.executescript("CREATE TABLE fixture_sequence_seed(value INTEGER PRIMARY KEY AUTOINCREMENT); DROP TABLE fixture_sequence_seed;")
    schema_path = Path(__file__).resolve().parent.parent / "compatibility/codex/0.165.0-alpha.1.2/state-schema-5.json"
    schema = json.loads(schema_path.read_text())
    for kind in ("table", "index", "trigger", "view"):
        for entry, name, _, sql in schema:
            if entry == kind and name != "sqlite_sequence" and sql:
                db.executescript(sql)
    cases = [
        ("abc", "MISSION1 README", "0.166.0", 0),
        ("abd", "Management fixture peer", "0.166.0", 0),
        ("ab0", "MISSION1 README", "0.166.0", 0),
        ("ab1", "MISSION1 archived notes", "0.166.0", 1),
        ("ab2", "", "0.166.0", 0),
        ("ab3", "MISSION1 unsupported producer", "0.166.0-alpha.6", 0),
    ]
    for suffix, title, version, archived in cases:
        ident = "12345678-1234-1234-1234-123456789" + suffix
        payload = root / f"sessions/rollout-2026-01-01T00-00-00-{ident}.jsonl"
        payload.write_text(json.dumps({"type": "session_meta", "payload": {
            "id": ident, "cli_version": version, "originator": "agentmender-synthetic-fixture"
        }}) + "\nSYNTHETIC TRANSCRIPT CANARY: MUST NOT BE EXPORTED\n")
        db.execute(
            "INSERT INTO threads(id,rollout_path,created_at,updated_at,source,model_provider,cwd,title,sandbox_policy,approval_mode,cli_version,archived,preview,first_user_message) VALUES(?,?,1767225600,1767312000,'cli','openai',?,?,'read-only','never',?,?,'PRIVATE PREVIEW CANARY','PRIVATE MESSAGE CANARY')",
            (ident, str(payload), str(project), title, version, archived),
        )
    db.commit()
    # End the fixture child after a committed WAL without closing SQLite. This
    # deliberately retains WAL/SHM for the protected reader; it is synthetic data,
    # not evidence that a real client produced an eligible Legacy conversation.
    os._exit(0)


def main():
    if len(sys.argv) != 1 or platform.system() != "Darwin" or platform.machine() != "arm64":
        raise SystemExit("No arguments; this disposable fixture is for macOS arm64 only")
    base = Path(tempfile.mkdtemp(prefix="agentmender-p0-c1-readonly-")).resolve()
    root, project = base / "provider", base / "MISSION1"
    root.mkdir(mode=0o700)
    project.mkdir(mode=0o700)
    child = os.fork()
    if child == 0:
        try:
            seed(root, project)
        except BaseException:
            os._exit(1)
    _, status = os.waitpid(child, 0)
    if not os.WIFEXITED(status) or os.WEXITSTATUS(status) != 0:
        raise SystemExit("Fixture creation failed; do not use the incomplete directory")
    manifest = {
        "fixture_only": True,
        "root": str(root),
        "state_root": str(base / "agentmender-state"),
        "project": str(project),
        "protected_fixture_peer": MANAGEMENT_PEER,
        "authenticated_caller_binding": False,
        "mutation_acceptance": False,
        "expected_mission1_matches": 6,
        "expected_duplicate_title_matches": 2,
    }
    (base / "fixture.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
