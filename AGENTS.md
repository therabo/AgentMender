# AgentMender engineering constraints

## P0-SR5 past-conversation resolution

For a normal request to find the conversation(s) that investigated, decided,
implemented or verified something, follow the [Luna workflow](docs/intent-aware-resolution.md)
and use `session_resolve` from the originating chat. A chat already running
GPT-6 Luna Medium can own the workflow directly. For other parent models,
delegate intent and semantic assessment to the named `session_relevance` agent. If the host does
not discover that role, explicitly spawn `gpt-6-luna` at medium effort with fresh
context and the same workflow. This instruction authorizes that delegation for
past-conversation search. The parent owns the MCP calls. Compare Luna's plan to
the live user request before starting, and give Luna evidence handles and short
quoted passages. Keep callable references and the run ID in the parent. For an
all request, continue acquisition and assessment through the available scan.
Use the fresh finish result as the only source of selected identities and cite
its evidence. Distinguish a direct supported match from global uniqueness.
Disclose unresolved evidence, incomplete work and Partial history as applicable.

## P0-SR4 read-only caller-filtered discovery

Follow [ADR-0031](docs/adr/0031-metadata-derived-caller-discovery.md).
Use `session_discover` for deterministic inventory, exact targets and
possible deletion candidates. Both discovery tools are read-only. The read-only route takes an apparent
Codex caller from RMCP request metadata and validates its canonical thread row
in the configured root before searching. Missing, malformed, discordant or
unmapped metadata refuses. Never accept a caller UUID in tool arguments.
Exclude before predicates, content, ranking, counts and paging. Bind discovery
cursors, evidence and context targets to caller scope and process. Keep
discovery targets separate from lifecycle targets and keep context reads in
the originating chat when handing evidence to Luna. Metadata is spoofable
and must never create a `ManagementIdentity` or authorize an operation.

## P0-SR3 resident traversal and default management refusal

Follow [ADR-0030](docs/adr/0030-resident-bounded-history-decoding.md),
[progressive traversal](docs/progressive-history.md) and
[host identity](docs/mcp-caller-identity.md). Total compressed file length is not an
admission limit. Keep bounded per-operation decoding/input, window, record, output,
recent-buffer capacity, resident-reader count and deadlines. Reuse only exact
root/path/UUID/revision state inside the existing confined worker; no transcript
spool, portable decoder snapshot, source writer or new database. Lost required
replay state refuses; no identical successful continuation. EOF is not logical
paginated completeness. Context suffixes cannot attest prefix coverage. Preserve
exact context-locator/source revalidation when unrelated metadata changes.

`session_search` defaults to management and fails before source reads without a
verified host route. Diagnostics require explicit purpose; never silently fall
back. Arbitrary `_meta.threadId`, model arguments, transport IDs, process identity
and privileged declarations cannot create caller authority. Typed shared-reader
fixtures are not actual-host exclusion acceptance. Retained EOF-framed parents
receive compatible small-source output; required resident replay refuses until
only that MCP connection is refreshed. Never restart Codex automatically.

## P0-SR2 compressed history

Follow [ADR-0029](docs/adr/0029-compressed-rollout-history.md). Missing plain
locators may use their exact `.jsonl.zst` counterpart before fixed relocation.
Decode only in the existing attested worker; never materialize into provider
state or start App Server for content. Decoded offsets differ from physical
source size/revision. Preserve explicit compressed evidence formats, decoder
window/per-operation work/deadline bounds, header UUID, NOFOLLOW/unshared identity and
source revalidation. Corruption/work limits cannot prove absence. History SQL,
host/model transcript ingestion and imports are not admitted. Signed search
tokens admit at most 32 KiB; retain HMAC kind/process binding and frame limits.
Private receipt paths must be new per milestone; never run a historical driver
unchanged when its output directory is fixed to an earlier acceptance folder.

## P0-SR1 reliability constraints

