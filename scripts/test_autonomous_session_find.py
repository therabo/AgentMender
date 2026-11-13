#!/usr/bin/env python3
"""Exercise the real MCP server and Rust reader with a disposable App Server double."""

import argparse
import json
import os
from pathlib import Path
import resource
import subprocess
import sys
import tempfile
import time

from benchmark_progressive_resolution import Client, RssSampler, fixture

TERMINAL = {"completed", "interrupted", "cancelled"}
APP_SERVER = r'''#!/usr/bin/python3
import json
from pathlib import Path
import os
import sys
import time
import uuid

if "--version" in sys.argv:
    print("codex-cli 0.166.0")
    raise SystemExit()

if sys.argv[1:2] == ["delete"]:
    root = Path(os.environ["CODEX_HOME"])
    assert root == Path.cwd() and (root / "state_5.sqlite").exists(), root
    with (root.parent / "native-delete.calls").open("a") as log:
        print(json.dumps({"argv": sys.argv[1:], "root": str(root)}), file=log)
    raise SystemExit(json.loads((root / "auth.json").read_text())["delete_exit"])

def write(value):
    print(json.dumps(value), flush=True)

def notify(method, **params):
    write(dict(method=method, params=params))

thread = str(uuid.uuid4())
mode = json.loads(Path("auth.json").read_text()).get("fixture")
for line in sys.stdin:
    rpc = json.loads(line)
    method, params = rpc.get("method"), rpc.get("params")
    if method == "initialize":
        result = {}
    elif method == "thread/start":
        assert params["model"] == "gpt-6-luna"
        assert params["ephemeral"] and params["sandbox"] == "read-only"
        assert params["approvalPolicy"] == "never"
        assert params["allowProviderModelFallback"] is False
        model = "gpt-6-luna" if mode != "wrong_model" else "gpt-6.1-terra"
        effort = "medium" if mode != "wrong_effort" else "high"
        result = dict(thread=dict(id=thread), model=model, reasoningEffort=effort)
        notify("thread/started", thread=dict(id=thread))
    elif method == "turn/start":
        assert params["threadId"] == thread
        assert (params["model"], params["effort"]) == ("gpt-6-luna", "medium")
        turn = str(uuid.uuid4())
        request = json.loads(params["input"][0]["text"])
        write(dict(id=rpc["id"], result=dict(turn=dict(id=turn))))
        if mode == "hang":
            time.sleep(600)
        if mode == "reroute":
            notify("model/rerouted", threadId=thread, turnId=turn)
        elif mode == "quota":
            notify("turn/completed", threadId=thread, turn=dict(id=turn, status="failed",
                   error=dict(codexErrorInfo="usageLimitExceeded")))
        elif mode == "tool":
            notify("item/started", threadId=thread, turnId=turn, item=dict(type="commandExecution"))
        elif "search_intent_schema" in request:
            terms = ["NonexistentTerm"] if request["request"] == "zero" else ["FlightProbe"]
            intent = dict(schema_version="1", subject="FlightProbe", activity="implementation", hints=dict(terms=terms))
            if request["request"] == "best":
                intent["result_policy"] = "single_best"
            notify("item/completed", threadId=thread, turnId=turn,
                   item=dict(type="agentMessage", phase="final_answer", text=json.dumps(intent)))
            notify("turn/completed", threadId=thread, turn=dict(id=turn, status="completed"))
        else:
            if mode == "stale":
                time.sleep(0.75)
            if "choices" in request:
                data = {"native_session_id": request["choices"][0]["native_session_id"]}
            else:
                rows = []
                for candidate in request["candidates"]:
                    rows.append(dict(native_session_id=candidate["native_session_id"],
                         disposition="verified_relevant", outcomes=["implemented"],
                         evidence=[candidate["evidence"][0]["handle"]],
                         explanation="Fixture states that FlightProbe was implemented.", relationship="direct"))
                data = {"assessments": rows}
            notify("item/completed", threadId=thread, turnId=turn,
                   item=dict(type="agentMessage", phase="final_answer", text=json.dumps(data)))
            notify("turn/completed", threadId=thread, turn=dict(id=turn, status="completed"))
        continue
    elif method == "initialized":
        continue
    else:
        raise AssertionError(method)
    write(dict(id=rpc["id"], result=result))
'''


