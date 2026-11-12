# Intent-aware session resolution

Ask Codex for a past conversation in ordinary language. For example, “Where
did we analyze Cesium rendering performance for MISSION1?” A current
`mcp --session-search` registration exposes the read-only `session_resolve`
workflow. Use `session_discover` for deterministic inventory, exact identities
and possible deletion candidates. “Search only conversations labeled MISSION1”
with no topic is a metadata inventory request.

## SearchIntent v1

Call `session_resolve` with the following ordinary input. The final
executable's `tools/list` advertises the exact [SearchIntent JSON Schema](search-intent-v1.schema.json).

```json
{
  "action": "start",
  "search_intent": {
    "schema_version": "1",
    "subject": "Cesium rendering performance analysis for MISSION1",
    "activity": "investigation",
    "hints": {
      "terms": ["Cesium", "rendering performance"],
      "project_names": ["MISSION1"]
    }
  }
}
```

- `activity` defaults to `any`. The specific activities are `discussion`,
  `investigation`, `implementation`, `testing`, `validation`, `decision` and
  `troubleshooting`. An activity requires supporting conversation evidence.
- `required_outcome` and `excluded_outcome` accept null or an observed outcome
  (`investigated`, `proposed`, `attempted`, `implemented`, `tested`, `validated`,
  `resolved`, `reverted`, `failed`, `unresolved`). An investigation can match
  without an implementation. A claim that an outcome never occurred requires
  complete logical history and an assessment of that history. The resolver
  cannot publish a positive verification of a session-wide exclusion from
  its current bounded assessment contract. Paginated Codex history
  remains Partial.
- `result_policy` defaults to `all_relevant`, including singular wording.
  Use `single_best` when the user asks to choose one. `coverage_goal` defaults
  to `standard`. Use `exhaustive` for explicit complete-coverage requests.
- `hints.terms` takes one to eight distinctive terms or phrases. The compiler
  creates a lexical OR. Keep the initial list short and selective. Project
  names in `hints.project_names` inform semantic assessment and leave eligible
  scope unchanged. A relevant session may have any stored label or none.
- `constraints` may be omitted or null. Set `project_membership` with mode
  `include` or `exclude`, one to four names and basis `observed` or `verified`.
  “Only conversations labeled MISSION1” calls for observed inclusion. “Opened
  under another project” calls for observed exclusion. Missing membership
  evaluates to UNKNOWN under either inclusion or exclusion. The current Codex
  metadata reader supplies observed labels. Verified membership stays UNKNOWN
  when independent membership evidence is unavailable.
- The other constraints are `target_provider` (`codex` or `claude`),
  `archive_state` (`active` or `archived`), `native_session_id` (canonical UUID),
  `created_range`, and `last_active_range`. Each may be omitted or null. Dates
  use inclusive `start_date` and `end_date` with an IANA `timezone`, or integer
  UTC seconds with inclusive `start` and exclusive `end`. One date bound may
  be absent. A target provider filters within the registered provider/root.
  The current resolver reads the configured Codex root.

The Rust compiler validates values, compiles the explicit predicates, and
uses the three-valued query evaluator. When the request specifies an exact
Boolean expression, supply `search_intent.filter` as the structured query
AST. It replaces the lexical hints as the candidate predicate and is
conjoined with the explicit constraints. This preserves AND/OR/NOT and
UNKNOWN semantics. For an exact session lookup set
`constraints.native_session_id` and use `hints:{"terms":[]}` when the user
has supplied no text condition. Rust offers a bounded original passage even
when that selection has no lexical anchor. Both modes use the same assessment
and publication rules.

## Required Luna execution

The origin owns the user's request and MCP calls. If the host identifies the
origin as GPT-6 Luna Medium, use it directly. Otherwise request the named
`session_relevance` role once. Verify the host's start and completion events,
including resolved `model:gpt-6-luna` and `reasoningEffort:medium`. If the
role is unavailable or the resolved model/effort is wrong, explicitly request
that model and effort and observe successful execution. The same delegate
interprets once and assesses the batches. If the host cannot establish Luna
execution, stop with an availability error. An active run can be interrupted
with `luna_unavailable:true` and no judgments.

