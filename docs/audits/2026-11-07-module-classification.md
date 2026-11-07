# Module classification — product-first audit, 2026-11-07

Companion to [the audit](2026-11-07-product-first-audit.md) and [ADR-0022](../adr/0022-product-first-session-lifecycle.md). Every `crates/*/src/*.rs` file is classified below. Primary responsibility precedes secondary reuse. “Deferred” describes product priority, not code deletion or production admission. Tests embedded in source stay with the protected capability.

| Module | Primary phase / reuse | Responsibility | Decision |
|---|---|---|---|
| [application/lib.rs](../../crates/application/src/lib.rs) | P0 / Shared | Query dispatch, fresh discovery, exact selectors, integrity/planning inputs | Keep; no search service implemented |
| [application/lifecycle.rs](../../crates/application/src/lifecycle.rs) | P0 / Shared P1/P2 | Approved execution, per-action progress and read-only reconciliation | Keep distinct schema handoffs; no generic rewrite |
| [application/transactions.rs](../../crates/application/src/transactions.rs) | Shared P0 | Read-only transaction queries and pending-recovery coordinator | Keep; no dispatch authority |
| [cli/approvals.rs](../../crates/cli/src/approvals.rs) | Shared P0 | Independent terminal review/challenge, preparation and grant administration | Keep; model cannot substitute for human |
| [cli/lifecycle.rs](../../crates/cli/src/lifecycle.rs) | P0 / Shared P2 | Version-aware composition and primary/complete reporting | Keep; document current exit-2 coupling, no behavior change |
| [cli/main.rs](../../crates/cli/src/main.rs) | P0 / Shared | Trusted provider/root composition, CLI/read-only MCP entry, fixed workers | Keep |
| [cli/transactions.rs](../../crates/cli/src/transactions.rs) | Shared P0 | Read-only transaction/recovery presentation and protected-root inventory | Keep |
| [domain/approval.rs](../../crates/domain/src/approval.rs) | Shared P0 | Human-review eligibility, bound scope and signing messages | Keep |
| [domain/cleanup.rs](../../crates/domain/src/cleanup.rs) | P0 fallback / P2 | Exact primary manifests/reports and logical accounting | Keep; primary and auxiliary roles already distinct |
| [domain/complete_removal.rs](../../crates/domain/src/complete_removal.rs) | P2 / Shared primary | Schema-3 expanded primary and schema-4 Desktop contracts, ordered action receipts | Keep; schema-4 admission empty |
| [domain/graph.rs](../../crates/domain/src/graph.rs) | P2 / Shared P0/P1 | Deterministic evidence graph, roots, references and conservative ownership | Keep; current planners and inspection consume it |
| [domain/integrity.rs](../../crates/domain/src/integrity.rs) | Shared P0/P1/P2 | Pure health/negative-proof evaluator | Keep exact M3 meanings |
| [domain/lib.rs](../../crates/domain/src/lib.rs) | P0 / Shared | Provider/session identity, observations, scope and compatibility facts | Keep; do not add NLP here |
| [domain/path_serde.rs](../../crates/domain/src/path_serde.rs) | Shared | Lossless native path serialization | Keep; old hashes depend on it |
| [domain/planning.rs](../../crates/domain/src/planning.rs) | P0 / Shared | Immutable native/scoped plans, freshness, typed effects and versioned hashes | Keep; future narrow native admission is new work |
| [domain/session_removal.rs](../../crates/domain/src/session_removal.rs) | P2 | Pure stronger persistent-projection assessment | Keep; no longer defines every P0 success |
| [domain/transaction.rs](../../crates/domain/src/transaction.rs) | Shared P0 | Typed journal events, immutable binding, replay and recovery knowledge | Keep historical schemas |
| [mcp-server/dto.rs](../../crates/mcp-server/src/dto.rs) | P0 | Strict bounded metadata/plan view schemas; current intents read-only | Keep; useful labels and removal DTOs are missing |
| [mcp-server/frame.rs](../../crates/mcp-server/src/frame.rs) | P0 / Shared | Frame bounds and duplicate-key rejection before SDK | Keep; not duplicate domain validation |
| [mcp-server/lib.rs](../../crates/mcp-server/src/lib.rs) | P0 | Thin module exports | Keep |
| [mcp-server/projection.rs](../../crates/mcp-server/src/projection.rs) | P0 | Ephemeral keyed identities, privacy projections, pagination/cursors | Keep; audit labels/lifetime gap before P0 expansion |
| [mcp-server/protocol.rs](../../crates/mcp-server/src/protocol.rs) | P0 | Eight-tool rmcp registry, request admission, cancellation/deadline/drain | Keep registry unchanged in audit |
| [persistence/handoff.rs](../../crates/persistence/src/handoff.rs) | Shared P0/P1/P2 | Distinct native, primary and expanded typed consumed-intent ports | Keep; differences carry effect/report/progress contracts |
| [persistence/lib.rs](../../crates/persistence/src/lib.rs) | Shared | Public protected storage/handoff exports | Keep |
| [persistence/owned.rs](../../crates/persistence/src/owned.rs) | Shared P0 | Private non-overlapping root, permissions, identity and publication | Keep |
| [persistence/recovery.rs](../../crates/persistence/src/recovery.rs) | Shared P0 | Bounded typed content-addressed reconciliation metadata | Keep; not conversation backup/restore |
| [persistence/store.rs](../../crates/persistence/src/store.rs) | Shared P0 | Append-only dual history, MAC approval, sealed durable intent, replay/migration | Keep; high complexity protects failure/authority boundaries |
| [persistence/windows_permissions.rs](../../crates/persistence/src/windows_permissions.rs) | Shared P0/P1 | Current-user ACL and ownership wrapper | Keep target-specific protection |
| [provider-api/desktop_process.rs](../../crates/provider-api/src/desktop_process.rs) | P1/P2 | Qualified Desktop process/writer census | Keep; not a lifetime fence |
| [provider-api/evidence.rs](../../crates/provider-api/src/evidence.rs) | Shared P0/P1 | Opaque fresh disposable evidence sandbox | Keep |
| [provider-api/filesystem.rs](../../crates/provider-api/src/filesystem.rs) | Shared P0/P1/P2 | Bounded no-link metadata and safe read helpers | Keep |
| [provider-api/graph.rs](../../crates/provider-api/src/graph.rs) | Shared | Literal provenance/physical graph helpers | Keep |
| [provider-api/lib.rs](../../crates/provider-api/src/lib.rs) | Shared P0 | Read-only ProviderAdapter and ScopedCleanupReadPort | Keep separate from mutation authority |
| [provider-api/macos_sandbox.rs](../../crates/provider-api/src/macos_sandbox.rs) | Shared | Reviewed macOS ABI policy, process and metadata queries | Keep sole unsafe exception; no expansion |
| [provider-api/process.rs](../../crates/provider-api/src/process.rs) | Shared P0/P1 | Sandboxed fixed probes and owned evidence launchers | Simplify: remove superseded unused launcher |
| [provider-claude/graph.rs](../../crates/provider-claude/src/graph.rs) | P0 / Shared P2 | Claude evidence interpretation and protected references | Keep |
| [provider-claude/integrity_tests.rs](../../crates/provider-claude/src/integrity_tests.rs) | Shared test infrastructure | Partial-proof provider regressions | Keep tests only |
| [provider-claude/lib.rs](../../crates/provider-claude/src/lib.rs) | P0/P1 | Detection, unknown-live metadata, synthetic Code and observed Desktop identity | Keep; no mutation or general SDK history bridge |
| [provider-codex/capture.rs](../../crates/provider-codex/src/capture.rs) | P0/P1 | Kernel-isolated read-only SQLite capture and scrubbed mirror | Keep; fixed native evidence path replaces retired launcher |
| [provider-codex/desktop_catalog.rs](../../crates/provider-codex/src/desktop_catalog.rs) | P2 / P1 research | Host-qualified exact catalog schema, retained commitments, guarded effects | Defer; component-tested private candidate |
| [provider-codex/desktop_catalog_capture.rs](../../crates/provider-codex/src/desktop_catalog_capture.rs) | P2 / Shared evidence | Protected catalog read worker and sealed effect boundary | Defer candidate; preserve source write denial |
| [provider-codex/desktop_cleanup.rs](../../crates/provider-codex/src/desktop_cleanup.rs) | P2 / P1 research | Exact bundle candidate, closed-writer preflight and schema-4 sequence | Defer; no admitted production profile |
| [provider-codex/desktop_json.rs](../../crates/provider-codex/src/desktop_json.rs) | P2 | Lossless byte-span JSON parser/reducer support | Defer; not interchangeable with generic serde reserialization |
| [provider-codex/desktop_legacy.rs](../../crates/provider-codex/src/desktop_legacy.rs) | P2 research | Bounded legacy browser reconstruction inspection | Defer; preserves evidence of unknown source roles |
| [provider-codex/desktop_shared.rs](../../crates/provider-codex/src/desktop_shared.rs) | P2 research | Lossless index/global reducers, sealed publication and late-writer regression | Defer; target global publication remains refused |
| [provider-codex/graph.rs](../../crates/provider-codex/src/graph.rs) | P0 / Shared P2 | Codex snapshot-to-graph interpretation | Keep |
| [provider-codex/integrity_tests.rs](../../crates/provider-codex/src/integrity_tests.rs) | Shared test infrastructure | Conservative partial graph/negative-proof regressions | Keep |
| [provider-codex/lib.rs](../../crates/provider-codex/src/lib.rs) | P0/P1 | Provider detection, bounded App Server transport, normalized discovery | Keep; exact historical discovery profiles |
| [provider-codex/lifecycle.rs](../../crates/provider-codex/src/lifecycle.rs) | P1 / Shared P0 | Pinned native unarchive handoff, observation and sandboxed launcher | Keep; delete/archive unsupported, real plans blocked |
| [provider-codex/lifecycle_live_tests.rs](../../crates/provider-codex/src/lifecycle_live_tests.rs) | Historical/experimental tests | Opt-in actual-binary synthetic unarchive and profile regressions | Keep; no live user-state tests |
| [provider-codex/planning.rs](../../crates/provider-codex/src/planning.rs) | P1 / P0 | Exact native capability descriptors without closed effects | Keep; schema support is not admission |
| [provider-codex/scoped_capture.rs](../../crates/provider-codex/src/scoped_capture.rs) | P0 fallback / P2 | Bounded target metadata, row/schema/retained commitments and evidence worker | Keep |
| [provider-codex/scoped_cleanup.rs](../../crates/provider-codex/src/scoped_cleanup.rs) | P0 fallback / P2 | Consumed-intent exact primary execution; schema-3 progress; Desktop routing | Keep; no automatic native fallback |
| [provider-codex/scoped_cleanup_acceptance.rs](../../crates/provider-codex/src/scoped_cleanup_acceptance.rs) | Shared test infrastructure | Disposable approved-adapter, exact binary opt-ins and crash acceptance | Keep; test composition does not grant production bypass |
| [provider-codex/scoped_config.rs](../../crates/provider-codex/src/scoped_config.rs) | Shared primary safety | Conservative config/root-selection rejection | Keep; restrictive is not unused |
| [provider-codex/scoped_files.rs](../../crates/provider-codex/src/scoped_files.rs) | Shared P0/P2 | Descriptor-relative physical identity, locks, sidecars, payload and reference validation | Keep; no weakening of writer/dependency checks |
| [provider-codex/scoped_profile.rs](../../crates/provider-codex/src/scoped_profile.rs) | P1 / P0 fallback | Pure exact 0.166.0 Legacy/WAL primary admission | Keep; separate from discovery compatibility |
| [provider-codex/scoped_storage.rs](../../crates/provider-codex/src/scoped_storage.rs) | P0 fallback / P2 | Static exact-key SQL, schema/dependency/retained-state and real WAL exclusion | Keep |
| [provider-codex/scoped_unsupported.rs](../../crates/provider-codex/src/scoped_unsupported.rs) | P1 safety | Non-Unix fail-closed stub for the same public composition API | Keep; cross-platform consumer, not duplicate executor |

