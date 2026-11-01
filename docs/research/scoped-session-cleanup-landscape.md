# Codex scoped session cleanup landscape

Research date: 2026-10-31. This is implementation research, not a grant of
destructive authority. M8.1's native-complete-cleanup **BLOCKED** finding remains
valid. M8.2 asks a different question: remove one exact session's primary state
and safely attributable storage, preserving uncertain and shared residuals.

## Acquisition and method

GitHub repository, code and issue searches covered Codex session deletion,
rollouts, `state_5.sqlite`, `session_index.jsonl`, `history.jsonl`,
`thread_spawn_edges`, archived sessions and disk usage. Public repository metadata
and licenses were checked separately from README claims. Nine third-party
repositories were cloned for static source inspection; no third-party program,
installation script or cleanup command was executed. Source links below pin the
inspected revisions. Not archived is an observed GitHub status, not a maintenance
or security endorsement. Additional search leads included `vyagh/reap`; its
implementation was not studied and supplies no design authority.

All seven requested starting leads were investigated. `seba-rb/codex-sessions`
returned GitHub API 404 and unauthenticated Git clone failure. Search still exposed
a cached README for cx-sessions. Its claims are explicitly **unverified** here;
neither a license nor an actual implementation was obtainable. This is not evidence
that the repository never existed, nor a source-code review of that project.

No third-party code is copied. AgentMender will implement its own typed design.
Observed patterns and their provenance are recorded below.

## Repository and strategy matrix

