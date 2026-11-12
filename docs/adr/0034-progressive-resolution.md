# ADR-0034 — Progressive session resolution

## Status

Superseded by [ADR-0036](0036-two-stage-luna-resolution.md) for orchestration and publication. [ADR-0035](0035-uuid-search-cohort.md) specifies the UUID inventory.

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

Assessments distinguish the activity in the candidate from references to
another session. Fresh publication checks identity and cited source revisions
in the confined worker. The Luna workflow judges the meaning of the evidence.

## Verification

Exercise stable large corpora, partial and missing histories, source changes,
replayed requests, and caller isolation in disposable tests. Compare call
counts and evidence at equivalent work. Keep lifecycle and provider write
denial checks in the release gates.

## Per-session verification and lexical completion

Each Luna judgment applies to one candidate. A direct judgment with fresh
source-bound evidence is confirmed. A judgment with insufficient evidence is
unconfirmed, and only a plausible contender with fresh supporting passages is
published as an alternative. Related or contradictory evidence can reject a
candidate for the requested activity. Rust validates evidence identity and
revision before publishing a row. Logical history coverage is required for
session-wide absence claims. A local positive can stand on partial history.

The run tracks execution independently. Ordinary completion requires a
finished lexical traversal and a conclusive assessment for every retrieved
candidate, including any admitted expansion. An unconfirmed judgment can
conclude a candidate. Unavailable sources, stale evidence that cannot be
revalidated, cancellation and unavailable Luna have explicit interruption
semantics. Keep scope and coverage facts separate from the two result lists.
A lexical pass cannot certify semantic recall.

The progressive MCP contract carries these states alongside the existing wire
fields. The originating chat assembles the two result tables from streamed
confirmed and plausible unconfirmed rows. The resolver keeps only the active
batch, the continuation and one replayable reply. Selection and Boolean
queries use the same progressive actions.

## One resolver and required Luna orchestration

Every conversational request uses one frozen SearchIntent and the same
progressive stream. Structured Boolean predicates and exact identities use the
same typed query evaluator as ordinary lexical plans. The origin accumulates
signed rows, and Luna can select a single best from those rows after traversal.
The origin retains the result policy, while acquisition, assessment and
publication are identical. Existing bounded context navigation and
deterministic discovery remain separate read-only primitives.

The Codex host owns model selection, delegation and execution observations.
Project instructions require the origin to confirm a Luna Medium execution
before submitting any semantic judgment, including an explicit pinned fallback
when the named role is unavailable. An unavailable interpreter stops before
retrieval. An unavailable assessor interrupts publication. The MCP boundary
accepts caller-scoped structured judgments and cannot authenticate their
model origin. It admits no model-identity claim or new lifecycle authority.
