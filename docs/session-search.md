# Session search and management identity

Canonical search/security contract under [ADR-0025](adr/0025-bounded-search-and-management-exclusion.md).
The [roadmap](product-roadmap.md) controls implementation order. This document
specifies the next read-only step; it does not claim transcript search is shipped.

## Current accepted behavior

The user accepted an actual Codex model invocation of
`session_search(query="MISSION1", include_archived=true)`: 37 real records,
explicit Partial coverage. A cleanup-time invocation returned 36 records with
Partial coverage. Live counts are observations, not stable acceptance assertions.
Earlier [real-world](milestones/p0-c1-real-world-integration.md) and
[client](milestones/p0-c1-client-acceptance.md) receipts retain their then-OPEN
client verdicts. The later read-only acceptance does not accept deletion.

`SessionMetadataReadPort` → `provider-codex/metadata.rs` →
`application/conversation.rs::search_inventory` → `mcp-server/conversation.rs`
provides opt-in display-name/title, project basename and UUID all-term substring
matching. A blank query lists bounded metadata. Available timestamps, archive
state, producer and parent observations are projected, not searched numerically.
Duplicate labels and provider/root identity remain distinct. Inventory is bounded
to 500 recent records/96 KiB from the selected DB; MCP budgets may refuse oversized
responses. Results always remain Partial; no pagination or relevance ranking is
implemented. `search_basis` explicitly says metadata only, no transcript search.

Registered Codex identities in the configured root are excluded in application
before query matching, sorting and projection in both conversational modes.
Read-only and supported preview refuse protected targets; supported preparation
does too. A same-UUID record in another root is distinct and cannot be previewed
through this root. Internal redundant protection flags were removed; existing
wire fields remain compatible. Search in removal mode requires registrations.

Without verified caller identity, read-only browsing remains diagnostic metadata
access only: `authenticated_caller_binding` is false. It can include the caller.
No automatic management-session exclusion guarantee is made.

## Required content-search contract

Search title, explicit project/workspace label, supported metadata, user messages,
assistant messages and relevant supported transcript text. A title containing
keywords is evidence of a title match, never sufficient proof of conversational
relevance. A neutral-titled conversation discussing Earth/Cesium rendering must
be discoverable even when none of those words occurs in its title or project.

Each match must return these bounded facts:

| Fact | Required meaning |
|---|---|
| Identity | Provider, opaque root namespace, exact native session ID and restart-scoped target reference |
| Labels/activity | Available title, explicit project label, creation/activity and archive observations; missing is unavailable |
| Match evidence | One or more `title`, `metadata`, `user_message`, `assistant_message`, `conversation_content` sources |
| Relevance | Deterministic matched terms and source-weight explanation; lexical score is not semantic certainty |
| Excerpt | At most two 320-character excerpts per match, at most 640 characters combined; omitted when sensitive or unsupported |
| Provenance | Reader/format contract, opaque evidence reference, record/turn/item position and source fingerprint; no raw path |
| Coverage | Sources attempted/skipped, inspected sessions/bytes and explicit reasons for Partial/Unsupported/budget exhaustion |

Proposed input extends the existing bounded query with explicit content opt-in,
optional exact root/project/archive/time filters, page size <=20 and an opaque
cursor. It accepts neither arbitrary paths/SQL nor a caller UUID. Additional
context, if necessary, uses an exact prior evidence reference, the same verified
exclusion and privacy checks, a <=2 KiB response and no arbitrary file/offset API.
No unrestricted full-transcript endpoint. These fields are proposals, not current
tool arguments; version and test the DTO changes before exposing them.

Apply explicit metadata filters before content reads, but never require query
words to occur in metadata before scanning content: that would hide the example
above. Exclude verified management identities before source/result limits,
matching, ranking, pagination, counts or context extraction. Stable tie breaking
uses exact provider/root/session identity; never collapse duplicate labels.

## Reusable access and missing readers

