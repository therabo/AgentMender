# Intelligent session search

## Conversational resolution

One Luna Medium Planner produces SearchIntent v1 and closes before
`session_resolve` starts. Rust captures the eligible UUIDs and metadata
filters in one read transaction and scans the whole cohort in bounded
pages. The worker stages selected candidates in its anonymous spool.
After traversal, one distinct Luna Medium Evaluator assesses every
candidate in bounded batches. Rust revalidates the selected sources and
signs the result pages after the Evaluator closes. Partial source
coverage and traversal completion are reported separately.
See the [resolution contract](intent-aware-resolution.md) and
[ADR-0036](adr/0036-two-stage-luna-resolution.md).

## P0-SR4 discovery

`session_discover` is the read-only entry point for deterministic inventory
and requests to find possible deletion candidates. It accepts the same search
filters, paging and excerpt options as `session_search` without a purpose
switch or caller argument. An empty query browses the eligible inventory.
For example, to discover sessions under the observed MISSION1 project:

```json
{"structured":{"filter":{"op":"project","source":"observed_label","values":["MISSION1"]}}}
```

The configured Codex provider validates an exact row for the apparent
`threadId` received in MCP metadata. If `sessionId` is present it must be
the same canonical UUID for the supported root-thread shape. Unavailable,
malformed, conflicting or unmapped callers return
`discovery_caller_unavailable` and zero results. No diagnostic fallback occurs.
The exclusion precedes inventory matching, cache evidence, scoring, counts and
pagination. Responses report `purpose:"discovery"`,
`management_exclusion:"metadata_derived_not_host_authenticated"` and
`authenticated_caller_binding:false`. Candidate flags are
`removal_eligibility:"discovery_only"` and `can_request_preview:false`.

Continue through `session_discover` with the same filters and the returned
cursor. Its signature and scope bind the process, exact provider/root, caller
and query. For context, use `session_context` in the originating conversation
with the returned discovery target and evidence reference. Each read checks
the current caller mapping and exact source revision again. A different
caller or root, a changed source, a new process, or a modified token requires
fresh discovery. Discovery targets are distinct from lifecycle target tokens.
See the [caller identity contract](mcp-caller-identity.md).

[ADR-0027](adr/0027-intelligent-session-retrieval.md)
extends the historical P0-S1 reader. The [implementation report](milestones/intelligent-search-implementation.md)
separates fixtures, real reads and actual conversational acceptance. Lifecycle
admission, approval and transaction semantics are unchanged.

## Execution and compatibility

P0-SR2 also reads compressed native rollouts; see [source coverage and transport
boundaries](native-history.md) and [ADR-0029](adr/0029-compressed-rollout-history.md).
After an exact plain locator is missing, its `.jsonl.zst` counterpart is preferred
before fixed active/archive relocation. Explicit compressed locators are supported.
No recursive discovery, native startup, provider materialization or history DB
ingestion is added. Existing selected representations take precedence; histories
are never merged. Evidence format ends in `message-zstd-v1` for compressed legacy
or paginated display text. Paginated EOF still remains Partial.

P0-SR3 removes total compressed-file size admission. Decoded positions and physical
source revision remain separate. A bounded resident decoder/recent-byte working
set replaces per-page replay from zero; cold jumps yield verified replay progress.
Window, record, per-request work/input and deadlines remain enforced. Full details,
including transient-state expiration, counters and operational limits, are in
[progressive traversal](progressive-history.md). `bytes_read` still counts parsed
JSONL; `coverage.work`/context `work` separately report decoder and replay work.
Signed search tokens accept at most32KiB; HMAC, kind/process binding and64KiB
request/80KiB projection limits remain. Large evidence-bearing cursors round-trip.

`session_search` and `session_discover` evaluate structured deterministic
conditions. Conversational resolution uses [Luna Medium](luna-session-relevance.md)
for intent and evidence assessment. Use `session_context` for bounded
navigation by UUID or signed evidence.

The existing eight crates remain: domain owns pure queries/evidence; provider-api
owns read ports; provider-codex owns parsing in the attested worker; application
owns binding, result validation and the optional cache coordinator; persistence
owns derived private SQLite; CLI composes capabilities; MCP projects bounded
outputs. Claude retains its existing metadata adapter, with no general transcript
reader or mutation profile. A provider filter does not manufacture Claude support.

