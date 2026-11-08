# Session search and management protection

P0-S1 implements bounded Codex content evidence and fail-closed guaranteed-mode
search. Verdict **OPEN**: actual updated conversational client acceptance and
independently verified protection of the real management conversation remain open.
See [ADR-0026](adr/0026-bounded-conversation-reader-and-independent-registration.md).
The earlier actual `session_search(query="MISSION1", include_archived=true)` call
returned 37 real metadata records, Partial. That legacy call stays supported;
changing live counts are not a frozen inventory or a deletion proof.

## Modes and tools

Launch `agentmender --provider codex mcp --session-search`. Default plain `mcp`
keeps eight tools; opt-in conversational mode exposes fourteen. No approval store
is initialized by read mode. Preview remains blocked without a plan; prepare,
status and execution handoff refuse `read_only_removal_unavailable`.

`session_search` accepts strict fields:

| Field | Default and bound |
|---|---|
| `query` | Empty; <=256 UTF-8 bytes, <=16 whitespace terms; no controls |
| `include_archived` | true |
| `include_content` | false; opt in to supported human-visible text |
| `purpose` | `diagnostic`; use `management` for a guaranteed candidate list |
| `project_label` | Optional exact project basename, <=256 bytes |
| `page_size` | Optional 1..20, advanced default 20 |
| `cursor` | Optional process-bound authenticated continuation, <=4096 bytes |
| `excerpts` | true; false suppresses text projection |

Calls using only the original query/archive inputs retain the existing metadata
response, recent inventory bound (500/96 KiB), labels/timestamps/optional parents,
duplicates and Partial coverage. They are labeled diagnostic with
`registered_only_no_caller_guarantee`. Explicit advanced fields select the new
bounded path. Diagnostic results, including content results, are never a guaranteed
management list. Do not turn a diagnostic result into proof of safe selection.

Example guaranteed request (refuses until independently bound):

```json
{"query":"Earth rendering performance Cesium camera","include_archived":true,"include_content":true,"purpose":"management","page_size":20}
```

For read-only troubleshooting, deliberately select `purpose:"diagnostic"` and
retain its `not_guaranteed_diagnostic` label. No caller UUID, raw source path,
provider SQL, shell command, principal or registration proof is a tool input.

`session_context` takes the exact returned `target` and `evidence_ref`, retrieving
one prior supported message under fresh binding, exclusion, identity and source
checks. It is not an arbitrary transcript endpoint. Tokens are HMAC authenticated,
kind-separated and process-local; changed filters/query/root/registration or a
restarted server invalidate them. A stale source refuses; reacquire search evidence.
Context returns no new context chain.

## Supported Codex reads

The current macOS implementation uses the existing kernel-attested helper with
source writes, network, auth/configuration and process spawning denied. SQLite is
READ_ONLY/NOFOLLOW/query_only and copied only to bounded memory. Only fixed DB
columns are interpreted. A rollout locator must be within the selected root's
`sessions` or `archived_sessions`, with no symlink/hardlink/shared-file traversal,
and an exact session header. Stale missing locators remain explicitly unreadable;
there is no unrestricted filesystem fallback or filename ownership inference.
No native session resume, model execution or provider/application restart occurs.

| Contract | Supported visible data |
|---|---|
| `codex-rollout-response-item-message-v1` | Header history mode absent/legacy; `response_item` message user `input_text` and assistant `output_text`, known visible phase/channel only |
| `codex-paginated-item-completed-message-v1` | Paginated header; `event_msg/item_completed` with exact thread ID and `UserMessage` text or `AgentMessage` Text parts; skip duplicate model view |

System/developer text, reasoning, tool calls/results, inter-agent messages,
attachments and unrelated file/config/env data are not extracted. Unknown record
shapes are not guessed. Additive DB schemas and unknown producer versions can be
readable when these exact shape contracts hold; this is separate from mutation
admission. The alpha.6 protocol/persistence references are linked in ADR-0026.
Paginated support reads known display items in the bounded source windows only;
it does not hydrate complete provider history or inspect a separate history store.
Live source changes and unsupported/oversized/malformed records reduce coverage.
Linux/Windows fixture coverage does not provide a live isolation backend.

## Matching, evidence and limits

Apply exact trusted exclusions first, then archive/project filters and content
reads. Query terms never gate transcript reads through title matches. Lowercase
substring terms must all occur in the union of eligible metadata and safe messages.
Every term hit adds user 4, assistant 3, title 2 or other metadata 1 per message/
field. Return score and match sources; sort within the inspected batch, then by
exact identity. There is no semantic claim, global top-K or silent deduplication.

| Bound | Value |
|---|---|
| Metadata inventory | 10,000 eligible rows; fixed SQL cap after trusted exclusion, before project basename filtering |
| Scan batch / results | 100 eligible sessions / at most 20 hits per page |
| Transcript / record / call | 1 MiB / 256 KiB / 32 MiB |
| Captured records | 4,096 per session |
| SQLite snapshot | 32 MiB, 3 seconds within the worker deadline |
| Worker / parent | 8 / 12 seconds; existing MCP deadlines/cancellation also apply |
| Excerpts | Two per hit, each at most 320 Unicode characters |
| Additional context | One message, <=512 characters and <=1,024 UTF-8 bytes |

