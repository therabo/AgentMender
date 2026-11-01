# ADR-0017: Scoped cleanup, residual preservation and provider-local mutation

Status: Accepted semantic/architecture direction for M8.2 (2026-10-31).
Implementation, independent code review, isolated exact-profile acceptance and
local gates are complete. Successful exact-head three-OS CI completes the release
gate; local checks alone do not establish remote release completion. Provider writes require the implemented
profile and receipt gates; acceptance of this ADR is not execution approval.

Partially supersedes ADR-0001/0003/0005/0007/0009/0013/0016 only where their
historical native-only or whole-provider readiness rules conflict with this exact
scoped-cleanup path. ADR-0008's M3 integrity meanings, ADR-0010's read-only MCP,
ADR-0011's durable knowledge and ADR-0012's independent approval remain in force.
ADR-0015's Claude refusal is unchanged. Historical evidence and decisions retain
their original reasoning.

## Context and decision drivers

M8.1 proves that current Codex native delete cannot establish complete preapproval
effects, complete postconditions and complete retained/global invariants. Its
capability verdict remains **BLOCKED BY PROVIDER** and its historical product
answer remains **NO**. That result does not prohibit a separately proven local
one-session cleanup boundary. The practical objective is safely reclaiming a
session's large primary history while preserving other sessions, not forensic
erasure of every possible trace. The [external implementation research](../research/scoped-session-cleanup-landscape.md)
shows both useful exact-key/revalidation patterns and unsafe cascade, retry,
filename-ownership, backup-size and concurrency assumptions.

The dominant destructive error is deleting or damaging another session. A missed
uncertain residual is acceptable. Precision therefore takes precedence over recall.

## Decision

Extend the existing immutable `LifecyclePlan`, transaction binding, recovery,
independent approval and move-only `DurableIntent` path. Add a semantic exact
session cleanup intention and a typed immutable cleanup manifest. Do not expose
arbitrary SQL, commands, directory deletion or file-write payloads. Do not add a
parallel transaction/recovery/approval engine.

The manifest freezes exact target/root/profile, required primary actions,
optional exact actions, per-action categorical evidence, safe physical identities,
expected row sets/counts and file sizes, retained artifacts/categories, and logical
reclaim estimates. Every destructive action must be a member of the approved
manifest. Fresh unsafe optional actions may be skipped; new actions need a new
plan and approval. The semantic law is `actual ⊆ approved`.

Existing native v1 plan wire identities remain stable. A separately versioned
scoped plan contract binds its entire manifest into `PlanDigest` and
`StateFingerprint`, and consequently `TransactionBinding`, `ApprovalGrant` and
`DurableIntent`. Validation rebuilds from fresh trusted adapter observations;
rehashing imported content grants no authority. Array order/duplicates must have
an explicit canonical contract; no ordered execution is inferred from set order.

### CleanupSet, RetainedSet and tiers

- Tier 0: exact native UUID, unambiguous primary/root/profile, safe physical
  identity and concurrency/dependency boundary. Uncertainty here blocks the target.
- Tier 1: required high-value primary payload, bound by canonical DB path **and**
  trusted root **and** regular single-link stable physical identity **and** payload
  session metadata. A filename or cwd alone never qualifies.
- Tier 2: required canonical exact thread metadata, plus precisely frozen
  target-linked metadata effects. The session-name index is a title cache, not
  primary history: exact name-record cleanup may be optional and independently
  retained when shared-file mutation cannot be proved safe.
- Tier 3: optional artifacts admitted by a dedicated categorical exact-ID/foreign-
  key/record rule. Failure retains that artifact and cannot become full Removed.
- Tier 4: Shared, Global, Referenced, Unknown, ambiguous, unsupported and future
  stores remain in RetainedSet. Derived/Orphaned remain descriptive, not authority.

Unknown is always protected. It blocks the target only when it prevents proof of
primary identity/removal, root/boundary safety, physical identity, dependencies or
writer exclusion. An unrelated unknown cache does not block an otherwise safe
primary cleanup. An external fork using the target's history **does** matter:
removing that shared primary payload would damage another session, so retain it
and block primary removal. Do not delete the referrer or descendants to resolve it.

