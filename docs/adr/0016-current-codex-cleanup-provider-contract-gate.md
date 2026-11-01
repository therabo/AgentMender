# ADR-0016: Current Codex cleanup requires a closed native observation contract

**M8.2 cross-reference (2026-10-31):** [ADR-0017](0017-scoped-cleanup-residual-preservation-provider-local-mutation.md)
partially supersedes native-only rules for its exact Codex scoped-local contract.
Semantic direction is accepted; implementation remains OPEN. The historical
native/read-only contract and evidence below remain unchanged; see
[current scoped-cleanup status](../scoped-session-cleanup.md).

## Status

Accepted, 2026-10-31. M8.1 full cleanup is **BLOCKED BY PROVIDER** after M9
blocker evidence was integrated at `9daa237ac6eadeb3d74e71b19ba31bcd6eef97ef`.
Extends ADR-0008/0009/0013. No current-release mutation profile is enabled.

## Context

The complete research, milestone specification, ADRs, integrations, source, tests,
compatibility evidence and Git/CI state were reconstructed before changes. M9
was closed as an investigation and integrated as evidence/guards, never as PASS
or completed Claude lifecycle capability. M10 and natural-language interpretation
remain out of scope.

Official stable Codex is `0.166.0`, independently checked through OpenAI's GitHub
release and npm distribution. The privately acquired macOS arm64 binary and archive
hashes, repeat-generated stable/experimental schemas, state layout and native
fixtures have [separate exact evidence](../../compatibility/codex/0.166.0/lifecycle-m8-1/README.md).
Matching storage layout and method schemas do not transfer lifecycle semantics.
Official tag association is established; a reproducible source-to-binary build
identity is not claimed.

## Decision

Keep current-release ArchiveSession, DeleteSession and changed-descendant
UnarchiveSession unsupported. Preserve the historical exact `0.165.0-alpha.1.2`
conditional target-only unarchive profile unchanged. Do not add an executor,
disabled dispatcher, broad version rule, inferred ownership, RemoveSession alias,
or special Ready-planner exception while these proofs are missing.

Current relationship-filtered enumeration improves on the historical ordinary-list
preview omission. Parent/ancestor filters include empty previews. An attached
spawned child omitted by those filters can be recovered by an attached query in
the tested fixture. That fixture alone does not prove union-based closure impossible.

The remaining source-derived mismatch is more fundamental: native cascade traversal
reads raw persisted spawn edges, while public relationship queries join thread
rows. An edge-only leaf is outside that exported row namespace. This is source
analysis, not an observed UI-created or actual-binary edge-only counterexample.
A dedicated fresh App Server can potentially keep its own live registry empty;
other processes' live registries must not automatically be included in its effects.
Neither restriction establishes a coherent complete persisted effect/retention
contract over the remaining namespaces.

Actual native archive can accept while a denied child stays active and a grandchild
archives. Native delete can fail after removing descendants. New unarchive restores
descendants, can skip a denied child, and restores children archived before the
forward archive. It is not scope-equivalent compensation. All compensation remains
NotCompensatable; recovery metadata is not a restoration backup.

`thread/read` can establish useful exact current-view metadata presence. Its exact
lookup/error semantics deserve source-bound interpretation; lack of a typed public
NotFound variant alone is not a blocker. However, the selected current thread view
does not certify every historical rollout, projected history, cleanup, shared or
global namespace touched by delete. Metadata can remain after its rollout is gone;
history-read failure cannot then prove entity absence. Initial metadata error
suppression is followed by a resolver re-query that can propagate persistent errors;
no persistent DB-error-as-absence counterexample is claimed.

Native lookup fallback can repair metadata, and startup can migrate/compress state.
Writable startup belongs after durable intent. Reconciliation must use a genuinely
protected observation boundary, with real-state write denial and only owned
disposable writes. An ordinary App Server read against writable real state cannot
be relabeled read-only reconciliation.

## Outcome and authorization invariants

Only proved refusal before writable startup can establish NeverDispatched/NoEffect.
After possible startup, errors, timeout, process death, partial effects, unchanged
root presentation or missing responses cannot establish NotApplied. RPC acceptance
is member-level evidence, not full Applied or Succeeded. Complete fresh proof of the
approved target/cascade postcondition and retained invariants would be required.
Missing coverage or baseline stays Unknown/RecoveryRequired; observe without retry.

Any future capability must reuse Ready LifecyclePlan, TransactionBinding,
verified RecoveryStore, independent M7 ApprovalGrant, immediate fresh revalidation,
authenticated atomic consumption, durable external journal and SQLite intent,
nonconstructible DurableIntent, native dispatch, DurableObservation, fresh graph,
recomputed IntegrityReport and verified postconditions. No constructor, raw RPC,
JSON command, SQL/file fallback, approval bypass, scope expansion or reconstructed
dispatch receipt is introduced. MCP retains eight read-only tools and cannot approve.

## Requirements to resume

Establish an exact-version authoritative contract that closes preapproval candidate
identity/effects, including raw-edge and relevant materialization namespaces;
defines startup/direct/cascade/shared/global/retained effects and concurrency;
provides coherent protected read-only observation over that same boundary; separates
unavailable/conflicting/error from absence; and supports decisive postcondition and
retained-state reconciliation after ambiguous outcomes. A typed not-found endpoint
alone or more successful fixture deletions would not be sufficient.

Internal improvements—nested metadata preservation, canonical UUID collision checks,
current-view error interpretation and retained-baseline persistence—may improve
evidence but cannot by themselves supply the missing complete provider contract.
Do not keep weakening requirements. A roadmap decision may follow this blocker;
M10 is not started.

## Validation

Keep all ordinary, property, approval, crash, reconciliation and old M8/M9 regressions.
Add exact current/neighbor-version refusal and wider-effect rejection, source-fitness
canaries and deterministic schema/evidence validation. Native counterexamples remain
explicit opt-in on newly generated private state and are never required by CI.
Require all six local gates and exact-final-commit Linux/macOS/Windows CI. Passing
these checks validates the evidence and refusal boundary, not full cleanup capability.