59 source files classified. P3 currently has discovery/project-association building blocks but no batch lifecycle module. There is no production NLP service. The only demonstrably retired production function in this audit is `process::spawn_codex_read_only`; its duplicate dependency declaration is also removed. No module was deleted wholesale.

## Tests and non-Rust surfaces

| Surface | Classification and decision |
|---|---|
| `crates/domain/tests/*` and stored proptest regressions | Shared safety, P0/P1/P2; retain exact old v2 plan/binding/journal fixtures and destructive properties |
| `crates/application/tests/*` | Shared P0 orchestration; fake-port approvals/effects are components, not actual client/provider acceptance |
| `crates/persistence/tests/*` and shared test fixtures | Shared P0 crash/replay/authority/permissions; retain |
| `crates/cli/tests/*` | P0 CLI and read-only stdio integration; preserve fixture byte manifests and Claude refusal |
| `crates/mcp-server/tests/protocol.rs` | P0 protocol/privacy/authority rejection, not destructive workflow acceptance |
| `crates/provider-claude/tests/lifecycle_guards.rs` | P1 version/operation refusal and zero-profile guards |
| `crates/provider-codex/tests/scoped_sqlite_fence.rs` | Shared P0/P2 cross-process actual WAL exclusion |
| `fixtures/codex/*`, `fixtures/claude/*` | P0/P1 sanitized/synthetic discovery evidence; provenance classes stay separate |
| `compatibility/codex/0.165.0-alpha.1.2/*`, `0.166.0/*` | P1 native schemas, exact binary pins and counterexamples; retain |
| `compatibility/codex/scoped-cleanup-v1/*`, `scoped-cleanup-v2/*` | P0 fallback/P2 exact primary contracts; retain distinct schema histories |
| `compatibility/codex/complete-removal-research/*`, `desktop-complete-removal*/*` | P1/P2 static research and unadmitted candidates; preserve evidence bytes |
| `compatibility/claude/lifecycle-m9/*` | P1 historical installed/current-at-investigation native purge research; retain exact scope |
| `scripts/check_architecture.py` | Shared structural fitness including embedded violation canaries; keep without relaxation |
| `scripts/check_codex_lifecycle_evidence.py` | P1 static compatibility/counterexample validator; keep |
| `scripts/check_complete_removal_evidence.py` | P1/P2 static source/profile/inventory validator; keep even when work deferred |
| `scripts/capture_codex_fixture.py`, `verify_live_discovery.py` | P0/P1 explicitly opted-in evidence tooling; no CI/user-state mutation authority |
| `scripts/probe_codex_current_lifecycle.py`, `probe_claude_lifecycle.py`, `probe_claude_current_lifecycle.py` | P1 research reproducers; preserve, never execute on user sessions |
| `.github/workflows/ci.yml` | Shared Linux/macOS/Windows locked gates; retain pinned checkout and matrix |
| Cargo manifests, Cargo.lock, rustfmt.toml | Shared build configuration; no package version, feature or toolchain change |
| `build.rs` | None exists in the workspace |

Current source/test consumers and historical encoding explain retention; line count or future-phase status alone is not evidence of redundancy. See the audit evidence key for executable tests and acceptance limits.
