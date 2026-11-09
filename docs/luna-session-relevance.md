# Luna session relevance

The project custom agent is [session_relevance](../.codex/agents/session-relevance.toml),
using `gpt-6-luna` with `model_reasoning_effort = "medium"`. The
[versioned JSON result contract](session-relevance-output.schema.json) uses
categorical relevance and confidence, never a claimed statistical probability.
Basic deterministic retrieval works without a model or this configuration.

Current [official Codex documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents)
locates project agents in `.codex/agents/` and requires `name`, `description` and
`developer_instructions`. Custom agents inherit the parent's permissions, provider
and MCP servers. Supported feature flags disable shell, apps, plugins, memories
and permission requests here. They do **not** supply an individual MCP tool
allowlist or independent sandbox. AgentMender `mcp --session-search` must enforce
the read-only boundary even if an inherited MCP catalog contains removal tools.
The model cannot approve, dispatch or acquire an approval key through this mode.

## Practical handoff

Use an existing authenticated Codex conversation opened in this repository.
Ask it to use `session_relevance` for semantic analysis only when useful. Exact
UUID, date, archive and project queries do not need delegation.

1. Interpret creation/activity explicitly, choose a timezone and preserve the
   original hard filters. Use `purpose:"management"` only with trusted binding;
   `caller_identity_required` means zero manageable candidates, not permission to
   retry diagnostically and call those results manageable.
2. Retrieve candidates through `session_search`, following signed cursors until
   the desired coverage or work budget is reached. A partial page is not a global
   semantic ranking. Consider explicit alternatives such as `frame drops`,
   `rendering`, `Cesium`, `camera updates` for “slow orbital visualization”.
3. Hand off the user intent, original hard filter JSON, **exact returned targets**,
   UUIDs, initial evidence refs, coverage and budgets. Example request:

   > Delegate these five AgentMender candidates to session_relevance (Luna Medium).
   > Determine whether each describes a resolved rendering performance problem.
   > Preserve the supplied date/project/provider conditions. Inspect later evidence
   > for rejected proposals. Use at most eight context requests and 64 KiB of
   > returned excerpt bytes. Return the version-1 JSON assessment.

4. Luna uses only exact signed references, for example:

   ```json
   {"target":{"provider":"codex","class":"session","token":"RETURNED_TARGET"},
    "evidence_ref":"RETURNED_EVIDENCE_REF",
    "window":{"before":5,"after":10,"max_messages":16,"max_bytes":8192}}
   ```

   For a distant resolution use `window:{"direction":"occurrences","query":"resolved"}`.
   Use returned `next_ref`/`previous_ref` in `evidence_ref` with the returned
   `next_window`/`previous_window` options to continue. These preserve occurrence
   queries and use `around` for a continuation inside a very long message, so its
   remaining text is not skipped. `window.message_offset:0` explicitly retrieves
   the beginning of the same validated message when a match excerpt starts later. `forward`/`backward` select message traversal. Returned individual
   messages carry fresh signed refs for citations and further expansion. A stale
   source or restarted MCP process invalidates old refs; rerun retrieval.
5. Validate the assessment against the schema and the original candidate allowlist.
   Reissue the original deterministic query with an additional exact `native_id`
   condition for the selected candidates, preserving `purpose` and original filters.
   Follow its continuation as needed. Only identities verified in this fresh result
   can be presented as matches. A semantic judgment never removes a hard condition.
6. Present supporting refs, unresolved context and coverage. Distinguish a proposed
   fix from one later rejected or demonstrated. “Insufficient” does not mean
   irrelevant. Session selection/deletion requires a separate explicit workflow.

## Availability, privacy and acceptance

The installed client observed in this implementation is Codex CLI
`0.166.0-alpha.6`, inside ChatGPT desktop `26.1029.61003`, build `24614`.
Repository agent discovery requires the client to load this project. Configuring
a model name is not proof of account availability or a successful custom-agent
spawn. Record actual execution separately from TOML/schema validation. If Luna
is unavailable, retain deterministic search; ask the user to choose an available
model or explicitly delegate with an available model while preserving the same
instructions. Never silently substitute a model or claim a reranking improvement.

AgentMender performs local reads. Evidence provided to Codex/Luna can be processed
by the configured remote model provider; it is not local-only inference. No
transcript upload or embedding service is initiated by AgentMender. Use the
suppression and output bounds, share only necessary passages, and acknowledge
that pattern suppression cannot detect every secret.

The read-only MCP catalog already visible to this development chat was exercised:
a guaranteed management request refused with `caller_identity_required` and zero
candidates. That verifies the real missing-binding failure path, **not** actual
caller identity or exclusive routing. The updated schema and custom agent need
separate post-build conversational acceptance. Production admission from operator registration files is disabled: even a valid
root-owned declaration cannot establish that a shared MCP process routes only one
conversation. The current CLI always returns no authenticated caller binding.
Explicit protected-session registrations still exclude known retained identities
in diagnostic reads, without becoming evidence of the actual caller. Typed
fixture bindings exercise future host contracts but do not admit a real client.

## Deterministic output check

Save the trusted handoff as an array of `{target,native_session_id,evidence_refs}`
and validate returned JSON before reissuing the original hard query:

```sh
python3 scripts/check_semantic_assessment.py assessment.json --candidates handoff.json
python3 scripts/test_semantic_assessment.py
```

The checker supports the exact versioned schema, rejects unknown schema keywords,
duplicate fields/candidates, forged identities/refs, extra authority fields and
budget/type violations. It supplies no source freshness or destructive authority;
fresh AgentMender query validation remains mandatory. The committed fixture is an
actual Luna Medium response to synthetic evidence, with one simulated context
request; it proves neither standalone GUI routing nor real MCP delegation.
