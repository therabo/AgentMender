# AgentMender product roadmap

Canonical roadmap, accepted 2026-11-07 in [ADR-0022](adr/0022-product-first-session-lifecycle.md).
Historical milestone receipts preserve their original verdicts; they are not the
current implementation order. See the [audit and readiness assessment](audits/2026-11-07-product-first-audit.md).

## Product vision

AgentMender is an MCP-native session lifecycle manager for AI coding agents.
The user asks a dedicated Codex or Claude Code agent to find, inspect and safely
remove a selected session. The calling agent interprets natural language;
AgentMender provides structured discovery, exact identity, immutable review,
independent human authorization, safe provider execution and verified results.

Codex and Claude Code are both initial target providers. The host agent and target
provider are independent: Codex→Codex, Claude→Claude, Codex→Claude and Claude→Codex
must eventually pass the same workflow, subject to supported local profiles and
explicit permissions. Exact provider/root identity belongs in every operation.

Example target experience, **P0-C1 OPEN; external human acceptance incomplete**:

> Find the old Codex session where we worked on the MISSION1 README.
>
> Review the matching provider, project, title and exact identifier. Resolve any
> ambiguity before previewing removal. The human approves that precise plan in
> the trusted independent channel. AgentMender removes and verifies only the
> approved scope, then reports the supported result and any residuals.

## P0 — Conversational session management MVP

Deliver one single-session removal vertical slice first, then the equivalent
Claude workflow. A Codex-only slice is an intermediate delivery; it does not
complete the initial two-provider product.

Required workflow: connect local stdio MCP → discover → search/select → resolve
ambiguity → inspect → preview exact removal → human approval outside model
authority → execute approved adapter → verify → understandable result.

Acceptance criteria:

- Document reproducible MCP setup for both calling clients and supported local
  provider/version/platform profiles. Exercise actual clients on disposable data.
- A request about a project/title returns useful matching metadata and an exact
  provider-scoped identity. Duplicate titles, alias IDs, roots and partial scans
  cannot silently choose a target or prove absence. No project path grants deletion.
- Content-aware discovery must also find supported user/assistant conversation
  text when query words are absent from title/project, with bounded excerpts,
  match provenance and explicit Partial coverage. The
  [search contract](session-search.md) defines privacy, limits and acceptance.
- Ordinary read-only discovery should omit the apparent calling conversation
  without a user instruction. `session_discover` supplies that behavior
  for supported Codex metadata and returns discovery candidates. See
  [P0-SR4](milestones/p0-sr4-implicit-caller-discovery.md).
- Identify and protect the active management session and other unsupported active
  targets before approval and again at dispatch. An unknown caller binding cannot
  be treated as an exclusion proof.
- Exclude the verified management identity before matching/ranking, pagination,
  counts and presentation across management listings, metadata/content/archive/
  project queries and bounded context. Missing verified identity in a guaranteed
  workflow must return caller_identity_required, never heuristic candidates.
- Preview freezes exact provider version, target, affected set, side effects,
  residuals, irreversible consequences, recovery limits and supported postconditions.
- An MCP client cannot grant/revoke/consume authority, supply `confirm=true`, choose
  a principal or forge a ready plan. A human-operated independent channel reviews
  the exact digest; stale, expired, revoked or consumed approvals cannot execute.
- A supported adapter uses the existing durable intent/transaction/recovery path.
  Journal intent precedes startup and all other effects. Freshness and exact
  destructive scope are enforced; descendants outside approval cause refusal.
- Structural verification proves the advertised primary/native postconditions and
  preservation of unrelated sentinel sessions. Command success or list absence
  alone cannot prove transcript deletion. Physical reclaim and GUI status remain
  separately described.
- Fixture integration, crash/reopen and same-client retry tests retain unknown
  outcomes without redispatch. Exact-final-commit Linux/macOS/Windows CI passes;
  this fixture matrix does not imply all platforms have mutation profiles.
- Both provider workflows and all four host/target combinations pass before the
  complete initial P0 product is declared ready. Unsafe/unsupported combinations
  return explicit limits rather than widening a session to a project purge.

Dependencies: existing discovery/planning, exact operation compatibility,
independent approval, persistence/recovery and structural verification. P0-C1 adds
metadata, registered exclusions and MCP preparation/status. Actual caller/client
validation and independently operated human approval/execution remain open. Exhaustive Desktop projections, caches and bulk work are not dependencies.
No local LLM or new natural-language interpretation service is required.

## Intelligent retrieval delivery — current checkpoint

P0-SR5.1 adds [SearchIntent v1](intent-aware-resolution.md) for ordinary
requests and preserves the advanced Boolean contract. Project mentions are
topical hints until the request explicitly limits membership. Real-client
acceptance and host routing are tracked in the
[SR5.1 report](milestones/p0-sr5-1-search-intent-acceptance.md).

P0-SR5 implements caller-scoped intent and outcome resolution through the
existing Codex host and FTS5-backed read service. The implementation
supports single, multiple, ambiguous and incomplete results. Promotion
requires actual-client behavior as well as acquisition, evidence and
safety regressions. [SR5 acceptance status](milestones/p0-sr5-intent-aware-session-resolution.md)
records the measured gaps. P0-SR4 made ordinary read-only discovery
caller-filtered; authenticated management remains a separate open gate.

