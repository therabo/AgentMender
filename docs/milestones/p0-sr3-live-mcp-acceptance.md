# AgentMender live Desktop MCP acceptance

November 10, 2026. The existing SR3 commits were published by a fast-forward
push to `main` after loading the existing Ed25519 identity into the macOS SSH
agent and authenticating to GitHub. A fresh remote read confirmed
`51f345a24b70daf83193685dc35b42bb6532de51`. GitHub [run 81](https://github.com/therabo/AgentMender/actions/runs/42816992163)
reported that the Linux, macOS and Windows jobs could not start because of
account payments or the spending limit.

## Live connection and retrieval

Codex Desktop 26.1103.60933 (build 25894) runs CLI 0.167.0-alpha.3.2.
The existing STDIO registration launches AgentMender with the selected local
Codex root and `mcp --session-search`. The initial executable hash matched the
SR3 baseline `e4ec7c39f00690f2a281b6365a3690a007cf81dc3f225c49353491f262ad4f57`.
After a user-operated AgentMender restart, the same conversation returned a
different registration instance. The running process mapped to the executable
whose measured hash was
`66b09b77b64c426b5829007c562a05d07668a787479c6c00a89704c6dac21d0c`
at restart verification. A subsequent workspace build relinked the binary path.
The live catalog exposed `session_search` and `session_context`. New signed
references worked, a 5,347-byte cursor continued, and the old instance's cursor
and context reference were rejected.

| Case | Evidence from real MCP calls | Coverage |
|---|---|---|
| A, ISS rendering | Cesium comparison and FPS occurrences | Physical EOF, logically Partial |
| B, Earth camera | Camera bounds, workload and later validation | Physical EOF, logically Partial |
| C, ISS telemetry | PostgreSQL checks and forward context | Physical EOF, logically Partial |
| D, database bootstrap | The 10,440-byte final message in three ordered chunks | Physical EOF, logically Partial |
| 147,957,003 compressed bytes | 19 search calls, 12 context pages, 137 unique message ordinals | 206,933,372 decoded bytes, physical EOF, logically Partial |
| 67,502,706 compressed bytes | 10 search calls, 7 context pages, 123 unique message ordinals | 104,900,622 decoded bytes, physical EOF, logically Partial |

All six source revisions matched across the restart. Both oversized sources
reached the last supported message with advancing continuations and no duplicate
chunks. The larger source yielded fresh references near 57% and 80% of the
decoded stream. Its search and forward navigation took 5.1 and 5.3 seconds of
aggregate MCP call time after restart. Earlier actual-client traversal and replay
sampled about 139 MiB peak AgentMender process-tree RSS. These are local
measurements on the selected sources. External paginated history is unaccounted
for. The prior 330-source benchmark was not repeated.

The October 10–20 Europe/Rome search and its UTC equivalent returned the same
31 identities. The October 25 DST day matched its explicit 25-hour UTC interval
with 18 identities. Observed project NOT MISSION1 returned 266 sessions. Observed
MISSION1 OR AgentMender returned 91, all labeled MISSION1. Negating verified
membership for a known observed MISSION1 target yielded UNKNOWN. The open-ended
date/project/PostgreSQL-or-MongoDB query returned a first page and then refused
its stale cursor. The supplemental query ending November 9 completed in 31 pages
with two positive matches and 182 UNKNOWN candidates. These results are
diagnostics and express observed project labels, not verified ownership.

## Caller evidence and safeguards

The original correlation diagnostic checked the emptied RMCP call parameter
instead of request context. The regression reproduced the false presence
indicator and now checks that the bit is true for wire metadata, false when
absent, and that values are never echoed. The restarted real conversation
reports `thread_id_metadata_present: true` and
`authenticated_host_route: false`. Default and exact-target management queries
refuse with `caller_identity_required` and zero candidates.

The [caller identity contract](../mcp-caller-identity.md) records the required
host-owned attestation. The installed host exposes routing and experimental
dynamic-tool callbacks but no verified third-party route supplying authoritative
thread, provider root and per-request epoch. No management identity was admitted.
The existing fixture suite exercises trusted composition, two callers on a
shared reader, root mismatch, protected context and stale references. A separate
direct protocol probe checked refusal for different caller-supplied IDs and
references from a different instance. Actual authenticated two-chat routing,
self-exclusion and management acceptance remain open.

Local locked formatting, Clippy, tests and build passed, along with architecture,
compatibility and semantic validators. The workspace log reports 563 passes
including nested harnesses, zero failures and eight ignored opt-ins. Real
acceptance calls were read-only. Private receipts and transcript excerpts remain
outside Git.
