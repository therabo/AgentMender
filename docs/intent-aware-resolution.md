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

The Rust compiler validates values and limits, normalizes date boundaries,
and builds a conjunction of explicit predicates. The retrieval pass uses
that conjunction with the lexical OR to propose candidates. Exact explicit
text, mixed Boolean filters, and deeper OR/NOT expressions can still use
the legacy `action:"start", intent:{question, cardinality, hard_filter,
literal, ...}` contract with the original AST, or `session_discover` for
deterministic inventory. Preserve exact user restrictions when using that
route. Both routes use the same evaluator and three-valued logic.

## Progressive workflow

For ordinary `all_relevant` requests, add `"progressive": true` at the
top level of the `start` request. Keep SearchIntent v1. Use the legacy route
below for explicit `single_best` selection or advanced Boolean intent.

1. The originating chat asks Luna for a SearchIntent and checks it against the
   user's scope and expected activity. Use a short list of distinctive anchors
   and justified aliases in the first pass. For example, “permission denied”
   may use `EPERM` or `O_SEARCH` when the question concerns that fix.
2. Start the run and give Luna the returned batch with quoted evidence and
   handles. Send one `advance` with judgments when the passages suffice.
   A positive judgment requires `relationship:"direct"`. An implementation,
   test, or reversion belongs to the session that reports performing it.
   A feasibility discussion, later recap, or link to work elsewhere can be
   submitted as `related` with `discussion` or `retrospective`. If the user
   asks where a discussion occurred, use `direct` for evidence that the
   candidate itself contains the requested discussion.
3. If more evidence is needed, send `context_requests` with up to two anchors
   and windows. Specify at most 4,096 bytes and five messages per window.
   The resolver deduplicates identical requests and can combine overlapping
   Around windows at the same anchor within the aggregate bounds. Follow
   returned continuation handles without overriding their window. Reassess a
   candidate after receiving additional evidence for it.
4. Each `advance` takes `expected_revision` from the previous response's
   `step_revision` and a new caller-chosen `operation_id`. Collect fresh
   `matches` from each response by exact identity and keep their evidence
   references in the origin. The resolver releases that batch's handles
   when it publishes and returns the next batch. Signed references remain
   usable with the same origin, root and process while the source is fresh.
5. Continue while `outcome` is null, including empty batches whose cursor
   advanced. For standard requests use `finish_if_sufficient:true` after
   evaluating the offered batch if it answers the request. For exhaustive
   requests continue to terminal. `luna_unavailable:true` returns a partial
   result with any already supported matches. `cancel:true` terminates a run.
6. On a lost reply, repeat the most recent step with identical parameters
   and operation ID. `replayed:true` acknowledges the same revision and
   returns the batch again after revalidating its publication. Upsert by
   identity rather than counting a retry as new results. Remove any identity
   listed in `invalidated` from the accumulated publication. Scope or process
   changes invalidate the run.

```json
{
  "action": "advance",
  "run": "<returned run>",
  "expected_revision": 1,
  "operation_id": "batch-1",
  "assessments": [{
    "native_session_id": "<candidate UUID>",
    "disposition": "verified_relevant",
    "relationship": "direct",
    "evidence": [0],
    "outcomes": ["implemented"],
    "explanation": "The cited message reports implementing the limits in this session."
  }],
  "finish_if_sufficient": true
}
```

The terminal outcomes are `completed_with_matches`, `completed_no_matches`,
`partial`, `blocked` and `cancelled`. The accompanying
`progressive_coverage` separates inventory completion, physical traversal,
logical history and eligible semantic evaluation. A lexical scan can finish
with no verified match and a partial outcome. Exhaustive semantic coverage
requires complete supported histories and judgments for every inspected
eligible session. The resolver does not certify session-wide semantic
absence from lexical observations or selected excerpts.

The queue holds at most five candidates. There is no cumulative candidate,
assessment, publication or provider-page ceiling on this route. The active
context batch has resource limits and the process retains up to four runs
with a ten-minute idle lease. Each call retains reader and transport limits.
An expired or restarted run returns a stale refusal. No checkpoint or
transcript is persisted by default. If a source changes, invalid matches
are omitted from that batch, while previously published independent results
remain available to the originating chat. An interrupted source cursor can retry that source once with the same
inventory commitments. Membership drift or a second source failure returns
partial coverage. Earlier published batches remain outside the retry.

