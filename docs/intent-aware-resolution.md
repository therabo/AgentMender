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

The origin owns the request, MCP calls and signed references. Start one
[session_planner](../.codex/agents/session-planner.toml) and check the actual
host events for `gpt-6-luna` and medium effort. The Planner produces one
SearchIntent. Validate it against the request and close the Planner.

Once Rust finishes retrieval, start a distinct
[session_evaluator](../.codex/agents/session-evaluator.toml) only if candidates
exist. Check the same model and effort and retain its thread ID for every
assessment and any single-best selection. Close it before publication.
Try the named role through the host's subagent interface when the
AgentMender project roles are loaded. They live under the checkout's
`.codex/agents` and are scoped to that project. A tool catalog lookup
can inspect MCP capabilities. Role discovery uses the host interface.

If a named role fails, attempt an explicit `gpt-6-luna` spawn at medium
effort and check its resolved settings and lifecycle events. A Desktop
surface that cannot expose that spawn or verify the role lifecycle can
run the [controlled App Server host](../scripts/resolve_with_luna.py) for
the entire search. Determine the configured command and provider root
from the AgentMender MCP registration. Use the originating chat UUID
from host metadata and the installed Codex executable.

```sh
python3 /path/to/AgentMender/scripts/resolve_with_luna.py \
  --agentmender /path/to/agentmender --codex /path/to/codex \
  --codex-root /path/to/codex-home --caller HOST_THREAD_UUID --trace \
  "Find every conversation about the requested topic."
```

The controlled host creates one ephemeral thread in each role's own
process. It observes the thread start and resolved model, checks
turn completion and reroutes, and waits for each process to exit. It
validates the Planner's JSON against the advertised schema before
closing the thread. Trace events identify the role, stage and observed
error, including a usage limit reported at turn completion. Report
concrete startup, configuration, permission or runtime evidence. If
both routes fail, stop before retrieval or use `luna_unavailable:true`
during an active run. The host verifies Luna execution and MCP validates
caller scope and evidence.


## Progressive workflow

1. Call `start` with the Planner's SearchIntent after the Planner closes.
   Rust freezes the eligible UUIDs, order and predicate metadata. Planned
   `hints.expansion_terms` take up to four distinct terms and form an OR
   with literal hints under the original constraints. Put Boolean aliases
   in the explicit filter. A typed constraints-only lookup can use empty
   terms and receive an original passage when available.
2. While `phase` is `retrieving`, call `advance` with no judgments or
   context. Continue through empty replies. Candidate passages are staged
   in the worker. `retrieval_complete` reports finished traversal
   independently of partial transcript coverage. An interrupted traversal
   ends without starting the Evaluator.
3. In `evaluating`, call `advance` for the next batch. Give the same
   Evaluator each candidate's identity, quoted evidence and coverage.
   Submit one conclusive judgment per candidate. Request up to two windows
   per advance for a specific evidence gap. Each window allows five
   messages and 4,096 bytes. A changed source can return with fresh handles
   for reassessment. A recap about another session does not prove original
   execution. A plausible unconfirmed result requires a supporting passage
   and a stated gap.
4. When `publication_ready` becomes true, perform any explicit
   `single_best` selection with the Evaluator using `prepared_choices`.
   These choices contain provisional judgments. Close the Evaluator and
   call `advance` with `publish:true` until terminal. Rust revalidates
   references before signing `matches` and `alternatives`. Accumulate
   them by UUID and remove any identity listed in `invalidated`.
5. Use the returned `step_revision` as `expected_revision` and a new
   `operation_id` for every operation. A long single-best comparison can
   renew the active evaluation with `keep_alive:true`. Repeat the exact request if its
   reply is lost. `cancel:true` interrupts the run and releases its spool.
   The response distinguishes `completed` from `interrupted` and reports
   coverage gaps even when UUID traversal finished.

Rust exposes `snapshot_sessions`, `sources_inspected`, `lexical_candidates`,
`candidates_staged`, `candidates_evaluated` and `remaining_candidates`.
Classifications and `retrieval_complete` and `evaluation_complete` have
separate counters. A predicate-only candidate can be staged without a
lexical match. Unrecoverable stale evidence has an explicit interruption
and remains unevaluated. Completed lexical traversal does not establish
absolute semantic recall.

The application retains one five-candidate batch and one replayable reply.
The worker's anonymous SQLite file holds frozen metadata, bounded sanitized
passages with source revisions and positions, and pending receipts. It uses
short transactions and a 512 KiB page-cache target. The oldest scan reply is
evicted after twelve provider pages. A ten-minute idle lease refreshes with
operations and the worker releases it on expiry, completion or cancellation.
A new process requires a new run. See [ADR-0036](adr/0036-two-stage-luna-resolution.md).

### Conversational answer

Render exactly these two tables, localized to the user. Use the title as the
session label, the grounded explanation for a confirmed row, and the concrete
gap for a plausible unconfirmed row. Keep signed references in the origin.
Use a clearly labeled empty row if a table contains no results. If the run was
interrupted or lexical coverage is limited, put one short status row in the
second table with a blank session ID.

**Sessioni confermate**

| Sessione | Motivazione | ID |
| --- | --- | --- |
| Nessuna sessione confermata | | |

**Possibili candidate non confermate**

| Sessione | Evidenza mancante | ID |
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

## Reopen a session by UUID

In discovery mode, `session_context` accepts a canonical
`native_session_id` on its own. It opens an independent view of the current
source and returns a signed `target` and the first bounded window. To find
distant material, start with an `occurrences` window and a short query.
Use the returned `target`, `next_ref` or `previous_ref` as `evidence_ref`, and
the corresponding suggested window to continue. The signed navigation is
caller- and source-bound. Reopen by UUID to read a new revision after append.
A missing or unsafe source reports `session_source_unavailable`.

```json
{
  "native_session_id": "00000000-0000-4000-8000-000000000001",
  "window": {"direction": "start", "max_messages": 5, "max_bytes": 4096}
}
```
