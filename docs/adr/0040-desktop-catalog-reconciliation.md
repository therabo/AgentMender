# ADR-0040 — Request Desktop catalog reconciliation after native Codex delete

Accepted 2026-11-14. Extends the native delete MVP in
[ADR-0038](0038-native-codex-delete-admission.md).

## Evidence

The installed macOS ChatGPT Desktop 26.1111.21308 (build 27194) uses
`CODEX_HOME/sqlite/codex.db`. Its bundle selects `codex-dev.db` for non-production
build flavors. Read-only inspection of both files found WAL mode and the same
catalog schema. `local_thread_catalog` keys entries by `(host_id, thread_id)`.
`local_thread_catalog_sync_state` keys progress by `host_id` and stores the
nullable integer `last_full_reconciled_at`. The relevant tables have no
triggers or foreign keys. Startup sync chooses a full scan if the initial
catalog build is incomplete or the stored full-reconciliation timestamp is
null. The catalog supplies the Desktop sidebar. See the
[projection inventory](../research/codex-desktop-projections.md) and
[upstream reproduction](https://github.com/openai/codex/issues/53175).

Desktop handles `thread/deleted` from its own App Server. The
[public App Server protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
documents that notification. No documented external sidebar refresh operation
was identified for a delete already completed by a separate CLI process.

## Decision

The CLI invokes a fixed Codex provider helper after the one confirmed native
command exits zero. On macOS the helper checks both known filenames beneath the
same configured home. It rejects symlink components, non-regular or hard-linked
database files and sidecars, and paths with a rollback journal or only one WAL
sidecar. It opens existing files with SQLite NOFOLLOW and without create flags.
When both WAL sidecars are absent, an advisory immutable read avoids creating
sidecars for a catalog that needs no update. The observed WAL mode, ordinary
tables, key columns and sync-state shape must match before any update. Triggers
or foreign keys affecting the sync-state table cause a skip.

Schema inspection and the gated update share one SQLite transaction. The update
sets `last_full_reconciled_at` to `NULL` only on the `local` sync-state row, if
that catalog contains the selected UUID for `local`. Already-null state needs
no write. The helper leaves entries, other hosts, checkpoints and transcripts in
place. SQLite handles ordinary WAL writes. The helper disables its own automatic
checkpoints and checkpoint-on-close so it leaves WAL/SHM cleanup to Desktop.
The connection uses a zero busy timeout and the helper makes one attempt per
candidate. Missing files, unsupported schemas and SQLite failures are silent
best-effort outcomes. The MCP response continues to reflect only the native
command's result.

## Consequences and validation

Desktop can hold sync state in memory and may overwrite the marker while open.
The marker requests a full reconciliation on a subsequent startup. A user may
need to quit Desktop completely and reopen it. AgentMender leaves app control
to the user, and the SQLite write alone cannot certify sidebar disappearance.

The helper compiles only on macOS. Linux and Windows use the existing native
delete executor without Desktop catalog access. The Windows Desktop and WSL
stores have no admitted layout in this change. Search and selection still
depend on the existing macOS-isolated workers, which can return
`CapabilityUnavailable` on other platforms.

Disposable tests use the captured catalog DDL, cover both filenames and check
host isolation, exact UUID matching, retained rows, WAL/SHM preservation,
missing or incompatible catalogs, path refusal and database contention. MCP
fixtures exercise signed selection, one elicitation, one native command, then
the best-effort SQLite step across success and failure cases.