P0-SR3 removes total compressed-size admission and retains bounded worker decoder
state. The exact 330 identities and five oversized histories reach physical EOF;
logical paginated coverage stays Partial. All four MISSION1 targets retain actual
conversational retrieval. [Current evidence](milestones/p0-sr3-management-and-traversal.md)
separates fixtures, real reads, retained-client results and blocked refreshed-parent
acceptance. Authenticated host routing remains OPEN. The management route
fails closed and developer diagnostics require explicit purpose.
Restore remote access/CI and host admission before claiming production management.
No semantic infrastructure, deletion or provider-startup scope is added.

[ADR-0027](adr/0027-intelligent-session-retrieval.md) extends P0 search with typed
nested filters, UTC/calendar dates, unknown-safe negative conditions, full supported
stream traversal, source-bound context windows, FTS5 and opt-in private indexing.
[Search](session-search.md), [indexing](full-text-index.md), [Luna](luna-session-relevance.md)
and the [implementation report](milestones/intelligent-search-implementation.md)
are canonical. These features do not complete deletion or the two-provider MVP.

Remaining mandatory acceptance is explicit: enforceable real caller routing,
updated actual conversational client/custom-agent acceptance, and documented
provider/format/coverage limits. Guaranteed production management requests refuse
without that host capability; a privileged registration file cannot substitute.
No speculative embeddings or GraphRAG milestone blocks useful local retrieval.

### Progressive conversational retrieval

