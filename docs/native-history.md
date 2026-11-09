# Codex native history sources and read boundary

P0-SR2 investigation, 2026-11-09–10. Installed desktop26.1103.41825/build25679,
CLI0.167.0-alpha.3.2. [ADR-0029](adr/0029-compressed-rollout-history.md) records the
decision; [acceptance](milestones/p0-sr2-native-history.md) separates its proof levels.

## Proven local source

The exact330-identity cohort has16 plain rollouts and314 compressed-only rollouts.
All four nominated missing `.jsonl` locators have exact `.jsonl.zst` counterparts.
Those bytes produce the same observed message/turn IDs and projected visible text
as the installed host's history reader. AgentMender retrieves them with network,
source writes, credentials and private config access denied. Their absence from
the plain-file reader was a representation gap, not proof of provider data loss.

The local `thread_history_1.sqlite` also contains projected items for all four
targets. It was inspected by a separate bounded, read-only, kernel-confined
research probe, not admitted as a production content source. The real database
is about1GiB and exceeds the existing32MiB backup contract. Projection/selected
lineage semantics need separate validation before a future SQL reader.

Installed bundle code routes history reads through `thread/read`,
`thread/turns/list` and `thread/items/list`. Official source describes compressed
rollout fallback and a local projected history store. Host `read_thread` paging
was exercised on a real target, including a subsequent older-turn cursor.
The host's exact per-request choice between cached projection and rollout replay
was not instrumented. No remote-source/authentication requirement is inferred
from the host already being signed in.

## Transport evaluation

| Mechanism | Evidence and decision |
|---|---|
| New App Server process | Non-resuming RPCs exist, but exact installed-binary initialization in fresh owned scratch creates SQLite/WAL/SHM stores, installation identity, skills and plugin synchronization state. With writes denied to that synthetic provider root it exits before serving reads. No live startup admission. |
| `thread/read` / `thread/turns/list` | Official non-resuming history interfaces; real host paging works. On the new default empty store, nonexistent IDs return `thread not loaded`, without login or model execution. This is not a real-session transport PASS. |
| `thread/items/list` | Bundle/schema availability does not prove backend support: the fresh default installed process returns unsupported. The running host has separate configuration/backend capabilities. |
| Trusted host-assisted input | Available host tools return useful corroboration, but no explicit bounded host-attested channel into AgentMender was established. Model-provided text never authenticates a provider source, caller identity or negative result. No connection/credential borrowing. |
| User export | Official TUI `/export` can save Markdown. No documented machine-verifiable session/revision attestation was established. A future explicit import must be separately labeled untrusted and cannot grant destructive or negative-proof authority. No import added and no session resumed to export. |
| Existing compressed rollout | Selected: exact local representation decoding under the already admitted worker, through the same read port/application/MCP. No extra database or native startup. |

## Reader and provenance contract

An existing selected locator wins. If missing, try its exact alternate plain/Zstd
representation first. Then try only the fixed active/archive counterparts already
derived from that locator/calendar prefix, including their compressed variants.
Multiple relocated candidates refuse. No recursive scan, title lookup, root
widening, history merging or shared-lineage reconstruction is added.

Descriptor-relative NOFOLLOW, ordinary single-link files, exact decoded header
UUID and source revision rechecks remain mandatory. Recheck selection as well as
the opened descriptor after reading. Physical replacement, append or selection
change invalidates references. The revision is a hash of physical metadata,
including device/inode/size and high-resolution times, not a cryptographic hash
of every transcript byte or a provider signature.

Legacy visible response items and paginated completed display items share the
existing parser. Evidence formats distinguish:

- `codex-rollout-response-item-message-zstd-v1`.
- `codex-paginated-item-completed-message-zstd-v1`.

Positions/checkpoints are decoded JSONL byte offsets and logical ordinals.
Sealed source length/revision describe the physical file. FTS5, typed filters,
context/chunks, backward/forward/occurrence navigation and optional cache use the
same source-bound messages. No new search engine/graph/embedding or port exists.
Paginated EOF remains Partial; unsupported external segments, suppressed text and
unknown shapes cannot prove absence. Luna's incomplete-evidence guard is unchanged.

## Bounds, privacy and client state

Compressed file size is capped at64MiB, decoder window at8MiB, and decoded work
including replay at128MiB per reader. Decoding streams without whole-file memory
allocation or provider materialization. Existing8-second worker deadline,
12MiB/session slice,32MiB parsed-byte aggregate,8MiB record and output bounds stay.
Context may open one backward probe plus one stream, each bounded under the same
deadline. Replaying prefixes costs work; this is not constant-time random access.

`bytes_read` measures parsed decoded JSONL, excluding decoder read-ahead/replay,
physical compressed reads and metadata backup. It is not total disk I/O. Closed
errors distinguish compressed corruption, physical size and decoded work limits.
They preserve UNKNOWN. Five real cohort files exceed64MiB and remain unavailable.

No decoded file/index is persisted by default. Pattern suppression and inert
evidence treatment precede scoring/projection. Historical tools/reasoning/system
instructions are not searched as visible conversation. Explicit index retention
and client/model processing keep their existing separate privacy choices.

Evidence-bearing signed cursors now accept up to32KiB, within the existing64KiB
request frame. HMAC kind/process binding is unchanged. This corrects a real emitted
cursor exceeding the old4KiB input limit. Parent changes require a fresh MCP parent.

The observed chat still reports the earlier registration instance after a
user-reported connection refresh. The new worker is proven through actual chat
tools; the new parent projection/token handling is separately tested with fresh
direct stdio and fixtures. Do not claim its exact parent build from the executable
currently on disk. Refresh **only AgentMender** through Settings → MCP servers →
AgentMender → Restart, then verify a different registration plus `content_coverage`.
If the chat retains the old parent, leave that acceptance OPEN; never restart
Codex or borrow its transport to obtain a passing label. Guaranteed management
continues to refuse with zero candidates before provider access.

References: [App Server](https://learn.chatgpt.com/docs/app-server),
[official compression source](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/compression.rs),
[projection implementation](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/thread_history.rs),
[official export command](https://learn.chatgpt.com/docs/developer-commands),
[MCP server controls](https://learn.chatgpt.com/docs/extend/mcp).
