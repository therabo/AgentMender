#!/usr/bin/env python3
"""Run the progressive resolver from a host that controls Luna execution."""

import argparse
from contextlib import ExitStack, contextmanager
from datetime import datetime, timezone
import errno
import json
from pathlib import Path
import queue
import re
import subprocess
import sys
import tempfile
import threading
import time

from check_semantic_assessment import validate

MODEL = "gpt-6-luna"
EFFORT = "medium"
RECOVERABLE_EVALUATION_REQUEST = {"invalid_resolution_request", "unknown_field", "invalid_enum",
                                  "invalid_search", "invalid_advance", "budget_exceeded"}
MATCH_QUALITY = {"exact": 4, "normalized": 3, "partial": 2, "approximate": 1}
ROOT = Path(__file__).resolve().parents[1]


class ResolverError(Exception):
    pass


def public_code(code, fallback):
    return code if isinstance(code, str) and re.fullmatch("[a-z_]{1,96}", code) else fallback


class RpcRejected(ResolverError):
    def __init__(self, error):
        data = error.get("data")
        code = data.get("error") if isinstance(data, dict) else None
        super().__init__(public_code(code, "rpc_rejected"))
        number = error.get("code")
        self.number = number if type(number) is int else None
        # The free-form server message can contain request data. Classify it
        # locally and report only the category and JSON-RPC number.
        message = str(error.get("message", "")).lower()
        if "model" in message and any(word in message for word in
                                      ("unavailable", "not available", "not found", "not supported", "does not support")):
            self.category = "model_unavailable"
        elif any(words in message for words in ("permission denied", "not authorized", "forbidden")):
            self.category = "permission_denied"
        elif self.number == -32602:
            self.category = "invalid_request"
        elif self.number == -32601:
            self.category = "method_unavailable"
        else:
            self.category = "rpc_rejected"


class LunaFailure(ResolverError):
    def __init__(self, role, stage, reason, **details):
        super().__init__(f"luna_{role}_{stage}_{reason}")
        self.event = {"event": "luna_failure", "role": role, "stage": stage, "reason": reason, **details}

    @classmethod
    def from_error(cls, role, stage, error):
        if isinstance(error, RpcRejected):
            return cls(role, stage, error.category, rpc_code=error.number, server_code=str(error))
        if isinstance(error, OSError):
            return cls(role, stage, errno.errorcode.get(error.errno, "os_error").lower())
        if isinstance(error, ResolverError):
            return cls(role, stage, str(error))
        return cls(role, stage, "invalid_configuration" if stage == "role_profile" else "invalid_event")


class Rpc:
    """One private stdio connection with a bounded wait per operation."""

    def __init__(self, command, cwd=None):
        self.child = subprocess.Popen(
            command, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True,
        )
        self.inbox = queue.Queue(maxsize=128)
        self.serial = 0
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()

    def read(self):
        try:
            for line in self.child.stdout:
                self.inbox.put(json.loads(line))
        finally:
            self.inbox.put(None)

    def close(self):
        self.child.stdin.close()
        try:
            self.child.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.child.terminate()
            try:
                self.child.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.child.kill()
                self.child.wait()

    def send(self, payload):
        self.child.stdin.write(json.dumps({"jsonrpc": "2.0", **payload}) + "\n")
        self.child.stdin.flush()

    def receive(self, deadline):
        try:
            message = self.inbox.get(timeout=max(0, deadline - time.monotonic()))
        except queue.Empty:
            raise ResolverError("rpc_deadline") from None
        if message is None:
            raise ResolverError("rpc_closed")
        return message

    def call(self, method, params, observe=lambda _: None):
        self.serial += 1
        self.send({"id": self.serial, "method": method, "params": params})
        deadline = time.monotonic() + 90
        while True:
            message = self.receive(deadline)
            if message.get("id") == self.serial and "method" not in message:
                if "result" in message:
                    return message["result"]
                raise RpcRejected(message.get("error", {}))
            observe(message)


