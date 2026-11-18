#!/usr/bin/env python3
"""Run the 2.1.222 transcript writer in a disposable macOS fixture."""
import argparse
import fcntl
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import platform
import subprocess
from tempfile import TemporaryDirectory
from threading import Thread


SESSION = "eb000000-1111-4111-8111-000000000001"
MODEL = "claude-sonnet-4-6"


class LoopbackModel(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        self.rfile.read(int(self.headers.get("content-length", 0)))
        message = {
            "id": "msg_fixture", "type": "message", "role": "assistant",
            "model": MODEL, "content": [], "stop_reason": None,
            "usage": {"input_tokens": 5, "output_tokens": 1},
        }
        events = [
            ("message_start", {"message": message}),
            ("content_block_start", {"index": 0, "content_block": {"type": "text", "text": ""}}),
            ("content_block_delta", {"index": 0,
                                     "delta": {"type": "text_delta", "text": "Fixture acknowledgement."}}),
            ("content_block_stop", {"index": 0}),
            ("message_delta", {"delta": {"stop_reason": "end_turn", "stop_sequence": None},
                               "usage": {"output_tokens": 2}}),
            ("message_stop", {}),
        ]
        body = "".join(
            f"event: {kind}\ndata: {json.dumps({'type': kind, **event})}\n\n"
            for kind, event in events
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def sandbox_string(path):
    return json.dumps(str(path), ensure_ascii=False)


def experiment(engine):
    real_home = Path.home().resolve()
    with TemporaryDirectory(prefix="agentmender-claude-probe-") as directory:
        with ThreadingHTTPServer(("127.0.0.1", 0), LoopbackModel) as model:
            Thread(target=model.serve_forever, daemon=True).start()
            try:
                probe(engine, real_home, Path(directory).resolve(), model.server_port)
            finally:
                model.shutdown()


def probe(engine, real_home, root, port):
    home = root / "home"
    config = home / ".claude"
    workspace = root / "workspace"
    scratch = root / "scratch"
    for path in (config, workspace, scratch):
        path.mkdir(parents=True)
    profile = (
        "(version 1)(allow default)"
        f"(deny file-write* (require-not (subpath {sandbox_string(root)})))"
        "(deny network*)"
        f'(allow network-outbound (remote ip "localhost:{port}"))'
        f"(deny file-read* (subpath {sandbox_string(real_home / '.claude')})"
        f" (literal {sandbox_string(real_home / '.claude.json')}))"
    )
    env = {
        "HOME": str(home), "PATH": "/usr/bin:/bin", "TMPDIR": str(scratch),
        "CLAUDE_CONFIG_DIR": str(config), "CLAUDE_CODE_TMPDIR": str(scratch),
        "ANTHROPIC_BASE_URL": f"http://127.0.0.1:{port}",
        "ANTHROPIC_API_KEY": "fixture-only-loopback",
        "DISABLE_AUTOUPDATER": "1", "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
    }

    def run(*args):
        result = subprocess.run(
            ["/usr/bin/sandbox-exec", "-p", profile, str(engine), *args],
            cwd=workspace, env=env, capture_output=True, text=True, timeout=24,
        )
        assert result.returncode == 0, f"Engine exited with {result.returncode}"
        return result.stdout

    assert run("--version").strip() == "2.1.222 (Claude Code)"
    flags = ("-p", "--bare", "--max-turns", "1", "--model", MODEL)
    run(*flags, "--session-id", SESSION, "First fixture request.")
    transcript, = config.glob(f"projects/*/{SESSION}.jsonl")
    assert transcript.is_file() and not transcript.is_symlink()
    before = transcript.read_bytes()
    digest = hashlib.sha256(before).digest()
    with transcript.open("rb") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert hashlib.sha256(handle.read()).digest() == digest
        run(*flags, "--resume", SESSION, "Second fixture request during the advisory lock.")
        handle.seek(0)
        after = handle.read()
        assert after.startswith(before) and len(after) > len(before), (
            "Writer did not append under the advisory lock. Reassess this release's contract."
        )
        assert transcript.read_bytes() == after
    print(f"Claude 2.1.222 appended to its open transcript while flock was held ({len(before)} → {len(after)} B).")
    print("Fixture produced no history.jsonl." if not (config / "history.jsonl").exists()
          else "Fixture produced history.jsonl. The behavior needs separate inspection.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--claude", required=True, type=Path)
    options = parser.parse_args()
    assert platform.system() == "Darwin", "This fixture requires macOS Seatbelt."
    experiment(options.claude.resolve(strict=True))
