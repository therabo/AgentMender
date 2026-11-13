# MCP interface

## Conversational resolution

The read-only `session_find` tool in `--session-search` mode has one
required argument, `query`. AgentMender starts a job that runs both Luna
stages, Rust retrieval and publication. `session_find_status`,
`session_find_result` and `session_find_cancel` take the returned job ID.
Results include separate confirmed and plausible unconfirmed pages,
signed context references and traversal, evaluation and source coverage.
The typed `session_resolve` contract remains available for diagnostics.
See the [search and installation contract](intent-aware-resolution.md)
and [ADR-0037](adr/0037-autonomous-mcp-session-find.md).

## Native deletion request

In `--session-search` mode, resolve an ordinal or label to one result row from
`session_find_result`. Call `session_delete` with that row's canonical
`native_session_id` and one `evidence_ref` from its `evidence_refs` array. The
server checks the signed evidence against the exact source in the configured
Codex root and excludes the apparent caller and configured protections.

After validation, AgentMender sends one MCP form elicitation with the title, UUID
and a warning that Codex can delete spawned subagents. Use a client that
supports form elicitation and routes it to a human. For Codex this requires an
interactive approval policy that allows MCP elicitation; see the
[Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference).
The MCP client displays the accept and cancel controls. Declining or cancelling
returns `{"error":"cancelled"}`; unavailable confirmation returns
`{"error":"confirmation_unavailable"}`. Neither starts Codex. The tool takes
no approval argument. Caller metadata and signed evidence identify the selection;
neither authorizes deletion. The bundled Codex App Server routed accept and
decline for AgentMender in a disposable test. The Desktop GUI has not been
verified.

Accepting runs `codex delete <UUID> --force` once with the configured executable
and original provider root as `CODEX_HOME`. After dispatch the result has just
`status` and `native_session_id`: exit zero is `succeeded`; a launch error,
a 30-second execution timeout or nonzero exit is `failed`. A failure may follow
partial effects. No retry or artifact verification follows. See
[ADR-0038](adr/0038-native-codex-delete-admission.md).

## P0-SR4 ordinary discovery

Use `session_discover` in `--session-search` mode for ordinary browsing and
possible deletion candidates. Its search inputs omit `purpose` and caller
arguments. It excludes the apparent caller before reading and ranking eligible
sessions. `session_context` accepts its caller-scoped evidence in the originating
conversation. Missing or unresolved caller metadata returns
`discovery_caller_unavailable`. Outputs label metadata-derived exclusion and
carry no removal eligibility. [Search and context contract](session-search.md).

## P0-SR3 management default and resident history

`session_search` now defaults to management and returns zero candidates before
source reads without an authenticated host route. Developer diagnostics require
explicit `purpose:"diagnostic"`; there is no fallback. `_meta.threadId` is not
admitted caller authority. [Identity contract](mcp-caller-identity.md).

`session_context.window.direction:"start"` begins at the first available supported
message. Signed replay/forward/backward continuations traverse bounded source
operations; `work` and `source_coverage` separate resource work, physical EOF and
logical completeness. [Traversal contract](progressive-history.md). Refresh only
AgentMender to load the new parent. The retained parent can read compatible small
sources but cannot attest SR3 resident traversal/defaults. No Codex restart.

## P0-SR2 history reads

The same search/context tools now consume bounded compressed rollouts through
the existing worker. Evidence `format` identifies legacy/paginated Zstandard;
positions are decoded, while the sealed source revision/size bind physical bytes.
No new tool, import endpoint, host attestation or destructive authority is added.
Search token input is bounded at32KiB so emitted evidence-bearing cursors can be
read back. See [native history and actual-parent distinction](native-history.md).

## P0-SR1 refinement