M3 may remain Unknown outside the cleanup boundary. Cleanup-specific verification
must not manufacture Healthy or weaken M3's complete negative-proof contract.
ArtifactGraph remains the normalized explanation and attribution model; scoped
observations supplement it with exact cleanup evidence rather than using graph
reachability as ownership.

Domain owns categorical manifest identity, unique bounded row sets, required vs
optional actions, path containment/physical disjointness, subset/report laws and
authority binding. The Codex adapter owns literal table/file names, exact binary
and schema pins, rollout naming and the full six-table effects contract. Domain
must not parse or hardcode provider storage layout to decide eligibility. Fresh
trusted read-port evidence and independent dispatch revalidation enforce those
adapter-specific facts; moving them out of pure domain never makes an imported
manifest a proof. Architecture fitness retains its domain-storage boundary.

### Strategy selection

Prefer a validated native operation when its entire effects fit the approved set.
Otherwise select a reviewed exact-profile ScopedLocal strategy **before approval**.
Current M8.1 native delete is not dispatched. Never attempt native delete first and
then improvise local deletion after an ambiguous failure. Existing historical
unarchive support and its pins remain separate. M8.2 deletes exactly one requested
session; child sessions are independently retained and require separate plans.

### First supported profile boundary

The initial implementation is deliberately narrow: official Codex `0.166.0`,
Darwin arm64 executable SHA-256
`39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`,
WAL journal mode, state schema fingerprint
`1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`,
adapter `scoped-cleanup-v1`, selected root-local state DB, and legacy plain rollout
layout with a single canonical target history. No inferred inheritance from
0.165, another binary, another schema, compressed/paginated/reverted histories or
a future layout. Read-only, native-lifecycle and scoped-local compatibility are
separate facts. Other OS profiles stay Unsupported until reviewed, without starting
the broader M10 milestone.

The target identity is explicitly namespaced by the selected canonical provider
root. Its `state_5.sqlite` row is the authoritative catalog record for this
declared local operation; this does not claim that every running client's layered
configuration was reconstructed. Known current external `CODEX_SQLITE_HOME` or
observed `sqlite_home` selections prevent admission. The first implementation may
conservatively require observable configuration files absent rather than parsing
arbitrary configuration. Its exact guard locations must be documented and checked
again before effects. Unobserved MDM/cloud/process configuration remains outside
this selected-root contract and must be disclosed as a limitation.

On 2026-11-01 independent Sol/high review confirmed from exact 0.166 source that
Legacy full-history/model-context readers require an existing matching rollout;
paged projection readers reject Legacy. A hypothetical external projection DB
alone therefore does not preserve an ordinary intact Legacy session after the
selected-root primary cut. Unsearched historical external stores/copies are
explicit unknown residual categories, not a demand for forensic closure. Known
independent functional primary representations still block. The first profile
continues to require the root-local thread-history DB absent; that conservative
layout exclusion does not establish external-store absence.

Require complete accessible relevant rollout-header/reference inventory in active
and archived namespaces, no ambiguous target aliases, no dependent history-base or
attached primary consumers, and no known unsupported functional primary history.
Unrecognized nonprimary files/stores stay residuals. State DB schema comparison
includes all tables, indexes, triggers and FK behavior; unsupported schema blocks
before opening a writable connection.

Executable identity is checked by read-only hashing, without process startup.
No provider process is launched to discover, dry-run or mutate this local path.
Test-only synthetic pins must be compile-time private to tests and cannot be
selected through CLI, environment, JSON, MCP or imported plans.

### Concurrency and filesystem boundary