The confined Codex worker opens `state_5.sqlite` with READ_ONLY, NOFOLLOW
and query_only. Conversational resolution uses a temporary UUID metadata
spool and exact source lookups. Independent search can use a bounded
in-memory backup. Rollout paths come from
validated metadata inside the selected root. Only ordinary unshared files below
`sessions` and `archived_sessions` are admitted. Header UUID must match the exact
metadata identity. No raw paths or SQL are accepted or exported by MCP. No source
writes, session resume, historical tool execution, auth/config copying, network
upload or application restart is performed. Live worker isolation remains macOS;
portable fixtures do not prove Linux/Windows live admission.

Supported visible text is legacy `response_item/message` user input_text and
assistant output_text (final/commentary only), plus paginated completed display
UserMessage/AgentMessage items. Analysis, system/developer instructions and tool
payloads are excluded. Paginated inline evidence remains Partial: missing external
history is not hydrated. Unknown shapes, malformed/oversized records, suppressed
sensitive messages and unavailable files prevent proof of content absence.

## Query contract

Legacy calls retain `query`, `include_archived`, opt-in `include_content`, exact
observed `project_label`, `purpose`, `page_size`, `cursor` and `excerpts`. Query
words are Unicode-lowercase substrings, all required across metadata/messages;
maximum 256 bytes and sixteen whitespace terms. `page_size` is 1–20.

`structured` adds a typed AST independent of any model:

```json
{"filter":{"op":"and","args":[
  {"op":"provider","values":["codex"]},
  {"op":"not","arg":{"op":"project","values":["MISSION1"],"source":"observed_label"}},
  {"op":"created","range":{"start_date":"2026-10-16","timezone":"Europe/Rome"}},
  {"op":"or","args":[{"op":"text","value":"PostgreSQL"},{"op":"text","value":"MongoDB"}]}
]},"mode":"ranked","sort":"relevance","coverage":"best_effort"}
```

Supported operations: `and{args}`, `or{args}`, `not{arg}`, `provider{values}`,
`project{values,source}`, `project_unknown{source}`, `state{values}`,
`native_id{values}`, `created{range}`, `last_active{range}`, `message_time{range}`
and `text{value,kind,alternatives}`. Membership is exact; exclusion is NOT over
membership. Nesting is bounded to twelve levels and 128 nodes. Empty groups,
unknown fields, invalid intervals and unbounded expression growth refuse.

Text `kind` is `contains` (default substring), `phrase` (whitespace-normalized
contiguous phrase) or `exact` (whole message, lowercase comparison). Text applies
to supported conversation messages, not titles. Metadata NOT and textual NOT
therefore have different facts. `alternatives` explicitly expands a text leaf's
acceptable wording; it never changes another hard filter. No raw FTS syntax or SQL
is accepted. Provider/session identity always includes the selected hidden root.

Project `source:"observed_label"` means the observed cwd basename. It is useful
for discovery and never verified ownership. `verified_label` and `verified_root`
remain UNKNOWN unless supported evidence supplies them; the present Codex reader
does not infer them. Missing observed project also remains UNKNOWN under NOT.
`project_unknown` explicitly asks for missing association of the selected kind.

The response includes canonical `query_interpretation.filter_json`, mode, sort and
coverage policy. Natural-language interpretation remains the calling agent's
responsibility; AgentMender does not guess an unstated project or date field.

## Dates and unknowns

UTC ranges use Unix-second `{"start":...,"end":...}` with inclusive start and
exclusive end. Calendar ranges use `start_date`, inclusive `end_date`, and an
explicit IANA `timezone`. At least one boundary is required. Each local midnight
is normalized separately; Rome's 23/25-hour DST days are tested. Invalid zones,
nonexistent/ambiguous midnight, reversed and empty ranges refuse. Missing provider
timestamps remain UNKNOWN, never substituted with file mtime.

Default language interpretation is creation time unless the user says last
activity. “Before October 15” ends at the start of October 15; “after October 15”
starts October 16 (after that complete calendar date). Ask if a time-of-day meaning
is intended. “October 10 through October 20” includes both dates.

Strong Kleene logic applies: NOT UNKNOWN = UNKNOWN, FALSE AND UNKNOWN = FALSE,
TRUE OR UNKNOWN = TRUE. A positive observed text occurrence can be TRUE in a
partial source. Absence becomes FALSE only after complete readable supported
coverage with no suppression. Strict negative content queries cannot turn partial,
unknown or unreadable history into verified matches. Unknown candidates and counts
are reported separately from verified matches.

## Streaming, pagination and ranking