The existing tools gain query-scoped freshness, completed-message progress and closed path-free source-failure codes; see [ADR-0028](adr/0028-search-reliability-and-scoped-freshness.md). Hits add content_coverage; legacy aggregate counters remain. A long-running parent MCP process can reuse newly built capture workers, but only refreshing the AgentMender connection loads its additive parent schema and no-progress guard. A rebuilt binary alone is not proof of refreshed client catalog. Never restart Codex automatically.

## Real-session incremental read mode

Historical real conversational metadata search is accepted (37 MISSION1 records,
Partial). Current [ADR-0027](adr/0027-intelligent-session-retrieval.md) adds structured
queries, incremental supported content, context navigation and optional private
FTS5 caching. [The canonical contract](session-search.md) defines actual inputs,
formats, privacy and limits. Live read-only discovery and independently verified
management routing have separate acceptance criteria.

Normal Codex browsing uses `agentmender --provider codex mcp --session-search`.
No fixture, historical executable, protected caller UUID or state root is required.
The explicit mode locates the existing bundled macOS executable without PATH;
trusted root/executable flags can override selection. See the
[quick start](p0-c1-manual-acceptance.md) and [ADR-0024](adr/0024-adaptive-read-only-metadata.md).

The discovery catalog includes autonomous `session_find` and its job tools,
`session_resolve`, `session_discover` and `session_context`.
`session_search` preserves the
legacy metadata request and adds structured filters, content, purpose, project,
excerpts and continuation. Guaranteed `purpose=management` requires enforceable
host binding, currently unavailable; file-only registration cannot admit it.
It returns `caller_identity_required` with zero candidates before any
provider read. Diagnostic responses have no caller guarantee. `session_context` opens a canonical
UUID in discovery mode and returns a signed target and bounded window. Continue
with that target and the signed evidence reference. Each navigation call rechecks
source and caller binding. These advanced reads are available in `--session-search` mode.
`preview_removal` returns a blocked compatibility
assessment rather than a Ready plan. Its output schema admits either that typed
assessment or the historical supported preview. A read-mode assessment has no plan
or digest and `execution_permitted: false`. Historical preparation, status and
execution handoff fail with `read_only_removal_unavailable`, without opening
owned state. The separate `session_delete` tool uses the native flow above.
`--session-search` and `--session-removal` are mutually exclusive; default remains
eight tools. Read-only metadata availability never transfers mutation authority.

Search adds optional producer and parent UUID fields plus a capability/coverage
header. Titles prefer native display names; unavailable/oversized optional labels
stay null. Legacy metadata never exports first-message bodies. Explicit content
search returns at most two 320-character known visible excerpts/hit and one
512-character/1,024-byte legacy context message, marked untrusted. Optional context
windows return at most20 messages/16KiB with source-bound chunk continuation;
there is no unbounded transcript-dump endpoint.
The legacy selected-root recent inventory is at most 500 records/96 KiB; advanced
search uses explicit 100-session batches and the limits in the search contract; all existing MCP
budgets and Partial/ambiguity semantics remain. Native methods advertised by the
installed schema are not equivalent to admitted effects. The separate historical
`providers`/scan profile catalog keeps its prior meanings.

## P0-C1 separately admitted removal mode

Default launch retains the original eight read-only tools and privacy behavior
specified below. Explicit session-removal mode shares the six-tool catalog, but advanced content/context
reads are unavailable there; its five original tools follow
[ADR-0023](adr/0023-conversational-primary-removal-handoff.md). Read the
[setup, exact compatibility, registration and human workflow](conversational-codex-removal.md)
before enabling it. P0-C1 remains OPEN pending actual client/human acceptance.

