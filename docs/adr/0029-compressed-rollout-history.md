# ADR-0029: Read compressed native rollouts without materialization

## Status

Accepted, 2026-11-09, P0-SR2. Extends the existing provider-owned read path.
P0-SR3 [ADR-0030](0030-resident-bounded-history-decoding.md) supersedes only the
total compressed-size/cumulative replay strategy; the original receipt follows.

## Evidence and decision

The four real acceptance targets have metadata locators ending in `.jsonl`, no
plain file, and an existing exact `.jsonl.zst` counterpart. A kernel-confined,
network-denied read also found their persisted items in `thread_history_1.sqlite`.
Official Codex 0.166 source implements compressed rollout fallback and streaming
Zstandard decoding; the installed desktop 26.1103.41825/build 25679 embeds CLI
0.167.0-alpha.3.2 and pages turns/items through App Server. These are local
representations, not evidence that remote authentication is needed for these reads.

Admit `.jsonl.zst` as a representation of the same selected rollout, inside the
existing attested content worker and existing sessions/archive roots. A missing
plain locator first tries its exact compressed counterpart. Relocation retains
the fixed counterpart set and refuses multiple candidates. An existing selected
representation is never replaced by a title/UUID scan or silently merged with
another history. Verify the decoded header's exact UUID and retain physical
source revision checks before/after reads and in cursors/context.

Decode in memory as a stream, never materialize files under provider state.
Positions are decoded JSONL offsets; source length/revision describe the physical
representation. Evidence formats explicitly identify Zstandard. Seeking replays
bounded decoding from the beginning; cap compressed files at 64 MiB, decoder
window at 8 MiB and decoded work per reader (including replay) at 128 MiB, under
the existing worker deadline. Existing parser/output/session/search budgets stay
in force. `bytes_read` measures parsed decoded JSONL bytes, not replay/decoder
read-ahead or compressed physical I/O. Replay is separately bounded, not free
unbounded work. Limit/corruption errors are closed diagnostics and cannot prove
absence. An incomplete paginated display stream remains Partial at physical EOF.

## Alternatives and trust boundary

Do not start a live App Server merely to read history: initialization/maintenance
and lazy projection can write provider-owned state, while generated method names
only prove availability. Existing isolated native experiments remain evidence,
not live startup admission. Do not borrow the host's connection or credentials.
The host `read_thread` tool corroborates visible messages, but is not an attested
ingestion channel for AgentMender. The official TUI `/export` command exports
Markdown, but supplies no documented machine-verifiable identity/revision or
complete-source attestation. It could support a future explicit untrusted import;
no import is needed or added here, and no session is resumed to produce one. Direct history
projection SQL is deferred: its selected-lineage/projection freshness semantics
need an independent contract and the real database exceeds the existing bounded
backup. Compressed rollouts avoid that new database/trust surface.

No new port, query engine, database, caller binding, lifecycle authority or
default persistence is introduced. Existing FTS5, context, occurrences, typed
filters, privacy suppression and Luna evidence rules consume the same messages.
Real paging exposed signed evidence-bearing cursors exceeding the old 4 KiB
input limit. Accept up to 32 KiB under the same HMAC/process/kind binding and
existing request/response limits; test emitted cursor round trips and oversized
input refusal. This grants no new authority or source access.

## Verification

Test compressed legacy/paginated messages, decoded offsets, continuation/context,
corruption/truncation, expansion/window/work bounds, foreign headers, links,
relocation ambiguity, source replacement and cache freshness. Replay the exact
330 identities and four real targets through AgentMender MCP. Keep native-client
corroboration, actual MCP parent acceptance and private receipts separate.

Sources: [official compression implementation](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/compression.rs),
[history projection](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/thread_history.rs),
[App Server](https://learn.chatgpt.com/docs/app-server).
The [official developer commands](https://learn.chatgpt.com/docs/developer-commands)
document `/export`.
