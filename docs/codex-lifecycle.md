# Codex native lifecycle execution

## M8.2.1 graphical removal boundary

M8.2 primary cleanup is PASS and [integrated](milestones/m8-2-integration.md).
[ADR-0018](adr/0018-complete-session-removal.md) reserves complete Session Removed
for verified disappearance from every supported projection after reload/restart,
plus maximum safe artifact cleanup and collateral preservation. No complete
Desktop mutation profile is admitted; the current legacy product assessment is
ProjectionIncomplete. Historical native/scoped pins and v2 grants remain unchanged.

Installed Desktop 26.1027.51404/build23880 embeds 0.166.0-alpha.5.1, whereas the
scoped adapter admits stable 0.166.0. Static source reveals independent catalogs,
global backup and renderer state; native thread/deleted invalidation is not a
proven external target-only cleanup API. Native descendant deletion remains
unsupported. Exact composite GUI coordination and isolated restart acceptance
are pending; no real application is stopped or restarted by this foundation.

## Current scope and separate M8.2 contract

M8.1 was integrated at `6f798bcebdfd2f763e31d62ef2695c52396ca709`
([receipt](milestones/m8-1-integration.md), recorded in `c9dd04d`). Its full native
cleanup remains BLOCKED. M8.2 is implemented on its scoped-cleanup branch;
[ADR-0017](adr/0017-scoped-cleanup-residual-preservation-provider-local-mutation.md) is **Accepted**. The separate scoped-local profile is implemented under final release review.

The historical exact unarchive contract and command path below remain unchanged.
The separately reviewed local one-session cleanup reuses durable intent and
approval while preserving children, shared/unknown state and unrelated data.
It must select its strategy before approval; never use local mutation as a fallback
after ambiguous native deletion. Its initial plain-legacy profile, dependency proof
and strong writer/SQLite-WAL fence are still review gates. See
[scoped cleanup architecture](scoped-session-cleanup.md).

M8.1 separately investigated stable `0.166.0`; its archive/delete and cascading
unarchive remain Unsupported. No historical exact profile was changed. See the
[current cleanup report](codex-full-cleanup.md) and ADR-0016.

M8 adds a typed, independently approved execution path. Within this historical native adapter only exact-profile native
unarchive is conditionally supported. Native v1 real plans remain Blocked/Unsupported:
their partial discovery, unclosed effects and unknown integrity cannot gain approval.
This adapter does not make current user sessions executable.

| Operation | Dispatch | Reason / effect boundary |
|---|---|---|
| Archive | Unsupported | Native cascade includes hidden descendants omitted from complete-looking listings |
| Unarchive | Conditional | Exact target restore; descendants remain retained; provider-managed backing changes are subject to independent verification |
| Delete | Unsupported | Unclosed cascade, no exclusive ownership or authoritative negative proof |

The exact profile binds Codex `0.165.0-alpha.1.2`, its observed macOS executable
digest and three signing-file digests, V165 interface/source schema, captured native
schemas, adapter `0.1.0`, and
`codex-unarchive-target-only-v165-sha256-d84a54b0-v1`. Versions, binaries, resources,
layouts, operations or contracts outside that profile fail closed. ReadOnlyKnown
discovery is never sufficient execution compatibility. See the
[sanitized actual-binary evidence](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/README.md)
and [ADR-0013](adr/0013-codex-durable-native-handoff.md).

## Local commands

Use the same trusted Codex root/executable configuration used during preparation
and independent terminal approval. Commands accept an existing durable transaction,
never a native thread ID, imported plan, raw method or JSON payload:

```sh
agentmender --provider codex --codex-root /absolute/codex-home --codex-executable /absolute/codex transactions --state-root /absolute/agentmender-state execute <transaction-id>
agentmender --provider codex --codex-root /absolute/codex-home --codex-executable /absolute/codex transactions --state-root /absolute/agentmender-state reconcile <transaction-id>
```

