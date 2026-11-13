# Architecture decisions

- [ADR-0029: Compressed native rollout history](0029-compressed-rollout-history.md) — Accepted; bounded local representation decoding in the existing isolated reader, without native startup or host transcript ingestion.

[ADR-0022](0022-product-first-session-lifecycle.md) governs current product
sequencing and separates primary/native lifecycle from deep cleanup. Older ADRs
retain their operation-specific safety contracts and historical evidence.
ADR-0018–0021 no longer impose complete Desktop removal as a global MVP gate;
their stronger complete-Desktop claim remains gated. M9 restrictions remain
version/operation-specific; no Claude mutation is enabled.

| Decision | Status |
|---|---|
| [0001: Read-only hexagonal foundation](0001-read-only-foundation.md) | Accepted |
| [0002: Real installation evidence](0002-real-installation-evidence.md) | Accepted |
| [0003: Disposable Codex evidence](0003-disposable-codex-evidence.md) | Accepted |
| [0004: Worker sandbox attestation](0004-worker-sandbox-attestation.md) | Accepted |
| [0005: Exact Codex capture profiles](0005-exact-codex-capture-profiles.md) | Accepted |
| [0006: Evidence-derived Artifact Graph](0006-evidence-derived-artifact-graph.md) | Accepted |
| [0007: Evidence rules and conservative ownership](0007-evidence-rules-and-conservative-ownership.md) | Accepted |
| [0008: Read-only integrity proofs](0008-read-only-integrity-proofs.md) | Accepted |
| [0009: Immutable evidence-bound planning](0009-immutable-evidence-bound-planning.md) | Accepted |
| [0010: Read-only MCP boundary and private bounded views](0010-readonly-mcp-boundary.md) | Accepted |
| [0011: Durable transaction knowledge before provider execution](0011-durable-transaction-knowledge.md) | Accepted |
| [0012: Independent local approval and atomic authorization handoff](0012-independent-local-approval.md) | Accepted |
| [0013: Exact Codex unarchive through a consumed durable handoff](0013-codex-durable-native-handoff.md) | Accepted |
| [0014: Claude project lifecycle evidence gate](0014-claude-project-lifecycle-evidence-gate.md) | Accepted historical safety boundary; M9 BLOCKED under 0015 |
| [0015: Current Claude purge provider blockers](0015-current-claude-purge-provider-blockers.md) | Accepted; M9 BLOCKED; preserves 0014 safety boundary |
| [0016: Current Codex cleanup provider-contract gate](0016-current-codex-cleanup-provider-contract-gate.md) | Accepted; M8.1 native full cleanup BLOCKED; integrated |
| [0017: Scoped cleanup, residual preservation and provider-local mutation](0017-scoped-cleanup-residual-preservation-provider-local-mutation.md) | Accepted; M8.2 historical primary-cleanup PASS, integrated |
| [0018: Complete Session Removal, Projection Consistency and Maximum Safe Cleanup](0018-complete-session-removal.md) | Accepted complete-Desktop assessment contract; historical OPEN, now deferred; global sequencing superseded by 0022 |
| [0019: Structural removal authority and isolated profile acceptance](0019-structural-removal-and-profile-acceptance.md) | Accepted structural contract; partially supersedes 0018; new profiles gated |
| [0020: Closed Desktop complete removal and qualified persistent actions](0020-closed-desktop-complete-removal.md) | Accepted design; explicit remain-closed operating prerequisite and [unadmitted exact-build candidates](../milestones/m8-2-1-desktop-admission-2026-11-06.md) |
| [0021: Structural admission and manual Desktop acceptance](0021-structural-admission-and-manual-desktop-acceptance.md) | Accepted. Structural verification gates admission. User observation gates graphical acceptance. |
| [0022: Product-first session lifecycle and separation of primary removal from deep cleanup](0022-product-first-session-lifecycle.md) | Accepted; supersedes global Desktop/project-first sequencing, preserves existing authority and safety contracts |

M8.1's integration/base is recorded in [the receipt](../milestones/m8-1-integration.md).
ADR-0017 partially superseded native-only restrictions for the reviewed exact
scoped-local Codex path. ADR-0022 changes sequencing without changing negative
proofs, provider pins, plan/journal schemas, independent approval or current MCP
read-only authority. The [canonical roadmap](../product-roadmap.md) and
[audit](../audits/2026-11-07-product-first-audit.md) describe present capability.

- [ADR-0023: Conversational primary-removal handoff](0023-conversational-primary-removal-handoff.md) — accepted bounded preparation/metadata/exclusion design; actual client/human acceptance OPEN.

[ADR-0024](0024-adaptive-read-only-metadata.md): adaptive read-only metadata separate from destructive compatibility (accepted 2026-11-08).

[ADR-0025](0025-bounded-search-and-management-exclusion.md): bounded conversation
search and verified management exclusion (accepted requirements, 2026-11-08;
registered filtering fixed, content/binding implementation deferred).

- [0026 — Bounded conversation reads and independent management registration](0026-bounded-conversation-reader-and-independent-registration.md): implemented read-only search; real updated client/binding gates OPEN.

- [ADR-0027: Structured queries and incremental conversation retrieval](0027-intelligent-session-retrieval.md) — Accepted; extends search without lifecycle authority.

- [ADR-0028: Search reliability and query-scoped freshness](0028-search-reliability-and-scoped-freshness.md) — Accepted; P0-SR1 source diagnostics, scoped revalidation, context progress and incomplete-evidence acceptance.

- [ADR-0029: Compressed native rollouts](0029-compressed-rollout-history.md) — Accepted SR2 representation; resource strategy extended by ADR-0030.
- [ADR-0030: Bounded resident history decoding](0030-resident-bounded-history-decoding.md) — Accepted SR3 worker reuse, replay, physical/logical coverage; host admission remains closed.
- [ADR-0031: Separate caller-filtered discovery from management](0031-metadata-derived-caller-discovery.md) — Accepted SR4 read-only apparent-caller scope and distinct discovery tokens.
- [ADR-0032: Caller-scoped session resolution](0032-caller-scoped-session-resolution.md) — Host-side Luna reasoning with bounded read-only acquisition and citation validation.

- [ADR-0033 — SearchIntent v1](0033-search-intent-v1.md)
- [ADR-0034 — Progressive session resolution](0034-progressive-resolution.md)

- [ADR-0035 — UUID cohort for conversational retrieval](0035-uuid-search-cohort.md)

- [ADR-0036](0036-two-stage-luna-resolution.md) records the separate Planner, exhaustive acquisition, Evaluator and publication phases.

- [ADR-0037 — Autonomous MCP session finding](0037-autonomous-mcp-session-find.md)

- [ADR-0038 — Native Codex deletion admission](0038-native-codex-delete-admission.md)

- [ADR-0039 — Luna-planned hybrid retrieval and verified publication](0039-luna-hybrid-query-publication.md)