A large transcript uses half the read budget for the header and half for its tail.
Clipped boundary records and the omitted middle are not inspected. `record_index`
is an ordinal within this bounded capture, not the original whole-file line.
Evidence includes source role, format, capture description, truncation and a sealed
reference to the exact source stamp/window digest. No source path or full dump is
exported. Freshness is checked on the open descriptor and a reopened no-follow
path; context repeats the same windows. Different captures are not a provider-wide
atomic snapshot. Cursors bind the metadata inventory and revisited batch; later
batches are observed fresh. Changes may require a new search.

Every advanced response is Partial, with inspected/eligible counts, bytes,
unsupported/truncated/suppressed records, reasons and `next_cursor`. Counts exclude
trusted protected identities before all matching/limits/paging. Follow cursors
through empty match pages to continue the inventory. Result pages in the same
batch repeat the same scan and bounds. Do not sum repeated coverage as distinct
sessions. The 10,000-row cap and selected-root boundary cannot establish absence.

## Independent management registration

Current MCP clientInfo, request metadata and environment provide no authenticated
per-conversation caller binding. A model-selected UUID/title/cwd/archive/activity
or project cannot establish it. Old trusted `--protect-codex-session` values still
exclude retained roots but do not by themselves authenticate the current caller.

Guaranteed `purpose:"management"` requests require a fresh independent registration
before any provider read. Missing, expired, wrong-instance/root or model-writable
registration returns `caller_identity_required`, `sessions:[]`, `result_count:0`,
`authenticated_caller_binding:false` and the ephemeral `registration_instance`.
Even a missing binding with an empty query returns zero candidates. Other invalid
source/schema/freshness conditions safely refuse rather than weakening protection.

The available macOS procedure is for an **independent operator**, outside the
model's shell/keys/PTY control:

1. Independently establish the actual host conversation's canonical native UUID,
   exact selected provider root and that this single server connection is used
   exclusively by that conversation. If host routing/exclusivity cannot be proved,
   leave management mode refused. A statement in a JSON file does not prove it.
2. Configure a fixed absolute `--management-registration` file path with
   `mcp --session-search`. Ask the running server for a management search and obtain
   its fresh bootstrap instance. Never reuse an instance from a previous process.
3. The operator creates the file in a root-owned directory chain (for example a
   dedicated directory under `/Library`), no symlink, ACL or group/world write on
   any component. The server must run as a non-root model user. No privileged
   registration writer is supplied by AgentMender.
4. Record exact physical root device/inode, expiry in Unix seconds no more than
   one hour ahead, and the independently established scope. The root-owned JSON
   has these strict fields (placeholders require operator verification):

```json
{"schema":1,"server_instance":"<fresh-instance>","provider":"codex","state_root":"<canonical-root>","root_device":0,"root_inode":0,"native_session_id":"<exact-native-uuid>","expires_at":0,"connection_scope":"single_conversation_operator_verified"}
```

The server rereads and checks the descriptor/control chain and registration every
call, validates the protected identity exists, and excludes it in bound SQL before
inventory counts/limits/matches/ranking/paging. Context also refuses protected or
stale references. No registration is automatically minted during this task.
This mechanism has negative/typed-binding tests; successful real independent
operator registration and host exclusivity remain unverified. It grants no
approval/execution authority and does not change historical destructive setup.

## Privacy and acceptance

All retrieved text is `untrusted_data:true`. Instructions in it remain inert.
Suppress whole messages with conservative secret/auth/config/env markers before
scoring and projection. This deliberately causes false negatives and is not
perfect redaction. Disable excerpts or content when appropriate; the caller may
retain received excerpts. AgentMender keeps no persistent transcript index, query
log or extracted text. Verification receipts contain aggregate counts/lengths,
formats and statuses, not real transcripts or credential copies.

Fixtures cover neutral titles and duplicate-title comparison, cross-message terms,
protected highest matches before blank/archive/project/count/paging, forged/missing
binding, stale context/cursors, secret/reasoning/tool suppression, inert injection,
unknown/link/traversal sources, byte charging on failure and 5,000-session paging.
[Real provider read evidence](../compatibility/codex/0.166.0-alpha.6/content-search-p0-s1/README.md)
includes user/assistant content and a fresh context read via a direct MCP driver.
Actual provider read tests, actual model tool calls, independent real management
exclusion and production acceptance must be reported separately. The existing
metadata-only conversational acceptance does not close the new content gate.

Claude reuses the pure evidence/relevance types, `SessionSearchReadPort`, application
binding/exclusion/continuation and MCP projection. P0-C2 still needs an independently
evidenced Claude message adapter, scoped read isolation/freshness/format tests and
its own real client/identity acceptance. Do not guess Claude formats or start
Claude mutation here. The next milestone closes these P0-S1 real client/binding
gates before any new real-session deletion functionality.
