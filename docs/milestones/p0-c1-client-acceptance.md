# P0-C1 final integration and real-client acceptance review

Date: 2026-11-08, Europe/Rome. Milestone verdict: **OPEN**.
Continues the [implementation report](p0-c1-conversational-codex-removal.md).
The [manual acceptance guide](../p0-c1-manual-acceptance.md) is the user handoff.

## Baseline and delivery

Clean local and remote P0-C1 HEAD initially matched
`44df8414ade5fe3a33d53a02a66e4295d90bef20`. Authenticated Safari reverified
[CI 42391059391](https://github.com/therabo/AgentMender/actions/runs/42391059391),
its full exact commit link and three completed successful matrix jobs. Main
remains `a284767157ace8769deee85b195b5b70efe374d8`; deferred M8.2.1 remains
`14a8931f38949d3dbde04a61b22091752a685402`. No merge/history rewrite.
The final delivery SHA and its distinct three-platform CI are recorded after
commit in the sibling internal verification receipt; baseline CI is not final CI.

This change adds a disposable search-fixture helper, strengthens existing
end-to-end assertions and corrects deployment instructions. It changes no runtime
authority, provider profile, schema, planner, transaction, grant or effect path.
The eight-crate design and ADR-0022/0023 remain. P0-C2 has not started.

## Separate completion levels

| Level | Result | Evidence / remaining gate |
|---|---|---|
| Implementation complete | PASS locally | Full locked gates, exact-profile worker and stdio/provider tests; final exact CI recorded separately |
| Actual client catalog/startup | PASS, bounded test | Installed alpha.6 initialized MCP and fetched all 13 schemas via a test-only stdio bridge |
| MCP client accepted | OPEN | Natural-language request stopped at 401 before any model tool invocation; actual Desktop conversation not exercised |
| Management protection | PASS for registered fixtures; OPEN for actual caller | UserProtected survives planning/CLI effects; no authenticated caller binding or independently verified real management registration |
| Human approval accepted | OPEN | Fixture grants only; unrestricted same-user agent can access key/PTY/approval APIs |
| Deletion accepted | OPEN | Disposable protocol/provider deletion passes; no actual client + independent-human eligible deletion accepted |
| Graphical confirmation | Not requested / unverified | No application quit, restart or GUI removal check |

## Actual client probes

The current installed executable reports **0.166.0-alpha.6**; Desktop is
26.1029.61003/build **24614**. The prior alpha.5.1 / 24141 observation is historical.
Neither build inherits stable cleanup authority.
The actual client configuration was inspected with OS write/network denial:
four servers were registered, **none for AgentMender**. The live configuration
was not edited; the guide supplies the missing registration step.

The first read-only `codex exec --ephemeral --ignore-user-config` probe retained
the ordinary auth home for client-owned credential access, redirected SQLite/logs
to scratch and kernel-denied all writes outside scratch. It failed in-process
App Server initialization with `Operation not permitted`, before MCP discovery.
No unsandboxed retry or real-state modification was used.

A second probe used an entirely isolated client home and no copied credentials.
The actual alpha.6 client negotiated MCP, initialized AgentMender and issued
`tools/list`; the response contained the original eight tools and all five opt-in
tools with input/output schemas. Its English MISSION1 request then failed with
**401 Unauthorized: Missing bearer or basic authentication**, before
`session_search`. No metadata presentation or model answer is claimed. This is
runtime catalog discovery, not a successful natural-language interaction.

A test-only stdio bridge connected the sandboxed client to separately owned
AgentMender, preserving the provider worker's own sandbox: a nested
`sandbox-exec` canary had failed with `sandbox_apply: Operation not permitted`.
The bridge allowed only search/preview calls; it could not prepare or execute.
It is not a shipped transport or an accepted direct Desktop launcher. Both
probes left provider file hashes unchanged and created no transaction store.

A safely isolated **stable** App Server `thread/start` also refused:
`failed to load configuration: Failed to synchronize managed preferences`.
No thread row was created. This describes the tested isolation, not an intrinsic
inability of stable Codex to create sessions. No genuine eligible same-root
management/target pair was established safely. The manual guide stops before
real destruction; it requires a supported operator-created case rather than
changing producer/history/version gates to manufacture eligibility.

## Compatibility, protection and authority

`session_search` uses the exact stable executable/schema protected metadata
reader. It returns title/project-basename/UUID, timestamps, archive state,
producer-match and protection separately. An alpha producer in a matching schema
can be shown without becoming eligible. A mismatched binary/schema refuses the
reader itself. Historical App Server discovery profiles remain separate and
partial; unknown versions gain no execution.

Deletion remains only **0.166.0**, Darwin arm64, binary SHA-256
`39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`,
schema `1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`,
Legacy/WAL/root-local, `scoped-cleanup-v1`, schema-2 plan, unshared plain rollout
and all existing configuration/dependency/physical/writer fences. Native delete
stays unadmitted. Schema-4 admission remains empty; the global lost-update
regression is unchanged. No authority transfers to the embedded alpha binary.

Trusted launch registration freezes exact management UUID exclusions in the
original UserProtected graph/plan. Approval/execution restore and freshly recheck
them. The operator must independently obtain the actual same-root caller UUID;
a fixture peer, title/cwd/Active label or model assertion cannot replace it.

MCP still cannot obtain a key, grant/revoke/consume approval or dispatch effects.
The existing grant binds exact plan, transaction, recovery, forward operation and
time; stale/different plans cannot borrow it. Intent consumes it durably before
effects. Unknown outcomes cannot authorize retry. This development agent has
unrestricted same-user shell/PTY/store access, so a terminal does not enforce
human independence. Manual operation is a constrained intermediate mode; a real
acceptance must demonstrate the declared model/operator separation.

## Disposable evidence and validation

The new `prepare_p0_c1_readonly_fixture.py` accepts no paths or authority input.
It creates only fresh synthetic temporary data. A direct JSON-RPC driver verified
all 13 tools, six MISSION1 matches, two duplicate titles, missing-title fallback,
archive state, protected/unsupported preview refusal and a compatible Ready
preview. Provider bytes stayed unchanged, no owned state was created and private
transcript/preview/message canaries were absent. This is not a model conversation.

The actual stdio/worker/CLI test covers search → exact target → preview → durable
prepare/retry → external handoff → **fixture** grant → fresh validation → consumed
intent → adapter effects → structural proof → reopened status → retry refusal.
It now independently reads target row absence and **all columns** of retained
management/duplicate-title peer rows, and compares **every file** of the other
root, including SQLite/WAL/SHM, after execution and rejected retry. Existing
payload checks remain. The provider conversation test also covers same-project/
cwd peers, similar UUIDs, archives, unsupported producers and changed protection
refusing dispatch.

The full locked suite retains incompatible/active-writer/shared-history/spawn
refusals, wrong-scope/expired/revoked/forged/consumed approval rejection,
stale plans, crash cuts, authenticated reopen/recovery and no unknown-effect
retry. Exact binary/worker primary, provider conversation and stdio conversation
opt-ins passed. An initial invocation omitted `AGENTMENDER_HELPER` and failed
before observation; the corrected invocation passed. A new test assertion first
failed compilation with an unsupported SQLite `u64` conversion, corrected to
`i64` before the passing run. Failed attempts remain in local logs.

All required local gates passed: fmt, locked Clippy/tests/build, architecture,
native lifecycle evidence, complete-removal evidence and whitespace checks.
The final receipt records exact counts/log hashes and final CI separately.

## User handoff and remaining blockers

The user must complete a natural-language call in an authenticated client,
independently verify/register the actual management UUID in a supported same-root
case, then review/approve/execute outside model control. Stop if no eligible real
disposable case or independent channel is available. Follow the exact launch,
review, execution and result commands in the manual guide. Primary structural
success, residuals, physical reclaim and optional GUI behavior are separate facts.
No real user session was deleted and no application stopped/restarted. Recommend
P0-C2 Claude Code parity only after P0-C1 acceptance; do not start it automatically.
