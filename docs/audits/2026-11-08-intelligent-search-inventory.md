# Intelligent search architecture inventory

Date: 2026-11-08 (Europe/Rome). Baseline: main
`83aeccb6041f574af4fe006313742b1053650aee`; local checkout clean at audit start.
This inventory records the execution paths and constraints used for ADR-0027.
Historical milestone reports retain their original acceptance levels. Release/CI
and actual client observations belong in the implementation report; source review
alone does not establish those gates.

## Eight-crate map and execution path

| Crate | Actual source responsibilities | Search reuse/extension |
|---|---|---|
| domain | `lib.rs` supplies provider-scoped identities and states; `session_metadata.rs` carries optional labels/timestamps/parents; `session_search.rs` has requests, bounded evidence, coverage, continuation, secret suppression and lexical relevance. Graph/integrity/planning/approval/transaction modules are pure observations and versioned safety models. | Reuse all identities, read evidence and scope hashing. Add pure `session_query.rs` for strict Boolean predicates, UTC/calendar ranges, Kleene logic and bounded incremental atom observations. Add source/message/navigation value types without ownership edges. |
| provider-api | `lib.rs` defines `SessionMetadataReadPort` and `SessionSearchReadPort`, separate from effect handoffs. `filesystem.rs` provides no-follow source observations; `evidence.rs` owns disposable evidence sandboxes; `process.rs` and reviewed `macos_sandbox.rs` implement process confinement and attestation. | Extend the existing read port for bounded context windows. Retain attested source write denial and verified-root opens; no generic SQL/path/executor capability. |
| application | `session_search.rs` gates management binding before any reader access, hashes query/root/exclusions/binding into scope, validates returned provider/root/class/exclusions and checks cancellation before/after a bounded call. `conversation.rs` separates evidence search/context from legacy metadata search and schema-2 preparation/status. `lifecycle.rs`/`transactions.rs` orchestrate existing immutable effects and recovery. | Reuse the service, control object and fixed ports. Revalidate structured metadata and accumulated text observations after retrieval. Keep diagnostic results separate from management candidates and index work separate from transaction authority. |
| provider-codex | `metadata.rs` validates fixed SQLite columns and snapshots read-only source into bounded memory. `content.rs` inventories exact identities, validates rollout location/header, parses two supported visible-message representations, scores messages and seals source observations. `capture.rs`/`scoped_capture.rs` reuse kernel-attested workers. Native/scoped/desktop modules implement separately gated historical compatibility and effects. | Evolve `content.rs` from head/tail capture into bounded sequential traversal and source-bound context navigation. Keep parsing in the worker. No native resume or execution of historical tool records. Native/scoped/Desktop admission stays unchanged. |
| provider-claude | `lib.rs` performs explicitly scoped read-only inventory with fixture provenance and conservative live compatibility. `graph.rs` interprets observed sidecar relationships. No independently evidenced general Claude conversational search adapter exists at baseline. | Reuse provider-neutral query/evidence interfaces. Do not infer that Rust-side neutrality admits an undocumented Claude transcript format or Claude effects. |
| persistence | `owned.rs` validates private AgentMender roots; `store.rs` authenticates durable transaction knowledge and one-shot intent; `recovery.rs`/`handoff.rs` enforce frozen recovery/handoff. It never calls provider adapters. | Use a separate private derived-search store in `search_index.rs`, with no approval key or transaction construction. Source indexing comes through read observations, never provider I/O from persistence. Index data does not establish authorization or ownership. |
| mcp-server | `conversation.rs` defines strict DTOs, chooses legacy/advanced requests, projects bounded evidence and HMAC-seals cursors/references via `projection.rs`. `protocol.rs`/`frame.rs` enforce transport/schema/budgets; `dto.rs` omits private source paths. | Extend existing `session_search`/`session_context`. Bind new query/options into integrity-protected scope. Preserve typed schemas and no arbitrary SQL/path/command inputs. No destructive endpoint. |
| CLI | `conversation.rs` fixes root/helper/adapter composition and separates read mode from removal mode. `management.rs` rereads independent registration with instance/root/device/inode/expiry/control-chain checks. `approvals.rs` is the separate local review path; lifecycle/transaction modules preserve execution/recovery gates. | Compose scanning/index options and refreshed registration without giving model inputs caller authority. A root-owned declaration is not proof of exclusive chat routing. Default diagnostics remain useful; guaranteed management refuses when routing/identity is not independently established. |

Actual baseline advanced path:

`MCP SearchInput -> ConversationRequest::EvidenceSearch -> CLI read_only_executor
-> SessionSearchService::search -> CodexAdapter::search_sessions -> confined worker
-> fixed SQLite inventory -> supported rollout reads -> application validation
-> MCP bounded projection + opaque references`.

Context follows the same service/binding/exclusion/worker path, resolving only the
exact referenced target and source revision. The separate legacy metadata path
uses `read_only_dispatch`/`search_inventory`; removal mode uses
`ConversationService`, fixed cleanup inputs and existing protected persistence.
These paths must not be accidentally conflated by an advanced search extension.

## Verified baseline claims

