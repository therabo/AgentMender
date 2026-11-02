# M8.2.1 continuation — 2026-11-03

**Verdict: OPEN.** This continuation implements an expanded primary cleanup
profile, not complete Desktop session removal. Required Desktop projection
mutations and exact-profile restart acceptance remain unadmitted. This report records
the implemented continuation and its evidence; missing profile gates are not passes.
The [previous report](m8-2-1-report.md) retains the detailed inventories,
provenance and historical investigation. [ADR-0019](../adr/0019-structural-removal-and-profile-acceptance.md)
defines the current structural contract.

## Baseline

The continuation started at `d299ccb` on
`milestone/m8-2-1-complete-session-removal`; remote branch matched that commit
and main matched `a284767157ace8769deee85b195b5b70efe374d8`. All seven
baseline local gates passed, including the retained validator. Baseline Actions
run `41462418925` passed all three jobs, checked in a fresh Safari session. This
is baseline evidence, not evidence for the eventual continuation commit.

## Exact complete-removal profile

No complete Desktop removal profile is admitted. The new expanded **primary**
profile is bound to the existing exact stable Codex 0.166.0 Darwin arm64 binary,
schema, WAL and native writer fences. The installed Desktop bundle embeds a
different alpha CLI; it does not inherit that stable binary's authority. See the
[profile receipt](../../compatibility/codex/scoped-cleanup-v2/README.md) and
[prior inventory](m8-2-1-report.md). Exact identity and applicable boundaries
must be revalidated at execution.

## Persistent projection inventory

Catalog rows and scan coordination, global state and its backup, recents,
project/workspace membership, archived views, search and automation records can
be persistent reconstruction or presentation sources. Index and history have
separate contracts; some Desktop records and legacy migration paths have unknown
roles. A visible or hidden current sidebar is not proof of persistent absence.
See the [projection inventory](../../compatibility/codex/complete-removal-research/projection-inventory.json)
and [research record](../../compatibility/codex/complete-removal-research/README.md).
No projection IO is enabled by this research. The independently revalidated
[nine-window persistent contract](../../compatibility/codex/complete-removal-research/persistent-contract-2026-11-03.json)
records the new writer and legacy-import evidence.

## Desktop catalog contract

Removal needs authoritative exact host plus thread identity and closure over scan
and reconciliation behavior, including observation sequence, tombstone or revision
semantics. A UUID alone, especially across hosts, is insufficient. The source
model identifies candidate rows and consumers but does not admit catalog mutation
or prove that a scan cannot restore the target.

## session_index contract

Exact structured ID records are candidates inside a shared JSONL container.
Lossless preservation of other records and bytes, frozen preimages, compatible
writer exclusion, durable replacement and postread are required. This
continuation does not enable index IO.

## history contract

Exact session ID command-history records are candidates, but shared contents and
its writer/inode semantics require separate closure. Its reconstruction role must
also be proven; it is not assumed harmless. No history rewrite is admitted.
Materialized history DBs require their own schema and consumer contract.

## global-state contract

Desktop loads a Map and writes main and backup; fallback or legacy migration may
reconstruct state. The default packaged macOS Desktop path bypasses Electron's
singleton, and no compatible external CAS or lifetime lock is established for
that writer. Legacy browser-atom import reconstruction remains unclosed; bare
local-kind custom-section keys are not proof of an actual host identity. Both
copies and all unrelated keys require independent treatment. No global-state
mutation is admitted.

## Writer/app-running policy

All required physical, schema and lifetime writer boundaries must hold before
the first destructive action. A process-absence observation cannot fence a future
launch, and the primary SQLite/native locks do not fence the Desktop global Map.
Accordingly no production Desktop projection effects are admitted. Where a future
profile requires it, instruct the user to close Codex; do not stop, kill, reload
or restart it for visual verification. A close instruction alone is not a proven
lifetime fence.

## Maximum-safe artifact promotions

The expanded profile removes exact target `state_5` attachment metadata and
guardian feedback rows under the reviewed primary boundary. Attachment payloads
that may be shared and spawn edges remain protected. Memory consolidation,
message boards, trace bundles, arbitrary project/user files and ambiguous or
unknown units remain protected. Logs, queue, goals, shell snapshots and other
DB/JSON candidates need separate schema, identity, fence, preimage, dependency and
consumer proof; source candidates are not enabled pins. Newly discovered actions
require a new plan. Every approved safe unit is attempted unless freshly
invalidated, within the admitted primary profile.

## New manifest/approval semantics

New preparation defaults to the v2 adapter and uses the version 3 outer plan
manifest for the expanded primary profile. A historical schema-2 review selects
v1 and is never automatically upgraded. Old plans, digests, approvals and replay
retain their original primary-only meaning and authorize no expanded effect.
The frozen exact scope requires independent approval and consumed DurableIntent;
existing journal version 2 and approval MAC remain the authority, with no
parallel approval or imported acceptance proof. Actual effects must be a subset
of the reviewed manifest.

## Operation ordering

Fences and frozen preimages are checked before destructive effects. The fixed
primary sequence unlinks the rollout and then commits the state DB work, recording
per-action durable knowledge at independent boundaries. An effect/record crash gap
is unknown, not permission to retry or infer reclaimed bytes. Reconciliation is
read-only. Review found two P2 replay contradictions involving Removed/Unknown and
Blocked/Failed; both fixes and their report/replay regressions passed the full gate.

## Structural verification

