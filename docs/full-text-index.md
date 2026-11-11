# Local full-text index

## P0-SR5 candidate retrieval

Resolution uses the same transient FTS5/BM25 passage ranking and the same
explicit private-index policy. Within a metadata window, title terms prioritize
which eligible sources are visited first. One traversal can observe the
literal expression and a targeted expansion. Per-source-slice ranks choose
candidate anchors, including evidence from early and later matching slices.
Source selection and passage rank remain provisional. A terminal run
reports traversed sources, bytes, records and gaps independently of whether
Luna verified an outcome. [Resolution and coverage](intent-aware-resolution.md).

## P0-SR3 traversal compatibility

Resident decoding uses the existing source/index ports and source-bound positions;
no index schema, private-retention default or provider authority is added. Physical
EOF, logical completeness and detailed coverage remain distinct. Cached query
facts do not manufacture a physical coverage certificate. A persisted checkpoint
is a logical source position; transient decoder replay state can expire on worker
restart, requiring a source replay/rebuild. See [traversal](progressive-history.md).

## P0-SR2 source representation

The same read port supplies bounded decoded `.jsonl.zst` messages and explicit
compressed format tags to FTS5/cache coordination. Positions are decoded offsets;
source size/revision remain physical. No second index or history database reader
is introduced. Representation replacement invalidates old source refs/cache facts.
Decoder limits and Partial paginated coverage still prevent negative proof.
See [native history](native-history.md).

## P0-SR1 source reliability

Persistence remains opt-in and FTS5/cache ownership is unchanged. Indexing cannot repair missing source history. Cached facts require source revalidation; query-scoped metadata freshness never permits stale source reuse. Fixed active/archive counterparts retain exact identity and revision checks. See [ADR-0028](adr/0028-search-reliability-and-scoped-freshness.md).

[ADR-0027](adr/0027-intelligent-session-retrieval.md) selects existing bundled
SQLite FTS5 instead of another engine/database. `unicode61` postings and BM25
rank lexical passages; typed exact filters remain domain/application decisions.
[Search semantics](session-search.md) and [Luna](luna-session-relevance.md) explain
coverage and semantic interpretation separately.

## Default and explicit persistence

With no index configuration, search incrementally scans provider sources and
builds bounded in-memory FTS for each slice. No extracted text, query or index is
persisted by AgentMender. This is a functional full-stream fallback, including
continuations through long conversations.

`--search-index-root /absolute/private/directory` opts the read-only MCP server
into a persistent derived cache. The directory must be dedicated, outside all
configured/default provider roots and existing project content. Its parent must
exist. It is created0700 with files0600 on Unix; Windows uses the existing strict
owner/DACL capability. Insecure existing roots, links, unexpected WAL/SHM and
provider overlap refuse. No index is created in the repository by default.
The index root is a CLI launch option, never a model/MCP path argument.

**Persistent mode stores supported safe user/assistant message text as well as
search terms and source-bound references.** Pattern suppression is imperfect;
private code, personal information or credentials can remain. Choosing this mode
is an explicit privacy decision. It is local, without remote services or cloud
embeddings. Received MCP excerpts have the client's separate retention/model
processing policy.

The FTS table is contentless-delete; it does not store its own original text.
Separate bounded message rows retain text in opted-in persistent mode so arbitrary
substring/whole-message predicates can be checked exactly across chunks. A purely
contentless cache cannot reconstruct text or prove these conditions. In-memory
scoring mode does not retain that additional body column's text.

## Operations

Build the executable first. Examples use the system cache parent, not a provider
or project directory; substitute an existing suitable parent on other platforms.

```sh
cargo build --locked --workspace
./target/debug/agentmender --provider codex search-index \
  --root "$HOME/Library/Caches/AgentMender-search" build --max-pages 1000
./target/debug/agentmender --provider codex search-index \
  --root "$HOME/Library/Caches/AgentMender-search" status
./target/debug/agentmender --provider codex search-index \
  --root "$HOME/Library/Caches/AgentMender-search" reset
```

Build uses only the existing attested source reader and exact metadata inventory.
Each page exports at most16KiB safe text to the local cache coordinator. Repeat a
bounded build to continue; completed unchanged sources are freshness-checked and
reused. Build has a five-minute cooperative deadline and a configurable page cap.
Partial work returns explicit pending/unsupported/partial source counts and exit2;
successful maintenance exits0. Cancellation does not execute source effects.
No provider session is resumed. Status is diagnostics, never absence proof.

