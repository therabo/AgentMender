# ADR-0020: Closed Desktop Complete Removal and Qualified Persistent Actions

Status: Accepted design (2026-11-03); exact mutation implementation and isolated acceptance require independent verification.
Partially supersedes [ADR-0019](0019-structural-removal-and-profile-acceptance.md) only for the explicit supported closed-writer operating model.

Implementation note, 2026-11-06. Builds 24141 and 23880 have distinct candidate
identities. The compiled admission list is empty, and any future entry binds
one exact bundle tuple. The decision below remains the governing contract. See
the [admission checkpoint](../milestones/m8-2-1-desktop-admission-2026-11-06.md).

[ADR-0021](0021-structural-admission-and-manual-desktop-acceptance.md) supersedes
the automated graphical acceptance requirements below.

## Context

The new continuation prioritizes the first actual complete Desktop removal profile.
The user explicitly authorizes a narrow operational prerequisite: Codex and every
identified writer must remain closed throughout the short cleanup transaction.
Preventing a user intentionally reopening the application inside an arbitrarily
small comparison/publication interval is not required. Silent stale overwrites
under the supported operating model remain forbidden. Existing primary profiles
and all historical approvals keep their exact meanings.

## Decision

Implement one exact macOS Desktop/binary/schema/local-host/Legacy profile. Its
resolvable RequiresProviderClosed prerequisite is distinct from unsupported version,
unsafe identity and ambiguous ownership. Production never quits, terminates,
starts or restarts Codex for verification. Any relevant writer or incomplete
inventory blocks before target destruction. Recheck inventory before acquisition,
after all boundaries/preimages, between publications and after verification.

The user must keep every relevant writer closed for the transaction. Combine this
explicit precondition with native maintenance/coordination/thread locks, each
SQLite boundary, the permanent session-index lock, no-follow descriptor-relative
physical checks, complete preimages immediately before publication and retained
postimages. Compare plus atomic rename is not kernel CAS. Do not claim protection
against arbitrary uncooperative writes in that comparison/publication gap. An
observed prepublication change is Stale and must preserve the changed contents;
a detected change after partial effects is Unknown/ProjectionIncomplete. Never
restore stale whole-container snapshots or retry unknown actions automatically.

Provider-specific exact actions remain in the Codex adapter. No general SQL,
JSON pointer, JSONL predicate, path executor or imported replacement bytes exist.
Catalog identity binds selected root, local host, native thread, source kind and
exact profile. Catalog row removal, exact-host observation sequence, conditional
scan suppression tombstone and conditional revision bump are one SQLite commit.
Freeze all14-table retained commitments, exact schemas and migration checksums;
preserve foreign hosts and shared checkpoints. Target tombstones required to
suppress scans are non-projecting technical state, not deletion failures.

Global main and backup are distinct frozen actions. Preserve unknown/unrelated
JSON lexemes and ordering outside exact approved spans. Remove only reviewed exact
session records/memberships qualified by their actual host/principal semantics.
Ambiguous bare-ID memberships block the target. Legacy browser atom imports are a
finite explicit admission boundary: prove no relevant legacy logical records or
implement their exact reviewed semantics. Never wipe browser storage or caches.
Index cleanup removes all approved structured target records while retaining every
unrelated/malformed byte span and delimiter. Missing stores are frozen absence;
appearance or new target units after approval require replanning.

Plan schema4 extends the existing CompleteRemovalManifest with an optional typed
Desktop subsection omitted for historical schemas. Old schema3 requires None and
its original two-action vector; schema4 requires the exact new Desktop contract.
Use new v4 plan/state/manifest hash domains, retaining journal schema2, independent
terminal approval, recovery, consumed DurableIntent and progress ports. No old
v2/v3 grant upgrades. The fixed action order is catalog composite, global backup,
global main, session index, primary payload and canonical state records. Acquire
and validate every mandatory boundary before the first destructive effect. Sync
per-action journal knowledge before advancing; missing receipts never imply effects
or reclaimed bytes. Reconciliation remains read-only and cannot backfill receipts.

