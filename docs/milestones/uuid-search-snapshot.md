# UUID snapshot for progressive conversational search

November 12, 2026 · Europe/Rome

This revision starts at `77b0975` on `feature/progressive-retrieval`.
The decision is recorded in [ADR-0035](../adr/0035-uuid-search-cohort.md).
The previous [invalidation investigation](../research/search-index-invalidation-investigation.md)
records the historical failure and reproduction.

## Contract and implementation

| Component | Contract |
| --- | --- |
| Snapshot | `start` captures eligible and metadata-unknown rows from one read-only SQLite transaction. The worker excludes the apparent caller and registered protected IDs first. It freezes the eligible UUID cohort, locator, title used during acquisition, project, timestamps and archive state. Metadata predicates use that view for the entire run. UNKNOWN never satisfies a hard filter without supporting observations. |
| Storage | One anonymous temporary SQLite spool per active run. It stores the UUID once and a compact metadata row. No transcript is copied to the spool. A 512 KiB SQLite page-cache target, bounded page reads and four active run slots bound resident inventory memory. The bundled Unix VFS unlinks temporary files on open inside the existing private sandbox. The spool is released on terminal completion, cancellation, opportunistic idle expiry or process exit. |
| Continuation | Native UUID order is fixed. The cursor carries the last consumed UUID, position and any one source checkpoint. Newly created sessions are visible to a later run. Renames, unrelated SQLite changes, WAL checkpoints and changes to filter fields leave the earlier run's cohort intact. A late lexical expansion uses that same cohort and the exact set of UUIDs previously offered. |
| Live sources | The source's physical size and revision are established when its file is opened. A checkpoint can only continue that same revision. A change while scanning discards the affected prefix and records a local coverage gap. Other UUIDs continue. A changed citation at publication can reacquire its exact UUID once. Continued failure leaves it unconfirmed and reports the gap. |
| Publication | Reads only the applicable live metadata and verifies the current locator, opened file, header UUID, source revision and citation position. Plain-file positions are checked against the physical extent. Compressed positions remain decoded offsets from the verified revision. Snapshot project, state and timestamps remain pinned. Current titles and producer labels can be displayed after local verification. |
| Navigation | Discovery `session_context` opens `native_session_id` directly and returns a caller-bound signed target and bounded window. Forward/backward paging and occurrence navigation use signed refs. Opening the UUID again after append starts a fresh source view. Missing or unsafe sources return `session_source_unavailable`. |
| Index | The optional private FTS cache uses live inventory preparation. The progressive cohort deliberately uses the live reader for lexical observation and FTS5/BM25 passage ranking. The standalone diagnostic and index-maintenance contracts retain the existing implementation. |
| Model | One Luna Medium SearchIntent and per-session judgments remain required. The origin retains signed results and renders the confirmed and plausible-unconfirmed tables. The model gate remains enforced by the controlled host. |

### Code paths

- [content/cohort.rs](../../crates/provider-codex/src/content/cohort.rs) contains
  `Cohort::capture`, `Cohort::read` and the worker's exact lookup routes. The
  provider transaction closes before the lexical scan begins.
- [content.rs](../../crates/provider-codex/src/content.rs) shares
  `search_batch` between acquisition routes. The cohort supplies its own
  UUID-ordered metadata and disables the diagnostic batch digest. It
  bypasses the DB/WAL/SHM digest, full SQLite backup, corpus fingerprint,
  inventory fingerprint and comparison against a later inventory.
  `open_provider_db`, `open_transcript`, `verify_evidence` and
  `context_window` retain read-only, NOFOLLOW, fixed-locator, single-link,
  exact-header and local source revision checks.
- [session_resolution.rs](../../crates/application/src/session_resolution.rs)
  owns cohort and run lifecycle. [progressive.rs](../../crates/application/src/session_resolution/progressive.rs)
  owns bounded assessment, backpressure, local refresh and replay. Replayed
  publications still verify the applicable source. Query membership comes
  from the original snapshot.
- [session_search.rs](../../crates/application/src/session_search.rs) checks
  scoped identities, result partitions and signed context windows.
  [conversation.rs](../../crates/mcp-server/src/conversation.rs) adds UUID
  opening to the discovery catalog. Management still requires signed
  references. The evidence HMAC, issuer, caller binding and frame limits remain.
