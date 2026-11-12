# Architecture and capability boundaries

## P0-SR5 resolution boundary

[ADR-0032](adr/0032-caller-scoped-session-resolution.md) keeps Luna in the
Codex host and the bounded read-only workflow in the existing application,
provider and MCP layers. The application owns a small caller-scoped run
store, AST composition, evidence handles, continuation and final
revalidation. The existing Codex adapter and confined worker order and
traverse eligible sources. MCP projects bounded results and sealed
citations. Existing diagnostic, management and lifecycle contracts retain
their authority boundaries.

## P0-SR4 discovery scope

[ADR-0031](adr/0031-metadata-derived-caller-discovery.md) adds an apparent
caller scope to the existing read pipeline. MCP parses request metadata,
application composes exact exclusions, and the confined Codex worker
validates the apparent caller row before selecting candidates. Discovery
targets, cursors and evidence preserve read provenance. Authenticated
management and lifecycle admission stay on their existing paths.

## P0-SR3 resident decoding

[ADR-0030](adr/0030-resident-bounded-history-decoding.md) retains the same eight
crates and read ports. Provider-codex owns bounded raw Zstandard state and framed
worker reuse; CLI retains the adapter; domain owns replay/coverage values; MCP
seals/projections remain inbound only. No new storage or mutation authority.
[Traversal](progressive-history.md) describes limits, source freshness and privacy;
[caller identity](mcp-caller-identity.md) explains why management stays closed.

## P0-SR2 native history representation

[ADR-0029](adr/0029-compressed-rollout-history.md) adds bounded Zstandard rollout
decoding within provider-codex's existing attested content worker. The same read
port/application/MCP pipeline receives decoded messages, positions and explicit
transport formats. No new service, SQL store, caller proof or native startup is
admitted. [Native history](native-history.md) records the proven local source,
rejected alternatives, provenance and resource/privacy limits.

## P0-SR1 reliability refinement

[ADR-0028](adr/0028-search-reliability-and-scoped-freshness.md) refines query-scoped cursor validation, fixed locator counterparts, forward context progress and incomplete-evidence semantic acceptance in the same read ports/layers. It adds no crate, source writer, native startup admission, persistent index default or lifecycle authority.

Current direction: [ADR-0022](adr/0022-product-first-session-lifecycle.md) and the
[product roadmap](product-roadmap.md). The [audit](audits/2026-11-07-product-first-audit.md)
distinguishes executable behavior from design, tests and deferred research.

## Existing eight-crate structure

| Crate | Responsibility | Internal runtime dependencies |
|---|---|---|
| domain | Observations, graph/evidence, integrity, immutable plans, approval eligibility, transaction replay, scoped/complete result validation | None |
| provider-api | Read-only ports, bounded filesystem/process observations, owned evidence sandbox, audited OS queries | domain |
| application | Provider-independent discovery/selection/planning and durable lifecycle orchestration | domain, provider-api, persistence |
| provider-codex | Codex discovery, exact profiles, native handoff, scoped local effects, private Desktop candidates | domain, provider-api, persistence |
| provider-claude | Read-only inventory, sidecar identity and graph interpretation | domain, provider-api |
| persistence | Protected AgentMender-owned store, journal, human-approval authentication, recovery metadata and sealed handoff | domain |
| mcp-server | Bounded stdio transport, strict DTOs, ephemeral references and privacy projections | application, domain |
| CLI (`agentmender`) | Trusted adapter/root composition, independent terminal approval, command presentation | All seven library crates |

This is a local modular application with ports and adapters, not a distributed
service. No new crates or runtime are needed for the roadmap realignment.
Codex's application dependency is test-only; persistence never calls providers.
The narrow application/Codex dependency on persistence protects the private
DurableIntent constructor (ADR-0013), rather than exposing write authority through
every read adapter. Domain has serialization/hashing dependencies but no I/O.

## Adaptive real-session reads

[ADR-0025](adr/0025-bounded-search-and-management-exclusion.md) defines content
evidence and mandatory management exclusion. [ADR-0026](adr/0026-bounded-conversation-reader-and-independent-registration.md)
introduced the provider-neutral read port, pure relevance types and the initial
bounded Codex worker. Current [ADR-0027](adr/0027-intelligent-session-retrieval.md)
extends these same components as described below. Launch-protected identities
are excluded before retrieval/counts/projection. Production guaranteed mode
refuses before provider access until enforceable host binding exists; a privileged
registration assertion cannot prove routing. The
[live MCP acceptance](milestones/p0-sr3-live-mcp-acceptance.md) verifies updated
conversational retrieval. Real management binding remains open.

