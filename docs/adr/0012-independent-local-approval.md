# ADR-0012: Independent local approval and atomic authorization handoff

## Status

Accepted, 2026-10-29, after verified M6 integration at
`f2268d2cdcff961a91e079af79314571fb3752d7` and three Sol/high design reviews.
Extends ADR-0009/0010/0011. M7 ends at authorization evidence; M8/M9 own effects.

## Context and threat model

The main agent completed the entire 37-page research, milestone specification,
repository documentation/ADRs, source/test/history reconstruction and live Git/CI
verification before changes. Research pp18–19 and M7 require an independent,
short-lived, single-use, authenticated grant binding the exact immutable action.
PlanDigest, StateFingerprint, TransactionId, Ready and durable intent are identity
or knowledge, never authorization. A model-controlled MCP confirmation is unsafe.

The protected boundary is an MCP client/model with only the fixed read-only MCP
surface. A separately invoked local interactive CLI is the trusted approval
broker. It proves that a process with current-user access completed a local
terminal interaction, not Daniel's personal identity. A same-user adversary with
arbitrary shell/PTY, secret-key or complete state-root access is outside this
boundary. Terminal detection/challenges cannot authenticate a person against that
adversary. No remote approval, new OS authentication service or M11 recovery
encryption is introduced.

## Decision

Keep eight crates. Domain owns pure scope, eligibility, lifecycle and replay.
Application obtains fresh trusted planning inputs and owns approval use cases/ports.
Persistence authenticates and serializes durable authority. CLI composes the
independent human workflow. MCP retains exactly its eight tools, DTOs and query
dispatch, with no approval, revocation, consumption, writer or key access.

ApprovalId and nonce are independent 256-bit OS random values. A grant binds one
TransactionId, the complete immutable TransactionBinding (including PlanDigest,
StateFingerprint, provider/target/native identity/intent/operation/profile/effects/
conditions commitments), RecoveryDigest, operation index zero, forward-only
scope, fixed local-interactive principal and issued/expires timestamps. At most
one grant may ever be issued for a transaction. Revocation/consumption are terminal;
expired or revoked permission requires a new freshly prepared transaction.
ApprovalId by itself grants nothing. Scope and time checks are pure; authenticity
is a separate keyed predicate enforced by persistence on every authoritative read.

Eligibility is centralized: protocol v2, Prepared transaction, exact Ready plan,
matching complete binding, fresh M4 Current replan, matching required recovery
manifest and verified durable recovery bytes, no previous grant. Approval cannot
override Unknown/Partial/conflicting/stale/Blocked/Unsupported or illegal states.
Fixed maximum lifetime is 300 seconds, with `issued_at <= now < expires_at` and
checked ranges. OS wall time is trusted; reject times before issuance and malformed
intervals. No caller-selected CLI clock/principal/expiry or environment bypass.
This does not claim resistance to arbitrary clock rollback or whole-store rollback.

### One history and exact review

Transaction protocol v2 records approval actions in the existing ordered journal,
with approval state orthogonal to provider outcome. New transactions retain their
bounded, validated original LifecyclePlan in their Created record for exact review.
No graph, discovery snapshot, arbitrary imported JSON/blob, transcript, prompt,
reply, source body, credential or cookie is retained. This narrowly extends M6's
commitments-only privacy policy: the exact plan necessarily contains typed local
identifiers, evidence locators and rule metadata. It stays inside the private owned
root, is limited by the existing journal record/history budgets and must reproduce
its original PlanDigest and binding. A redacted presentation is not the plan.

Protocol v1 records retain exact original text and hashes and remain inspectable;
their attempts cannot be upgraded to new authority or advanced by M7 writers.
New v2 attempts require fresh trusted planning. Transaction origin version never
changes. V2 rejects unsigned legacy Intent and CompensationIntent. Compensation
authorization remains future work.

Authorization actions are Granted, Revoked and Consumed. Consumed is one compound
event that both consumes the exact grant and records the forward effect intent,
moving Prepared to OutcomeUnknown. There is no standalone consume-then-intent gap.
The existing external write/sync then SQLite commit must complete before returning
a noncloneable DurableIntent. It identifies the approval and exact attempted
operation; M7 has no provider dispatcher. Missing observation remains unknown even
when no dispatch occurred. Every future effect still requires immediate fresh
replanning, recovery, compatibility and provider-contract checks.

