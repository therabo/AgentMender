# UI-first runtime cleanup and acceptance

Date 2026-11-16. The starting checkout was clean on `main` at
`0b8ec50859771480fc142d095d3c55143faee3ad`. The baseline locked Rust build,
592 Rust tests, three Vitest tests, both temporary-archive HTTP checks and the
exact native fixture passed. Eight pre-existing Rust tests were ignored.

## Removal and retained behavior

The [reachability audit](../audits/2026-11-16-ui-first-cleanup.md) maps the
browser entry points through application queries, provider reads and native
dispatch. It also records the Session Steward review.

- Removed the MCP server crate and CLI registration, Luna host and role assets,
  `session_find` jobs, search intent/AST, Context windows and FTS5 persistence.
  The only UI consumer used sequential transcript pages. That reader now uses
  typed provider-local cursors and the existing confined snapshot and decoder.
- Removed the persistence crate, transaction and approval schemas, graph and
  integrity analysis, manual scoped cleanup, historical Desktop mutation
  candidates and their CLI commands. The browser issues and consumes its own
  short-lived receipts. The Codex native provider only accepts reviewed
  one-session evidence and preserves the existing guarded reconciliation marker.
- Flattened session metadata and preserved exact provider, archive root and
  provider namespace. Both inventory readers, Unicode search, sorting, paging
  and explicit coverage remain available. The Claude adapter keeps bounded
  project JSONL parsing and installation observations.
- Trimmed the HTTP/React data contract to fields used by inventory and detail.
  Kept the visual layout and interaction model. Replaced historical CI and
  architecture guards with checks over the current runtime and temporary
  fixtures.

The [current architecture](../architecture.md) and [ADR 0043](../adr/0043-ui-first-runtime.md)
describe responsibilities, dependencies and read/write boundaries.

## Quantitative change

| Measure | Result |
|---|---|
| Tracked files removed, added, and modified | 442 removed, 12 added and 36 modified |
| Lines added and removed | 3,533 added and 178,111 removed |
| Rust workspace members | 8 to 6 |
| Direct crate dependency edges, including development and target-specific names | 87 to 39 |
| Names completely absent from the new direct dependency set | `agentmender-mcp-server`, `agentmender-persistence`, `chrono-tz`, `hmac`, `proptest`, `rmcp`, `rusty-leveldb`, `schemars`, `snap`, `winapi-util`, `windows-permissions` |
| Legacy fixture and compatibility tree | 44 fixture files and 135 compatibility files removed. The active Desktop reconciliation fixture retains only its two relevant tables. |
| Retired test and benchmark artifacts | 37 tracked test-tree files and seven dedicated test or benchmark scripts removed. Retained decoder, read-scope, version and Desktop marker checks cover the surviving code. |

Move counts use Git's add/delete view. Historical data remains in the existing
Git history and archival refs. The npm dependency set and lockfile carried
forward after checking that the UI and tests still use them.

## Validation

| Check | Result |
|---|---|
| macOS arm64 Rust | Formatting, locked workspace Clippy with warnings denied, tests and build passed. 31 tests passed. SQLite compile options confirm FTS5 is absent. |
| Linux arm64 Rust in Docker | Formatting, locked workspace Clippy, tests, build, architecture check and portable HTTP acceptance passed. 19 tests passed. Codex reports the existing macOS reader requirement. |
| Windows x64 GNU target | Locked cross-target `cargo check --all-targets` passed from Linux. Windows runtime and native GitHub CI were unavailable for this local branch. |
| Frontend | Locked npm install, three Vitest tests, TypeScript and the Vite build passed. |
| Browser fixture | Chrome desktop and mobile passed on isolated data. Inventory, filtering, paging, detail, transcript and removal review worked. Axe found zero violations in the checked views and the console contained zero errors. |
| Inventory and browser boundary | Portable HTTP fixtures verified both providers, duplicate UUID isolation across roots, 3,000 Codex rows, Unicode and UTC filters, pagination, unknown fields, missing and malformed archives, rejection of unconfined reader launch and authorization guards. |
| Transcript | Bounded UTF-8 and Zstandard pages, current locator and revision, same-session cursors, wrong-header and stale refusal, paginated coverage, oversize records and secret/internal message suppression passed. |
| Exact native Codex fixture | The validated binary removed the selected archived rollout and canonical row. Byte comparisons preserved unrelated sessions. Protected and active sessions, known and orphan spawn edges, stale receipts and forged approvals were refused. Simultaneous duplicate confirmation produced one acceptance. A nonzero exit remained uncertain and replay was refused. |
| Repository checks | Architecture guard and `git diff --check` passed. |

All provider tests used disposable archives. The final refactor stays on
`refactor/ui-first-cleanup`. The precise final commit is in Git.

## Remaining limits

Codex live reads require the attested macOS worker. Native removal is limited
to the reviewed executable on macOS arm64. Claude Code discovery remains
partial and read-only. Some closed WAL stores can be unavailable when their
SQLite coordination files are absent. The reader has no authority to create
those files. Native command status leaves physical reclaim and full artifact
or Desktop disappearance unverified.
