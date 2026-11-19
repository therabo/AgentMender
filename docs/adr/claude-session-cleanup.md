# ADR 0046 — Local session deletion

Accepted 2026-11-18. Updated 2026-11-19 for complete removal. Extends
[ADR 0043](0043-ui-first-runtime.md) and [ADR 0045](0045-codex-storage-capabilities.md).

## Decision

Both providers expose one permanent deletion flow through the authenticated
loopback UI. A single explicit confirmation identifies the selected session
by its opaque catalog key. The provider resolves the paths from the archive
and session identity.

- Codex runs the installed `codex delete <UUID> --force` command in the selected
  archive. After success it filters the exact UUID from `history.jsonl` and
  `session_index.jsonl`, removes the local thread from recognized Desktop
  catalogs, prunes thread references in both Desktop state files when present,
  then requests sidebar reconciliation. A catalog write records the scan
  tombstone and advances the observation sequence and catalog revision.
  Native success with an auxiliary error is reported as partial. Project
  deletion of conversations uses the same flow serially for each verified
  member.
- Claude previews the main transcript, every entry in its session directory,
  recognized sidecars, and associated Desktop and UUID-only artifacts. The
  latter include checkpoints. The preview walks the scoped trees iteratively
  and retains the reviewed paths. Confirmation removes reviewed entries and
  their directories. Links are unlinked without following them. When
  another namespace or root could share the UUID, the shared artifacts stay
  in storage.
- Claude streams shared history through the canonical `history.jsonl.lock`
  directory lock and renews it periodically while writing. Both providers
  replace JSONL files only after the rewrite completes and retain the
  original bytes of other rows. Missing history is skipped. Occupied or
  unwritable history is reported alongside the result of session file removal.

Session deletion creates no backup and offers no recovery. Independent export
groups use the existing export workflow and its own confirmation.

## Consequences

Writers can recreate files or change history during an attempt. The UI reports
partial results and refreshes inventory after each attempt. Bounded inventory
and storage diagnostics remain independent of the Claude deletion preview.

[Session Steward reference at ac80fc3](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1).
