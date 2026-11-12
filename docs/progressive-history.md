# Progressive source traversal (P0-SR3)

[ADR-0030](adr/0030-resident-bounded-history-decoding.md) extends the existing
read ports and worker. The compressed file's total physical size is no longer an
admission criterion. This is bounded progressive operation, not an unlimited
single request or a claim that every provider format is complete.

## Decoder and worker lifetime

Observed Codex compression writes a single ordinary Zstandard frame with a
pledged decoded length. It supplies no seek table or portable decoder checkpoints.
A decoded byte offset cannot be converted into a compressed seek offset. The raw
streaming decoder from locked zstd 0.13.3 preserves its dictionary/window state
inside the existing kernel-confined content worker. It handles concatenated frames
and validates their termination; corrupt/truncated frames cannot certify EOF.

The CLI retains one read adapter for its MCP connection. A serialized,
bounded JSONL exchange reuses its isolated worker. Each operation has a
deadline and validates the source it uses. Conversational resolution keeps
a frozen UUID cohort in the worker. Independent search can reuse a bounded
metadata snapshot when DB/WAL/SHM fingerprints match. EOF on the input
closes the worker. Parent drop terminates only its own reader. See
[ADR-0035](adr/0035-uuid-search-cohort.md).

At most two compressed readers remain in the worker's LRU working set. Each has
an 8 MiB maximum decoder window, 128 KiB compressed input buffer and a recent
byte buffer capped at 40 MiB + 16 KiB, including allocated capacity. Source keys
bind canonical selected root, selected path, native UUID and physical revision.
Header UUID/producer checks use a separate bounded reader on every operation.
Eviction releases decoder state; it never persists transcript bytes.

Recent bytes permit record replay, message chunking and nearby backwards probes
without decompressing a prefix again. New forward decoding is linear in source
length, apart from bounded header checks and parser record-boundary replay. Context
requests may parse recent bytes again; work counters distinguish decoder replay
from parser work. This is not constant-time arbitrary random access.

## Operation budgets and progress

Existing parser limits remain 12 MiB per search source slice, 32 MiB aggregate
parsed bytes, 8 MiB per record, 256 KiB header and bounded message/output sizes.
Oversized records are skipped incrementally and explicitly prevent completeness.
The decoder checks deadlines between bounded raw steps and emits at most 8 KiB
at once. Each reader lease admits at most 128 MiB decoded and 128 MiB physical
input work. Seek replay yields after 96 MiB, reserving work for parsing. A context
operation can use a backwards probe and a stream under the same eight-second
worker deadline; the sum is bounded, not a new total-history limit. The parent
transport deadline remains twelve seconds. Blocking OS reads have no stronger
latency guarantee than the existing parent timeout/owned-worker termination.

A continuation binds exact provider/root/session, physical source revision and
logical JSONL position. Replay progress additionally records verified compressed
consumption and decoded frontier inside a signed reference. It is transient state,
not a model-supplied position or a portable decoder snapshot. A subsequent request
advances replay, reaches the requested record or refuses. Required state lost to
worker restart/eviction returns stale evidence; an ordinary verified reference can
initiate a new bounded replay. Search cursors still expire with their MCP parent.

Work exhaustion during seeking, parsing or EOF probing preserves a continuation.
The same logical record may be revisited to finish a chunk, but chunk offsets or
replay progress must advance. Identical successful continuation positions refuse.
No total source-length ceiling is substituted for the removed physical-size cap.
Cancellation stops the MCP wait; already dispatched reads can finish their bounded
operation before releasing the gate. Deadlines/transport failures terminate the
owned worker when its parent operation fails, never the Codex application.

## Navigation

In discovery mode, open or refresh a transcript with
`session_context.native_session_id`. A signed reference also supports
`window.direction:"start"` to begin at the first available supported message. Follow
`next_ref` with `next_window` to traverse all supported messages to `at_end`.
Forward navigation ignores an inherited `before` setting; it cannot move behind
its anchor. Completed messages advance; unfinished UTF-8 chunks retain their
parent ordinal and increase offsets.

