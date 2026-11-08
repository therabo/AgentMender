# AgentMender — P0-S1 Content Search & Management Exclusion

Implementation receipt, 2026-11-08. Verdict **OPEN**. The final exact commit/CI
receipt is delivered outside the repository to avoid a self-referential SHA.
Starting main: `ff8b5fe2ee0fadb0c1a407f9e4be7bff240fa24c`.

| Gate | Status |
|---|---|
| Code implemented | Yes: bounded Codex content search/context and management-mode refusal |
| Fixture-tested | Yes: neutral titles, lexical relevance, exclusion, trust, privacy, isolation and 5,000-session paging |
| Real provider read-tested | Yes: installed alpha.6, both visible roles, 24 bounded evidence records and fresh context; Partial |
| Actual conversational client-tested | Legacy metadata yes (36 current MISSION1 records); updated content schema not yet exposed to this chat |
| Management exclusion verified | Typed independently bound fixtures yes; missing/forged binding refuses; real host identity/exclusivity not established |
| Production-supported | Guaranteed real management workflow not accepted; automatic caller binding unavailable |

The [canonical contract](../session-search.md) specifies the two known legacy/
paginated shapes, header/tail capture bounds, source freshness, within-batch lexical
ranking, signed evidence/context and independent root-controlled registration.
[ADR-0026](../adr/0026-bounded-conversation-reader-and-independent-registration.md)
records the eight-crate architecture and trust decision. The
[sanitized real read receipt](../../compatibility/codex/0.166.0-alpha.6/content-search-p0-s1/README.md)
contains no transcript text, session UUIDs, private roots or credentials.

All eight local gates passed: fmt, warning-free Clippy, workspace tests/build,
architecture and both historical compatibility validators, diff whitespace.
Workspace tests: 483 primary tests passed, 8 nested child-process checks also
passed, 8 explicit opt-ins ignored by default. Three relevant pinned stable-binary
disposable integrations were then run explicitly and passed, including the full
stdio preparation/external execution/reopen/status fixture. Its restart concerns
only the disposable AgentMender server; Codex was never restarted. Fixture grants
are not independent human approval. No real sessions were deleted.

The largest paging fixture visits 5,000 eligible sessions in 250 pages, <=100
inspected sessions and <=20 results per page, with the protected highest-priority
identity absent before counts/limits. Debug execution took about 36 seconds across
all pages; it is a metadata pagination correctness test, not a full transcript
throughput benchmark. Real batches stayed within the 32 MiB read bound. Source
changes during live use are explicitly partial and can end continuation.

The updated MCP connection still has to be exercised by an actual conversation.
An independent operator must prove the real management UUID/root and single-chat
connection routing before registration; a root-owned declaration alone cannot
prove host exclusivity. Until then management requests refuse with zero candidates.
Next: close those P0-S1 acceptance gates, then implement the evidenced Claude read
adapter and equivalent tests for P0-C2. Do not add real deletion functionality,
Claude mutation, bulk work or Desktop global publication in this milestone.
