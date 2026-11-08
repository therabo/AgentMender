# ADR-0025: Bounded conversation search and verified management exclusion

## Status

Accepted requirements and next-step design, 2026-11-08. Extends ADR-0022–0024.
This cleanup implements registered-identity filtering and refusal fixes only;
transcript search and automatic caller binding are not implemented or admitted.

## Decision

Use [one search contract](../session-search.md) for metadata and future bounded
conversation evidence. Keep provider interpretation in adapters, filtering and
selection in application, pure evidence types in domain, and bounded projection
in MCP. Reuse the eight crates, existing read isolation and exact identity. Add no
index, embedding service, new database, generic executor or destructive authority.

The verified management provider/root/session identity must be excluded before
matching, ranking, pagination, aggregate counts and presentation, and protected
again at selection, preview, planning and execution. A management workflow without
verified identity must return `caller_identity_required`, not guess a caller.
That result is a required future contract, not an existing MCP error code.

Current trusted launch registrations are useful exclusions, not authenticated
automatic caller binding. Apply those exclusions to both current conversational
search modes and refuse their protected previews. The separately admitted removal
service also refuses search without registrations. Existing read-only metadata
browsing without registration remains available with no self-exclusion guarantee;
it must not be advertised as an admitted management-candidate workflow.

Retain UserProtected plan roots, original-plan restoration, fresh checks,
independent approval, consumed grants, durable intent and recovery semantics.
No identity derived from title, cwd, activity, timestamp, newest UUID, arbitrary
model arguments or a fixture peer can satisfy the requirement. Do not assume the
historically observed `Cerca sessioni MISSION1` record is the caller.

Prefer a demonstrated host-authenticated per-conversation binding; otherwise a
trusted independent operator verifies and registers the exact host provider/root/
session through configuration the model cannot change. A per-thread client API
does not itself prove an external MCP server receives trustworthy thread identity.
Do not build speculative authentication before obtaining real transport evidence.

## Consequences and validation

The accepted real conversational metadata search remains usable. This is weaker
than complete content discovery and guaranteed automatic self-exclusion. Historical
metadata-only client OPEN receipts keep their original outcomes; the README and
roadmap record the user's later accepted 37-record invocation separately.

Test blank/keyword/exact-UUID/archive search in both modes, same-UUID foreign roots,
protected preview and preparation refusal, and missing-registration refusal. Keep
other-session preservation, old encoding/replay, crash, native cascade and the
M8.2.1 global lost-update regression. The next milestone's content, privacy,
performance and verified-binding acceptance cases are in the search contract.
