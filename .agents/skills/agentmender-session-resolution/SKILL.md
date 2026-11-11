---
name: agentmender-session-resolution
description: Find the past Codex conversation that investigated, decided, implemented, tested, or resolved something. Use for natural-language past-conversation lookup and comparison through AgentMender.
---

Follow the [ordinary search workflow](../../../docs/intent-aware-resolution.md#originating-chat-workflow).
Use `session_discover` with metadata only for exact titles, IDs and inventory.
For activity or outcomes, use `session_resolve` from the originating chat.
This skill authorizes one `session_relevance` delegate, or GPT-6 Luna Medium
if the named role is unavailable. A Luna Medium origin can work directly.

Start once with SearchIntent v1. Assess the initial evidence and finish a
useful standard result on that run. Report verified matches from
`finish.matches` and disclose coverage. Continue an explicit all-results
request within the run's bounds.
