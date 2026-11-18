# ADR 0043 — Retire the conversational runtime

The current session deletion workflow is defined by [ADR 0046](claude-session-cleanup.md).

Accepted 2026-11-16. Supersedes the product and runtime portions of ADR 0042,
which remains in Git at the cleanup baseline `0b8ec50`.

## Decision

Keep the existing browser UI and its Rust runtime as the primary workflow.
Retain six workspace packages for the CLI, application, domain, shared
provider boundaries and two provider implementations.

Remove MCP, Luna orchestration, contextual search, persistent retrieval and
the unused durable lifecycle. Extract Codex's bounded metadata snapshot and
rollout reader into provider-specific operations for inventory and transcript pages. Keep only the provider-neutral observations used
in the application inventory. Return only typed visible messages and bounded
continuations from the reader.

Keep browser authorization and explicit confirmation in the CLI. Each
provider handles its own deletion. See ADR 0046 for the active workflow.

## Consequences

The application has one interactive management surface, a narrower provider
process protocol and no model runtime. Existing historical planning,
persistence, compatibility and search artifacts remain available in Git.
The new active documentation describes the local runtime exclusively.

The [dependency and reachability audit](../audits/2026-11-16-ui-first-cleanup.md)
records the decisions. The [acceptance report](../milestones/ui-first-cleanup.md)
records validation and remaining limits.