| Existing path | Reuse / limitation |
|---|---|
| Codex `metadata.rs`, `capture.rs`, provider-api evidence/process isolation | Fixed-column worker, attestation, READ_ONLY/NOFOLLOW, query_only, bounded RAM backup; intentionally excludes bodies and rollout paths from MCP |
| Installed alpha.6 `ThreadListParams` evidence | `searchTerm` searches extracted title only; `excludedThreadIds` filters before the provider limit, but does not authenticate those IDs |
| Installed `ThreadReadParams` and native schema | `includeTurns` can hydrate history; generated advertisement is not safe startup or bounded read admission |
| Codex `scoped_files.rs::header`, `inspect_rollouts` | Exact session-meta/physical/filename checks for supported Legacy payloads; not message extraction or support for paginated history |
| Claude `lib.rs::transcript_shape`, `parse_metadata`, `read_sidecar` | Bounded synthetic metadata-prefix/sidecar fixtures; live unknown profiles deliberately avoid parsing conversation bodies |
| provider-api `filesystem::{read_bounded,read_prefix}` | No-link scoped primitives and bounded reads; whole-prefix reads alone cannot search large transcripts completely |
| Application `QueryControl`, MCP frame/projection | Cancellation, deadlines, response bounds, opaque targets and privacy; no content retrieval port currently exists |

