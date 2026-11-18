# Provider compatibility

## Codex reads

The macOS worker recognizes fixed columns in an ordinary `threads` table in
`state_5.sqlite`. It accepts additive schemas for metadata observation. The
read-only SQLite backup is bounded and source files are opened under kernel
write denial. Transcript locators stay under `sessions` or `archived_sessions`.
Only fixed plain/compressed and archive counterparts are considered. A matching
`session_meta` header is required. Known legacy and paginated display records
are supported. Paginated display previews remain partial regardless of physical
EOF. The reader exports user and assistant display text from the requested
rollout, with conservative credential suppression.

Visible conversations require a recognized main source, no inbound spawn
edge, and no fork, attachment or worker evidence. A scoped rollout must have
matching root identity metadata. A metadata-only observation requires explicit
fork and attachment columns with empty values and a readable spawn table.
Unknown sources, malformed references and ambiguous evidence are excluded
from the visible inventory. The presence of a parent in the visible page
plays no part in classification.

## Codex native removal

AgentMender invokes `codex delete <UUID> --force` for the selected local
session and shows the executable's result. Available versions are determined
by whether the installed command can execute successfully. Codex owns the
deletion semantics and its protections. AgentMender refreshes the inventory
following the attempt. [Codex CLI reference](https://learn.chatgpt.com/docs/developer-commands?surface=cli#codex-delete).

### Registered Codex projects

In Codex 0.169.0, migration 49 defines `projects`, `project_roots` and the
nullable `threads.project_id` association. AgentMender reads these fields from
the confined SQLite snapshot. Registered projects can have multiple roots,
duplicate display names and zero conversations. A thread's `cwd` supplies
context and can match any project's root independently of its assignment.
[Native schema](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/state/migrations/0049_projects.sql).

Codex Desktop can retain project registrations in `local-projects` and
assignments in `thread-project-assignments` while native thread migration is
incomplete. The worker reads a bounded, no-follow observation of
`.codex-global-state.json`. It uses the recognized local record shape,
`thread-project-membership-host-ids`, explicit `projectless-thread-ids`, and the
migration map under `local:<configured archive>`. Matching registered IDs or
an explicit migration alias reconcile native and legacy registrations. A
mapped legacy record whose native target has disappeared is skipped when the
native catalog is readable. A recognized local legacy assignment can fill a
null native assignment. Conflicting assignments and dangling IDs go to Recenti.
Unknown formats qualify coverage. The reader suppresses unrelated state fields
and checks file identity around the bounded read.

In the inspected Mac installation, the two registered native projects also had
legacy entries linked by the host migration map. Native `threads.project_id`
was null throughout the observed database, while Desktop retained explicit
local thread assignments. The matching inventory therefore needs both sources.
The installed app-server did not initialize against the live archive with writes
denied. The installed build's experimental list was verified on a disposable
archive, and the live catalog was read through the confined snapshot instead.

The app-server exposes experimental `project/list`, `project/read` and
`project/delete`. The 0.169.0 and 0.169.0-alpha.11.1 fixtures exercised these
methods, including rejection without experimental initialization. The native
delete method clears the project assignments of surviving threads and deletes
the registration. Session deletion requires the separate UUID cascade.
[Protocol](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/app-server-protocol/src/protocol/v2/project.rs) ·
[Request processor](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/app-server/src/request_processors/projects.rs) ·
[Storage implementation](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/state/src/runtime/projects.rs).

AgentMender offers bulk session deletion for a native registered project
with complete membership. It dispatches the same command serially for its
main conversations and stops on failure. Project registrations and workspaces
remain separate from the session action.

## Codex discovery and export inventory

| Capability | macOS | Linux | Windows |
|---|---|---|---|
| Discovery | Verified bundle metadata, native image architecture, fixed version probe, home and Library candidates | CLI paths and XDG candidates with Codex evidence. Version probe requires system Bubblewrap. | CLI paths and Windows AppX registration. Package version and executable headers are available when readable. |
| Verified session inventory | Seatbelt read-only worker on macOS, including Intel when its reader is available | Unsupported until an isolated catalog reader is verified | Unsupported until an isolated catalog reader is verified |
| Native session removal | Installed executable with `delete` and readable session inventory | Browsing requires a supported inventory reader | Browsing requires a supported inventory reader |
| Export inventory and selection preview | With Unix block counts | With Unix block counts | Logical sizes. Reparse points are protected. |
| Export removal | Permanent after explicit confirmation | Permanent after explicit confirmation. Directory removal requires `openat2` support. | Unsupported |

The usable default is `CODEX_HOME` or the provider home. The platform adapter
can select a single corroborated alternate when the default is absent.
The discovery view also shows additional archives, split SQLite or log
roots, installed executables and protected user output. Separate SQLite
roots are reported even when the current session worker cannot read them.
Network paths on Windows are displayed as protected. WSL is a separate
environment and the Windows manager does not auto-traverse it.

The `exports` directory is discovered from the Codex archive layout. A group
without producer metadata can be eligible through its export root and explicit
selection. Desktop Artifact Session publications have a versioned marker and
lease, a shared `latest` area and an exporter-owned quota protocol. AgentMender
preserves the shared area and groups with active leases. Externally configured
export groups are identified as user output when their metadata does not indicate
an independent backup.
[Windows AppX registration](https://learn.microsoft.com/en-us/powershell/module/appx/get-appxpackage) ·
[Codex config reference](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/config/src/config_toml.rs).

## Claude Code

| Capability | macOS | Linux | Windows |
|---|---|---|---|
| Discovery | CLI, Desktop bundle and bundled Code engine, default and explicit roots | CLI and configuration roots. The version probe requires Bubblewrap | CLI and Win32 candidates, recognized Store/MSIX AppX registration, native and explicit roots |
| Inventory and preview | Bounded local project JSONL | Bounded local project JSONL | Read-only project JSONL with reparse point, volume and link checks. Native Windows runners are in the CI matrix |
| Storage metadata and preview | Descriptor-relative inventory with observed file blocks | Descriptor-relative inventory with observed file blocks | Reparse-point checks and logical sizes. Allocation is unavailable |
| Standard cleanup | Recognized local transcripts, associated subagents, registered Desktop session files, known sidecars and session state | Read-only | Read-only |
| Approfondita | Standard plus scoped `file-history` files | Read-only | Read-only |
| Shared history | Rewrite under the canonical directory lock when UUID ownership and the current file can be checked | Read-only | Read-only |

Project identity includes the archive root and namespace. Equal UUIDs in
different namespaces remain distinct. Corroborated Desktop records supply
archive status and some relationships. Subagents, sidechains and recognized
forks stay outside the main list. Transcript dates and filesystem modification
time are independent. Server-held conversation cursors expire and become stale
when the underlying transcript changes. Unknown versions can be read if the
location and record evidence support the format.

Claude deletion is permanent and best-effort. Active writers can change or
recreate files. Shared history or UUID-only state is preserved when it cannot
be safely associated with the selected conversation. A locked or changed history
file is reported in the result.
[Storage audit](audits/claude-code-storage.md) ·
[ADR 0046](adr/claude-session-cleanup.md).

[Complete storage audit and native writer probe](audits/claude-code-complete-storage.md).
