# Luna session relevance

Luna Medium is required for AgentMender conversational search. It interprets
one request into one SearchIntent, judges each candidate in bounded batches
and compares already verified sessions if the user requested a single best
result. Use the [current workflow](intent-aware-resolution.md) and the
[project agent](../.codex/agents/session-relevance.toml).

The origin verifies real execution before sending a plan or judgments. Native
Codex hosts expose resolved model and effort on supported subagent start
events. The project role and instructions alone do not prove a successful
spawn. A confirmed unavailable or unobservable Luna run stops. Rust verifies
scope and evidence and never supplies a semantic verdict in place of Luna.
The controlled [app-server host](../scripts/resolve_with_luna.py) checks
effective model, effort, reroute and turn completion itself.

AgentMender reads locally. The configured model provider can process the
bounded excerpts shared with Luna. Pass only the intent, identity and material
passages. Keep callable evidence references in the origin. The retrieval
system has no lifecycle authority.

The [historical assessment schema](session-relevance-output.schema.json)
and [offline checker](../scripts/check_semantic_assessment.py) remain for
reproducing historical research receipts. They are not a resolver action.
The current wire is the typed `session_resolve` catalog.
