# Claude evidence boundary

The official Agent SDK documents structured local history helpers in Python and
TypeScript. The Rust adapter does not ship or execute either runtime. CLI
`--output-format json` runs a conversation; it is not a past-session inventory
endpoint. `claude agents --json` describes background sessions and does not
replace full transcript inventory. `claude rm` retains transcripts. No lifecycle
operation is exposed by this crate.

Live discovery is **AuditOnlyUnknown**, including a detected CLI version of
2.1.311. It enumerates regular-file metadata and never parses transcript or
sidecar JSON. Exact-release real capture validation remains an M0 release gate.
There is no installed Claude executable or real capture used for these fixtures.

The `2.1.311-synthetic` fixture tests only our narrow parser contract. Its manifest
explicitly says synthetic, and its profile is reachable only via `from_fixture`.
This is not evidence that Claude 2.1.311 generated these bytes. Current official
SDK source corroborates the broad metadata and sidecar concepts; its mutable
main branch is not an immutable CLI 2.1.311 layout specification. Promote a live
profile only after isolated real-provider captures, provenance and release tests.

The fixture parser requires an exact version, sessionId and (for subagents)
agentId matching the locator. It reads at most a 256 KiB prefix and examines at
most 128 lines of at most 64 KiB each; incomplete coverage is visible. Changed
cwd, malformed records or conflicting identities prevent interpretation of that
file. This deliberately undercounts large or irregular transcripts. No prompt,
title, message content, tool result or parser error excerpt enters output.

The project bucket is opaque. A project path comes only from explicit absolute
`cwd` metadata. This matters because path encoding is lossy and
`CLAUDE_CODE_PROJECT_DIR_NAME` permits custom buckets. A `parentUuid` links
messages, not sessions. `sessionId` groups subagents with a root transcript;
only an explicit `parentAgentId` resolving within that group creates a nested
SpawnedBy edge. A missing sidecar never implies an immediate parent. The adapter
does not infer lifecycle ownership; every observed artifact stays protected.
Nested workflow transcript directories remain unknown artifacts in this profile.

All reads use shared bounded/scoped helpers; links are skipped, and unsafe,
unreadable, changed or unknown state remains visible through diagnostics.
Filesystem observation is not an atomic snapshot or deletion precondition.

Official sources checked 2026-10-28:

- [SDK Python history reference](https://code.claude.com/docs/en/agent-sdk/python#list_sessions)
- [SDK TypeScript reference](https://code.claude.com/docs/en/agent-sdk/typescript)
- [Internal transcript warning and custom project buckets](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)
- [Current SDK implementation, not a pinned CLI contract](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/sessions.py)
- [CLI reference and background session semantics](https://code.claude.com/docs/en/cli-reference)
