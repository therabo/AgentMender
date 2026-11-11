# AgentMender — P0-SR5.1 SearchIntent v1 & Luna Orchestration Acceptance

November 11, 2026 · Europe/Rome

## Implementation and verdict

Starting local and remote `main` both resolved to
`9de47856f5e8fd1c2343db969488fc9b7ad418bf`. The tested executable includes
implementation commit `d31adb1bb6321952dab7d496f9232d95cfc3b80e` and
catalog guidance commit `8affbae9243aa7c7defabb57d662c3c6ff86ec50`. The release receipt records the
exact pushed head and the observed GitHub Actions status.

SearchIntent v1 is implemented and verified at the executable MCP and
disposable provider boundaries. Real Codex calls exercised the new route,
including parent-mediated Luna and real cross-project discovery. Full
conversational acceptance remains open. Routing and evidence publication
varied by client run. Some ordinary requests exceeded the acceptance time
budget. Desktop graphical acceptance remains unverified.

- The canonical ordinary request is `start.search_intent`. See the
  [exact schema](../search-intent-v1.schema.json) extracted from the running
  MCP catalog and the [workflow](../intent-aware-resolution.md).
- The compiler keeps topical project names out of eligibility predicates.
  It compiles the explicit provider, observed or verified membership, archive
  state, native identity and UTC date boundaries to the existing typed AST.
  The lexical OR proposes candidates within the registered scope. It uses
  the existing FTS5/BM25 passage ranking and confined reader.
- The legacy `start.intent` and deterministic discovery routes remain
  available for advanced AND, OR and NOT, exact text and inventory. Existing
  negative logic preserves UNKNOWN. A target provider is a query predicate.
  The reader remains scoped to its configured provider/root.
- A topic can match at any activity. An investigation can match with
  investigation evidence alone. Explicit implementation and outcome
  requirements are checked in the assessment before publication. A
  session-wide excluded outcome requires logically complete history.
- The default `all_relevant` applies to singular wording. Explicit
  `single_best` supports an evidence-backed preference and bounded
  alternatives. Explicit exhaustive requests retain verified matches and
  report incomplete eligible evaluation on the lexical/Partial route.
- V1 expands at most once per run with new lexical terms. The original
  validated constraints, provider/root, registration instance and
  apparent caller remain bound. The originating chat holds navigation
  handles and evidence references. Fresh finish revalidates the exact
  session and source revisions before publishing signed references.

## Validation and security

The final executable exposes a typed object-shaped resolver input with
action-specific branches, bounded v1 fields and a structured output schema.
The object-shaped fields aid host discovery. The exact branches and
server-side validation enforce action shape. The final registry contained
16 tools and fit in a 226,876-byte catalog response. The resolver input and
output schemas were 12,392 and 5,997 bytes. The MCP catalog does not expose
an observed host-level `strict` switch. Server checks also enforce
semantic consistency, dates, distinct terms, evidence ownership and the
bounded expansion state. MCP does not supply authenticated original
user wording in the tested route. The originating chat compares Luna's
proposal with the live request.

The fixture exercised all project hint and filter examples against the real
stdio executable and disposable Codex sources. The investigation request
acquired MISSION1, AgentMender and unknown-label sessions. It published
two independent investigations with signed evidence. An incidental
unknown-label candidate remained eligible and was not selected. Explicit
observed inclusion returned MISSION1. Explicit observed exclusion returned
AgentMender and retained UNKNOWN separately. Omitting the project yielded
the same candidates. The fixture verified implementation with validation,
failed or reverted work, ambiguity, one justified best result, and incomplete
exhaustive results with two retained matches.

Malformed versions, unknown fields, enum values, null/omitted fields,
wrong types, lengths, invalid dates, contradictory outcomes, expansion
shape, wrong caller/root, stale runs and invalid evidence handles were
exercised. The existing frame guard, source revision tests, complete
versus Partial tests, approval/recovery tests and management refusals
remain active. Source bytes in the disposable fixture were unchanged.
Historical text remained untrusted throughout. Only read-only searches ran on real data.

## Historical benchmark regression

All 2,668 V2 manifest entries matched their original size and SHA-256.
The frozen cohort, 20 needs, 44 judged pairs and 52 anchored excerpts
were preserved. The original C1–C4 lexical predicates ran on the frozen
353-ID cohort using the installed app server and real caller routing.
The cohort filter identified the benchmark universe. Nominated targets
and anchors were used only for scoring after candidate acquisition.

