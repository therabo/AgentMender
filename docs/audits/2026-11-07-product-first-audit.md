# AgentMender — Architecture Audit & MVP Readiness Assessment

Date: 2026-11-07, Europe/Rome. Verdict: **READY WITH CONSTRAINTS**.
Canonical priorities: [product roadmap](../product-roadmap.md).
Decision: [ADR-0022](../adr/0022-product-first-session-lifecycle.md).
This cycle ends at audit/refactoring/documentation; no next feature is implemented.

## Executive summary

AgentMender is currently a Rust safety and provider-evidence foundation with a
read-only MCP interface and a narrowly supported, separately approved Codex local
primary-cleanup path. It is not yet a conversational session-deletion product.
It can inventory supported metadata, retain uncertain identities and references,
inspect/explain them, evaluate integrity, build immutable intentions, persist
protected transaction knowledge and require independent terminal review. Exact
Codex cleanup can remove an admitted Legacy payload and canonical row while
preserving other sessions. It cannot presently perform that workflow through MCP.
Claude has partial discovery and no mutation adapter.

The architectural layers are coherent. The strongest complexity problem is the
development sequence and product-wide success criterion: exhaustive Desktop
projection closure became a prerequisite for delivering any conversational
removal. The code's existing private, typed boundaries already allow most of the
advanced work to remain without putting it on every native execution path.
Removing the entire graph, journal or Desktop research would discard useful
integrity/compatibility evidence and create migration risk without delivering P0.

The new roadmap puts conversational single-session lifecycle first, native
provider coverage second, extra artifact cleanup third and project/bulk lifecycle
fourth. Both Codex and Claude Code are required for the complete initial product.
Provider-native operations are preferred when their exact effects fit approval;
an operation's return code is never its verification. M8.2.1 remains
**unfinished/deferred, not PASS, and no longer a global blocker for the
conversational MVP**. Its lost-update regression and unadmitted schema-4 profiles
remain protected. M8.3 multi-session cleanup is not the next implementation.

## Reconstruction method and Git baseline

The actual paths are `/Users/daniel/Desktop/AgentMender` and
`/Users/daniel/Desktop/AI Agent cleaner` (lowercase `cleaner` on disk). The original
37-page Deep Research PDF was extracted/read, including visual review of its
architecture and transaction diagrams. The internal milestone file, cleanup
addendum, dated implementation/integration reports and verification receipts were
reviewed alongside the repository's README/AGENTS, ADRs 0001–0021, current contracts,
all crate manifests, module/API/reference inventory, test suites, fixtures,
compatibility manifests, scripts, CI and relevant history. Findings below separate
static/component evidence, retained historical acceptance and fresh execution.
No new live-session acceptance is inferred from historical reports.

