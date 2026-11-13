# ADR-0038 — Native Codex deletion admission

## Decision

The native deletion MVP accepts one selected Codex UUID and one signed evidence
reference from the existing search results. It revalidates the selected source
and exact provider/root through the existing read worker, with the discovery
caller and configured protections excluded. Search evidence carries selection
identity only. It cannot grant approval.

Dispatch stays closed for the installed Codex CLI `0.167.0-alpha.4.1` in
ChatGPT Desktop `26.1106.51428` (build `26605`), Darwin arm64, binary SHA-256
`7cb8700a47be3c81bb3c5efbf2d2adc716ce3db4806e9dc992f2925f78b2b194`.
The MCP request returns `failed` with `native_delete_not_admitted` after a valid
selection. It stops before elicitation, transaction preparation, or provider
startup for deletion. Historical scoped cleanup remains a separate mode.

## Disposable evidence

The native probes used fresh private homes and a kernel policy denying writes
outside each case and reads of the real home. Turn tests used a synthetic model
response held open on one loopback port. Isolation canaries verified denial of
outside writes, real-home reads, unrelated network connections, and process fork.

| Probe | Observation |
| --- | --- |
| `thread/delete` and `codex delete UUID --force` | Both cascaded to spawned descendants. A separate session survived with its rollout bytes intact. App Server success was followed by exact `thread/read` failures for the removed IDs, including after a fresh process. |
| `codex delete UUID` | Closed stdin refused for lack of an interactive terminal. In a disposable PTY, `n` preserved all sessions and `y` removed the target and child. Both answers returned exit zero. The prompt displayed the title and root UUID with a generic cascade warning. |
| Attached threads | A spawned child attached to its spawn parent blocked native deletion before any rollout was removed. When the child was attached to a different retained thread, both active and archived `thread/list` queries with `ancestorThreadId` and `attachedToThreadId` on the root returned empty for a root with that sole child. Both `thread/delete` and the forced CLI removed the hidden child. The unrelated attachment owner survived. |
| Turn in progress | With the model request still pending, `thread/read` on the owning server reported `active`. `thread/delete` on that server shut the turn down and removed the rollout. A separate App Server reported `notLoaded` for the same target but refused deletion with an active-writer error. An isolated `codex delete UUID --force` also failed and preserved the rollout in that case. |
| Ephemeral thread | The owning App Server refused `thread/delete` for the nonpersisted UUID. |
| Partial error | Denying the native unlink of the root produced `-32603` from App Server and exit 1 from the CLI. Spawned rollouts had already disappeared while the root survived. Exact metadata reads for removed descendants still succeeded after a restart. |

The [App Server protocol](https://learn.chatgpt.com/docs/app-server#delete-a-thread)
documents cascading deletion. In the exact upstream version, [delete builds its
scope from the spawn graph](https://github.com/openai/codex/blob/d2c0356/codex-rs/app-server/src/request_processors/thread_delete.rs),
while the [listing query joins thread rows and excludes attachments](https://github.com/openai/codex/blob/d2c0356/codex-rs/state/src/runtime/threads.rs).
The disposable attachment case proves that a successful list can understate the
effects. This blocks dispatch even for an apparently childless result.

## Admission requirements

A native route needs an enforceable scope that covers every affected UUID and
prevents deletion during a running turn. Once that route is verified, AgentMender
can request one human confirmation bound to the exact title, UUID, affected set
and permanent effect. A host prompt must establish independent human consent
before dispatch. Otherwise use the existing independent approval channel.
The installed schema advertises MCP elicitation, but independent consent and
Desktop end-to-end deletion have not been established for this path.

An admitted adapter would require a native success response plus a targeted
identity check and profile-supported postconditions for `deleted`. An error
after possible partial effects is `unknown` and bars automatic retry. A verified
pre-dispatch refusal is `failed`.

## Local validation

The stdio fixture ran from outside the repository and verified selection from a
published search row, protected-caller refusal, rejection of forged or malformed
inputs, and byte-for-byte preservation of the provider fixture. Workspace build,
Clippy, tests and the existing architecture and compatibility guards passed.
Computer Use denied access to the installed Desktop app, so the Desktop
conversation check remains open.
