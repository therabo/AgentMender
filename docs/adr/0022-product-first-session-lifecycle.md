# ADR-0022: Product-first session lifecycle and separation of primary removal from deep cleanup

## Status

Accepted, 2026-11-07, by the product-first audit request. Supersedes the product
sequencing and universal complete-removal prerequisite in ADR-0018 through
ADR-0021, and the project-first sequencing of the historical M9 investigation.
Their operation-specific safety contracts and historical findings remain valid.
This decision admits no provider operation and changes no serialized authority.

## Context

AgentMender's original purpose is conversational, MCP-native session lifecycle
management for Codex and Claude Code. The original research recommends native
provider operations, external result verification, independent approval and
version-aware inspection. It does not make forensic erasure a precondition for
every useful lifecycle operation.

M0–M7 supplied discovery, evidence, planning and durable human authorization. M8
established a conditional native unarchive handoff. M8.1 and M9 found real limits
in specific native contracts. M8.2 proved one narrow Codex primary cleanup path.
M8.2.1 then made all persistent Desktop projections and maximum artifact cleanup
the product-wide next gate. That sequence delayed a complete conversational
workflow while adding exact-build catalogs, renderer/legacy state, multi-host
identity and shared JSON coordination. Those are valuable provider/deep-cleanup
problems, but do not determine the semantics of every native session operation.

The late-writer counterexample at `14a8931` is a valid collateral-loss regression.
Closing Desktop, taking an advisory inode lock and comparing before rename do not
prevent its cached Map writer from replacing the file in the publication gap.
Deferring that work does not make JSON publication safe.

## Decision

1. The canonical [product roadmap](../product-roadmap.md) orders P0 conversational
   session management, P1 provider/native coverage, P2 additional artifact cleanup,
   then P3 project and bulk lifecycle. Both Codex and Claude Code are required for
   the complete initial product. First prove one supported vertical removal path.
2. Keep the eight crates and their existing authority boundaries. The calling
   agent interprets natural language; MCP carries precise provider/root/target
   references and structured results. Discovery is independent of the MCP host's
   provider. Cross-provider management is a required future acceptance matrix.
3. Core lifecycle is discover → resolve/inspect → preview → independent approval
   → remove → verify. Additional cleanup extends it with artifact attribution and
   separately approved effects. Native deletion need not depend on exhaustive
   cache, renderer or Desktop artifact closure. It still needs complete evidence
   for its own destructive scope and retained-session invariants.
4. Prefer a native operation only after exact-version evidence establishes its
   actual effects, startup behavior, descendants and supported postconditions.
   Compare that affected set with the user's approved set. Otherwise evaluate an
   already supported scoped local fallback before planning/approval. Unsupported
   scope refuses. An ambiguous native result never triggers a fallback or retry.
5. Reuse immutable plans, `TransactionBinding`, protected `TransactionStore`,
   independent human review, verified reconciliation metadata, consumed
   `DurableIntent`, durable action observations and read-only reconciliation.
   Do not add an unprotected session executor or a second authorization store.
6. Keep existing result types with their versioned meanings. `NativeOutcome` is
   dispatch knowledge; `CleanupReport` describes primary effects;
   `SessionRemovalAssessment` describes the stronger persistent-projection claim;
   `TransactionState` describes durable execution knowledge. They are not duplicate
   success enums. Describe a verified narrower operation as such, and separately
   disclose residuals, graphical status and unknown physical reclaim. Do not
   rename old `Removed` encodings or upgrade historical plans/grants.
7. M8.2.1 is **unfinished/deferred**, not PASS and **no longer a global blocker
   for the conversational MVP**. Preserve private candidate code, exact profile
   pins, empty compiled Desktop admission, global-write refusal, source inventory
   and the lost-update counterexample. P1 may revisit provider coordination; P2
   owns additional projection/artifact cleanup. ADR-0021 still governs any future
   claim of complete Desktop structural removal and manual graphical acceptance.
8. M9's project-purge blocker remains true for the investigated versions and
   operations. It is not a proof that all Claude session management is impossible.
   Future work must investigate session-level operations, distinguish removal
   from a list from transcript deletion, and refuse unsafe project widening.
9. Active management-session protection is a P0 admission requirement. Existing
   writer fences and session-state labels are not a caller-session exclusion
   contract. Unknown active identity must block destructive admission for that
   profile; never infer the target provider from the host agent.

## Implementation in this audit

Remove only the unused `spawn_codex_read_only` entry point, superseded by the
owned-sandbox capture/evidence path, and the duplicate provider-api development
declaration of its existing runtime `tempfile` dependency. Workspace/reference and
Git-history checks show no consumers of the retired launcher. Keep the underlying
version-probe sandbox and all active evidence launchers. No plan, approval,
journal/recovery schema, outcome predicate, dispatch or MCP tool changes.

Documentation establishes the smaller active product path; the existing private
advanced adapters remain isolated by immutable schemas and explicit admission.
No new crate, generic lifecycle framework, feature flag or speculative migration
is necessary. Significant future provider/MCP authority changes require their own
reviewed design and acceptance tests.

## Safety retained

Exact identity, cross-session protection, ownership evidence, immutable approved
scope, immediate freshness, compatibility, independent human authorization,
journal-before-effects (including startup and lock creation), durable intent,
explicit partial/unknown outcomes and recovery/reconciliation remain mandatory.
The LLM cannot approve its own request. A terminal challenge is the existing local
human channel, with its documented same-user shell/PTY adversary limitation; it
must not be advertised as stronger identity authentication.

No real session deletion, provider-state edits or application restarts are
authorized in this audit. The current eight MCP tools remain read-only. New
destructive tools, native dispatch, Claude fallbacks, search services and bulk
operations are proposals for later milestones, not part of this decision's code.

## Alternatives and consequences

Continuing Desktop-first sequencing delays the user workflow. Deleting advanced
code solely because it is deferred loses tested safety and compatibility evidence.
Merging all handoffs/outcomes obscures different effect and replay contracts.
Keeping the existing boundaries and retiring only demonstrated redundancy is
smaller and preserves recovery compatibility, at the cost of retaining some large
private modules whose stronger capabilities remain unadmitted.

P0 can begin with constraints: provider scope proof, useful metadata/resolution,
caller-session protection and a trusted human/MCP handoff remain missing. There
is no demonstrated crate-architecture blocker. Full initial-provider readiness
cannot be claimed until both providers complete real disposable end-to-end tests.

## Validation

Run fmt, locked workspace Clippy/tests/build, architecture fitness, both static
compatibility validators, historical v2 plan/binding/journal regression, native
scope guards and the late-writer counterexample. Check all eight tools and their
approval rejection tests remain intact. Verify exact final CI on Linux, macOS
and Windows. Preserve historical evidence bytes. The
[audit](../audits/2026-11-07-product-first-audit.md) records capabilities, module
classification, simplification decisions and the next vertical slices. Future
admission needs behavior tests for the approved scope, not relaxed source guards.