Actual primary results feed product assessment; without accepted persistent
projection closure and exact-profile acceptance, the result is
`ProjectionIncomplete` and CLI exit 2. `SurfaceKind` distinguishes persistent,
ephemeral and unknown reconstruction surfaces. A proven ephemeral live UI can
remain stale without changing the structural predicate; an unknown surface or
persistent reconstruction risk cannot be treated as harmless. Product success
requires verified primary and required persistent absence, approved safe actions,
collateral preservation and accepted independent profile evidence.

## Isolated restart acceptance

The pure assessment requires `ExactProfileAcceptance` with two
reconstruction cycles, bound to the exact profile. No imported proof is accepted.
No actual isolated Desktop restart has passed for this continuation; no Desktop
startup was performed on November 3. The November 2 main-only probe ended in
SIGTRAP and remains historical evidence, not acceptance. Both opt-in exact-binary/sandboxed-worker tests passed (28.51 s historical,
20.47 s expanded). Neither starts native Codex or Desktop; these passes do not
prove Desktop projection closure.

## Reappearance challenge

A future accepted Desktop profile must execute the actual production discovery,
plan, independent approval, DurableIntent, adapter and structural verification in
disposable owned state, then survive two fresh supported reconstruction cycles.
It must show the target absent and the positive collateral sentinels present.
This challenge has not passed and is not simulated into acceptance.

## Collateral preservation

The primary contract preserves unrelated records and shared payload/spawn-edge
state; target metadata is exact-scoped. Full Desktop collateral preservation,
including similar UUID, same cwd/project, foreign host, pinned, recent and archived
sentinels across all persistent projections and reconstruction cycles, remains
unproven. No cwd, substring or bare UUID grants ownership.

## Reclaim

Only actually observed unlinked approved logical file bytes may be reported as
reclaimed. Unknown outcomes do not count. Physical allocation, snapshots, clones,
open handles and SQLite freelists are distinct and cannot be inferred from logical
bytes; this continuation does not claim secure erasure.

## Tests

All eight final local gates passed: fmt, clippy with warnings denied, locked
workspace tests, locked workspace build, architecture, retained exact research
(including installed-bundle/source-window rehash), lifecycle evidence and diff.
The suite has 402 top-level passed tests, zero failures, five opt-in ignores;
eight additional nested child pass summaries are counted separately. Both exact
primary-worker opt-ins passed in addition to that provider-independent suite.

Expanded hard-crash tests assert the committed action prefix after unlink/commit,
foreign metadata and shared-asset preservation, journal reopen, read-only
reconciliation and rejection of another execution attempt. A progress-record
failure stops before the SQLite commit. Guardian preimage drift refuses mutation.

Independent frozen fixtures generated by executing the original d299ccb tests
prove fresh historical v2 plan/binding/report equality, exact serialized journal
bytes and unchanged successful replay. The deterministic structural model uses
seed `0x4d3832315f763300`; live ephemeral UI alone cannot alter the result, while
missing/mismatched profile acceptance or required persistent residue blocks success.
Actual Desktop writer/race acceptance and two reconstruction cycles remain open.

## Documentation

ADR-0019 records structural authority and isolated acceptance. The existing
[full report](m8-2-1-report.md), inventories and
[expanded-primary profile](../../compatibility/codex/scoped-cleanup-v2/README.md)
provide detailed boundaries and evidence. Roadmap addenda preserve historical
milestones while recording this continuation. Production structural verification,
not live sidebar repaint or user visual judgment, is the intended success authority.

## Git/CI

All final local gates and both opt-in workers passed before committing this
report. Exact-final-commit Linux/macOS/Windows CI is verified after ordinary push
and recorded in the external delivery report/verification receipt. No main
integration is requested or performed; the milestone stays OPEN on its branch.

## Independent destructive safety review

Sol/high found no remaining P1 or unresolved P2 in the bounded implementation
review. Exact target data cannot authorize deletion of another session's row/file: **NO**, within the admitted primary contract. Production automatically
restarting Codex for sidebar verification: **NO**. Structural verification has no
visual judgment input; the current complete-profile product result remains
unverified because required persistent closure and restart acceptance are missing.

## Product questions

1. **NO** — all required persistent reconstruction/projection state cannot yet be
   structurally removed by a supported complete profile.
2. **NO** — current complete-cleanup success cannot yet be established; the
   intended structural predicate does not depend on visual checking.
3. **NO** — no accepted exact Desktop restart/reappearance profile exists.
4. **YES** — production does not automatically restart the user's running Codex
   merely for visual verification.
5. **YES, within the approved primary profile** — approved safe exact actions
   are attempted unless freshly invalidated; unadmitted candidates are retained.
6. **YES, for proven primary state and metadata** — full persistent Desktop
   projection preservation across the specified sentinels remains unproven.
7. **YES, conditional on the pure model** — only proven non-projecting technical
   residue may coexist with successful `RemovedWithResiduals`, after every other
   acceptance condition holds. No current complete profile establishes this result.

## Verdict

**OPEN**. Continue by closing exact Desktop catalog, global main/backup and legacy
reconstruction semantics, a compatible lifetime writer fence, identity and
consumer contracts for each admitted surface, then implement separately approved
frozen actions and per-action crash/race validation. Run the actual isolated
production path and two reconstruction cycles with collateral sentinels, the
complete local gates, opt-in exact workers and exact-final-commit three-OS CI.
The safe expanded primary implementation is delivered; remaining Desktop
implementation and acceptance gates are not complete. The present limitation does
not establish provider impossibility. M8.3, M10 and
the natural-language layer remain unstarted.
