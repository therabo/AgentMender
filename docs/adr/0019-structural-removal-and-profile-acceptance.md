# ADR-0019: Structural Removal Authority and Isolated Profile Acceptance

Status: Accepted structural product contract (2026-11-03); new mutation profiles require the independent gates below.
Partially supersedes [ADR-0018](0018-complete-session-removal.md). M8.2.1 remains OPEN.

[ADR-0021](0021-structural-admission-and-manual-desktop-acceptance.md) supersedes
the automated graphical acceptance requirements below.

## Context

The continuation request separates persistent removal from the currently rendered
sidebar. A stale in-memory item is not evidence that persistent state survived.
Conversely, hiding an item is not evidence of persistent removal. Restarting the
user's application merely to inspect its sidebar would interrupt unrelated work.

ADR-0018 combined runtime observations and isolated profile validation. Its
requirements for an immediately refreshed GUI and a production reload/restart
are replaced by this decision; its attribution, approval and collateral rules remain.

## Decision

Production success is established by structural persistent-state verification.
The currently open graphical sidebar may temporarily be stale and is not the
authority for cleanup success. The validated compatibility profile independently
proves through isolated restart/reload acceptance that structurally removed
sessions do not reappear. Visual user confirmation is optional.

Classify every reconstruction surface as RequiredPersistentProjection,
EphemeralUiProjection, or UnknownReconstructionSurface. Only a proven ephemeral
surface incapable of repersisting target state may be ignored at the runtime
cutoff. Unknown reconstruction roles, partial inventories and required persistent
residuals prevent product Removed and RemovedWithResiduals.

Production never starts, stops, kills, reloads or restarts Codex solely for visual
verification. A required store without a compatible lifetime writer fence blocks
before destructive actions: Close Codex to complete removal of the named session.
A process-list absence check alone is not a stable writer fence against a later
launch. Native rollout/SQLite locks do not protect the Desktop global Map writer.
No unsupported store is mutated to work around that missing coordination proof.

Profile acceptance binds the complete binary/bundle/schema/root-selection and
reconstruction contract. It uses disposable owned state and the actual production
plan/independent approval/consumed DurableIntent/adapter path, then at least two
fresh supported reconstruction cycles with collateral sentinels. A source review,
synthetic model, hidden sidebar or a primary-only acceptance does not supply this
proof. Production does not repeat this restart test on user state.

New effects use separately versioned frozen manifest/plan semantics. Historical
v2 manifests, hashes, approvals, transaction events and replay retain primary-only
meaning. Existing independent approval, durable intent, recovery and journal
remain the authority; there is no parallel permission or imported acceptance proof.
A compiled exact-profile acceptance claim must match its profile identity. Missing
acceptance keeps product ProjectionIncomplete even when primary reclaim succeeds.

Freeze exact primary, persistent projection and auxiliary actions, root/profile,
preimages, expected counts, retained commitments, writer requirements and structural
postconditions. Effects remain a subset of approval. Newly discovered units require
replanning. Journal durable per-action knowledge before advancing to later effects;
an effect/record crash gap is Unknown, never inferred reclaimed bytes or permission
to retry. Reconciliation remains read-only unless separately reviewed.

Safe state_5 attachment metadata and guardian feedback promotion can be implemented
under a distinct expanded primary profile without claiming complete Desktop
removal. Shared attachment payloads, spawn edges, memory consolidation, message
boards, trace bundles and user/project files retain their protections. Extra DBs,
JSON/JSONL containers and snapshots require their own reviewed schema, physical,
preimage, dependency and lifetime writer boundaries before admission.

## Operation ordering

Acquire every required writer/physical/schema boundary and validate every frozen
preimage before the first target destructive effect. Prepare/validate rollbackable
SQL changes while fences hold. Commit only exact approved actions in a fixed
manifest order; retain durable observations at each independent commit boundary.
Primary payload disappearance with surviving persistent projection is incomplete.
Main and backup are distinct actions; never infer one from the other. Verification
covers required persistent absence, approved maximum-safe actions and all retained
collateral. Production app-control operations are absent from this order.

## Validation

Before implementation, Sol/high defines deterministic properties: stale ephemeral
UI alone cannot change structural success; required persistent residue or missing
profile acceptance cannot pass; unknown/shared units are never deleted; every
approved safe unit is attempted or freshly invalidated; effects are a subset of
approval; fresh discoveries remain protected; unrelated and foreign-host same-UUID
records survive; shared containers/unknown JSON keys survive; cwd grants no scope;
validated reconstruction cannot restore the target. Add writer drift, schema drift,
physical drift, per-action crash and read-only recovery tests.

Fitness rejects production app restart/termination, broad store deletion, generic
mutation ports, substring/cwd attribution and successful product assessment without
persistent closure plus accepted exact-profile restart evidence. Retain old v2
regressions and all local/exact-head three-OS gates. No M8.3, M10 or NLP work.

## Consequences

A stale open sidebar can coexist with proven structural success under an admitted
profile. A closed-app message describes a mutation prerequisite, not a visual test.
The default macOS Desktop global writer currently has no validated stable external
fence; this remains an admission limitation, not permission for unsafe mutation.
