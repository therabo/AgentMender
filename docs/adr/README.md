# Architecture decisions

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
| [0018: Complete Session Removal, Projection Consistency and Maximum Safe Cleanup](0018-complete-session-removal.md) | Accepted product/assessment contract; M8.2.1 OPEN, no complete Desktop mutation profile |

| [0019: Structural removal authority and isolated profile acceptance](0019-structural-removal-and-profile-acceptance.md) | Accepted structural contract; partially supersedes 0018; new profiles gated |

M8.1 integration/base is recorded in [the receipt](../milestones/m8-1-integration.md)
(`c9dd04d` after integrated `6f798bc`). ADR-0017 accepts narrow partial
supersession for the independently reviewed exact-profile local cleanup path.
M3 integrity, MCP read-only authority, durable knowledge, independent approval and
Claude refusal retain their existing meanings. See [scoped cleanup status](../scoped-session-cleanup.md).
