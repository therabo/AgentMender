# Unified progressive resolver and Luna host

November 12, 2026 · Europe/Rome

Local branch `feature/progressive-retrieval`, starting at `c9f8bc4`. The
remote advertised `main` at `8c63ef6` and no head at the feature branch name.
The [invalidation investigation](../research/search-index-invalidation-investigation.md)
is a separate diagnostic result. See the
[UUID snapshot milestone](uuid-search-snapshot.md) for the current acquisition
and continuation contract.

## MCP and responsibilities

`session_resolve` has two actions. `start` accepts one validated
`search_intent`. `advance` accepts a run, step revision, operation ID and
optional assessments, bounded context requests, lexical expansion terms,
cancellation or an explicit Luna-availability failure. The tool no longer
exposes `next`, `context`, `assess`, `expand`, `finish`, the alternate
`start.intent` or the opt-in progressive flag. The cumulative page, candidate,
assessment and run-duration ceilings of the old resolver were removed.
Standalone `session_discover` and `session_context` remain deterministic
read-only primitives.

| Component | Responsibility |
| --- | --- |
| Domain | Frozen SearchIntent, structured constraints, Boolean predicate, exact identity, candidate verification state and execution state. |
| Application | Single progressive state machine, source-bound handles, bounded batches, backpressure, monotonic continuation, replay, freshness and incremental publication. |
| Provider ports and Codex reader | Scoped inventory, original evidence and read-only source verification. A metadata-selected session can return an original passage even without a matching text term. |
| Persistence | Existing optional derived FTS index. |
| MCP server | Action schema, input validation, caller-scoped capability projection and public statuses. |
| Originating host | Confirm Luna execution, submit one plan, own the MCP run, collect signed rows and render two tables. Luna makes a final comparison for `single_best`. |

Advanced AND/OR/NOT and exact-ID resolution run through the same stream.
Known lexical aliases can be included in the first plan. One late
expansion can run after the first pass. After a nonempty first pass the
expanded scan uses a definite nonmatch on the first terms to avoid
revisiting stable identities. UNKNOWN on partial histories stays a coverage
gap. If a transcript changes between passes, the originating client
upserts publications by identity. The stream retains bounded payloads and
one replayable reply. The [current contract](../intent-aware-resolution.md)
describes completion and interruption.

## Luna controls and observed execution

The controlled host in [resolve_with_luna.py](../../scripts/resolve_with_luna.py)
launches an ephemeral Codex app-server thread with the exact model and medium
effort. It checks the effective configuration, successful turn completion and
model-reroute events before accepting semantic output. It cannot start
retrieval after a planner failure. It discards user-facing confirmed output
on an execution failure. The origin sends only candidate identifiers and
bounded, inert excerpts to Luna, and retains signed references.

In the native Codex flow the originating chat interprets directly when its
host shows Luna Medium. Otherwise it checks the named role's resolved start
event, then uses an explicit GPT-6 Luna Medium spawn if necessary. The
project instructions, skill, role and MCP catalog describe this procedure.
`AGENTS.md` and skills supply instructions. The role declares configuration.
The host events provide execution evidence. MCP receives caller metadata and
structured judgments. It does not receive an authenticated model identity,
trusted original wording or a delegation event. Consequently the controlled
host enforces the gate for its own calls, and the unmodified Desktop client
must perform its own checks. AgentMender never treats a deterministic match
as a Luna judgment. A resolver step declaring `luna_unavailable` cannot also
submit or publish judgments.

| Control | Observation |
| --- | --- |
| Direct Luna Medium on installed Codex app-server `0.167.0-alpha.4.1` | Effective model and medium effort verified. One planning turn and one two-candidate assessment turn completed. The fixture returned two source-bound matches and rendered two tables. |
| Non-Luna named-role attempt | A disposable trusted-project control emitted a start event for `/root/session_relevance` with resolved `gpt-6-sol` and `xhigh`. That event does not satisfy the Luna gate. This control did not establish that the named configuration was applied. |
| Explicit fallback | A non-Luna parent in the installed app-server emitted a start event with resolved `gpt-6-luna` and `medium` for the explicit fallback and completed its turn. A separate collaboration run returned independent direct and retrospective assessments. Its first response used unsupported enum strings and needed a schema correction. |
| Complete unavailability | A subprocess that exited before initialization yielded `luna_unavailable`, exit status 1 and empty tables. Rust also refused an unavailability step carrying assessments. |
| Reroute and rendering | Host regressions exercised model/effort mismatch, a rerouted turn, no retrieval after planner failure, invalid single-best selection and the two output groups. |

Native graphical Desktop acceptance of the new executable, including the
named-role fallback sequence inside the product, remains open. The installed
native tool registration was not changed. Official host behavior references
are [AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md),
[skills](https://learn.chatgpt.com/docs/build-skills),
[subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)
and [app-server events](https://learn.chatgpt.com/docs/app-server).

## Regression verification

The workspace suite recorded 578 passed, zero failed and eight existing
opt-ins skipped, including child harness cases. Formatting, warnings-denied
Clippy, the locked workspace build, architecture and lifecycle guards
were run. The executable fixtures verify the two-action catalog and
canonical schema, 105 unique candidates, exact IDs, structured project
constraints, Boolean queries, single-best policy, source-bound evidence,
replay, stale run and caller isolation. The late-expansion regression
checks overlap exclusion and replay across the phase change. The fixture
benchmark confirmed 105 unique matches and citations in 22 MCP calls.
Six host tests and six historical semantic-validator tests passed.

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --workspace
python3 scripts/check_architecture.py
python3 scripts/test_luna_host.py
python3 scripts/benchmark_progressive_resolution.py target/debug/agentmender --scenario exhaustive --count 105 --repeats 1
```