Follow [ADR-0028](docs/adr/0028-search-reliability-and-scoped-freshness.md). Cursor fingerprints bind eligible identities, predicate leaf truth, lexical facts and sort keys; unused display facts are read fresh. Relevant membership/order and selected source changes still refuse. Only fixed active/archive counterparts of missing locators may be tried, with exact header UUID, NOFOLLOW, single-link and revision checks. No recursive scanner or root widening. Positive next offsets mean unfinished messages; completed messages advance. Reject identical continuation pairs. Session-wide semantic negatives require trusted complete coverage, fresh source/filter validation and parent-vetted negative evidence. Keep real receipts outside Git. No management admission is added.

## Current product direction

Read [ADR-0022](docs/adr/0022-product-first-session-lifecycle.md), the
[canonical roadmap](docs/product-roadmap.md), [architecture](docs/architecture.md)
and [audit](docs/audits/2026-11-07-product-first-audit.md) before changing scope.
The product is MCP-native conversational session lifecycle management, initially
for Codex and Claude Code. The calling agent interprets language; AgentMender
provides structured reliable operations. Provider identity is independent of the
host agent. Codex→Claude and Claude→Codex remain target scenarios.

Priority order: P0 conversational single-session workflow; P1 native operations
and provider coverage; P2 additional artifact cleanup; P3 project/bulk lifecycle.
First prove one complete vertical removal path, then both initial providers.
M8.3 multi-session cleanup is not the next implementation priority.

M8.2 primary cleanup remains historical PASS at main
`a284767157ace8769deee85b195b5b70efe374d8`. M8.2.1 is **unfinished/deferred**, not
PASS and **no longer a global blocker for the conversational MVP**. Preserve its
tip `14a8931f38949d3dbde04a61b22091752a685402` through archival tag
`archive/2026-11-08/milestone/m8-2-1-complete-session-removal` and historical evidence.
Schema-4 builds 24141/23880 remain separate unadmitted candidates with an empty
compiled admission list. Never enable unsafe Desktop global JSON publication;
preserve the child-process lost-update regression and zero-operation refusal.

M8.1 native full cleanup and M9 project-purge blockers retain their investigated
version/operation scope. They do not prove all Codex or Claude session management
impossible. Current Claude has zero mutation profiles. Do not transfer historical
Codex unarchive or stable primary-cleanup authority to a newer binary/Desktop.

## P0-C1 implementation boundary

Read [ADR-0023](docs/adr/0023-conversational-primary-removal-handoff.md) and the
[conversational contract](docs/conversational-codex-removal.md). P0-C1 is OPEN,
with bounded components and disposable tests. Default MCP retains eight read-only
tools. Explicit session-removal mode adds metadata search, exact schema-2 preview,
existing durable preparation, authenticated status and external execution handoff.
MCP cannot grant/revoke/consume approval, obtain keys or dispatch effects. Only
application/conversation.rs may prepare existing transactions; it has no grant or
intent/effect authority. Preserve the source canaries for this boundary.

Trusted launch configuration independently registers management UUIDs. Freeze them
as retained UserProtected graph roots; restore them from the original plan during
CLI approval/execution and recheck at preflight/fresh validation. No model-supplied
caller identity, Active label or title authenticates the management session. No
registration means no conversational destructive admission. Same-user shell/PTY,
configuration or authority-key access does not enforce human independence: retain
external human approval AND execution, and do not claim production/client acceptance
from a fixture grant. No real-user destructive tests or app restart. Read-only real metadata checks are explicitly authorized for this milestone.

Read [ADR-0024](docs/adr/0024-adaptive-read-only-metadata.md). Real-session
`mcp --session-search` has no cleanup port, owned state, approval key or caller
requirement. Fixed-column metadata runs only in the reused kernel-attested worker:
READ_ONLY/NOFOLLOW, query_only, bounded memory backup; legacy metadata exports no
transcripts/auth/config. ADR-0026 separately admits fixed bounded rollout reads
under the same kernel write denial, with auth/config contents still denied.
Additive schemas/unknown producers may be readable. Generated native method names
are availability observations only. Read-mode preview stays blocked with no plan;
prepare/status/execution handoff refuse. Do not turn this reader into raw mutation
or infer caller protection. Synthetic onboarding is retired; preserve deterministic
regression fixtures, not the standalone onboarding generator.