class Luna:
    def __init__(self, rpc, cwd, role, trace=lambda **_: None):
        self.rpc = rpc
        self.role = role
        self.turns = 0
        self.usage = {}
        self.trace = trace
        started = set()

        def observe_start(message):
            if message.get("method") == "thread/started":
                started.add(message.get("params", {}).get("thread", {}).get("id"))

        stage = "initialize"
        try:
            rpc.call("initialize", {"clientInfo": {"name": "agentmender-luna-host", "version": "1"}})
            rpc.send({"method": "initialized"})
            stage = "role_profile"
            profile = (ROOT / f".codex/agents/session-{role}.toml").read_text(encoding="utf-8")
            # Embedded profiles use one TOML multiline basic string without escapes.
            # The fixed delimiter also supports the macOS system Python 3.9.
            _, body = profile.split('developer_instructions = """', 1)
            instructions, _ = body.split('"""', 1)
            instructions = instructions.removeprefix("\n")
            stage = "thread_start"
            thread = rpc.call("thread/start", {
                "cwd": cwd, "ephemeral": True, "model": MODEL, "sandbox": "read-only",
                "approvalPolicy": "never", "allowProviderModelFallback": False,
                "developerInstructions": instructions,
            }, observe_start)
            model, effort = thread.get("model"), thread.get("reasoningEffort")
            if (model, effort) != (MODEL, EFFORT):
                raise LunaFailure(role, stage, "unexpected_model_or_effort", model=model, effort=effort)
            self.thread = thread["thread"]["id"]
            stage = "thread_started_event"
            deadline = time.monotonic() + 10
            while self.thread not in started:
                observe_start(rpc.receive(deadline))
            if started != {self.thread}:
                raise LunaFailure(role, stage, "unexpected_thread")
        except LunaFailure:
            raise
        except (ResolverError, OSError, KeyError, TypeError, ValueError) as error:
            raise LunaFailure.from_error(role, stage, error) from None
        self.trace(event="luna_ready", role=role, thread=self.thread, model=model, effort=effort)

    def turn(self, payload):
        messages = {}
        completed = {}
        reroutes = set()
        unexpected_threads = set()
        unexpected_tools = set()

        def observe(event):
            method = event.get("method")
            params = event.get("params", {})
            if "id" in event and method:
                self.rpc.send({"id": event["id"], "error": {"code": -32601, "message": "Unsupported"}})
                return
            if method == "thread/started":
                identity = params.get("thread", {}).get("id")
                if identity != self.thread:
                    unexpected_threads.add(identity)
            if params.get("threadId") != self.thread:
                return
            turn_id = params.get("turnId")
            if method == "thread/tokenUsage/updated":
                self.usage = params.get("tokenUsage", {}).get("total", self.usage)
            if method == "model/rerouted":
                reroutes.add(turn_id)
            if method in ("item/started", "item/completed"):
                kind = params["item"]["type"]
                if kind == "collabAgentToolCall":
                    unexpected_threads.add(self.thread)
                elif kind not in ("userMessage", "agentMessage", "reasoning", "contextCompaction", "plan"):
                    unexpected_tools.add(kind)
            if method == "item/completed" and params["item"]["type"] == "agentMessage":
                item = params["item"]
                if item.get("phase") in (None, "final_answer"):
                    messages[turn_id] = item["text"]
            if method == "turn/completed":
                turn = params["turn"]
                completed[turn["id"]] = turn

        stage = "turn_start"
        try:
            started = self.rpc.call("turn/start", {
                "threadId": self.thread, "model": MODEL, "effort": EFFORT,
                "input": [{"type": "text", "text": json.dumps(payload, ensure_ascii=False)}],
            }, observe)
            turn_id = started["turn"]["id"]
            stage = "turn_completed_event"
            deadline = time.monotonic() + 120
            while turn_id not in completed and turn_id not in reroutes and not unexpected_threads and not unexpected_tools:
                observe(self.rpc.receive(deadline))
        except (ResolverError, OSError, KeyError, TypeError) as error:
            raise LunaFailure.from_error(self.role, stage, error) from None
        if unexpected_threads:
            raise LunaFailure(self.role, stage, "unexpected_thread")
        if unexpected_tools:
            raise LunaFailure(self.role, stage, "unexpected_tool")
        if turn_id in reroutes:
            raise LunaFailure(self.role, stage, "model_rerouted")
        turn = completed[turn_id]
        if turn["status"] != "completed":
            detail = (turn.get("error") or {}).get("codexErrorInfo")
            if isinstance(detail, dict):
                detail = next(iter(detail), None)
            info = detail if isinstance(detail, str) and re.fullmatch(r"[A-Za-z_]{1,64}", detail) else None
            reason = ("usage_limit_exceeded" if info == "usageLimitExceeded"
                      else public_code(turn["status"], "invalid_status"))
            raise LunaFailure(self.role, stage, reason, codex_error_info=info)
        self.turns += 1
        self.trace(event="luna_turn_completed", role=self.role, thread=self.thread, turn=turn_id,
                   model=MODEL, effort=EFFORT, token_usage=self.usage)
        if turn_id not in messages:
            raise LunaFailure(self.role, stage, "missing_final_message")
        try:
            result = json.loads(messages[turn_id])
            # Reject values that cannot be forwarded as standard UTF-8 JSON.
            json.dumps(result, allow_nan=False, ensure_ascii=False).encode("utf-8")
        except (TypeError, ValueError):
            raise ResolverError("invalid_luna_response") from None
        if not isinstance(result, dict):
            raise ResolverError("invalid_luna_response")
        return result


