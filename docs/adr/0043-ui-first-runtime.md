# ADR 0043 — Retire the conversational runtime

Accepted 2026-11-16. Supersedes the product and runtime portions of ADR 0042,
which remains in Git at the cleanup baseline `0b8ec50`.

## Decision

Keep the existing browser UI and its Rust runtime as the primary workflow.
Retain six workspace packages for the CLI, application, domain, shared
provider boundaries and two provider implementations.

Remove MCP, Luna orchestration, contextual search, persistent retrieval and
the unused durable lifecycle. Extract Codex's bounded metadata snapshot and
rollout reader into provider-specific operations for inventory, transcript
pages and removal evidence. Keep only the provider-neutral observations used
in the application inventory. Return only typed visible messages and bounded
continuations from the reader.

Keep browser authorization and confirmation in the CLI. Keep the executable
pin, preflight, fresh revalidation and native dispatch in the Codex adapter.
The existing Desktop marker remains guarded and success-only. Claude supplies
read-only observations.

## Consequences

The application has one interactive management surface, a narrower provider
process protocol and no model runtime. Existing historical planning,
persistence, compatibility and search artifacts remain available in Git.
The new active documentation describes the local runtime exclusively.

The [dependency and reachability audit](../audits/2026-11-16-ui-first-cleanup.md)
records the decisions. The [acceptance report](../milestones/ui-first-cleanup.md)
records validation and remaining limits.
