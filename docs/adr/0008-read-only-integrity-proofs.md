# ADR-0008: Pure integrity evaluation with explicit closed observation boundaries

## Status

Accepted, 2026-10-28, after formal M2 integration and three independent Sol design
reviews. Extends ADR-0006/0007 without changing ownership or process policy.

## Context

M2 preserves observations, evidence ceilings, retained roots and conflicts. Its
Complete flag describes a declared interface, not exhaustive provider storage.
Unresolved identities have no expected entity class. Evidence-only variants also
produce graph diagnostics. None of these facts alone establishes corruption,
orphanhood, missing files, writer activity or residual state.

The complete 37-page research, milestone specification, all seven prior ADRs,
source, tests, fixtures and Git history were reviewed before repository changes.
Milestone ordering controls: M3 adds read-only verification; planning, repair,
MCP, recovery and lifecycle execution remain later work.

## Decision

Keep six crates and existing Evidence, GraphEvidence, EvidenceRule, Confidence,
ArtifactId and ArtifactGraph. Add a pure domain evaluator:
`evaluate_integrity(&ArtifactGraph, &IntegrityContext) -> IntegrityReport`.
The read-only port gains a pure `integrity_context` hook over the already built
graph. Application discovers once, builds the graph, obtains normalized context,
then evaluates it. Integrity evaluation performs no I/O, rescanning, process
dispatch, clock sampling or provider-specific storage interpretation.

IntegrityContext is adapter-produced, Serialize-only input: an optional
IntegrityContract, NodeCoverage, RelationshipCoverage, RequiredTarget,
RequiredOwner and ResidualObservation records. There is no proof-import CLI or
caller-authored authorization surface. No new production dependency is needed.

### Exact contract and completeness

IntegrityContract names a fixed contract ID, exact provider version, adapter
version, interface ID, layout ID, explicit coherent-observation assertion, and
GraphEvidence. The exact fingerprint must match the graph. Proof evidence must
be Observation, effective Authoritative, cite a unique admitted EvidenceRule, and
have provider_version equal to the contract version. A version string, timestamp,
confidence label or rule description alone cannot supply coherence or required
existence semantics. Current adapters emit no closed-observation contract.

NodeCoverage means the ENTIRE named GraphNodeKind in this provider/state root.
RelationshipCoverage means the ENTIRE directed source-class/target-class/EdgeKind
namespace. No subset, project filter or active-only scope may assert this closure.
Each record has explicit completeness and evidence. Contradictory coverage records
block proof; they are never strongest-wins.

Negative conclusions require BOTH Complete graph discovery and explicit Complete
relevant namespace witnesses. Unknown compatibility, missing/mismatched contract,
incoherent capture, stale/concurrent observations, ambiguous identity or conflicting
rules block negative proof. Other discovery limitations remain blockers.

### Required targets

RequiredTarget records a present source, typed expected target, directed EdgeKind,
target expectation kind and contract evidence. It binds to an observed graph edge
with the same source, kind and provider/root/local target identity; that edge may
end in a protected Unresolved placeholder. An unrelated assertion cannot invent
a reference. The expected real class is never inferred from a placeholder.

An authoritative mandatory reference plus authoritative complete target namespace,
within the coherent exact contract, can establish DanglingReference,
MissingTranscript, MissingChild or BrokenDatabaseReference. A present typed target
defeats absence regardless of evidence strength. A same-ID target of another real
class is ambiguity, not absence. Heuristic/optional references are unverifiable.
SpawnedBy points child to parent and cannot prove MissingChild; that requires an
explicit parent-to-child expectation contract.

### Required owners

RequiredOwner records a present known entity and an EXHAUSTIVE list of valid owner
roles. Each OwnerRelation specifies owner class, edge kind, direction and nonempty
admitted owner rule IDs. These are explicit adapter contract semantics, not a claim
that References or MapsTo generally means ownership. All listed node and relation
namespaces must be complete. Any valid observed owner defeats orphanhood. Weak,
ambiguous or contradictory candidate-owner claims block it instead of strengthening
absence. Protected roots and Unknown artifacts cannot be pronounced orphaned.
A relation under a known rule excluded by the exhaustive contract is not an owner.
Current providers have no owner contract; no owner observed remains uncertainty.
Orphan kind follows the required entity's known kind. It never means SafeToDelete.