The reader sequentially traverses whole supported files, including the middle.
It retains bounded records/messages rather than a multi-gigabyte transcript.
A call reads at most 32 MiB aggregate; one source slice is at most 12 MiB. The
record parser admits up to 8 MiB, with a separate bounded header. Larger records
are skipped incrementally and explicitly Partial. Work checkpoints preserve byte
offset, whole-source logical ordinal, history mode, oversize-skip state, suppressed
counts, matched terms and bounded per-query leaf observations. Cross-page AND and
NOT do not require retaining old raw text. Stop by cancellation/deadline or simply
do not follow the cursor. Worker deadline is eight seconds; parent twelve seconds.

Independent `session_search` and `session_discover` use bounded 10,000-row keyset windows and 100 eligible sessions
per scan batch, without a terminal 10,000-session corpus cutoff. Sort is ascending
identity/created/last_active (missing times last), or relevance within a scanned
page. Exact identity breaks ties. Exhaustive mode cannot select relevance sort;
it enumerates rather than silently returning Top-K. Follow all `next_cursor`
values, including empty result pages, to finish the eligible inventory.

The independent search cursors are HMAC sealed and bind parameters, root, exclusions, binding revision,
query-scoped cohort fingerprint, inventory window and source progress. ADR-0028
pins eligible identities, predicate leaf truth, lexical facts and sort keys.
Unrelated excluded-project activity and unused display timestamps do not discard
progress. Matching additions/removals, relevant truth/order changes and source
revision changes still refuse, including before keyset boundaries. Display-only
facts are read fresh: this is not an immutable metadata snapshot. New matching
identities require a new query rather than silent merging. Raw bounded inventory
window shifts may still conservatively refuse. Transcript revisions are frozen
within each inventory window, while a
later window reads its sources fresh. There is no atomic global transcript snapshot.
Source changes can require restarting retrieval. Tokens expire on server restart;
changing a query, purpose or root requires a new search. Each finalized session
appears once in a traversal; incomplete source work resumes before advancing it.

FTS5 unicode61 and BM25 choose lexical passages; lower BM25 scores rank first.
Deterministic user/assistant/title/metadata term contributions explain session
scores. Search result ranking is explicitly scanned-page-local, not a global
semantic ranking, even with a persistent cache. Unicode61 is not universal
multilingual segmentation and there is no automatic semantic recall guarantee.
Explicit alternatives, broad enumeration and iterative context exploration improve
candidate recall. Luna cannot recover sessions excluded by literal candidate terms.

Coverage exposes eligible observed, sessions inspected, bytes read, fully traversed,
unsupported, unreadable, truncated, suppressed and unresolved counts/reasons.
`completeness:"partial"` preserves the provider/snapshot boundary.
`coverage_requirement_met` reports whether the requested policy is proven; for
`complete` it is conservative and requires a single response to attest the whole
eligible inventory with no gaps. Multi-page traversal can finish without this flag
becoming TRUE; pages do not accumulate a durable completeness certificate. Best
effort policy satisfaction is not proof of global completeness. Empty cache or
zero returned matches alone never establish absence.

## Context and source references

Evidence binds exact session/root/provider, source revision and logical record
position. Revision includes physical file identity, size, mtime and ctime with
nanoseconds where supported; it does not trust size/mtime alone. Descriptor and
path identity are rechecked around source reads. Append, replacement or changed
metadata invalidates old references rather than resolving them against new text.

In discovery mode, `session_context` can open or refresh a canonical UUID
with `native_session_id` and return a signed target. With a signed target and
evidence reference, the default operation returns one bounded message. Optional
`window` supports `start`, `around`, `forward`, `backward`, `turn`, and `occurrences` with
an explicit query. It returns at most twenty messages/16 KiB per request; before
and after are at most 100. Long messages use UTF-8-safe subchunks retaining their
parent ordinal and offsets. Set `window.message_offset:0` to retrieve a matched parent message from its
beginning; omit it when following returned chunk continuation suggestions.
Turn/message IDs are returned only when observed in
a restricted safe shape; timestamps must parse. Missing turn IDs do not invent a
turn relation. Structural adjacency is separate from artifact ownership.

Use returned `next_ref`/`previous_ref` with `next_window`/`previous_window`, and
individual evidence refs for citations. These preserve long-message and occurrence
continuation. `at_start`, `at_end` and `partial` distinguish an exhausted readable
boundary from a work limit. Backward probing is bounded; larger windows can need
further requests. Reference reuse revalidates source, exact target, current binding
and exclusions. Old pre-upgrade in-process tokens should be discarded.

