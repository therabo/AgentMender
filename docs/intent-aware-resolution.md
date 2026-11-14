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
returns one finalized snapshot. `results.confirmed` and
`results.possible_unconfirmed` include the title, UUID, verification basis
and signed selection references. `results.excluded` includes rejected
candidates with their UUID, title and reason.
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
  `troubleshooting`. A specific event, action or outcome requires supporting conversation
  evidence. Ordinary topics use `any` or `discussion` and can be verified
  from lexical observations. `event` covers a requested occurrence that
  has no more specific activity.
- `required_outcome` and `excluded_outcome` accept null or an observed outcome
  (`investigated`, `proposed`, `attempted`, `implemented`, `tested`, `validated`,
  `resolved`, `reverted`, `failed`, `unresolved`). An investigation can match
  without an implementation. A claim that an outcome never occurred requires
  trusted complete logical history and an assessment of that history.
  A bounded or paginated source with incomplete logical coverage keeps
  such a candidate possible.
- `result_policy` defaults to `all_relevant`, including singular wording.
  Use `single_best` when the user asks to choose one. `coverage_goal` defaults
  to `standard`. Use `exhaustive` for explicit complete-coverage requests.
- `hints.terms` takes up to eight terms or phrases. The compiler creates
  a lexical OR and skips generic hints when distinctive terms are present. Project
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

Use `kind: "contains"` for a title fragment and `kind: "approximate"` for
an uncertain title. Approximate titles allow word reordering, distinctive
token overlap and one minor edit. Lexical hints allow Unicode accents,
simple inflections and alternate terms. Explicit Boolean filters preserve
AND, OR and NOT. The Planner translates vague dates into calendar bounds
using the specified or local zone. For example, early November is November
1–10 and around November 7 is November 4–10. Explicit dates and intervals
retain their bounds. Title, creation and activity
dates, provider, archive state, project and UUID can satisfy a lookup from
metadata. A title OR text query can be satisfied by either branch. If the
title proves the whole predicate and no global activity or outcome requires
semantic judgment, Rust verifies the row without reading its transcript.
Text predicates use observed messages. An exact term or ordinary topic can
be verified without an Evaluator. A future action such as deleting the returned
conversations contributes no historical terms or outcomes.

## AgentMender-owned Luna execution

The Rust host starts an ephemeral Codex App Server for the Planner and
requires `gpt-6-luna` with medium effort and provider fallback disabled.
The coordinator checks the observed thread start, resolved model and
effort, and turn completion. It validates the Planner payload against
SearchIntent v1 and the Rust compiler. Corrections stay in the same
Planner. Rust starts retrieval after the Planner process exits.

After Rust traverses the cohort, the host starts an Evaluator only when a
candidate needs semantic assessment. The same process handles the batches
and any bounded context request. The host checks the model events and
reaps the process before finalization.

The Evaluator submits `assessments` with the supplied `native_session_id`,
`relevance` (`relevant`, `possible`, or `not_relevant`), integer `evidence`
handles and an optional `reason`. A positive judgment needs a citation.
Rust checks each entry independently and validates its citations against the
candidate. Missing, malformed and conflicting judgments become possible.
A whole-response failure ends evaluation. Rust keeps valid judgments,
classifies unfinished candidates as possible and records the interruption.
A session-wide claim of absence requires trusted complete logical coverage.

For `single_best`, a uniquely stronger lexical match can be selected from
the deterministic results. Semantic comparison takes at most one turn
when the confirmed set is small enough. Ties remain ambiguous.

## Finalization and diagnostics

The private Rust host owns the run ID, revision and operation sequence.
The diagnostic `session_resolve` tool retains its explicit revision and
operation IDs for retries. Its evaluator handoff carries inert citation
handles. Each verified receipt is staged after evaluation. The host publishes
the completed snapshot through `session_find_result` and serves all pages
from that snapshot. Rust validates the source references before staging
and again during finalization. Excluded rows have no selection references.

The job reports candidate and publication counts, inventory and lexical
coverage, and any interruption. Match quality is independent of semantic
judgment. The `lexical_match` field reports `exact`, `normalized`,
`partial` or `approximate` when a positive title or text branch supports
the result. Pages order by that quality, then UUID. A source gap can
limit coverage even when Rust reached the end of the inventory.

The application retains one five-candidate batch and one replayable reply.
The worker's anonymous SQLite file holds frozen metadata, bounded sanitized
passages with source revisions and positions, and pending receipts. It uses
short transactions and a 512 KiB page-cache target. The oldest scan reply is
evicted after twelve provider pages. A ten-minute idle lease refreshes with
operations and the worker releases it on expiry, completion or cancellation.
A new process requires a new run. See [ADR-0036](adr/0036-two-stage-luna-resolution.md).

### Conversational answer

Present confirmed and possible rows with their original title, UUID and
reason, localized to the user. When excluded rows are returned, present
them separately with the evaluator's reason. Include any partial coverage
or interruption and keep signed references in the originating chat.

## Errors and independent discovery

Structural validation returns sanitized codes such as `unknown_field`,
`invalid_enum`, `unsupported_schema_version`, `conflicting_constraints`,
and `invalid_date_range`. Run errors include `stale_search_run`,
`invalid_query_expansion` and `budget_exceeded`.
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