The existing P0-C1 destructive fallback selects exact stable 0.166.0 Darwin arm64 scoped-cleanup-v1 Legacy/WAL.
Native delete remains unadmitted; no fallback after unknown native effects. Keep
schema-3/4, old hashes/journals/outcomes and historical CLI meanings unchanged.
Protected conversational schema-2 plans have operation-qualified primary reporting.
Do not start Claude deletion, bulk/project cleanup, Desktop global mutation or
additional artifact families. Full P0-C1 PASS needs actual Codex client + independent
human approval/execution + structural primary/collateral proof + final exact CI.

## Intelligent retrieval (ADR-0027)

Read [ADR-0027](docs/adr/0027-intelligent-session-retrieval.md), the current
[search contract](docs/session-search.md), [index contract](docs/full-text-index.md)
and [implementation report](docs/milestones/intelligent-search-implementation.md).
Extend the same read ports/tools, never a parallel provider scanner. Pure typed
queries use three-valued logic; missing/suppressed/unsupported content cannot
prove absence. Calendar dates require explicit zones and half-open UTC bounds.
Sequential source-bound positions replace head/tail-only discovery. Preserve
bounded work, continuations, source freshness, exact exclusions and inert evidence.

FTS5 memory scoring and explicit private persistent cache belong in existing layers.
Persistent mode stores safe visible message text as well as sensitive terms; require
an explicit launch/CLI root, private permissions, provider disjointness and tested
fixed-file rebuild. Never copy auth/config or let the source worker write an index
outside its owned evidence boundary. Semantic adjacency grants no artifact ownership.
No embeddings/cloud/vector/graph service is required. Luna Medium is optional Codex
orchestration, not an authority or security boundary.

Production file-only management admission is disabled: a root-owned assertion does
not prove actual host routing. Until an enforceable host binding exists, guaranteed
management search refuses before source access with zero candidates. Typed fixtures
may exercise exclusion; do not claim real binding from them. Diagnostic launch
protections remain effective but are not automatic caller authentication. Historical
ADR-0026/P0-S1 statements below describe their earlier implementation; ADR-0027 and
the current contract govern this extension. No lifecycle admission changes.

## Architecture

Read [ADR-0025](docs/adr/0025-bounded-search-and-management-exclusion.md),
[ADR-0026](docs/adr/0026-bounded-conversation-reader-and-independent-registration.md)
and current [ADR-0027](docs/adr/0027-intelligent-session-retrieval.md).
P0-S1 supplied bounded parsing, lexical evidence and sealed references. ADR-0027
extends supported full-stream traversal, typed queries, contextual navigation
and optional private indexing. Actual updated conversational client acceptance,
real management binding and current-version destructive acceptance remain open.
Exclude launch-protected provider/root/session identities before retrieval,
counts and projection; refuse protected preview/preparation. Guaranteed management
workflows require enforceable host binding and otherwise return
caller_identity_required before provider access. Root-owned declarations alone
are inadmissible. Diagnostic inventory is not a manageable candidate list.
Never infer caller identity from title/cwd/activity or model-supplied UUIDs.

Keep the eight crates. Domain is pure and owns shared approval eligibility.
Provider-api has read-only ports and protected observation helpers. Provider
interpretation belongs in adapters. Application obtains fresh trusted inputs and
owns lifecycle orchestration. Persistence owns private authority construction,
journal/recovery and authenticated history; it never calls a provider. CLI composes
concrete adapters and the independent local human broker. MCP is a thin inbound
interface over application/domain, never an authorization authority.

Keep native, primary cleanup, complete Desktop assessment and durable transaction
results semantically distinct. Consolidate only genuine duplication; do not merge
security or serialization boundaries. Do not add crates or generic mutation ports
for appearance. Advanced private candidate modules may remain when they provide
reusable evidence without adding admission to the active path.

Prefer provider-native operations only when exact-version effects and supported
postconditions satisfy the user's approved target set. Native cascade/startup
side effects count. Otherwise evaluate a reviewed local fallback before approval;
never fallback after an unknown native outcome. Session requests never widen to
project purge. Additional cleanup is optional and separately scoped, using the
same approval/transaction engine. Preserve M3 health/negative-proof meanings.

