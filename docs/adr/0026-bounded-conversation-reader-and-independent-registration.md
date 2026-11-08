# ADR-0026 — Bounded conversation reads and independent management registration

Status: Accepted implementation decision, 2026-11-08. Supersedes ADR-0025's
unimplemented search design; historical acceptance and mutation verdicts stand.
P0-S1 remains **OPEN** pending actual updated conversational client use and an
independently established binding of the real management conversation.

## Decision

Keep the eight crates. Pure domain types describe lexical relevance, read bounds,
evidence and continuation. Provider-api adds `SessionSearchReadPort`; application
requires a trusted exact provider/root/session binding before any management read.
Codex owns its fixed SQL and rollout interpretation. CLI composes the reader and
independently controlled registration; MCP only validates and projects bounded data.
No index, embeddings, service, new crate or destructive operation is introduced.

The installed native App Server cannot initialize real state under the existing
kernel write denial. There is no unrestricted retry. Reuse the attested helper,
read-only SQLite snapshot (32 MiB/3 seconds), fixed selected-root rollout paths,
descriptor-relative no-follow opens, regular/unshared source files and fresh file
stamps. The content worker permits reads of `sessions` and `archived_sessions`,
not auth/configuration; the provider executable is not started for content reads.
The literal root-directory allowance permits descriptor traversal, not home-file
contents. Existing sandbox negative tests retain auth/link/write/network denial.

Support known visible `response_item/message` text for absent/legacy history mode.
For `paginated`, use completed display `UserMessage`/`AgentMessage` text items,
checking the event's thread identity. Ignore its duplicate model view. The exact
alpha.6 public protocol and rollout policy establish these shapes; strict tests
exercise them. Unknown formats and missing files remain partial/unsupported.
Neither the stable mutation hash nor general producer-version equality determines
read compatibility. Do not infer complete paginated history hydration.

Search at most 100 eligible sessions per batch and 10,000 DB rows per inventory.
Read at most 1 MiB/session, 256 KiB/record, 4,096 captured records and 32 MiB/call;
the worker has an 8-second deadline, parent 12 seconds. Large files use half header
and half tail windows; discard their clipped boundary records. The middle remains
unread. Context reproduces the exact windows and revalidates their hash/stamp.
Do not search metadata first and use it to gate transcript eligibility. All terms
must occur across the union of metadata and supported safe messages. Weight each
term once per field/message: user 4, assistant 3, title 2, other metadata 1. Sort
within a batch, with exact identity as tie-breaker. This is lexical, not semantic
or globally ranked search. HMAC cursors bind purpose/query/filters/root/exclusions,
registration revision, metadata inventory and the revisited batch. They expire on
server restart; content in later batches is observed fresh, not a frozen global
transcript snapshot. Live changes can refuse continuation.

## Caller trust

Current MCP initialization/clientInfo/request metadata does not prove a hosting
conversation. No model argument or environment UUID is accepted as caller proof.
The narrow fallback is a root-owned, non-model-writable, ACL-free registration
chain for one randomly identified running server, exact physical provider root,
canonical session UUID and expiry no longer than one hour. The server rejects
running as root. It has no registration writer or privileged helper.

An independent operator must first prove that this connection is exclusively
used by the named conversation. A root file cannot establish host routing by
itself. Shared/reused connections and unverified host identity remain inadmissible.
The registration procedure is available, but real host exclusivity and independent
operator acceptance have not been established on this workstation. Automatic
caller binding and production guaranteed-mode support are not claimed.

Without valid independent registration, `purpose=management` returns
`caller_identity_required`, zero candidates and an ephemeral bootstrap instance.
The application refuses before calling any provider. Valid exclusions enter SQL
before matching, inventory limits, counts, ranking and paging; the worker confirms
the exact management identity exists. Context rechecks binding/exclusions/freshness.
Old `--protect-codex-session` values remain retained roots for the existing
preview/approval/execution path; they alone do not authenticate a management caller.
Search references never grant destructive authority. Diagnostic/legacy searches
are explicitly marked as having no caller exclusion guarantee.

## Evidence and consequences

Return at most 20 hits, two 320-character excerpts/hit, opaque provenance and one
bounded context message (512 characters/1,024 UTF-8 bytes). Historical text is inert
untrusted data. Suppress whole messages containing conservative secret/config/env
markers before scoring or output; expect false positives and imperfect detection.
No extracted text/query/index is persisted by AgentMender. The client may retain
received excerpts; users can disable excerpts or content. No full-transcript API.

The fixture matrix covers neutral titles, cross-message terms, relevance,
protected strongest matches, blank/archive/project/pagination/counts, missing or
forged/stale identity, partial formats, injection, secrets, links, changed evidence
and 5,000-session stable bounded paging. Real-provider drivers and actual model
tool use have separate acceptance columns. Claude reuses the port/application/MCP
contract but requires its own evidenced adapter; no Claude content parser or
mutation profile is added. Destructive and recovery regressions remain mandatory.

Primary format references (read at the exact alpha.6 tag):
[model messages](https://github.com/openai/codex/blob/rust-v0.166.0-alpha.6/codex-rs/protocol/src/models.rs),
[display items](https://github.com/openai/codex/blob/rust-v0.166.0-alpha.6/codex-rs/protocol/src/items.rs),
[completion events](https://github.com/openai/codex/blob/rust-v0.166.0-alpha.6/codex-rs/protocol/src/protocol.rs),
[rollout persistence policy](https://github.com/openai/codex/blob/rust-v0.166.0-alpha.6/codex-rs/rollout/src/policy.rs).
