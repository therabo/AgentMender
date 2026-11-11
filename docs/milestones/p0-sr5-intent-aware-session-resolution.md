# AgentMender — P0-SR5 Intent-Aware Session Resolution

**November 11, 2026. Implementation and local verification complete. Full
conversational acceptance remains open.**

The baseline is main at `121518eb30c873e201f1d5d2e51ec73af40fcf0d`. The
macOS arm64 client was ChatGPT Desktop 26.1103.60933 (build 25894) with
bundled Codex CLI 0.167.0-alpha.3.2. The resolver uses the existing confined
read worker, three-valued AST, transient FTS5/BM25, source-bound context,
and apparent-caller exclusion. See [ADR-0032](../adr/0032-caller-scoped-session-resolution.md).

## Evidence by test surface

| Surface | Result |
| --- | --- |
| Implementation | Typed intent, single-pass original/expanded retrieval, optional bounded follow-up expansion, exact candidate and passage binding, multi-result continuation and fresh finalization. |
| Unit and disposable fixtures | Pass. Includes unknown hard filters, cross-caller access, partial/no-progress work, expansion provenance, ambiguous and plural selection, stale source refusal, forged handles, and lifecycle separation. |
| Real provider | Five of five nominated MISSION1 query–session pairs acquired. All four same-scope V2 positive sets preserved. A bounded replay finished 353 historical sources with Partial logical history. |
| Controlled semantic replay | All 44 independently judged pairs reacquired with anchored passages. Luna's final assessments and AgentMender's fresh publications selected the judged target for 20 of 20 needs. This replay supplied the judged candidate pools and passage anchors. |
| Actual conversational client | The installed CLI selected the resolver and project skill from ordinary requests. It returned the verified camera implementation. Ordinary routing from a non-Luna parent did not reliably invoke Luna. The broad plural run did not finish within the evaluation budget. |
| Production | Read-only tooling is available on the supported macOS Codex reader. Complete conversational acceptance, refreshed Desktop SR5 routing, and authenticated management remain open. |

The named project agent was discovered in an explicit installed-CLI
control. Its inherited MCP catalog included discovery. The ephemeral
delegate received a caller-mapping refusal. AgentMender's default handoff
keeps retrieval and navigation in the originating conversation. A separately
selected Luna Medium originating model can access the tools in its own
conversation. Successful invocation requires host support and model availability.

## Retrieval and model results

The four terminal MISSION1 replays covered the same 91 eligible sources. SR5
returned the first known rendering, camera, and ISS telemetry targets in
resolver batches 2, 1 and 1. V2 reached them on search pages 93, 106 and 90.
The two PostgreSQL targets arrived in batches 10 and 11, against V2 pages 87
and 90. SR5's batches combine existing provider pages. The full SR5 scans
used 13–16 tool calls, 136–138 provider pages and 59–86 seconds. V2 used
137 tool calls per query and approximately 56–58 seconds. These runs
demonstrate earlier useful presentation and fewer host calls. Total scan
speed varied with source state and concurrent test load. All source and
passage rankings remain scoped to observed work.

The literal permission query missed the known canary case. One targeted
expansion with EPERM and O_SEARCH recovered it under the same four-source
scope. A full real replay of the oversized compressed source preserved
physical traversal and Partial logical coverage. The synthetic Luna
verification selected the evidence-supported outcomes in three cases
covering prompt injection, tests without a merge, and a later correction.
Luna classified 120 cardinality and outcome variants in a separate
structured intent exercise. That exercise did not run retrieval for each
variant.

A broad Cesium investigation question surfaced several plausible
conversations, including the nominated benchmark. A narrower benchmark
question selected the nominated session with incomplete search coverage.
The ordinary-client integration still needs reliable default delegation
and comprehensive finalization across user intents. The evaluation did
not establish globally exhaustive relevance judgments. No demonstrated
miss required a cross-session graph.

## Reproduce

1. Keep the Benchmark V2 directory immutable. Verify its receipt manifest
   and load its 353 historical inventory IDs, 20 needs, and 44 judgments.
   Keep replay traces and model evidence in a new private directory outside
   Git and the provider root.
2. Build the current executable and register `mcp --session-search` against
   the supported read-only Codex root. Use the installed client's normal
   caller metadata. Compare scope and exact IDs with V2, and exclude newly
   created evaluation chats from the frozen cohort.
3. For each V2 C query, copy the provider, project and frozen-cohort filters
   into `hard_filter`. Preserve the original content predicate in `literal`,
   representing the original alternatives with explicit text OR nodes.
   Follow resolver continuation to the end. Record target acquisition,
   elapsed time, provider and tool pages, decoded work, sampled worker RSS
   and all coverage gaps. Distinguish returned position from global rank.
4. Replay the permission literal and its bounded EPERM/O_SEARCH expansion
   under the same scope. Keep both hit sets. Use fresh context handles for
   semantic judgments. Evaluate the fixed judged pools independently of
   ordinary-language client acceptance and count invalid attempts and retries.
5. Run ordinary Codex requests from the project and from another working
   directory. Keep an explicit named-agent control separate. Check that the
   originating chat calls the new resolver, the correct Luna route runs,
   context and final references are validated, and the response preserves
   cardinality and Partial coverage. Record the model and client versions.
   Do not reuse old signed references or run lifecycle actions on real data.

Full evaluation receipts stay private. The workspace gates include Cargo
format, Clippy, tests and build, the architecture and lifecycle evidence
checks, the legacy semantic schema tests and a Git diff check. Remote CI
and release status must be verified against the exact pushed SHA.