| Tool | Strict input | Capability |
|---|---|---|
| `session_search` | Legacy query/archive or advanced structured/content/purpose/project/page/cursor/excerpts; see [schema](session-search.md) | Diagnostic metadata or bounded content; guaranteed mode requires independent binding before all matching/counts/paging |
| `session_context` | exact opaque `target`, sealed `evidence_ref`, optional `window` | Fresh bounded message/window/occurrences in read mode; offsets apply only to the authorized parent message |
| `preview_removal` | opaque session `target` | Fresh protected exact stable-profile schema-2 primary plan and original digest |
| `prepare_removal` | `target`, `expected_plan_digest` (`sha256:` + 64 lowercase hex) | Prepare the existing durable review/recovery transaction; no grant |
| `removal_status` | optional `transaction_id` (64 lowercase hex) | Matching configured-root durable status/results across restart; empty input lists attempts |
| `request_removal_execution` | `transaction_id` | Status plus trusted external-terminal handoff; never dispatch or consume |

Only preparation writes AgentMender-owned metadata. No tool grants/revokes approval,
imports proofs, selects roots/principals/clocks or accepts `confirm`/caller IDs.
The host fixes exact roots/binary and independently registered protected management
UUIDs. Human approval and execution stay external because a same-user shell/PTY
agent is not an independent human channel. Search references grant no authority.
Search metadata is opt-in; existing default projections still hide titles/IDs/time.
All tools share the existing frame, response, busy, deadline, cancellation and drain
bounds. Oversize search results require a narrower query; legacy requests have no silent truncation; advanced search uses explicit
continuation and Partial coverage. The default read-only interface reference follows.

## Launch the current read-only server

Build with `cargo build --locked --workspace`, then configure your MCP client to
launch the local executable with `mcp`:

```sh
/absolute/path/to/agentmender mcp
```

The only transport is stdio. Stdout contains newline-delimited MCP JSON-RPC frames;
diagnostics use sanitized stderr. Server identity is `agentmender`, application
version `0.1.0`. The official rmcp SDK negotiates MCP independently of provider
versions: modern `2026-07-28` discovery/per-request metadata and legacy
`2025-11-25` initialization are tested. No HTTP, OAuth or remote service exists.

Trusted launch flags configure provider roots/executables, as for CLI discovery.
For example, select one provider with `--provider codex`. Those values are fixed
before requests are handled; tools cannot change roots, executables, fixtures,
environment, process policies or the internal evidence worker. Existing CLI root
defaults and provider isolation apply. A deterministic fixture launch is:

```sh
agentmender --provider codex --fixture /absolute/fixtures/codex/known mcp
agentmender --provider claude --fixture /absolute/fixtures/claude/4.29.0-observed --desktop-fixture mcp
```

The workspace has eight crates after M6. CLI composes concrete adapters locally inside each
admitted worker; `mcp-server` depends internally only on application and domain.
Application dispatch reuses DiscoveryService, provider ports and the existing
graph/integrity/planning evaluators. No provider parsing or filesystem inspection
lives in MCP. See [ADR-0010](adr/0010-readonly-mcp-boundary.md).

## Tools

All eight tools are read-only and non-destructive. Their input/output schemas are
explicit typed JSON Schema. Unknown fields, bad enums/types, malformed tokens,
invalid budgets and invalid cursors are rejected before provider dispatch.

| Tool | Input | Result / application use case |
|---|---|---|
| `providers` | `{}` | Configured provider availability, versions, compatibility, installation surfaces and read-only capabilities; `providers` |
| `workspace_scan` | Optional provider, scope, page | Workspace/project/session/artifact metadata and literal references; `scan` |
| `inspect` | Target reference, optional provider/scope/page | One observed entity's metadata; shared graph-backed target resolver |
| `graph` | Optional provider, scope, page | Nodes, observations, edges, roots, evidence rules, diagnostics and protection; `graph` |
| `explain` | Target reference, optional provider/scope/page | Relationships, raw/effective evidence confidence, rules and ownership blockers; `explain` through the full-identity selector |
| `verify_integrity` | Optional provider, scope, page | Status, findings, blockers, residuals and proof context; `verify` |
| `plan_lifecycle` | Target, intent, optional provider/scope/page | Immutable Archive/Unarchive/SoftDelete/Purge intention; `plan` through the same-observation selector |
| `repair_plan` | Target, optional provider/scope/page | Existing pure Repair intention, currently Unsupported; the same planner, without a repair engine |

