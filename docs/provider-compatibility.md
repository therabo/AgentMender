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

### Codex workspace groups

The exact absolute workspace spelling comes from the scoped thread archive.
Grouping works for a missing on-disk workspace. Ambiguous path spellings or
missing metadata go to **Senza progetto**. Workspace paths are never opened
for removal. A complete preflight checks canonical locators, workspace
agreement with the rollout header for selected main conversations, orphan
rollouts that could contain more main conversations and all spawned trees.
The scan is bounded to 16,384 records per collection, eight seconds per
worker observation, 32 main conversations and 256 total members (128 per
tree). The worker also enforces a 112 KiB project evidence payload limit.
Large or ambiguous archives can still support individual removal where a
single-tree review succeeds.

The published CLI contract and state schema store threads with a workspace
field. They expose UUID deletion and no separately removable workspace
object. The Desktop reconciliation contract is for a derived thread catalog.
Desktop workspace registrations and UI preferences have no verified deletion
contract here and are excluded. The Codex group disappears from AgentMender
when its observed main conversations are gone.
See the [Codex CLI](https://github.com/openai/codex/blob/rust-v0.169.0/codex-rs/cli/src/main.rs)
and [state migrations](https://github.com/openai/codex/tree/rust-v0.169.0/codex-rs/state/migrations).

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
