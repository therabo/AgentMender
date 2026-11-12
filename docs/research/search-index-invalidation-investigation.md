# AgentMender search invalidation investigation

November 12, 2026 · Europe/Rome

## Original incident

The original Codex chat trace was available. The app turn timeline and the
recorded MCP outputs were inspected read-only. This report retains only
operation names, state codes and counters. The failing turn started at
00:38:56 CET. It used the installed legacy resolver through AgentMender.

| Operation | Provider pages | Freshness failures | Stop reason | Outcome |
| --- | ---: | ---: | --- | --- |
| `start` | 1 | 0 | — | Scan still open |
| `next` | 2 | 1 | `search_revision_changed` | Scan interrupted |
| `context` | 2 | 1 | `search_revision_changed` | Retained run status |
| `finish` | 2 | 1 | `search_revision_changed` | `incomplete_search` |

The first observed failure was a provider page continuation. The subsequent
context and finish responses carried the existing stop reason. The preceding
turn in the same chat ended after 63 provider pages with
`scan_complete:false`, `freshness_failures:0` and no stop reason. That was a
separate incomplete search.

**The changed state observed in the failing turn was a rejected search
continuation.** The legacy resolver sets `search_revision_changed` when
`SessionSearchService::search` returns `StaleSearchEvidence` during scan
continuation. The saved response does not retain the provider comparator,
old and new commitments, or rollout checkpoint. Several branches use the
same code. The exact file, row and comparator for the historical call
cannot be recovered from that record. The phrase about an index changing
was the client’s interpretation. The visible MCP registration has no
`--search-index-root` option, and the trace contains no evidence of a
persistent FTS index. It does not record the process's executable hash
or launch options at the time of the call.

An eligible title update reproduces the same page count, reason and freshness
counter on a disposable copy of the baseline executable at `8c63ef6`.
An unrelated SQLite table write also changes WAL/SHM in that control and the
second page succeeds. A title change is a demonstrated explanation for the
signature. The original event remains unattributed.

## How the checks work

The checked-out branch started at `c9f8bc4`. The remote query returned
`main` at `8c63ef6` and no remote head for the feature branch. The table below
uses symbols from the local branch. The original legacy scan has the same
corpus, inventory and source-revision refusal families. The later progressive
worker adds resident snapshot reuse and scoped reanchoring.