class FindClient(Client):
    def __init__(self, binary, root, caller, codex, *, protect=(), cwd=None, elicit=False):
        self.child = subprocess.Popen(
            [str(binary), "--provider", "codex", "--codex-root", str(root),
             "--codex-executable", str(codex),
             *[arg for identity in protect for arg in ("--protect-codex-session", identity)],
             "mcp", "--session-search"],
            cwd=cwd, text=True, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        )
        self.caller = caller
        self.serial = 0
        capabilities = {"elicitation": {"form": {}}} if elicit else {}
        self.info = self.exchange("initialize", {"protocolVersion": "2025-11-25",
                   "capabilities": capabilities, "clientInfo": {"name": "session-find-fixture", "version": "1"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def tool(self, name, caller=None, meta=True, **arguments):
        params = {"name": name, "arguments": arguments}
        if meta:
            params["_meta"] = {"threadId": caller or self.caller}
        response = self.exchange("tools/call", params)
        return response["structuredContent"]

    def terminal(self, job, caller=None):
        deadline = time.monotonic() + 120
        while job["state"] not in TERMINAL:
            assert time.monotonic() < deadline, job
            time.sleep(0.05)
            job = self.tool("session_find_status", caller, job_id=job["job_id"])
        return job

    def rows(self, job):
        groups = {"confirmed": [], "possible_unconfirmed": []}
        offset = 0
        while offset is not None:
            results = self.tool("session_find_result", job_id=job["job_id"], offset=offset)["results"]
            for group in groups:
                groups[group].extend(results[group])
            offset = results["next_offset"]
        return groups


def auth(root, mode=None, delete_exit=0):
    path = root / "auth.json"
    path.write_text(json.dumps({"fixture": mode, "delete_exit": delete_exit}))
    path.chmod(0o600)


def inventory(root):
    return {str(path.relative_to(root)): path.read_bytes() for path in root.rglob("*")
            if path.is_file()}


def active_children(parent):
    table = subprocess.check_output(["/bin/ps", "-axo", "pid=,ppid=,command="], text=True)
    processes = [line.split(None, 2) for line in table.splitlines()]
    return {int(pid) for pid, ppid, command in processes
            if int(ppid) == parent and ("sandbox-exec" in command or "resolve_with_luna" in command
                                       or "fake-codex" in command)}


def wait_for_luna(client, job, role):
    deadline = time.monotonic() + 10
    while not job["metrics"]["luna"].get(role, {}).get("thread"):
        assert time.monotonic() < deadline and job["state"] not in TERMINAL, job
        time.sleep(0.02)
        job = client.tool("session_find_status", job_id=job["job_id"])
    return job


def scale(binary, codex, root, count, protect=False):
    matches = 43 if count >= 105 else 2
    truth, caller = fixture(root, count, matched=matches)
    protected = [min(truth)] if protect else []
    auth(root)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    client = FindClient(binary, root, caller, codex, protect=protected, cwd="/")
    try:
        with RssSampler(client.child.pid) as rss:
            job = client.tool("session_find", query="Find every FlightProbe implementation.")
            # Read-only tools stay responsive while the job runs.
            assert client.exchange("tools/list", {})["tools"]
            # Leave the pipeline unattended through at least one MCP wait.
            job = client.tool("session_find_result", job_id=job["job_id"], wait_seconds=10)
            assert job["state"] == "completed" and job["results"] is not None, job
            assert job["metrics"]["mcp_tool_calls"] == 2, job
            rows = client.rows(job)
        assert job["state"] == "completed", job
        ids = [row["native_session_id"] for row in rows["confirmed"]]
        assert len(ids) == len(set(ids)) and set(ids) == truth - set(protected), ids
        assert not rows["possible_unconfirmed"]
        progress = job["progress"]
        assert progress["snapshot_sessions"] == progress["sources_inspected"] == count - len(protected)
        assert progress["candidates_staged"] == progress["candidates_evaluated"] == len(ids)
        roles = job["metrics"]["luna"]
        assert set(roles) == {"planner", "evaluator"} and roles["planner"]["thread"] != roles["evaluator"]["thread"]
        assert all((role["model"], role["effort"], role["closed"]) == ("gpt-6-luna", "medium", True)
                   for role in roles.values())
        assert roles["planner"]["turns"] == 1
        assert roles["evaluator"]["turns"] == (len(ids) + 4) // 5
        assert not active_children(client.child.pid)
        evidence = next(iter(rows["confirmed"]))
        context = client.tool("session_context", target=evidence["target"], evidence_ref=evidence["evidence_refs"][0])
        assert context.get("error") is None, context
        if protect:
            delete = client.tool("session_delete", native_session_id=evidence["native_session_id"],
                                 evidence_ref=evidence["evidence_refs"][0])
            assert delete == {"error": "confirmation_unavailable"}, delete
    finally:
        client.close()
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    print(json.dumps({"sources": progress["sources_inspected"], "candidates": len(ids),
                     "metrics": job["metrics"], "spool_peak_bytes": progress["provider_work"]["cohort_spool_bytes"],
                     "parent_rss_kib": rss.parent_kib, "child_rss_kib": rss.worker_kib,
                     "user_cpu_s": round(after.ru_utime - before.ru_utime, 3),
                     "system_cpu_s": round(after.ru_stime - before.ru_stime, 3)}), flush=True)


def delete_outcomes(client, root, codex, caller, row, selection):
    calls = root.parent / "native-delete.calls"

    def count():
        return len(calls.read_text().splitlines()) if calls.exists() else 0

    def request(action, *, content=None, missing=False):
        before = count()
        client.serial += 1
        client.send({"jsonrpc": "2.0", "id": client.serial, "method": "tools/call",
                     "params": {"name": "session_delete", "arguments": selection,
                                "_meta": {"threadId": caller}}})
        prompt = json.loads(client.child.stdout.readline())
        assert prompt["method"] == "elicitation/create", prompt
        assert prompt["params"]["mode"] == "form" and prompt["params"]["requestedSchema"]["properties"] == {}, prompt
        message = prompt["params"]["message"]
        assert row["title"] in message and row["native_session_id"] in message and "subagents" in message, prompt
        assert count() == before, "Codex ran before consent"
        parked = codex.with_suffix(".parked")
        if missing:
            codex.rename(parked)
        try:
            client.send({"jsonrpc": "2.0", "id": prompt["id"],
                         "result": {"action": action, "content": content}})
            reply = json.loads(client.child.stdout.readline())
            assert "method" not in reply and reply["id"] == client.serial, reply
            return reply["result"]
        finally:
            if missing:
                parked.rename(codex)

    before = inventory(root)
    for action in ("decline", "cancel"):
        assert request(action)["structuredContent"] == {"error": "cancelled"}
    assert request("accept", content={"confirm": True})["structuredContent"] == {"error": "confirmation_unavailable"}
    assert count() == 0 and inventory(root) == before

    for status, exit_code, missing, expected_count in (
        ("succeeded", 0, False, 1), ("failed", 9, False, 2), ("failed", 0, True, 2)
    ):
        auth(root, delete_exit=exit_code)
        before = inventory(root)
        reply = request("accept", content={}, missing=missing)
        assert reply["structuredContent"] == {"status": status, "native_session_id": row["native_session_id"]}, reply
        assert reply.get("isError", False) == (status == "failed"), reply
        assert count() == expected_count and inventory(root) == before
    for call in calls.read_text().splitlines():
        assert json.loads(call) == {"argv": ["delete", row["native_session_id"], "--force"], "root": str(root)}


def isolation(binary, codex, root):
    truth, caller = fixture(root, 6, matched=3)
    auth(root)
    client = FindClient(binary, root, caller, codex, cwd="/private/tmp", elicit=True)
    try:
        catalog = client.exchange("tools/list", {})
        schema = next(tool["inputSchema"] for tool in catalog["tools"] if tool["name"] == "session_find")
        assert set(schema["required"]) == {"query"} and schema["additionalProperties"] is False
        assert "session_find" in client.info["instructions"] and "session_planner" not in client.info["instructions"]
        assert client.tool("session_find", meta=False, query="zero")["error"] == "discovery_caller_unavailable"
        foreign = "99999999-9999-4999-9999-999999999999"
        missing = client.terminal(client.tool("session_find", foreign, query="zero"), foreign)
        assert missing["reason"] == "discovery_caller_unavailable" and not missing["metrics"]["luna"], missing
        first = client.tool("session_find", query="zero")
        again = client.tool("session_find", query="zero")
        assert first["job_id"] == again["job_id"]
        different = min(truth)
        other = client.tool("session_find", different, query="best")
        assert first["job_id"] != other["job_id"]
        for method in ("status", "result", "cancel"):
            assert client.tool("session_find_" + method, different, job_id=first["job_id"])["error"] == "job_unavailable"
        first = client.terminal(first)
        other = client.terminal(other, different)
        assert first["state"] == "completed" and set(first["metrics"]["luna"]) == {"planner"}, first
        assert other["state"] == "completed" and other["progress"]["candidates_evaluated"] == 3, other
        # This metadata can claim an existing row. It remains scoped read-only,
        # and the apparent caller is excluded from the published selection.
        published = client.tool("session_find_result", different, job_id=other["job_id"])["results"]["confirmed"]
        assert len(published) == 1 and published[0]["native_session_id"] != different
        assert other["authenticated_caller_binding"] is False

        delete = next(tool for tool in catalog["tools"] if tool["name"] == "session_delete")
        annotations = delete["annotations"]
        assert (annotations["readOnlyHint"], annotations["destructiveHint"],
                annotations["idempotentHint"]) == (False, True, False)
        assert set(delete["outputSchema"]["properties"]) == {"status", "native_session_id"}
        row = published[0]
        selection = {"native_session_id": row["native_session_id"], "evidence_ref": row["evidence_refs"][0]}
        before = inventory(root)
        wrong_caller = client.tool("session_delete", **selection)
        assert wrong_caller["error"] == "stale_cursor", wrong_caller
        caller_target = client.tool("session_delete", row["native_session_id"], **selection)
        assert caller_target["error"] == "protected_management_session", caller_target
        no_caller = client.tool("session_delete", meta=False, **selection)
        assert no_caller["error"] == "discovery_caller_unavailable", no_caller
        for args in (dict(selection, confirm=True), dict(selection, native_session_id=different),
                     dict(selection, native_session_id=[row["native_session_id"]]),
                     dict(selection, evidence_ref=selection["evidence_ref"] + "0")):
            client.serial += 1
            client.send({"jsonrpc": "2.0", "id": client.serial, "method": "tools/call",
                         "params": {"name": "session_delete", "arguments": args,
                                    "_meta": {"threadId": different}}})
            response = json.loads(client.child.stdout.readline())
            assert response["id"] == client.serial and response["error"]["code"] == -32602, response
        assert inventory(root) == before and not active_children(client.child.pid)
        delete_outcomes(client, root, codex, different, row, selection)
        print("Native selection, protected caller, single consent, exact argv/root and process outcomes passed.")
    finally:
        client.close()
    for mode, reason in (("wrong_model", "unexpected_model_or_effort"),
                         ("wrong_effort", "unexpected_model_or_effort"),
                         ("reroute", "model_rerouted"), ("quota", "usage_limit_exceeded"),
                         ("tool", "unexpected_tool"), ("hang", None)):
        auth(root, mode)
        client = FindClient(binary, root, caller, codex)
        try:
            job = client.tool("session_find", query=mode)
            if mode == "hang":
                job = wait_for_luna(client, job, "planner")
                active = client.tool("session_find_result", job_id=job["job_id"], wait_seconds=0)
                assert active["state"] == "planning" and active.get("results") is None
                assert len(active_children(client.child.pid)) >= 2
                job = client.tool("session_find_cancel", job_id=job["job_id"])
            result = client.terminal(job)
            if reason is None:
                assert result["state"] == "cancelled", result
            else:
                assert result["state"] == "interrupted" and result["reason"].endswith(reason), result
                assert result["metrics"]["resolver_operations"] == 0, result
            assert not active_children(client.child.pid)
        finally:
            client.close()

    client = FindClient(binary, root, caller, codex)
    wait_for_luna(client, client.tool("session_find", query="disconnect"), "planner")
    children = active_children(client.child.pid)
    assert len(children) >= 2
    client.close()
    for child in children:
        try:
            os.kill(child, 0)
        except ProcessLookupError:
            continue
        raise AssertionError(f"orphaned child {child}")

    auth(root)
    before = inventory(root)
    client = FindClient(binary, root, caller, root / "missing-codex")
    try:
        result = client.terminal(client.tool("session_find", query="missing executable"))
        assert result["reason"] == "codex_executable_unverified", result
    finally:
        client.close()
    assert inventory(root) == before
    print("Caller binding, duplicate starts, concurrency, model/effort, reroute, quota, tools, cancellation, EOF and executable refusal passed.")


def source_change(binary, codex, root):
    truth, caller = fixture(root, 8, matched=3)
    missing, stale, outside = sorted(truth)
    files = {identity: root / "sessions" / f"rollout-{identity}.jsonl" for identity in truth}
    escaped = root.parent / "outside-source.jsonl"
    files[outside].rename(escaped)
    files[outside].symlink_to(escaped)
    auth(root, "stale")
    client = FindClient(binary, root, caller, codex)
    try:
        start = client.tool("session_find", query="Find the FlightProbe implementation.")
        job = wait_for_luna(client, start, "evaluator")
        files[missing].unlink()
        files[stale].write_text(files[stale].read_text().replace("FlightProbe", "DifferentTopic"))
        result = client.terminal(job)
        assert result["progress"]["retrieval_complete"], result
        assert result["progress"]["freshness_failures"] or result["state"] == "interrupted", result
        assert not any(client.rows(result).values()), result
    finally:
        client.close()
    print("Missing, modified and out-of-scope sources were withheld from publication.")



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agentmender", type=Path, required=True)
    parser.add_argument("--scale", type=int, nargs="*", default=[105, 539, 1000, 10000])
    args = parser.parse_args()
    if sys.platform != "darwin":
        print("macOS isolation acceptance requires Darwin.")
        return
    binary = args.agentmender.resolve()
    with tempfile.TemporaryDirectory(dir="/private/tmp", prefix="agentmender-find-test-") as tmp:
        base = Path(tmp)
        codex = base / "fake-codex"
        codex.write_text(APP_SERVER)
        codex.chmod(0o700)
        isolation(binary, codex, base / "isolation")
        source_change(binary, codex, base / "stale")
        for count in args.scale:
            scale(binary, codex, base / f"provider-{count}", count)
        scale(binary, codex, base / "protected", 6, protect=True)


if __name__ == "__main__":
    main()
