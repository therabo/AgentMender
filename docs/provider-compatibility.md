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

Darwin arm64 builds reporting `0.169.0-alpha.11.1` or `0.169.0` are covered
by disposable native cascade tests. At launch AgentMender resolves a regular
executable to its canonical absolute path, probes its version under write
and network denial, and checks that `delete --help` exposes noninteractive
UUID deletion. The version, command surface and exercised release semantics
are all required. AgentMender pins the executable file and configured root
identities through confirmation. Builds reporting other versions are disabled
until their cascade, writer and reference behavior is verified.

Removal requires the observed `state_5.sqlite` contract through migration
62, recognized thread and spawn-edge tables, and the known timestamp and
recency triggers. The read-only worker traverses all outgoing spawn edges
from the selected main UUID, including IDs with an edge but no canonical
row. It reconciles canonical locators and active/archived rollout headers
against the complete bounded graph. Reviewed files must be unshared, plain,
canonical archived rollouts. A closed edge can establish a descendant with
no canonical row and no observed file. Other archive representations,
ambiguous locators, unsupported storage or exhausted evidence budgets can
block removal even while inventory remains available.

An active or in-use member, an unclosed edge, a protected UUID anywhere in
the tree, or an external fork, attachment or shared history reference blocks
review. The receipt binds the selected session, full membership, SQLite and
rollout revisions and provider config file revision. A change observed before
dispatch invalidates the review.

Confirmation runs exactly one `codex delete UUID --force` in the original
root with the fixed `sqlite_home` override, a 180-second deadline, kernel
write confinement and network denial. Writes in the rollout trees are limited
to the exact reviewed paths. Codex applies its cross-process writer and
reference checks. The CLI has no expected-graph-revision parameter, so an
independent writer may still change SQLite after the final observation and
before native discovery. Use a fresh review after any pre-dispatch refusal.
See the [Codex deletion implementation](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/app-server/src/request_processors/thread_delete.rs)
and the [native writer and reference checks](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/thread-store/src/local/delete_thread.rs).

After dispatch a read-only check looks for remaining canonical rows,
expected rollout files and spawn edges, and compares evidence of unrelated
sessions. A failed command, timeout, residual evidence or incomplete read
produces an uncertain or partial result. AgentMender issues no automatic
retries. Verified removal can request the existing guarded
`last_full_reconciled_at` reset for the matching local Desktop catalog.
Desktop visibility, other artifacts and physical bytes reclaimed are outside
the verification. The [disposable fixture](local-session-manager.md#test-with-disposable-archives)
checks byte-for-byte preservation of unrelated fixture rollouts and rows.

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

AgentMender offers bulk **session** removal for verified native membership.
Desktop-backed projects support browsing and individual reviews. The bulk
preflight verifies all assigned roots and spawned descendants, checks
cross-project assignments and fails closed if a rollout-only main or incomplete
catalog prevents a complete plan. The scan budget covers 16,384 records per
collection, eight seconds per worker observation, 32 main conversations and
256 total members (128 per tree), and a 112 KiB evidence payload. Each step
revalidates the registration and expected archive. The final observation
checks that the project remains registered with its reviewed sessions removed.
Project registrations, root directories, and Desktop preferences are outside
the operation.

## Claude Code

Project JSONL with recognized location and embedded session identity supplies
bounded metadata on all supported Rust build platforms. Only verified main
conversations appear in the browser. Archive status,
workspace, title and dates may be unknown. The manager exposes Claude Code as
read-only. Desktop installation detection supplies presence and version
observations without promoting sidecar formats into a mutation capability.


Each observed directory directly under the configured `projects` root is a
persistent archive namespace. Namespaces with the same recorded workspace
retain separate identities. Empty observed directories can appear in the
project list. Main transcript identity comes from its filename and embedded
session ID. Nested subagent transcripts, tool-result sidecars, task state and
auto memory have distinct lifecycles and are not exposed as conversations.

Anthropic's documented `claude purge` resolves a workspace path and can
touch history, project-specific trust settings in global configuration,
and state outside the namespace. Its documented interface has a human-readable
dry run. AgentMender has no verified contract to bind that plan to an exact
observed namespace, prove ownership of every sidecar and coordinate concurrent
Claude Code or Desktop writers. All Claude projects stay read-only, including
unknown files and metadata. [Claude Code archive layout and purge](https://code.claude.com/docs/en/claude-directory#clear-local-data).