| Repository / inspected SHA | License; observed status / last push | Strategy and layout evidence |
|---|---|---|
| [Session Steward](https://github.com/mallikcheripally/session-steward/tree/3c394b2674e066c00268155823c7e04d783ebbc0) | MIT; not archived; 2026-09-24 | Local storage, backup first, multi-store best effort with verification/restore. Dynamically discovers state/log/memory/goal/queue/history DB families; no exact immutable Codex version capability. |
| [LeeeeTX/codex-history-cleaner](https://github.com/LeeeeTX/codex-history-cleaner/tree/7058d580170c2d47fbc3aac2c4742da8df62f8d8) | MIT; not archived; 2026-05-11 | Local storage; recoverable trash or permanent deletion. Current `threads`/spawn-edge layout detected operationally rather than an exact semantic profile. |
| [1939869736luosi/codex-sessions-manager](https://github.com/1939869736luosi/codex-sessions-manager/tree/dec7d29e23f78400698fd38fc63c98b50f32afa7) | Apache-2.0; not archived; 2026-08-02 | Local storage, backup first, journal/checkpoints and recovery. v0.8.1 sources retain 0.144.x compatibility evidence; POSIX mutation only, explicit Windows refusal. |
| [seba-rb/codex-sessions](https://github.com/seba-rb/codex-sessions) | Unavailable; status/license unverified | Cached README describes native App Server deletion and writer-lock checks; code inaccessible. Not adopted as an implementation reference. |
| [getoverroot/codex-clean-subagents](https://github.com/getoverroot/codex-clean-subagents/tree/20a7c44d45a1ac18f14ba434c3ae58e4cf53911c) | MIT; not archived; 2026-08-15 | Native-only closed-leaf subagent cleanup; preview, canary, batch. Detects CLI support through help, not an exact version/storage contract. |
| [hanityx/threadlens](https://github.com/hanityx/threadlens/tree/7a42d977126d508ee7e26a918209868e1edf819a) | MIT; not archived; 2026-05-20 | Local file cleanup, backup first, partial-result reporting. This cleanup path is not a canonical SQLite thread deletion. |
| [gabrielhamalwa/codex-cliner](https://github.com/gabrielhamalwa/codex-cliner/tree/fdf092c762a2528b7ab653e1e71d586f2ffb4798) | MIT; **archived**; 2026-06-01 | Local backup-first cleanup/restore of transcript/index/history files; no exact SQLite/provider version gate. |
| [AyoParadis/codex-cleaner](https://github.com/AyoParadis/codex-cleaner/tree/8740e0cc6987a4bc501691bced6ecf698d581162) | MIT; not archived; 2026-08-14 | macOS maintenance: native archive/unarchive plus local backup/move/delete of several storage categories. Not a one-session closure contract. |
| [coramba/codex-sessions-manager](https://github.com/coramba/codex-sessions-manager/tree/c458e9f4bdc6d1f6652b4f8bf0e1d4756978475d) | MIT; not archived; 2025-11-20 | Session browser. Remove UI copies an `rm` command to the clipboard; inspected application does not execute deletion. |
| [Gares95/codex-session-cleanup](https://github.com/Gares95/codex-session-cleanup/tree/a5ddd449110e4b8d0b9c01ea6cc0c69c2166ef79) | MIT; not archived; 2026-10-30 | Native-only retention planner, saved immutable manifests and stop-on-failure execution. Conformance matrix measured Codex 0.164.0; live schema/help checks are capability checks, not full mutation effects proof. |

## Implementation findings

### Session Steward

Primary source: [Codex store](https://github.com/mallikcheripally/session-steward/blob/3c394b2674e066c00268155823c7e04d783ebbc0/lib/providers/codex/store.mjs).
It combines database IDs, rollout headers, session names and command history;
recognizes plain/compressed siblings and multiple database families. Cleanup
mutates thread/dynamic-tool/spawn metadata, desktop catalog, names/history,
logs, queue, thread-history materialization and payloads. Deep mode additionally
changes memory/jobs, goals and known global-state references. Descendant expansion makes its default scope unsuitable for
AgentMender's exact-one-session operation.

It probes Codex writer-lock files with platform-specific locking and repeats checks
after backup, but releases probe locks before effects. Missing lock support can
become a warning. Its own mutation lock does not fence all Codex writers. Full
backups and automatic restoration of shared stores can retain the very bytes to
be reclaimed or overwrite concurrent unrelated changes. Useful patterns are
explicit storage diagnostics, preflight/revalidation, residual reporting and
recognition of native writer locks. Reject flexible-schema destructive authority,
probe-then-release concurrency, global memory jobs and implicit descendant scope.

### Codex History Cleaner

Sources: [delete.rs](https://github.com/LeeeeTX/codex-history-cleaner/blob/7058d580170c2d47fbc3aac2c4742da8df62f8d8/src/delete.rs),
[store.rs](https://github.com/LeeeeTX/codex-history-cleaner/blob/7058d580170c2d47fbc3aac2c4742da8df62f8d8/src/store.rs).
Selection uses exact `threads.id`; SQLite removes target spawn endpoints and the
thread in a transaction. It rewrites structurally matched session-index IDs and
deletes rollout/snapshot files after commit. Recovery mode backs up payloads,
metadata and removed index lines; permanent mode skips copies. It protects the
invoking environment session, but that does not prove absence of other writers.
Snapshot filename prefixes and a database path alone are insufficient ownership
proof. The fixed index temporary name, newline normalization, lack of a durable
cross-store effect journal and absent exact affected-row/postcondition proof leave
crash and concurrency gaps. Useful: exact keys, previews, explicit irreversible
mode. Reject prefix ownership and backup-everything as reclaim semantics.

### codex-sessions-manager (1939869736luosi)

Sources: [delete.ts](https://github.com/1939869736luosi/codex-sessions-manager/blob/dec7d29e23f78400698fd38fc63c98b50f32afa7/src/core/delete.ts),
[sqlite.ts](https://github.com/1939869736luosi/codex-sessions-manager/blob/dec7d29e23f78400698fd38fc63c98b50f32afa7/src/core/sqlite.ts),
[destructive-policy.ts](https://github.com/1939869736luosi/codex-sessions-manager/blob/dec7d29e23f78400698fd38fc63c98b50f32afa7/src/core/destructive-policy.ts).
It requires canonical UUIDs for destruction, refreshed selection, managed paths,
stable file metadata and root/DB identities. Its own durable operation lock,
checkpoints, pre-mutation recovery payloads and compare-before-rewrite helpers are
stronger than simple unlink loops. Files/index/history/global-state changes precede
exact SQLite metadata cleanup; failures attempt recovery. Known-key filtering
preserves unsupported memory and unknown global fields explicitly.

SQLite operations cover exact thread-linked tools/goals/stage outputs, edge
endpoints and thread rows, with assigned-agent references nulled. Flexible table
presence is not an exact schema/effect contract. Its own lock does not establish
Codex-wide coordination; active presentation overrides do not prove quiescence.
Payload copying into memory/backups is unsuitable for multi-GB reclaim. Adopt
exact identities, separate compatibility axes, checkpoints and retained residuals;
reject unsafe rollback over concurrently shared data and implicit transfer of
its tested layout to current Codex. Its inspected recovery is more selective than
whole-store replacement: recorded pre/post values and exact row keys gate recovery,
with conflicts retained as recovery-required.

### codex-clean-subagents

Source: [PowerShell/Python scripts](https://github.com/getoverroot/codex-clean-subagents/tree/20a7c44d45a1ac18f14ba434c3ae58e4cf53911c/scripts).
Read-only SQLite discovery computes descendants and eligible closed leaves,
excluding main roots, non-closed incoming status and outgoing edges. Frozen scope
hashes, one-item canary, subsequent batch, disappearance checks and free-space
measurements are useful patterns. Actual deletion uses `codex delete --force`;
nonzero exit is retried up to three times. There is no local-storage fallback or
metadata recovery. Closed status/help support do not prove inactivity or exact
native effects. Global free-space change is not attributable per-action reclaim.
AgentMender must reject retries after ambiguous native effects and must not inherit
the provider's cascading deletion merely because the selected candidate is a leaf.

### ThreadLens

Sources: [cleanup.ts](https://github.com/hanityx/threadlens/blob/7a42d977126d508ee7e26a918209868e1edf819a/apps/api-ts/src/domains/threads/cleanup.ts),
[forensics.ts](https://github.com/hanityx/threadlens/blob/7a42d977126d508ee7e26a918209868e1edf819a/apps/api-ts/src/domains/threads/forensics.ts).
The inspected cleanup path selects active/archived session-log files, backs them
up and unlinks them individually. Its `delete_cache` option does not add cache
targets in this path. It optionally edits a known UI-global-state surface and
returns partial failures.
Its confirmation token hashes selection/options/paths; it is not independent
authenticated authorization. Artifact discovery uses UUID substring matching in
basenames and suppresses enumeration errors. No SQLite primary-state deletion,
writer fence, physical-identity commitment or restart journal is established by
this path. Useful: preview and explicit per-file failures. Reject filename-based
destructive attribution, incomplete enumeration as closure and backup counts as
reclaimed bytes. Shared/unknown files outside the selected scan are simply unseen.

### codex-cliner

Sources: [actions.ts](https://github.com/gabrielhamalwa/codex-cliner/blob/fdf092c762a2528b7ab653e1e71d586f2ffb4798/src/commands/actions.ts),
[fs.ts](https://github.com/gabrielhamalwa/codex-cliner/blob/fdf092c762a2528b7ab653e1e71d586f2ffb4798/src/utils/fs.ts).
It creates backup manifests/payload copies, removes planned files and structurally
filters `session_index.id` / `history.session_id`, with a restore command. It does
not remove canonical SQLite state or establish native writer coordination. JSONL
helpers discard malformed lines and treat read errors as empty lists, so shared
file rewrites can lose unrelated information. Temporary rename alone does not
establish durable atomic recovery across stores. Adopt explicit recoverability
disclosure; reject lossy parsing and unverified shared-file reconstruction.

### Codex Cleaner (AyoParadis)

Sources: [CodexCleaner.swift](https://github.com/AyoParadis/codex-cleaner/blob/8740e0cc6987a4bc501691bced6ecf698d581162/Sources/CodexCleaner/CodexCleaner.swift)
and [SessionMaintenance.swift](https://github.com/AyoParadis/codex-cleaner/blob/8740e0cc6987a4bc501691bced6ecf698d581162/Sources/CodexCleaner/SessionMaintenance.swift).
The macOS cleaner refuses while the
Desktop app is seen running, backs up/moves selected directories, and offers
permanent deletion for stale generated images, visualizations and shell snapshots.
Worktrees and log families are moved rather than permanently deleted in the inspected
cleanup path. Native session archive/unarchive are separate; pinned-file recovery
can also move an archived rollout when an archived database state is not recognized.
App-process absence does not cover CLI writers. Age, directory membership and cwd do not establish
exclusive session ownership. Its separation of moved/deleted bytes is useful;
same-filesystem backup moves are not reclaim. No exact one-session crash/recovery
or cross-session SQL proof follows from a general maintenance run.

### coramba session browser

Source: [App.vue](https://github.com/coramba/codex-sessions-manager/blob/c458e9f4bdc6d1f6652b4f8bf0e1d4756978475d/src/App.vue).
It displays rollout-derived sessions and copies resume/new/remove shell commands.
Remove does not execute. Consequently there are no implemented SQL mutations,
index rewrites, locks, recovery, verification or measured reclaim to adopt. The
unquoted copied `rm` command is not an acceptable AgentMender execution boundary.

### Gares95 native retention planner

Sources: [cleanup_sessions.py](https://github.com/Gares95/codex-session-cleanup/blob/a5ddd449110e4b8d0b9c01ea6cc0c69c2166ef79/scripts/cleanup_sessions.py),
[compatibility matrix](https://github.com/Gares95/codex-session-cleanup/blob/a5ddd449110e4b8d0b9c01ea6cc0c69c2166ef79/contract/compatibility-matrix.md).
It groups roots with descendants, excludes protected/malformed/cross-boundary
groups, and produces fresh immutable saved plans. Private atomic manifest updates,
content hashes, Codex-version equality, one-shot manifest states and stopping after
the first native failure are useful. Its advisory lock serializes this tool, not
all provider processes. A standalone App Server cannot observe all other servers'
loaded sessions. Discovery starts provider processes and is not inherently
write-free. Execution delegates archive/delete effects to native CLI; no local
backup/SQL/index mutation or exact file-byte reclaim measurement is established.
AgentMender must not adopt group cascade as exact-one-session scope, schema shape
as effect proof, or manifest hashes as independent approval.

## Official Codex empirical reports

Issues are user observations, not specifications or reproduced AgentMender tests.
Open/closed status and descriptions were inspected on the research date.

| Issue | Relevant observation | Design implication |
|---|---|---|
| [34337](https://github.com/openai/codex/issues/34337) | Reported roughly 70 GiB across 644 rollouts; request for session storage management | Payload reclaim is a practical product requirement; tiny metadata removal alone is insufficient. |
| [34061](https://github.com/openai/codex/issues/34061) | Reported hundreds of GiB, mostly subagent histories | Descendants can be large, but size does not grant authority to delete them with their parent. |
| [39919](https://github.com/openai/codex/issues/39919) | Native delete reportedly removed a 9.85 GB rollout, then returned failure with stale thread metadata | Native failure can follow irreversible effects. Observe; do not retry or switch to unapproved local fallback. |
| [41645](https://github.com/openai/codex/issues/41645) | Loaded-thread deletion reportedly hung on lifecycle/writer coordination | Presentation, process existence and locks are separate facts; bound waits and distinguish no effect from ambiguity. |
| [40365](https://github.com/openai/codex/issues/40365) | A fork-history reference reportedly prevented bulk archived deletion | A parent's history may be primary data for another session. This is an essential shared-payload blocker, not an optional residue. |
| [43197](https://github.com/openai/codex/issues/43197) | Windows archived-session deletion failures | Do not infer cross-platform safety from macOS/Linux observations. |
| [53175](https://github.com/openai/codex/issues/53175) | Desktop catalog reportedly retained deleted entries independently of main state DB | UI catalog staleness is distinct from an intact resumable primary session; unsupported catalogs remain explicit residuals. |
| [54730](https://github.com/openai/codex/issues/54730) | Session-end hooks do not provide a reliable deletion notification for unloaded/plugin state | Unattributed plugin state is protected residue, not a reason to guess ownership. |

## Common storage layers and priorities

1. **Primary history:** active/archived rollout JSONL, compressed variants,
   immutable rollout IDs and optional thread-history materialization. These may
   contain most logical bytes. Canonical thread UUID and rollout UUID can differ.
2. **Canonical metadata:** state DB thread rows, exact auxiliary rows and spawn
   edges. Deletion needs the entire exact schema, including triggers/FKs, not just
   a list of familiar table names.
3. **Name/history indexes:** session-name index is append-only and shared;
   command history is a separate shared store. Exact-key parsing does not by
   itself prevent lost concurrent updates. Use native locking contracts or retain.
4. **Optional/unsupported stores:** goals, queues, logs, memories, Desktop
   catalogs, generated assets, caches and plugin state. Unknown/shared contents
   stay retained. SQLite free pages and WAL allocation are not reclaimed file
   bytes; no global VACUUM or checkpoint is justified for one-session cleanup.

## Official source investigation boundary

The official source inspected is tag `rust-v0.166.0`, exact commit
[`6ff1e1ca3b30fc8b914720681fcf5fe62e95f856`](https://github.com/openai/codex/tree/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856).
M8.1 independently retained its schema/protocol evidence; identical state schema
to an older release does not transfer cleanup authority.

Relevant independently read implementation paths are
[local delete](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/delete_thread.rs),
[writer coordination](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/writer_lock.rs),
[session index](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/session_index.rs),
[reference index](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/rollout_reference_index.rs),
[thread metadata deletion](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/state/src/runtime/threads.rs).
Native local deletion holds writer ownership, checks attached/history references,
removes history projections and rollout variants, then names and metadata. The
App Server adds descendant scope. Individual native store transactions do not
form a global filesystem/SQLite transaction. A native writer lock probe released
before deletion is inadequate. The home coordination lock serializes writer-lock
creation/removal and compressed publication; session-index writers have their own
persistent lock namespace. Neither is a universal filesystem lock.

## AgentMender implications and review gate

- Preserve native-first selection only when the exact effects fit the approved
  set. Current M8.1 native delete does not meet that condition. Select scoped local
  before approval; never improvise fallback after an ambiguous native result.
- Extend existing immutable plans/transactions/approval/recovery and DurableIntent;
  create no parallel cleanup transaction engine or generic SQL/file-write port.
- Split required primary removal from optional exact artifacts and retained
  residuals. A global M3 Unknown verdict is not a substitute for scoped evidence.
  Unknown identity, root, primary sharing, physical identity or writer boundary
  still blocks the target. Unknown unrelated caches do not.
- Freeze exact paths, native IDs, row sets/counts, rules, sizes and physical
  commitments before approval. Exact UUID, trusted root, stable safe file and
  payload metadata must agree. Execute a subset only; never infer ownership from
  cwd, filename proximity or a failed native call.
- Do not copy multi-GB payloads into a backup merely to delete them. Approval
  must disclose intentional payload irreversibility; recovery is reconciliation
  metadata, not a full backup or automatic shared-store restoration.
- Choose and document payload/metadata/quarantine ordering, crash boundaries,
  per-action observations and unknown outcomes. Reconciliation is read-only over
  the frozen manifest, without expanded discovery or automatic retry.
- Report approved/planned/deleted **logical file bytes** separately from unknown
  physical allocation and unrelated global free-space changes. Optional failures
  cannot become full Removed; failed primary removal cannot become success.
- Use generated preservation properties, exact SQL rollback/count tests, byte-
  preserving index tests, writer contention, link/substitution cases, crash/reopen
  scenarios and a positive disposable production-path acceptance test.

This research report precedes production destructive implementation. The exact
supported profile and concurrency/dependency proof must be decided and reviewed in
ADR-0017 before any provider-local writer is introduced. Historical M8/M8.1/M9
evidence remains unchanged.
