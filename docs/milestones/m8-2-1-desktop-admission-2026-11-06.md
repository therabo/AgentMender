# M8.2.1 Desktop admission checkpoint, 2026-11-06

**OPEN.** Build 24141 now has a distinct read-only schema-4 candidate. The
production admission list is empty. The [earlier schema-4 checkpoint](m8-2-1-desktop-candidate-2026-11-06.md)
describes the original adapter and its historical verification.

## Baseline and exact compatibility

The starting branch `milestone/m8-2-1-complete-session-removal` was clean at
`1c8e19acccc89ad78fb3d53e05ed2b0cd8be7c03` locally and on origin.
Local and remote main were `a284767157ace8769deee85b195b5b70efe374d8`.
The authenticated [Read-only foundation run 42208604297](https://github.com/therabo/AgentMender/actions/runs/42208604297)
verified that exact starting SHA and branch. Linux job 127861376418, macOS
job 127861376751 and Windows job 127861376651 succeeded.

The installed OpenAI-signed arm64 application is `com.openai.codex` build
24141, version 26.1027.61615. The separately recovered signed 23880 bundle
(version 26.1027.51404) retains its original candidate identity. Both ASARs
exceed the old 512 MiB ceiling. The candidate now streams each member at its
exact pinned length, checks physical identity and rejects a mixture of the
static pin tuples. The 24141 bundle, embedded CLI, stable worker, matching
source-derived catalog and remaining runtime prerequisites have a
[separate compatibility record](../../compatibility/codex/desktop-complete-removal-24141-v1/README.md).
Catalog source equivalence supports the typed candidate. Renderer assets differ
and the embedded alpha with the stable primary worker has no graphical
acceptance. Independent review identified the need for a coherent full signed
bundle and relevant helper/native-dependency identity at runtime.

## Persistent reconstruction and exact actions

The table distinguishes inspected source and owned component tests from missing
end-to-end evidence. None of these surfaces has passed a 24141 graphical cycle.

| Surface | Candidate or source evidence | Remaining condition |
| --- | --- | --- |
| Canonical active, archived, all and rollout | Stable Legacy 0.166.0 primary adapter binds state root, physical payload and exact thread. Owned tests cover cutoff and retained rows. | Revalidate stable/embedded composite, actual roots, and fresh provider views. |
| Local Desktop catalog and scan | Exact root/schema/ledger, host `local`, thread and source kind `cli`. One catalog transaction changes the exact row, host sequence, conditional scan suppression and visible revision. Shared checkpoint and foreign rows are retained. Fresh local scan still consumes primary results. | Observe fresh startup and checkpoint/scan reconstruction. Tombstones cover the captured scan and can be cleared by a fresh one. |
| Global state and fallback backup | Source-backed reducers bind exact qualified pin/tab, custom-section, project/migration and supported per-thread metadata. Unknown keys and unrelated memberships remain. With an existing invalid or unreadable main, startup can load the backup. An absent main yields an empty Map in the reviewed code. | Demonstrate valid-main, fallback and crash cases on exact runtime. A foreign reference in either image blocks ambiguous bare membership edits across both. |
| Session title index and command history | Index matches exact canonical IDs under the root and permanent index lock. It retains peer and malformed bytes and now refuses alternate spellings of the same provider UUID. History is preserved. | Prove embedded index routing and ownership for a foreign-host UUID collision. History consumer and target-only eligibility remain open. |
| Recent, pins, sections, project/workspace and title/content search | Recents and search have separate App Server paths. Pin and project views mix catalog, provider and legacy assignments. Component reducers exercise narrow supported memberships. | Positively observe each applicable view on the exact renderer before cleanup and through both restarts. |
| Legacy atoms and browser storage | The 24141 renderer can republish legacy localStorage keys on startup. The bounded LevelDB observer checks logical absence for one reviewed origin and refuses live legacy atoms. Owned WAL/SST fixtures cover the parser. | Real Electron import and other profiles/IndexedDB remain unresolved. The current renderer reads `codex-browser-host` persisted atom state, including account-specific preferences. Browser data is preserved. |
| Automation inbox/history and other independent records | An eligible automation run can surface independently of canonical state. The catalog candidate refuses related target records. Activity reminders, timeline and visualizations have separate principal/host/thread consumers. | Establish exact applicability and structural absence. ChatGPT/cloud and foreign authorities require separate proof. No cascade or browser-store reset is used. |

The catalog's qualified component test preserves a same-UUID foreign row.
Integrated observation still refuses this collision before constructing the
Desktop scope because bare global/index ownership is unresolved. This protects
the foreign row and prevents a positive end-to-end collision case today.
See the [projection inventory](../research/codex-desktop-projections.md) and
[source role closure](../research/codex-desktop-role-closure.md).
The stable CLI resolves index IDs using the provider's UUID parser.
[Index source](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/session_index.rs),
[thread ID source](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/protocol/src/thread_id.rs).

## Writer and durable execution boundaries

Under [ADR-0020](../adr/0020-closed-desktop-complete-removal.md), production requires
the user to keep all relevant writers closed. The same-UID census, native
rollout and coordination locks, SQLite exclusion, permanent index lock, fresh
preimages and retained-state verification remain in place. Active or uncertain
writers refuse the candidate. Production never closes or restarts Codex.

The global writer persists a cached whole Map without a compatible shared lock.
After the last destination check and before rename, a noncooperating writer
can publish an unrelated update which the rename overwrites. Checking the
resulting replacement cannot recover or identify that lost update. This gap and
the relaunch window remain open outside a cooperative remain-closed boundary.
Observed preimage and staging conflicts refuse publication.

Schema 4 still freezes the full ordered effect set, independent human approval
and consumed durable intent. Exact-build admission is now an empty allowlist
over complete pinned tuples. Schema 2/3 grants, reports and journals retain
their existing authority. The immutable approval plan binds the exact build.
The existing schema-4 approval summary label is generic and should expose the
exact profile in a future admitted review.

Order remains catalog composite → backup → main → index → rollout → canonical
state DB. A shared replacement verifies and synchronizes its own result before
its action receipt. A crash after backup can leave the old valid main effective.
A crash after an effect and before its receipt is ambiguous. A writer appearing
or a grant expiring after fence acquisition is now Unknown because lock/sidecar
setup can have persistent effects. Desktop reconciliation reobserves only
primary state and preserves the recorded prefix. It grants no retry or
success authority. Historical primary child tests cover unlink and DB commit.
The schema-4 catalog, backup, main, index and final-cutoff child crash matrix has
yet to run through an admitted path. A process interruption can also leave a
staged file in the owned root.

## Graphical test and release evidence

An owned CLI smoke first reached primary evidence capture and refused the
fixture. The existing read sandbox denied ordinary directory opens needed by
the closed SQLite paths. The primary and catalog workers now use Darwin
`O_SEARCH` for descriptor-relative lookups. The sandbox policy still denies
directory enumeration, unrelated reads and writes. An independent owned canary
confirmed the distinction. With the fix, the production read-only `plan` command
produced the 24141 schema-4 manifest with one target unit in the main, backup
and index. It returned `RequiresProviderClosed` with zero executable operations.
All ten fixture files kept their SHA-256 digests. This exercised the protected
workers and candidate planning only.

The available macOS arm64 host has only the current interactive user. A second
local test account requires administrator authentication. There is no usable
disposable VM or separate GUI session available. A prior same-UID helper
canary established limited file/network denial and the signed main-only probe
exited SIGTRAP. That launcher still lacks independently validated graphics
IPC, descendant confinement and a structured observation transport. This
checkpoint read only bundle metadata/source and used owned fixtures. It did
not launch or change the installed application or open its live state.

No genuine isolated seed run, interactive schema-4 destructive execution or
first or second reconstruction cycle took place. Target T and the unrelated,
same-cwd, similar-ID, same-project, foreign-host, pinned, recent, archived and
unknown-artifact sentinels have no complete positive graphical result.
Collateral component tests and the independent Sol/high reviews of the
exact-build and projection diffs support continued work. The reviews confirmed
the closed gate and identified bundle coherence, foreign/bare identity,
noncooperating writer races and the crash matrix as admission gaps.

The maximum-safe objective has no new promoted family or measured graphical
reclaim. Expanded primary tests cover approved attachment and guardian
metadata with shared data retained. History, shell snapshots, logs, queue and
goal records, alternate histories and other Desktop metadata remain protected
pending exact ownership, consumers and coordination. There are no planned,
verified-deleted or retained logical byte totals for a graphical transaction.
Unknown residual allocation was not measured. SQLite page reuse establishes
no filesystem allocation reclaimed.

Local format, strict Clippy, workspace tests, build, architecture and both
compatibility checks passed. The workspace run passed 449 top-level tests
and skipped six explicit opt-ins. The read-only exact-bundle opt-in passed
separately against both the installed and recovered signed bundles.
Both exact stable-worker opt-ins passed on owned primary fixtures.
Existing deterministic fixtures include
the 4,096-case scoped model (seed `0x4d382e3253434f50`) and the catalog/index
preservation models (seed `0x4d3832315f763431`). The workspace journal and
approval property tests use seeds `0x4d365f74786e7331` and
`0x4d375f6170707276` respectively.

Local and final exact-head remote receipts are recorded separately in the
internal `AgentMender_M8_2_1_Verification/desktop-complete-profile/final-admission-2026-11-06`
folder so they can bind a commit without changing it. The branch is separate
from main. Live architecture, approval, recovery, integrity, compatibility and
internal planning references point to this checkpoint. Claude complete removal,
cloud/remote/account-scoped sessions, other Desktop builds and platforms have
no authority from this candidate. M8.3 follows only after M8.2.1 PASS.

## Product answers

1. **No.** The Desktop candidate cannot remove all required persistent surfaces
   under a production-admitted profile today.
2. **No for complete Desktop removal.** The contract requires structural
   verification independent of sidebar paint. Current evidence covers primary
   removal and selected component postconditions.
3. **No.** Neither exact-build isolated graphical cycle ran.
4. **Component evidence only.** Exact targeting and refusal preserve peers in
   fixtures. The complete graphical collateral sentinel set remains unverified.
5. **Yes within the current gate and census boundary.** Production rejects
   Desktop mutation. The candidate detects active or uncertain writers and
   neither quits nor restarts Codex. This is no lifetime relaunch fence.
6. **No for complete Desktop removal.** Primary tests cover frozen approved
   artifacts and protect ambiguous/shared data. Desktop maximum-safe coverage
   has yet to be established.
7. **Neither build is admitted.** The installed 24141 and recovered 23880 each
   have an exact candidate.
8. **Yes as an assessment rule.** `RemovedWithResiduals` requires proven
   non-projecting technical residues. Neither candidate can produce that result.

**OPEN**
