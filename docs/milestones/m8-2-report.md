# M8.2 implementation and verification report

2026-11-01. This report accompanies the final branch. Local capability evidence is
**YES** for the exact bounded profile below. Local gates and independent security
review are **PASS**. The concluding release verdict additionally requires the
exact final branch commit's Linux/macOS/Windows Actions; the concluding response
supplies that exact SHA/run rather than treating local evidence as remote CI.

## Baseline, integration and commits

M8.1 `milestone/m8-1-codex-full-cleanup` at
`6f798bcebdfd2f763e31d62ef2695c52396ca709` passed required local gates and exact
Actions [41189341451](https://github.com/therabo/AgentMender/actions/runs/41189341451)
on all three OS jobs, then was fast-forwarded/pushed into main. Main and M8.2 base
remain that SHA. New tracked branch: `milestone/m8-2-scoped-session-cleanup`.
Receipt: `c9dd04d`; research/design documentation: `03b9acb`; integrated feature:
`29237cb29771da4c4bf94f6a5954171c9caf85a1`; subsequent documentation records final
review/evidence. No squash, force push or automatic M8.2 merge into main.

**M8.1 historical provider-native complete-cleanup capability remains BLOCKED;
M8.2 introduces a different scoped-cleanup success model.** M9 Claude remains
BLOCKED BY PROVIDER CONTRACT, zero mutation profiles.

## Research and design

Nine actual implementations were inspected at pinned commits: Session Steward,
LeeeeTX/codex-history-cleaner, 1939869736luosi/codex-sessions-manager,
getoverroot/codex-clean-subagents, hanityx/threadlens,
gabrielhamalwa/codex-cliner, AyoParadis/codex-cleaner,
coramba/codex-sessions-manager and Gares95/codex-session-cleanup. All are MIT except
1939869736luosi (Apache-2.0); codex-cliner is archived. cx-sessions/seba-rb was
investigated but unavailable (API 404/clone failure); cached README is explicitly
unverified, not code/permission evidence. Additional samples/leads are separately
qualified in the [implementation matrix](../research/scoped-session-cleanup-implementation-matrix.md).
No third-party implementation was copied.

The [landscape](../research/scoped-session-cleanup-landscape.md) records strategy,
identity, SQL/files/index/history effects, descendants, backup/recovery, locks,
crash/residual behavior, verification, reclaim and rejected assumptions for each.
Useful patterns: exact IDs, immutable preview, revalidation, per-action knowledge,
guarded metadata recovery. Rejected: cwd/filename ownership, native retry after
partial effects, implicit child cascade, flexible schema authority, lossy JSONL
rewrites, tool-only locks and multi-GB backup as reclaim.

Official empirical reports include #34337/#34061 (large rollout storage), #39919/
#36558 (partial native deletion/stale rows), #41645 (historical lock behavior),
#40365 (shared fork history), #43197 (Windows failures), #53175 (Desktop catalog)
and #54730 (plugin residue). They are observations, not specifications. Exact
official source commit `6ff1e1ca3b30fc8b914720681fcf5fe62e95f856` supplies reviewed
format/coordination/Legacy history behavior; see the
[source review](../research/codex-0.166-storage-and-coordination.md).

ADR-0017 was written and its semantic foundation independently accepted before
provider cleanup implementation. It partially supersedes native-only/whole-provider
readiness rules without rewriting historical ADR reasoning or M3 health semantics.

## Implemented semantics and primary criterion

`LifecycleIntent::CleanupSession`, semantic `ScopedCleanupSession`, strict scoped
plan schema v2 and `CleanupManifest` extend the existing LifecyclePlan. Native v1
fields/digests remain on their existing serialization/hash path; absent optional
fields stay absent. Provider-specific pins/storage interpretation stay in Codex;
domain owns structural identity, approved-subset, accounting and report laws.

The manifest is CleanupSet plus RetainedSet: exact primary payload and catalog row,
frozen auxiliary row-set preimages/counts, physical identities, categorical evidence,
retained logical DB/file baselines and residual categories. The existing binding,
recovery, authenticated independent approval and consumed DurableIntent commit to
it. Fresh trusted read-port inputs rebuild the plan immediately before intent.
No imported manifest, boolean approval, MCP execution or parallel transaction engine.

`Removed` requires all required actions verified with no retained residual known.
`RemovedWithResiduals` is successful required primary removal with protected residues
or optional failures. `Blocked` is essential primary safety uncertainty before
effects; `OutcomeUnknown` protects possible incomplete effects; `Failed` cannot
claim primary removal. A global M3 Unknown is not automatic cleanup failure.

For this profile, session removed means the unique frozen Legacy replay payload
and exact canonical thread row are absent at the fenced verification cutoff, with
retained-session primary state unchanged. Every known catalog-selected rollout
must match complete inspected path/header identity inventory. Known shared primary
references block; child sessions are never silently added. Legacy full readers
need the matching rollout; title indexes/external projections alone do not restore
intact Legacy history. This is selected-root removal, not all-historical-copy erasure
or permanent future sidebar absence.

## Exact capability and effects

| Gate / surface | Supported rule |
|---|---|
| Executable | Official Codex 0.166.0 Darwin arm64; 268,972,064 bytes; SHA-256 `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`; read-only hash, never started |
| State schema | Complete `state_5.sqlite` schema SHA-256 `1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`; adapter `scoped-cleanup-v1`; no profile inheritance |
| Primary layout | Selected root-local WAL, one canonical Legacy plain rollout; exact DB/header producer; no compressed/reverted/paginated/attached/shared primary history; root-local thread-history DB absent |
| Coordination | Existing safe native maintenance/coordinator and all idle writer locks; existing no-link/single-link same-device WAL+SHM held through SQLite close; rollback journal absent |
| SQLite | Fresh connection EXCLUSIVE locking before first access; actual main-file exclusive acquisition; same connection through commit/poststate; exact receipt and thread deletes, reviewed target dynamic-tool FK cascade |
| Refused target rows | Nonzero target attachments, guardian feedback or incident spawn edges; unknown schema/trigger/profile; hidden/custom selected rollout; uncertain sharing or identity |
| Files | One exact descriptor-relative primary unlink + parent sync; no recursive directory removal, quarantine, payload backup or sidecar deletion |
| Index/history | No rewrite in this adapter; shared name index and command history retained |

Observable root/system/project config and current environment overrides are refused
as documented in [the exact profile](../../compatibility/codex/scoped-cleanup-v1/README.md).
MDM/cloud/other-process arguments and historical external stores are not searched;
this is an explicit declared-root limitation, not a fabricated effective-default proof.
Linux/Windows mutation remains unsupported; ordinary CI is provider-independent.

Unknown caches, shared memories, unsupported logs/catalogs, generated images,
unattributed snapshots and future stores remain retained. Only essential primary
identity/root/schema/physical/dependency/concurrency uncertainty blocks the target.

## Reclaim, crashes and collateral safety

The real-pinned-binary + real-protected-worker isolated acceptance passed after
the final inventory/sidecar fixes: **1 passed**, zero failures, 23.06 seconds.
It uses manually seeded disposable exact-schema state and the actual plan,
transaction, recovery, independent test grant, DurableIntent and production adapter.
Target planned/unlinked logical bytes: **67,108,864 (64 MiB sparse logical size)**.
No full payload copy was made. Physical reclaimed allocation is **unknown**.
The same-cwd/similar-UUID child row and payload plus unknown file were preserved;
normal result is `RemovedWithResiduals`. See
[machine-readable receipt](../../compatibility/codex/scoped-cleanup-v1/isolated-acceptance.json).

SQL preimages/counts and retained projection are checked under the transaction;
scratch deletion verifies effects before payload unlink. Payload unlink precedes
catalog commit. A partial crash cannot become Removed from an absent file alone.
Typed `CleanupObserved` records per-action knowledge in the existing journal;
uncertain effects remain RecoveryRequired/OutcomeUnknown. Recovery is bounded
reconciliation metadata, deliberately not restoration of the irreversible payload.
Reconciliation never deletes replacements, expands scope, recreates a receipt,
automatically resumes or compensates. The current conservative restart observer
cannot establish a common fenced cutoff, so incomplete crash attempts stay protected.

Actual child-process abrupt exits after unlink and after catalog commit prove
rollback/commit distinctions, owned journal reopening, preserved unrelated bytes/rows,
read-only reconciliation and refusal to redispatch. These tests are not a sudden
power-loss or malicious same-user-writer certification. Exact cooperative native
locks, safe descriptors, all-known-path proof, six target row sets and the 17-table
retained semantic projection protect against collateral deletion in the admitted
profile. [Every destructive site and authority chain](../research/m8-2-mutation-audit.md)
was audited; [independent security review](../research/m8-2-security-review.md) is APPROVED.

## Tests and documentation

Full local workspace: **379 primary passed, 0 failed, 4 ignored**; eight nested
child-pass summaries produce 387 raw summaries. Five compile-fail doctests are
included in the primary count. Required fmt, locked all-target Clippy `-D warnings`,
locked workspace test/build, architecture, retained evidence and whitespace checks
all passed. The ignored exact-profile acceptance was separately invoked and passed.

Scoped coverage: six domain tests include **4096 deterministic model cases**, seed
`0x4d382e3253434f50`; seven filesystem tests include **128 actual fixture cases**,
seed `0x6d382f53636f7065`; seven storage tests cover exact effects/retained rows,
count drift/rollback, schema/profile and hidden-path refusal; three worker tests;
five WAL concurrency integration tests; four application authority/reopen tests;
positive private/default and exact-worker/opt-in acceptance; injected and actual
process-termination cuts. Existing property regression files and historical refusal
guards remain intact. Model-case counts are not thousands of real provider executions.

Current documentation updated: README, AGENTS, planning, integrity, approvals,
transactions, Codex native/full cleanup, Claude lifecycle, MCP, dependencies,
compatibility indexes, ADR index/cross-references, new scoped contract, research,
security/mutation audit and progress/report. Historical integration receipts,
provider evidence, original PDF and old engineering reports are preserved; old
ADR decision bodies retain their reasoning under explicit supersession markers.
Internal `AgentMender_Milestones.md` and the separate scoped architecture addendum
were updated. M8.3 is planned after M8.2 PASS; no M8.3/NLP/M10 work started.

## Agents

| Work | Model / effort | Result / owned files |
|---|---|---|
| architecture_reconstruction_finish | Sol/high | Full core reconstruction; official lock/history and architecture review; read-only reports |
| interface_history_reconstruction_finish | Sol/high | Interfaces/history and external implementation review; research/live-doc draft |
| provider_reconstruction_finish | Sol/high | Provider/evidence reconstruction; official issues, threat/property and semantic review |
| scoped_domain_foundation / scoped_codex_adapter | Terra/high | Interrupted by usage limits; partial domain groundwork / API proposal only |
| m82_domain_finish | Terra/high | Domain plan/binding/replay/report/properties; private SQLite/profile split |
| m82_codex_finish | Terra/high | Protected worker, exact profile/adapter, full-chain and interruption tests; profile evidence |
| m82_workflow_finish | Terra/high | Application/persistence/CLI and filesystem, sidecar and known-path safety fixes |
| m82_security_review | Sol/high | Independent final code/security review, seven findings resolved; review report only |
| Main agent | Coordination | Integration, docs/internal roadmap, source fitness, complete local gates, exact acceptance, mutation audit and Git/CI verification |

## Product answer and next gate

**YES**: AgentMender demonstrates safe removal of one Codex session's admitted local
primary state and reclaim of its attributable logical storage without requiring
complete forensic erasure. `RemovedWithResiduals` is a normal successful result.
Final release **PASS** requires exact-final-commit three-OS CI; it is not inferred
from this local receipt. Only after that gate recommend **M8.3 — Project Cleanup &
Multi-Session Storage Reclaim**, without beginning it. Do not recommend M10 yet.
