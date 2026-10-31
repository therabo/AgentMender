#!/usr/bin/env python3
"""Opt-in exact 0.166.0 counterexamples on fresh synthetic macOS state only.

No provider-state path is accepted. This research tool is not an execution adapter,
approval bypass, compatibility importer, or proof of complete native effects.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import selectors
import stat
import subprocess
import sys
import tempfile
import time

VERSION = "0.166.0"
BINARY_SHA256 = "39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1"
MAX_OUTPUT = 8 * 1024 * 1024
METHODS = {"initialize", "thread/read", "thread/list", "thread/archive",
           "thread/unarchive", "thread/delete"}
R, C, G, S, U, F, M = [f"ab000000-1111-4111-8111-{n:012d}" for n in range(1, 8)]


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(65536):
            h.update(chunk)
    return h.hexdigest()


def lit(path):
    value = str(path.resolve())
    if not value.isascii() or any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ValueError("unreviewed Seatbelt path encoding")
    return json.dumps(value)


def policy(case, binary, real_home, denied=()):
    return ("(version 1)(allow default)"
            f"(deny file-write* (require-not (subpath {lit(case)})))"
            f"(deny file-read-data (require-all (subpath {lit(real_home)}) "
            f"(require-not (literal {lit(binary)}))))"
            "(deny network*)(deny process-fork)(deny mach-lookup)" +
            "".join(f"(deny file-write* (literal {lit(p)}))" for p in denied))


def new_case(base, name):
    case = base / name
    case.mkdir(mode=0o700)
    for name in ("home", "provider", "tmp", "cwd"):
        (case / name).mkdir(mode=0o700)
    return case


def validate_isolation(base, binary, home):
    """Kernel denial, not ordinary file modes, must reject each canary."""
    case = new_case(base, "isolation-canaries")
    code = """import errno,json,os,socket,sys
results={}
for name in ['outside_write','protected_directory_read','network','fork']:
 try:
  if name=='outside_write':
   with open(sys.argv[1],'wb') as f:f.write(b'canary')
  elif name=='protected_directory_read':os.listdir(sys.argv[2])
  elif name=='network':
   with socket.socket(socket.AF_INET,socket.SOCK_STREAM) as s:
    s.settimeout(1)
    s.connect(('127.0.0.1',9))
  else:
   pid=os.fork()
   if pid==0:os._exit(0)
   os.waitpid(pid,0)
  results[name]='ALLOWED'
 except OSError as e:results[name]='DENIED' if e.errno in (errno.EPERM,errno.EACCES) else 'OTHER_ERROR'
