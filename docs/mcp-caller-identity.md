# MCP caller identity: management admission remains OPEN

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
JSON `meta` object. This proves an interface exists, not an authenticated inbound
claim to an arbitrary local MCP server. Upstream source is corroborating evidence,
not proof that every installed routing path has the same implementation.

MCP authorization does not specify a provider-thread attestation for local STDIO.
The observed `_meta` values can be forged by another client running AgentMender;
the regression sends forged `threadId`/`sessionId` values and still receives zero
management candidates. AgentMender does not authenticate the STDIO peer or its
provider root. Merely finding a UUID in request metadata therefore cannot create
`ManagementIdentity`. No self-issued token or privileged assertion file is admitted.
A diagnostic in the new refusal reports only presence of thread metadata, never
its content or an authentication claim.

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

Actual conversational MCP management refusal passes. Real authenticated caller
identification/self-exclusion and simultaneous actual-host routing acceptance
remain OPEN. The user could not refresh this chat's retained MCP parent; its old
default must not be described as the new default. Explicit management still refuses.

Sources: [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp),
[App Server and dynamic tools](https://learn.chatgpt.com/docs/app-server),
[MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization),
[MCP session security](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices),
[official call metadata](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/mcp_tool_call.rs),
[direct routing](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/app-server/src/request_processors/mcp_processor.rs),
[connection sharing](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/codex-mcp/src/connection_pool.rs).