class Mcp:
    def __init__(self, rpc, caller):
        self.rpc = rpc
        self.caller = caller
        rpc.call("initialize", {"protocolVersion": "2025-11-25", "capabilities": {},
                                "clientInfo": {"name": "agentmender-luna-host", "version": "1"}})
        rpc.send({"method": "notifications/initialized"})

    def resolve(self, **arguments):
        try:
            response = self.rpc.call("tools/call", {
                "name": "session_resolve", "arguments": arguments, "_meta": {"threadId": self.caller},
            })
        except RpcRejected as error:
            raise ResolverError(str(error)) from None
        result = response.get("structuredContent", {})
        if response.get("isError") or not result or "error" in result:
            raise ResolverError(public_code(result.get("error"), "resolver_refused"))
        return result


def passages(citations):
    return [{"handle": item["handle"], "excerpt": item.get("excerpt")} for item in citations]


def plan(planner, question, trace, validate_intent=lambda _: None, date_context=None):
    schema = json.loads((ROOT / "docs/search-intent-v1.schema.json").read_text())
    correction = None
    for _ in range(3):
        try:
            intent = planner.turn({
                "task": "Return one SearchIntent JSON object conforming to the schema.",
                "request": question, "date_context": date_context,
                "search_intent_schema": schema, "correction": correction,
            })
            validate(schema, intent, schema)
            validate_intent(intent)
            return intent
        except ValueError as error:
            correction = str(error)
        except ResolverError as error:
            if str(error) != "invalid_luna_response":
                raise
            correction = "The response was not a SearchIntent JSON object."
        trace(event="planner_payload_rejected", reason=correction, thread=planner.thread)
    raise ResolverError("invalid_search_intent")