print(json.dumps(results))
"""
    result = subprocess.run(
        ["/usr/bin/sandbox-exec", "-p", policy(case, binary, home),
         str(Path(sys.executable).resolve()), "-I", "-c", code,
         str(base / "outside-canary"), str(home)],
        env={"HOME": str(case / "home"), "TMPDIR": str(case / "tmp"), "PATH": "/usr/bin:/bin"},
        cwd=case / "cwd", capture_output=True, timeout=10, check=True)
    if len(result.stdout) > 4096 or len(result.stderr) > 4096:
        raise ValueError("isolation canary output budget")
    results = json.loads(result.stdout)
    (case / "canaries.json").write_text(json.dumps(results, indent=2) + "\n")
    if set(results) != {"outside_write", "protected_directory_read", "network", "fork"} or any(
            value != "DENIED" for value in results.values()):
        raise ValueError(f"OS isolation not established; no provider process started: {results}")
    return results


def seed(case, ident, parent=None, archived=False, preview=True, attached=False,
         fork=None, project="target-project"):
    folder = case / "provider" / ("archived_sessions" if archived else "sessions/2026/10/31")
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / f"rollout-2026-10-31T10-00-00-{ident}.jsonl"
    source = "cli" if parent is None else {
        "subagent": {"thread_spawn": {"parent_thread_id": parent, "depth": 1}}}
    meta = {"id": ident, "timestamp": "2026-10-31T10:00:00.000Z",
            "cwd": str(case / "cwd" / project), "originator": "codex_cli_rs",
            "cli_version": "SYNTHETIC-current-probe", "source": source,
            "model_provider": "openai",
            "base_instructions": {"text": "Synthetic AgentMender fixture only."}}
    if attached:
        meta.update(attached_thread=True, forked_from_id=fork)
    elif fork:
        meta["forked_from_id"] = fork
    rows = [{"timestamp": "2026-10-31T10:00:00.000Z", "type": "session_meta", "payload": meta}]
    if preview:
        rows.append({"timestamp": "2026-10-31T10:00:01.000Z", "type": "event_msg",
                     "payload": {"type": "user_message", "message": "Synthetic; no model call.",
                                 "images": [], "local_images": [], "text_elements": []}})
    path.write_text("".join(json.dumps(row) + "\n" for row in rows))
    return path


class Server:
    def __init__(self, case, binary, real_home, denied=()):
        if digest(binary) != BINARY_SHA256:
            raise ValueError("exact binary digest changed")
        self.case, self.n, self.buf, self.total = case, 0, b"", 0
        self.err = (case / "native-stderr-private.log").open("wb")
        self.sel = selectors.DefaultSelector()
        self.proc = None
        env = {"HOME": str(case / "home"), "CODEX_HOME": str(case / "provider"),
               "TMPDIR": str(case / "tmp"), "PATH": "/usr/bin:/bin", "RUST_LOG": "error"}
        try:
            self.proc = subprocess.Popen(
                ["/usr/bin/sandbox-exec", "-p", policy(case, binary, real_home, denied),
                 str(binary), "app-server", "--listen", "stdio://"],
                env=env, cwd=case / "cwd", stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=self.err, bufsize=0)
            self.sel.register(self.proc.stdout, selectors.EVENT_READ)
            response = self.call("initialize", {"clientInfo": {"name": "agentmender-research", "version": "0.1.0"},
                                                 "capabilities": {"experimentalApi": True}})
            result = response.get("result", {})
            if Path(result.get("codexHome", "/missing")).resolve() != (case / "provider").resolve():
                raise ValueError("unexpected native state root")
            if f"/{VERSION} " not in result.get("userAgent", ""):
                raise ValueError("unexpected App Server version")
            self.send({"method": "initialized", "params": {}})
        except BaseException:
            self.close()
            raise

    def send(self, value):
        encoded = (json.dumps(value, separators=(",", ":")) + "\n").encode()
        if len(encoded) > 4096:
            raise ValueError("request budget")
        self.proc.stdin.write(encoded)
        self.proc.stdin.flush()

    def call(self, method, params):
        if method not in METHODS:
            raise ValueError("unreviewed research method")
        self.n += 1
        ident = self.n
        self.send({"id": ident, "method": method, "params": params})
        deadline = time.monotonic() + 20
        notifications = 0
        while time.monotonic() < deadline:
            if b"\n" in self.buf:
                line, self.buf = self.buf.split(b"\n", 1)
                if not line:
                    continue
                value = json.loads(line)
                if value.get("id") == ident:
                    return value
                if "id" in value:
                    raise ValueError("unexpected native request/response")
                notifications += 1
                if notifications > 256:
                    raise ValueError("notification budget")
                continue
            if not self.sel.select(max(0, deadline - time.monotonic())):
                break
            data = os.read(self.proc.stdout.fileno(), 65536)
            self.total += len(data)
            if self.total > MAX_OUTPUT or len(self.buf) + len(data) > 2 * 1024 * 1024:
                raise ValueError("response budget")
            if not data:
                raise ValueError("native process ended; outcome unknown")
            self.buf += data
        raise TimeoutError("native response deadline; outcome unknown")

    def close(self):
        if self.proc is not None:
            if self.proc.poll() is None:
                self.proc.terminate()
            try:
                self.proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=3)
            for stream in (self.proc.stdin, self.proc.stdout):
                if stream is not None:
                    stream.close()
        self.err.close()
        self.sel.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def read(server, ident, turns=False):
    return server.call("thread/read", {"threadId": ident, "includeTurns": turns})


def categorical(response):
    if "error" in response:
        error = response["error"]
        message = error.get("message", "")
        category = "thread_not_loaded" if message.startswith("thread not loaded:") else "provider_error"
        return {"error_code": error.get("code"), "category": category, "absence_proof": False}
    thread = response.get("result", {}).get("thread")
    return {"accepted": True, **({"canonical_id": thread["id"], "metadata_present": True}
                                 if thread else {})}


def listing(server, archived=False, **extra):
    ids, cursor, seen = [], None, set()
    for _ in range(40):
        params = {"limit": 2, "archived": archived, "useStateDbOnly": True,
                  "sourceKinds": ["cli", "vscode", "exec", "appServer", "subAgent",
                                  "subAgentReview", "subAgentCompact", "subAgentThreadSpawn",
                                  "subAgentOther", "unknown"], **extra}
        if cursor is not None:
            params["cursor"] = cursor
        response = server.call("thread/list", params)
        if "error" in response:
            raise ValueError("native list failed; no absence inference")
        ids.extend(row["id"] for row in response["result"]["data"])
        cursor = response["result"].get("nextCursor")
        if cursor is None:
            return {"ids": sorted(ids), "pagination_ended": True, "namespace_closed": False}
        if cursor in seen:
            raise ValueError("repeated cursor")
        seen.add(cursor)
    raise ValueError("page budget")


def snapshot(case):
    # Independent finite fixture measurement, never native deletion proof.
    return {p.relative_to(case / "provider").as_posix(): digest(p)
            for p in (case / "provider").rglob("*.jsonl")}


def states(files, ids):
    result = {}
    for ident in ids:
        matches = [p for p in files if ident in p]
        result[ident] = ("unobserved" if not matches else "ambiguous" if len(matches) != 1 else
                         "archived" if matches[0].startswith("archived_sessions/") else "active")
    return result


def run_archive(base, binary, home, name, mixed=False, deny_child=False, attached=False, single=False):
    case = new_case(base, name)
    paths = {R: seed(case, R), U: seed(case, U, project="unrelated-project")}
    if not single:
        paths.update({C: seed(case, C, R, archived=mixed, preview=False,
                              attached=attached, fork=R if attached else None, project="other-workspace"),
                      G: seed(case, G, C), S: seed(case, S, R)})
    if attached:
        paths[F] = seed(case, F, attached=True, fork=R)
    before = snapshot(case)
    with Server(case, binary, home, [paths[C]] if deny_child else ()) as server:
        for ident in paths:
            assert "result" in read(server, ident)
        ordinary = listing(server)
        ancestor = {str(a): listing(server, a, ancestorThreadId=R) for a in (False, True)}
        attached_list = listing(server, attachedToThreadId=R) if attached else None
        archive = categorical(server.call("thread/archive", {"threadId": R}))
        after = snapshot(case)
        unarchive = categorical(server.call("thread/unarchive", {"threadId": R}))
        restored = snapshot(case)
    assert archive.get("accepted") and unarchive.get("accepted")
    assert states(after, [R])[R] == "archived"
    if not single:
        assert states(after, [C])[C] == ("active" if deny_child else "archived")
        assert states(after, [G])[G] == "archived"
        assert C not in ordinary["ids"]
        ancestor_ids = set(ancestor["False"]["ids"]) | set(ancestor["True"]["ids"])
        assert C in (attached_list["ids"] if attached else ancestor_ids)
        assert states(restored, [C])[C] == "active"
    assert all(restored.get(p) == value for p, value in before.items() if U in p or F in p)
    return {"case": name, "before": states(before, paths), "ordinary": ordinary,
            "ancestor": ancestor, "attached": attached_list, "archive": archive,
            "after_archive": states(after, paths), "unarchive": unarchive,
            "after_unarchive": states(restored, paths), "retained_fixture_rollouts_unchanged": True,
            "complete_effect_or_retention_proof": False}


def run_delete(base, binary, home, name, deny_root=False):
    case = new_case(base, name)
    paths = {R: seed(case, R), C: seed(case, C, R, preview=False),
             G: seed(case, G, C, archived=True), U: seed(case, U, project="unrelated-project"),
             F: seed(case, F, fork=R)}
    with Server(case, binary, home, [paths[R]] if deny_root else ()) as server:
        for ident in paths:
            assert "result" in read(server, ident)
        before = snapshot(case)
        deleted = categorical(server.call("thread/delete", {"threadId": R}))
        after = snapshot(case)
        reads = {i: {str(t): categorical(read(server, i, t)) for t in (False, True)} for i in [R, C, G, M]}
    # Same disposable root, fresh process; these native reads may write private state.
    with Server(case, binary, home) as server:
        restart = {i: {str(t): categorical(read(server, i, t)) for t in (False, True)} for i in [C, G, M]}
    assert states(after, [C, G]) == {C: "unobserved", G: "unobserved"}
    assert all(after.get(p) == value for p, value in before.items() if U in p or F in p)
    if deny_root:
        assert deleted.get("error_code") == -32603
        assert reads[C]["False"].get("metadata_present")
        assert restart[C]["False"].get("metadata_present")
        assert reads[C]["True"]["category"] == reads[M]["True"]["category"] == "thread_not_loaded"
    else:
        assert deleted.get("accepted")
    return {"case": name, "delete": deleted, "before": states(before, paths),
            "after": states(after, paths), "reads": reads, "restart_reads": restart,
            "retained_fixture_rollouts_unchanged": True, "authoritative_absence_proof": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allow-isolated-native", action="store_true", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != "Darwin" or not args.binary.is_absolute():
        parser.error("macOS and an absolute exact official binary are required")
    import resource  # macOS-only native probe; help remains portable.
    import pwd
    binary = args.binary.resolve(strict=True)
    if not stat.S_ISREG(binary.stat().st_mode) or digest(binary) != BINARY_SHA256:
        parser.error("unvalidated binary; no provider process was started")
    # This standalone process stores private runtime output, never caller data.
    resource.setrlimit(resource.RLIMIT_FSIZE, (16 * 1024 * 1024, 16 * 1024 * 1024))
    # Ambient HOME is caller-controlled and cannot define the protected account.
    home = Path(pwd.getpwuid(os.getuid()).pw_dir).resolve(strict=True)
    base = Path(tempfile.mkdtemp(prefix="agentmender-m8-1-reproduce-", dir="/private/tmp")).resolve()
    assert stat.S_IMODE(base.stat().st_mode) == 0o700
    canaries = validate_isolation(base, binary, home)
    results = [run_archive(base, binary, home, "single", single=True),
               run_archive(base, binary, home, "hidden-tree"),
               run_archive(base, binary, home, "mixed-tree", mixed=True),
               run_archive(base, binary, home, "archive-denied-child", deny_child=True),
               run_archive(base, binary, home, "attached-tree", attached=True),
               run_delete(base, binary, home, "delete-mixed-tree"),
               run_delete(base, binary, home, "delete-denied-root", deny_root=True)]
    summary = {"version": VERSION, "binary_sha256": BINARY_SHA256,
               "classification": "native operations on explicitly synthetic fixtures",
               "production_compatibility_promoted": False, "isolation_canaries": canaries,
               "mach_lookup": "explicitly denied in policy; no separate canary", "cases": results}
    (base / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"passed_cases": len(results), "failed_cases": 0, "private_evidence_root": str(base)}))


if __name__ == "__main__":
    main()
