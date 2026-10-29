# ADR-0011: Durable transaction knowledge before provider execution

## Status

Accepted, 2026-10-29. M6 follows verified M5 integration at
`c426a7fe4e12c24afc2f383087912e7f54111b39`. Extends ADR-0009/0010.

## Context and scope discrepancies

The main agent read all 37 research pages, the complete milestone specification,
all existing repository documentation and ADRs before changing repository files.
Three Sol/high reviews covered domain/planning, provider isolation/durability and
application/CLI/MCP. M5 passed local checks and exact-commit three-platform CI.

The milestone's illustrative state list includes approval and pre-planning states.
The explicit task and M7 boundary prohibit implementing approval in M6. Existing
immutable M4 plans already represent planning. M6 starts with a bound transaction;
it has no Approved state, grant, executor or provider lifecycle port. The research
recommends encryption early; the milestone deliberately defers key management to
M11. M6 stores only typed reconciliation metadata, with private OS permissions.

## Decision

Add one persistence adapter crate, depending internally only on domain. Domain
owns pure validated transaction events, immutable binding and ordered replay.
Application owns a read-only transaction inspection port/service. CLI composes
the store only for explicit transaction inspection. MCP's eight tools and query
dispatch remain unchanged. No recovery, security or executor crate is needed.

A random 256-bit TransactionId identifies an attempt and differs from PlanDigest.
The persistence adapter obtains randomness from existing getrandom. Domain does
not read the clock or randomness. Binding is constructed from a valid Ready plan
and retains PlanDigest, StateFingerprint, typed provider/intent/operation and
domain-separated commitments to target, native identity, provider profile,
capability/effect contract, pre/postconditions and operation. M4 v1 permits exactly
one operation and treats arrays as sets. M6 explicitly supports one operation;
journal arrays are ordered and never use M4's set canonicalizer. Persist no rich
plan, native IDs, locators, paths, provider prose or content bodies. Commitments
cannot reconstruct the original plan: any future execution must supply that same
immutable plan and freshly replan against trusted inputs. A new plan needs a new
transaction. Hashes provide integrity, not authenticity or authorization.

The pure state machine distinguishes Created, recovery preparation, Prepared,
OutcomeUnknown, EffectObserved, Verifying, Succeeded, Failed, compensation and
RecoveryRequired. Intent immediately means outcome unknown, including when no
dispatch has yet occurred. No missing outcome produces Failed or Succeeded.
Only an explicit observed no-effect result can establish Failed. Success requires
an observed applied effect and passed independent verification. Unknown/failed
verification protects recovery. Terminal states reject advancement. Compensation
requires a separately evidenced contract and a known applied forward effect;
no inverse is inferred from archive/delete. M6 records NotCompensatable unless an
explicit semantic contract is supplied. Compensation has its own durable intent,
unknown-outcome window and verification; it is not SQLite rollback.

Every record has a version, transaction ID, per-transaction sequence, immutable
binding commitment, previous record digest and record digest. Replay checks all
fields, order, chain and transitions. No materialized status table is authoritative.
Unknown event/schema versions, gaps, duplicates, conflicting bindings, malformed
payloads and invalid transitions fail closed. Timestamps are omitted: sequence
is ordering authority.

## Persistence and failure semantics

Use existing bundled rusqlite. SQLite schema v1 holds append-only event records;
triggers reject UPDATE/DELETE. An external append-only, hash-chained journal keeps
the same records independently of database health, as required by the research.
A store-wide OS file lock enforces one writer across processes (safe std File
locking, available within MSRV 1.89). Expected sequence provides compare-and-set
protection against stale callers. SQLite BEGIN IMMEDIATE additionally bounds
writers. Exact duplicate advancement is rejected, never applied twice.

Append validates and serializes first, writes and syncs the external record, then
inserts and commits the SQLite record. A durable intent receipt is constructed
only after both boundaries succeed. Any I/O/commit ambiguity poisons that instance.
Reopen requires exact agreement, valid chains and replay. A durable external-only
record, database-only record or torn tail disables advancement; evidence is never
silently truncated, repaired or treated as a completed effect. Read-only external
journal inspection can report a verified prefix even if SQLite is unavailable,
but never upgrades divergent state to success. There is no claim of ACID across
SQLite, journal and provider state. Conservative recovery can require an operator.

SQLite uses rollback DELETE mode, synchronous=EXTRA, foreign_keys=ON,
trusted_schema=OFF, temp_store=MEMORY and a bounded busy timeout, with settings
read back. macOS enables fullfsync. EXTRA includes directory synchronization after
rollback-journal removal; low throughput does not justify WAL/checkpoint machinery.
Initialize schema and version atomically. Unknown versions and existing unversioned
nonempty schemas fail closed. Check SQLite integrity and foreign keys on open.
Do not auto-migrate unknown schemas or expose raw SQL/Connection handles.