| Baseline fact | Verified value |
|---|---|
| Starting branch | `milestone/m8-2-1-complete-session-removal` |
| Local and freshly fetched remote HEAD | `14a8931f38949d3dbde04a61b22091752a685402` |
| Local and remote main | `a284767157ace8769deee85b195b5b70efe374d8` |
| Unmerged M8.2.1 history | 13 commits above main, including schema-3, schema-4, writer/admission fixes and counterexample |
| Working tree at entry | Clean |
| Tags / release checkpoints | No Git tags; named milestone branches and dated exact-commit integration receipts supply checkpoints |
| New branch | `refactor/product-first-roadmap`, directly from verified M8.2.1 HEAD |
| Preserved branches | All existing milestone branches and main; no merge, squash or history rewrite |
| Baseline CI | [Run 42366724588](https://github.com/therabo/AgentMender/actions/runs/42366724588), exact 14a8931; independently checked in signed-in Safari |

Git fetch/ls-remote independently matched every existing local tracking head.
M0/M1–M8.2 branches are retained; M9 was integrated as blocker evidence and M8.1
as native full-cleanup investigation, not successful delete capabilities. The
GitHub connector returned 404 for this private repository; the user selected the
signed-in Safari session for final CI verification. Git SSH remains available.

The original research remains unchanged. It is useful architectural history, not
a current provider-version contract. In particular, its illustrative recovery
restore/encryption and provider mutation recommendations do not describe present
runtime capabilities. Later exact evidence and ADRs narrow them.

## Historical milestone assessment

| Milestone | Historical outcome | Actual reusable result / limitation |
|---|---|---|
| M0–M1 | Foundation and closeout PASS, qualified coverage | Provider/installation distinction, bounded inventory, scrubbed Codex mirror, observed Claude sidecars. Partial coverage is still partial; PATH miss is not absence. |
| M2 | PASS | Deterministic evidence graph, scoped identities, references, roots and conservative ownership. No current general rule proves all artifacts exclusively deletable. |
| M3 | PASS | Pure integrity evaluator with closed negative-proof prerequisites. Actual partial provider contexts remain Unknown; findings do not authorize repair. |
| M4 | PASS | Immutable semantic plans, canonical digests, exact target selection and fresh replan. Native actual profiles lack complete effects/integrity contracts and cannot Ready. |
| M5 | PASS | Eight strict read-only stdio tools with bounded privacy views. No preparation, approval or execution authority. |
| M6 | PASS | Private store, dual append-only history, recovery metadata, conservative crash/reopen knowledge. No conversation backup or general restore. |
| M7 | PASS | Terminal review/challenge, authenticated expiring one-shot grants, atomic consumption and durable intent. Independent local channel has a same-user shell/PTY adversary limit. |
| M8 | PASS for bounded handoff | Exact 0.165.0-alpha.1.2 native unarchive adapter and isolated synthetic native evidence. Actual discovery still cannot produce an eligible real native plan. Native archive/delete unsupported. |
| M8.1 | BLOCKED for investigated complete native cleanup | 0.166.0 exact schemas and real isolated counterexamples: hidden cascade, partial deletion, stale metadata and incomplete negative proof. No production mutation profile added. |
| M8.2 | PASS, integrated at a284767 | Exact 0.166.0 Darwin arm64 unshared Legacy/WAL primary cleanup, optional exact name-index action, collateral and crash acceptance. Meaningful primary removal/reclaim, not exhaustive Desktop erasure. |
| M8.2.1 | Historical OPEN; now unfinished/deferred | Pure complete-removal assessment, schema-3 expanded primary metadata/progress, schema-4 private Desktop candidates, source-role research, foreign-host and writer protections. Empty admission list; no complete Desktop PASS. |
| M9 | BLOCKED BY PROVIDER; investigation closed for that contract | Project-purge identity collisions, nested effects, writable dry-run and ambiguous/partial verification for investigated Code versions. Zero Claude mutation profiles. Does not prove all future session operations impossible. |

See [milestone history](../milestones/README.md) for original receipts. Older
PASS/OPEN/BLOCKED text remains unchanged, including reports that recommended an
older next milestone. ADR-0022 supersedes sequencing, not observations.

## Executable evidence key

All paths are repository-relative; normal suites use sanitized or fresh disposable
state. These are the responsible modules and tests behind the capability claims.

| Key | Implementation | Relevant verification and its limit |
|---|---|---|
| E1 | [Codex discovery](../../crates/provider-codex/src/lib.rs), [capture](../../crates/provider-codex/src/capture.rs) | Inline `fixture_discovers_archives_parents_and_projects_without_live_version_claim`, `partial_pages_remain_partial_and_visible`, `unknown_schema_stays_metadata_only`, capture scrub/schema tests; CLI `fixture_scan_and_inspect_round_trip`. Historical live read-only evidence is retained, not rerun on user state. |
| E2 | [Claude discovery](../../crates/provider-claude/src/lib.rs), [graph](../../crates/provider-claude/src/graph.rs) | `observed_metadata_maps_only_explicit_ids`, `live_state_even_matching_fixture_version_is_metadata_only`, `namespace_and_project_filter_remain_distinct`; CLI `observed_desktop_fixture_maps_seven_sessions`. Observed sidecars and synthetic transcript parser are different evidence classes. |
| E3 | [Application queries/selection](../../crates/application/src/lib.rs), [domain identity](../../crates/domain/src/graph.rs) | `plan_selector_matches_full_identity_on_the_same_observation`, `selector_ambiguity_and_pre_cancel_are_conservative`, `planning_discovers_once_and_resolves_identity_conservatively`; CLI ambiguous/partial target regression. No keyword or semantic search tests exist. |
| E4 | [MCP protocol](../../crates/mcp-server/src/protocol.rs), [DTOs](../../crates/mcp-server/src/dto.rs), [projection](../../crates/mcp-server/src/projection.rs), [frame](../../crates/mcp-server/src/frame.rs) | [Protocol suite](../../crates/mcp-server/tests/protocol.rs), [CLI stdio suite](../../crates/cli/tests/mcp.rs). Eight-tool registry, privacy, exact references, stale cursors, limits, busy/cancel/drain and authority rejection. These are real stdio fixture sessions, not actual Codex/Claude conversational removal. |
| E5 | [Native Codex](../../crates/provider-codex/src/lifecycle.rs), [descriptors](../../crates/provider-codex/src/planning.rs) | `only_unarchive_is_eligible_and_synthetic_plans_are_rejected`, `historical_contract_cannot_be_transferred_to_current_version_or_wider_scope`, `current_and_neighboring_versions_cannot_inherit_historical_authority`; retained isolated M8 evidence and ignored disposable native test. No delete dispatch. |
| E6 | [Primary Codex cleanup](../../crates/provider-codex/src/scoped_cleanup.rs), [capture](../../crates/provider-codex/src/scoped_capture.rs), [files](../../crates/provider-codex/src/scoped_files.rs), [SQL](../../crates/provider-codex/src/scoped_storage.rs) | [Acceptance module](../../crates/provider-codex/src/scoped_cleanup_acceptance.rs): `approved_adapter_removes_only_frozen_session_and_survives_reopen`, `expanded_profile_removes_target_metadata_and_preserves_foreign_metadata`, progress/crash tests; [WAL cross-process suite](../../crates/provider-codex/tests/scoped_sqlite_fence.rs). Actual exact-binary disposable acceptance is historical retained evidence; installed-binary tests remain explicit opt-ins. |
| E7 | [Approval policy](../../crates/domain/src/approval.rs), [terminal broker](../../crates/cli/src/approvals.rs), [store](../../crates/persistence/src/store.rs) | [Approval properties](../../crates/domain/tests/approval_properties.rs), store signed-field/expiry/consumption/crash tests, CLI `approval_and_revocation_reject_non_terminal_without_writes`. Tests do not authenticate a person against an arbitrary same-user agent with shell/PTY/key access. |
| E8 | [Execution/reconciliation](../../crates/application/src/lifecycle.rs), [handoff](../../crates/persistence/src/handoff.rs), [transaction model](../../crates/domain/src/transaction.rs) | [Application lifecycle](../../crates/application/tests/lifecycle.rs), [scoped orchestration](../../crates/application/tests/scoped_cleanup.rs), [transaction properties](../../crates/domain/tests/transaction_properties.rs), private-gate compile-fail doc tests. Fake ports prove orchestration only; E6 separately proves actual local effects on disposable state. |
| E9 | [Recovery metadata](../../crates/persistence/src/recovery.rs), [owned root](../../crates/persistence/src/owned.rs), [read coordinator](../../crates/application/src/transactions.rs) | [Owned/recovery tests](../../crates/persistence/tests/owned_recovery.rs), [store integration](../../crates/persistence/tests/store.rs), child abort/dual-history tests, CLI inspection byte manifests. Reconciliation observes; it cannot reconstruct receipt authority or restore the payload. |
| E10 | [Graph/integrity/planning](../../crates/domain/src/integrity.rs), [planning](../../crates/domain/src/planning.rs) | [Graph](../../crates/domain/tests/graph_properties.rs), [integrity](../../crates/domain/tests/integrity_properties.rs), [planning properties](../../crates/domain/tests/planning_properties.rs). Determinism, protected scope, false absence and forged-health rejection; no blanket ownership or deletion closure. |
| E11 | [Primary report](../../crates/domain/src/cleanup.rs), [complete contracts](../../crates/domain/src/complete_removal.rs), [assessment](../../crates/domain/src/session_removal.rs), [CLI report](../../crates/cli/src/lifecycle.rs) | [Scoped contracts](../../crates/domain/tests/scoped_cleanup.rs), [removal properties](../../crates/domain/tests/session_removal.rs): truthful residual/accounting, `historical_v2_plan_binding_and_journal_retain_their_exact_encoding`, persistent-source/admission refusal. Old receipts cannot claim new scope. |
| E12 | [Desktop private code](../../crates/provider-codex/src/desktop_cleanup.rs), [shared reducers](../../crates/provider-codex/src/desktop_shared.rs), [catalog](../../crates/provider-codex/src/desktop_catalog.rs), [legacy reader](../../crates/provider-codex/src/desktop_legacy.rs) | Exact catalog/reducer/legacy component tests, `late_writer_counterexample_requires_global_publication_refusal`, domain shared-publication blocker and empty compiled admission. No admitted schema-4 production end-to-end or graphical acceptance. |
| E13 | [Claude lifecycle guards](../../crates/provider-claude/tests/lifecycle_guards.rs), [CLI rejection](../../crates/cli/tests/transactions.rs) | Generated/deterministic zero-operation tests, `claude_unsupported_prepare_never_creates_state_or_grants`, `lifecycle_rejects_fixture_and_claude_configuration_before_state_access`. Historical native purge probes are research on synthetic data, not an adapter. |
| E14 | [Fitness](../../scripts/check_architecture.py), [native evidence validator](../../scripts/check_codex_lifecycle_evidence.py), [Desktop validator](../../scripts/check_complete_removal_evidence.py), [CI](../../.github/workflows/ci.yml) | Eight-crate graph, purity, exact authority/effects allowlists, violation canaries, preserved pins/manifests/counterexamples and three-OS locked gates. Static source checks supplement behavior tests; they do not prove ownership or production admission. |

## Functional capability audit

| Capability | Current classification | Evidence / actual usable boundary | End-to-end status |
|---|---|---|---|
| Codex session discovery | Implemented, narrowly version-gated, read-only | E1: exact 0.164.0-alpha.11.1 and 0.165.0-alpha.1.2 mirror/App Server profiles. Partial coverage. Other versions audit file metadata. | Historical protected live discovery; fresh fixture CLI/stdout checks. No current-version broad guarantee. |
| Claude session discovery | Implemented partial read-only; generic Code inventory missing | E2: observed Desktop 4.29 sidecar mapping to CLI IDs, sanitized fixtures; 2.1.311 transcript parsing is synthetic only. Unknown live Code has metadata-only inventory. | Historical seven-session partial Desktop smoke; fresh fixtures. No general installed-Code session listing acceptance. |
| Session search | Not implemented as a product capability | E3/E4: exact IDs/opaque references can select; no title/content field in Session/WireThread, no search API/service, MCP hides paths/time/native IDs. | MISSION1 README request cannot be reliably fulfilled today. |
| Project association | Implemented descriptive read-only | E1/E2: cwd grouping and explicit metadata, with provenance. Claude project has no native purge identity. | Fixture/read-only evidence; no project lifecycle authority. |
| Identity and ambiguity | Implemented exact-reference component | E3: full provider/root/class/local identity, fresh same-observation selectors and ambiguity/partial absence refusal. | Fixture integration works; user-friendly labels and active caller binding missing. |
| MCP integration | Implemented usable read-only stdio | E4: eight tools, SDK negotiation, bounds/privacy/cancellation. | Actual subprocess protocol fixture round trips; real two-client setup/removal acceptance not established. |
| Natural-language management through MCP | Read-only building blocks only; deletion workflow absent | The host can interpret a request but receives insufficient search labels and no removal/approval/execute tools. | No complete conversational deletion acceptance. |
| Native Codex lifecycle | Conditional historical unarchive component; archive/delete unsupported | E5: exact V165 private dispatch with strict gate. Real native plans blocked by incomplete effects/integrity. 0.166 research is not a registered execution profile. | Isolated synthetic unarchive evidence retained; no normally eligible real native removal. |
| Native Claude lifecycle | Research only / unsupported by current adapter | E13 and dated M9 evidence. Provider project purge exists; no lossless one-session scope. Background removal may preserve transcript and affect worktree. | No production dispatcher/profile or deletion acceptance. Not a provider-wide impossibility result. |
| Scoped local deletion fallback | Implemented, usable only under exact admitted conditions | E6: 0.166.0 Darwin arm64 Legacy/WAL primary cut. Schema-2 historical, schema-3 expanded metadata/progress. No CLI flag bypasses profile/config/fence checks. | Disposable production-path evidence retained and deterministic path freshly tested; no user sessions touched here. |
| Independent approval | Implemented local human channel; MCP integration missing | E7: exact plan/binding/recovery, short-lived MAC, one grant/attempt, fresh checks and atomic consume. | Fixture/store/CLI security tests; no actual conversational human broker acceptance. |
| Destructive transaction execution | Implemented narrow CLI/local orchestration | E6/E8: one semantic operation, sealed intent, typed effects and results. No general execution port. | Disposable scoped adapter acceptance; not accessible over MCP. |
| Result verification | Implemented per existing contract; complete Desktop admission disabled | E8/E10/E11: primary and retained proof at fenced cutoff; native verify conservative; projection assessment stronger. | Primary fixture acceptance works. Code success/command exit alone never proves erasure. |
| Recovery/reconciliation | Implemented protected metadata and read-only observation | E8/E9: dual history/replay, pending reports, frozen-object inspection. Unknowns stay protected after crash; no payload restore or auto-resume. | Extensive disposable child-crash/reopen tests; no global provider rollback claim. |
| Storage measurement | Implemented logical estimates/unlinks; physical reclaim unknown | E6/E11: frozen sizes, dedup/accounting constraints; physical_reclaimed_bytes None. | Sparse/logical accounting tests. SQLite rows, APFS clones/open handles/backups prevent a physical claim. |
| Deep artifact cleanup | Small exact subset implemented; broader P2 deferred | E6: primary/index and expanded attachment/guardian rows. E12: catalogs/global/legacy candidates. Logs/caches/snapshots/shared families mostly retained. | No exhaustive safe artifact removal or complete storage-reclaim workflow. |
| Project/bulk operations | Not implemented | Discovery can associate candidates; one target/semantic operation per transaction. | No aggregate approval, batch outcomes or batch recovery acceptance. |
| Version/platform compatibility | Implemented exact gates; limited production coverage | E1/E2/E5/E6/E13: detection/read-only/native/scoped axes differ. E14 CI validates fixtures on three OSs. | Unknown profiles remain protected; Linux/Windows CI does not admit native/local mutation there. |
| Graphical applications | Partial metadata evidence and unadmitted research | E12: candidate 24141/23880, qualified host/catalog identity, unresolved reconstruction and global writer gap. | No complete Desktop structural admission; graphical NotTested. Running app repaint is not primary verification. |
| Active management-session protection | Not implemented as caller exclusion | Writer fences and SessionState::Active concern provider state/presentation; MCP has no trusted host-session binding. | P0 blocker for destructive admission until exact client/target exclusion is tested. |

## Architecture assessment and phase classification

[Architecture](../architecture.md) gives the dependency table; the
[module map](2026-11-07-module-classification.md) assigns **every one of 59 source
files** and the build/test/fixture/script surfaces to P0/P1/P2/P3, shared,
research or redundancy. P3 has no batch implementation. No build.rs exists.
No new crate is justified: authority, provider-specific semantics and inbound
interfaces are already separated. Persistence's sealed handoff explains the
narrow application/Codex dependency on it. Moving that constructor into a generic
provider port would broaden authority.

The Codex adapter is the largest complexity concentration. It mixes several
versioned capabilities in private modules, but does not expose arbitrary SQL,
commands or paths to MCP. The native, cleanup and complete-removal handoffs look
similar because each needs preflight/dispatch/observe. They carry different
receipt/report/progress contracts and serialization histories. A generic executor
rewrite would cost more than it removes. Likewise dispatch, transaction and
complete-projection outcomes are distinct knowledge, not competing truth models.

Privacy is a real P0 product constraint. Original native discovery deliberately
strips preview/title content, while MCP also omits local IDs, path/project labels
and times. Exact opaque lookup alone cannot support meaningful conversational
search. Future work should choose minimal useful labels with reviewed disclosure
and ambiguity semantics, rather than adding a new language engine or exporting
raw transcripts. Same-process tokens are not durable plan/transaction references.

Native v1 planning's complete effects/M3 requirements currently prevent all real
native plans from Ready. A useful future narrowly scoped native operation needs
its own precise relevant-scope proof, not exhaustive unrelated artifact cleanup
and not globally changing Unknown to Healthy. No such admission is implemented
by this audit. The 0.166 fallback and historical list profiles also do not align;
that discovery/execution compatibility gap belongs in the first vertical slice.

## Simplifications performed and candidates retained

| Candidate / files | Decision and reference evidence | Preserved behavior / validation |
|---|---|---|
| `provider-api/src/process.rs::spawn_codex_read_only` | Removed 25-line public helper with no production/test/doc consumer. Git `-S` history shows older direct startup superseded by c25a386's capture/scrub/owned evidence path. All current callers use spawn_codex_capture/scoped_capture/catalog_capture/evidence. | Probe sandbox, source write denial, owned workers and active discovery unchanged. Targeted provider-api tests, full workspace and fitness pass. This removes an unused workspace API; unknown out-of-tree callers would need the supported owned-sandbox path. |
| `provider-api/Cargo.toml` duplicate `[dev-dependencies] tempfile` | Removed 2 redundant lines; same package/features already present as runtime dependency. No independent feature selection/consumer. | Tests still use runtime tempfile; no Cargo.lock/package-version change. |
| Root README/AGENTS and current status documentation | Replaced the universal Desktop-first roadmap with one canonical P0–P3 roadmap and ADR-0022. Removed obsolete active-milestone instructions from current contributor guidance. | Historical receipts, ADR bodies and evidence remain; no runtime admission or authorization is changed. |
| Multiple lifecycle ports/planners/outcomes | Kept. E5/E6/E8/E11 and historical replay demonstrate separate native/primary/ordered projection semantics. Cleanup branches in one domain planner already share hashes/approval rather than competing authorities. | No schema/digest/grant upgrade or crash regression risk introduced. |
| `ScopedCleanupReadPort` versus `ProviderAdapter` | Kept. General inventory lacks the exact physical/schema/dependency evidence needed for primary deletion; dedicated observation grants no effect authority. | Existing narrow Ready path remains separate from read-only compatibility. |
| Desktop catalog/global/legacy code and LevelDB/Snappy dependencies | Kept private/deferred. Reducers, oracle tests, reconstruction readers, sealed sequencing and counterexample have actual consumers and P1/P2 reuse. | Empty schema-4 admission and global target-publication refusal unchanged. No unsafe JSON effects enabled. |
| Non-Unix unsupported adapter and platform cfg branches | Kept. CLI/public composition consumes these on other platforms; not dead duplicates. | Cross-platform fail-closed behavior and CI remain. |
| General graph/ownership/integrity/recovery infrastructure | Kept. Current query/planning, authority and historic serialization consume it. Useful across phases even where a future native operation needs a smaller relevant proof. | Negative proofs, other-session safety, MAC/grant freshness, durable intent and replay remain intact. |
| Generic per-module splitting / feature flags | Rejected. No clear new responsibility/security boundary needs a crate; extra runtime flag does not supply admission. | Simpler maintainable dependency graph, no speculative configuration. |

The source edit is intentionally small. Refactoring does not mean deleting
advanced work merely because it is not next. Significant removed code is fully
listed above; nothing else was removed from production. Existing tests were
retained unchanged. No recovery format, plan digest, transaction event, provider
profile, destructive scope or MCP schema changed.

## Deferred advanced functionality

Track native version/session-type coverage and writer coordination under P1.
Track complete Desktop catalogs/scans, global main/backup, renderer/legacy import,
host-qualified pins/projects/search, extra snapshots/history/log/cache artifacts
and maximum safe reclaim under P2. Track M8.3 project/bulk under P3. M10 platform
hardening becomes capability-specific rollout work; M11 recovery key/retention
design and M12 repair remain later, with no false current restore/encryption claim.

The loss-update counterexample is preserved in E12: a real child atomically
publishes an unrelated peer update after the final destination check, then the
private replacement loses it while its final digest succeeds. Production refuses
the target global publication; domain plans report
SharedWriterCoordinationUnavailable and zero operations. Process census, remain-
closed instructions, advisory inode locks and compare/rename do not invalidate
that result. Completing P0 never grants permission to bypass it.

## Current provider strategy and semantics

Codex: `thread/delete` is a **candidate native interface**, not an admitted
AgentMender delete adapter. The freshly checked
[official App Server documentation](https://learn.chatgpt.com/docs/app-server)
describes deletion of the persisted target plus spawned descendants. That supports
the need for effect-set checking; it does not establish a closed target-only
contract for the pinned installed version. Existing E5 and 0.166 evidence remains
the executable limit. A leaf-looking list result is insufficient if hidden
descendants or startup effects remain unclosed. Native-first does not mean native
at any cost. An exact existing E6 fallback may be the first deliverable path if
its actual supported root/history/config/fences satisfy the selected request.

Claude: freshly checked [official CLI documentation](https://code.claude.com/docs/en/cli-reference)
describes `purge` as project-scoped and `rm` as background-list removal with a
resumable transcript; worktree effects also require scrutiny. These are different
operations from one-session transcript deletion. M9's historical 2.1.222/308/314
research adds exact-version counterexamples, but the adapter supports none of
those mutation profiles. No new installed-version assertion or native probe was
made in this audit. A narrow future local fallback is a research option requiring
its own approved contract, not an implementation promised safe here.

Use existing models, with explicit contract-qualified human descriptions:

| Observation | Permitted reporting |
|---|---|
| Verified archive | Archived; transcript/resumability retained per supported contract |
| Verified view/list removal | Removed from that view; report transcript retention/resumability |
| E6 required payload and canonical row absent, collateral verified | Primary session state deleted at the fenced cutoff; disclose residuals |
| Future verified native deletion | Native session deletion verified for the stated target/effect set/postconditions |
| Verified primary plus optional residuals | Primary removal with retained artifacts, not every trace erased |
| No safe compatible operation | Unsupported or Blocked, with exact prerequisite |
| Possible effects / incomplete verification | OutcomeUnknown/RecoveryRequired; never automatic retry |

These are semantic descriptions, not new Rust variants. Current CLI continues
to expose primary fields and a stronger projection assessment/exit 2. This audit
does not repurpose old Removed/RemovedWithResiduals encodings or silently change
exit behavior. A future P0 result must be scoped to its requested capability.

## MCP readiness and trust boundary

Current tools are `providers`, `workspace_scan`, `inspect`, `graph`, `explain`,
`verify_integrity`, `plan_lifecycle` and `repair_plan`. The last is planning-only
and unsupported for actual repair. There is no dedicated session search, scoped
cleanup preview tool, transaction preparation, approval request/handoff, approved
execution, transaction-result lookup or scoped removal verification over MCP.
The server's executor accepts only ReadOnlyRequest/ReadOnlyResponse and has no
store/key/receipt constructor. This is a deliberate trust boundary, not a nearly
finished delete API hidden behind a tool name.

Keep MCP inbound and provider-neutral. A future preparation/execute request may
orchestrate the same protected application service, but it cannot sign approval.
The human must review the exact original plan/digest in a channel the model does
not control. Existing terminal challenge/MAC/recovery machinery is reusable;
its external invocation and durable lookup need integration. If the client agent
can synthesize terminal input or access the authority key, terminal possession
does not meet a stronger adversary model. Resolve that deployment assumption
explicitly; do not equate a tool annotation or client identity with a human.

Caller-session identity is also missing. Explicit provider target identity is
already present, but an Active session label is not evidence of the current MCP
management chat, and writer-idle evidence does not establish caller exclusion.
P0 admission must reject the management session and unsupported active targets
before review and dispatch, across both clients and providers. An untrusted caller
cannot simply assert which protected session should be ignored.

## P0 readiness and gap analysis

| Capability | Current state | Existing implementation | Missing work | Risk | Next action |
|---|---|---|---|---|---|
| Codex discovery | Partial historical exact profiles | E1 capture/adapter, E3 query and scope | Align a supported discovery profile with the actual removal adapter; expose minimum useful labels | High: audit-only 0.166 scan cannot be assumed to list executable sessions | Select one exact vertical profile and fixture/live disposable evidence boundary |
| Claude discovery | Partial Desktop + synthetic Code only | E2 sidecar identity/graph and filesystem bounds | General supported Code inventory/identity for chosen operation | High: Desktop ID, Code ID and project path differ | Evaluate exact official structured history/interface; preserve provenance |
| MCP integration | Working read-only stdio | E4 SDK, frame, DTO, paging and cancellation | Actual client setup acceptance, safe durable removal-preparation/result surfaces | Medium/high: broadening interface authority | Reuse existing tools where useful; separately design the minimal new command flow |
| Natural-language resolution | Exact lookup, no useful search | E3 selectors, E4 references | Minimal human labels/search/filtering and ambiguity dialogue; agent interprets text | Medium: privacy and wrong-session selection | Define fixture title/project collisions; avoid transcript export/local NLP |
| Session inspection | Metadata/graph implemented | E3/E4 CLI and inspect/explain | Human-readable provider/project/title/exact ID disclosure with versioned privacy semantics | Medium: opaque token mistaken for native identity | Review minimum label/output contract and restart behavior |
| Active management-session protection | Missing caller binding | Existing exact provider/root identity and writer fences | Trusted client/session binding, explicit exclusion and dispatch-time recheck | High: deleting the management chat | Make both same-provider and cross-provider self-target refusal acceptance gates |
| Native delete adapter | Not implemented | E5 exact profile/transport patterns; E6 supported fallback | Closed exact effect set/startup scope and supported deletion proof, then narrow dispatcher if viable | High: hidden descendants and partial failure | Native candidate scope gate; use existing fallback only when already supported and chosen before approval |
| Independent approval | Local channel implemented | E7 policy, broker, MAC/expiry and store | MCP→human review handoff/lookup; explicit same-user client threat model | High: model self-approval or stale review | Reuse protected grant issuance; client cannot provide approval/key/principal |
| Delete execution | Exact primary CLI path | E6/E8 intent and typed effects | Invoke the already approved operation from reviewed application/MCP flow | High: receipt replay / unapproved effect expansion | No new unprotected executor; negative consumed/expired/stale/retry tests |
| Structural verification | Narrow primary proof and conservative native checks | E6/E8/E10/E11 | Define operation-specific native postconditions and retained commitments for P0 | High: successful command/list miss presented as deletion | Test independent primary absence plus unrelated sentinels; keep unsupported conclusions Unknown |
| User result reporting | CLI primary/complete distinction; no MCP removal result | E11 reports/bytes and categorical MCP errors | Contract-qualified success/residual/failure/unknown result through MCP and agent wording | Medium: complete-cleanup predicate hides useful primary success | Report verified capability and residuals separately; preserve old wire formats |
| Recovery | Strong metadata/replay foundation | E8/E9 dual history, frozen read-only reconciliation | Integrate transaction lookup and ambiguous client disconnect/restart in new flow | High: client repeats destructive call | Reopen tests and at-most-one dispatch; new approval required for new scope |

## Proposed next implementation milestones

These are proposals only. Each includes its own provider-compatibility gate and
ends in a user-visible workflow; none is started by this audit.

### P0-C1 — One conversational Codex removal

Outcome: from a dedicated Codex client, find/select one disposable Codex session,
preview exact removal, have a human approve in the independent channel, remove
through one supported adapter, verify structural postconditions and explain any
residuals. The management and unrelated sentinel sessions survive.

Scope: one exact local profile/platform, one session, minimum useful labels and
ambiguity handling, durable plan/transaction lookup, reviewed MCP command handoff,
caller-session exclusion and narrow results. Reuse E1/E3/E4/E6–E11. No new crate,
local LLM, artifact family, Desktop projection action or bulk API is needed.

Dependencies: prove target and caller identity; align discovery/execution; establish
an actually independent human interaction; define exact effect and verification
scope. First inspect `thread/delete` for that version. No native delete is currently
admitted. If its scope cannot be closed, an existing exact local E6 fallback is
the practical candidate, provided its Legacy/WAL/config/writer prerequisites
are met and approved before execution. Do not accept hidden-descendant risk just
because the fixture appears leaf-only. If neither is safe, the compatibility
gate refuses and the milestone is not declared complete.

Acceptance: actual client plus disposable exact provider state, genuine protected
plan→recovery→human approval→durable intent→adapter→fresh verification. Test equal
titles/different roots, stale target, unknown version, caller target, shared history,
active writers, hidden descendants, denied/partial effects, response loss,
grant expiry/reuse, client cancellation and restart. Assert no unrelated effect,
no self-approval and no second dispatch. Run full exact final local/three-OS gates.

Non-goals: all versions/platforms, all Desktop views, physical reclaim, additional
caches/artifacts, project cleanup, secure erasure or a new native implementation
when the scoped path already supplies the supported semantics.

### P0-C2 — Equivalent Claude session workflow

Outcome: a dedicated Claude Code client can find/inspect a disposable Claude
session, resolve its true Code identity, approve one exact supported removal and
receive a verified, semantically accurate result. Reuse the P0-C1 orchestration,
independent approval, durable lookup, caller exclusion and result presentation;
reuse E2/E13 as conservative discovery and refusal evidence.

Dependencies: P0-C1 common flow and a separately established exact Claude
single-session contract. Investigate documented session operations independently
of historical project purge. `rm` may supply a narrower remove-from-view workflow
only with its worktree/retention effects proven; it cannot satisfy transcript
deletion acceptance while the transcript remains resumable. Project purge cannot
be substituted. A local fallback needs separate version-gated ownership/writer
research and safety review; none exists today.

Acceptance: two unrelated disposable sessions in the same project; exact native
ID/sidecar mapping, ambiguity and active-caller refusal; human scope review;
startup-write, partial-failure and concurrency tests; independent transcript/resume
and retained-state postconditions appropriate to the actual operation; crash and
no-retry tests. If only view removal is safely supported, label that intermediate
delivery explicitly and keep the complete initial deletion product OPEN.

Non-goals: project-purge widening, undocumented broad JSONL unlinking, worktree or
user-file removal, sweeping temp state, remote/cloud cleanup or claiming provider
impossibility from one version's failure.

### P0-C3 — Cross-provider acceptance and initial product closeout

Outcome: both clients manage either provider using the same exact-target workflow
and truthful semantics. Dependencies are P0-C1/C2 and independently configured
local adapters/permissions, not complete Desktop cleanup. Reuse all proven
contracts; add only missing host/target composition and documented setup.

Acceptance: Codex→Codex, Claude→Claude, Codex→Claude and Claude→Codex actual-client
disposable tests; duplicate UUID/title/root cases, denied permissions, caller
protection, disconnect/restart/expiry and unrelated sentinel preservation. Both
providers must meet the declared session-removal contract before full P0 PASS.
Unsupported combinations remain honest refusals. Non-goals: new providers,
project/bulk, exhaustive artifact erasure or extra transport/OAuth infrastructure.

Then P1 expands native/version/platform profiles; P2 promotes additional artifact
families independently; P3 composes exact sessions. Their measurable gates and
dependencies are in the canonical roadmap. P2 is not a prerequisite for every
P3 primary-only batch, and P1's native-first rule also applies within P0.

## Outstanding risks and readiness verdict

**READY WITH CONSTRAINTS** means a bounded P0 implementation can start after
user selection; it does not mean P0 is ready to ship. There is no demonstrated
crate-architecture blocker requiring another infrastructure milestone. Concrete
gates are operation-specific provider scope/postconditions, discovery/removal
profile alignment, independent human integration, useful private metadata and
active management-session binding. Claude transcript deletion may require new
provider evidence or a separately reviewed fallback. Full two-provider deletion
cannot be promised before that work.

Retain these risks: native descendant and startup effects; provider drift;
metadata/identity collisions and incomplete discovery; prompt injection and
model-accessible approval channels; caller-session ambiguity; writer races;
non-atomic cross-store effects; response/journal crash gaps; recovery not being
a payload backup; graphical repaint being separate; and logical bytes differing
from physical allocation. No reduction in safety is justified by a narrower
product goal.

## Documentation changes and preservation

The README is product-focused; AGENTS now describes the current priorities and
unchanged safety constraints. Architecture/development docs explain the eight
crates, phase boundaries, authority, fitness and release checks. MCP, provider,
planning, integrity, approval, transactions/recovery, cleanup, compatibility and
dependency docs now distinguish primary operations from deferred complete Desktop
work. Their detailed versioned contracts remain intact. Dated milestone reports,
old ADR bodies and pinned research/fixtures are not rewritten. The milestone/ADR
indexes explicitly identify supersession and preserve historical verdicts.

The internal `AgentMender_Milestones.md` and
`AgentMender_Scoped_Cleanup_Architecture_Addendum.md` gain a governing product-first
section and a clearly labeled historical body retained verbatim. The original PDF,
other research, prior reports and evidence logs remain untouched. An external
audit receipt records original hashes and final verification. No duplicated
canonical roadmap is introduced in those files: they point to the repository.

## Quality and final delivery evidence

The starting workspace passed 453 primary tests (including five compile-fail
doc tests), plus eight separately reported nested child-process passes, zero
failures and six deliberately ignored installed/live opt-ins. The first
refactoring group passed targeted provider-api tests and architecture fitness.
Final fmt, locked Clippy/tests/build, architecture and both compatibility
validators passed locally. Final whitespace/link/status checks accompany delivery.
The final test gate retained the same primary/nested/ignored accounting. Historical v2 replay,
approval/consumption, crash/WAL and late-writer tests remain in the workspace gate.

Final exact SHA, individual CI job URLs/results, local logs and untouched historical
hash checks are retained in the sibling internal project folder:
`AgentMender_Product_First_Audit_Verification/final-receipt.json` and
`AgentMender_Product_First_Audit_Final_Report.md`. The receipt is written after
the final commit so it can bind that commit without a self-referential Git hash.
The audit's final delivered summary must report that exact SHA and CI results;
an earlier green run does not satisfy this gate. Main and the original M8.2.1
branch remain unchanged. No automatic merge or new feature follows delivery.
