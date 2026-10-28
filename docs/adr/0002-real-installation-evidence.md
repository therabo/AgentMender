# ADR-0002: Separate installation, compatibility and observed evidence

## Status
Accepted, 2026-10-28. Supersedes ADR-0001's global no-temporary-write restriction
only for explicitly isolated disposable provider evidence.

## Context
The complete 37-page research and milestone specification were reread before
changes. Real inspection found Claude Desktop 4.29.0 plus Desktop-owned Code
2.1.222 outside PATH. A PATH miss cannot establish provider absence. Desktop and
Code versions are separate facts; installed runtime version does not identify
the producer of every historical transcript.

The current user request permits disposable evidence snapshots while preserving
all real provider state. A consistently captured SQLite database is evidence,
not a recovery point. Codex needs writable runtime state to initialize.

## Decision
Keep the six-crate structure and detect/discover-only ports. Record each observed
installation surface (CLI, desktop, bundled CLI), location, separately sourced
version and provenance. A candidate is never presumed to be the active runtime.
Rename the bounded search miss to NotDetected; state-only remains distinct.

Compatibility describes exact schemas/layouts understood. EvidenceStatus records
Unverified, SyntheticOnly, FixtureVerified, RealPartial, RealVerified or
SnapshotVerified separately on provider and discovery reports. A strong evidence
label never grants mutation or implies complete coverage. ReadOnlyKnown remains
the maximum compatibility; no Full or lifecycle interfaces are added.

Real desktop metadata is read with exact version gates, byte/entry limits and
whitelisted fields. Transcript producer versions must be established separately;
unknown producers remain protected. Synthetic fixtures never become real evidence.
Sanitized observed fixtures retain provenance but contain no private content.

Any Codex mirror must use a consistent SQLite backup with read-only source access,
not a naive live DB copy. It is private, temporary, minimal and disposable.
Provider subprocesses may write only into that owned directory; real provider
roots remain write-denied, network remains denied and runtime/output remain
bounded. A snapshot is never a substitute for current provider health, atomic
cross-store capture, absence, ownership or recovery. The final mirror design will
be recorded separately after the experiment demonstrates native enumeration.

## Validation
Domain serialization tests distinguish installation, compatibility, evidence and
coverage. Deterministic tests have no real-installation dependency. Real smoke
tests are explicit and compare conservative before/after fingerprints without
atime; concurrent writers and partial coverage are reported. A final independent
safety review audits temporary writes, source read-only enforcement and fixtures.
No M2 algorithms or lifecycle operations are introduced.