[ADR-0024](adr/0024-adaptive-read-only-metadata.md) separates useful metadata from
mutation contracts. Explicit `--session-search` uses a fixed-column protected
source worker and bounded in-memory SQLite snapshot, accepting additive structures
and unknown producer versions without granting effects. Provider-api confines
schema generation for the installed executable; provider-codex interprets advertised
native methods separately from effect admission. Legacy metadata exports no transcript/auth/config. Explicit content search
exports only bounded known visible message excerpts; auth/config files remain
denied and conservative secret/config/env filtering precedes relevance. Root identity, optional parent data and Partial coverage survive MCP.
The read-mode application dispatcher has no cleanup port or owned state. Its
preview is a blocked exact-target assessment; other removal calls refuse. The
existing exact-profile preparation/approval/intent/adapter path remains separate.

## Intelligent retrieval extension

[ADR-0027](adr/0027-intelligent-session-retrieval.md) evolves the same read-only
pipeline. Domain owns typed AST, calendar normalization and incremental three-valued
query observations. The Codex worker streams bounded source records with stable
positions and checkpoints, rather than only head/tail windows. Provider-api adds
bounded transcript/context page methods without mutation authority. Application
revalidates exact conditions, output partitions and exclusions; optional indexed
composition supplies query-scoped facts that the worker freshness-checks again.

Persistence adds a separate derived FTS5 store under an explicit OwnedStateRoot,
with private files, contentless postings, opted-in safe text, source/chunk keys,
transactions and fixed offline rebuild. It never reads providers. The ownership
graph and lifecycle journal are unchanged. Message adjacency is a retrieval
structure, not destructive ownership. MCP keeps deterministic search/context primitives separate from
`session_resolve`, which exposes a single progressive `start`/`advance`
state machine. Application owns the run, context budget, backpressure and
publication. Domain owns the typed intent and query predicates. The CLI
composition owns the autonomous `session_find` jobs, bounded private
coordinator and App Server child processes. The MCP adapter projects verified
rows and signs source references with the originating connection key. CLI
alone supplies persistent index paths and maintenance controls.

Production management registration files now remain inadmissible as caller proof;
only a future enforceable host transport can enable that guarantee. Missing binding
refuses before source access. [Current search](session-search.md) supersedes the
historical bounded-reader feature limits; historical receipts retain their
original results. [Index privacy](full-text-index.md) and [Luna](luna-session-relevance.md)
document data flow and limitations.

Conversational retrieval freezes one eligible UUID cohort in a confined
worker spool. It closes the source transaction before scanning and stages
candidates before the host starts one separate Evaluator. The worker verifies
source revisions for staged evidence, publication and UUID navigation. See
[ADR-0035](adr/0035-uuid-search-cohort.md) and [ADR-0036](adr/0036-two-stage-luna-resolution.md).

## Core session lifecycle versus additional cleanup

The target flow is:

```text
Codex or Claude MCP client (interprets the user's language)
  -> structured provider/root/session query and immutable preview
  -> application orchestration
  -> independent human channel -> plan-bound one-shot authorization
  -> protected durable intent -> provider-specific approved operation
  -> independent supported postconditions -> structured result
```

Default MCP remains read-only. The opt-in [P0-C1 handoff](conversational-codex-removal.md)
adds metadata search, exact primary preview, existing durable preparation and result
lookup. Human approval and execution remain in the trusted external terminal.
[ADR-0023](adr/0023-conversational-primary-removal-handoff.md) admits application
preparation only; MCP still has no persistence, grant, intent or effect authority.

Core removal needs exact target identity, its relevant effect/dependency proof,
approval and retained-session verification. It need not enumerate every cache or
renderer artifact. P2 adds artifact graph attribution, extra frozen actions,
independent approval and verification for each supported family. Those actions
use the same protected engine. A future native delete must not be routed through
Desktop cleanup merely to obtain a successful label.

Current native v1 planning requires a closed native effects and M3 integrity
contract and therefore cannot Ready from the real partial discovery profiles.
That restriction remains unchanged. A future scoped native capability needs
explicit relevant-scope proof and reviewed versioned plan semantics; it must not
manufacture global Healthy or reuse an old grant for a new effect.

## Native-first selection contract

Resolve a unique target with explicit provider/root identity. Determine available
operations and actual scope for the exact version. Include startup writes,
descendants, references, project/remote state and concurrency. Prefer the native
operation only when its entire effect set fits approval and its advertised
postconditions can be verified. Otherwise evaluate an admitted scoped local
fallback before producing the plan. If neither is suitable, return Unsupported
or Blocked with the specific cause. Once effects are possible, ambiguity means
OutcomeUnknown; it cannot authorize a fallback or automatic retry.

