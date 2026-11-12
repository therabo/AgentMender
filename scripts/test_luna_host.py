#!/usr/bin/env python3
"""Check host stage ordering and verify no model replacement is admitted."""

from contextlib import contextmanager
import unittest

from resolve_with_luna import Luna, LunaFailure, ResolverError, RpcRejected, render, resolve, select_best


INTENT = {"schema_version": "1", "subject": "Test", "hints": {"terms": ["test"]}}


class FakeRpc:
    def __init__(self, model="gpt-6-luna", effort="medium", reroute=False, nested=False, usage_limit=False):
        self.model = model
        self.effort = effort
        self.reroute = reroute
        self.nested = nested
        self.usage_limit = usage_limit

    def call(self, method, params, observe=None):
        if method == "thread/start":
            observe({"method": "thread/started", "params": {"thread": {"id": "test"}}})
            return {"model": self.model, "reasoningEffort": self.effort, "thread": {"id": "test"}}
        if method == "turn/start":
            assert params["model"] == "gpt-6-luna" and params["effort"] == "medium"
            if self.reroute:
                observe({"method": "model/rerouted", "params": {"threadId": "test", "turnId": "turn"}})
            if self.nested:
                observe({"method": "item/started", "params": {"threadId": "test", "turnId": "turn",
                         "item": {"type": "collabAgentToolCall"}}})
            if self.usage_limit:
                observe({"method": "turn/completed", "params": {"threadId": "test", "turn": {
                    "id": "turn", "status": "failed", "error": {"codexErrorInfo": "usageLimitExceeded"},
                }}})
            return {"turn": {"id": "turn"}}
        return {}

    def send(self, payload):
        pass


class Scenario:
    def __init__(self, count=7, evaluator_error=False, reject_once=False, invalid_plan_once=False,
                 invalid_context_once=False):
        self.active = None
        self.count = count
        self.evaluator_error = evaluator_error
        self.spawned = []
        self.closed = []
        self.submitted = []
        self.steps = []
        self.revision = 0
        self.reject_once = reject_once
        self.correction = None
        self.invalid_plan_once = invalid_plan_once
        self.invalid_context_once = invalid_context_once
        self.planner_turns = 0

    @contextmanager
    def spawn(self, role):
        assert self.active is None
        self.spawned.append(role)
        self.active = role

        def turn(payload):
            if role == "planner":
                self.planner_turns += 1
                if self.invalid_plan_once and self.planner_turns == 1:
                    return {**INTENT, "hints": {"terms": ["test"], "unsupported": True}}
                return INTENT
            if self.evaluator_error:
                raise LunaFailure("evaluator", "thread_start", "rpc_rejected")
            assert payload["search_intent"] is INTENT
            self.correction = payload["correction"]
            if self.invalid_context_once:
                self.invalid_context_once = False
                return {"context_requests": [{"anchor": 99}]}
            return {"assessments": [
                {"native_session_id": row["native_session_id"]} for row in payload["candidates"]
            ]}

        try:
            yield type("Thread", (), {"thread": role, "turn": staticmethod(turn)})()
        finally:
            self.closed.append(role)
            self.active = None

    def resolve(self, **arguments):
        self.steps.append(arguments)
        self.revision += 1

        def page(phase, candidates=None, ready=False):
            return {
                "run": "test", "step_revision": self.revision, "phase": phase,
                "execution_state": phase if phase in ("completed", "interrupted") else "in_progress",
                "progress": {"retrieval_complete": phase != "retrieving", "publication_ready": ready},
                "progressive_coverage": {"lexical_coverage_limited": False},
                "candidates": candidates or [], "matches": [], "alternatives": [],
            }

        if arguments["action"] == "start":
            assert self.closed == ["planner"] and self.active is None
            assert arguments["search_intent"] is INTENT
            return page("retrieving")
        if self.revision <= 3:
            assert self.active is None and self.spawned == ["planner"]
            assert not {"assessments", "publish"} & arguments.keys()
            if self.revision == 2:
                return page("retrieving")
            return page("evaluating" if self.count else "completed")
        if arguments.get("luna_unavailable"):
            assert self.closed == ["planner", "evaluator"]
            assert "assessments" not in arguments
            return page("interrupted")
        if arguments.get("publish"):
            assert self.closed == ["planner", "evaluator"]
            assert self.submitted == [str(i) for i in range(self.count)]
            return page("completed")
        assert self.active == "evaluator"
        if "context_requests" in arguments:
            self.revision -= 1
            raise ResolverError("invalid_scope")
        if "assessments" in arguments:
            if self.reject_once:
                self.reject_once = False
                self.revision -= 1
                raise ResolverError("invalid_enum")
            self.submitted.extend(row["native_session_id"] for row in arguments["assessments"])
            return page("evaluating", ready=len(self.submitted) == self.count)
        return page("evaluating", [
            {"native_session_id": str(identity), "evidence": []}
            for identity in range(len(self.submitted), min(len(self.submitted) + 5, self.count))
        ])


