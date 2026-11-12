---
name: agentmender-session-resolution
description: Find past Codex conversations using one Luna Medium Planner, complete Rust retrieval and one Luna Medium Evaluator.
---

Follow the [resolution workflow](../../../docs/intent-aware-resolution.md).
Start one named `session_planner`. Verify `gpt-6-luna` and medium effort
from host events. Close it after obtaining one SearchIntent. Use a pinned
Luna Medium fallback only when the host can verify its identity, effort
and lifecycle. Keep explicit Boolean filters and UUIDs. Plan lexical
aliases before calling `start`.

While `phase` is `retrieving`, send empty `advance` operations until
retrieval completes. Start no model during this phase. If candidates exist
after traversal, start one distinct `session_evaluator` and verify its model
and effort. Send each batch to that same thread, with the immutable request
and intent and bounded original evidence. Keep all MCP calls, revisions and
signed references in the parent. Use context requests for specific gaps.
Continue through every batch, including refreshed candidates.

When `progress.publication_ready` is true, ask the same Evaluator for any
explicit `single_best` choice from provisional `prepared_choices`.
Close the Evaluator. Call `advance` with `publish:true` to drain the
fresh signed rows. Accumulate `matches` and `alternatives` by UUID and remove
`invalidated` rows. Render the two localized result tables and put any
coverage gap or interruption in a status row.

Use the host's subagent interface to discover named roles. If a named
role fails, attempt an explicit GPT-6 Luna Medium spawn. When the
Desktop surface cannot attest that lifecycle, use the workflow's
controlled App Server host for both stages. If the required instance
still fails, report the observed stage and error. Stop before retrieval
or send `luna_unavailable:true` during an active run. Deterministic
`session_discover` is a separate inventory operation.