def resolve(spawn, mcp, question, trace=lambda **_: None, *, validate_intent=lambda _: None,
            host_managed=False, date_context=None):
    started = time.monotonic()
    with spawn("planner") as planner:
        planner_id = planner.thread
        intent = plan(planner, question, trace, validate_intent, date_context)
    trace(event="planner_complete", elapsed_ms=elapsed_ms(started))

    tick = time.monotonic()
    page = mcp.resolve(action="start", search_intent=intent)
    run = page["run"]
    operation = 0
    results = {"matches": {}, "alternatives": {}, "excluded": {}}
    choices = []
    winner = None
    selection_ambiguous = False

    def advance(**changes):
        nonlocal operation, page
        operation += 1
        if not host_managed:
            changes.update(run=run, expected_revision=page["step_revision"], operation_id=str(operation))
        page = mcp.resolve(action="advance", **changes)
        return page

    while page["phase"] == "retrieving":
        if page["candidates"]:
            raise ResolverError("candidates_before_retrieval_complete")
        advance()
    trace(event="retrieval_finished", elapsed_ms=elapsed_ms(tick), progress=page["progress"])
    if page["phase"] == "evaluating":
        if not page["progress"]["retrieval_complete"]:
            raise ResolverError("retrieval_incomplete")
        tick = time.monotonic()
        interruption = None
        try:
            with ExitStack() as evaluation:
                evaluator = None

                def get_evaluator():
                    nonlocal evaluator
                    if evaluator is None:
                        evaluator = evaluation.enter_context(spawn("evaluator"))
                        if evaluator.thread == planner_id:
                            raise LunaFailure("evaluator", "thread_start", "planner_thread_reused")
                    return evaluator

                while page["phase"] == "evaluating" and not page["progress"]["publication_ready"]:
                    advance()
                    choices.extend(page.get("prepared_choices", [])[:max(0, 17 - len(choices))])
                    if page["candidates"]:
                        evaluate(get_evaluator(), question, intent, page, advance, trace)
                        choices.extend(page.get("prepared_choices", [])[:max(0, 17 - len(choices))])

                if (page["phase"] == "evaluating" and page["progress"]["publication_ready"]
                        and intent.get("result_policy") == "single_best" and choices):
                    if page["progress"].get("evaluation_complete", True):
                        if len(choices) == 1:
                            winner = choices[0]["native_session_id"]
                        elif any(row.get("match_basis") == "semantic_evidence" for row in choices):
                            winner = select_best(get_evaluator(), question, choices)
                        elif len(choices) < 17:
                            ranked = sorted(choices, key=lexical_quality, reverse=True)
                            if lexical_quality(ranked[0]) > lexical_quality(ranked[1]):
                                winner = ranked[0]["native_session_id"]
                    selection_ambiguous = winner is None and len(choices) > 1
        except LunaFailure as error:
            trace(**error.event)
            interruption = "luna_unavailable"
        except ResolverError as error:
            if str(error) == "invalid_luna_response":
                interruption = "invalid_luna_response"
            elif str(error) in RECOVERABLE_EVALUATION_REQUEST:
                interruption = "invalid_resolution_request"
            else:
                raise
        if interruption is not None and page["phase"] == "evaluating":
            winner = None
            advance(interrupt_evaluation=interruption)
            while page["phase"] == "evaluating" and not page["progress"]["publication_ready"]:
                advance()
            selection_ambiguous = intent.get("result_policy") == "single_best" and page["progress"]["confirmed"] > 1
        trace(event="evaluation_finished", elapsed_ms=elapsed_ms(tick), progress=page["progress"])

        tick = time.monotonic()
        while page["execution_state"] == "in_progress":
            if page["phase"] != "publishing" and not page["progress"]["publication_ready"]:
                raise ResolverError("evaluation_incomplete")
            advance(publish=True, **({"winner": winner} if host_managed and winner else {}))
            if not host_managed:
                for group, retained in results.items():
                    for identity in page.get("invalidated", []):
                        retained.pop(identity, None)
                    for row in page.get(group, []):
                        retained[row["native_session_id"]] = row
        trace(event="publication_finished", elapsed_ms=elapsed_ms(tick))

    status = []
    if page["execution_state"] == "interrupted":
        phase = "Scansione lessicale interrotta" if not page["progress"]["retrieval_complete"] else "Ricerca interrotta"
        status.append(f"{phase} ({page['progress']['stop_reason']}).")
    if page["progressive_coverage"]["lexical_coverage_limited"]:
        status.append("Copertura lessicale limitata alla cronologia disponibile.")
    if selection_ambiguous:
        status.append("La ricerca ha lasciato più risultati a pari merito.")
    trace(event="resolver_terminal", state=page["execution_state"], mcp_calls=operation + 1,
          elapsed_ms=elapsed_ms(started), progress=page["progress"])
    if host_managed:
        return {"state": page["execution_state"], "selection_ambiguous": selection_ambiguous,
                "status": " ".join(status) or None, "winner": winner}

    def ordered(group):
        return sorted(results[group].values(), key=lambda row: (
            -lexical_quality(row), row["native_session_id"]))

    matches = ordered("matches")
    if winner is not None:
        matches = [row for row in matches if row["native_session_id"] == winner]
        if not matches:
            status.append("La fonte della sessione scelta è cambiata prima della pubblicazione.")
    return matches, ordered("alternatives"), " ".join(status) or None, ordered("excluded")


def lexical_quality(row):
    return MATCH_QUALITY.get(row.get("lexical_match"), 0)


def elapsed_ms(started):
    return round((time.monotonic() - started) * 1000, 3)