- [check_architecture.py](../../scripts/check_architecture.py) permits the
  two fixed spool writes in the confined worker and guards the resolver
  against returning to live-inventory acquisition. Approval, recovery and
  lifecycle capabilities remain governed by their existing controls.

## Benchmarks

The before binary was built at `77b0975` and the after binary at `c233209`. Both runs used disposable
synthetic provider roots, locked macOS arm64 debug binaries, one replicate
per size and the same scripted judgments. Latency covers MCP retrieval and
publication. CPU includes child processes and the RSS sampler. RSS is the
peak sampled separately for the MCP parent and confined worker.

| Sessions | Confirmed before → after | MCP calls | First result ms before → after | Total s before → after | CPU s before → after | Parent / worker RSS KiB before → after |
| ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 105 | 105 → 105 | 22 | 55.38 → 50.96 | 0.266 → 0.210 | 0.328 → 0.264 | 16,336 / 11,200 → 16,240 / 11,184 |
| 1,000 | 1,000 → 1,000 | 201 | 94.31 → 45.61 | 4.586 → 1.654 | 5.296 → 1.900 | 16,160 / 12,240 → 16,480 / 11,936 |
| 10,000 | 10,000 → 10,000 | 2,001 | 692.10 → 219.83 | 288.142 → 17.588 | 333.378 → 19.712 | 16,384 / 24,864 → 16,304 / 14,256 |

| Sessions | Provider metadata bytes hashed before → after | Full SQLite backups before → after | First windows opened on the new UUID route |
| ---: | ---: | ---: | ---: |
| 105 | 4,472,832 → 0 | 1 → 0 | 105/105 |
| 1,000 | 268,697,600 → 0 | 1 → 0 | 1,000/1,000 |
| 10,000 | 26,083,328,000 → 0 | 1 → 0 | 10,000/10,000 |

The new navigation control opened each fixture UUID in one call and verified
a returned passage and forward continuation. The prior signed-reference
control opened three sampled sessions per corpus in three calls each, including
acquisition and publication. It established 3/3 for every size. The latency
tables exclude these separate navigation controls and model latency.

```sh
python3 scripts/benchmark_progressive_resolution.py /absolute/path/to/binary --scenario exhaustive --count 10000 --repeats 1 --sample-rss
python3 scripts/benchmark_progressive_resolution.py /absolute/path/to/new-binary --scenario navigation --count 10000
python3 scripts/benchmark_progressive_resolution.py /absolute/path/to/old-binary --scenario navigation --count 10000 --signed-sample
```

## Acceptance and limits

The disposable integration tests exercise two concurrent runs while adding a
chat, renaming a session, changing project and archive metadata, appending to
a rollout, writing SQLite data and truncating WAL. Both runs finish with
their own UUID membership and zero metadata hashing or full backups. Calls
interleave on the single confined worker.
Independent navigation after completion covers forward/backward paging,
a distant occurrence, a stale continuation after append, missing history,
caller isolation and an out-of-root locator. The source-level regression
changes a rollout mid-checkpoint and verifies that the reader drops the old
prefix and advances to the next UUID. Existing fixtures cover partial,
compressed and missing histories, same-byte replacement, signed evidence,
late expansion, Boolean queries, structured filters and `single_best`.

The locked Rust workspace suite recorded 579 passing tests and child harness
cases, zero failures and eight existing opt-ins skipped. Formatting, Clippy
with warnings denied, the locked workspace build, architecture fitness,
both lifecycle evidence guards, six host tests and six semantic-validator
tests passed.

An installed Codex app-server control observed GPT-6 Luna Medium complete
one plan and one two-candidate assessment batch against the UUID resolver.
Both rows were confirmed and rendered in the two tables. The model run and
performance fixtures used synthetic histories. Native graphical Desktop
acceptance remains open. The worker remains subject to per-operation
deadlines and available temporary disk space. A source whose revision
continues changing can leave an explicit coverage gap. Reopening by UUID
observes the current source, while the search itself retains its original
metadata cohort. A locator that no longer resolves through the confined
fixed counterparts can also leave a local gap.
