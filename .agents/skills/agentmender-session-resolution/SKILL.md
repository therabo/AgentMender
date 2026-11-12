---
name: agentmender-session-resolution
description: Find and verify past Codex conversations using the autonomous AgentMender MCP search.
---

Follow the [resolution workflow](../../../docs/intent-aware-resolution.md).
Pass the user's original natural-language request as `query` to
`session_find`. AgentMender starts and verifies the Planner and Evaluator,
drives the complete Rust scan, and publishes signed results.

Use the returned `job_id` with `session_find_result` and `wait_seconds:60`
until terminal. Use `session_find_status` for immediate progress. Follow `results.next_offset`
to collect all verified rows. Render confirmed and possible unconfirmed
sessions in two localized tables with original titles and UUIDs. Keep the
signed evidence references for navigation. Report any interruption and the
reported source coverage. Use `session_discover` for deterministic inventory.
