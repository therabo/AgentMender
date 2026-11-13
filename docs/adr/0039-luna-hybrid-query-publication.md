# ADR-0039 — Luna-planned hybrid retrieval and verified publication

Accepted, 2026-11-13. Extends ADR-0033 and ADR-0037 in the existing eight crates.

Luna Planner selects the typed query from the original request. The domain
adds a title leaf to the existing three-valued query. Rust can prove a whole
predicate from the cohort metadata even if another branch needs messages.
A plan with a session-wide semantic activity or outcome still needs evidence.
A hybrid title-or-topic query with no such global constraint can satisfy the
title branch directly. Positive message observations remain candidates for Luna.

The existing cohort and read worker stage both kinds of receipts. Before
publication, a metadata receipt revalidates the selected Codex row and the
captured predicate fields. A transcript receipt revalidates the source as before.
Insufficient evidence can produce an identifiable unverified row. An unknown
query predicate stays unresolved and cannot enter the verified result group.

MCP seals metadata selections under a distinct token kind. The token commits
to the row's identity, caller scope and observed metadata. The existing
session_delete input accepts either selection kind and revalidates it through
the same worker. Human confirmation and native dispatch retain ADR-0038.
The coordinator starts at most one Evaluator, when a candidate actually needs
assessment. A single-best metadata lookup with duplicate matches preserves the
ambiguity.

The disposable MCP fixtures cover mixed predicates, publication and selection.
The separate live Planner probe runs the two original requests against the
same fixture through the autonomous MCP path.