| Component | Code and observations | Condition, impact and rationale |
| --- | --- | --- |
| Provider DB, WAL and SHM | [capture.rs](../../crates/provider-codex/src/capture.rs) `fingerprints`, `metadata_record`, `fingerprints_changed`, `same_primary_identity`. Reads the three fixed files with bounded safe access. Observes length, SHA-256, device, inode, mode and nanosecond mtime/ctime. Optional sidecar absence is recorded. An unstable before/after stat also changes the result. | A physical difference invalidates snapshot reuse. If detected within one operation, the worker attempts fresh validation when the primary still has the same device, inode and mode. Replacement or an unstable primary fails closed. An otherwise irrelevant sidecar write can increase I/O, trigger a backup or hit a deadline without changing the eligible rows. |
| SQLite snapshot | [content.rs](../../crates/provider-codex/src/content.rs) `read_snapshot`, `capture_request` and [capture.rs](../../crates/provider-codex/src/capture.rs) `bounded_backup`. Read-only, NOFOLLOW and query-only source, bounded SQLite backup to memory, 128 pages per step. One resident snapshot on the same root and physical fingerprints. | The worker stages a result, checks files again, and can verify against a fresh backup. Search uses `revalidate_search_snapshot`. Batch publication re-verifies selected evidence. Continued physical drift during that batch verification can still refuse it, even when the first fresh selected rows agree. |
| Query-scoped metadata facts | [content.rs](../../crates/provider-codex/src/content.rs) `query_metadata_facts`, `metadata_fingerprint`, `candidate_eligible`, `discovery_title_weight`. Serializes UUID, predicate truth, requested time-sort keys, raw title and observed project when ranking, selected locator for content and optional source stat. | Raw title/project values are stronger commitments than their computed rank. A title edit can change the digest while leaving every eligible identity, lexical result and title score unchanged. An update timestamp matters if it changes a predicate result or the requested sort key. The caller and other protected identities are excluded before the corpus digest. |
| Corpus commitment | [content.rs](../../crates/provider-codex/src/content.rs) `inventory_cached`. Hashes metadata facts for every metadata-eligible row in native-ID order, without source stat in the corpus hash. Prefix `agentmender/search-corpus/v3`. | A saved digest mismatch rejects before scanning the active page. An insertion, removal, changed locator or changed admitted metadata can trigger it even beyond the current inventory window. This protects coverage and stable enumeration across windows. A lexically irrelevant row can still be metadata-eligible and affect the commitment. |
| Inventory commitment | [content.rs](../../crates/provider-codex/src/content.rs) `inventory_cached`, `PriorityWindow::page`, `revalidate_search_snapshot`. Hashes the bounded keyset window with prefix `agentmender/search-inventory/v2`. A priority window reuses stored digests and ID order. DiscoveryPriority omits per-source revisions from this digest. Other sorts can include them. | A mismatched saved inventory or corpus refuses. A metadata change within a page is also checked against fresh inventory before the staged result escapes. The window prevents offset reuse after membership or order changes. Both digests can change together, with the corpus checked first on cold recomposition. |
| Query binding and batch digest | [session_search.rs](../../crates/application/src/session_search.rs) `SessionSearchService::search`, [domain session_search.rs](../../crates/domain/src/session_search.rs) `search_scope` and [content.rs](../../crates/provider-codex/src/content.rs) `search_cached`. Scope hashes parameters, root, exclusions and caller binding. A separate batch digest hashes read observations and source status. | A scope mismatch refuses before provider access. The worker verifies stored query observations against the typed predicate. The resolver's normal `next` cursor carries no previous batch fingerprint. The returned batch digest is not the rejecting comparison for that continuation. |
| JSONL and Zstd revision | [content.rs](../../crates/provider-codex/src/content.rs) `stamp`, `stream_transcript`, `verify_evidence`, `context_candidate` and `revalidate_context_snapshot`. File revision hashes device, inode, link count, size, mtime and ctime. The locator, header UUID and bounded source position are checked. Zstd checkpoints track decoded positions and replay. | A changed or replaced source refuses old evidence, including same-bytes replacement. A changed in-flight rollout invalidates its old checkpoint. Prior published passages from other identities remain independent. An unrelated source append outside the active checkpoint need not invalidate the priority scan. |
| Temporary FTS5 | [content.rs](../../crates/provider-codex/src/content.rs) `rank_passages` and [search_index.rs](../../crates/persistence/src/search_index.rs) `SearchIndex`. Builds an in-memory ranking index when several matching passages benefit from BM25. | Builds are disposable. No saved FTS generation participates in the continuation comparison. Rebuilding the index for the next batch is normal. |
| Private persistent FTS5 | [indexed_search.rs](../../crates/application/src/indexed_search.rs) `IndexedSearchReader::search_sessions` and [content.rs](../../crates/provider-codex/src/content.rs) `search_cached`. Optional derived shortlist plus stored source revision, observations and query scope. | A stale or unusable cache entry falls back to the live worker. The worker checks exact source revision and typed observations. Stale derived data cannot create a confirmed row. This cache is opt-in and separate from the provider's metadata SQLite. |
| Run, source checkpoint and MCP | [session_resolution.rs](../../crates/application/src/session_resolution.rs) `ResolutionStore::execute`, `Run::scan`; [progressive.rs](../../crates/application/src/session_resolution/progressive.rs) `cursor_advances`, `reanchor_source`, `publish_batch`, `verify_replayed_results`; [session_search.rs](../../crates/application/src/session_search.rs) `reader_error`. | The worker transports the sanitized `ContentReadError::Stale`. The read adapter restores `stale_search_evidence` and application maps it to `StaleSearchEvidence`. Scan continuation records `search_revision_changed`. One in-flight source reanchor can succeed without changing scope and inventory. Repeated or nonmonotone progress refuses independently. Changed caller, root or server registration produces stale run rather than a model or index verdict. Signed references bind the process and source. |

The concrete title-only chain is **SQLite update → WAL/SHM changes →
snapshot refresh → raw title bytes differ in the corpus digest →
`stale_search_evidence` → `search_revision_changed` → interrupted
acquisition**. In the direct Rust comparison both commitments changed
and eligible count stayed constant. The cold path tests corpus before
inventory, so that fixture identifies the exact rejecting check.

## Disposable reproduction

Run `python3 scripts/probe_search_invalidation.py /absolute/path/to/agentmender`
on macOS. The script builds only generated roots, leaves the resolver in
read-only mode and changes fixture files between MCP calls. It records
physical deltas by filename and aggregate outcomes without paths, raw
fingerprints, passages or queries. The direct Rust test
`discovery_priority_cursor_observes_titles_and_eligibility_before_lexical_content`
also compares the internal digests and eligibility count before and after.
The ordinary batch has five candidates. The long run adds a row after eight
batches in a 110-session fixture.