Prefer demonstrated native paged history access when its entire startup/read
boundary satisfies source-write denial. Official [App Server documentation](https://learn.chatgpt.com/docs/app-server)
describes `thread/turns/list` and `thread/items/list`; the active store must support
item pagination. The installed read-only startup previously refused under kernel
denial. Do not retry unsandboxed or resume a thread to search it. Otherwise add a
small provider-owned bounded streaming reader for an evidenced format. Unknown
Legacy/paginated/Claude formats remain unsupported; do not guess table or line
semantics from filenames. No provider-native full-content search is established
by the checked-in title-search schema.

## Privacy, bounds and scale

Treat every label and excerpt as untrusted quoted data. Historical instructions
never become executable commands, tool calls, configuration or higher-priority
agent instructions. Restrict extraction to supported user/assistant text. Exclude
system/developer prompts, auth/config, tool outputs, environment dumps, attachments,
binary data and hidden reasoning. Suppress credential/token/private-key patterns
and whole sensitive records; redact before returning evidence. Pattern matching
alone cannot guarantee absence of all secrets, so disclose limits and allow
content/excerpts to be disabled. Do not persist queries, excerpts or transcripts
in logs, journals, scratch, reports or caches. Source paths never come from MCP.

Initial proposed request ceilings: 500 inspected sessions, 32 MiB total source
bytes, 1 MiB per transcript and 256 KiB per record, 5-second cancellable scan,
20 returned matches and the existing MCP byte budget. Enforce bounds while reading,
before allocation. Omitted/unknown/changing sources, unreadable archives and budget
cuts yield Partial with a reason; exhausting the last response slot is not full
coverage. A protected identity must not consume a session/match limit. Cursors
bind query/filters, verified caller/exclusion revision and source snapshot; changed
inputs invalidate cursors rather than duplicate/drop evidence silently.

Start without an index: stream supported records and retain only bounded matches.
Measure bytes read, peak memory, cancellation and latency on thousands of
disposable sessions, including late matches. The current 500-most-recent reader
cannot establish older-session absence; the next reader needs bounded continuation
or explicit incomplete-source reporting. Do not raise caps silently to obtain a
match. Only measured need can justify a later local index, with a prior ADR for
private disjoint storage, permissions, stale/deleted-session invalidation, root/
schema changes, sync, explicit clearing and preventing recovery of deleted text.
No external database, embedding service or semantic-search subsystem is proposed.

## Trustworthy management identity and audited gaps

Required identity is the host provider/root/exact conversation, independent of the
target provider. Use a host-authenticated binding tied to the connection/request,
or an independently verified operator registration outside model-writable control.
Rebind on conversation switch/reconnect and recheck before context/preview/plan/
execution. Never infer from title, cwd, project, recent activity, UUID ordering,
archive state or an arbitrary model field. Protect both the current thread and
any separately established host-session scope; do not confuse a thread-tree root
with proof of the current conversation identity.

The current stdio server ignores initialize client metadata for authorization and
receives no verified host thread identity. Official [MCP configuration guidance](https://learn.chatgpt.com/docs/extend/mcp)
describes connection/auth configuration; it does not establish per-conversation
binding for this server. The App Server's thread-scoped MCP client API does not
prove that identity is forwarded authentically to an external stdio tool. This
is an evidence-based integration gap, not a claim that every client lacks it.
Claude host binding has not been demonstrated either.

Smallest missing integration: capture sanitized real Codex and Claude MCP
initialize/tools-call envelopes and establish whether the host forwards a verified
provider/root/thread identity outside model-controlled arguments. If absent,
provide an independent operator-controlled launch registration and attest its
association with the actual management conversation, including switch/reconnect
and model-write restrictions. No new OAuth or signing framework is authorized
by the cleanup. OAuth account identity alone is not a conversation binding.

| Path audited | Current protection / remaining work |
|---|---|
| Conversational blank/keyword/UUID/archive search | Registered identities filtered before matching/projection in both modes; automatic caller identity absent |
| Project/content search, rank/page/count/context | Not implemented; same exclusion must precede all these operations |
| Read-only/support preview and supported prepare | Protected identities refuse; supported path requires registered UserProtected graph roots |
| CLI planning/approval/execute | Existing roots frozen in original plan, restored and freshly checked; real independent operator/caller deployment still unaccepted |
| Native/scoped adapter preflight and retained-state verification | Existing exact identity, protected-root/freshness/collateral refusal retained; no new admission |
| Default MCP workspace_scan/graph/inspect and CLI scan/inspect | Diagnostic read-only inventory can show protected or unbound sessions; no guaranteed management-list policy wired here |
| Transaction status/recovery | Historical authenticated knowledge, not candidate discovery or permission to retry |

Every future management listing, including generic tools used in that workflow,
must share verified exclusions or return `caller_identity_required` with zero
candidates/counts/excerpts. Current general diagnostics must not be relabeled as
safe management candidate lists. Existing `management_protection_required` is
the removal service's missing-registration error, not a new authentication proof.

## Next small milestone and acceptance tests

**P0-S1: bounded content evidence with verified management exclusion**, read-only.
Do not start deletion, Claude mutation, new artifact cleanup or indexing.

1. Establish actual caller evidence first; support one independently verified
   deployment and explicitly refuse guaranteed workflows without it.
2. Add only the narrow read port/evidence types needed by one validated Codex
   transcript representation. Preserve the current metadata API and per-source
   coverage. Unknown formats and unsupported Claude content report limits.
3. Add bounded streaming extraction, safe evidence/term ranking and MCP projection;
   apply shared exact identity exclusion before any limits/counts/context.
4. Exercise the following deterministic cases and a real read-only Codex client;
   capture no private transcript payload in repository evidence.

| Acceptance case | Required result |
|---|---|
| Neutral title, Earth/Cesium rendering only in user or assistant text | Exact conversation found with bounded grounded excerpt/source, without title overlap |
| Misleading title, irrelevant body; duplicate titles and same UUID in another root | Explain source and preserve ambiguity/exact namespaces; no automatic target choice |
| Management transcript is the strongest match, including archived/project scope | Absent before top-K, pagination, counts and context; no suppressed identity leakage |
| Missing/spoofed/stale binding, conversation switch or cross-provider host | Guaranteed workflow returns caller_identity_required, zero candidates; stale cursor/context/preview refused |
| Exact protected selection/preview/plan/execution via previously acquired ref | Refusal survives fresh validation and original-plan recovery; no effects |
| Secrets, prompt injection, giant/malformed lines, unknown formats, traversal/links | No command execution, secret/path/transcript dump, unbounded allocation or guessed parsing |
| Thousands of sessions, late match, truncation, changing source, cancellation | Bounded bytes/memory/deadline; explicit Partial and stable/invalidation-tested continuation |
| Read isolation and old safety suites | No source writes or approval-state access; exact encodings, replay, consumed intent and lost-update regression unchanged |
