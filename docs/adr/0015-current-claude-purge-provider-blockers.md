# ADR-0015: Current Claude purge still requires an upstream lifecycle contract

## Status

Accepted, 2026-10-30. M9 is BLOCKED under the continuation's explicit definition:
safe completion requires an upstream provider change or authoritative new contract.
Extends ADR-0014's unchanged safety boundary; supersedes only its OPEN milestone
assessment. No Claude lifecycle profile, operation or dispatcher is enabled.

## Context

The previous exact installed profile (Desktop 4.29.0 / Code 2.1.222) remains
unsupported. Its committed evidence is preserved. Reusing mutated cases in the
historical reproducer limits attribution between startup and command phases;
it does not turn observed writes, collisions or partial deletion into safety proofs.
The new investigation uses independently generated state for every sequential
invocation and explicitly marks the deliberately shared concurrency experiment.

Official release and distribution observations distinguish latest non-prerelease
Code 2.1.314 from the delayed `stable` channel, 2.1.308. Both are separate exact
research candidates, downloaded only into private temporary evidence storage.
See the [current-version record](../../compatibility/claude/lifecycle-m9/current-2026-10-30/README.md)
for provenance, hashes, native results, limits and cross-version comparison.

Actual 2.1.314 probes retain writable dry-run/startup, non-injective path selectors
(`a-b` and `a/b`) and parent/nested cascades. Deletion denial now reports exit 1
and continues other removals, confirming an improvement without atomicity.
An unrelated encoded bucket with the same transcript cwd can survive purge:
cwd grouping is neither a complete membership index nor native target authority.

Neither the local CLI nor reviewed local SDK observation helpers provide an
authoritative coherent closure over all affected and retained state. Transcript,
directory, list omission, completion text or exit status cannot establish the
required negative postcondition. Writable dry-run cannot supply read-only planning
or reconciliation. Cloud Managed Agents APIs describe a different product.

## Decision

Retain zero Claude mutation capabilities and effects contracts. Do not add project
operation abstractions, a disabled dispatcher, storage mutation or an execution
shortcut. Exact version/hash/provenance is necessary compatibility evidence, never
sufficient authority. The installed 2.1.222 remains protected and is not upgraded.
No version range or floor becomes executable compatibility.

Preserve M6-M8's independent approval, recovery, fresh revalidation and consumed
DurableIntent pipeline. Its current session-unarchive operation/observation types
cannot represent project deletion. A session request must remain Unsupported;
it cannot widen into project purge. MCP cannot approve or dispatch.

For any future Claude contract, startup belongs after durable intent whenever it
may write. Only positive refusal before startup can establish NeverDispatched.
Once startup may occur, interruption, timeout, rejection, partial deletion or a
missing response remain OutcomeUnknown. A finite unchanged test fixture is not a
general NoEffect proof. Reconciliation observes only, never retries purge or its
writable dry-run. Purge remains NotCompensatable.

## Requirements to resume

Anthropic must provide an exact-version local contract with:

- A lossless native project identity and exhaustive membership mapping, including
  colliding keys, custom directory names, aliases and nested projects.
- A complete versioned startup/direct/cascade/retention boundary, including global
  configuration, backups, temporary data and concurrent writers.
- A genuinely read-only coherent observation interface covering that boundary,
  with authoritative absence and retained-state checks after ambiguous effects.
- Defined partial-failure, interruption and concurrency semantics sufficient to
  distinguish Applied, NotApplied and Unknown without a repeated mutation.

Then independently validate the exact artifact and operation, extend project
granularity through the existing pipeline, and run positive native approval,
crash, restart, retained-state and reconciliation acceptance suites. A renamed
command or improved exit status alone cannot satisfy these requirements.

## Alternatives and consequences

Raw JSONL/config/history edits, filesystem purge and hand-built native emulation
violate provider authority and remain forbidden. Native backups do not prove an
equivalent restore. More examples of successful deletion cannot close an unknown
namespace. M9 remains on its branch without integration or M10 work. Codex M8
semantics and the durable/approval/MCP schemas stay unchanged.

## Validation

Keep version-specific evidence separate, record fresh-case native counterexamples,
retain deterministic unsupported guards and source-fitness canaries, and run all
six local gates plus exact-final-commit Linux/macOS/Windows Actions. Investigation
success and green CI do not mean the M9 lifecycle milestone passed.