| Change between operations | Observed state | Result | Relevance |
| --- | --- | --- | --- |
| Create a new chat | WAL/SHM change. Eligible corpus gains an identity | Interrupted with `search_revision_changed`. 5 already verified rows returned | Cohort membership changes, even though the new fixture text is unrelated |
| Rename a candidate to a different neutral title | WAL/SHM and corpus/title facts change | Same interruption. 5 source-bound rows returned | Current digest commits raw title. Title score can stay unchanged |
| Increment an unused last-active timestamp | WAL/SHM changes. Predicate and sort facts unchanged | Continues. 1 fresh snapshot | Unused display fact |
| Append in the originating chat and update its timestamp | Caller rollout plus WAL/SHM change | Continues. 1 fresh snapshot | Caller was excluded before digest and retrieval |
| Append to the assessed candidate | Rollout changes. Provider DB files stable | 4 publish, 1 reacquired. One freshness failure | Old signed evidence belongs to the old revision |
| Archive the candidate with an active-only filter | WAL/SHM and eligible membership change | 4 publish. Next corpus page interrupts | Hard filter now excludes that identity |
| Archive the candidate without a state filter | WAL/SHM changes. Eligibility and order stable | Continues, 5 publish | Display-only for this query |
| Write only to an unrelated SQLite table | WAL/SHM changes. Selected rows and digests unchanged | Continues. 1 fresh snapshot | Physical write only |
| WAL checkpoint TRUNCATE | Main DB, WAL and SHM change. Logical rows stable | Continues. 1 fresh snapshot | Physical rewrite only |
| Build temporary FTS for the next batch | No physical changes. 5 new temporary FTS builds | Continues, 5 publish, snapshot reused | Independent ranking cache |
| Rename a lexically irrelevant, metadata-eligible chat | WAL/SHM and raw title commitments change | Same interruption and 5 fresh rows returned | Reproduced conservative conflict |
| Append in an unrelated rollout | Rollout revision changes. Metadata stable | Continues, 5 publish | Revision is outside active evidence and the priority metadata commitments |
| Rename a chat excluded by project | WAL/SHM changes. Row remains outside the predicate | Continues. 1 fresh snapshot | Excluded row |
| Insert during the longer scan | WAL/SHM and corpus change after eight batches | Next provider page interrupts. Current 5 still publish | Cohort membership changes |
| Change a source with an opted-in warm index | Rollout revision changes while stored postings still describe the previous revision | 4 publish, 1 reacquired through live read | Stale private cache entry has no publication authority |

This matrix observes changes between calls. It does not force the in-call
SQLite replacement, unstable read or additional `Verify` drift branches.
Those remain code paths and potential causes rather than historical
findings. Source stamps, eligibility predicate flips, locator/UUID changes,
caller/root changes and unsupported checkpoint positions require
revalidation or refusal before using the affected evidence.

## Costs and future alternatives

File fingerprinting reads and hashes the three files around worker operations.
A cold backup costs I/O and in-memory SQLite pages. Recomputed corpus
commitments touch every metadata-eligible row. Inventory sorting/retention is
bounded to the current window. Exact evidence verification probes up to five
sources with header and stat checks. The existing 10,000-session benchmark
recorded about 26.1 GB of physical fingerprint bytes, one snapshot and 3,999
reuses. It was a synthetic run and the count is bytes hashed, not physical
device I/O. [Benchmark receipt](../milestones/progressive-session-resolution.md)

| Strategy | Correctness, security and result continuity | Resource, implementation and portability effects |
| --- | --- | --- |
| Coherent SQLite snapshot for a run | Stable row cohort and order. Must continue validating exact sources and disclose later rows outside the snapshot. Long reader transactions can hold WAL pages. | Pinned memory or WAL, fewer backups, higher lifetime complexity. SQLite-specific implementation behind provider ports. |
| Freeze the initial cohort | Clear cutoff. Requires stable exact ordering, honest as-of scope, and explicit treatment of deletion and archive changes. | IDs and order consume RAM or private storage. Initial full enumeration costs latency. Provider-neutral at the application boundary once stable IDs are available. |
| Query-scoped fingerprint over rank and predicate facts | Display-only edits can pass. Missing any order or membership fact risks gaps or duplicates. Preserve locator and protected identity checks. | Smaller encoded observations, but proving equivalence may still require a full eligible scan. Depends on explicit provider metadata semantics. |
| Granular source revalidation | Retains independent evidence and can retry a changed transcript without disturbing other rows. It cannot certify enumeration after membership drift. | Exact file probes and bounded replay per affected source. Provider supplies a stable identity and revision abstraction. |
| Reuse read-only snapshots with stronger change detection | Avoids repeated work only when replacement and concurrent WAL writes remain observable. Stale snapshots can miss rows or validate old metadata. | Potentially lower hash I/O and backups. SQLite connection lifetime, WAL semantics and OS file replacement complicate porting and review. |
| Robust keyset continuation with frozen boundaries | Allows some churn while preserving monotonic order. Moving identities across a consumed frontier can be missed or duplicated unless the contract defines reconciliation. | Bounded per-page memory is possible. Metadata indexing and formal progress cases add implementation work per provider. |
| Invalidate only the affected session | Correct for isolated evidence staleness under a stable cohort. Cannot absorb a changed global predicate or sort order on its own. | Low read amplification in the common case. Requires per-session receipts, with old references still checked at use time. |
| Reconcile revisions with a bounded retry | Can preserve progress when the changed state can be categorized. Revisiting emitted identities needs exact deduplication or a clear upsert contract. | CPU and latency grow with churn. Needs explicit budgets, stable ordering and stress tests across provider variants. |

A later phase can start by recording a sanitized comparator category and
a scope-relevance bit, and by testing title-score-preserving edits and in-call
WAL churn separately. Compare alternatives against the same fixed cohort,
source-change and duplicate/missing-result controls. This phase changes no
fingerprinting, snapshot, revision or invalidation rule.
