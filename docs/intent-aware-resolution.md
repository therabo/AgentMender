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
   A positive judgment requires `relationship:"direct"` and original
   evidence in that candidate. A substantive discussion can satisfy a
   discussion request. Implementation, testing and validation need passages
   that establish those activities in that session. Judge an unrelated
   lexical hit or a recap of work elsewhere as `related`, or use
   `contradiction` when cited evidence rules out the requested outcome.
3. When decisive context is missing, send `context_requests` with up to two
   anchors and at most 4,096 bytes and five messages per window. Follow
   continuation handles without overriding their window. Reassess only the
   candidate whose context was extended. Use `insufficient` with a concrete
   missing-evidence reason once available context is exhausted. Set
   `plausible_alternative:true` only when cited passages support a plausible
   connection to the original request. `context_failures` identifies any
   unavailable window by anchor and gives a sanitized reason.
4. Use the previous response's `step_revision` as `expected_revision` and
   give each new operation an ID. Retain every confirmed `matches` row and
   plausible unconfirmed `alternatives` row by exact identity, with their
   signed evidence references. Released handles are replaced by those
   references. A stale candidate in the active batch can return with fresh
   handles for a new assessment. The original corpus cursor and independent
   rows are preserved.
5. Continue while `execution_state` is `in_progress`, including empty
   batches. A full scan with no candidates allows an optional expansion
   before the empty final advance. The compatibility field
   `finish_if_sufficient` is accepted without requesting early completion.
   `luna_unavailable:true` interrupts after publishing submitted valid
   judgments. `cancel:true` interrupts without further reads.
6. On a lost reply, repeat the most recent step with identical parameters
   and operation ID. The server sets `replayed:true` and revalidates the
   publication. Upsert by exact identity and remove every `invalidated` row
   from the accumulated output. The resolver has one replayable reply, and
   stale evidence in a released reply is withdrawn. Scope or process changes
   invalidate the run. If an MCP failure leaves no usable continuation, report
   an interruption with the valid rows already collected.

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
  }]
}
```

The per-row `verification_state` is `confirmed` for `matches` and
`unconfirmed` for `alternatives`. Rejected candidates contribute to the
coverage counters. An unconfirmed judgment without a plausible connection
also contributes to coverage without generating a row.

`execution_state` is `completed` when the lexical traversal finishes and
every retrieved candidate has a conclusive judgment, including unconfirmed
judgments. `progressive_coverage.candidate_evaluation_complete` records this
criterion. Missing sources or unresolved technical failures yield
`interrupted` with a `progress.stop_reason`. The previous `outcome` field
remains available. It uses `completed_no_matches` when there are zero
confirmed rows. It uses `partial`, `blocked` or `cancelled` on interruption.

`progressive_coverage` reports inventory, physical and logical coverage, and
the conservative eligible semantic assessment flag. These are independent of
the two result groups. A partial original history can establish a local
positive. Report `lexical_coverage_limited` when unsupported or omitted
history limits the searchable material. The resolver does not certify
session-wide absence from lexical observations or selected excerpts.

The queue holds at most five candidates. There is no cumulative candidate,
assessment, publication or provider-page ceiling. Context and refresh work
remain bounded per active batch. The process retains up to four runs with a
ten-minute idle lease. A new process or an expired lease requires a new run.
An interrupted in-flight cursor can retry its current source once with the
same inventory commitments. At publication, the resolver reacquires only
stale candidate identities and asks Luna to assess their new excerpts once.
Repeated change or an unavailable replacement yields an explicit
interruption. A replay can withdraw stale rows from the most recent reply.
Earlier signed references remain subject to their normal source checks.

`expansion_terms` offers one follow-up pass after a complete empty first
pass. The resolver drains the expanded scan with the same queue bounds.
Include known justified aliases in the initial plan.

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
complete eligible semantic coverage. On the legacy v1 route,
`coverage.eligible_evaluation_complete` is false and exhaustive requests
return `incomplete_search` with the supported matches.
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