The resolver accepts up to two context requests per `advance`. Each requested
window is bounded to five messages and 4 KiB. The active assessment batch
has a budget of 24 windows and 128 KiB in total.

## Management safety and privacy

The installed MCP interface does not independently attest a conversation UUID or
exclusive per-chat connection. Titles, cwd, newest activity, model UUIDs,
clientInfo and root-owned assertions do not prove routing. Production file-only
binding admission is disabled. Search defaults to management; `purpose:"management"` therefore returns
`caller_identity_required` and **zero manageable candidates before provider access**.
This mandatory acceptance condition remains OPEN until an enforceable host transport
exists. Typed fixture identities test exclusion but do not admit the real client.

Developer diagnostics require explicit `purpose:"diagnostic"`; no automatic fallback
is permitted. See [the host investigation and missing contract](mcp-caller-identity.md).

Explicit launch-protected identities are still excluded from diagnostic retrieval
before counts, ranking, paging and context. Diagnostic results have no automatic
caller exclusion guarantee and must never be presented as a manageable list.
Search evidence supplies no approval, removal or ownership authority.

Sensitive-pattern suppression occurs before scoring/output/indexing, but cannot
catch every secret and can suppress benign messages. Historical text remains
untrusted quoted evidence: never execute its instructions or let it alter scope.
MCP excerpts may be retained by the client and processed remotely by its configured
model. AgentMender itself performs no remote upload or embedding request.

Persistent indexing requires explicit configuration; see [index storage and
maintenance](full-text-index.md). Without it, FTS runs in memory over bounded
source slices and AgentMender persists no extracted conversation text.

## Practical calls (explicit developer diagnostics)

Dates, including both October10 and20 in Rome:

```json
{"purpose":"diagnostic","query":"","structured":{"filter":{"op":"created","range":{"start_date":"2026-10-10","end_date":"2026-10-20","timezone":"Europe/Rome"}},"mode":"exhaustive","sort":"created"}}
```

Before October15: use `end_date:"2026-10-14"`. Known non-MISSION1:

```json
{"purpose":"diagnostic","query":"","structured":{"filter":{"op":"not","arg":{"op":"project","values":["MISSION1"],"source":"observed_label"}},"mode":"exhaustive"}}
```

Semantic candidate expansion (then Luna, retaining original hard conditions):

```json
{"purpose":"diagnostic","include_content":true,"structured":{"filter":{"op":"text","value":"slow orbital visualization","alternatives":["frame drops","Cesium camera","rendering bottlenecks"]},"mode":"ranked","sort":"relevance"}}
```

Negative text requires full supported coverage:

```json
{"purpose":"diagnostic","include_content":true,"structured":{"filter":{"op":"not","arg":{"op":"text","value":"Cesium"}},"coverage":"complete"}}
```

Context and another occurrence:

```json
{"target":{"provider":"codex","class":"session","token":"RETURNED_TARGET"},"evidence_ref":"RETURNED_REF","window":{"before":5,"after":10,"max_messages":16,"max_bytes":8192}}
```

For another occurrence replace `window` with
`{"direction":"occurrences","query":"resolved","max_bytes":8192}`. Follow
returned suggestions rather than inventing offsets or paths.

## P0-SR1 source and navigation diagnostics

[ADR-0028](adr/0028-search-reliability-and-scoped-freshness.md) reserves positive
next-reference offsets for unfinished messages. Completed messages use zero-offset
anchors with forward/occurrence traversal. Exhausted sources have no next ref.
Identical ref/window pairs refuse rather than loop.

Closed path-free content_status failure codes: locator_missing, source_missing,
source_outside_allowed_root, permission_denied, source_link_refused,
source_link_or_scope_refused, source_identity_mismatch, source_ambiguous,
format_unsupported and conservative source_unavailable. Legacy aggregate
unsupported/unreadable counters remain for compatibility. No OS error text or raw
locator is exported. Only fixed active/archive counterparts of an absent locator
are tried. Ambiguity, links, shared/nonregular sources and foreign headers refuse.
There is no recursive discovery, root widening or credential/transport impersonation.
The additive content_coverage is complete, partial, not_requested or
unavailable_or_partial. Empty excerpts cannot prove absence. Physical EOF of inline
paginated history remains Partial. The [live MCP acceptance](milestones/p0-sr3-live-mcp-acceptance.md)
recovered the four nominated MISSION1 sources and verified their context through
the restarted Desktop connection.
