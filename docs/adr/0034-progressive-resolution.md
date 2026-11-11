# ADR-0034 — Progressive session resolution

## Status

Accepted for implementation on the retrieval feature branch.

## Decision

Keep the read-only resolver in the application layer and use the existing
provider worker and caller-bound evidence. Give new clients a streaming
contract on the existing MCP tool. The host retains verified result pages as
it consumes them. The server retains an active candidate batch, its source
cursor, progress counters, and bounded retry state. A busy queue must be
drained before requesting further candidates.

Separate the terminal disposition from inventory traversal, supported logical
history, and the scope of semantic assessment. In particular, a lexical pass
can finish even when session-wide absence remains unresolved.

An active run stays tied to the caller, root, and server instance. An idle
expiry or process loss requires a new run. The host can retain earlier signed
receipts as a record of what it saw, subject to their source and process
lifetime. A portable checkpoint would require a separate privacy and
authentication design. Decoder state and transcript payloads stay in bounded
memory.

Legacy actions preserve the existing response shapes. For new clients,
assessments distinguish the activity in the candidate from references to
another session. Fresh publication checks identity and cited source revisions
in the confined worker. The Luna workflow judges the meaning of the evidence.

## Verification

Exercise stable large corpora, partial and missing histories, source changes,
replayed requests, and caller isolation in disposable tests. Compare call
counts and evidence at equivalent work. Keep lifecycle and provider write
denial checks in the release gates.