def evaluate(evaluator, question, intent, page, advance, trace):
    batch = page["candidates"]
    packet = [{"native_session_id": row["native_session_id"], "title": row.get("title"),
               "evidence": passages(row["evidence"])} for row in batch]
    context = []
    windows_remaining = 2
    while True:
        changes = evaluator.turn({
            "task": ("Assess the supplied candidates with the relevance contract. "
                     "If needed, request bounded context before giving judgments."),
            "request": question, "search_intent": intent, "candidates": packet, "context": context,
            "coverage": page["progressive_coverage"], "context_windows_remaining": windows_remaining,
        })
        if set(changes) == {"assessments"} and isinstance(changes["assessments"], list):
            advance(assessments=changes["assessments"])
            trace(event="luna_assessment", candidates=len(batch), thread=evaluator.thread)
            return
        requests = changes.get("context_requests")
        if set(changes) != {"context_requests"} or not isinstance(requests, list) or not (
            1 <= len(requests) <= windows_remaining
        ):
            if set(changes) == {"context_requests"}:
                raise ResolverError("invalid_resolution_request")
            raise ResolverError("invalid_luna_response")
        page = advance(context_requests=requests)
        if page["phase"] != "evaluating":
            return
        windows_remaining -= len(requests)
        context.extend({"messages": passages(window["messages"]),
                        "next": window["next"], "previous": window["previous"]}
                       for window in page["windows"])
        context.extend(page.get("context_failures", []))


def select_best(luna, question, matches):
    # A large set stays ambiguous rather than requiring a tournament of turns.
    if len(matches) > 16:
        return None
    try:
        comparison = luna.turn({
            "task": "Choose the uniquely better confirmed match. Return native_session_id (null if tied).",
            "request": question,
            "choices": [{"native_session_id": row["native_session_id"],
                         "explanation": row["explanation"]} for row in matches],
        })
    except ResolverError as error:
        if str(error) != "invalid_luna_response":
            raise
        return None
    identity = comparison.get("native_session_id")
    if set(comparison) == {"native_session_id"} and any(identity == row["native_session_id"] for row in matches):
        return identity
    return None


def render(matches, alternatives, status, excluded=()):
    def cell(value):
        value = " ".join((value or "").split())
        return re.sub(r"[&<>|\\`*_!\[\]]", lambda match: f"&#{ord(match[0])};", value)

    def rows(items, empty, reason):
        if not items:
            return [f"| {empty} | | |"]
        labels = {"verified_metadata": "Metadati verificati", "verified_lexical": "Riscontro lessicale",
                  "semantic_evidence": "Evidenza semantica",
                  "plausible_evidence": "Alternativa plausibile", "retrieved_unverified": "Non verificata"}
        return [f"| {cell(item.get('title') or item['native_session_id'])} | "
                f"{cell(labels.get(item.get('match_basis'), ''))} · "
                f"{cell(reason(item['assessment']))} | `{item['native_session_id']}` |" for item in items]

    return "\n".join([
        "**Sessioni confermate**", "",
        "| Sessione | Motivazione | ID |", "|---|---|---|",
        *rows(matches, "Nessuna sessione confermata", lambda item: item["explanation"]),
        "", "**Possibili candidate non confermate**", "",
        "| Sessione | Evidenza mancante | ID |", "|---|---|---|",
        *rows(alternatives, "Nessuna candidata plausibile",
              lambda item: item.get("missing_evidence") or item["explanation"]),
        *([f"| Stato della ricerca | {cell(status)} | |"] if status else []),
        *(["", "**Sessioni escluse**", "", "| Sessione | Motivazione | ID |", "|---|---|---|",
           *[f"| {cell(row.get('title') or row['native_session_id'])} | "
             f"{cell(row['explanation'])} | `{row['native_session_id']}` |" for row in excluded]]
          if excluded else []),
    ])


def child_usage():
    if sys.platform == "win32":
        return None
    import resource
    return resource.getrusage(resource.RUSAGE_CHILDREN)


class PrivateBridge:
    """One stdio link to the parent. The Rust host owns processes and the resolver."""

    def call(self, operation, **fields):
        print(json.dumps({"operation": operation, **fields}), flush=True)
        line = sys.stdin.buffer.readline(1024 * 1024 + 1)
        if not line or len(line) > 1024 * 1024:
            raise ResolverError("bridge_closed")
        reply = json.loads(line)
        if "error" in reply:
            raise ResolverError(public_code(reply["error"], "bridge_failed"))
        return reply["value"]

    def resolve(self, **arguments):
        return self.call("resolve", arguments=arguments)

    def validate_intent(self, intent):
        try:
            self.call("validate", intent=intent)
        except ResolverError as error:
            raise ValueError(str(error)) from None

    def trace(self, **event):
        self.call("trace", event=event)

class RelayedRpc(Rpc):
    def __init__(self, bridge):
        self.bridge = bridge
        self.serial = 0

    def send(self, payload):
        self.bridge.call("send", message={"jsonrpc": "2.0", **payload})

    def receive(self, deadline):
        return self.bridge.call("receive", wait_ms=max(0, round((deadline - time.monotonic()) * 1000)))


