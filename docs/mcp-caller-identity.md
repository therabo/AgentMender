# MCP caller identity: management admission remains OPEN

## P0-SR4 apparent caller for read-only discovery

`session_discover` reads `threadId` and optional `sessionId` from
RMCP's `RequestContext.meta`. The persisted Codex identity is the exact
native thread row keyed by `threads.id` in the configured root. That UUID
forms the Codex session artifact identity. App Server describes its session
ID as the live session tree root. Root threads use the same UUID. Forked
threads can carry a different tree root UUID, so the current implementation
refuses a discordant pair. A thread-only call is supported when its exact
canonical row exists. The caller's existence is rechecked in the confined
worker for discovery and context. The worker reads no caller body.

Discovery binds its cursors and context to that apparent caller, exact
provider/root and the existing process-local signatures. Discovery target
tokens also have a separate namespace from general entity and lifecycle
targets. Explicit diagnostics retain their separate behavior. An arbitrary
client can misrepresent its own apparent caller through spoofed metadata.
This mode has no authenticated host claim and no removal authority.

The [SR4 receipt](milestones/p0-sr4-implicit-caller-discovery.md) records
actual client observations and the manual two-chat procedure.

P0-SR3 investigated desktop 26.1103.60933/build 25894 and CLI
0.167.0-alpha.3.2. These differ from SR2's observed desktop build 25679.
No installed application bundle, credential store, session or MCP configuration
was modified. AgentMender does not identify its caller from title, cwd, activity,
parent PID, transport session ID, environment variable or model argument.

## What is established

Codex supports global and trusted project MCP configuration, not a documented
conversation-only registration authority. The current official source adds
`threadId` and `sessionId` to model-originated tool-call `_meta`; its direct
`mcpServer/tool/call` path overwrites `threadId` from the routed outer request.
Its connection pool supports negotiated sharing, so one process/connection must
not imply one conversation. A transport session ID identifies neither the exact
Codex thread nor the root session tree identity in `thread.sessionId`.

The installed binary's generated schema, obtained under network/source-write
and auth/config read denial, confirms thread-targeted MCP calls with an optional
JSON `_meta` object. The installed Desktop direct-call wrapper also sets
`threadId` and `thread_id` from its routed request. Its dynamic tool callbacks
carry `threadId`, `turnId` and `callId` to the App Server client. The inspected
Desktop dispatch provides no third-party adapter to attest those claims to
AgentMender. Upstream source corroborates the routing and connection sharing.

MCP authorization does not specify a provider-thread attestation for local STDIO.
The observed `_meta` values can be forged by another client running AgentMender;
the regression sends forged `threadId`/`sessionId` values and still receives zero
management candidates. AgentMender does not authenticate the STDIO peer or its
provider root. Merely finding a UUID in request metadata therefore cannot create
`ManagementIdentity`. No self-issued token or privileged assertion file is admitted.
A diagnostic in the new refusal reports only presence of thread metadata, never
its content or an authentication claim.
RMCP moves wire `_meta` to `RequestContext.meta`. The earlier diagnostic read
the emptied parameters field, so its false value cannot establish wire absence.
The corrected diagnostic reports true through the restarted real Desktop
connection, while `authenticated_host_route` remains false.

## Missing enforceable contract

Production admission requires a host-owned route carrying the actual originating
thread UUID and caller provider/root physical identity on every request, with an
independently verifiable issuer/audience, request/connection epoch, freshness and
replay policy. Model tool arguments and arbitrary client metadata must not control
those claims. Reconnects and conversation switches must rebind; stale evidence
and cursors must fail before source reads. Target provider identity must remain
separate, including future Codex-to-Claude/Claude-to-Codex searches.

An operator-controlled companion could consume supported App Server dynamic-tool
callbacks on a connection it actually owns and attest the routed thread to
AgentMender. To enforce this, its authority and request channel must be outside
the model's control, its provider-root mapping must be validated, and every request
path must be mediated. A same-user wrapper, root-owned registration file, PID
census or model-accessible signing secret does not meet that contract. No existing
supported adapter satisfying it was established. Starting a new App Server against
real provider state or borrowing the desktop's transport/credentials is not an
admissible shortcut. No companion is represented as implemented.

## Enforced product behavior

New MCP parents default `session_search` to management. Missing authentication
returns `caller_identity_required`, `sessions:[]`, `result_count:0` before source
access. Empty/UUID/metadata/content/Boolean/negative/date/project/archive requests
share that gate. There is no automatic diagnostic fallback. Developer browsing
requires explicit `purpose:"diagnostic"` and is labeled as having no caller
exclusion guarantee. Diagnostic references cannot be upgraded by editing a token.

Trusted typed fixture composition exercises provider/root-scoped exclusion before
body reads, matching, ranking, counting, pagination and context. Shared-reader,
switch/reconnect, stale-reference, missing/mismatched and forged-input tests are
fixtures, not real management self-exclusion acceptance. Launch-protected
identities remain excluded in diagnostics and preview/preparation. No lifecycle
admission, approval authority or destructive operation was added.

SR3 conversational MCP management refusal passed on the restarted connection
for default search and an exact target. A cursor and an evidence reference from
the earlier instance were refused after reconnect. Real authenticated caller
identification and authenticated two-chat routing remain OPEN. See the
[SR3 live acceptance receipt](milestones/p0-sr3-live-mcp-acceptance.md).

Sources: [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp),
[App Server and dynamic tools](https://learn.chatgpt.com/docs/app-server),
[MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization),
[MCP session security](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices),
[official call metadata](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/mcp_tool_call.rs),
[direct routing](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/app-server/src/request_processors/mcp_processor.rs),
[connection sharing](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/codex-mcp/src/connection_pool.rs).