| Controlled scan | Prior positives in cohort | Acquired | Nominated | Provider pages | Resolver calls | Elapsed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| C1 Cesium, MISSION1 label | 39 | 39 | 1/1 | 137 | 15 | 74.3 s |
| C1 Cesium, whole root | 41 | 41 | 1/1 | 209 | 30 | 97.4 s |
| C2 Earth View camera, whole root | 23 | 23 | 1/1 | 209 | 30 | 160.6 s |
| C3 ISS telemetry, whole root | 62 | 62 | 1/1 | 211 | 32 | 161.8 s |
| C4 PostgreSQL, whole root | 36 | 36 | 2/2 | 209 | 30 | 159.0 s |

The whole-root C1 scan succeeded after one stale start. Elapsed values
include the existing provider work and concurrent acceptance load. The C1
label-scoped run remains within the historical 136–138-page and 59–86-second
SR5 range. The C1 label-scoped scan read 2.21 GB. Whole-root runs read
about 3.52 GB each. They acquired two C1, one C2, eleven C3 and three
C4 lexical candidates outside the MISSION1 label. These are acquisition
measurements. Logical Codex history remains Partial.

A separate Luna Medium exercise judged all 44 fixed V2 pairs from bounded
anchored excerpts. It preserved every grade-2 selection and changed 16
related grades from 1 to 0. Exact agreement was 28/44. This exercise
tested interpretation of supplied evidence for the 20 needs, including
the EPERM/O_SEARCH fix, SQLite WAL boundary, LevelDB classifier, graphical
isolation still open and the worker-connectivity correction. It supplied
the historical evidence packets. Ordinary-client tests received none of
those anchors or expected UUIDs in their user prompts.

A synthetic Luna control kept MISSION1 topical in the investigation,
distinguished observed exclusion, resolution, an explicit single best and
exhaustive coverage, and identified the implemented and validated durable
fix. On its first pass it treated a metadata inventory as a content search
and over-interpreted an Earth View excerpt as MISSION1 Cesium evidence.
Both were corrected on the targeted second pass. This demonstrates the
difference between structural validity and intent fidelity.

## Historical malformed calls

The original private SR5 traces contain 247 resolver calls across 11
conversations. There were 25 malformed calls and four separately counted
stale failures. Malformed actions were 14 start, nine assess and two expand.
Five of 11 first calls were accepted. Seven were accepted within the first
two calls. This historical sample includes both ordinary and explicitly
selected Luna conversations.

The old schema required a low-level AST for each start. Clients sent strings
where typed nodes were required, invented semantic fields and omitted or
misnamed required fields. Assessment shape and bounded evidence checks and
expansion validity accounted for the remaining failures. SearchIntent removes
the ordinary AST requirement. Release fixes also accept bounded evidence gap
lists from Luna and return a targeted correction for discovery page sizes
above 20. The final tool description includes a minimal valid start
request and instructs the parent to execute MCP calls sequentially.

## Host routing and live acceptance

The installed Codex CLI is `0.167.0-alpha.3.2`. An installed app-server
control using an unprimed request and the project instructions emitted
`subAgentActivity` for the named `session_relevance` role running
`gpt-6-luna` at medium effort. The originating thread executed the MCP calls
and finalized source-bound matches. This control also opened five runs for
one question, so per-run enforcement and orchestration adherence need
separate evaluation. An explicit named-role control observed the same
model and effort. CLI stdout alone omits some collaboration events.
Host thread inspection supplied the separate delegation evidence.

