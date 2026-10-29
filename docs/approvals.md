# Independent local approval

M7 produces authorization evidence and performs no provider effect. Current real
and sanitized provider profiles lack complete effects/integrity contracts, remain
Blocked/Unsupported and cannot obtain grants. Positive tests use synthetic closed
contracts. MCP retains its eight read-only tools and cannot prepare, grant, revoke,
consume, execute or access the key.

## Human channel and commands

A separately invoked local CLI requires terminal input/output, a complete review
and an unpredictable challenge bound to the full transaction and plan digest.
There is no imported plan/proof/grant JSON, --yes/force/confirm, noninteractive grant,
caller-selected principal or clock. This proves a current-user process completed a
terminal interaction; it does not authenticate Daniel personally or resist arbitrary
same-user shell/PTY, secret-key or whole-store access. OS wall time is trusted;
arbitrary clock rollback and complete-store rollback are outside this guarantee.

```sh
agentmender --provider codex plans prepare archive <observed-id> --state-root /absolute/agentmender-state
agentmender plans inspect <plan-digest> --transaction-id <transaction-id> --state-root /absolute/agentmender-state
agentmender --provider codex approve <plan-digest> --transaction-id <transaction-id> --state-root /absolute/agentmender-state
agentmender --json approvals list --state-root /absolute/agentmender-state
agentmender --json approvals inspect <transaction-id> --state-root /absolute/agentmender-state
agentmender --json approvals history <transaction-id> --state-root /absolute/agentmender-state
agentmender approvals revoke <approval-id> --transaction-id <transaction-id> --state-root /absolute/agentmender-state
```

Preparation uses fresh application planning, never imported proofs. Blocked
preparation shows the plan/blockers without creating state. Use the same trusted
global provider/root/executable configuration when approving. Persisted metadata
does not choose executable paths; mismatched launch configuration fails closed.
Preparation and grant/revoke reject JSON mode. Inspection/history are read-only.

Reviews include exact provider/profile, target, intent/native operation/effects,
PlanDigest/StateFingerprint, transaction/status, all affected/retained ownership
classes, pre/postconditions, recovery digest/limitations, space and blockers.
Output escapes terminal controls/bidi, is formatted and capped at 512 KiB; oversize
fails without truncation. Fresh eligibility/blockers are checked before the
challenge, then repeated under the reacquired writer lock and reviewed-sequence CAS.

## Scope and lifecycle

ApprovalId and ApprovalNonce are independent random 256-bit identifiers.
ApprovalGrant binds ApprovalScope: TransactionId, complete TransactionBinding,
RecoveryDigest, operation index zero and ForwardOnly direction. The binding retains
PlanDigest, StateFingerprint, provider/intent/native operation and commitments to
target, native identity, profile, capability/effects, conditions and operation.
Principal is LocalInteractiveUser. Lifetime is at most 300 seconds, with checked
arithmetic and `issued_at <= now < expires_at`.

ApprovalState records None, Granted, Revoked or Consumed. ApprovalStatus also
computes Expired/Invalid from current time. One grant ever per transaction is
allowed. Revoked/Consumed cannot revive; expiry/revocation needs a fresh attempt.

`approval_eligible` centralizes v2/Prepared, Ready original plan, exact binding,
fresh M4 Current inputs, required recovery and no prior grant. Persistence also
verifies durable recovery bytes. `approval_valid` checks exact recorded grant,
lifecycle, scope and time, but does not authenticate a MAC or reobserve providers.
`scope_and_time_valid` is only a component check. All require authenticated history.
Approval never overrides blockers, stale/Unknown/Partial/conflicting evidence,
missing recovery, terminal states or OutcomeUnknown. Execution-time revalidation
and provider-contract checks remain mandatory for future M8/M9.

## Durable authority

There is one SQLite/journal authority. V2 Created retains the exact bounded
LifecyclePlan for review alongside its binding. This extends M6's commitment-only
metadata with necessary local IDs, evidence locators and rule metadata. No graph,
provider snapshot, prompt/reply/transcript/source body, credential or cookie is
stored. Existing private owned-root, permissions, identity and history bounds apply.

TransactionEvent::Authorization contains ApprovalAction and
AuthorizationAuthentication. Granted, Revoked and Consumed use full HMAC-SHA256
tags over an ordered versioned message: key ID, transaction, sequence, binding
digest, previous record digest and every unsigned action field. Ordinary record
hashing includes the tag. Authoritative open/list/inspect/write verifies every tag
with Mac::verify_slice, pure replay and full external/SQLite agreement.

The dedicated random 32-byte `authority.key` is created without replacement,
synced with its directory and anchored by a separated key ID in immutable schema2
metadata. A complete pre-authority orphan is re-synced before adoption. Existing
authority with a missing, partial or substituted key fails without regeneration.
No rotation interface or M11 encryption is introduced. MCP's ephemeral key is separate.

Explicit migration validates exact schema1, integrity and both histories under
the exclusive lock, then commits metadata and schema2 together with BEGIN IMMEDIATE:

```sh
agentmender approvals migrate --state-root /absolute/existing-m6-state
```

Original v1 row text, sequences, digests and external bytes remain unchanged.
Readers never migrate. V1 attempts remain inspection-only; prepare a new v2 attempt
from trusted fresh inputs. Unknown/partial schemas and malformed keys fail closed.

Existing unsafe SQLite sidecars also fail closed under ADR-0011. In particular,
a Windows crash may leave a rollback journal owned by a different SID. Opening
or migrating that store preserves the evidence and returns PermissionDenied;
it does not repair ownership or claim migration succeeded. Crash tests verify
this protected outcome separately from successful atomic migration and preserve
the external journal, file bytes, sidecar identity and security descriptor.

## One-shot handoff

`TransactionStore::record_intent` now requires an authenticated exact active grant,
fresh plan validation and verified recovery. One signed Consumed action both
consumes permission and moves Prepared to OutcomeUnknown. There is no standalone
consume-then-intent gap and no consume/execute CLI or MCP command.

External append/sync and SQLite commit precede the noncloneable DurableIntent,
which identifies approval, transaction, sequence, plan/state and operation. M7
dispatches nothing. Future M8/M9 must use it only for the matching native operation,
then explicitly observe and independently verify; a receipt is not provider success.

Ambiguous writes poison the instance. Torn/divergent histories stop authority on
reopen without truncation, repair, regrant or retry. Committed consumption persists
after restart; missing outcomes stay unknown. ExternalPrefix is unauthenticated
diagnostic evidence and cannot produce an authorization snapshot or receipt.
See [ADR-0012](adr/0012-independent-local-approval.md) for rationale and validation.