Execute requires the exact original Ready plan, v2 Prepared transaction, active
authenticated grant and verified recovery. It obtains immediate fresh trusted
planning inputs, validates Current and atomically consumes approval with intent.
`DurableIntent` exists only after external journal sync and SQLite commit.
Dispatch consumes it by value; only an observation handle returns. Missing,
revoked, expired, consumed, mismatched or stale authority cannot dispatch.
Fixture and Claude configurations are rejected. MCP keeps its eight read-only tools.

The private launcher copies only the exact executable and three bounded signing
files into owned scratch, verifies their digests and launches that private image.
An OS sandbox restricts writes to the approved provider root and scratch, denies
network/forks/Mach lookup and prevents image modification. Provider startup itself
is an effect and follows durable intent. No provider SQL write, rollout editing,
filesystem lifecycle fallback, retry or compensation exists.

## Observation and failure knowledge

The adapter returns a typed native result. It persists no raw native response,
preview, prompt, reply, transcript, credential or error prose. The existing private
plan/history/recovery privacy and same-user threat-model limits still apply.

| Boundary | Durable interpretation |
|---|---|
| Before durable intent | No dispatch gate; no provider effect |
| After durable intent, before startup | OutcomeUnknown after crash; an observed proved pre-start failure can record NoEffect/Failed |
| Startup, initialization, request or response failure | OutcomeUnknown; startup may already have written provider-managed state |
| Valid response before durable observation | OutcomeUnknown after crash |
| Durable accepted response | EffectObserved; independent verification still required |
| Before/during verification | Fresh observation required; no automatic success or repeated effect |
| Complete same-run postcondition and retained-invariant proof | Succeeded |
| Unknown/failed post-verification | RecoveryRequired |

Fresh read-only observation uses the existing kernel-attested scrubbed mirror.
Positive native active-list membership for the exact target can establish logical
restored presentation. Missing entries, runtime status, notifications, filesystem
names and generic native errors do not prove absence or no effect. Direct native
read needs source rollout contents under this build and is not added to this
content-denied observer. No transcript header is copied to work around that limit.

One capture supplies native state, graph and integrity context. The application
recomputes integrity and verifies exact identity/profile, complete coherent evidence,
healthy integrity, no conflicts/blockers/findings and retained logical state against
the immediate pre-intent graph. Only the target state and corresponding retention
root may transition. Other identities, metadata, paths, evidence and relationships
remain bound using lossless fact comparison. A successful RPC is not success.

Reconcile authenticates consumed history and recovery, then observes only. It never
recreates DurableIntent or reissues an operation. Positive restore may record Applied;
a still-archived or unobserved target remains Unknown. After restart the old full
retained baseline is unavailable, so verification cannot claim success and remains
Unknown/RecoveryRequired. No baseline is invented from the fingerprint or poststate.
Recovery manifests are reconciliation metadata, not backups. Every operation is
NotCompensatable; unarchive is not the inverse of cascading archive.

## Tests and limits

Deterministic tests use synthetic complete contracts and fake ports/transports.
They require no installed provider. Actual-binary evidence and the ignored explicit
native test use only generated disposable provider roots. Neither promotes current
real partial observations into readiness. The observed native binary is macOS-only;
other production platforms remain Unsupported until independently validated.

The real-binary test is explicitly opt-in and accepts no provider-state path:

```sh
AGENTMENDER_ALLOW_ISOLATED_NATIVE=1 AGENTMENDER_CODEX_BINARY=/absolute/exact/codex cargo test --locked -p agentmender-provider-codex isolated_durable_unarchive_restores_only_target -- --ignored
```

It prepares a complete explicitly synthetic proof fixture, obtains a real signed
consumed intent and calls the production adapter. It verifies target restoration
and retained-child state. It does not claim that current production discovery can
make a real plan Ready or that the full application succeeds on partial evidence.

M8 does not eliminate concurrent provider changes or all TOCTOU. Owned private
image pinning and immediate validation minimize the boundary; arbitrary same-user
control of AgentMender state/memory, system-library replacement, device durability
and later M10/M11 hardening retain their documented limits.
