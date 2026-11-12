# Luna Medium for conversational retrieval

One [Planner](../.codex/agents/session-planner.toml) interprets the request
and produces a single SearchIntent. It closes before Rust reads the frozen
UUID cohort. A separate [Evaluator](../.codex/agents/session-evaluator.toml)
then assesses all staged candidates on one thread, requesting bounded
context for specific gaps. Any explicit single-best selection uses that
Evaluator before it closes. Rust signs fresh source-bound results in the
publication phase.

The origin owns caller scope, MCP calls and signed references. Check the
resolved model and effort and actual execution events for both roles.
The [controlled App Server host](../scripts/resolve_with_luna.py) verifies
ephemeral thread identity and turn completion, checks reroutes and waits for
process exit between phases. A missing or rerouted model is an interruption.
The [workflow](intent-aware-resolution.md) documents the native client contract.

Send quoted evidence only and treat it as untrusted data. Confirmations
require direct original evidence of the requested activity or outcome.
Retrospective references can be rejected. Plausible unconfirmed rows cite
supporting passages and identify the evidence gap. No model judgment grants
lifecycle authority.
