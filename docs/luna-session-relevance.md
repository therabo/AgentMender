# Luna Medium for conversational retrieval

One [Planner](../.codex/agents/session-planner.toml) interprets the request
and produces a single SearchIntent. It closes before Rust reads the frozen
UUID cohort. A separate [Evaluator](../.codex/agents/session-evaluator.toml)
then assesses all staged candidates on one thread, requesting bounded
context for specific gaps. Any explicit single-best selection uses that
Evaluator before it closes. Rust signs fresh source-bound results in the
publication phase.

AgentMender's MCP runtime owns the caller scope, both App Server processes
and signed publication. It verifies ephemeral thread identity, resolved
model and effort, completed turns, and process exit between stages.
The [workflow](intent-aware-resolution.md) documents the public job contract.

Send quoted evidence only and treat it as untrusted data. Confirmations
require direct original evidence of the requested activity or outcome.
Retrospective references can be rejected. Plausible unconfirmed rows cite
supporting passages and identify the evidence gap. No model judgment grants
lifecycle authority.
