# AgentMender — Implicit Caller Exclusion Acceptance

November 10, 2026. Starting commit `48a835510eaa47a782b5f0bbadca205facb440e2`.

Codex Desktop 26.1103.60933 (build 25894), CLI 0.167.0-alpha.3.2, loaded
`session_discover` in the existing chat after a user-operated AgentMender
connection restart. The running executable matched the inode and SHA-256
`25505492d5602dc55000248a7a7b884dda9cd23f811dd4d44671f14a231ccc8a`
at verification. Codex app thread inspection identified the current thread
independently from MCP. Discovery arguments contained filters only. The
exact UUID was used as a **target filter** solely in the acceptance probes.

| Scenario | Actual result |
|---|---|
| A. Ordinary AgentMender search | Seven other matching conversations in four pages. The calling chat was absent. |
| B. Strong self-match | The explicit diagnostic comparison found the calling chat by its exact title. Discovery excluded the same identity for the title and the SR4 phrase, with zero eligible sessions and zero transcript bytes read. |
| C. Exact caller UUID | Discovery returned zero sessions, zero unresolved candidates, zero eligible rows, no cursor and zero transcript bytes read. |
| D. Unrelated MISSION1 conversation | Discovery returned the nominated ISS rendering chat and signed evidence. Context expanded to two ordered passages, including the Cesium discussion. Substituting the caller's target refused with `stale_cursor`. |
| E. Possible MISSION1 deletion candidates | The supplied acceptance request was interpreted in the actual chat as read-only `session_discover`. It returned 91 candidates in five pages under the observed MISSION1 label, with `discovery_only` and `can_request_preview:false`. No lifecycle tool was called. This was the supplied scenario in the implementation turn. An unprimed standalone user-turn test was not run. |

All discovery outputs indicated metadata-derived exclusion and no authenticated
binding. Management search still returned `caller_identity_required` and zero
candidates from this same chat. The supported mapping treats `threadId` as
the exact persisted Codex thread and checks a supplied `sessionId` for equality.
A successful live call establishes an admitted shape, either thread-only or
matching identifiers. The response does not disclose which fields were present
or their raw values. See the [identity contract](../mcp-caller-identity.md).

The 147,957,003-byte compressed source reached physical EOF in 19 advancing
discovery calls in about 6.1 seconds of aggregate MCP call time. It accounted for
206,933,372 decoded source bytes and remained logically Partial. A cursor
replayed with a changed filter was refused. Fixtures cover changed source,
root and provider, missing or malformed metadata, an unknown UUID, discordant
thread/session IDs, a forged apparent caller, a forged reference, archived
callers, exact matches, Boolean filters, strongest-match exclusion, empty
query, pagination, shared process routing, caller switching, reconnect and
management refusal. Disposable test sources alone were edited for stale-source
testing. Real calls were read-only.

## Manual two-chat acceptance

The user selected the current-chat tests. Two real chats sharing a process and
an unprimed standalone UX turn remain open. The shared-process fixture
alternates callers on the same STDIO process. To test the Desktop route:

1. Open two ordinary local Codex chats, A and B, with the updated AgentMender
   catalog. Independently identify each chat's UUID through Codex thread
   inspection. Confirm both exist in the configured root.
2. Give both chats a common benign phrase. In A, send only “Find the
   conversations about that phrase that I could delete.” Inspect the tool call
   for `session_discover`. Verify that A is absent and that a matching B can
   appear. Repeat from B with the same phrase and verify the inverse.
3. In each chat filter by its own UUID with content enabled and archived
   results included. Verify zero results, no evidence and zero transcript
   reads. Attempt context with the other chat's signed evidence and target.
   Verify refusal. Keep receipts and identifiers local.
4. Use the same paged query in both chats. Try A's discovery cursor in B
   without changing its filters. Expect stale or invalid cursor. Return to A
   and verify its own continuation still works if source and scope are fresh.
   Compare `registration_instance` to distinguish one shared process from two.
5. Restart only the AgentMender connection and verify that an earlier cursor
   and evidence reference fail while a fresh discovery succeeds. Stop at
   read-only candidates.

Private MCP receipts and exact final delivery verification remain outside Git.

The final local catalog gives discovery its own refusal schema. This fixed
the shared registry's size regression without changing discovery result
semantics. The final workspace build SHA-256 is
`8d5c56f318a25c54c6d955d05ac299b3234d562cfd004f1d0ca0b07f1a0c5691`.
Local Rust and compatibility gates passed. The Rust log reports 565 passing
tests including nested harnesses, zero failures and eight ignored opt-ins.
