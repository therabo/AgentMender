# Module inventory — consolidation audit, 2026-11-08

Dated audit evidence, not a second roadmap. All 64 source files in the eight-crate
workspace are classified below. This refreshes the historical 59-file inventory
with the five P0-C1 files. Decisions use source/module/export references, test
consumers, platform cfgs, Git history, direct dependency uses and warnings-denied
Clippy. A source file may contain meaningful regression tests as well as runtime
code. Deferred and compatibility-gated modules are not automatically dead.

| Module | Phase / reuse | Responsibility / consumers | Decision |
|---|---|---|---|
| [application/conversation.rs](../../crates/application/src/conversation.rs) | P0 / Shared safety | Metadata selection, registered exclusion, protected preview/prepare, durable status | Keep; shared filter corrected in both modes; no grant/dispatch |
| [application/lib.rs](../../crates/application/src/lib.rs) | P0 / Shared | Query dispatch, fresh discovery, exact selectors, integrity/planning inputs | Keep; diagnostic inventory distinct from management candidates |
| [application/lifecycle.rs](../../crates/application/src/lifecycle.rs) | P0 / Shared P1/P2 | Approved execution, per-action progress and read-only reconciliation | Keep distinct schema handoffs; no generic rewrite |
| [application/transactions.rs](../../crates/application/src/transactions.rs) | Shared P0 | Read-only transaction queries and pending-recovery coordinator | Keep; no dispatch authority |
| [cli/approvals.rs](../../crates/cli/src/approvals.rs) | Shared P0 | Independent terminal review/challenge, preparation and grant administration | Keep; model cannot substitute for human |
| [cli/conversation.rs](../../crates/cli/src/conversation.rs) | P0 / Shared safety | Trusted fixed-root read/removal composition and UUID validation | Keep; model arguments cannot register caller identity |
| [cli/lifecycle.rs](../../crates/cli/src/lifecycle.rs) | P0 / Shared P2 | Exact adapter composition, protected preflight and qualified reporting | Keep historical and conversational result meanings |
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
| [domain/session_metadata.rs](../../crates/domain/src/session_metadata.rs) | P0 / Shared read contract | Labels, exact identity, independent read compatibility and coverage | Keep; none grants authority; no content model yet |
| [domain/session_removal.rs](../../crates/domain/src/session_removal.rs) | P2 | Pure stronger persistent-projection assessment | Keep; no longer defines every P0 success |
| [domain/transaction.rs](../../crates/domain/src/transaction.rs) | Shared P0 | Typed journal events, immutable binding, replay and recovery knowledge | Keep historical schemas |
| [mcp-server/conversation.rs](../../crates/mcp-server/src/conversation.rs) | P0 | Strict conversational schemas, bounded label/preview/status projection | Keep; redundant internal flags removed, wire fields retained |
| [mcp-server/dto.rs](../../crates/mcp-server/src/dto.rs) | P0 | Strict bounded diagnostic metadata/plan schemas | Keep; conversational DTOs live in conversation.rs |
| [mcp-server/frame.rs](../../crates/mcp-server/src/frame.rs) | P0 / Shared | Frame bounds and duplicate-key rejection before SDK | Keep; not duplicate domain validation |
| [mcp-server/lib.rs](../../crates/mcp-server/src/lib.rs) | P0 | Thin module exports | Keep |
| [mcp-server/projection.rs](../../crates/mcp-server/src/projection.rs) | P0 | Ephemeral keyed identities, privacy projections, pagination/cursors | Keep; audit labels/lifetime gap before P0 expansion |
| [mcp-server/protocol.rs](../../crates/mcp-server/src/protocol.rs) | P0 | Eight default / thirteen opt-in tools, admission, cancellation and deadlines | Keep; instructions now disclose metadata-only and caller-binding limits |
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
| [provider-api/process.rs](../../crates/provider-api/src/process.rs) | Shared P0/P1 | Sandboxed fixed probes and owned evidence/schema launchers | Keep; retired launcher absent; no generic command authority |
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
| [provider-codex/metadata.rs](../../crates/provider-codex/src/metadata.rs) | P0 / P1 | Attested fixed-column adaptive reader and native schema observations | Keep; no transcript/config/approval state access |
| [provider-codex/planning.rs](../../crates/provider-codex/src/planning.rs) | P1 / P0 | Exact native capability descriptors without closed effects | Keep; schema support is not admission |
| [provider-codex/scoped_capture.rs](../../crates/provider-codex/src/scoped_capture.rs) | P0 fallback / P2 | Bounded target metadata, row/schema/retained commitments and evidence worker | Keep |
| [provider-codex/scoped_cleanup.rs](../../crates/provider-codex/src/scoped_cleanup.rs) | P0 fallback / P2 | Consumed-intent exact primary execution; schema-3 progress; Desktop routing | Keep; no automatic native fallback |
| [provider-codex/scoped_cleanup_acceptance.rs](../../crates/provider-codex/src/scoped_cleanup_acceptance.rs) | Shared test infrastructure | Disposable approved-adapter, exact binary opt-ins and crash acceptance | Keep; test composition does not grant production bypass |
| [provider-codex/scoped_config.rs](../../crates/provider-codex/src/scoped_config.rs) | Shared primary safety | Conservative config/root-selection rejection | Keep; restrictive is not unused |
| [provider-codex/scoped_files.rs](../../crates/provider-codex/src/scoped_files.rs) | Shared P0/P2 | Descriptor-relative physical identity, locks, payload, sidecars and reference checks | Keep; removed unnecessary module-wide dead-code suppression |
| [provider-codex/scoped_profile.rs](../../crates/provider-codex/src/scoped_profile.rs) | P1 / P0 fallback | Pure exact 0.166.0 Legacy/WAL primary admission | Keep; separate from discovery compatibility |
| [provider-codex/scoped_storage.rs](../../crates/provider-codex/src/scoped_storage.rs) | P0 fallback / P2 | Static exact-key SQL, schema/dependency/retained-state and real WAL exclusion | Keep |
| [provider-codex/scoped_unsupported.rs](../../crates/provider-codex/src/scoped_unsupported.rs) | P1 safety | Non-Unix fail-closed stub for the same public composition API | Keep; cross-platform consumer, not duplicate executor |

No production module or package is removed. All direct dependencies have source
consumers (including target-specific and test-only uses); no Cargo manifest or
lockfile changes are justified. The retired launcher/dependency from ADR-0022
was already absent at the starting commit. Public serialization/replay contracts
and no-follow/platform denial stubs remain required consumers.

The standalone synthetic onboarding generator is removed: no test/CI caller,
normal onboarding now reads the installed provider, and application/stdio tests
already cover its useful search invariants. No tracked generated provider root
or test binary was found. The 44 discovery fixture files, 133 compatibility files,
proptest seeds and historical plan/journal fixtures remain useful regression or
provenance inputs. Three validators remain distinct: source/authority fitness,
native effect evidence, and complete-removal inventories. Native disposable
probes and sanitized capture/live read validators remain reproducible research
tooling and are not production paths.

See [search and management identity](../session-search.md) for the path-by-path
exclusion audit and exact next-step contract. Existing Desktop JSON/catalog/
legacy reducers retain compiled non-admission and the child-process lost-update
counterexample; removing them would discard evidence needed by P1/P2.