`inspect` returns entity metadata; `explain` adds graph/evidence analysis. They share
target resolution, not duplicate provider logic. Targets are structured
`{provider, class, token}` references returned by this server, for example from
`workspace_scan` or `graph`. Classes distinguish sessions, projects, artifacts and
other graph entities. The token binds the full original identity, including hidden
provider root. No arbitrary pathname is a target. Multiple matching observations
fail explicitly; missing state under incomplete coverage returns `target_unavailable`.

Example requests use arguments such as:

```json
{"provider":"codex","scope":{"max_entries":1000,"include_archived":true},"page":{"page_size":50}}
```

```json
{"target":{"provider":"codex","class":"session","token":"<token returned by this process>"},"intent":"archive"}
```

`scope.max_entries` defaults to 1000 and accepts 1–10000; archived sessions are
included by default. There is no project-path filter in MCP. CLI scope flags belong
to human CLI commands; MCP query scope is the typed tool argument. `page.page_size`
defaults to 50 and accepts 1–100. Provider selection is restricted to configured
Codex/Claude adapters. A configured-provider mismatch returns `provider_unsupported`.

## Results and pagination

Success returns a typed PageView in both `structuredContent` and serialized JSON
text content for client compatibility. Errors return a typed `error` code through
the same two content forms with `isError: true`; malformed tool arguments use the
protocol's invalid-parameters error. Neither form contains raw parser/provider prose.

Contexts retain presence, exact compatibility status, interface stability,
capabilities, installation/version provenance, provider evidence status, discovery
evidence status, coverage and diagnostics as separate facts. `complete`, `partial`
and `unavailable` concern the declared discovery boundary; absence from a result
never establishes provider absence, ownership or exclusivity.

Each record has `namespace_ref` identifying its provider/root context. Rule and
evidence references are scoped to that namespace; findings retain subject and
supporting observations/relationships. Verification summaries associate each status
with its namespace. Original conflicts and variant counts survive projection.
Private fields are omitted deliberately; conclusions are computed on original
evidence before projection, never reconstructed from redacted values.

Pages repeat context, status/blocker/plan headers and record counts. Follow
`next_cursor` with the same tool and normalized arguments. Cursors authenticate the
tool, arguments, offset and whole original observed result. A changed result yields
`stale_cursor`; restart the query. No snapshot/plan/result database or cache exists.
`presentation_complete` means every record has been returned; it is independent of
provider discovery completeness. Unknown compatibility remains `audit_only_unknown`.

Input frames are limited to 64 KiB. A preflight rejects duplicate JSON keys before
the SDK can collapse them; malformed/oversized duplicate frames close the transport
without invoking a provider. SDK framing and negotiation remain authoritative.
Typed view pages have an 80 KiB budget; complete tool frames, including text fallback,
structured output and JSON-RPC overhead, must fit 256 KiB. Oversized records or scalar
headers return `response_too_large`, without silently dropping facts. Retry with a
smaller observation budget where necessary. The fixed tool registry is bounded too.

Stable tool errors include `busy`, `cancelled`, `deadline_exceeded`, `query_failed`,
`target_not_observed`, `ambiguous_target`, `target_unavailable`, `provider_not_detected`,
`provider_unsupported`, `compatibility_blocked`, `capability_unavailable`, `caller_identity_required`, `invalid_scope`,
`stale_cursor`, `invalid_cursor` and `response_too_large`. Provider failures within
multi-provider reports retain provider and typed diagnostic codes. Unknown integrity
and blocked/unsupported plans are successful observations, not protocol failures.

## Privacy and authority

MCP exports metadata, categorical evidence/protection facts, sizes, recognized
version/profile/rule names and opaque references. It excludes local root/path names,
native/local IDs (including embedded paths), activity timestamps, evidence locators,
untrusted diagnostic/error prose, prompt/response/source-code bodies, credentials,
cookies, tokens, private configuration, raw transcripts and raw databases. Unknown
version/profile/rule values receive opaque references rather than verbatim export.
CLI JSON retains its existing local disclosure behavior.

