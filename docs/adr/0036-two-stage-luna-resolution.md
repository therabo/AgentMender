# ADR-0036 — Two-stage Luna resolution

## Status

Accepted. Supersedes the interleaved assessment workflow in [ADR-0034](0034-progressive-resolution.md) and retains the UUID cohort in [ADR-0035](0035-uuid-search-cohort.md).
[ADR-0037](0037-autonomous-mcp-session-find.md) places the host and its
process lifecycle inside AgentMender's MCP installation.

## Decision

The host closes one Luna Medium Planner before starting the Rust resolver.
The validated SearchIntent fixes optional lexical alternatives before capture.
Rust traverses the frozen cohort with FTS5/BM25 and the existing typed
predicates. It reports traversal and source coverage separately. A fresh
Luna Medium Evaluator starts only after acquisition and handles all batches
and any explicit single-best selection in one thread. The host closes that
thread before requesting publication.

The confined worker uses its anonymous SQLite cohort spool for selected UUIDs,
bounded passages and their source positions, and pending publication receipts.
The provider database is read-only. Transactions on the spool are short and
the application keeps one active batch and one replayable reply. Per-operation
limits and idle expiry apply throughout the run. Source revision and identity
are checked before presenting a staged passage and again before signing a
result. Terminal completion, cancellation, idle expiry and worker exit
release the spool.

## Consequences

The earliest confirmed result follows acquisition and assessment. Eligible
sources and admitted candidates are counted separately. Partial transcript
coverage can coexist with a finished UUID traversal. An interrupted traversal
cannot start semantic assessment. The host attests the model and its lifecycle,
while Rust enforces the phase and evidence boundary.

## Validation

Exercise phase refusals, bounded batches, replay, exact caller exclusion,
source change and stale publication with disposable data. Verify actual
model and thread identity from App Server events separately from mocked
assessment tests.
