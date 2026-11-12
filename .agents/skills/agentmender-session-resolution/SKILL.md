---
name: agentmender-session-resolution
description: Find and verify past Codex conversations with mandatory Luna Medium interpretation and assessment.
---

Use the [resolution workflow](../../../docs/intent-aware-resolution.md).
Require Luna Medium before starting a conversational search. If the host
identifies the originating model as GPT-6 Luna with medium effort, interpret
and assess directly. Otherwise start the named `session_relevance` agent
once and verify the host's resolved model and effort on its start event.
A role name or self-report alone is insufficient. If the role is unavailable
or resolves incorrectly, explicitly spawn `gpt-6-luna` with medium effort and
verify its execution. Use the same delegate for interpretation, batch
assessments and any final selection. A failed or unobservable Luna execution
is an availability error. Before a run, stop before retrieval. During a run,
use `advance` with `luna_unavailable:true` and no judgments. Do not substitute
another model or deterministic discovery for semantic verification.

The parent checks one SearchIntent against the original request and sends
`start`. Use `filter` for advanced Boolean expressions and
`constraints.native_session_id` for an exact identity. Keep the run and
signed references in the parent. Give Luna only the user's intent, candidate
identities and bounded passages with handles. Pass submitted judgments,
context requests and continuation through `advance` with the returned
`step_revision` and a new operation ID. Continue while the resolver reports
`in_progress`. Accumulate confirmed and plausible unconfirmed rows by identity,
removing invalidated rows. For `single_best`, ask the same Luna to choose among
fresh confirmed rows, or report ambiguity. Render exactly the two localized
tables in the workflow. Put any coverage limit or interruption in a status row.

`session_discover` is an independent deterministic inventory primitive. Its
results carry no Luna verification and do not answer conversational searches.
