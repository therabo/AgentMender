# AgentMender — P0-SR5.2 Single-Search Simplification & Optimization Acceptance

November 11, 2026 · Europe/Rome

The starting local and remote main was
`b6b5dd442743b0025648ec15952c9b1f01e7c493`. The release receipt records the
final pushed SHA, executable digest and GitHub Actions observation.

## Pipeline and code changes

Before this milestone, the resolver could already finish with verified
matches from an incomplete scan. The audit found that hosts often continued
scanning, re-targeted IDs in fresh runs, or fetched context for candidates
they never assessed. In the audited Cesium turn, four ID-specific restarts
followed the initial search. They did not reuse the original run's evidence
or observations. The model was free to do this, and the server had no
authenticated user-turn key with which to identify equivalent starts.

The [ordinary workflow](../intent-aware-resolution.md#originating-chat-workflow)
now states the preferred route in one place. An exact title, ID or inventory
lookup uses metadata discovery. For a semantic question, one Luna Medium
origin or delegate interprets SearchIntent v1 and assesses available
passages. The origin sends one start, assesses the first useful batch and
finishes a sufficient standard result immediately. Context addresses a
specific evidence gap. Explicit exhaustive requests continue within the
existing budget. Expansion remains optional after a completed first scan,
with the original constraints and work budget. Each request starts
independently.

| Area | Consolidation |
| --- | --- |
| Expansion | MCP validation produces the same bounded lexical expression the application consumes. The application still checks the action against the live run, original terms and frozen constraints. |
| Intent and cursor state | V1 transfers its compiled predicates into the run instead of retaining a second compiled plan. Expansion moves the original filter into its retained slot. Structural equality replaces JSON serialization for identical queries and stalled continuations. |
| Publication | The selected pages keep candidate indices. Finish clones metadata, judgments and citations for results on the requested page after fresh checks. It no longer prebuilds payloads for all selected candidates or looks each candidate up again by identity. |
| Worker metadata | Both inventory passes use the same eligibility helper. Fingerprinting uses the existing candidate and prepared ranking terms. Initial and fresh reads use the same bounded snapshot function. |
| Worker passages | Prepare the FTS query and required terms per page. Use the existing relevance observation for evidence selection instead of lowering the same message per search term. Skip transient FTS for slices with no evidence to rank. BM25 still uses the full slice when ranking passages, and comparison keys are computed once per excerpt. |
| Instructions | Project and skill guidance point to the workflow. The Luna role describes its planning and assessment tasks. MCP retains concise routing, caller ownership, one-run, early-finish and signed-publication guidance for clients outside the repository. |

The public catalog keeps the same 16 tool names, schemas and annotations,
including the legacy AST. Caller exclusion, scope and registration binding,
read-only and NOFOLLOW access, exact source identity, cursor fingerprints,
same-revision evidence, and the fresh finish probe remain enforced. Search
does not grant lifecycle authority. Fresh verification still uses the exact
selected IDs and source revisions.

The two implementation commits change 14 files (307 insertions, 381
deletions, including workflow documentation and the regression test).

## Coverage and publication

An assessed candidate has passed semantic input checks. AgentMender
verification requires a fresh result in `finish.matches` with evidence
references. Alternatives remain alternatives. Standard finish can publish
fresh matches while evidence or scan coverage is incomplete. The fixture
publishes one signed result while other candidates remain unassessed,
then reuses the same run for the rest. Stale sources are excluded at finish.
Work-limit, freshness and unassessed gaps appear in progress. An expired
run cannot be continued. Physical EOF of paginated history still leaves
logical coverage Partial. Explicit exhaustive requests retain positive
matches and report the limits of eligible semantic coverage.

The originating chat owns the MCP connection, including when Luna runs
as a delegate. The project authorization and explicit Luna Medium fallback
are still available. Missing, discordant or unmapped caller metadata
refuses discovery. Arbitrary host prose remains outside server enforcement.

## Independent CLI matrix

The installed Codex CLI ran each of the eight specified prompts in a fresh
chat. Both phases used medium parent effort, read-only approval settings,
the same working directory per question, and the existing Codex MCP
registration with only the executable path overridden to pin its hash.
No prompt supplied a target ID, prior answer or workflow instruction.
The live root could change between and during runs. Both matrices ran
concurrently and shared the machine with controlled probes. Counts below
are observed calls, including refusals.

| Question | Calls before → after | Starts before → after | First retrieval before → after | Total before → after | Signed or metadata outcome |
| --- | ---: | ---: | ---: | ---: | --- |
| Cesium rendering investigation | 12 → 4 | 2 → 1 | 384 → 678 ms | 86.2 → 36.9 s | The baseline signed the nominated match. The first after run stopped stale before verifying it. |
| Earth View camera bounds | 1 → 1 | 1 → 1 | 2,733 → 1,387 ms | 30.3 → 32.1 s | Neither turn finished. The after turn cited repository files. |
| ISS telemetry implementation | 15 → 3 | 3 → 1 | 754 → 789 ms | 124.5 → 37.3 s | The baseline signed the nominated match. The first after run only surfaced a retrospective lead. |
| PostgreSQL bootstrap | 2 → 2 | 1 → 1 | 750 → 964 ms | 28.1 → 33.1 s | Both finished with zero signed matches. The after turn cited the historical benchmark separately. |
| Related macOS permission fix chats | 15 → 4 | 3 → 1 | 631 → 591 ms | 179.2 → 80.9 s | The after turn signed two related lookups from its first batch. One assessment needed correction. |
| MISSION1 stored under other labels | 8 → 3 | 1 → 1 | 5,451 → 5,265 ms | 69.3 → 34.9 s | Both initial intents omitted the explicit exclusion. The after turn stopped stale with zero signed results. |
| Title “Benchmark ISS tracker” | 6 → 7 | 0 → 0 | Metadata only | 26.0 → 26.6 s | Both found the closest title “Benchmark ISS tracker rendering.” The after turn reported the exact-title mismatch. |
| All substantive Cesium discussions | 6 → 4 | 1 → 2 | 561 → 727 ms | 44.8 → 34.1 s | The after turn's first start used an invalid coverage field. Its corrected start omitted the exhaustive goal. Neither phase signed a result. |

The matrices recorded 65 → 28 MCP calls, 12 → 8 start attempts, 26 → 4
next calls, 7 → 1 context calls, 5 → 2 assessment attempts, 1 → 0
expansions and 8 → 6 finishes. The after phase made one accepted start
per semantic question, down from 11 accepted starts in the baseline. Candidate
batches fell from 37 to 11. The second attempt in the exhaustive case was a
schema correction before a run existed. Less conversational work came
with less retrieval and two lost signed outcomes in this matrix, so the
turn durations do not demonstrate an equal-effectiveness speedup.

The baseline first signed Cesium and ISS at 78.9 and 118.3 seconds. The
after permission query first signed at 71.5 seconds. The largest sampled
worker RSS was about 117 MiB before and 78 MiB after. The scans covered
different work. Exact provider-page, decoded-byte and candidate-batch
counters are in the receipts. Retrieval milliseconds measure the worker
calls reported by the initial response. They exclude prior model and
host time.

Delegate activity was visible in three of seven after semantic chats.
The role-path summaries do not identify the exact model on every run.
Some hosts completed without a signed resolver match or without
finalizing. Some gave independently sourced answers or retrospective
leads. No observed after turn explicitly called an unsigned candidate
AgentMender-verified, but some presented an unsupported association as
the answer.

Two intermediate serial retries explicitly used Luna Medium as the
originating model. The Cesium turn stopped after retrospective leads;
the ISS turn signed a retrospective as the original implementation.
The signature bound the evidence to the source, while the model's
semantic classification was wrong. The follow-up implementation
clarifies that distinction and points failed assessments to the right
schema. Four targeted repeats of the original prompts on the delivery
binary also used Luna Medium as the originating model:

| Question | Calls / starts / finishes | Wall time | Result |
| --- | ---: | ---: | --- |
| Cesium investigation | 6 / 1 / 0 | 63.4 s | Five failed assessments. The host paired the nominated thread ID with the wrong title without a signed match. |
| ISS implementation | 4 / 1 / 1 | 38.6 s | Correctly separated a feasibility discussion from implementation; no implementation verified. The scan stopped after a source change. |
| MISSION1 under other labels | 3 / 1 / 0 | 29.3 s | Dropped the explicit exclusion again and found no cross-label result. |
| All substantive Cesium discussions | 16 / 2 / 1 | 125.2 s | Both starts encoded the exhaustive goal. The host attempted expansion twice, restarted and stopped with zero signed matches. It described the coverage limit and cited a substantive lead from a later summary. |

The original matrices used the baseline and the first after binary.
These four targeted repeats used the final binary. None of the repeats
produced a signed match, so they establish neither reliable semantic
publication nor consistent adherence to a single run. The title lookup
remained metadata-only in the original matrix.

## Local checks and client surfaces

The locked workspace test suite recorded 570 passes and eight ignored
opt-in cases. Formatting, Clippy with warnings denied, the workspace build,
architecture, semantic assessment, compatibility and lifecycle evidence
checks passed. The test build disabled incremental compilation and
stripped debug symbols to fit the available disk space. A separate normal
build matched the executable pinned in the after trials. Its actual MCP
catalog had the same schemas and annotations as the baseline and matched
the checked-in SearchIntent schema.

The controlled replays used the installed headless App Server with the
same MCP registration and normal caller metadata. That evidence is
separate from the CLI and Desktop. The current Desktop chat still
advertised an older resolver description. No fresh graphical Desktop
acceptance was established.

## Fixed-scope acquisition and latency

The headless acquisition control replayed the original V2 C1–C4 predicates
with an observed MISSION1 label and the frozen 353-ID inventory cohort.
The filters were identical across versions. Nominated IDs and evidence
were scored after acquisition. The same five nominated query–session
pairs were found before and after. Physical traversal of each successful
full scan reached the same 91 sources and retained Partial logical history.

| V2 control | Candidates before → after | Provider pages before → after | Decoded bytes before → after | Wall time before → after |
| --- | ---: | ---: | ---: | ---: |
| C1 Cesium, serial pair | 39 → 39 | 137 → 137 | 2,068,861,091 → 2,068,861,091 | 55.439 → 55.397 s |
| C2 camera, completed retries | 21 → 21 | 136 → 136 | 2,068,836,515 → 2,068,836,515 | 120.531 → 60.159 s |
| C3 ISS telemetry, concurrent | 51 → 51 | 138 → 138 | 2,068,885,667 → 2,068,885,667 | 62.4 → 122.2 s |
| C4 PostgreSQL, concurrent | 32 → 32 | 138 → 138 | 2,068,885,667 → 2,068,885,667 | 56.8 → 83.1 s |

The completed runs returned the same 143 query–candidate pairs, with
identical passage payloads after removing per-registration signatures.
Both initial C2 scans stopped stale after 16 pages and five candidates,
including the nominated target. An intermediate after retry also stopped
stale. The completed final C2 retry made 12 calls against 22 in the
completed baseline, with different concurrent load.

The two serial C1 replays each made 13 calls and took about 55.4 seconds.
The concurrent C1 after replay took 118.5 seconds with the same provider
work as the baseline. C3 and C4 also took longer in the after wave, which
overlapped workspace compilation and client trials. These timings establish
neither a full-scan speedup nor an isolated slowdown. The code removes
work per page and avoids FTS builds for empty evidence. Decode volume,
repeated fresh snapshots and broad traversal remain the dominant
opportunities for further measurement.

## Readiness

The code, local gates, tool contract and controlled acquisition regression
passed. The independent CLI matrix shows fewer calls and accepted starts,
with weaker signed results. A repeatable wall-time improvement for
equivalent work and consistent Luna intent, judgment and signed
publication remain unproven. This is ready for further integration
testing; full conversational and graphical Desktop acceptance remain
open.
