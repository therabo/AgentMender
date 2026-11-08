# Independent local approval

## Read-only real-session deployment

`mcp --session-search` has no approval key, transaction state root, cleanup port or
preparation capability. It returns a blocked compatibility assessment for preview;
other removal calls refuse. No caller UUID is required for browsing, and none is
inferred. Readability never satisfies management protection or independent human
approval. The legacy destructive mode retains all exact-profile requirements.
Same-user shell/PTY/key access still prevents enforced human independence; do not
approve a real session here until the declared separation is actually established.


## P0-C1 current handoff

Opt-in MCP can prepare the existing durable transaction and read status. It cannot
issue/revoke/consume a grant or execute. The human independently reviews the exact
original plan/digest and runs approval AND execution in the external terminal.
Frozen management protections are restored from that original plan. Same-user
shell/PTY/key access remains outside the authentication guarantee; fixture grants
are not human acceptance. See [the current workflow](conversational-codex-removal.md)
and [ADR-0023](adr/0023-conversational-primary-removal-handoff.md). Older statements
below about eight-tool MCP refer to the default launch or historical milestones.


[ADR-0022](adr/0022-product-first-session-lifecycle.md) reuses this independent
human channel for the future conversational workflow. Default MCP cannot prepare transactions; opt-in MCP prepares protected attempts
but cannot request/issue human authorization. A future review request may only
initiate the trusted human interaction; the model must never grant authorization.
Human-readable scope, durable lookup and client shell/PTY threat assumptions need
explicit P0 integration. Additional artifact effects require their own frozen
review, not broader authority on a successful primary removal.

M8.2.1 is unfinished/deferred; builds 24141/23880 remain unadmitted candidates.
It no longer gates every primary workflow. The existing complete-Desktop
structural constraints and global-write refusal remain in force.

## Shared human authority and deferred expanded scope

[ADR-0019](adr/0019-structural-removal-and-profile-acceptance.md) adds separately
versioned expanded primary review. Fresh schema-3 plans bind exact attachment and
guardian rows, ordered primary actions, retained commitments, writer requirements
and unverified persistent reconstruction. Existing terminal challenge, short-lived
MAC grant, recovery binding and consumed DurableIntent remain mandatory.

Old v2 grants cannot authorize schema-3 effects. Expanded-primary preparation uses its own
profile; inspection, approval, execution and reconciliation of an existing plan
select its original version. No fallback or silent upgrade occurs. Neither grant
authorizes index/history/global/backup/catalog mutation or application control.

AgentMender verifies exact persistent removal and collateral preservation.
The user checks graphical disappearance separately. Automated restart is not a
development gate under [ADR-0021](adr/0021-structural-admission-and-manual-desktop-acceptance.md).
All relevant writers must remain closed throughout a Desktop cleanup. A
process-absence snapshot cannot fence a later launch.

No complete Desktop profile is admitted. Review discloses the selected profile and unknown maximum-safe closure. Coverage, a pure assessment and static source evidence
never grant authority. M8.2.1 remains historically OPEN and is now deferred.

## M8.2 scoped cleanup approval

[Scoped session cleanup](scoped-session-cleanup.md) is implemented under accepted
ADR-0017. The dedicated exact-profile CLI path reuses the existing human
channel, authenticated history, expiry, exact binding, one-shot consumption and fresh
validation below remain mandatory. A scoped grant binds the complete immutable
required/optional/retained manifest and disclose irreversible primary payload
removal; recovery metadata does not restore that payload. Approval cannot cure
primary identity, sharing, root, schema, physical identity or writer-fence uncertainty.
No MCP boolean, imported manifest or direct command may issue authorization.

M7 produces authorization evidence and performs no provider effect. Native v1 real
and sanitized provider profiles retain their complete effects/integrity blockers.
The scoped path uses fresh dedicated local evidence; unrelated M3 Unknown does not
automatically prevent approval. Exact primary, boundary and profile uncertainty does. MCP retains its eight read-only tools and cannot prepare, grant, revoke,
consume, execute or access the key.

M8 adds the separate [approved Codex execution workflow](codex-lifecycle.md).
Grant issuance and this human-channel boundary are unchanged. Dispatch consumes
the gate by value; only an observation handle returns. No current real partial plan
is promoted to eligibility.

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
and provider-contract checks remain mandatory for every admitted operation.

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
consume-then-intent gap or standalone consume command. The later CLI
`transactions execute` invokes application orchestration and cannot mint a grant;
MCP still exposes neither consumption nor execution.

External append/sync and SQLite commit precede the noncloneable DurableIntent,
which identifies approval, transaction, sequence, plan/state and operation. M7
dispatches nothing. Future M8/M9 must use it only for the matching native operation,
then explicitly observe and independently verify; a receipt is not provider success.
M8 implements that handoff for conditional exact-profile Codex unarchive only.

Ambiguous writes poison the instance. Torn/divergent histories stop authority on
reopen without truncation, repair, regrant or retry. Committed consumption persists
after restart; missing outcomes stay unknown. ExternalPrefix is unauthenticated
diagnostic evidence and cannot produce an authorization snapshot or receipt.
See [ADR-0012](adr/0012-independent-local-approval.md) for rationale and validation.
