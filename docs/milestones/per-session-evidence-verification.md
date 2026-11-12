# Per-session evidence verification and lexical search completion

This records the implementation at `c9f8bc4`. See the
[unified resolver milestone](unified-progressive-resolver.md) for the current API. [ADR-0034](../adr/0034-progressive-resolution.md) records the decision. The [resolver workflow](../intent-aware-resolution.md#progressive-workflow) defines the MCP contract and the two conversational tables.

## Verification and publication

The originating chat gives Luna one batch of source-bound excerpts at a time. Rust validates each submitted citation against the candidate, the exact source revision and the original scope. A local positive can establish a discussion or activity in a partially available history. Testing and validation can cite different messages in the same session. A recap of work elsewhere cannot establish original execution.

Progressive MCP keeps `matches`, `alternatives` and `outcome`. It adds `execution_state`, published row `verification_state`, candidate-evaluation counters and a lexical coverage flag. Confirmed rows go into `matches`. Only plausible unconfirmed rows with fresh cited passages go into `alternatives`. Other unconfirmed judgments and rejections contribute to counts. The originating chat retains signed references and assembles the two tables across the released batches.

Ordinary completion requires a finished lexical traversal and a conclusive judgment for every candidate found. Insufficient evidence can conclude a candidate. The compatibility flag `finish_if_sufficient` is accepted without ending the scan early. The queue holds up to five candidates and one replayable response. The permitted expansion after a complete empty first pass uses the same queue and completes by the same criterion. Technical interruption has a distinct execution state and reason. Lexical completion and logical history coverage are reported separately.

When an active citation becomes stale, the resolver publishes unaffected judgments and reacquires the affected identity using the same scope. A new judgment can restore its publication without repeating the corpus scan. Another revision change or an unavailable replacement interrupts completion. Replaying a released step rechecks that step's source evidence and frozen predicates. An unrelated title edit can update the display label. Stale published evidence is withdrawn by identity. Earlier signed references remain subject to the usual source checks.

## Acceptance

The tests use disposable provider roots, the real MCP STDIO path, FTS5/BM25, the confined Codex worker and scripted Luna judgments. They assert row identities, distinct verification states, signed evidence and completion independently.

| Scenario | Fixture result |
| --- | --- |
| Seven relevant sessions | Seven confirmed identities across bounded batches after one admitted expansion. |
| Five confirmed and two plausible unconfirmed | Exactly five confirmed rows and two alternatives. A third insufficient judgment without supporting evidence generates no row. |
| Irrelevance, keyword ambiguity and retrospective attribution | The Atlas card-game discussion and the scraper recap from another session are rejected and omitted. The server refuses a positive judgment marked retrospective. |
| Partial history | An original paginated result is confirmed. The run reports the lexical coverage limit separately. |
| Distant evidence | A targeted occurrence window adds the validation message more than 120 records away. Both passages are sealed for the confirmed row. |
| More than 96 candidates | All 105 identities are confirmed exactly once across 21 batches, with the compatibility flag set. |
| Source change | Four unaffected candidates publish immediately. Only the changed candidate is offered again with new handles. Reassessment restores it. A replay preserves fresh results after a title edit and withdraws the one stale source after a file change. |
| Exhaustion and interruption | Clean exhaustion, including zero candidates, completes. A missing source and a repeated revision change interrupt with explicit reasons. |

The locked Rust workspace suite, warnings-denied Clippy, workspace build, formatting, architecture fitness checks and both lifecycle evidence guards passed. The suite reported 577 successful test cases including child harness cases, zero failures and eight existing opt-ins skipped. Six semantic-validator tests also passed.

The tests verify the two MCP result groups that feed the conversational tables. Live Luna interpretation and client Markdown rendering were outside this fixture run. The legacy `single_best` and advanced Boolean actions retain their prior behavior. Lexical traversal cannot establish absolute semantic recall or certify a session-wide negative without a supported complete review.
