# AgentMender — Intelligent Session Search Architecture: Implementation Report

Delivery checkpoint2026-11-09. Verdict **IMPLEMENTED WITH OPEN ACCEPTANCE CONDITIONS**.
Starting main:83aeccb6041f574af4fe006313742b1053650aee. Final delivery SHA and exact
three-OS CI run are supplied in the delivery receipt, avoiding a self-referential
commit. Historical evidence and archival tags are preserved; ordinary branch main.

## A. Repository audit

The [inventory](../audits/2026-11-08-intelligent-search-inventory.md) follows all eight
crates and the actual search/approval/recovery paths. Domain remains pure; provider-api
read-only; application orchestrates; Codex/Claude interpret their own formats;
persistence owns local stores; CLI composes; MCP remains thin and cannot authorize
effects. Baseline local/remote/default branch main matched83aeccb; baseline run
42516305258 was successful on Linux/macOS/Windows, verified through authenticated
GitHub because the connector lacked private-repository access.

Reused: attested worker, safe NOFOLLOW filesystem access, protected SQLite backup,
exact identity, search/context ports, HMAC tokens, secret suppression, query control,
OwnedStateRoot, durable lifecycle and existing fitness/regression suites.
Extended: domain queries/progress, worker streaming/navigation, application result
partitions/revalidation/cache coordinator, private derived storage, CLI maintenance,
MCP schemas/projection and custom agent instructions. Removed the head/tail-only
retrieval path and its synthetic capture positions; no parallel provider scanner,
new crate or duplicate lifecycle engine was introduced. Shared schema compaction
removes validation-equivalent annotations/defaults and entailed constraints to
preserve the fixed MCP frame/catalog budget, with canary tests.

## B. Technology decision

[ADR-0027](../adr/0027-intelligent-session-retrieval.md) records official sources,
versions, costs and alternatives. Existing bundled SQLite FTS5/BM25 minimizes
integration/maintenance cost. Tantivy0.27 adds a second segment/writer lifecycle
without a demonstrated local advantage. unicode61 has multilingual limits;
lexical scores are not semantic confidence. Memory scanning works without
persistence; opted-in private rows support exact arbitrary text predicates.

Message/record adjacency, actual turn/item IDs and UTF-8 subchunks supply a small
hierarchy. No Neo4j, inferred ownership edge, LLM community graph, cloud embedding
or vector database is introduced. RAPTOR/GraphRAG and embeddings/RRF remain justified
future experiments, not dependencies blocking useful retrieval. Explicit lexical
alternatives and grounded model evaluation provide the initial recall strategy.

## C. Search functionality

Nested AND/OR/NOT, exact membership/exclusion, provider/state/native ID, observed
versus verified project facts, UTC/calendar creation/activity/message-time ranges,
text contains/phrase/exact and explicit alternatives are implemented. Time uses
half-open UTC bounds and explicit IANA zones, including Rome DST. Missing metadata
and unreadable/suppressed/unsupported content remain UNKNOWN. Verified matches
and unresolved candidates are separate and revalidated after retrieval/ranking.

Exhaustive enumeration has stable identity/time sorting and continuation, without
Top-K truncation. Ranked search exposes scanned-page-local ranking. Keyset metadata
windows continue beyond10000 and reject insertion/change against a whole-metadata
fingerprint. Complete supported transcripts can be traversed sequentially using
12MiB source slices/32MiB call budgets, including middle passages and cross-page
query facts. Supported records are bounded8MiB; larger/unknown portions remain
Partial. Inline paginated Codex formats do not imply hydrated history. Claude
transcript search is unsupported. Full details/examples: [search contract](../session-search.md).

`coverage:"complete"` is conservative: the API currently does not mint a cumulative
multi-page completeness certificate. A complete traversal may still report Partial
provider coverage/policy unmet. It does not claim an atomic global transcript
snapshot or perfect semantic recall. These are explicit practical limitations.

## D. Context navigation

Whole-source ordinals, source byte positions, observed IDs/timestamps and source
revisions replace head/tail capture ordinals. Forward/backward/around/turn/other
occurrences return bounded pages and signed references. Long messages support
safe subchunks and an explicit message_offset0 to retrieve the parent from its
beginning. Returned continuation suggestions preserve chunk/occurrence semantics.
Source append/replacement/metadata mutation refuses stale references; typed worker
errors preserve staleness without exposing filesystem details. Limits:20 messages,
16KiB output, bounded backward lookback, worker8s/parent12s. Per-analysis aggregate
budgets remain orchestration policy, not connection-wide replay state.

