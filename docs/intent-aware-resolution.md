# Intent-aware session resolution

Ask Codex for a past conversation in ordinary language. For example, “Find the
conversation where we investigated Cesium rendering performance in MISSION1.”
A current `mcp --session-search` registration exposes the read-only
`session_resolve` tool. Use deterministic `session_discover` for inventory,
exact identities and possible deletion candidates.

## Originating chat workflow

1. A chat already running GPT-6 Luna Medium can interpret and assess
   directly. Otherwise, delegate the live question to `session_relevance`
   before retrieving text. If the role is unavailable, use an explicit
   `gpt-6-luna` medium delegate with fresh context. Ask for `start.intent`.
   Check the plan against the actual question. Keep all explicit conditions
   under `hard_filter`, even Boolean negations and literal constraints.
   An observed cwd label is usable for discovery. Verified project membership
   needs its own predicate. Choose `multiple` for requests for all matches
   or related discussions. Preserve outcome requirements such as validated
   implementation. Use the optional `focus` field for a decision, event or
   chronology. A decision or timeline can require multiple cited events.
2. Call `session_resolve` with `action:"start"` in the originating chat.
   `literal` and optional `expansion` use positive text leaves and Boolean
   AND/OR. Put topical terms in `literal` and at most one targeted lexical
   alternative in `expansion`. Boolean nodes use `args` and text nodes use
   `value`. The server reads both branches in one traversal and returns
   separate match truth for each candidate. If the literal pass finishes
   without enough evidence and no expansion was included, `expand` can
   add one bounded expression. That new pass shares the original budget
   and the server retains the frozen hard filter and candidate identities. Favor
   literal matches when they answer the question. Consider expanded matches
   when the original wording is insufficient or when the user wants all.
3. Send up to five candidates at a time to the same Luna delegate. Give it the
   original intent and bounded excerpts with integer handles, positions and
   coverage. Candidate aliases can reduce transcription errors. The parent
   maps them to exact returned identities before submitting an assessment.
   Keep the run ID and callable references in the originating chat.
   Ask for an assessment for every presented candidate. Ask Luna to request
   context when it cannot resolve a material outcome or comparison. Give
   particular weight to later corrections, tests and reported validation.
4. Use `action:"context"` with the run and a returned anchor handle. Supply
   `window` options like those of `session_context`, including occurrence
   queries. Follow returned `next` or `previous` handles with no window override.
   Citable handles come from returned messages. A navigation handle cannot be
   cited. Stop on repeated work or a limit and retain the open questions.
5. Submit Luna's typed assessments with `action:"assess"`. A
   `verified_relevant` assessment must cite visible content and include the
   required outcomes. Use `related` for a related workstream or outcome,
   `contradiction` for an explicit conflicting observation, and `insufficient`
   with a specific gap when evidence is missing. Use `plausible_alternative`
   only with evidence for an unresolved competing match. If the server rejects
   an assessment, ask Luna to correct it once against the same evidence.
6. Follow `action:"next"` while relevant candidates remain. Stop a run
   when its budget is exhausted. Retain its verified results and disclose
   the gap. A stale scan before usable evidence permits one fresh attempt.
   Continue until
   `more_candidates:false` on all requests, subject to the reported budgets.
   For a singular request compare competing candidates before finishing.
   `action:"finish"` checks original filters against fresh metadata and the
   source revision behind cited passages. Follow `next_offset` with the
   returned `revision` for additional verified results. Retain a live run
   for continuation when a search stopped early. When Luna is unavailable,
   use `finish` with `luna_unavailable:true` and report the limitation.
7. Return verified identities and evidence from `finish`. Describe the actual
   outcome. Present a small set of justified alternatives for an ambiguous
   singular request. Indicate when only related evidence is available, when
   the search stopped early and when logical history remains Partial. A
   direct match is a supported selection within the searched history. Avoid
   a global uniqueness or absence claim.

The repository [lookup skill](../.agents/skills/agentmender-session-resolution/SKILL.md)
and MCP instructions describe the ordinary-language route. Native project
agent registration requires a chat opened in the project. The caller-scoped
MCP operations run in the originating chat even with a named Luna delegate.
Outside the project, the host can use explicit model delegation or a chat
already using GPT-6 Luna Medium. When the host cannot run Luna, deterministic
discovery remains available and semantic resolution remains incomplete.
See the [observed host behavior](milestones/p0-sr5-intent-aware-session-resolution.md).

## Search and evidence bounds

A run admits search and assessment work for eight minutes and retains state
for up to ten minutes so verified partial results can still be published.
The server retains up to four runs per registration. A batch traverses up to twelve provider pages before
yielding and returns up to five distinct candidate identities. A run admits
up to 256 provider pages, 96 presented candidates, 48 assessment submissions,
24 context requests and 128 KiB of returned context. Each search and context
operation also retains its existing source/decoder deadlines and work bounds.
Newer runs can evict older runs. Changed source revisions and scope changes
invalidate pending or cited evidence. Start fresh when continuation expires.
No search index or transcript spool is created.

Titles affect which eligible sources are visited first inside an inventory
window. FTS5/BM25 selects passages within a source slice. An early batch is
provisional. A terminal scan traverses the same eligible inventory, subject
to the reported source gaps. Each source has its own observation revision.
In-flight checkpoints and final citations recheck that exact revision. Other
sources can change between visits. This scan does not certify a global snapshot. Neither the scan order
nor per-slice scores imply a global relevance rank. Logical completeness
remains Partial in paginated native history even at physical EOF.

Results are discovery-only and carry no management binding. Luna's evidence
and judgment are semantic inputs. Only the originating request and the
validated query can control retrieval. See the [caller contract](mcp-caller-identity.md)
and [ADR-0032](adr/0032-caller-scoped-session-resolution.md).
