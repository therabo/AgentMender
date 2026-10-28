# ADR-0007: Explainable rules and conservative ownership assessments

## Status

Accepted, 2026-10-28. Builds on ADR-0006; adds no lifecycle authority.

## Context

Existing Evidence has categorical Confidence, source, producer version and rule.
Collection provenance, compatibility and completeness are separate facts. Codex
snapshot metadata is Strong; synthetic metadata and observed Claude field mappings
are Heuristic. Literal IDs do not automatically make an undocumented mapping
authoritative, and installed versions do not identify every historical producer.

## Decision

Reuse Evidence and Confidence. GraphEvidence retains the original Evidence plus
typed rule identity, Observation/Inference nature, optional locator and optional
observation time. Current snapshots supply no capture timestamp, so observed_at is
None. Session activity times are never substituted. Rule descriptors record source
admissibility, confidence ceiling, explanation and provider/interface/layout/adapter
context. Effective confidence can only stay equal or decrease. Missing/conflicting
rules or inadmissible sources yield Unknown and visible blockers, retaining facts.
No numeric score or strongest-evidence-wins conflict resolution is used.

Provider-specific rule definitions live in adapters. Codex cwd grouping and every
current Claude Desktop CLI/fork/cwd mapping remain Heuristic. Raw evidence and
unknown producer versions survive unchanged. Serialized descriptors are explanatory
data, not new authority; no descriptor or caller-selected class can create an
exclusive ownership assessment. Graph import is not an authorization surface.

Assessments compute classification, supporting facts and protection reasons.
Provider-global state is Global. Multiple distinct observed retained referrers may
produce Shared; a supported reference may produce Referenced. These describe
observed dependence and retention, not proof of runtime liveness. Unknown artifact
kinds, missing identities, conflicting observations and unresolved conclusions
remain Unknown. Active, archived-retained, provider-global, user-protected and
unknown-state roots retain explicit protection. Active presentation does not prove
an active writer or an inactive one.

Exclusive, Derived and Orphaned remain representation foundations where no current
validated rule supports the conclusion. No current builder proves exclusive
lifecycle ownership, reconstructibility or orphanhood; Complete covers only its
declared interface. Partial/Unavailable, unknown compatibility, missing producer
context, conflicts and NoOwnershipContract are explicit blockers. All M2 nodes
remain protected. Adding references or contradictions cannot make them exclusive.

## Alternatives and consequences

A one-reference heuristic would confuse partial discovery with absence. Treating
cwd/path proximity as ownership or promoting fixture evidence to native authority
would similarly overclaim. Accepting desired ownership labels from callers would
create an unsafe future abstraction. These alternatives are rejected. M2 therefore
does not compute a deletion closure; the milestone's closure safety intent is
tested at its conservative assessment/protection boundary without implementing M4.

## Safety implications and validation

Unknown and Global are protected; Shared never becomes target-exclusive;
Referenced alone is not ownership; Derived would need an independently reviewed
reconstructibility contract; absent discovered owners cannot establish Orphaned.
Property tests include positive Global/Shared/Referenced results, confidence caps,
partial inventories, added references, conflicting variants, retained roots,
determinism and full provenance round trips. No later milestone may turn these
descriptive classes into mutation permission without a separately reviewed contract.
