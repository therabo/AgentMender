# Two-stage Luna resolution acceptance

2026-11-12. Local branch `feature/two-stage-luna` starts from the verified
local and public `main` at `c9ea7aadaf6d09216142e6333bf98baf57acd015`.

| Commit | Delivery |
| --- | --- |
| `5e38d54` | Exhaustive UUID retrieval, candidate staging, phased MCP contract and fresh publication |
| `87bf4a6` | Separate Planner and Evaluator, host checks, project roles, documentation and benchmark driver |
| `d354b7b` | Validate and correct the Planner payload before opening retrieval |

## Execution contract

`session_resolve` keeps `start` and `advance`. One Luna Medium Planner
produces a validated SearchIntent and closes. Rust enters `retrieving`
and traverses the frozen eligible UUIDs with the existing predicate
evaluator and FTS5/BM25 passage ranking. Planned aliases share the same
traversal and hard constraints. No candidate payload is exposed during
retrieval. In `evaluating`, one distinct Luna Medium Evaluator assesses
the staged batches and handles any explicit single-best selection. The
host closes it before `publish:true` starts `publishing`. Rust verifies
the current source revision before returning a signed row. The terminal
phase is `completed` or `interrupted`.

Traversal, source coverage and conclusive assessment have independent
counters. A structured or UUID-only match can stage a candidate with no
lexical match. Partial history can coexist with completed traversal.
A failed traversal, unavailable Luna or unrecoverable source change has
an explicit interruption. The origin renders the two result tables from
confirmed and plausible unconfirmed rows.

| Area | Files |
| --- | --- |
| SearchIntent and worker DTOs | `crates/domain/src/search_intent.rs`, `session_search.rs`, `session_stream.rs` |
| Application state and evidence checks | `crates/application/src/session_resolution.rs`, `session_resolution/progressive.rs`, `session_search.rs`, `indexed_search.rs` |
| Confined worker and port | `crates/provider-codex/src/content.rs`, `content/cohort.rs`, `crates/provider-api/src/lib.rs` |
| MCP and integration tests | `crates/mcp-server/src/conversation.rs`, `crates/cli/tests/conversational_removal.rs`, `docs/search-intent-v1.schema.json` |
| Luna lifecycle | `scripts/resolve_with_luna.py`, `scripts/test_luna_host.py`, `scripts/check_semantic_assessment.py`, the Planner and Evaluator files under `.codex/agents`, and the project skill |
| Safety and benchmarks | `scripts/check_architecture.py`, `scripts/benchmark_progressive_resolution.py`, `scripts/probe_search_invalidation.py`, [ADR-0036](../adr/0036-two-stage-luna-resolution.md) |

The existing worker-private anonymous SQLite file stores frozen metadata
and two optional blobs per staged UUID. A candidate blob holds ranking,
query observations and up to two sanitized 320-character passages with
positions and source revisions. This preserves the ranked passage across
compressed source reads. Pending receipts contain a bounded assessment and
source-bound anchors. A row has a 20 KiB limit and queue pages have at most
five rows. SQLite uses a 512 KiB page-cache target and short writes. The
worker retains twelve scan replies for retry after an interrupted advance.
The application keeps the current batch and one replayable response.
The spool closes on completion, cancellation, ten minutes of inactivity
or worker exit. Long single-best comparisons can renew the active lease.

## Before and after on generated data

Identical generated JSONL corpora on macOS arm64. The baseline executable
came from `c9ea7aa`. Assessment was scripted for both runs. Each value is
one sampled run, with initialization and fixture creation outside the
timed section. The first signed result on the new path comes after all
assessments. These measurements do not establish a production latency SLO.