Codex and Claude do not have interchangeable operation semantics. Keep concrete
operations behind stable orchestration and typed handoffs. The current native
port is historical Codex unarchive; it is not a generic provider delete API.
Claude project purge must never substitute for a one-session request.

## Existing schemas and outcomes

| Contract | Meaning and admission |
|---|---|
| Native plan schema 1 | Immutable semantic intentions; historical exact unarchive dispatcher only; real plans blocked |
| Cleanup plan schema 2 | M8.2 `CleanupManifest`, exact primary payload/row and optional name-index effects; historical primary results |
| Expanded plan schema 3 | `CompleteRemovalManifest`, expanded primary row set, ordered durable action progress; PrimaryOnly admission |
| Desktop plan schema 4 | Separate exact bundle/catalog/global/index manifest; all compiled candidates unadmitted |
| Journal schema 1 | Historical inspection-only transactions |
| Journal schema 2 | Authenticated original review plan, independent grants, atomic consumption/durable intent and typed action observations |

Plan schema numbers and journal schema numbers are different namespaces. Do not
upgrade an old grant or hash domain. Historical v2 fixtures retain exact encoding,
binding digest and replay. New destructive effects require new reviewed scope.

`PlanStatus` reports readiness. `NativeOutcome` reports dispatch knowledge, not
deletion. `CleanupOutcome` and `CleanupReport` describe the approved primary
contract and bytes. `SessionRemovalAssessment` retains the stronger complete
persistent-projection criterion. `TransactionState` records durable knowledge and
verification. These models serve different consumers; merging them loses facts.
Human reporting should identify which contract succeeded and what remains.

Historical CLI plans retain the stronger assessment and exit-2 behavior. Protected
conversational schema-2 plans use an operation-qualified presentation and return
success for verified primary removal while disclosing residuals and pending GUI
observation. Old JSON, plan hashes, journals and outcome encodings are unchanged.

## Authority, identity and recovery

The private store constructs a noncloneable/nonserializable DurableIntent only
after exact original binding, verified recovery, fresh Current validation and
authenticated one-shot human approval. Consumption and intent are one signed
event, externally synced and committed before effects. Adapters consume the token
and return only observation authority. Per-action journaling stops later effects
if recording fails. Reconciliation cannot recreate a dispatch gate.

Terminal approval proves local interactive process participation, not personal
identity against an arbitrary same-user shell/PTY adversary. P0 must compose an
actually independent human interaction with the MCP workflow under an explicit
client threat model. MCP client identity, annotations and boolean confirmation
cannot supply it. Recovery stores bounded reconciliation metadata, not payload
backups, automatic rollback or secure erasure.

MCP references bind provider, class and hidden root to an ephemeral HMAC key. They
are lookup/privacy tokens, not durable approval IDs. Default views omit native IDs/paths/titles/time. The opt-in metadata projection
discloses bounded labels/IDs/time, and durable lookup survives restart. Trusted
launch registrations are frozen as UserProtected roots and restored from the
original plan during CLI approval/execution; they are not authenticated caller
metadata. Strong same-user agent/human separation remains an acceptance gap. Cross-provider configuration
is already independent of host identity on the read side.

## Deferred Desktop and artifact work

Desktop catalog/global/backup/index/legacy code is private to provider-codex and
separate from MCP. It is preserved for P1/P2 because it contains exact reducers,
host ownership, physical boundaries and valid regression evidence. Production
global replacement with target records remains refused; both candidate profiles
are unadmitted. Reclassification is not admission or permission to mutate JSON.

Keep ADR-0020/0021's complete structural predicate for any future Desktop claim.
Writer closure and all required persistent reconstruction sources still need
proof. User graphical acceptance is separate; production cannot control the app
for visual verification. The late-writer child-process regression remains a gate
even though complete Desktop removal no longer blocks P0.

## Fitness and maintenance

`scripts/check_architecture.py` checks the eight-crate dependency graph, graph and
planning purity, approval boundaries, one-shot gates, read-only Claude/MCP,
reviewed exact SQL/filesystem effects, global publication refusal and product
assessment constraints. Its built-in canaries detect prohibited source shapes;
these are structural regression checks, not substitutes for behavioral safety
tests or ownership proof. See [development verification](development.md).

Retain native scope counterexamples, historical replay, generated destructive
properties, crash/reopen tests and compatibility validators. Avoid speculative
abstraction or deleting deferred code merely to shrink line counts. Review actual
callers and serialization before removal. Future changes to MCP authority or
provider operations must update the relevant ADR and narrow fitness rules with
positive/negative behavior tests, rather than bypassing them.