Use the exact 0.166 native filesystem lock namespaces, validated stable no-link
lock files. Acquire bounded nonblocking maintenance/coordination/index locks in a
fixed order, and verify **all** extant native writer locks idle while coordination
prevents new acquisitions/publication. Hold the fence, target identities, index
lock and SQLite exclusion through effects and fresh verification. SQLite
`BEGIN IMMEDIATE` alone is insufficient. For this WAL-only profile set
`locking_mode=EXCLUSIVE` on a fresh connection **before its first database access**,
then actually acquire the main-database exclusive lock and retain it through
commit and final verification. Any already attached normal WAL connection,
including an idle one, prevents that acquisition; new readers/writers are excluded
while it is held. A successful pragma string alone is not lock evidence. Rollback
journal layouts are unsupported by this first concurrency contract. Source and
experiment provenance is recorded in the research review. This exclusion does not
erase arbitrary application memory after all originating DB handles have closed;
the supported claim is a positively verified state at a fenced cutoff, not a
promise that a future provider can never recreate metadata. For the admitted
Legacy/plain history mode, a later stale metadata-only row cannot restore an intact
session without its primary rollout. Such a reappearance is new residual state,
never authority to repeat deletion. No reference-backed/paginated prepared fork
is admitted by this profile. This
may refuse busy roots; it does not require terminating every Codex process or
equate process absence with safety. Missing/unusable lock contracts block primary
cleanup rather than degrading to warnings. Planning/approval never create lock
files. Any necessary lock-file creation is itself a frozen effect after durable
intent; the first profile may instead require existing native lock infrastructure.

Use safe descriptor-based relative filesystem operations where available, refusing
symlinked/reparse components, hardlinks, unsafe roots/aliases, identity/size/mtime
drift and cross-filesystem substitutions. No recursive provider directory removal.
The workspace unsafe prohibition remains. A hostile same-user process can bypass
advisory locks and approval; do not claim protection from a malicious same-user
actor. Cooperating exact-profile provider writers and ordinary substitution/stale
state are within the tested boundary.

### SQL and index policy

SQL stays private to the Codex adapter. Only static parameterized exact UUID/PK
predicates, known target auxiliary rows and exact endpoint relation records are
eligible. No title/cwd/LIKE selection, dynamic SQL import, unchecked triggers or
unknown cascade. Freeze expected rows/counts; revalidate schema and row contents
under the transaction; unexpected affected counts roll back. Check all non-target
rows/relations remain semantically identical before commit.

Optional session-name index removal uses structural exact IDs, its native cross-process
lock, byte-preserving selection, a private same-directory temporary file,
flush/sync and atomic publication. Preserve non-target bytes, order and malformed
unrelated lines; ambiguous target records cannot silently disappear. Never rebuild
shared indexes from an incomplete list. Command `history.jsonl`, unsupported logs,
memories, Desktop catalogs and global caches may be deliberately retained: they
are not an intact resumable primary session by themselves.

### Ordering, recovery and crash semantics

Evaluate payload-first, metadata-first and quarantine-first explicitly. Choose
payload-first for this one-session irreversible boundary: freeze and revalidate all
required actions while fenced; unlink the exact payload and sync its directory;
optionally publish the approved exact name-index rewrite; commit validated exact metadata removal;
freshly verify every required primary surface and retained invariants. Database
changes are prepared/validated transactionally before payload unlink where possible,
then committed after payload/optional-index effects. An unsafe or failed optional
index action is retained and cannot enlarge the primary action set. A SQL failure before unlink rolls back
without deleting the payload. A crash after unlink may leave stale metadata and
is **incomplete/OutcomeUnknown**, never success inferred from an absent file alone.

Quarantine without final unlink does not reclaim space. It adds a second pathname
and restart authority and is not the initial default. A destructive canary would
make primary removal partial before the same proof obligations are met; use
read-only preflight and disposable acceptance canaries instead of mutating a real
unrelated metadata row as a test.

M6's single semantic-operation durable intent precedes **all** provider effects.
Record typed per-action outcomes in the existing append-only transaction journal,
not a separate authority store. Missing outcomes remain unknown. Recovery retains
bounded typed manifest/commitments and metadata needed for reconciliation, not a
multi-GB payload copy. Approval explicitly discloses irreversible payload removal;
recovery metadata is not a backup. No automatic compensation or whole-store restore.

After restart, independently inspect each frozen action and retained commitment.
Absent expected file, absent exact DB row and absent exact index records can prove
individual completion. A replacement file/row is not the approved object and must
not be deleted. Reconciliation never expands the manifest, reconstructs a receipt
or automatically resumes destructive work. Remaining required actions require a
fresh reviewed plan/approval if execution is desired.

