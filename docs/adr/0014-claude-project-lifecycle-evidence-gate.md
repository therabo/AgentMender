# ADR-0014: Claude project lifecycle evidence does not confer dispatch authority

## Status

Accepted safety boundary, 2026-10-30. M9 remains OPEN. Extends ADR-0008/0009/0013;
does not enable a Claude mutation interface or change Codex behavior.

## Context and evidence

M8 was formally integrated at `83f0692cdda33bc3be34146ab8ceefab5dadf1f3`
before M9 began. Independent detection still observes Desktop 4.29.0 and its
bundled Code 2.1.222. ReadOnlyKnown describes the existing Desktop metadata parser,
not lifecycle compatibility or historical transcript producers.

Current [official directory documentation](https://code.claude.com/docs/en/claude-directory#clear-local-data)
describes project purge and says its pre-2.1.288 name was `claude project purge`.
The exact installed binary recognizes that interface. Isolated actual-binary
experiments, using only synthetic state, demonstrate that even `--dry-run` can
write configuration and backups before returning its proposed deletion plan.
These observations are evidence about a candidate contract, not a production
profile or imported planning proof. See the retained M9 evidence record.

Current Claude Project observations have no provider_project_id. Their cwd-based
grouping and Desktop-to-CLI mappings remain heuristic. They do not establish the
native purge target or exhaustive project membership. Desktop archive UI exists,
but no exact external lifecycle/effects/verification contract has been validated.

M8's durable substrate is reusable, but its planning operations describe sessions
and its application workflow implements session unarchive. Neither may be renamed
or relabeled as project purge. Deletion also lacks an authoritative complete native
postcondition and retained-state proof suitable for the existing integrity gate.

## Decision

Enable zero Claude lifecycle operations. Retain empty Claude planning capabilities,
no NativeEffectBoundary, no production Claude dispatcher and no lifecycle profile.
Preserve read-only discovery, every existing readiness blocker, unchanged durable
schemas/approval authority, and the eight read-only MCP tools.

Do not add a placeholder mutation adapter, a generic executor or a project operation
enum without a usable independently validated contract. In particular, do not encode
project purge as DeleteSession, resolve authority from an arbitrary path/cwd, or
widen a session request to its containing project. Unsupported requests contain no
executable operations; unknown profiles remain mutation-blocked.

Record the native command, exact version/binary evidence, documented versus observed
effects, residual categories and missing proofs separately. Guard properties and
architecture checks demonstrate the protected absence of support. They cannot
demonstrate native acceptance, crash recovery for a Claude effect, or M9 completion.

## Future execution and failure semantics

Any future supported project contract must explicitly extend target granularity,
immutable plan validation, exact capability/effect commitments, application
observation and authoritative verification. Reuse the existing independent approval,
verified recovery and noncloneable DurableIntent rather than creating a shortcut.
All potentially writable startup, including a native dry-run, belongs after durable
intent unless an independently validated read-only isolation contract proves otherwise.

Before writable startup, positive proof of refusal can establish NeverDispatched.
After possible startup, failure/timeout/rejection/exit status alone cannot establish
NoEffect. A success response is only observed acceptance. Fresh complete native
evidence must verify every promised deleted and retained entity before success.
File/list absence and continued presence after ambiguous dispatch are insufficient.
Reconciliation observes only, never recreates intent/approval or retries. Missing
retained baselines remain Unknown/RecoveryRequired. Recovery metadata is no backup;
purge has no demonstrated native inverse and remains NotCompensatable.

## Alternatives

Direct transcript/directory/sidecar mutation violates provider authority. Treating
cwd or cliSessionId as a project capability widens scope without proof. Invoking a
writable dry-run before approval violates journal-before-effect. Synthetic successes
cannot validate the real binary. Premature project abstractions or a disabled native
dispatcher add attack surface without a supported operation. All are rejected.

## Security implications and validation

No Claude effect path exists, so no Claude effect can bypass intent, approval or
fresh validation. No raw command, filesystem lifecycle, SQL or JSONL/sidecar write
fallback is introduced. Preserve provider privacy, unknown producers, Global/Shared/
Referenced/Unknown protection, and Codex's exact narrow support set.

Test both Claude fixture classes, project/session targets, every intent, unknown
versions, weakened/claimed evidence and changed storage observations. Unsupported
preparation must not create durable state; Claude execute/reconcile must reject
before state access. Keep MCP authority tests and all Codex regressions unchanged.
Run all six local gates and exact-final-commit Linux/macOS/Windows CI. M9 remains
OPEN until actual native project orchestration and its missing proofs are delivered.