`backward`, `around`, `turn` and `occurrences` remain available. Empty intervals
containing only hidden/tool records expose earlier authenticated backwards
boundaries. Replay may return an empty window with a same-direction `next_window`;
follow that continuation before expecting message text. A single oversized record
that prevents finding a bounded preceding boundary refuses explicitly; start/
forward traversal still skips it and records Partial coverage. No fabricated
message ordinal or text is supplied for an unsupported record.

Evidence from a later search occurrence or the existing opt-in index can jump to
that verified position. Cold jumps replay incrementally; nearby jumps use resident
bytes. Neither Luna nor the parent has a fixed total number of context requests
imposed by this reader. Luna's orchestration budget and inherited-tool limitations
remain separate. No context operation grants selection or deletion authority.

Conversational acquisition uses its initial UUID cohort. Publication and
discovery navigation check the applicable live row, locator, UUID,
exclusions and exact source revision. Files are opened descriptor-relative
with NOFOLLOW and checked for ordinary single-link identity, selected
representation and revision around source reads. Independent search uses
the scoped metadata cursor checks in [ADR-0028](adr/0028-search-reliability-and-scoped-freshness.md).

## Completeness and counters

`source_coverage` distinguishes physical EOF from logical completeness. Its
`decoded_bytes` is the logical parsed source position, not disk traffic. It reports
skipped records, unsupported interpretations, suppressed messages and unaccounted
external paginated history. Skipped oversized records cannot always be classified
as supported text, so their count is not a fabricated count of lost messages.

A context window reaching EOF may have started in the middle: its message
reference does not attest prefix coverage. Such suffix reads remain Partial.
A forward navigation reaching `at_end` accounts for returned supported records;
a full search traversal also accounts for parsed/unsupported records in the source.
Paginated inline EOF never certifies all external segments or selected lineage.
`external_history_unaccounted:true` means unknown external coverage, not a measured
number of missing segments. Full eligible-corpus coverage additionally requires
complete inventory and every logical history; none is claimed for the real cohort.

`work` reports compressed bytes fetched, decoded bytes produced (plain reads also
count here), decoded seek replay and records inspected per operation. Existing
`bytes_read` measures parsed JSONL, including bounded header/record revisits; it
excludes decoder read-ahead/replay and metadata backup. Memory benchmarks separately
sample per-process RSS and the reader process tree. A cache hit without detailed
coverage facts returns no new physical-completeness certificate.

## Privacy, compatibility and operational limits

Raw decoded bytes are retained only in bounded worker RAM for the connection
lifetime/eviction. The anonymous UUID spool holds bounded evidence passages and
pending receipts for the active resolver run. Transcript bodies and decoder
checkpoints stay outside that spool.
Sensitive-pattern suppression still precedes evidence/scoring/index storage;
raw decoder RAM necessarily contains source bytes. Explicit private indexing
remains the only persistent derived text cache and requires the existing user
configuration/permissions. Its stored checkpoints retain source freshness; a lost
resident replay state can require restarting a source build. Small build page caps
across process restarts are not a portable decoder-resume mechanism.

The retained pre-SR3 MCP parent uses EOF-framed one-shot workers and strict DTOs.
A compatibility projection preserves its proven small-source reads and omits new
fields. Required resident replay refuses rather than emitting a repeated token.
Refresh only AgentMender to use resident traversal, default management refusal and
new coverage fields through a conversational client. Restarting Codex is unnecessary
and is never performed by AgentMender. A binary on disk is not proof that an
existing MCP parent loaded it.

Tested real compressed sources span 67,502,706–147,957,003 physical bytes and up to
206,933,372 decoded bytes. Synthetic tests cross both old ceilings. Validity,
window, record, memory, time, finite integer positions, process lifetime and source
stability still constrain operations. No arbitrary-size production SLO is asserted.
See [acceptance](milestones/p0-sr3-management-and-traversal.md).