To enable reuse, add to the MCP command's launch arguments before `mcp`:

```text
--search-index-root /absolute/private/AgentMender-search
```

Use `mcp --session-search` after the normal provider/root/executable arguments.
Refresh only the AgentMender connection when the client supports it; never restart
Codex automatically. Omit this flag for no-persistence scanning.

Reset removes derived rows/postings, preserving provider state. After corruption:

```sh
./target/debug/agentmender --provider codex search-index \
  --root "$HOME/Library/Caches/AgentMender-search" rebuild
```

Rebuild recreates only the fixed derived database/journal inside a validated
private root. Active connections hold shared file locks; offline rebuild requires
an exclusive lock and refuses while the MCP server has that index open. Close
only that MCP connection if required, rebuild, then reconnect. Unknown WAL/SHM
artifacts refuse rather than following/unlinking arbitrary files. Rerun build
before expecting cached coverage. Ordinary source scanning remains usable by
omitting the cache flag if derived storage is unavailable.

## Structure, freshness and resource bounds

Sources bind provider/root/session, revision, next byte offset and completeness.
Message rows bind source, ordinal, chunk offset and original source reference.
Unique(source,ordinal,chunk) makes page replay idempotent. Transactions commit page
postings with text; saved checkpoints let a restarted build replay the last page
without duplicate rows. Every page includes a canonical ending checkpoint,
including EOF after unsupported/trailing records. A changed source restarts at0
and invalidates old rows. Missing/unreadable refresh removes its derived source;
a complete inventory sweep removes disappeared sessions. A partial inventory
never infers source deletion. Rebuild is also required for unknown schema versions.

The search wrapper streams one cached message at a time and validates every
chunk's source, ordinal, byte identity, contiguous offsets and final length.
Malformed, incomplete, too-large or out-of-budget reconstruction falls back to
source scanning rather than proving absence. Cache reconstruction is bounded to
32MiB and two seconds per request, with at most ten cached source facts from the
bounded source selection. The worker independently checks fresh header identity,
physical revision, query scope, current metadata and exclusions before reuse.
Metadata changes cannot inherit stale cached title/project facts.

This is a bounded accelerator, not a global index-only query engine. Only a bounded
subset of stored sources is reused per request; the remaining sources follow the
same complete scanning/continuation path. Relevance remains local to scanned
pages. No cache hit grants a stronger completeness or management guarantee.
Index storage uses a4MiB SQLite page cache, memory temporary storage and a262144
page database ceiling (normally1GiB at4096-byte pages). Hitting a resource limit
is an explicit error/partial operation, never evidence of no more matches.

SQLite source snapshots remain bounded32MiB under the existing compatibility
contract; unsupported larger databases must be reported, not silently truncated.
The provider record/parser and supported-layout limits are separate from index
storage capacity. Cross-platform fixtures validate storage/query behavior; live
source isolation remains separately admitted.

## Privacy limitations and version constraints

Bundled SQLite currently observed3.54.1 supplies FTS5. Contentless-delete requires
>=3.43; FTS secure-delete requires>=3.42. Both FTS secure-delete and core
`PRAGMA secure_delete=ON` are enabled. The derived DB uses DELETE journaling,
rejects WAL/SHM and validates private rollback artifacts. SQLite quick/integrity
checks detect corruption; unexpected versions fail rather than loading another
format as valid data.

Deletion/reset is logical cleanup, not secure physical erasure. Terms/text can
remain in filesystem snapshots, SSD wear-leveling, backups, crash copies or
previous artifacts. File permissions are not encryption. The existing owned-root
capability detects changed root/file identities but is not an adversarial
same-user filesystem sandbox for ancestor replacement; do not claim otherwise.
The source worker's kernel write denial continues to protect provider state.

Authoritative references: [FTS5 and contentless indexes](https://sqlite.org/fts5.html),
[SQLite secure_delete](https://sqlite.org/pragma.html#pragma_secure_delete),
[WAL behavior](https://sqlite.org/wal.html). Technology comparisons and deferred
Tantivy/embeddings/GraphRAG/RAPTOR decisions are in ADR-0027. Reproducible metrics
are in [the benchmark receipt](research/intelligent-search-benchmark.md).
