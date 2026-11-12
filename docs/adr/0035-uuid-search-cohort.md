# ADR-0035 — UUID cohort for conversational retrieval

## Decision

The confined Codex content worker captures eligible metadata in one SQLite
read transaction. The worker stores compact metadata rows in an anonymous SQLite
spool under its existing private sandbox. Native UUID order and metadata
predicates are fixed at capture. The source transaction closes before
transcript retrieval. New rows belong to a later run.

A progressive cursor identifies a UUID boundary and position and, when necessary, the
bounded checkpoint of one exact rollout revision. The worker reuses the
existing lexical evaluator, passage ranking and source reader. Planned aliases participate in the same scan. [ADR-0036](0036-two-stage-luna-resolution.md) extends the anonymous spool to stage candidates and pending publication receipts.
The spool stores no transcripts. Bounded pages and a limited number of live
runs bound resident memory independently of corpus size.

Evidence and context operations read the applicable rows directly. Source
identity, header UUID, safe locator, local revision, and caller exclusions
remain required. Changes to a single source can trigger a bounded local
retry or a coverage gap. Completion, cancellation, lease expiry and worker exit release the spool.
The bundled Unix SQLite VFS unlinks its temporary file on open. An
independent UUID open starts a new local source view with signed context
navigation.

The conversational path uses no DB/WAL/SHM digest, corpus fingerprint,
inventory fingerprint or full SQLite backup. The independent diagnostic
and management contracts retain their existing integrity mechanisms.
