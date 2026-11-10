# ADR-0030: Retain bounded decoder state for progressive history reads

## Status

Accepted, 2026-11-10, P0-SR3. Supersedes ADR-0029's total compressed-file
and cumulative replay ceiling; preserves its source and isolation boundary.

## Decision drivers

Codex compression writes an ordinary single Zstandard frame. Arbitrary decoded
offsets are not independently seekable. Reopening a decoder for every request
both repeats prefixes quadratically and makes a per-request replay budget an
invisible total-history ceiling. Increasing that ceiling does not solve this.

## Decision

Reuse the existing kernel-confined content worker through a bounded framed
request loop. Retain a small, explicitly bounded in-memory decoder working set,
keyed by exact root, selected path, session and physical revision. Keep a bounded
recent decoded-byte buffer for record replay and nearby context. Never serialize
decoder internals, silently materialize transcripts, or create a derived database.
The CLI retains its read adapter for the MCP connection lifetime.

Each request receives fresh deadlines and physical/decoded work budgets. Total
physical file length is not an admission criterion. Sequential continuation reuses
decoder state; distant positions may require bounded replay. Replay continuation
reports an increasing verified decoder position separately from the requested
logical record position. Expired state, corruption, changed identity or revision
refuses explicitly. No identical continuation is a successful response.

The worker takes a fresh metadata snapshot and revalidates source selection on
every operation. When metadata changes, context rechecks the exact target locator
and physical source revision in a fresh snapshot; unrelated row updates do not
force a global refusal. Target/source/root/DB identity changes still refuse. Idle workers do no decoding; cancellation/timeout releases the
parent operation, and a timed-out worker is terminated without source effects.
State is transient and bounded; eviction may increase random-access latency.
Existing opt-in private indexing remains the only persistent text cache.

Physical EOF is separate from supported-record coverage, logical-session coverage
and eligible-corpus coverage. Paginated inline histories remain logically Partial
at verified physical EOF; unsupported/missing segments and suppressed records
cannot establish negative evidence.

## Alternatives

Frame-aware seeking is unavailable for the observed single-frame sources.
Portable decoder snapshots are unsupported by the selected library. Replaying
from zero per page is quadratic. Default transcript persistence violates the
existing explicit privacy choice. A new service or crate is unnecessary.

## Caller identity

This decoder change grants no caller authority. An originating host thread ID
must be bound to each request and the exact provider root through an enforceable
host integration. Model arguments, transport session IDs, process identity and
operator assertions do not establish routing. Missing host admission stays closed.

## Validation

Exercise large single-frame sources, aggregate work above prior limits,
continuation/replay progress, EOF, bounded memory, corruption, source replacement,
exclusion before source access, and existing query/context/index/lifecycle gates.
Separate real-provider, fixture and actual-client evidence in the milestone report.
