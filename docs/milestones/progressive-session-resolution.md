# AgentMender — Progressive retrieval implementation and acceptance

Baseline `8c63ef6ff7af9edf3e5e32d7ba7f801b31b919a2`. Implementation `c862382` and `df9f842` on `feature/progressive-retrieval`. The comparison uses generated Codex histories and a disposable STDIO client.
This is a historical benchmark receipt. Reproduce its old command line
with its original script at the referenced commit. For the current contract,
see [Unified progressive resolver](unified-progressive-resolver.md).

## Architecture

```mermaid
flowchart LR
    U[User request] --> L[Luna interprets SearchIntent]
    L --> S[Origin starts the caller-bound run]
    S --> R[Confined Rust reader finds candidates and passages]
    R --> A[Luna assesses original evidence and gaps]
    A --> V[Origin advances with judgments or windows]
    V --> P[Rust revalidates and publishes a batch]
    V -->|New context or candidate batch| A
    P --> F[Origin accumulates signed results and reports terminal coverage]
```

The eight crates retain their previous boundaries. `application/session_resolution` owns queueing, revisions, bounded replay and coverage. MCP parses and projects `start` and `advance`. The existing provider port performs a batch of read-only evidence verifications. The Codex worker retains a fingerprint-bound SQLite snapshot and one title-prior inventory window. Persistence remains the owner of opt-in derived FTS storage. [ADR-0034](../adr/0034-progressive-resolution.md) · [Wire and Luna workflow](../intent-aware-resolution.md#progressive-workflow)

## Milestones

| Milestone | Implementation and evidence |
| --- | --- |
| **M0 · Baseline** | Isolated local branch, original audit and baseline binary, fixture harness and numeric worker counters. Telemetry covers metadata hash bytes, SQLite backup and reuse, rows, FTS, decoded/replay bytes and publication checks. Host-side sampling covers MCP calls, time to first/terminal results and process resources. |
| **M1 · Outcomes** | Missing content no longer produces the v1 legacy no-match outcome. Progressive status separates inventory, physical, logical and semantic coverage, with typed terminal results. V1 assessment with a session-wide excluded outcome cannot publish a verified match under the current excerpt-level contract. |
| **M2 · Progress** | Progressive mode removes cumulative candidate, provider-page, assessment and duration limits. It holds five active candidates, releases published payloads and uses a monotonic inventory/source position for deduplication. It retries an unfinished source once without resetting inventory commitments. Drift beyond that recovery returns partial. The run lease renews on active steps. Legacy limits remain on the old actions. |
| **M3 · Attribution** | `relationship:direct` is required for progressive positive judgments. `discussion`, `retrospective` and `contradiction` can describe related material. New context invalidates a preceding assessment of that candidate. Updated tool, skill and Luna role guidance retain the original constraints and require attribution in the candidate itself. The tests exercise structured judgments, including a rejected retrospective. Model fidelity in an unprimed live client remains open. |
| **M4 · Orchestration** | Additive `advance` accepts assessments, up to two context windows, an eligible expansion and a termination request. A single call may publish one batch and return the next. The last operation can be replayed with freshness revalidation and explicit invalidations. Worker batch publication validates the current caller, identity, locator, revision, header and retained query observations. Legacy actions retain their response shapes. |
| **M5 · Worker and storage** | Shared snapshot and inventory window on exact unchanged fingerprints. One prepared normalization for relevance and AST substring matching. Typed cursor comparisons, prepared lexical terms, batched FTS inserts and ranked ordinals by lookup. Skip FTS when ordering cannot change the candidate passages. The 10,000-source run shows repeated full-file metadata hashing remains the dominant cost. |
| **M6 · Context and indexing** | Same-anchor overlapping Around windows merge when their aggregate fits the reply budget. Disjoint windows share an MCP round-trip and reuse the fingerprint-bound metadata snapshot when safe. Existing turn IDs and source positions support ordered deep navigation. Private FTS `MATCH` proposes eligible sources for observation reconstruction, with exact checks and live fallback. It preserves the inventory order and is not a global session reranker. Use initial aliases to avoid a second pass. |
| **M7 · Semantic experiment** | Measured local multilingual E5, lexical variants and RRF on an annotated synthetic set. Alias rewriting recovered all judged originals by rank 5, and hybrid added zero recall at that cutoff. The deployed retrieval path stays FTS5/BM25 with Luna assessment. |
| **M8 · Acceptance** | Workspace and fixture gates, STDIO scale checks, comparison benchmarks, updated ADR and workflow. This is local read-only acceptance. Installed CLI/Desktop with actual Luna and real source churn, as well as remote multi-OS CI, remain open. |

## Before and after

The fixture compares the original and progressive binaries on the same generated histories, on macOS 26.6.2 arm64 with an Apple M1 and 8 GiB RAM, using locked debug builds without debuginfo or incremental compilation. Each paired case runs 20 serial repeats. Timing covers MCP operations from the first tool call to the final reply, excluding process initialization and model execution. Ground-truth judgments are scripted. All results and cited reference counts were checked, and the same expected identities were published for each matched workload. The contextual comparison checks the implementation anchor and both later source spans on each run.

| Workload | MCP calls | Terminal p50 / p95 ms before → after | First verified p50 / p95 ms before → after | Result recall |
| --- | ---: | ---: | ---: | ---: |
| Exact ID | 1 → 1 | 12.99 / 17.17 → 16.18 / 38.72 | 12.98 / 17.15 → 16.16 / 38.71 | 100% |
| Simple semantic | 3 → 2 | 18.78 / 21.56 → 15.94 / 16.21 | 18.77 / 21.52 → 15.93 / 16.21 | 100% |
| Two context windows | 5 → 3 | 22.30 / 22.46 → 19.12 / 19.27 | 22.29 / 22.45 → 19.11 / 19.26 | 100% |
| 90 eligible sessions | 54 → 19 | 484.86 / 496.30 → 194.57 / 199.18 | 229.74 / 236.00 → 32.85 / 33.91 | 100% |

The exact lookup sample varied between repetitions and showed no stable latency improvement. For the 90-session case, median child CPU was 426 → 165 ms user and 59 → 30 ms system. These CPU figures are from the fixture subprocess family.

The legacy 90-result case uses 8 acquisition pages, 18 fresh metadata searches and 90 single-result context probes. The progressive case uses 18 acquisition pages and 18 batch verification operations. Its smaller acquisition pages let the client publish earlier while holding only one active candidate batch. The number of bytes parsed and decoded during acquisition is equal at 59,760 and 109,620 respectively. Exact discovery remains on the original MCP path. The benchmark includes no Luna calls or model latency.

### Scale

| Sessions | Unique matches and citations | MCP calls | Provider scan / verification | First verified | Terminal |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 105 | 105 | 22 | 21 / 21 | 34.74 ms | 0.235 s |
| 1,000 | 1,000 | 201 | 200 / 200 | 97.61 ms | 4.781 s |
| 10,000 | 10,000 | 2,001 | 2,000 / 2,000 | 709.38 ms | 313.718 s |

All three runs reported `completed_with_matches`. The separate 1,000-source RSS sample measured a 16.1 MiB MCP parent peak and a 12.2 MiB worker peak, with 50 ms polling. These are separate sampled peaks.

The 10,000-result run crossed the old 96-candidate, 256-page and 48-assessment ceilings. It used one SQLite snapshot and 3,999 reuses. DB/WAL/SHM fingerprint work accounted for 26,083,328,000 bytes. Parsed and decoded scan bytes were 6,640,000 and 12,180,000 respectively. The scale timing is one development sample on the progressive implementation before the final recovery and presentation guard changes. Its executable SHA-256 was `d6cfb9b5f6441a7c8aa2761b71aadbd49a797efe7c7050dd0037eee56d0488e9`. The harness retains each result for uniqueness checks. The server only holds the active batch, one replay and the current bounded inventory window.

### Opt-in private index

| Fixture | No index p50 | Warm index p50 | Acquisition decoded bytes before → after | Result recall |
| --- | ---: | ---: | ---: | ---: |
| 90 short histories, 20 repeats | 194.57 ms | 736.28 ms | 109,620 → 59,760 | 100% |
| 40 histories with 4 MiB of hidden records each, 10 repeats | 624.31 ms | 305.84 ms | 168,282,000 → 327,680 | 100% |

Both variants emitted the same expected source evidence. The warm index excludes construction time. Its observation reconstruction cost dominates on short histories. It saved substantial decode work on the larger fixture. A separate fixture with an unsupported event record produced `partial` with all positives and the index build reported an incomplete result. The deployment default remains live read-only traversal. Indexing remains explicit opt-in.

### Local semantic comparison

The offline set has 12 questions with one original each and 24 hard distractors. The aliases are manually curated to test the effect of intent rewriting. They do not measure Luna's own alias quality. Contextual FTS appends the adjacent result message to the indexed passage for the ablation. Production uses source-bound context at assessment time.

| Approach | P@1 | R@5 | MRR | Warm query p50 |
| --- | ---: | ---: | ---: | ---: |
| FTS5/BM25 with literal terms | 8.3% | 8.3% | 0.083 | 0.031 ms |
| FTS5 with aliases | 66.7% | 100% | 0.819 | 0.022 ms |
| FTS5 with aliases and adjacent text | 16.7% | 100% | 0.528 | 0.021 ms |
| Multilingual E5 small int8 | 33.3% | 75.0% | 0.546 | 3.104 ms |
| Contextual FTS and E5 fused with RRF | 58.3% | 100% | 0.757 | 3.132 ms |

The quantized model is about 118 MB and the process peak RSS was about 478 MiB. Model load and corpus embedding took 769 ms. Query timing excludes that build. Real-history retrieval and model attribution are outside this corpus. Keep alias rewriting and targeted context, with neither dense retrieval nor GraphRAG in the runtime. [Multilingual E5](https://huggingface.co/intfloat/multilingual-e5-small) · [Quantized ONNX files](https://huggingface.co/Xenova/multilingual-e5-small/tree/761b726/onnx) · [RRF paper](https://doi.org/10.1145/1571941.1572114)

## Verification and reproduction

The locked workspace test suite passed with 576 successful tests and child harness cases, zero failures, and eight live or explicit opt-ins ignored. Formatting, warnings-denied Clippy, the workspace build, the architecture guard, both lifecycle evidence guards, six semantic-validator tests and `git diff --check` passed. The STDIO replacement test also passed after adding an exact same-bytes source replacement between acquisition and publication. One stale candidate was excluded and the other four were published.

The Rust fixture tests cover source replacement, stale replay, caller binding, missing sources, window merging, progress monotonicity and bounded source restart. Existing compressed-history cases exercise decoder eviction, replay and physical versus logical coverage.

```sh
cargo build --locked --workspace
python3 scripts/benchmark_progressive_resolution.py target/debug/agentmender --mode progressive --scenario exhaustive --count 105 --repeats 20
python3 scripts/benchmark_progressive_resolution.py target/debug/agentmender --mode progressive --scenario exhaustive --count 1000 --repeats 1 --sample-rss
python3 scripts/benchmark_progressive_resolution.py target/debug/agentmender --mode progressive --scenario exhaustive --count 10000 --repeats 1
```

For the comparison, run the same harness with the baseline executable and `--mode legacy`. Use the same profile, environment and fixture counts. The exact, simple and context scenarios generate one matching session. For the warm index control, use `--index --count 40 --hidden-bytes 4194304 --repeats 10` on the exhaustive scenario and repeat with the same flags excluding `--index`. The default exhaustive workload generates 90. The semantic script uses local `model.onnx` and `tokenizer.json` in a supplied directory, with `numpy==2.5.4`, `onnxruntime==1.31.1` and `tokenizers==0.23.3` in an isolated Python 3.13 environment. Model SHA-256 `4d24e2bc01a447951524466ef533e52944bf48509e6552810bcee1a2711cb02c`. Model and tokenizer are fetched separately from the fixed Xenova revision. The scripts do not open a user's provider root.

## Readiness and limits

- Ordinary `all_relevant` Codex resolution is ready for disposable integration. The run is ephemeral and the consumer retains the emitted result pages. Restart or idle expiry requires a new run. Released pages cannot be fetched again from the server beyond the immediately preceding operation. Explicit `single_best` and advanced Boolean resolution still use the legacy route and its total retention limits.
- Exhaustive terminal coverage requires logical history and evaluation of every inspected eligible session. A lexical miss, a provider gap, an unresolved judgment or a stale revision can leave a partial result even after traversal. Expansion after nonempty acquisition, automatic ranking across inventory windows and arbitrary resumable context across process restart remain outside this implementation.
- The scope and freshness guards stayed in place. Content work is read-only and confined, and provider lifecycle behavior is unchanged. The branch has no real-provider or unprimed Luna acceptance. The structured attribution check cannot prove a natural-language claim if a model incorrectly labels a retrospective as direct.
- Reuse reduces SQLite backup and inventory work. Metadata hashing remains costly on large catalogs. Optimizing it further requires evidence that another observation method detects concurrent changes with equivalent guarantees. The historical SR5.2 CLI matrix reduced calls while losing two signed results. Its fixed-scope control showed unchanged serial full-scan time. This acceptance compares matched generated workloads and makes no claim of a new live MISSION1 speedup. [SR5.2 receipt](https://github.com/therabo/AgentMender/blob/9027e61068a414ee0609c0afa81e51481ffc459f/docs/milestones/p0-sr5-2-single-search-acceptance.md)
