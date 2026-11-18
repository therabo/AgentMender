# ADR 0046 — Claude Code observation and cleanup admission

Accepted 2026-11-18. Extends the Claude read boundary in [ADR 0043](0043-ui-first-runtime.md).

## Context

Claude Code transcripts, shared prompt history and Desktop registrations have separate writers.
The inspected engine exposes project-wide purge. The provider needs session-specific evidence
before AgentMender can issue a one-use mutation receipt.

## Decision

Keep Claude inspection in the Rust provider. Group by effective root and project namespace.
Bind every conversation page to the observed transcript identity and revision. Use the existing
loopback authorization and browser-side pagination. Expose a diagnosis of storage dependencies
in the session detail. Admit a narrowly scoped offline removal workflow for the
verified macOS layout. Require an exact per-session plan, closed Claude
processes, validated markers, a single-use receipt and verified backup.

| Session Steward idea | Decision |
|---|---|
| Stream JSONL, cache metadata, inspect the end for activity | Adopt with per-file and inventory budgets, durable identity checks where available and explicit partial coverage |
| Custom titles, generated titles and first requests | Adopt ordered precedence, transcript-aware provenance and conservative display suppression |
| Group copies by UUID and match Desktop state by CLI UUID | Require the transcript's root and namespace and an unambiguous Desktop association. Keep scoped copies independent |
| Plan ordinary and checkpoint cleanup separately | Surface observed dependencies. Standard and Approfondita require separate verified ownership contracts before either can issue a receipt |
| Use process markers and PID start identities | Treat registry observations as evidence. An empty registry and a PID check cannot prevent a writer from reopening the archive |
| Use canonical proper-lockfile for shared history | Keep history read-only until the installed writer's canonical path, lock lifetime and replacement semantics are verified |
| Recursive filesystem cleanup and automatic restore after failed verification | Reject recursive UUID targets and automatic recovery. Require enumerated identities, an independently authorized recovery and destination revalidation |
| Verified backup before mutation | Retain as an admission requirement, including free-space check, a versioned manifest, SHA-256, durable file and directory sync, and revision checks |

## Consequences

The offline workflow adopts Session Steward's bounded concurrency model.
Process checks run at review, after preparing the backup and immediately
before mutation. A writer can start after a check. Descriptor-relative identity
checks and exclusive renames protect individual targets, and verification stops
on unexpected state. The archive can change between individual file operations.
The operation retains its backup on partial or uncertain outcomes. Platforms and
formats without an exercised write contract remain read-only. The
[audit](../audits/claude-code-storage.md) records the inspected formats and tests.

## Admitted layout and recovery

Standard admission on macOS covers fully enumerated engine 2.1.222
transcripts and attributable subagents. It includes a separately reconciled
Desktop registration when the observed Desktop bundle is 5.11492.5.
The shared scheduler remains preserved. Duplicate UUIDs, fork references,
unreadable markers and incomplete catalog evidence block the review.
An exact checkpoint basename must be referenced by a rewind snapshot in
the transcript for Approfondita to include it. Other sidecars, attached
data, unknown descendants and shared state are protected.

The per-backup manifest records every target and its SHA-256, original
filesystem identity and revision. AgentMender persists the phase before
moving files. Renames first move the named objects to the private backup
directory on the same volume for identity verification. Only planned
directories found empty are moved. They retain their identities for
recovery. Restored file publication uses an exclusive rename and journals
the intended new identity before publishing. The backup survives process
exit and remains available after a partial or uncertain operation.

Shared history remains byte-preserved. Session Steward coordinates
`proper-lockfile` on the canonical path and keeps the lease through its
rewrite. Engine 2.1.222 has a second append path whose cooperation has
not been established. AgentMender observes history revision and lock
presence and admits a limited cleanup of exclusive data with history
residues reported. The offline model is exercised in native temporary
archives and can race a process that starts after an observation.

[Session Steward reference at ac80fc3](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1).

## Validation

On macOS arm64, an isolated Claude Code 2.1.222 process produced a new
transcript. AgentMender verified its backup, removed the transcript and
restored its contents byte for byte. The HTTP fixtures covered the Desktop
5.11492.5 registration layout, preserved the scheduler and history, and
exercised Approfondita with a recognized rewind reference. The crash fixtures
recovered interrupted removal and restore. The Codex native regression and
desktop and narrow browser checks passed on disposable archives.

In a preview-only pass over the ten observed local main Claude sessions,
one was eligible for Standard, eight had unsupported transcript versions,
and one failed the exclusive ownership check. The Claude read-only crate
passed cross-target compilation for Linux and Windows arm64.

## Inventory reconciliation

Reconcile the bounded local transcript scan with the recognized Desktop Code
registration scan in the provider. Count malformed, ambiguous and out-of-budget
observations separately from verified conversations. Report absent transcript
candidates only when project enumeration covered the configured roots. A
registration with unresolved ownership remains diagnostic evidence. The UI
shows the same snapshot as the inventory before any list filtering.
[Reconciliation report](../audits/claude-desktop-code-reconciliation.md).

## Artifact storage inventory

Keep usage measurements and dependency evidence in the Claude adapter.
Enumerate bounded metadata in recognized provider roots. Model allocation,
scope attribution, protection and cleanup eligibility as independent fields.
Link exact transcripts and reconciled registrations through the existing
catalog identity. Treat version-dependent filenames and temporary layouts
as candidates. Serve usage through a dedicated read-only endpoint and reuse
the loopback session for mode-by-mode preflight in the session detail.

The [complete storage audit](../audits/claude-code-complete-storage.md)
records the native writer experiments and remaining admission requirements.