| Sources / candidates | Inspected / evaluated after | MCP calls before → after | Total before → after | First signed row before → after | Worker peak RSS before → after | Final spool |
| --- | --- | --- | --- | --- | --- | --- |
| 105 / 105 | 105 / 105 | 22 → 65 | 221 → 316 ms | 57 → 223 ms | 10.9 → 12.0 MiB | 516 KiB |
| 539 / 43 | 539 / 43 | 10 → 28 | 186 → 238 ms | 55 → 202 ms | 11.1 → 11.9 MiB | 388 KiB |
| 1,000 / 1,000 | 1,000 / 1,000 | 201 → 617 | 1.67 → 2.94 s | 45 ms → 1.98 s | 11.5 → 12.4 MiB | 4.55 MiB |
| 10,000 / 10,000 | 10,000 / 10,000 | 2,001 → 6,167 | 17.33 → 32.39 s | 237 ms → 22.12 s | 13.7 → 14.1 MiB | 45.39 MiB |
| 539 / 0 | 539 / 0 | 2 → 1 | 125 → 125 ms | — | 10.4 → 10.6 MiB | 212 KiB |

All emitted UUID sets matched fixture ground truth with no duplicates. On
the 10,000-row run, the new path took 5.99 s to retrieve, 16.12 s for
scripted assessment and 10.28 s to publish. Child CPU went from 19.54 s
to 35.39 s. Parent peak RSS was 16.1 MiB before and 15.1 MiB after.
The additional calls page staged candidates and revalidate signed results.
For 539 / 43, the new phases took 132 ms, 60 ms and 42 ms respectively.

Reproduce the sparse fixture with
`python3 scripts/benchmark_progressive_resolution.py <binary> --scenario exhaustive --count 539 --matched 43 --repeats 1 --sample-rss`.

## Actual Codex App Server

The bundled Codex CLI `0.167.0-alpha.4.1` ran against disposable provider
data. The host observed two distinct `thread/started` identities and
resolved `gpt-6-luna` with medium effort for both. Nested agents were
disabled and a reroute or extra thread would interrupt the run. The
Planner exited before the first retrieval request. The same Evaluator
thread completed nine assessment batches and exited before publication.
[Codex App Server protocol](https://learn.chatgpt.com/docs/app-server)

| Phase | Elapsed | Thread starts | Turns | Tokens reported by host |
| --- | --- | --- | --- | --- |
| Planner | 5.149 s | 1 | 1 | 15,346 |
| Rust retrieval | 0.201 s | 0 | 0 | — |
| Evaluator | 76.631 s | 1 | 9 | 182,270 |
| Rust publication | 0.070 s | 0 | 0 | — |
| Total | 82.051 s | 2 | 10 | 197,616 |

Rust inspected all 539 eligible sources, admitted 43 lexical candidates
and ran FTS5 ranking in 43 sources with multiple ranked passages. All
43 staged UUIDs were judged and confirmed. The final result contained
43 signed rows with exactly the fixture UUID set and zero unresolved
candidates. The run used 28 MCP resolution calls. Child CPU was 3.33 s
and the largest direct child peak RSS was 123.8 MiB. The worker spool
peaked at 364 KiB. The two table sections were verified against the
expected UUID set. The empty 539-source fixture finished with one
Planner, one Rust call, zero candidates and no Evaluator.

Initial exploratory runs exposed malformed semantic JSON. The controlled
host now validates a Planner draft before retrieval and can ask the same
Evaluator to correct a rejected batch. A fixture that provided only a
retrospective claim produced 43 unconfirmed assessments. The measured
acceptance above used original synthetic implementation evidence. Raw
host traces and fixture data remain outside Git.

## Tests and remaining scope

The locked workspace suite on macOS passed 580 tests with eight opt-in
tests ignored. Clippy with warnings denied, locked build, formatting,
architecture, lifecycle evidence and safety checks passed. The host has
11 tests and the semantic validator has six. The generated mutation probe
completed 15 scenarios. Integration tests cover source changes before and
during evaluation, stale publication replay, missing and partial history,
distant context, planned aliases, retrospective rejection, UUID reopening,
interruption, caller binding and read-only provider state. A worker test
covers idle expiry.

Desktop lifecycle remains unvalidated. The App Server run uses the
bundled executable and checks its protocol events in a disposable host.
The feature branch was kept local. GitHub Actions on the public baseline
recorded seven Windows provider test failures. The feature branch has no
new Windows CI run. [Public baseline CI](https://github.com/therabo/AgentMender/actions/runs/43207261014)
