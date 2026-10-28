# ADR-0003: Enumerate a scrubbed disposable Codex database snapshot

## Status
Accepted, 2026-10-28, after a read-only real-installation experiment.

## Context and observed result
The exact installed Codex 0.164.0-alpha.11.1 cannot initialize under global write
denial. A consistent SQLite Backup API snapshot, scrubbed in memory before disk
persistence, lets its official App Server list 164 captured summaries in this
observation. Source DB/WAL/SHM content hashes and inode/size/mtime/ctime/mode were
unchanged during capture and enumeration. Independent activity changed row counts
between experiments. Database rows, native summaries and rollout-file counts are
separate facts. Denying all real-home metadata reads silently yields zero native
rows; denying content reads while permitting metadata reads produces inventory.

## Decision
Keep snapshot/schema/redaction policy in provider-codex. Use a fixed internal
capture worker of the current AgentMender executable, launched under the shared
process sandbox. Source SQLite is opened read-only and query-only; Backup API
copies into RAM, exact schema is checked, unnecessary content-bearing tables are
cleared and sensitive text is redacted while preserving null/empty/nonempty
predicates. Secure-delete/VACUUM precede first disk persistence. No credentials,
config, rollouts, history database or log database are copied.

The only writable location is a newly created private, opaque evidence sandbox
owned by AgentMender. Its constructor rejects overlap with protected source roots.
The capture worker and App Server have fixed operations, bounded time and output,
network denial, and no permission relaxation. App Server cannot read real-home
file contents but can stat metadata required by this build. HOME/CODEX_HOME,
working directory and temporary runtime paths point inside the disposable home.
Worker requests are internal query infrastructure, never arbitrary SQL or path
write tools. Ports remain detect/discover only. Test-only fixtures do not require
the internal helper or a real provider.

Child processes are reaped before the owned directory is dropped. The directory
is an evidence mechanism, never a recovery store. Sources can change concurrently;
consistent SQLite capture does not imply cross-store atomicity or current liveness.
Source fingerprint changes produce explicit stale/concurrent diagnostics. Unknown
schema/version, corruption, missing WAL requirements, capture failures or sandbox
unavailability return protected audit metadata rather than an unsafe fallback.

## Dependency decision
Use maintained rusqlite 0.41 with only safe backup/limits and bundled native SQLite
features, avoiding optional extension loading, virtual tables and wasm backends.
It provides the actual Backup API and a controlled SQLite version across builds.
Raise Rust MSRV from 1.85 to 1.89 as required by this stable release. Use SHA-256
(sha2 stable 0.11 line) for bounded source fingerprints/schema evidence, and promote
existing tempfile to runtime only for owned disposable directories. No new crate,
Python/Node production bridge or provider-state persistence is introduced.

## Validation
Deterministic tests cover coherent WAL backup, corrupt/unknown schemas, private
isolation, content scrubbing, stale source classification, partial/failed native
queries, limits and child/directory cleanup. Real tests are explicit and separately
fingerprint relevant source files. Final independent safety review inspects every
new write and verifies it targets only capability-owned disposable state.
