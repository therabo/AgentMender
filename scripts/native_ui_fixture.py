"""Isolated archive seed for the native Codex UI acceptance contract."""
from contextlib import closing, contextmanager
import json
import os
from pathlib import Path
import selectors
import sqlite3
import subprocess
import sys
import time

MAX_OUTPUT = 8 * 1024 * 1024
METHODS = {
    "initialize", "thread/read", "project/import",
}
def session_id(n):
    return f"ab000000-1111-4111-8111-{n:012d}"


R, C, G, S, M, REPLACEMENT = map(session_id, range(1, 7))


def lit(path):
    value = str(path.resolve())
    if not value.isascii() or any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ValueError("unreviewed Seatbelt path encoding")
    return json.dumps(value)


def policy(case, binary, private_home):
    return ("(version 1)(allow default)"
            f"(deny file-write* (require-not (subpath {lit(case)})))"
            f"(deny file-read-data (require-all (require-any (subpath {lit(Path.home())}) (subpath {lit(private_home)})) "
            f"(require-not (literal {lit(binary)}))))"
            "(deny network*)(deny process-fork)(deny mach-lookup)")


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
    if set(results) != {"outside_write", "protected_directory_read", "network", "fork"} or any(
            value != "DENIED" for value in results.values()):
        raise ValueError(f"OS isolation not established; no provider process started: {results}")
    return results


def seed(case, ident, parent=None, archived=False):
    folder = case / "provider" / ("archived_sessions" if archived else "sessions/2026/10/31")
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / f"rollout-2026-10-31T10-00-00-{ident}.jsonl"
    source = "cli" if parent is None else {
        "subagent": {"thread_spawn": {"parent_thread_id": parent, "depth": 1}}}
    meta = {"id": ident, "timestamp": "2026-10-31T10:00:00.000Z",
            "cwd": str(case / "cwd" / "target-project"), "originator": "codex_cli_rs",
            "cli_version": "synthetic-ui-fixture", "source": source,
            "model_provider": "openai",
            "base_instructions": {"text": "Synthetic AgentMender fixture only."}}
    rows = [
        {"timestamp": "2026-10-31T10:00:00.000Z", "type": "session_meta", "payload": meta},
        {"timestamp": "2026-10-31T10:00:01.000Z", "type": "event_msg",
         "payload": {"type": "user_message", "message": "Synthetic fixture",
                     "images": [], "local_images": [], "text_elements": []}},
    ]
    path.write_text("".join(json.dumps(row) + "\n" for row in rows))
    return path


class FixtureServer:
    def __init__(self, case, binary, private_home):
        self.case, self.n, self.buf, self.total = case, 0, b"", 0
        self.err = (case / "native-stderr-private.log").open("wb")
        self.sel = selectors.DefaultSelector()
        self.proc = None
        env = {"HOME": str(case / "home"), "CODEX_HOME": str(case / "provider"),
               "TMPDIR": str(case / "tmp"), "PATH": "/usr/bin:/bin", "RUST_LOG": "error"}
        try:
            self.proc = subprocess.Popen(
                ["/usr/bin/sandbox-exec", "-p", policy(case, binary, private_home),
                 str(binary), "app-server", "--listen", "stdio://"],
                env=env, cwd=case / "cwd", stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=self.err, bufsize=0)
            self.sel.register(self.proc.stdout, selectors.EVENT_READ)
            response = self.call("initialize", {"clientInfo": {"name": "agentmender-ui-fixture", "version": "0.1.0"},
                                                 "capabilities": {"experimentalApi": True}})
            result = response.get("result", {})
            if Path(result.get("codexHome", "/missing")).resolve() != (case / "provider").resolve():
                raise ValueError("unexpected native state root")
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
            raise ValueError("unsupported fixture method")
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



@contextmanager
def native_fixture(base, binary):
    private_home = base / "private-canary"
    private_home.mkdir(mode=0o700)
    (private_home / "canary").write_text("fixture")
    validate_isolation(base, binary, private_home)
    case = new_case(base, "native")
    rollouts = {
        ident: seed(case, ident, parent=G if ident == C else None, archived=(ident != R))
        for ident in (R, G, C, S, M, REPLACEMENT)
    }
    with rollouts[R].open("a") as out:
        for text in ("Visible disposable message.", "password: secret-marker", "fixture text " * 1536):
            row = {"type": "response_item", "payload": {"type": "message", "role": "user",
                   "content": [{"type": "input_text", "text": text}]}}
            out.write(json.dumps(row) + "\n")
    with FixtureServer(case, binary, private_home) as server:
        for ident in rollouts:
            result = server.call("thread/read", {"threadId": ident, "includeTurns": False})
            assert "result" in result, result.get("error")
    root = case / "provider"
    # Keep the disposable WAL sidecars live for the read-only worker.
    with closing(sqlite3.connect(root / "state_5.sqlite")) as db:
        with db:
            db.execute("INSERT OR REPLACE INTO thread_spawn_edges VALUES(?,?,'closed')", (G, C))
        assert (root / "state_5.sqlite-wal").is_file() and (root / "state_5.sqlite-shm").is_file()
        yield root, rollouts
