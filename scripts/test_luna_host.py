#!/usr/bin/env python3
"""Regression tests for the controlled host's Luna and publication gates."""

import unittest

from resolve_with_luna import Luna, ResolverError, render, resolve, select_best


class FakeRpc:
    def __init__(self, model="gpt-6-luna", effort="medium", reroute=False):
        self.model = model
        self.effort = effort
        self.reroute = reroute

    def call(self, method, params, observe=None):
        if method == "thread/start":
            return {"model": self.model, "reasoningEffort": self.effort, "thread": {"id": "test"}}
        if method == "turn/start":
            self.assert_requested_medium(params)
            if self.reroute:
                observe({"method": "model/rerouted", "params": {"threadId": "test", "turnId": "turn"}})
            return {"turn": {"id": "turn"}}
        return {}

    @staticmethod
    def assert_requested_medium(params):
        assert params["model"] == "gpt-6-luna" and params["effort"] == "medium"

    def send(self, payload):
        pass


class LunaHostTests(unittest.TestCase):
    def test_rejects_wrong_model_or_effort(self):
        for rpc in (FakeRpc(model="gpt-6-sol"), FakeRpc(effort="low")):
            with self.assertRaisesRegex(ResolverError, "luna_unavailable"):
                Luna(rpc, ".")

    def test_rejects_turn_reroute(self):
        luna = Luna(FakeRpc(reroute=True), ".")
        with self.assertRaisesRegex(ResolverError, "luna_unavailable"):
            luna.turn({"task": "interpret"})

    def test_planner_failure_never_calls_mcp(self):
        luna = Luna(FakeRpc(reroute=True), ".")
        with self.assertRaisesRegex(ResolverError, "luna_unavailable"):
            resolve(luna, object(), "question")

    def test_assessor_failure_interrupts_without_a_judgment(self):
        class PlannerOnly:
            def turn(self, payload):
                if "candidates" in payload:
                    raise ResolverError("luna_unavailable")
                return {"schema_version": "1", "subject": "Test", "hints": {"terms": ["test"]}}

        class Resolver:
            def __init__(self):
                self.actions = []

            def resolve(self, **arguments):
                self.actions.append(arguments)
                if arguments["action"] == "start":
                    return {"run": "test", "step_revision": 1, "execution_state": "in_progress",
                            "candidates": [{"native_session_id": "test", "evidence": []}]}
                return {"step_revision": 2}

        resolver = Resolver()
        with self.assertRaisesRegex(ResolverError, "luna_unavailable"):
            resolve(PlannerOnly(), resolver, "question")
        self.assertEqual([step["action"] for step in resolver.actions], ["start", "advance"])
        self.assertTrue(resolver.actions[1]["luna_unavailable"])
        self.assertNotIn("assessments", resolver.actions[1])

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
                self.choices = payload["choices"]
                return {"native_session_id": self.answer}

        rows = [{"native_session_id": identity, "assessment": {"explanation": identity}}
                for identity in ("first", "second")]
        self.assertEqual(select_best(Choose("second"), "question", rows), rows[1])
        self.assertIsNone(select_best(Choose(None), "question", rows))
        with self.assertRaisesRegex(ResolverError, "invalid_luna_response"):
            select_best(Choose("forged"), "question", rows)


if __name__ == "__main__":
    unittest.main()