| Reported capability | Implementation evidence / qualification |
|---|---|
| Conversational MCP metadata search | Implemented. Historical real model/client acceptance is documented separately; legacy path is bounded, diagnostic and Partial. |
| Bounded Codex transcript extraction | Implemented in `content.rs`: known legacy response-item messages and paginated inline completed display items, supported roles only. Baseline large reads capture head/tail, omitting the middle. |
| Lexical relevance | Implemented in domain `SearchRelevance`: lowercase substring terms, all legacy terms required across observed fields/messages; user/assistant/title/metadata weights 4/3/2/1. Not semantic relevance. |
| Evidence references/cursors | MCP process-local HMAC references, kind separated; application scope includes parameters, root, exclusions and binding revision; reader verifies inventory/batch/source snapshots. |
| Limited context | One supported message from the same bounded capture at baseline; no complete forward/backward chain. |
| Protected filtering | Trusted provider/root/session exclusions precede inventory matching, limits, counts and projection; app rejects foreign/excluded returned hits and protected context. |
| Automatic authenticated caller identity | Absent. Management mode refuses before provider access without independent registration. Registration validation cannot itself establish actual single-chat routing. |
| Read isolation | Source SQLite opens READ_ONLY/NOFOLLOW/query_only, bounded in-memory backup and existing macOS kernel-attested write denial. Linux/Windows fixture execution does not imply a live isolation backend. |
| Coverage | Explicit Partial status, unsupported/unreadable/truncated/suppressed counts/reasons and continuation. The inventory cap and source omissions prevent absence proof. |
| General indexing/structured engine | Absent at baseline. No persistent full-transcript index or typed Boolean/temporal/negative content engine. |
| Semantic subagent | No project custom relevance agent or automatic grounded semantic loop at baseline. Model availability/configuration is a distinct acceptance gate. |

## Missing behavior and bottlenecks

Baseline `SEARCH_INVENTORY=10000`, `SEARCH_BATCH=100`, transcript capture 1 MiB,
record 256 KiB, call reads 32 MiB, two excerpts per hit and small context protect
resources. They also prevent complete middle-message retrieval. Result pages
inside one batch repeat scans; inventory capture/hash repeats across calls;
unchanged content has no shared lexical index. The 32 MiB SQLite snapshot bound
can reject larger metadata stores. Ranking is within the current batch, never
an already established global Top-K. Existing coverage cannot prove negative text
conditions. Full traversal requires resumable per-source offsets/record positions
and aggregate leaf matches, not increasing bounds without limit.

Dates are optional epoch values at baseline, with no calendar/timezone query
normalization. `project_label` is derived from cwd basename; it is an observation,
not verified project ownership. Unknown timestamps, state, project or content must
remain unknown under negation. No model decision may repair missing source facts.
Parent IDs are optional observations and not complete descendant closure.

## Consolidation decisions

Retain the eight crates, exact identities, shared relevance/suppression functions,
existing worker process, `QueryControl`, application binding/exclusion checks,
MCP projector and private-root permission helpers. Replace head/tail extraction
on the active content path instead of adding another unconstrained reader.
Add a single strict query model and incremental evaluator, reused by streaming
and application validation, instead of independently implementing Boolean
semantics in MCP, SQL fragments and model prompts.

Legacy metadata compatibility remains deliberately separate because it has
existing response and coverage semantics. Lifecycle outcome types, plan versions,
transaction states and stronger Desktop assessment are not redundant: merging
these would widen or obscure historical authority. The deterministic message
adjacency/navigation structure does not belong in the destructive artifact graph.
No separate graph engine or embedding service is needed for this extension.

## Trust boundaries

- Retrieved historical text is untrusted quoted data. System/developer/tool,
  reasoning and inter-agent records are not automatically human-visible messages.
  Conservative secret/auth/config/environment suppression causes false negatives;
  redaction is not perfect and suppression prevents proof of absence.
- Model query values cannot change the composed provider root, authorize source
  paths, register a caller, execute commands or create approval. Exact project
  matches describe only their selected observed/verified fact source.
- Source references must bind provider/root/native identity, validated source
  revision and logical position. Replacements/appends/moves require revalidation;
  size/mtime alone is insufficient, and a reused cursor cannot silently mix data.
- Derived FTS postings and snippets are sensitive local data. Persistence must be
  explicitly selected, private, separated from provider roots, invalidated when
  sources disappear/change and never treated as proof that unindexed data lacks
  a match. Default scanning persists no transcript content.
- Guaranteed management filtering occurs before retrieval/counting/ranking and
  again before context/selection. Missing trustworthy caller identity yields zero
  manageable candidates. Diagnostic search does not authenticate the caller.
- Read evidence and semantic relevance never construct deletion authority. Existing
  independent approval, frozen scope, consumed DurableIntent and recovery remain
  the only admitted lifecycle path. No real destructive regression is authorized.

## Regression set retained

Domain property suites cover graph classification, ownership/integrity,
immutable plans, authenticated approval, transaction replay, scoped cleanup and
session-removal predicates, including historical schema-2 fixture encodings.
Application suites cover read dispatch, protected preview/preparation, lifecycle
scope, transaction orchestration and fail-closed search/context. Provider suites
retain native descendant-cascade/unknown-effect tests, scoped SQLite fences,
source isolation and the Desktop lost-update counterexample. Persistence tests
retain authority authenticity, consumed intent, private permissions, crash/reopen
and recovery. CLI/MCP tests retain strict schemas, bounded frames, process-local
references, wrong-root/protected input rejection and absence of effect authority.

Baseline content fixtures already test unrelated/duplicate titles, cross-message
terms, protected strongest matches, forged/missing binding, cursor/source staleness,
secret/tool/reasoning suppression, inert injection, unknown formats and linked or
traversal locators. The 5000-session pagination fixture checks exact counts and
no protected result. New query tests add complete Kleene tables, generated nested
Boolean equivalence between retained-message and incremental evaluation, unknown
project ownership, inclusive Europe/Rome days across 23/25-hour DST transitions,
strict AST bounds, explicit lexical alternatives and missing-message timestamps.

Required release checks remain fmt, locked Clippy/tests/build, architecture fitness,
both compatibility evidence validators, whitespace validation and exact final SHA
CI on Linux/macOS/Windows. Benchmarks, real-provider reads and actual conversational
model/client tests must be reported separately, including unperformed/open gates.