`session_resolve` uses one SearchIntent v1 and one `start`/`advance`
workflow. Luna assesses candidates in bounded batches. Rust freezes the
UUID cohort, publishes source-bound evidence and reports coverage.
`session_context` can reopen an exact UUID after the run.
See [ADR-0035](adr/0035-uuid-search-cohort.md), the
[progressive workflow](intent-aware-resolution.md#progressive-workflow) and
the [benchmark and acceptance report](milestones/uuid-search-snapshot.md).
Installed app-server model controls and graphical Desktop acceptance remain
separate.

## P1 — Provider coverage and native lifecycle operations

Expand the same workflow across selected Codex and Claude versions, session types
and platforms. Apply native-first policy inside P0 too; P1 expands coverage rather
than postponing all native operations until after P0.

Acceptance criteria:

- Publish a per-operation exact profile matrix separating list removal, archive,
  transcript deletion, associated local state, descendants, projects and remote
  effects. Unknown versions remain audit-only; installation is not compatibility.
- Prefer official APIs/App Server/documented CLI operations when their complete
  effect boundary fits approval. Test startup writes, hidden descendants, partial
  failures, references, retention and independent postconditions on disposable data.
- A narrower supported local fallback is evaluated before plan/approval; it is
  never selected automatically after uncertain native effects. Each new profile
  has explicit ownership, writer, schema, crash and collateral tests.
- Archive and remove-from-view results never claim transcript deletion. A
  still-resumable Claude transcript is reported as retained. Remote state remains
  out of scope until a separate contract is supported.
- Upgrade/drift tests refuse unsupported fingerprints without carrying old grants
  into new operations. Active-session protection holds across host providers.

Dependencies: a P0 vertical workflow and operation-specific evidence. M8.1/M9
findings constrain the investigated operations; they are not provider-wide
impossibility proofs. Provider coordination research from M8.2.1 is reusable here.

## P2 — Deep artifact cleanup and storage reclamation

Extend a working primary lifecycle with additional safely attributable snapshots,
history records, logs, cache entries, tool artifacts and persistent projections.
ArtifactGraph/evidence, existing typed manifests, approval and transaction machinery
are reusable. Their availability does not force their entire inventory into P0.

Acceptance criteria:

- Every extra artifact or exact record has sufficient categorical target
  attribution, a supported compatibility/schema boundary and writer coordination.
  Cwd, project membership, filename resemblance and time proximity grant no scope.
- Remove every supported safe exclusive approved unit; preserve shared,
  referenced, global, ambiguous, unknown and unrelated state. New discoveries
  require replan, never dispatch expansion.
- Reverify both added effects and retained commitments; crash gaps stay unknown.
  Report logical unlinked bytes separately from physically reclaimed allocation.
- A complete Desktop-removal claim additionally passes its own persistent source,
  host identity, writer and compiled admission gates. Preserve and pass the
  late-writer lost-update regression. No unsafe global JSON mutation is enabled.
- User graphical confirmation remains separate from structural verification;
  AgentMender does not restart a real application for UI verification.

Dependencies: working primary management plus evidence for each extra family.
M8.2.1 is **unfinished/deferred under P1/P2 and no longer a global blocker for the
conversational MVP**. Green code tests alone cannot relabel it PASS.

## P3 — Project and multi-session lifecycle

Add project discovery/selection, multi-session preview, aggregate approval, bulk
archive/removal, estimates, partial success and batch recovery.

Acceptance criteria:

- Project membership proposes exact sessions; the immutable approved set excludes
  ambiguous sessions, the management session and protected state. Project files
  and unrelated worktrees are never included by association.
- Each member uses a supported single-session operation with the same safety
  gates. Aggregate effects cannot exceed aggregate approval. Changing membership
  invalidates review; no implicit descendant or project expansion is allowed.
- Mixed success/failure/unknown outcomes and per-member recovery survive restart
  without automatic retry. Aggregate logical bytes deduplicate identities and
  never manufacture a physical reclaim total.

Dependencies: reliable P0/P1 single-session execution and suitable per-member
contracts. P2 is needed only for optional batch deep-cleanup claims. Historical
M8.3 multi-session cleanup is not the next implementation priority.

## Current implementation and next decision

| Area | Current state |
|---|---|
| Codex discovery | Real fixed-column metadata search independent of executable hash; separate historical App Server profiles; partial coverage |
| Claude discovery | Partial observed Desktop sidecars and synthetic transcript fixtures; no general live Code SDK inventory |
| Search / useful labels | Real conversational metadata search accepted: user-reported 37 MISSION1 records, Partial; title/name/project/UUID matching, available time/archive/parent observations; bounded content search implemented; updated client acceptance OPEN |
| Management-session exclusion | Guaranteed management mode refuses without independent exact binding; typed exclusion precedes content/counts/ranking/paging/context; automatic/real binding acceptance OPEN |
| MCP | Eight default read-only tools plus six opt-in search/context/preview/preparation/status/external-handoff tools |
| Native Codex | Exact historical unarchive handoff is tested but real plans remain blocked; native archive/delete not executable |
| Scoped Codex | M8.2 primary cleanup PASS for exact 0.166.0 Darwin arm64 Legacy/WAL profile; schema-3 expanded primary has disposable evidence |
| Desktop complete removal | Schema-4 candidates 24141/23880 unadmitted; global writer refusal and reconstruction gaps remain |
| Claude mutation | Zero adapters/profiles; historical project-purge blockers preserved |
| Approval / transactions | Existing terminal challenge/grants/journal; MCP prepares durable attempts and reads status; human approval and execution remain external |
| Deep / bulk cleanup | Some exact primary/index actions exist; broader artifact families and batch execution are deferred |

P0-C1 delivery: **OPEN**. See [the current report](milestones/p0-c1-conversational-codex-removal.md)
and [operating contract](conversational-codex-removal.md). Metadata, protected
preparation and provider execution components are implemented; an actual Codex
conversation with independently operated human approval remains unaccepted.
The [real-world integration report](milestones/p0-c1-real-world-integration.md)
records real read-only discovery and the current-version native counterexamples.
Use the [real-session quick start](p0-c1-manual-acceptance.md). The later accepted
real model metadata invocation supersedes the earlier reports' client gap only;
verified management binding and independent human deletion remain acceptance gates.

Current **P0-S1: bounded content evidence with verified management exclusion**
implements the bounded reader, grounded results and independent-registration
refusal in [ADR-0026](adr/0026-bounded-conversation-reader-and-independent-registration.md).
The [live MCP acceptance](milestones/p0-sr3-live-mcp-acceptance.md) verifies the
updated content tools. The remaining work is independently verified real host
binding.


Audit baseline readiness: **READY WITH CONSTRAINTS**. Review the
[proposed bounded vertical slices](audits/2026-11-07-product-first-audit.md#proposed-next-implementation-milestones)
before selecting work. This audit implements no new feature.

## Risks and release discipline

Provider drift, native cascade/startup effects, incomplete discovery and ambiguous
ownership can invalidate scope. Human approval must remain independent from the
calling model, including its available shell/PTY capabilities. Opaque MCP identities expire at restart; durable transaction IDs survive. Opt-in
labels and trusted registered exclusions are implemented. Registration is not an
authenticated caller binding, and an unrestricted same-user agent can cross the
terminal/key boundary. Writer races and recovery
crash gaps require explicit unknown results. Exact-version support can be narrow,
but must be honestly described and tested. Do not turn complete global M3 health
into a proxy for every future scoped operation or weaken M3's negative proofs.

`main` is the sole ordinary development branch; annotated archival tags and the
verified bundle preserve original tips. Changes need relevant targeted tests,
full locked local gates and exact final three-OS CI. No history rewriting,
real-user destructive tests or app restarts. Historical ADRs,
research and PASS/OPEN/BLOCKED receipts remain evidence, read with ADR-0022.

### P0-S1 implementation status

Bounded Codex legacy message and paginated completed-display-item search is now
implemented with grounded excerpts/context and diagnostic/management separation.
Known shape contracts are independent of the mutation pin. Missing independent
caller registration refuses with zero candidates. Neutral-title/ranking/strongest
protected-match, privacy, source isolation and 5,000-session pagination have fixture
coverage. See [the current search contract](session-search.md) and
[ADR-0026](adr/0026-bounded-conversation-reader-and-independent-registration.md).

P0-S1 remains **OPEN** on verified caller binding and automatic self-exclusion.
Real updated content invocation passed in the live MCP acceptance. Close the
caller gate before new deletion functionality. Historical P0-C1 deletion
remains OPEN.
