# ADR-0006: A reconstructible evidence graph with scoped identity

## Status

Accepted, 2026-10-28. M2 extends the read-only foundation after M0/M1 and the
separate exact Codex compatibility refresh passed their gates.

## Context

M1 exposes provider-neutral observations, not ownership conclusions. Sessions,
projects, workspaces, artifacts, evidence and references already exist. Providers
have different authority and incomplete coverage; the graph must retain that
information rather than replace native contracts or reinterpret paths as owners.

## Decision

Use a strongly typed in-memory ArtifactGraph in domain, without a graph database,
persistence, cleanup closure or new runtime. Graph nodes retain existing entity
payloads. Structured identity consists of provider, logical state-root namespace,
entity class and the existing opaque EntityId. Namespace paths distinguish
installations; they are not physical identity or ownership proof. Physical identity
is explicitly Unavailable. No filesystem resolution, hashing, random identity or
wall-clock sampling occurs during transformation.

Provider adapters implement a pure snapshot-to-graph port in adapter-local modules.
They define provider rule interpretations and may share neutral observation wiring
in provider-api. Application discovers once, passes that snapshot to the same
adapter's builder, and exposes read-only graph/explanation use cases. CLI calls
application; provider storage details stay outside domain and application.

Nodes retain distinct same-identity payload variants. Exact duplicate facts may
coalesce; differing provenance may not. Edges coalesce by source/target/kind and
retain every distinct supporting fact. Unresolved or ambiguous references have
protected placeholder nodes rather than guessed targets. Existing upstream stale,
concurrent or first-wins duplicate diagnostics remain visible; a graph cannot
recover discarded M1 payloads or claim their absence.

Use References, SpawnedBy, MapsTo and AssociatedWith only. References points from
referrer to target without lifetime ownership. SpawnedBy points child to the
explicit spawning-parent observation. MapsTo points from observed identity to its
explicit mapping target. AssociatedWith points session to project or project to
workspace; it records grouping, never ownership. All can inform later dependency
analysis; none grants lifecycle authority. Unsupported research relationship names
are not added without actual evidence and precise semantics.

Canonical graph construction total-orders complete facts and exact-deduplicates
them. Serialization and deserialization use the same validation/canonicalization
path; malformed endpoints, foreign scopes and inconsistent payload IDs fail.
Existing serde_json is permitted for canonical full-fact serialization keys; this
avoids assigning a misleading semantic Ord to categorical evidence strength.
No new production dependency package is needed.

## Alternatives and consequences

Recreating sessions as duplicate artifact payloads would lose existing semantics.
Global path IDs would merge different providers/installations. First-wins graph
deduplication would suppress conflict. A persistent graph store would introduce
unneeded durable state. These alternatives are rejected. Identity intentionally
does not survive moving a logical provider root and does not establish hard-link
equivalence. Later physical-identity hardening remains separate work.

## Safety implications and validation

The graph is evidence-derived and read-only. It has no plan, garbage collector,
repair engine, lifecycle port or deletion operation. Generated properties cover
ordering, duplicate/conflict retention, namespace separation, missing endpoints,
cycles, validated round trips and provider evidence fidelity. Architecture fitness
checks enforce crate direction, pure builders and absence of provider storage
knowledge in domain. M3 and later milestones remain out of scope.