The installed app server reported `explicitRequestOnly`. Project
instructions can trigger delegation on an ordinary request. Outside the
project, the named role or explicit model fallback depends on the host.
One unprimed camera run outside the project returned
`luna_unavailable:true`. Selecting Luna Medium as the originating model
avoids dependency on automatic delegation. It remains subject to
conversation latency and complete evidence checks. A fresh Luna Medium
child sent a structurally valid v1 request directly on the final binary.
The MCP reply was `discovery_caller_unavailable` using its normal metadata.
The originating chat remains the required executor for this route.
[Official subagent configuration](https://learn.chatgpt.com/docs/agent-configuration/subagents).

Real cross-project lookup found and verified eleven prior MISSION1 research
or lookup conversations recorded under AgentMender in one CLI run. The
same-scope real archive did not establish an original MISSION1 technical
investigation stored outside MISSION1. The synthetic AgentMender
investigation supplied that case. A normal topical Cesium request acquired
candidates under other labels without a project restriction. Explicit
project-only and cross-project requests used observed membership semantics.

Desktop UI automation refused access to the installed app. The retained
Desktop registration exposed the prior SR5 schema. The installed app-server
reload method takes no server selector, so an AgentMender-only Desktop
refresh was not available through that interface. The CLI and headless tests used fresh processes and per-invocation settings.
The headless installed app-server control is distinct from Desktop UI
acceptance.


## Local release gates

Cargo formatting, Clippy with warnings denied, the locked workspace test
suite and the locked workspace build passed. The suite reported 570
passes and eight explicitly ignored cases. The architecture, adaptive
compatibility, lifecycle evidence, complete-removal evidence, semantic
assessment and Git diff checks passed. The test build used disabled
incremental compilation and stripped debug symbols to fit the available
disk space. The final normal build and its advertised catalog were
verified separately.

## Final executable CLI matrix

The ten requested prompts ran unprimed using the installed CLI with
medium parent effort and a read-only sandbox. Odd-numbered cases ran in
AgentMender. Even-numbered cases ran in another workspace. An additional
pair used an explicitly selected Luna Medium originating model. The
parent kept the MCP connection and capabilities. Model input, candidate
identities, signed references and work counters are in the private traces.

| Request | First tool | Observed delegate | Signed resolver outcome or inventory result | Last run pages / context calls | Total |
| --- | --- | --- | --- | ---: | ---: |
| 1. Cesium analysis for MISSION1 | Resolve v1 | Project role | Two distinct verified investigations. MISSION1 was a topical hint. | 7 / 1 | 164.5 s |
| 2. MISSION1 conversations opened elsewhere | Resolve v1 | Project role path | Initial valid intent omitted the required exclusion. No signed match before the timeout. | 144 / 1 | 195.6 s |
| 3. Only the MISSION1 label | Discover | None | 91 exact observed-label sessions, including two archived. | — | 31.3 s |
| 4. Earth View camera implementation | Resolve v1 | Project role path | Incomplete with zero signed matches. The final text cited the correct implementation using other evidence. | 2 / 1 | 69.0 s |
| 5. ISS telemetry implementation | Resolve v1 | Fallback role path | The nominated implementation was signed and published with incomplete coverage. | 28 / 6 | 147.9 s |
| 6. Clean PostgreSQL bootstrap | Resolve v1 | None observed | Incomplete with zero signed matches. The final text identified the nominated session from other evidence. | 2 / 1 | 40.8 s |
| 7. macOS permission fix | Resolve v1 | Fallback role path | One signed O_SEARCH fix result. It was outside the fixed V2 judged pool for this need. | 2 / 1 | 138.2 s |
| 8. Investigation without a fix | Resolve v1 | Project role path | Incomplete with zero signed matches. The initial plan omitted the session-wide excluded outcome. | 10 / 2 | 134.2 s |
| 9. All Cesium investigations | Resolve v1 | Fallback role path | Exhaustive goal was set. Timed out before signed finish. | 167 / 0 | 195.6 s |
| 10. Possible MISSION1 deletion candidates | Discover | None | Read-only exact-label inventory and candidate review. | — | 89.1 s |

All ten first calls were accepted. The ten traces contain 200 AgentMender
calls. Three failed advertised input validation and two more failed
resolver action checks. Five stale and four busy refusals are separate.
All successful structured results validated against the live output
schemas. The largest observed successful result serialized to 77,975
bytes before the outer transport envelope. These samples differ in
prompts and model settings from the historical 11-run baseline. They
measure improved first-call usability without isolating a causal effect.

The three ordinary runs with signed semantic matches reached their
first signed finish in 130.9–146.8 seconds. The largest sampled worker
RSS was about 203 MiB. The exhaustive run's last reported scan read
2.72 GB. The page and context columns refer to the last reported run.
Several clients started fresh runs, so these work counts can be lower
than the whole conversational workload.

Host traces recorded delegate activity for seven of eight semantic
requests. An app-server trace confirmed the exact model and medium
effort for the named role. CLI thread summaries exposed role paths.
The two explicitly selected Luna-originating controls each published a
prior lookup as a match even though the user asked where the original
activity occurred. The role instructions now require evidence of the
activity within the candidate and classify a later lookup pointing
elsewhere as related. A fresh camera request inside the repository after
that clarification published the nominated original with three signed
references and incomplete coverage. The first signed finish took 73.0
seconds and the turn completed in 81.6 seconds. This was a separate
in-project control. Ordinary matrix cases 4 and 6 supplied answers despite
having zero signed finish matches. These remain orchestration and
evidence-fidelity gaps.

The synthetic fixture established original work across observed labels.
Real controlled scans established cross-project lexical eligibility and
a prior ordinary CLI run verified cross-project lookup conversations.
Substantive original work outside the MISSION1 label remains an open
real-source acceptance case. Results support the typed read-only
resolver and a usable parent-mediated route. Production conversational
acceptance remains open for reliable intent fidelity, signed-only
publication, bounded orchestration and the Desktop UI.