## E. Luna

[Configuration](../../.codex/agents/session-relevance.toml): session_relevance,
gpt-6-luna, medium reasoning, feature reductions and read-only retrieval instructions.
[Schema](../session-relevance-output.schema.json), [workflow](../luna-session-relevance.md)
and [validator](../../scripts/check_semantic_assessment.py) define exact identities,
evidence refs, categorical confidence, missing evidence and bounded requests.
The validator rejects identities/refs outside the trusted handoff; the caller must
still reissue original deterministic conditions before presenting model choices.

Actual Luna Medium spawning was exercised through the available subagent interface.
In a synthetic two-candidate fixture it requested later context, distinguished a
rejected cloud workaround from confirmed camera batching, ignored malicious text,
and kept the Docker-only candidate insufficient. The saved result conforms to the
bounded schema and exact fixture allowlist. [Fixture result](../research/intelligent-search-evidence/luna-assessment.json).
This is direct subagent/model availability and fixture acceptance, **not** proof
that the installed GUI discovers/routes the standalone custom-agent TOML or that
Luna invoked the updated MCP. Missing model/tool availability retains deterministic
search; no automatic substitution, cloud embedding or destructive workflow occurs.

## F. Security and privacy

The actual client exposes no independently enforceable per-conversation MCP routing.
Production admission from a privileged registration file is now disabled: even a
valid declaration does not prove exclusivity. Guaranteed management calls refuse
with caller_identity_required and zero candidates before source access. Typed
fixture bindings prove exclusion before counts/ranking/paging/context; diagnostic
launch-protected identities remain excluded, without an automatic caller guarantee.
**Real automatic management identity remains OPEN.**

Persistent indexing is explicit and local, private0700/0600 or Windows owner DACL,
disjoint from provider roots, contentless FTS plus opted-in safe visible text,
revision/checkpoint uniqueness, transactional/idempotent updates, stale-source
invalidation on refresh and complete-inventory purge. Invalid chunks/cache data
cannot prove absence. Source freshness is independently checked before cache reuse.
Reset/rebuild affects only derived storage; offline rebuild locks out active readers.
[Privacy/version/deletion limits](../full-text-index.md) include sensitive terms/text,
imperfect suppression, same-user ancestor/path races, backups/SSD/journal remnants
and absence of secure-erasure guarantees. No new source writer, native mutation
profile, approval route or unknown-effect retry is added. Historical text stays
inert; model evidence cannot change identity, filters or destructive scope.

## G. Verification and performance

Final local locked workspace suite: **532 primary tests passed**, including five
compile-fail authority doctests; **8 nested child-process checks passed**; zero
failed; eight opt-ins ignored by default. Four relevant pinned stable-binary
fixture integrations were then run explicitly and passed: primary cleanup,
expanded metadata cleanup, conversational primary workflow and stdio preparation/
external execution/reopen/status. These use disposable provider roots and fixture
authority; they do not represent real deletion or independent human acceptance.
The four other opt-ins remain unexecuted (live Codex/Claude discovery, exact historic
bundle-pair read and native unarchive experiment).

All required local gates pass: formatting, warning-free all-targets Clippy,
workspace tests/build, architecture, both compatibility validators and whitespace.
The architecture gate adds narrowly scoped derived-index SQL/fixed-file canaries;
it does not relax provider/lifecycle safety checks. Final remote CI is reported
for the exact delivery SHA, separately from the baseline run.

[Reproducible benchmark](../research/intelligent-search-benchmark.md):1000 generated
sessions/40000 messages/15.86MB text, build5.03s, incremental append1.72ms, lexical
query1.08ms, index22.87MB, maximum resident9.85MB on AppleM1/8GiB. Synthetic lexical
and explicit-expanded queries each yield Recall@10=1,Precision@5=1,MRR=1,FNR=0;
unexpanded vocabulary mismatch yields recall0. No general semantic or production
latency claim follows. Real driver samples: date157ms, negativeproject157ms,
content212ms and four-message context182ms. No full real index build, embeddings
comparison or actual model token-cost benchmark is claimed.

## H. Real-world acceptance