Schema4 complete structural success requires every approved action completed,
primary/canonical absence, every required persistent surface verified absent and
retained collateral unchanged. Product Removed additionally requires matching
compiled exact-profile two-cycle reconstruction acceptance. A serialized admission
value or external JSON receipt cannot mint this evidence. Safe mutation-contract
tests may exercise the real approved production path in disposable owned state
while product assessment remains ProjectionIncomplete pending acceptance.

Acceptance uses the exact signed application, real renderer/main services and
pinned CLI in owned HOME/CODEX_HOME/userData/workspaces. Kernel denial of real state,
network, credential brokers and AppleEvents must hold in every descendant. Any
required graphics exception and process-supervision mechanism requires separate
canary and source review before application startup. Baseline positive sentinels,
actual cleanup, two fresh reconstruction cycles and backup/checkpoint/legacy
challenges are mandatory. No fabricated Electron service or extracted catalog
implementation counts as application acceptance.

## Validation

Sol/high defined properties before implementation, seed0x4d3832315f763431 with256
cases: effects subset approval; qualified foreign-host preservation; exact catalog
sequence/tombstone/revision postimages; lossless index/global unrelated bytes;
prepublication drift refusal; required persistent residue/unknown roles/missing
acceptance prevent product success; ephemeral rendered UI does not decide success;
writer presence gives zero initial destruction; every crash leaves only its durable
action prefix; read-only recovery cannot retry or credit bytes; historical v2/v3
wire/hash/binding/replay remains unchanged; actual reconstruction preserves every
positive sentinel while target stays absent. Keep source fitness against broad
store deletion, generic mutation, cwd/substrings and production app control.

## Consequences

Closed Desktop is an explicit supported operating condition, not an invisible
lifetime-fence claim. This resolves the previous requirement for impossible
absolute relaunch prevention without weakening ownership or approving stale
snapshot overwrite. No new auxiliary family is promoted until the persistent
Desktop path works. M8.2.1 remains OPEN until implementation, exact isolated
acceptance, collateral proof and local/exact-head three-OS gates pass.

## Narrow macOS metadata ABI extension

The sole existing audited provider-api macOS C-ABI module may add descriptor-only
filesec_init/fstatx_np/filesec_query_property/filesec_free queries, using the local
Apple SDK sys/fcntl.h and sys/stat.h layouts. Positive FILESEC_ACL absence is
required; no security metadata is changed. Descriptor xattr/flags checks reject
unsupported replacement metadata. Every allocation is freed exactly once and no
opaque pointer is dereferenced. Owned positive/ACL/xattr rejection canaries and an
independent Sol/high ABI review precede production profile admission.

## Read-only macOS process census

The same audited ABI module may query libproc and XNU code-signing metadata.
Use bounded `proc_listpids` queries for the real and effective UID of the current
unprivileged user, followed by `PROC_PIDTBSDINFO`, `proc_pidpath` and the fixed
`csops(CS_OPS_IDENTITY)` query. Bind executable and signing identity to PID,
parent PID, credentials and process start time. Require agreement between two
complete observations before reporting a closed writer set. Unreadable,
unqualified, or changing observations block the operation. The process result
contains only a prerequisite status. Process arguments, environments and state
contents are outside the query contract.

An exact CLI path or a known Codex identity within the validated Desktop bundle
establishes active presence. Codex-like path components and signing identifiers
outside the qualified paths establish uncertainty. This deliberately treats
same-user Codex writers as relevant across roots. A simultaneous real Desktop
and isolated acceptance run therefore needs a separate unprivileged account
with a private, physically disjoint root, or a separate virtual machine. The
cleanup roots must belong to the census principal, who keeps the relevant
writers closed until verification completes.

The [Apple libproc interface](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/libsyscall/wrappers/libproc/libproc.h),
[XNU credential filtering](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/proc_info.c)
and [XNU signing identity implementation](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/kern_proc.c)
define the reviewed byte counts and buffer layout.
