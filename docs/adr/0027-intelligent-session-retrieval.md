# ADR-0027: Structured queries and incremental conversation retrieval

## Status

Accepted, 2026-11-08. Extends ADR-0026's read contract; does not change lifecycle admission.

## Context

The audit at 83aeccb confirms eight crates, reusable read-only worker isolation,
exact identities, HMAC evidence/cursors and fail-closed management search. Missing
capabilities are structured filters, three-valued negatives, middle-of-transcript
retrieval, expandable context and a local full-text index. Head/tail excerpts cannot
establish transcript absence. The installed MCP client does not attest chat identity.

## Decision

Evolve the existing session_search/session_context pipeline. Keep provider parsing
inside the attested worker, pure predicates in domain, authorization in application,
derived SQLite storage in persistence, composition in CLI and projection in MCP.
Source snapshots and offsets bind navigation; unsupported and suppressed records
prevent proof of absence. Use bounded streaming and explicit continuation. A work
budget is not a completeness assertion. Metadata predicates use Kleene three-valued
logic and half-open UTC intervals; calendar dates require an explicit IANA zone.
Observed cwd labels and verified project ownership remain distinct.

Use the existing bundled SQLite FTS5 for local lexical candidate retrieval and
BM25. Default scanning must work without persistent private content. Persistent
indexing is an explicit local configuration choice, in private AgentMender-owned
storage, never inside provider roots. An empty or stale index is not negative proof.
Contentless postings alone cannot validate arbitrary substring/whole-message predicates;
opted-in persistent mode therefore also retains supported safe message text in
separate private rows. Memory mode does not persist it. Indexing grants no source
write or destructive authority. References revalidate
source identity and revision. Semantic assessments never override hard filters.

Use message/record adjacency instead of another graph database. Deterministic
message references are separate from the artifact ownership graph. Keep semantic
or concept relationships out of destructive ownership. Optional Codex session_relevance
uses gpt-6-luna with medium reasoning, bounded evidence and further context requests.
Custom agent instructions are not a tool authorization boundary. No cloud embeddings,
LLM-generated summaries or automatic transcript upload are introduced.

## Alternatives and evidence

| Approach | Quality / operations | Decision |
|---|---|---|
| SQLite FTS5 + BM25 | Phrase, prefix, proximity and Boolean lexical retrieval; existing bundled engine; local transactions and bounded cache; index terms remain sensitive | Selected. Unicode61 has language limits; semantic equivalence needs explicit expansion/context |
| Tantivy 0.27 | Strong Rust BM25 and tokenizer/segment facilities; another storage lifecycle and dependency | Defer until measured SQLite limitations justify it |
| Plain streaming substring scan | Minimal private storage; exact content and structured predicates; repeated I/O | Required fallback and correctness reference |
| Embeddings + RRF | Can improve vocabulary mismatch; additional model, privacy and storage budget; no measured local benefit yet | Defer; explicit alternatives and broad contextual discovery first |
| RAPTOR | Recursive embeddings/clustering/summaries for long documents; model cost and derived claims | Defer; deterministic message hierarchy first |
| Microsoft GraphRAG | LLM entity/community extraction and summaries; expensive index and provenance management | Defer; relational message adjacency is sufficient |

FTS5 contentless-delete requires SQLite >=3.43; FTS secure-delete >=3.42. Contentless
postings still reveal terms. secure_delete and journal cleanup do not guarantee
physical erasure from SSDs, backups or snapshots. Persistent status/reset must say so.
BM25 is lexical relevance, not calibrated semantic confidence; rank coverage must
state whether it covers a batch or a corpus. No global semantic recall guarantee.

Sources inspected 2026-11-08:
- https://sqlite.org/fts5.html (FTS5, tokenizers, BM25, contentless-delete, secure-delete)
- https://docs.rs/tantivy/latest/tantivy/ (Rust alternative)
- https://www.anthropic.com/research/contextual-retrieval (context and reranking)
- https://arxiv.org/abs/2401.18059 (RAPTOR)
- https://microsoft.github.io/graphrag/ (GraphRAG)
- https://learn.chatgpt.com/docs/agent-configuration/subagents (Codex orchestration)
- https://learn.chatgpt.com/docs/app-server (host APIs are not MCP caller attestation)

## Consequences and validation

Keep the existing eight crates and safety fitness checks. Test nested Boolean
unknowns, DST date bounds, source replacement/appends, missing and suppressed text,
continuation, middle messages, context and management exclusion. Measure a sanitized
corpus separately from private read-only acceptance. Report actual model/client and
identity acceptance independently; mandatory unsupported host guarantees stay OPEN.

Implementation refinement 2026-11-09: privileged file-only caller admission is
disabled, since no declaration proves exclusive host routing. Metadata keyset
windows continue beyond10,000 with a whole-inventory metadata fingerprint; source
transcript snapshots remain window-scoped. The bounded cache is an accelerator,
not an index-only/global-ranking or cumulative completeness certificate.