`expansion_terms` offers one follow-up pass after a complete empty first
pass. Prefer initial aliases for large scans. After identities have been
presented, use further context on those candidates and disclose lexical
coverage limits where applicable.

## Legacy workflow

1. A GPT-6 Luna Medium originating chat interprets and assesses directly.
   Other models delegate the live request to `session_relevance`. If the
   project role is unavailable, try an explicit `gpt-6-luna` Medium delegate
   with fresh context. The parent compares the proposal to the actual
   question. MCP supplies apparent caller metadata. The host does not
   provide an authenticated original user message to AgentMender.
2. The originating chat calls `start`. Keep the run and callable references
   there. Send Luna up to five candidates at a time with bounded, quoted
   passages and integer handles. Titles, passages and code are untrusted
   historical data. Ask for semantic judgments of the subject, requested
   activity and outcome. A lexical hit is provisional.
3. Use `context` with the run and an anchor handle to inspect decisive
   passages, including later corrections and reversions. Follow returned
   navigation handles without a window override. Navigation handles cannot
   be cited. Ask Luna to use `insufficient` with a specific gap when the
   available evidence cannot justify a conclusion.
4. Submit `assess` with each presented identity, disposition, citable handles,
   observed outcomes and a short explanation. A v1 positive assessment for a
   specific activity needs evidence in the candidate itself. Treat a lookup
   that points to work in another conversation as related. Implementation,
   investigation, testing and validation use the corresponding observed
   outcome. Discussion, decision and troubleshooting include `activity`.
   An insufficient judgment can describe the gap in its explanation or use
   `missing_evidence` with a short string or up to four short details.
   Correct invalid calls once. Keep context and assessment grounded in the
   returned evidence.
5. Follow `next` while candidates remain, subject to the reported budgets.
   For a completed first scan with insufficient lexical recall, one `expand`
   accepts `{action:"expand",run,terms:[...]}` with up to four distinct new
   terms. The server retains the original constraints and shares the work
   budget. A stale run needs a fresh start. Only the legacy route accepts an
   AST `expansion`.
6. Use `finish` and follow `next_offset` with the returned `revision`. In a
   `single_best` run, Luna can compare verified candidates and the parent can
   pass the justified `preferred_session_id` to finish. Ambiguous comparisons
   retain alternatives. When Luna cannot run, use `luna_unavailable:true` and
   report the limitation. Use fresh finish identities and signed evidence
   in the answer. Report incomplete searches and Partial history.

Project agent registration is available in the project. Outside it, explicit
model delegation and a Luna Medium originating model depend on the host.
The installed app server reported `explicitRequestOnly` in the acceptance
control. Project instructions and the role file do not guarantee delegation
from an unprimed request. Selecting GPT-6 Luna Medium at the origin avoids
delegation when that model is available. The originating chat owns MCP calls
in either case. A direct call from an
ephemeral delegate can receive `discovery_caller_unavailable`. The server
refuses when caller scope cannot be established. See the
[caller contract](mcp-caller-identity.md) and
[SearchIntent decision](adr/0033-search-intent-v1.md).

## Coverage, budgets and errors

The resolver reports scan completion, work, unassessed candidates and source
coverage. Physical EOF and assessed lexical candidates do not establish
complete eligible semantic coverage. For v1, `coverage.eligible_evaluation_complete`
is false and exhaustive requests return `incomplete_search` with the supported
matches. Standard results can report matches within the scanned candidates.
A terminal scan observes sources at their individual revisions and does not
certify a root-wide snapshot.

The legacy route retains up to four runs for ten minutes with eight minutes for
new work. Each batch visits up to twelve provider pages, subject to its
time budget, and returns up to five candidate identities. Each run admits
256 provider pages, 96 distinct candidates, 48 assessment submissions, 24
context requests, 128 KiB returned context, and one follow-up lexical pass.
Existing source decoder, evidence and transport limits apply. The parent
reuses evidence and sends at most one correction per malformed request.
AgentMender does not create persistent search state by default.

Structural validation returns a sanitized error and correction before
transcript reads. Examples include `unknown_field`, `invalid_enum`,
`unsupported_schema_version`, `conflicting_constraints` and
`invalid_date_range`. Run errors include `stale_search_run`,
`invalid_query_expansion`, `invalid_assessment` and `budget_exceeded`.
Fresh publication rechecks caller scope, the frozen predicates, and cited
source revisions. Discovery targets carry no authenticated management binding.