The repository provides [a controlled Codex app-server host](../scripts/resolve_with_luna.py).
It uses an ephemeral pinned Luna Medium thread, disables provider-model fallback
and checks the resolved configuration, reroute notifications and successful turn
completion before allowing the corresponding tool step. It renders the same two
tables and does not emit confirmations if the model fails. The direct MCP
interface has caller exclusion and evidence validation. Its transport cannot
authenticate model identity or delegation. Desktop instruction loading, the
tool description and the role configuration guide the native origin, which must
verify its own host events.

## Progressive workflow

1. Ask Luna for one SearchIntent and verify it against the original request.
   Use `start` with that object in all modes. The origin retains the immutable
   plan, run, evidence handles and signed references.
2. Give Luna the candidate identities and bounded quoted evidence. Submit one
   judgment per candidate with `advance`. Direct original evidence can confirm
   a session with partial history. A recap of work done elsewhere does not
   establish original execution. Use `insufficient` when targeted context
   cannot settle the question. Mark an insufficient result as a plausible
   alternative only when a cited passage supports plausibility.
3. To resolve a specific gap, request up to two context windows in an advance.
   Each window allows five messages and 4,096 bytes. Follow navigation handles
   without overriding their options. Reassess only the candidate whose context
   was extended. `context_failures` identifies unavailable windows.
4. Pass the last `step_revision` as `expected_revision` and a new
   `operation_id` for each advance. Accumulate `matches` and `alternatives`
   by exact identity and retain signed references. A changed source can return
   with fresh handles for reassessment. If a reply is lost, repeat its operation
   ID and exact arguments. Remove identities in `invalidated`.
5. Continue while `execution_state` is `in_progress`, even with empty pages.
   For ordinary lexical hints, one `expansion_terms` submission is allowed at
   the end of the first pass. Submit it with the last outstanding judgments.
   The server preserves scope. After a nonempty first pass it requires a definite
   nonmatch on the original terms to keep the lexical partitions disjoint.
   Original terms that are UNKNOWN on partial history remain coverage gaps.
   Sources can change between passes, so the origin upserts by exact identity.
   Prefer known aliases in the initial SearchIntent. For Boolean intent include
   allowed alternatives in the explicit filter.
6. Completion means the eligible lexical scan ended and all acquired candidates
   received a conclusive judgment. Coverage also reports physical and logical
   history and independent lexical limitations. Interruption reports a reason.
   Only for `single_best`, ask the same Luna to choose among the confirmed rows
   after traversal. Report ambiguity if the evidence cannot select one.
   `cancel:true` interrupts without further reads.

The queue holds up to five candidates. Per-operation and per-batch limits,
one replayable reply and a ten-minute idle lease keep work bounded without a
cumulative candidate ceiling. Process loss invalidates the run and references.
The route preserves signed source validation and cannot certify a session-wide
negative from bounded excerpts or lexical recall alone.

### Conversational answer

Render exactly these two tables, localized to the user. Use the title as the
session label, the grounded explanation for a confirmed row, and the concrete
gap for a plausible unconfirmed row. Keep signed references in the origin.
Use a clearly labeled empty row if a table contains no results. If the run was
interrupted or lexical coverage is limited, put one short status row in the
second table with a blank session ID.

**Sessioni confermate**

| Sessione | Evidenza / motivazione | ID sessione |
| --- | --- | --- |
| Nessuna sessione confermata | | |

**Possibili candidate non confermate**

| Sessione | Motivo della mancata conferma | ID sessione |
| --- | --- | --- |
| Nessuna candidata plausibile | | |

## Errors and independent discovery

Structural validation returns sanitized codes such as `unknown_field`,
`invalid_enum`, `unsupported_schema_version`, `conflicting_constraints`,
and `invalid_date_range`. Run errors include `stale_search_run`,
`invalid_query_expansion`, `invalid_assessment` and `budget_exceeded`.
When a call failure leaves no usable continuation, report interruption in the
second table. Model availability failure is explicit and semantic
confirmation requires successful Luna execution.

Use `session_discover` for deterministic inventory and possible deletion
candidates. `session_context` independently navigates signed passages.
These operations carry no semantic verification or management authorization.
See [caller identity](mcp-caller-identity.md) and the
[progressive decision](adr/0034-progressive-resolution.md).
