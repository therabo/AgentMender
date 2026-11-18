# ADR 0046 — Local session deletion

Accepted 2026-11-18. Updates the removal path in [ADR 0043](0043-ui-first-runtime.md).

## Decision

Deletion is permanent. Both providers require a single explicit confirmation
through the authenticated loopback UI. The browser identifies the selected
session by its opaque catalog key. The providers accept neither browser-supplied
filesystem paths nor commands.

- Codex runs the selected installation's local `codex delete <UUID> --force`
  command and reports the command result. The existing project action applies
  the same command serially to observed main conversations of a native project
  with complete membership.
- Claude follows the exact local paths recognized by Session Steward. The
  preview lists individual files and totals. The confirmed plan removes these
  entries and prunes empty scoped directories. Standard includes recognized
  session files. Approfondita also includes recognized checkpoint files.
  Equal UUIDs in different namespaces stay separate.
- Claude rewrites shared history through the canonical `history.jsonl.lock`
  directory lock. The replacement contains only retained bytes. When history
  is missing the operation skips it. When it cannot be safely rewritten the
  operation reports the remaining history and proceeds with scoped targets.

Session removal creates no backup and has no recovery path. Existing copies
remain on disk. The separate Codex Export Manager retains its own contract.

## Consequences

Claude deletion is best-effort. Close Claude before confirming to reduce the
risk of concurrent changes. Writers can recreate files or change history and
the operation can finish partially. The UI reports the errors and refreshes
the inventory. It does not retry a destructive action.

The inventory still groups recognized transcripts by root and namespace. Its
bounded storage observations and recognized Desktop registration evidence
remain available independently of removal.

[Session Steward reference at ac80fc3](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1).
