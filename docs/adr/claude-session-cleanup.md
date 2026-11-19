# ADR 0046 — Local session deletion

Accepted 2026-11-18. Updated 2026-11-19 for complete removal. Extends
[ADR 0043](0043-ui-first-runtime.md) and [ADR 0045](0045-codex-storage-capabilities.md).

## Decision

Both providers expose one permanent deletion flow through the authenticated
loopback UI. A single explicit confirmation identifies the selected session
by its opaque catalog key. The provider resolves the paths from the archive
and session identity.

- Codex builds the graph in the read-only worker from native spawn and
  attachment relations and verified rollout headers. A single receipt binds
  the reviewed roots and descendants. Confirmation starts native deletion
  on the selected roots directly. Subsequent observations identify reviewed
  descendants still eligible for deletion and report changed or unreviewed
  branches. Eligible descendants are deleted deepest first. A root refused
  because of linked children is retried after those children are verified
  absent. The result counts successful native UUIDs and verified absences
  separately. The provider cleans their union once, even if a later
  observation fails. Projects use the same workflow for verified main
  conversations. The graph operation shares one 60-second budget across
  snapshot acquisition and rollout header reads.
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
partial results and refreshes inventory after each attempt. Claude inventory
walks recognized locations and reads complete transcripts. Conversation
preview and storage diagnostics keep separate bounds.

[Session Steward reference at ac80fc3](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1).
