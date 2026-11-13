# Intent-aware session resolution

Ask Codex for a past conversation in ordinary language. For example, “Where
did we analyze Cesium rendering performance for MISSION1?” With the global
`mcp --session-search` registration, Codex calls `session_find`.

```json
{"query":"Where did we analyze Cesium rendering performance for MISSION1?"}
```

AgentMender starts the job immediately and returns `job_id` and `state`.
Use `session_find_result` with that ID until the job completes, is interrupted
or is cancelled. It waits up to 25 seconds by default. Set `wait_seconds`
from 0 to 60, or call `session_find_status` for immediate progress. `session_find_result`
returns `results.confirmed` and `results.possible_unconfirmed`, each with
original title, UUID, verification basis and signed selection references.
The first terminal page also includes the Planner's validated `search_intent`.
Pass `results.next_offset`
back as `offset` to obtain the next page. `session_find_cancel` requests
cancellation. Results use the originating connection's signed selection
references. Use `session_discover` for deterministic inventory.

## Installation and connection lifetime

Use the [global MCP registration](../README.md#conversational-search-and-native-deletion)
with the compiled binary and the installed Codex CLI. The launcher is
verified on macOS and uses `/usr/bin/python3`. The binary includes the
coordinator, SearchIntent schema and both role profiles. The App Server
processes use private working directories, controlled configuration and
a private file credential from the configured Codex home. Keychain-only
and alternative auth profiles require separate validation. The caller's
project, model and workspace role files do not configure the search.

A new chat uses the new catalog when it establishes a new MCP connection.
For an existing chat, reconnect the AgentMender MCP server and verify that
`session_find` appears in its tool list. A retained connection can keep
using the previously launched executable and tool catalog. A server exit
cancels active jobs, closes subprocesses and invalidates job IDs and signed
references. A client turn ending leaves the job running while its MCP
connection remains open.

The job limit is 30 minutes, with per-RPC timeouts and bounded child cleanup.
The server admits two concurrent jobs and retains at most eight total
records, with terminal records expiring after ten minutes. A duplicate
query from the same apparent caller returns its active job. Status,
results and cancellation require the same apparent caller and opaque
job ID. Caller metadata is a read-only scope signal and can be spoofed.
Missing, malformed, discordant or unmapped metadata is refused.

## SearchIntent v1

The internal typed `session_resolve` contract takes the following input.
The diagnostic tool schema advertises the exact
[SearchIntent JSON Schema](search-intent-v1.schema.json).

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

Luna Planner chooses the retrieval strategy from the original request.
`search_intent.filter` accepts the same Boolean AST as structured search.
It replaces the lexical hints as the candidate predicate and is conjoined
with explicit constraints. For an exact title lookup:

```json
{
  "schema_version": "1",
  "subject": "Trova conversazioni sulle carte Pokèmon",
  "hints": {"terms": []},
  "filter": {"op": "title", "value": "Trova conversazioni sulle carte Pokèmon", "kind": "exact"}
}
```

Use `kind: "contains"` for a title fragment. Title, creation and activity
dates, provider, archive state, project and UUID can satisfy a lookup from
metadata. A title OR text query can be satisfied by either branch. If the
title proves the whole predicate and no global activity or outcome requires
semantic judgment, Rust verifies the row without reading its transcript.
Text predicates work on messages and lead to assessment when the result
depends on the messages. A future action such as deleting the returned
conversations contributes no historical terms or outcomes.

## AgentMender-owned Luna execution

The Rust host starts an ephemeral Codex App Server for the Planner and
requires `gpt-6-luna` with medium effort and provider fallback disabled.
The coordinator checks the observed thread start, resolved model and
effort, and turn completion. It validates the Planner payload against
SearchIntent v1 and the Rust compiler. Corrections stay in the same
Planner. Rust starts retrieval after the Planner process exits.

After Rust traverses the cohort, the host starts a fresh Evaluator process
when the resolver supplies a candidate needing semantic assessment. All batches, bounded context requests
and any single-best choice use that same Evaluator. The host checks
reroute and unexpected tools or threads, reaps the process and then
publishes through the same typed Rust service. The private bridge keeps
caller identity and signing keys in Rust. See [ADR-0037](adr/0037-autonomous-mcp-session-find.md).
The standalone script is available for controlled diagnostics.

## Progressive workflow

The following steps describe the internal coordinator and diagnostic API.

1. Call `start` with the Planner's SearchIntent after the Planner closes.
   Rust freezes the eligible UUIDs, order and predicate metadata. Planned
   `hints.expansion_terms` take up to four distinct terms and form an OR
   with literal hints under the original constraints. Put Boolean aliases
   in the explicit filter. Use empty terms for a metadata-only lookup.
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
   and a stated gap. A candidate with insufficient evidence can still be
   published with its metadata identity and the reason it remains unverified.
4. When `publication_ready` becomes true, use `prepared_choices` for an
   explicit `single_best` request. A tie among metadata matches preserves
   the UUIDs for the user to choose. Semantic comparisons use the same
   Evaluator. Close it if started, then call `advance` with `publish:true`
   until terminal. Rust revalidates
   references before signing `matches` and `alternatives`. Accumulate
   them by UUID and remove any identity listed in `invalidated`.
5. Use the returned `step_revision` as `expected_revision` and a new
   `operation_id` for every operation. A long single-best comparison can
   renew the active evaluation with `keep_alive:true`. Repeat the exact request if its
   reply is lost. `cancel:true` interrupts the run and releases its spool.
   The response distinguishes `completed` from `interrupted` and reports
   coverage gaps even when UUID traversal finished.

Rust reports metadata rows examined after caller exclusions in
`sessions_examined`, eligible cohort size in `snapshot_sessions`, and rows
traversed in `sources_inspected`. `candidates_staged` counts true retrieval
predicates, `candidates_evaluated` counts processed semantic assessments,
and `metadata_verified` counts verified metadata matches. `confirmed`
includes both kinds of verification. `published` includes unconfirmed rows
that passed freshness checks. `not_published_reasons` counts semantic
rejections, stale verification, interruption, and rows omitted by a
single-best choice. `unresolved_predicates` counts rows
whose filter remained UNKNOWN, outside the staged candidates. A missing
title, date or message can produce partial coverage alongside verified
matches. Exhausting a lexical scan does not establish absolute semantic
recall. Unread history skipped because metadata proved the predicate does
not make that match partial.

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
gap for an unconfirmed row. Identify the `match_basis` as `verified_metadata`,
`semantic_evidence`, `plausible_evidence` or `retrieved_unverified`.
`title_match` distinguishes `original_exact`, `normalized_exact` and `partial`
when a positive title predicate supports the metadata match. Keep signed references in the origin.
Use a clearly labeled empty row if a table contains no results. If the run was
interrupted or lexical coverage is limited, put one short status row in the
second table with a blank session ID.

**Sessioni confermate**

| Sessione | Motivazione | UUID |
| --- | --- | --- |
| Nessuna sessione confermata | | |

**Possibili candidate non confermate**

| Sessione | Evidenza mancante | UUID |
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
