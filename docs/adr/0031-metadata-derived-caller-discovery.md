# ADR-0031: Separate caller-filtered discovery from management

Accepted November 10, 2026.

Expose ordinary read-only retrieval as `session_discover`. Interpret the
request's `threadId` as an apparent native Codex thread, and require any
supplied `sessionId` to corroborate the same UUID in this root-thread scope.
Codex may use a different session tree ID for forks. Refuse that shape until
the selected provider store can verify the relationship. Use the configured
root and require one canonical inventory row. Never infer a caller from a
query argument or pick a different identifier as a fallback.

Use the existing confined worker and exclusions before inventory evaluation.
Bind discovery cursors and context to the apparent caller and exact provider
root, in addition to their existing process, query and source constraints.
Separate discovery targets from general entity references so that a discovery
result supplies no removal or preview target. Carry purpose through context and
semantic assessment.

A client can supply false metadata. Its apparent identity only affects this
read workflow. The management gate still requires independently verified host
identity. The explicitly requested diagnostic route retains its separate
semantics. These decisions preserve the existing reader, cache and lifecycle
boundaries.

Verify the fail-closed mapping, pre-ranking exclusion, provenance separation
and per-request binding on disposable stores. Record actual Desktop catalog,
caller exclusion and natural-language use separately from protocol fixtures.