def internal_coordinator():
    bridge = PrivateBridge()
    question = bridge.call("query")
    date_context = bridge.call("date_context")

    @contextmanager
    def spawn(role):
        bridge.trace(event="luna_spawn_attempt", role=role, model=MODEL, effort=EFFORT)
        try:
            cwd = bridge.call("spawn", role=role)
        except ResolverError as error:
            raise LunaFailure.from_error(role, "app_server_launch", error) from None
        try:
            luna = Luna(RelayedRpc(bridge), cwd, role, bridge.trace)
            yield luna
        except BaseException:
            # Preserve the stage-specific failure even if the App Server
            # exited before it could process a graceful stdin closure.
            try:
                bridge.call("close")
            except ResolverError:
                pass
            raise
        else:
            try:
                bridge.call("close")
            except ResolverError as error:
                raise LunaFailure.from_error(role, "close", error) from None
            bridge.trace(event="luna_closed", role=role, thread=luna.thread,
                         turns=luna.turns, token_usage=luna.usage)

    try:
        result = resolve(spawn, bridge, question, bridge.trace,
                         validate_intent=bridge.validate_intent, host_managed=True, date_context=date_context)
        bridge.call("done", result=result)
    except (ResolverError, OSError, KeyError, TypeError, ValueError) as error:
        if isinstance(error, LunaFailure):
            bridge.trace(**error.event)
        bridge.call("failed", reason=public_code(str(error), "host_failed"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agentmender", type=Path, required=True)
    parser.add_argument("--codex", default="codex")
    parser.add_argument("--codex-root", type=Path, required=True)
    parser.add_argument("--caller", required=True, help="Apparent originating chat UUID from the host")
    parser.add_argument("--trace", action="store_true", help="Print sanitized execution events to stderr")
    parser.add_argument("question")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}", args.caller):
        parser.error("--caller requires a canonical chat UUID")
    def trace(**event):
        if args.trace:
            print(json.dumps(event), file=sys.stderr, flush=True)

    before = child_usage()
    try:
        with ExitStack() as stack:
            cwd = stack.enter_context(tempfile.TemporaryDirectory(prefix="agentmender-luna-"))
            command = [args.codex, "app-server", "-c", 'model_reasoning_effort="medium"',
                       "-c", "mcp_servers={}", "-c", "mcp_servers.agentmender.enabled=false",
                       "-c", "features.shell_tool=false", "-c", "features.apps=false",
                       "-c", "features.plugins=false", "-c", "features.memories=false",
                       "-c", "features.multi_agent=false", "-c", "features.multi_agent_v2=false"]

            @contextmanager
            def spawn(role):
                trace(event="luna_spawn_attempt", route="explicit_app_server", role=role,
                      model=MODEL, effort=EFFORT)
                try:
                    codex = Rpc(command, cwd=cwd)
                except OSError as error:
                    raise LunaFailure.from_error(role, "app_server_launch", error) from None
                luna = None
                try:
                    luna = Luna(codex, cwd, role, trace)
                    yield luna
                finally:
                    codex.close()
                    if luna:
                        trace(event="luna_closed", role=role, thread=luna.thread,
                              turns=luna.turns, token_usage=luna.usage)

            mcp = Rpc([str(args.agentmender.resolve()), "--provider", "codex", "--codex-root",
                       str(args.codex_root.resolve()), "mcp", "--session-search"])
            stack.callback(mcp.close)
            print(render(*resolve(spawn, Mcp(mcp, args.caller), args.question, trace,
                                  date_context={"now_utc": datetime.now(timezone.utc).isoformat()})))
        after = child_usage()
        if before is not None and after is not None:
            trace(event="host_resources", child_cpu_user_s=after.ru_utime - before.ru_utime,
                  child_cpu_system_s=after.ru_stime - before.ru_stime, child_maxrss=after.ru_maxrss,
                  rss_unit="bytes" if sys.platform == "darwin" else "KiB")
    except (ResolverError, OSError, KeyError, TypeError, ValueError) as error:
        if isinstance(error, LunaFailure):
            trace(**error.event)
        code = str(error) if isinstance(error, ResolverError) else "host_failed"
        print(render([], [], f"Ricerca interrotta ({code})."))
        return 1
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--internal-coordinator"]:
        internal_coordinator()
    else:
        raise SystemExit(main())