### Outcomes and accounting

- Removed: every required primary action freshly verified complete and every
  approved optional removable action complete, with no known retained residual.
- RemovedWithResiduals: required primary removal verified; optional failures or
  protected shared/unknown/unsupported residuals remain. This is successful cleanup.
- Blocked: primary safety prerequisites unavailable before effects.
- OutcomeUnknown: effects may have occurred but required/retained verification is
  inconclusive, including interrupted multi-store operations.
- Failed: a decisive observed failure that cannot claim primary removal.

Keep these cleanup outcomes distinct from M6 transaction states. Succeeded requires
positive fresh primary and collateral verification, not adapter return success.
The primary criterion for this profile is absence of all frozen target primary
rollout payloads and the exact canonical thread record at the fenced verification
cutoff, with retained-session primary state unchanged. Known unsupported independently
functional primary history prevents this profile from Ready. Unsearched external
projection-cache content alone remains residue under the Legacy cut above.
Optional name-index records and
small command-history/catalog/cache residues are truthfully retained. Official
legacy history/model-context readers require an existing matching rollout; a name
index alone cannot recreate intact history. A target row still present at the
cutoff cannot satisfy this chosen primary criterion.

Report known eligible/planned bytes, actually unlinked approved logical file bytes,
retained known bytes and unknown residual categories. No count for failed/skipped
or already-missing actions; no double counting aliases; no SQLite free-page estimate
as reclaimed bytes. Physical allocation/free-space reclaim stays explicitly unknown
unless independently measured. Keep payload handles closed before reporting
reclaim; sparse logical length is not physical allocation.

## Alternatives and consequences

Complete native forensic erasure retains M8.1 blockers and misses the practical
goal. Unrestricted storage mutation risks collateral deletion. Backup-everything
retains large payload bytes. Broad cwd/project removal guesses ownership. A second
transaction engine duplicates authority and recovery. All are rejected.

The chosen boundary increases provider-specific maintenance and can refuse busy,
shared, compressed or future roots. It permits useful safe reclaim without claiming
global health or perfect erasure. M8.3 may later compose independently approved exact
session cleanups; it may not widen one session's approved set by cwd. Natural-
language resolution remains a future layer above frozen exact identities. Claude,
M10, M11 and M12 receive no implementation in this milestone.

## Validation and approval gate

Before provider-local implementation, independent Sol/high review must accept
the research, primary/retained split, profile concurrency/dependency proof, transaction
integration and generated properties. Terra/high owns approved implementation.
Record review findings and resolutions here or in the M8.2 review receipt.

Sol/high reviewers `architecture_reconstruction_finish` and
`provider_reconstruction_finish` explicitly approved the updated architecture,
primary/residual split, cutoff semantics and formal-property requirements on
2026-10-31. The architecture approval covers the domain/application foundation;
the exact provider implementation still requires code/concurrency review and
isolated tests. The external implementation landscape was independently corrected
and accepted by `interface_history_reconstruction_finish`. Review receipts and
source research accompany this ADR under `docs/research/`. No production mutation
implementation preceded those reviews. The WAL-exclusive mechanism must be
established in code/tests; neither an accepted ADR nor a pragma response is proof.

Required checks: categorical attribution and approved-subset generated properties;
unrelated/shared/unknown/cwd preservation; exact UUID SQL affected counts, rollback,
schema drift and non-target row equivalence; index byte/order/atomic publication;
symlink/hardlink/root/physical replacement; active/pending writer and lock races;
required vs optional failures; crash at every payload/index/DB/outcome boundary;
read-only manifest reconciliation; logical/sparse/zero/large-file accounting; and an
opt-in positive disposable test through real plans, transactions, test-independent
approval, DurableIntent and the production adapter. No destructive real-user test.

Architecture fitness must allow only this reviewed Codex module and typed semantic
handoff while retaining generic write/SQL/command, MCP, Claude, domain-I/O and
approval-bypass canaries. Full local gates and exact-final-commit Linux/macOS/Windows
CI remain release requirements. Design acceptance is not milestone PASS.