HMAC-SHA256 references use a fresh OS-generated process key. They correlate within
one process and expire at restart. There is no identity registry. These references
are privacy-preserving lookup identifiers, never authority or approval tokens.

A PlanView preserves the original PlanDigest and StateFingerprint with
`digest_basis: original_unredacted_domain_plan_v1`. It is a privacy projection, not
canonical LifecyclePlan wire data; hashing the view does not reproduce those hashes.
It preserves status, blockers, pre/postconditions, retained/effect roles, estimates,
capability context and future requirements. `planning_only` is true, `executed` is
false, and independent approval remains required. Current real profiles remain
Blocked/Unsupported; synthetic Ready fixtures test mapping without granting authority.

A digest identifies content. It is not a signature, capability token, approval or
permission to execute. Ready is not approved. MCP is an interface and is not the
authorization authority. Tool annotations describe behavior and enforce no permission.
No execute/approve/archive/delete/purge/restore/repair execution tool or arbitrary
command/filesystem/SQL primitive exists. M6 durable transaction storage is separate
from the default eight-tool interface and is neither exposed nor initialized by
it. The explicit removal mode above may prepare existing durable attempts; the
real-session read mode cannot open owned state.

## Cancellation and limitations

One blocking application worker is admitted; concurrent requests return `busy`
without a waiting queue. Each request has a fixed 120-second deadline. Cancellation
sets cooperative application control and prevents late delivery. The worker keeps
its admission permit and owns adapters, child guards and disposable evidence state
until cleanup actually finishes. EOF waits for that cleanup.

Cancellation is checked at synchronous observation boundaries. An in-progress
provider call drains under its existing bounds; blocking filesystem syscalls are not
instantly interruptible. No started blocking job is claimed to be forcibly aborted.
SDK logs have no raw-content subscriber. Provider processes retain their existing
OS enforcement, bounded output and child cleanup; no isolation fallback is relaxed.

Tools alone serve M5's fresh queries and bounded views. Resources would duplicate
these reads or require storage/lifetime semantics without an application need.
There is no persistent `plans/{digest}` URI, subscription or transaction store.

## Verification

```sh
cargo test --locked -p agentmender-mcp-server
cargo test --locked -p agentmender --test mcp
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
git diff --check
```

Normal tests use synthetic/sanitized fixtures and fresh temporary roots. They cover
modern/legacy MCP, strict schemas/inputs, fixed tools, opaque identities, privacy,
original plan commitments, Ready/Blocked/Unsupported, uncertainty, pagination/limits,
stale cursors, cancellation/deadlines/busy/drain and unchanged fixture bytes. Installed
providers are not required. Remote Linux/macOS/Windows evidence is recorded separately
for the exact final commit; local tests do not substitute for that gate.

## Intelligent search extension (ADR-0027)

The same `session_search` accepts optional typed `structured` predicates, mode,
sort and coverage policy. `query_interpretation.filter_json` returns the validated
canonical condition; verified and unresolved results are distinct. Follow sealed
cursors to continue source/inventory work, including empty pages. Ranking is
scanned-page-local and all provider gaps remain explicit. `session_context`
accepts optional bounded `window` with around/forward/backward/turn/occurrences,
returns individual source-bound evidence refs and continuation suggestions.
See [complete schemas/examples/limits](session-search.md).

Index maintenance stays CLI-only: [private index operations](full-text-index.md).
No new MCP filesystem, SQL, destructive or approval endpoint is added. The
optional persistent cache explicitly stores safe message text and terms.
Production guaranteed management search refuses because the installed transport
cannot attest per-chat routing. A root-owned file alone cannot grant that status;
diagnostic mode has no automatic caller guarantee. [Luna](luna-session-relevance.md)
can analyze bounded evidence but cannot override conditions or authorize effects.