## Mandatory safety invariants

- Verify exact provider/root/native identity, aliases, relevant dependencies,
  physical identity, compatibility, schema and writer boundary. Unknown versions
  remain audit-only. Presence, evidence quality, compatibility and coverage differ.
- Observation is not inference. Project paths, cwd, time proximity and filenames
  never establish destructive ownership. Preserve conflicting observations and
  partial/unknown evidence. Graph/planning transformations are deterministic and
  perform no I/O or clock sampling. Integrity findings grant no deletion authority.
- Freeze exact required/optional/retained effects. Actual effects must be a subset
  of approved effects. New units or changed scope require a fresh plan. Preserve
  every unrelated session and shared/referenced/global/ambiguous/unknown unit.
- Use existing immutable planning, fresh validation, independent human review,
  protected persistence, recovery and one-shot consumed DurableIntent. A model
  cannot approve its own destructive request; no boolean, --yes/force, noninteractive
  grant, imported proof or caller-chosen principal can bypass the human boundary.
  Terminal possession alone does not resist a same-user shell/PTY adversary.
- Journal and sync intent before every effect, including provider startup and lock
  creation. Record observations after effects. Missing outcomes stay unknown.
  RecoveryRequired/OutcomeUnknown never automatically retry, compensate, restore
  an entire provider store or reconstruct dispatch authority. Reconciliation reads
  only the frozen manifest. Recovery metadata does not back up deleted payloads.
- Preserve old plan/hash/approval/journal/replay semantics. V1 history remains
  inspection-only; v2/v3/v4 manifests never inherit expanded authority. Do not
  delete crash regressions or valid destructive property tests to obtain green CI.
- Native return success/list absence is not transcript-erasure proof. Report the
  verified operation, remaining artifacts, graphical status and logical bytes
  separately. Never claim physical reclaim or secure erasure without evidence.
- Protect the active management session before any future P0 destructive admission.
  Existing Active state/writer detection does not implement caller-session binding.

## Provider isolation and Desktop research

Provider processes require OS-enforced real-state write denial; no unsandboxed
retry. Temporary effects need an opaque owned evidence sandbox (ADR-0003).
Source SQLite is opened read-only only inside the kernel-attested worker
(ADR-0004). Disposable mirror SQL is not repair/recovery authority.

Workspace unsafe_code is forbidden except the existing explicitly reviewed macOS
C-ABI query module in provider-api. Do not expand that exception without an ADR
and safety review. No general SQL, file-write, command or app-control interface.

Read ADR-0017 for scoped primary fences and ADR-0020/0021 plus the projection and
artifact inventories before any future Desktop work. Complete Desktop structural
claims still require independently verified persistent reconstruction sources,
collateral preservation and admitted exact profiles. The user verifies graphical
disappearance separately. Production never starts/stops/kills/reloads/restarts
Codex for visual verification. Closing writers is a prerequisite, not a lifetime
lock or CAS. A process census plus compare/rename cannot close the demonstrated
global Map writer race. Keep global target publication refused.

## Development and release

Use sanitized/disposable fixtures. Real probes are explicit and separate from CI;
never test destruction on user state. Run relevant tests after each meaningful
refactor, then the full locked gates:

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
python3 scripts/check_codex_lifecycle_evidence.py
python3 scripts/check_complete_removal_evidence.py
git diff --check
```

Use `main` as the sole ordinary development branch after consolidation. Preserve
historical milestone tips through verified archival tags and the
[original-ref manifest](docs/audits/2026-11-08-original-refs.json), not active branches.
Before future branch retirement, verify a complete outside-repository bundle and
recovery refs, remote/default HEAD, exact final three-OS CI and worktree/dependency
absence. Use coherent commits and normal pushes. Do not force push, rewrite
history or substitute an earlier green run for final CI. Historical
milestone reports, compatibility evidence and ADRs retain their original verdicts;
new decisions supersede them explicitly. The current roadmap controls sequencing.
