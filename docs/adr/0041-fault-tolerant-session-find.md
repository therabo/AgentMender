# ADR-0041 — Fault-tolerant session finding

Accepted 2026-11-14. Extends ADR-0037 and updates the assessment and publication
rules in ADR-0039.

The validated SearchIntent controls retrieval and whether a result requires
semantic judgment. Metadata and lexical observations can satisfy ordinary
lookups. Luna assesses requested activities, events and outcomes. A relevant
judgment requires citable evidence for that candidate. Rust checks structure,
candidate identity, source provenance and freshness. Each missing or invalid
judgment leaves its candidate possible. Session-wide absence still requires
trusted complete coverage.

The coordinator sends model output to the typed resolver without repairing
individual judgments. A failed Evaluator or rejected evaluation request can
close evaluation early. Rust then drains the existing cohort, keeps judgments
already recorded and prepares possible results for unfinished candidates.
Caller, revision, scope and integrity failures retain their refusal behavior.

The Rust host owns the private run cursor and stages all verified rows in the
job's existing spool. The job exposes that snapshot only after finalization.
Lexical match quality is a separate field and controls display order with
stable UUID ties. Signed selection continues to depend on fresh reader
references and the originating scope.
