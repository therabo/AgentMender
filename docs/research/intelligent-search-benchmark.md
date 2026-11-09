# Intelligent retrieval evaluation receipt

2026-11-09, sanitized reproducible corpus. [Raw aggregate measurements](intelligent-search-evidence/benchmark.json).
Hardware: Apple M1 MacBookAir10,1,8GiB RAM, macOS/aarch64. SQLite3.54.1,
Rust debug profile without debug info/incremental artifacts. No private transcript
text or provider IDs occur in these fixtures.

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo run --locked -p agentmender-persistence --example search_benchmark
/usr/bin/time -l target/debug/examples/search_benchmark
```

Dataset:1000 sessions,40000 messages,15860370 text bytes. Ten relevant sessions
are IDs divisible by100; message20 describes Cesium frame drops,35 rejects a
workaround,38 confirms a fix. Other messages concern packaging. The executable
itself generates the complete ground truth and documents its assumptions.

| Measurement | Observed |
|---|---:|
| Initial FTS5 + private text build |5030.34ms|
| Incremental one-message append |1718.96µs|
| Lexical FTS query |1078.0µs|
| Database size |22872064bytes|
| Maximum resident set (`time -l`) |9846784bytes|
| Total process time including corpus generation/checks |5.17s|

| Retrieval strategy |Recall@10|Precision@5|MRR|False-negative rate|
|---|---:|---:|---:|---:|
| FTS5 “Cesium rendering” |1.0|1.0|1.0|0.0|
| Literal semantic wording “sluggish spacecraft display” |0.0|0.0|0.0|1.0|
| Explicit expansion “frame drops rendering bottlenecks” |1.0|1.0|1.0|0.0|

These are deliberately small synthetic lexical evaluations, not universal
semantic recall, a statistical model comparison or end-to-end production latency.
The expansion result demonstrates why literal semantic phrasing can miss a
candidate; it does not demonstrate a general embedding substitute. No vector
retrieval was implemented or benchmarked. Luna's bounded synthetic evaluation is
recorded separately, with no fabricated improvement percentage/token measurement.

P0-S1 head/tail coverage is compared structurally in provider tests: a36MiB source
places occurrences around17MiB and combines facts on different pages. New streaming
completes over roughly four bounded calls (targeted observed13.62s), whereas old
1MiB head/tail windows cannot inspect that occurrence. This is not a reimplementation
of the old algorithm disguised as a production benchmark. Another fixture visits
5000 sessions without lost/duplicate results; the10001-session fixture proves
continuation beyond the former cutoff and rejects earlier insertions.

Exact filter/negative/coverage correctness is asserted in domain property tests,
application index tests and worker fixtures, rather than assigning an unsupported
percentage to a private corpus with no known ground truth. Context and metadata
latency were measured separately in the [real read receipt](intelligent-search-evidence/real-read.json):
calendar156.63ms, negative project157.16ms, content212.35ms, four-message context182.28ms.
Those include the attested worker overhead on this installation and are one sample,
not p50/p95 guarantees. The real source snapshot changed concurrently; no global
unchanged-state or corpus-completeness claim is made. Source writes remained denied.

Persistent-cache update/reuse, EOF after unknown records, chunk gaps/malformed refs,
replacement during build and stale-text removal have deterministic regression
coverage. Cache reconstruction is capped; uncached sources still stream. A1000-file
real-provider index build and model token consumption are **not measured**. The
Luna fixture tracks a simulated context budget423 excerpt bytes/one request; those
are not actual API token accounting or actual MCP calls from Luna.
