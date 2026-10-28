# ADR-0005: Bind Codex evidence capture to an exact version profile

## Status

Accepted, 2026-10-28. Extends ADR-0003 without changing its isolation boundary.

## Context

The user intentionally upgraded Codex from 0.164.0-alpha.11.1 to
0.165.0-alpha.1.2. The new executable's generated initialization and list-parameter
schemas match the historical files. Its list-response schema adds a nested
`modelNotFound` error variant outside the normalized metadata projection.
Independent kernel-attested read-only capture found a new source schema: a nullable
`attached_thread_parent_id` column and its index. Reusing the old exact profile
would therefore be incorrect.

An isolated experiment with the new exact schema returned 166 native summaries
across eleven pages. Source DB/WAL/SHM fingerprints matched across capture and
enumeration; redaction and isolation checks passed. These are bounded observations,
not claims of complete history, health, ownership, or real-client liveness.

## Decision

Keep immutable, separate exact-version profiles. Each binds the executable
version, generated interface identifier, and expected source-schema fingerprint.
The internal capture request selects only a compiled known profile; it cannot
supply arbitrary SQL, schema hashes, or a general command. Verify the selected
profile's source schema before scrubbing and validate the returned manifest against
that same profile. A schema accepted for one profile does not validate the other.

Retain all historical schemas and provenance. New protocol/schema evidence and
sanitized observations have a separate version directory. Version proximity never
selects a profile. Unknown versions or layouts fail closed to protected audit
observations. Compatibility remains at most ReadOnlyKnown and never grants mutation.

The new column is opaque identifier metadata, like existing thread identifiers
and reference records. Retaining it in the disposable copy does not establish a
parent, ownership, or lifecycle relationship. Only existing normalized App Server
fields are interpreted. No new relationship inference is introduced here.

SQLite Backup API, kernel denial, bounds, RAM scrubbing, private owned evidence
homes, fixed native requests, and child cleanup retain ADR-0003/0004 semantics.
The source fingerprint applies before scrubbing; SQLite VACUUM may normalize the
disposable schema and must not be mistaken for a change in the source contract.

## Alternatives

Replacing the historical profile loses reproducibility. Accepting either known
schema for every known version breaks the version/schema binding. Passing arbitrary
hashes into the worker turns a reviewed gate into caller-controlled policy. All
three alternatives are rejected.

## Consequences and safety implications

Each future upgrade requires independent protocol, schema, redaction, isolation,
and enumeration evidence. Some legitimate layouts remain unsupported. Exact-profile
regressions must cover both versions, mismatched version/schema pairs, unknown
versions, source preservation, and sanitized fixtures. No provider-state write,
lifecycle method, graph, recovery, or persistence service is added.

## Validation

Use exact-schema tests and cross-profile rejection tests, bounded capture/privacy
regressions, old and new fixture discovery, production-path live verification,
architecture checks, and the Linux/macOS/Windows CI matrix. Keep collection coverage
and evidence status separate from compatibility.