## Recovery-before-intent and observation

Recovery staging stores an immutable, typed, content-addressed manifest binding
transaction, operation, provider, target commitment, plan/state and recovery strategy.
It contains reconciliation commitments, not a provider backup or a promise of
restoration. No arbitrary byte/blob API is provided. Metadata is written privately,
flushed, published without replacing another artifact and verified before the
RecoveryPrepared event. Artifacts are retained conservatively; no cleanup policy.
Publication writes a random private staging file, syncs its bytes, then atomically
hard-links it to the digest name without replacement. The held file and both names
must have the same identity and exactly two links before removing only the staging
name and syncing the directory. Interrupted staging and failed publications remain
as evidence. A crash while both links remain blocks verification rather than
silently repairing link counts. Reusing an existing final artifact syncs it and its
directory again before verification; concurrent visible bytes alone are not durable.

The sole durable intent method requires the original plan and fresh graph,
integrity report and planning context. It compares the whole binding and calls
M4 validate_against inside that method. It verifies recovery bytes again and
records validation plus intent as one logical durable append before returning a
non-cloneable receipt. A prior Prepared record alone can never admit an effect.
No M6 code can dispatch any provider effect. Test-only synthetic effects consume
the receipt to demonstrate ordering. Future M7 authorization and M8/M9 provider
contracts must be added independently; a receipt is not an authorization token.

Restart recovery reconstructs durable knowledge and recovery availability. Pending
forward or compensation intent always remains Unknown/RecoveryRequired. It does
not retry, compensate or infer external state. Fresh provider reconciliation is
future adapter work. Missing/corrupt required recovery blocks advancement even
when journal replay is otherwise valid.

## State ownership, privacy and limitations

Persistent writes require an explicit absolute AgentMender root and protected
provider roots. CLI inspection requires an explicit root, so normal queries create
no persistent state. Reject traversal, root overlap in either direction, linked
state roots/files, Windows reparse points and unsafe file identities. Resolve
configured paths before overlap checks, include standard and configured provider
homes/Desktop stores, and revalidate before writes. Only fixed private child names
or validated content digests may address durable files. Unix directories/files use
0700/0600; existing broad modes fail rather than chmod arbitrary data. Windows needs
a protected owner-only DACL, not std readonly flags. Do not use provider filesystem
observation helpers as persistent write capabilities.
Physical ancestor identity checks cover case and Unicode aliases, including two
missing endpoints under differently spelled existing ancestors. Missing non-ASCII
tails on macOS/Windows fail closed rather than guessing filesystem normalization.
Dangling protected links fail before any creation. Existing SQLite rollback
sidecars receive the same private regular-file checks; unexpected WAL/SHM is
rejected before SQLite opens the database. Unknown schema/mode is inspected before
writable PRAGMAs. Read-only inspection uses SQLite read-only plus query_only.
On Windows, token default-owner semantics can select Administrators even when
TokenUser is the caller. Assign the current-user owner explicitly on the new empty
root and newly CREATE_NEW-created owned files/stages before content writes. Keep
existing-owner/DACL validation strict and never normalize existing evidence.
Unexpected ownership of a retained SQLite-created rollback journal fails closed.

OS synchronization is a durability contract subject to filesystem/device behavior.
Network filesystems are unsupported. Full adversarial same-user path replacement,
all Windows directory power-loss guarantees and cross-filesystem content staging
belong to M10; do not claim stronger protection than tested. M11 must add an explicit
key-management model before encryption. SHA-256 chains detect accidental corruption
and inconsistent histories, not a same-user attacker rewriting all copies.

## Alternatives and consequences

SQLite alone loses independent recovery evidence. An authoritative filesystem log
with an automatically rebuilt SQLite index adds migration/rebuild complexity.
Automatic dual-store repair risks inventing committed knowledge after a crash.
Global cross-provider ACID, raw plan persistence, generic effect closures/commands,
provider backups, inferred rollback and premature approval are rejected.
The chosen design spends extra flushes and may stop safely on a dual-store gap.

## Validation

Keep every M0-M5 gate. Add generated state-machine/replay properties with fixed
seeds, malformed/reordered/duplicate/corrupt history, plan immutability, recovery
ordering, missing outcomes and terminal-state protection. Add schema, permissions,
root overlap/link/hardlink, stale-plan, concurrent writer/CAS, recovery corruption,
commit/write/flush fault tests and child-process interruption/reopen at durable
boundaries. Provider tests remain fixture-only/read-only. Extend architectural
fitness for the eighth crate, pure domain/replay, owned persistence writes,
unchanged MCP surface, no arbitrary payloads, no provider dependencies or execution.

Primary durability references: https://www.sqlite.org/pragma.html#pragma_synchronous,
https://www.sqlite.org/atomiccommit.html, https://www.sqlite.org/wal.html,
https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock.