### Findings, blockers and residuals

IntegrityIssue has typed kind, severity, disposition (EstablishedViolation or
SuspiciousInconsistency), deterministic structured identity, affected ArtifactIds,
evaluated original evidence/rule IDs and explanation. Severity describes consistency
impact, independently of Confidence: proven orphan/missing-reference findings are
Warning; missing-child/broken-database-reference/duplicate-session findings are
Error. Thus a suspicious identity can have higher severity than a proven orphan.
Duplicate identity is suspicious only when
normalized provider facts differ after excluding Evidence. Exact duplicates and
provenance-only variants are benign. Parent disagreements retain all claims;
the strongest edge never hides conflicting evidence.

Graph validation checks AgentMender graph structure. Construction errors remain
application/provider failures, never provider corruption. Original graph diagnostics
remain separately visible; only interpreted facts become integrity findings.

IntegrityBlocker covers compatibility/layout, missing contracts, partial coverage,
stale/concurrent evidence, skipped links, malformed/unreadable observations, unknown
rules/producers/artifacts/mappings and unmet proof prerequisites. Unknown is not
corruption. Active presentation does not establish ActiveWriter. LinkSkipped does
not establish symlink/reparse escape. Diagnostic English is never parsed for rules.

ResidualObservation explicitly names a residual candidate; unattributed files are
not automatically residuals. ExpectedProviderResidual requires the exact coherent
contract and authoritative expectation evidence. UnknownResidual remains protected.
SafeResidual and DangerousResidual are representation values without current
producer rules; ignorance cannot yield Safe. No residual class grants removal.

### Status and identity

Status precedence: Failed (critical established violation), Degraded (established
or significant suspicious finding), Unknown (remaining blockers/unverified checks),
HealthyWithExpectedResiduals, Healthy. Degraded preserves blockers and does not
claim complete verification. Failed has no current automatic producer rule; it is
reserved for severe established violations preventing trustworthy interpretation.

Healthy means all declared checks are verified inside the displayed boundary with
exact compatibility, coherent contract, complete namespace coverage, no blockers,
findings or residuals. It never claims global perfection. HealthyWithExpectedResiduals
also requires every declared residual to have explicit expected-provider evidence.
No contract or empty coverage/check set is Unknown, even with no findings. At least
one explicit consistency check must be verified; namespace coverage alone is not
a consistency check. Present targets/owners never bypass check validation. Current
real providers therefore remain Unknown unless reportable inconsistency is Degraded.

Issue identity contains provider/root, kind, subject, sorted affected IDs and rule
IDs; excludes time, random UUIDs, order, wording and severity. Reports canonicalize
complete lossless facts, retaining conflicting variants. Same graph/context yields
identical bytes. Structured output retains context and proof provenance.

## Alternatives

Inferring absence from unresolved nodes, missing incoming edges, counts, paths or a
bare Complete flag produces false certainty. Treating all graph conflicts as
corruption mistakes provenance for payload disagreement. Domain storage reads,
a second graph/rule runtime, JSON-authored proofs and repair placeholders are
rejected. Synthetic closed-contract tests exercise generic rules without promoting
current provider fixtures to unsupported authority.

## Consequences and safety implications

Future exact contracts need independent provider evidence and architectural review.
Codex row/list/file scopes remain distinct; attached_thread_parent_id stays opaque.
Claude CLI mappings/fork References remain Heuristic with unknown producers. No
local_ stripping, parentUuid parenthood or fork SpawnedBy is introduced. Integrity
and residual classifications remain separate from ownership and deletability.

## Validation

Nonvacuous synthetic scenarios prove orphan/mandatory-target findings, then remove
them with Partial node/relation coverage, added owners/targets, unknown compatibility,
producer mismatch, stale/concurrent evidence or conflicts. Generated properties
test monotonic negative evidence, ordering, conflicts, ceilings and protection.
Real/sanitized reports must avoid unjustified absence/writer/residual conclusions.
Architecture checks enforce pure integrity modules and the fixed read-only port;
fixture manifests verify unchanged provider bytes for verify/JSON/text commands.
