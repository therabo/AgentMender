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

## Codex removal profile

Native removal is admitted on macOS arm64 for the reviewed
`0.169.0-alpha.11.1` executable with SHA-256
`3ea20787e106af94573d3d6c19480aed93a1334b0dd32b6b04e2f780ce82cf4b`.

The reviewed path requires the canonical archived row and an exact matching
plain archived rollout. Changed identities, configured protections, unsupported
rollouts, a second compressed representation and observed outgoing spawn edges
cause refusal. The edge check includes children absent from the thread table.
Archive state describes retention. The exact native binary supplies writer and
reference checks. Independent provider writers can still act between the final
observation and the command's own checks.

The confirmed command is the fixed `codex delete UUID --force` in the original
root with a fixed `sqlite_home` override, kernel write scope and network denial.
It runs once with a 180-second deadline. A nonzero result or timeout leaves the
outcome uncertain. Exit zero can request the existing best-effort reset of
`last_full_reconciled_at` for the matching local Desktop catalog. That reset
requires compatible ordinary tables, no relevant trigger or foreign-key
relationship, and a validated local row. It leaves catalog entries in place.

A command result does not establish Desktop disappearance, complete artifact
removal or reclaimed physical bytes. Scope is limited to the selected native
operation and the prior guarded marker. The [fixture test](local-session-manager.md#test-with-disposable-archives)
proves target removal and retained-session preservation in the reviewed case.

## Claude Code

Project JSONL with recognized location and embedded session identity supplies
bounded metadata on all supported Rust build platforms. Archive status,
workspace, title and dates may be unknown. The manager exposes Claude Code as
read-only. Desktop installation detection supplies presence and version
observations without promoting sidecar formats into a mutation capability.