### Authentication, keys and migration

Reuse existing locked RustCrypto HMAC-SHA256 and OS entropy. Authenticate every
authorization action over a fixed versioned domain plus key ID, transaction ID,
sequence, previous record digest, binding digest and complete unsigned typed action.
Ordinary record hashing includes the resulting full authentication tag. Verify
full tags with the maintained constant-time verification API; digests are not MACs.
Domain signing serialization is fixed and ordered, never M4's set canonicalizer.

One narrowly scoped random 32-byte key occupies a fixed private owned file. Create
with no replacement, flush bytes and directory, and anchor its domain-separated
key ID in immutable schema2 metadata. Existing authority never regenerates a missing,
partial, malformed or replaced key. A complete orphan key may be adopted only in
the validated pre-authority initialization/migration state. No rotation CLI exists;
future rotation requires an explicit migration and invalidation policy.

Schema2 initialization and explicit v1 migration run under the existing exclusive
lock. Before migration validate exact old schema, integrity and full external/DB
agreement. Create authority metadata and version update atomically with
BEGIN IMMEDIATE; preserve all v1 rows and external bytes. Read-only opening never
migrates. Unknown/partial schemas fail closed. A database rollback does not roll
back key-file creation; partial files remain protected evidence.

All authoritative open/list/inspect/grant/revoke/consume paths verify the anchored
key and every authentication tag as well as replay and dual-store agreement.
Store locks and expected sequence serialize races. Any ambiguous write poisons the
instance; divergence/torn history fails closed on reopen, with no truncation,
automatic regrant, retry or inferred provider result. HMAC does not prevent an
attacker restoring an entire older valid store or reading the current user's key.

### Human workflow

`plans prepare` computes a fresh plan through trusted application/provider ports,
then stores an eligible exact plan, transaction and recovery metadata.
`approve <plan-id> --transaction-id ... --state-root ...` loads that durable plan;
there is no imported plan/proof/grant input. Read-only plan/approval inspection
and explicit exact-grant revocation remain separate commands. No consume/execute
command, --yes/force/confirm or JSON/noninteractive grant mode exists.

Before confirmation show the whole bounded exact review: provider/profile, target,
intent/native operation/effect boundary, plan/transaction/state identities, status,
affected and retained/shared/global/unknown artifacts, pre/postconditions, recovery
requirement/limitations, space and blockers. Escape terminal controls including
newlines, ANSI and bidi characters; do not truncate away commitments or blockers.
Require actual terminal input/output and an unpredictable exact challenge. After
the interaction reacquire the writer lock, CAS the reviewed sequence and revalidate
fresh trusted state/recovery again before issuance. No lock or old review closes
the approval-time/execution-time TOCTOU interval.

## Alternatives and consequences

MCP approval tools, caller booleans, noninteractive flags, unauthenticated hashes,
bearer IDs, broad future-plan grants, separate competing approval databases,
mutable approval rows, cross-transaction replay and standalone consumption are
rejected. Same-user OS authentication and remote/web broker designs would change
the trust model and are deferred. Current real providers have no complete effects
or integrity contracts, so they remain Blocked/Unsupported and cannot obtain grants.
Positive tests use explicit synthetic contracts, never upgraded real fixtures.

## Validation and M8/M9 handoff

Generated properties and focused regressions must cover every scope/authentication
field, illegal order, one-shot/revocation/expiry, determinism and immutable plans.
Adapter tests cover old-schema preservation/migration interruption, key loss/
replacement, restart, corruption/dual-store gaps, authority crash cuts and races.
CLI/protocol tests prove no MCP authority, no noninteractive bypass, complete escaped
review and no provider effects. Fitness enforces crate/port direction, unchanged MCP,
pure eligibility and narrowly owned writers. Preserve every M0–M6 assertion.
Require all six local gates and actual exact-final-commit Linux/macOS/Windows CI.
Future adapters may dispatch only through a matching consumed-and-durable receipt
after fresh checks, and must observe/verify rather than infer crash outcomes.
