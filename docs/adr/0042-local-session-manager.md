# ADR 0042 — Local session manager

## Status

Accepted 2026-11-16. Supersedes the primary product experience, milestone order
and MVP acceptance criteria in [ADR 0022](0022-product-first-session-lifecycle.md).
The existing MCP tool contracts continue under [ADR 0037](0037-autonomous-mcp-session-find.md),
[ADR 0038](0038-native-codex-delete-admission.md) and their subsequent decisions.

## Context

The user needs to browse and manage local Codex and Claude Code archives directly.
A prompt, a signed-in model, and MCP configuration were required for the previous
primary workflow. The underlying Rust workspace already has provider-scoped
identity, bounded archive reads and a fixed native Codex command.

## Decision

- `agentmender ui` starts an ephemeral HTTP listener bound to `127.0.0.1`. Rust
  serves compiled React, TypeScript and Vite assets plus typed JSON endpoints.
  CLI composes the same two provider adapters. The browser receives display
  fields and opaque root/provider/session keys. Only trusted launch arguments
  can select the provider roots and executable.
- The application indexes bounded metadata in memory for deterministic Unicode
  normalization and full case folding, state and UTC date filtering, stable
  global sorting and pages. Provider discovery keeps its partial or unavailable
  status. The runtime reads a Codex transcript only on request through the existing
  confined reader. The API issues scoped, expiring transcript cursors.
- The browser exchanges a one-use launch ticket from the URL fragment for a
  per-launch bearer credential. Same-origin mutations require both that
  non-ambient credential and the exact Origin, with exact Host on every request.
  The listener neither enables CORS nor grants mutation through cookies, paths
  or caller-supplied commands. CSP denies inline script, external resources and
  embedding. Terminal access and other same-user processes remain part of the
  local trust boundary.
- A successful preflight issues one server-held, short-lived removal preview for
  a particular Codex UUID and observed root, database and transcript revision.
  The browser requires the typed review phrase. The backend checks the phrase
  against the live preview and consumes the opaque receipt under lock before
  creating a single in-flight operation. Immediately before dispatch the
  server rechecks the pinned binary, root, canonical row, archive state,
  configured protections, transcript identity/revision and outgoing spawn edges.
  Known descendants and uncertain evidence cause refusal. Independent provider
  writes remain possible between that check and dispatch. The prompt discloses
  the native cascade and the reviewed Codex binary adds native writer locks.
- This surface admits the exact Darwin arm64 Codex CLI
  `0.169.0-alpha.11.1`, SHA-256
  `3ea20787e106af94573d3d6c19480aed93a1334b0dd32b6b04e2f780ce82cf4b`.
  The CLI reuses the existing native delete builder, timeout and eligible
  best-effort Desktop marker refresh. A fixed `sqlite_home` override binds the
  command to the observed root. macOS denies network and native writes outside
  that root. The same command handles writer and reference checks. New
  executable builds require their own evidence before admission.
- Dispatch status is process-local. It records success of the command,
  pre-dispatch refusal or uncertain outcome. A lost response allows a status
  query with the original receipt. No branch retries the command. A dispatch
  result is distinct from the durable lifecycle and historical cleanup proof
  contracts. Native exit status leaves residuals, Desktop state and physical
  storage reclaim unverified.
- Claude Code reads bounded metadata prefixes under its existing project
  discovery boundary. Verified metadata can be displayed without granting
  mutation. The provider has no enabled single-session mutation profile.
  Artifact attribution can extend the detail response when a provider supplies
  evidence of ownership. Transcript bytes describe measured file size.

## Consequences

The normal runtime requires the local binary and browser. Building frontend assets
requires Node, and the next Cargo build embeds them. Existing CLI and MCP users
can continue to build with Cargo alone. Their optional search mode retains Luna
and its verified publication, FTS5, exclusion and evidence checks.

Codex live UI reads depend on the existing macOS-attested worker. Claude project
metadata uses the existing filesystem inventory helper. Unknown provider
versions can still be displayed where a read contract is supported. Results from
the disposable exact fixture establish the native path and sentinel preservation
for that case. Structural cleanup proof for arbitrary archives remains outside
that claim.

See the [audit](../audits/2026-11-16-local-session-manager.md) and
[local API contract](../local-session-manager.md).
