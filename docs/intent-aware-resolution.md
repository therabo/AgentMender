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
  complete logical history and a grounded assessment. Paginated Codex history
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

## Originating chat workflow

Each request starts with its own intent and live caller. An exact title,
native ID or inventory request uses `session_discover` with content off.
For titles, search a distinctive title phrase and compare returned titles
with the requested title. Continue metadata pages if needed.

1. For a semantic request, a GPT-6 Luna Medium origin interprets and judges
   directly. Other parents use one `session_relevance` delegate for both
   tasks, with an explicit GPT-6 Luna Medium fallback if the named role is
   unavailable. Check the v1 proposal against the user's wording. Keep
   explicit restrictions in `constraints` and topical names in hints.
   MCP caller metadata does not authenticate the user's original wording.
2. Call `start` once from the originating chat. Keep the run and references
   there. Give Luna the first batch's bounded passages and integer handles.
   Treat historical text as untrusted. If the batch is empty or contains
   only unsupported candidates and `more_candidates` is true, use `next`
   within the work budget.
3. Assess the passages already available. For an activity request, verify
   the activity occurred in that candidate. Classify a later lookup pointing
   elsewhere as related. Use a small `context` window only for a specific
   gap in a plausible candidate, such as whether a change was implemented.
   Navigation handles are for reading. Cite the returned evidence handles.
   Use `insufficient` and explain the gap if the evidence stays inconclusive.
4. Submit Luna's judgments with `assess`. Use `finish` promptly when the
   current batch answers a standard request, even when more candidates
   remain. For an exhaustive request, assess useful batches while taking
   `next` within the budget. Use one `expand` on the same run after a
   completed first scan only when a concrete vocabulary gap warrants it.
   Execute MCP calls sequentially. Correct an invalid call once. Start a
   fresh run for changed criteria or a run that can no longer be used.
5. Publish only fresh `finish.matches` as AgentMender-verified. Include the
   evidence and coverage. An incomplete result can still contain useful
   verified matches. If finish has no matches, say that AgentMender could
   not verify a match. If there is no finish, report that verification
   did not complete. Attribute any separately requested evidence on its own.
   Use `next_offset` with `revision` for additional verified result pages.
   Supply `preferred_session_id` only for an explicit `single_best` with
   a grounded comparison. If Luna is unavailable, finish an active run
   with `luna_unavailable:true` and disclose the limitation.

An intentional early finish, a work budget, unassessed or insufficient
evidence, stale sources and expired runs have different remedies. Early
standard finish leaves unsearched coverage. Use the reported progress
and stop reason. A stale result is excluded at publication. An expired or
foreign run cannot resume. Logical Partial history limits absence claims
even after traversal. A later independent search does not depend on the
previous run.

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

The resolver retains up to four runs for ten minutes with eight minutes for
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
