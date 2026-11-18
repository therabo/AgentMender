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
in the session detail and refuse to issue a removal receipt while writer exclusion, complete
ownership and recovery contracts remain unverified.

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

The live Mac and unknown installations remain read-only. A cleanup contract needs a documented
provider operation or an enforced exclusion mechanism that covers writer startup, backup and
mutation. Process polling, marker age, file suffixes and version labels alone supply insufficient
evidence. The [audit](../audits/claude-code-storage.md) records the inspected formats and tests.