class LunaHostTests(unittest.TestCase):
    def test_rejects_wrong_model_or_effort(self):
        for rpc in (FakeRpc(model="gpt-6-sol"), FakeRpc(effort="low")):
            with self.assertRaisesRegex(LunaFailure, "planner_thread_start_unexpected_model_or_effort") as caught:
                Luna(rpc, ".", "planner")
            self.assertEqual(caught.exception.event["model"], rpc.model)
            self.assertEqual(caught.exception.event["effort"], rpc.effort)

    def test_rejects_turn_reroute(self):
        luna = Luna(FakeRpc(reroute=True), ".", "planner")
        with self.assertRaisesRegex(LunaFailure, "planner_turn_completed_event_model_rerouted"):
            luna.turn({"task": "interpret"})

    def test_rejects_nested_agent_execution(self):
        luna = Luna(FakeRpc(nested=True), ".", "evaluator")
        with self.assertRaisesRegex(LunaFailure, "evaluator_turn_completed_event_unexpected_thread"):
            luna.turn({"task": "assess"})

    def test_planner_failure_never_calls_mcp(self):
        @contextmanager
        def spawn(role):
            self.assertEqual(role, "planner")
            yield Luna(FakeRpc(reroute=True), ".", role)

        with self.assertRaisesRegex(LunaFailure, "planner_turn_completed_event_model_rerouted"):
            resolve(spawn, object(), "question")

    def test_records_model_rejection_without_copying_provider_message(self):
        error = RpcRejected({"code": -32000, "message": "Model is not available for private account"})
        failure = LunaFailure.from_error("planner", "thread_start", error)
        self.assertEqual(str(failure), "luna_planner_thread_start_model_unavailable")
        self.assertEqual(failure.event["rpc_code"], -32000)
        self.assertNotIn("private account", str(failure.event))

    def test_reports_usage_limit_from_completed_turn(self):
        luna = Luna(FakeRpc(usage_limit=True), ".", "evaluator")
        with self.assertRaisesRegex(LunaFailure, "evaluator_turn_completed_event_usage_limit_exceeded") as caught:
            luna.turn({"task": "assess"})
        self.assertEqual(caught.exception.event["codex_error_info"], "usageLimitExceeded")

    def test_two_stages_for_all_43_candidates(self):
        scenario = Scenario(43)
        resolve(scenario.spawn, scenario, "question")
        self.assertEqual(scenario.spawned, ["planner", "evaluator"])
        self.assertEqual(scenario.closed, scenario.spawned)
        self.assertEqual(scenario.submitted, [str(i) for i in range(43)])
        self.assertEqual(sum("assessments" in step for step in scenario.steps), 9)

    def test_rejected_payload_is_corrected_in_the_same_evaluator(self):
        scenario = Scenario(3, reject_once=True)
        resolve(scenario.spawn, scenario, "question")
        self.assertEqual(scenario.spawned, ["planner", "evaluator"])
        self.assertEqual(scenario.submitted, ["0", "1", "2"])
        self.assertIn("invalid_enum", scenario.correction)

    def test_invalid_context_scope_can_be_corrected_without_restarting(self):
        scenario = Scenario(3, invalid_context_once=True)
        resolve(scenario.spawn, scenario, "question")
        self.assertEqual(scenario.spawned, ["planner", "evaluator"])
        self.assertEqual(scenario.submitted, ["0", "1", "2"])
        self.assertIn("invalid_scope", scenario.correction)
        self.assertEqual(sum(step["action"] == "start" for step in scenario.steps), 1)

    def test_planner_validates_and_corrects_before_retrieval(self):
        scenario = Scenario(0, invalid_plan_once=True)
        resolve(scenario.spawn, scenario, "question")
        self.assertEqual(scenario.planner_turns, 2)
        self.assertEqual(scenario.spawned, ["planner"])
        self.assertEqual(sum(step["action"] == "start" for step in scenario.steps), 1)

    def test_no_evaluator_for_empty_retrieval(self):
        scenario = Scenario(0)
        self.assertEqual(resolve(scenario.spawn, scenario, "question"), ([], [], None))
        self.assertEqual(scenario.spawned, ["planner"])

    def test_evaluator_failure_interrupts_without_a_judgment(self):
        scenario = Scenario(evaluator_error=True)
        with self.assertRaisesRegex(LunaFailure, "evaluator_thread_start_rpc_rejected"):
            resolve(scenario.spawn, scenario, "question")
        self.assertTrue(scenario.steps[-1]["luna_unavailable"])
        self.assertEqual(scenario.submitted, [])
        self.assertFalse(any(step.get("publish") for step in scenario.steps))

    def test_tables_use_the_correct_rows(self):
        row = {"native_session_id": "confirmed", "title": "A | B",
               "assessment": {"explanation": "Evidence"}}
        other = {"native_session_id": "unconfirmed", "title": "C",
                 "assessment": {"explanation": "Generic", "missing_evidence": "No original test"}}
        tables = render([row], [other], "Interrotta")
        confirmed, unconfirmed = tables.split("**Possibili candidate non confermate**")
        self.assertIn("A &#124; B", confirmed)
        self.assertNotIn("`unconfirmed`", confirmed)
        self.assertIn("No original test", unconfirmed)
        self.assertIn("Interrotta", unconfirmed)
        self.assertIn("Nessuna sessione confermata", render([], [], None))
        self.assertIn("Nessuna candidata plausibile", render([], [], None))

    def test_single_best_requires_luna_selection(self):
        class Choose:
            def __init__(self, answer):
                self.answer = answer

            def turn(self, payload):
                return {"native_session_id": self.answer}

        rows = [{"native_session_id": identity, "explanation": identity}
                for identity in ("first", "second")]
        self.assertEqual(select_best(Choose("second"), "question", rows), rows[1])
        self.assertIsNone(select_best(Choose(None), "question", rows))
        with self.assertRaisesRegex(ResolverError, "invalid_luna_response"):
            select_best(Choose("forged"), "question", rows)


if __name__ == "__main__":
    unittest.main()