Installed Codex CLI0.166.0-alpha.6, ChatGPT desktop26.1029.61003/build24614 observed.
The earlier active conversational tool verified missing-binding refusal. The final
aggregate [real read driver](../research/intelligent-search-evidence/real-read.json)
observed updated schemas, date/negative filters, one verified content result,
19 unresolved candidates,1.59MB inspected, and four real context messages. Coverage
remained Partial. No private text, paths or provider IDs are committed in that
receipt; persistent indexing was disabled.

Source DB fingerprints changed concurrently. The worker denies writes and
revalidates snapshots, but this does not prove the whole live provider remained
bitwise unchanged. Earlier live continuations/context refused; typed staleness
now survives the worker boundary. Final context succeeded against a valid source.
No Codex application restart, real lifecycle mutation or credential copy occurred.
Updated actual conversational tool use/custom-agent routing remain **OPEN**: the
active tool catalog no longer exposed AgentMender after reconnection/context
updates. A protocol driver is not substituted for model/client acceptance.

Minimal remaining human/client check:

1. Build, refresh only the configured AgentMender MCP connection, and verify
   session_search has structured and session_context has window.
2. Ask for the documented Rome date range, non-MISSION1 and content examples;
   follow cursors and observe separate unresolved/partial data.
3. Expand returned evidence, find a later resolution/contradiction, then delegate
   bounded candidates to session_relevance and validate its returned allowlist.
4. Repeat purpose:management: until a verifiable host transport exists it must
   refuse with zero manageable candidates. Do not bypass this diagnostically.

## I. Documentation and operation

Created ADR0027, full-text-index, Luna workflow/schema, architecture inventory,
benchmark/evidence receipts, this report, semantic validation and real-read driver.
Updated README, AGENTS, roadmap, architecture, MCP, development, search contract,
ADR index and Codex compatibility status. Historical milestone evidence remains
unchanged. No ordinary milestone branches were recreated.

Quick start: cargo build --locked --workspace; existing --provider codex mcp
--session-search runs no-persistence retrieval. Persistent indexing requires the
explicit CLI build/root described in [index operations](../full-text-index.md).
[Search examples](../session-search.md) cover dates, negatives, Boolean expressions,
semantic alternatives, partial coverage and context. [Luna handoff](../luna-session-relevance.md)
provides progressive analysis and fallback without depending on a model.

## J. Capability matrix

|Capability|Implemented|Unit/property|Fixture/integration|Real-provider read|Actual conversational updated client|Production status|
|---|---|---|---|---|---|---|
|Typed date/project/provider/Boolean query|Yes|Pass|Pass|Date/project pass|Open|Supported read contract, provider limits|
|Unknown-safe negatives/separate unresolved|Yes|Pass|Pass|Partial/unresolved observed|Open|Conservative; no absence from partial data|
|Full supported transcript traversal|Yes|Pass|36MiB middle/continuation pass|Bounded real content pass|Open|Codex known shapes; caps/unknowns Partial|
|Exhaustive inventory beyond10000|Yes|Pass|10001/5000 fixtures pass|Bounded current inventory|Open|Selected root; metadata snapshot bound|
|Context/occurrences/message chunks|Yes|Pass|Resolution/contradiction/UTF8 pass|Four-message context pass|Open|Bounded; paginated hydration unsupported|
|FTS5 and explicit private cache|Yes|Pass|Resume/revision/privacy/rebuild pass|No persistent private index enabled|Open|Bounded accelerator; no global ranking|
|Semantic Luna handoff/result validation|Yes|Validator checks|Actual model + synthetic navigation pass|No private model rerank test|Custom-config routing open|Optional; no semantic recall guarantee|
|Management exclusion with typed binding|Yes|Pass|Strongest-match/count/context pass|Missing binding refuses|Earlier actual refusal pass|Host identity unsupported; real guaranteed mode refused|
|Automatic caller identity/exclusive routing|No|Fail-closed tests|Declaration rejected|Unavailable|Open|Mandatory acceptance OPEN|
|Claude transcript/provider parity|No|Existing metadata safety pass|Existing metadata fixtures|No new content test|No|Unsupported content; no mutation expansion|
|Global semantic rank/cumulative complete certificate|No|Honest partial tests|Page-local continuation tested|No claim|No|Explicitly incomplete|
|Lifecycle approval/recovery/isolation preservation|Unchanged|Pass|Four pinned extra integrations pass|No real destruction|No new claim|Existing exact historical profiles only|

Fully accepted end-to-end management search is **not claimed** while mandatory
host identity and updated conversational acceptance are open. Useful read-only
retrieval, deterministic filters and contextual evidence are delivered now.
