# Integrity verification contract

Current Desktop candidate status: [2026-11-06 checkpoint](milestones/m8-2-1-desktop-candidate-2026-11-06.md). Schema 4 remains unadmitted and M8.2.1 remains OPEN.

## M8.2.1 product verification

M3 integrity is unchanged. [ADR-0019](adr/0019-structural-removal-and-profile-acceptance.md)
separates RequiredPersistentProjection, EphemeralUiProjection and
UnknownReconstructionSurface. Runtime persistent absence and independent exact
profile acceptance are distinct inputs to the existing pure removal assessment.
Only proven ephemeral state incapable of repersisting target data is nonblocking.
Unknown roles, required persistent residue, incomplete inventory, missing profile
acceptance or collateral changes cannot produce product success.

Production success is established by structural persistent-state verification.
The currently open graphical sidebar may temporarily be stale and is not the
authority for cleanup success. An admitted exact compatibility profile must
independently prove through isolated restart/reload acceptance that structurally
removed sessions do not reappear. Production never restarts Codex for visual
verification. The Desktop candidate requires relevant writers to remain closed throughout the
transaction; a process-absence snapshot alone does not fence a later launch.

Expanded primary schema-3 receipts remain ProjectionIncomplete. No compiled
complete Desktop profile or isolated Desktop restart acceptance exists. Technical
residual success requires no primary or persistent reconstruction risk. M3 Healthy,
a provider return code or a visually hidden item supplies no cleanup authority.

## M8.2 scope clarification — no change to M3

M8.2 is implemented and its [scoped cleanup contract](scoped-session-cleanup.md)
does not alter this evaluator or its Healthy/Unknown/negative-proof meanings.
Accepted ADR-0017 separates exact cleanup verification from whole-provider health.
Primary cleanup is verified within its explicit complete action boundary
while unrelated provider state remains Unknown and retained. This is not an M3
Healthy or complete-erasure claim. Unknown primary identity, shared history,
root/physical/schema/writer safety or inconclusive retained invariants still blocks
or protects an attempted cleanup. No graph finding, residual or ownership label
independently grants deletion authority.

M3 is a read-only interpretation of one ArtifactGraph plus adapter-produced
IntegrityContext. `DiscoveryService::verify` discovers once, builds that graph,
calls the pure integrity-context hook and runs `evaluate_integrity`. The core never
opens files, provider databases or subprocesses. Existing discovery isolation is
unchanged. M3 itself introduces no repair, lifecycle port, plan, deletion closure
or approval authority.

## Evidence and results

IntegrityIssue retains kind, structured IntegrityIssueIdentity, affected IDs,
IntegritySeverity, IssueDisposition, original/effective evidence, supporting
NodeObservations, ReferenceEdges and explanation. The report also retains complete
rule descriptors, graph diagnostics, discovery/compatibility context and normalized
proof context. Evidence-only variants are not conflicting provider facts. Equal raw
provider IDs in distinct structured namespaces are not automatically duplicates.
Same structured session identity with incompatible facts produces a suspicious
DuplicateSessionIdentifier; all conflicting payload variants survive.

Issue identity contains provider, state root, kind, subject, sorted affected IDs and
rule IDs. Evidence order/locators/timestamps, wording and severity are not identity.
Repeated requirements merge into one issue with all provenance. Graph and context
permutations produce equal report bytes. Evaluation samples no clock or randomness.

Severity describes impact independently from certainty:

| Severity | Current meaning |
|---|---|
| Info | Representation for informational impact; no current automatic finding |
| Warning | Orphan or missing-transcript/reference inconsistency; generic identity/parent disagreement |
| Error | Missing child, broken database reference, or duplicate session identity |
| Critical | Reserved for severe established violations; no current automatic producer |

EstablishedViolation and SuspiciousInconsistency are separate dispositions, not
severity levels. For example a proven orphan is Warning, while an uncertain but
conflicting session identity is Error. Raw/effective Confidence remains visible.
No current detector produces Failed or Critical; neither is claimed as implemented
provider diagnosis merely because the representation exists.

## Verification statuses

| Status | Meaning |
|---|---|
| Healthy | At least one explicit consistency check verified; all declared checks and complete observed namespaces supported by exact coherent contract; no findings, blockers or residuals |
| HealthyWithExpectedResiduals | Same verification, with every residual explicitly expected by the contract |
| Degraded | Established violation or significant suspicious inconsistency; all limitations remain visible |
| Failed | Reserved for a critical established violation preventing trustworthy interpretation; no automatic rule yet |
| Unknown | Insufficient coverage, compatibility, contract, evidence or checks |

Health is bounded by the report's displayed scope. Coverage alone, no findings,
an empty graph, or no declared consistency checks cannot establish Healthy.
Unknown is protected, not a failed provider. `verify` exits 0 only for the two
Healthy statuses with no provider failures; every other conclusion exits 2.

## Negative proofs

IntegrityContract binds one exact executable/profile version, adapter, interface,
layout, explicit coherent observation and authoritative GraphEvidence.
NodeCoverage names the entire entity class inside this graph's provider/state
root. RelationshipCoverage separately closes the directed class/kind namespace.
The input is an internal Serialize-only adapter contract, not an imported approval
or policy document. Descriptors and version strings alone create no authority.

Every negative proof requires Complete graph discovery, relevant Complete coverage,
exact known compatible coherent contract, authoritative observational evidence under
unique admitted rules, exact producer binding, and no blocking conflicting/stale/
concurrent/unknown context. A partial or unavailable boundary cannot prove absence.

RequiredTarget also needs an observed source and matching mandatory reference.
An Unresolved placeholder supplies literal reference identity, never absence.
The contract declares the real expected class and expectation kind. A present
target defeats absence; wrong class/subtype becomes uncertainty rather than a
missing target. Transcript expectations require Transcript artifacts. MissingChild
requires Session source/target and a parent-to-child expectation; SpawnedBy is
child-to-parent and cannot establish it.

RequiredOwner supplies exhaustive OwnerRelations, including direction, valid owner
classes and admitted rule IDs. Both owner-node AND relationship enumeration must
be complete. Present valid owners defeat orphanhood; weak/ambiguous/wrong-class
claims under an admitted owner rule block it. Conflicting exhaustive declarations
block all negative conclusions. A known excluded nonowner rule does not become
ownership. Protected roots and Unknown artifacts cannot be called orphaned.

## Implemented and unsupported conclusions

| Condition | Generic core | Current live Codex / Claude |
|---|---|---|
| Orphan transcript/child/attachment/todo/plan | Exact RequiredOwner contract; synthetic scenarios | Unsupported: no exhaustive owner contract |
| Dangling reference, missing transcript/child, broken database reference | Exact RequiredTarget contract; synthetic scenarios | Unsupported: no complete target namespace contract |
| Duplicate session identity / conflicting identity facts | Suspicious normalized same-identity disagreement, excluding provenance | Can interpret retained graph variants; cannot recover facts discarded in discovery |
| Conflicting immediate-parent claims | Suspicious, retaining all edges/evidence | Only explicit SpawnedBy facts; Claude fork is References |
| Unknown provider mapping/layout/version | Typed unresolved/compatibility/producer/prerequisite blockers | Protected uncertainty; never corruption |
| Stale snapshot / concurrent change | Separate typed observation blockers | Uses structured upstream diagnostic codes |
| Unknown artifact / unattributed transcript | Protected UnknownArtifact/unmet-verification context | Never automatically an orphan or residual |
| Expected/safe/dangerous residual | See below | No validated residual producer contract |
| Stale project mapping/worktree | No automatic rule | Unsupported; path/cwd is not ownership or Git health |
| Active writer | No producer/rule | Unsupported; Active presentation is not writer evidence |
| Symlink/reparse escape | No proved-escape rule; LinkSkipped blocker | Skipped link is excluded coverage, not a proved escape |
| Missing expected mapping | RequiredTarget only with exact contract | Existing Claude missing mappings remain provider-scope IncompleteDiscovery |

SafeResidual means a residual independently proven consistent within the declared
boundary, without a provider retention expectation. DangerousResidual means a
residual independently proven to violate a provider consistency or security
invariant. Both are explicit taxonomy values without current producer rules; neither
is a removal decision. ResidualObservation must explicitly designate a candidate; arbitrary
unattributed files do not. An exact authoritative expectation yields
ExpectedProviderResidual. Unverified/conflicting declarations yield UnknownResidual,
including when a relevant established violation exists. Duplicate candidates merge
by artifact while preserving evidence. No classification implies safe removal.

## Provider mappings

Codex native summary facts retain Strong confidence; cwd grouping remains Heuristic.
Database rows, native summaries and rollout counts have distinct scopes and are
never reconciled by subtraction into orphans. attached_thread_parent_id stays
opaque. Current enumerator version is not historical transcript-producer evidence.
The duplicate-page fixture has first-wins observations plus ConcurrentChange;
M3 cannot reconstruct discarded facts and reports instability, not identity corruption.
Active and archived sessions remain retained protected roots, alongside global state.

Claude explicit cliSessionId and Desktop fork fields retain Heuristic confidence
and unknown historical producer. Fork is References, never SpawnedBy. No local_
stripping, parentUuid parenthood or bucket decoding is added. Unattributed root and
subagent locators stay protected. Missing mapping targets that discovery did not
retain remain provider-scope diagnostic limitations; the core never parses diagnostic
English or invents IDs from it. Desktop namespace components stay in identity.

Both current adapters deliberately inherit an empty integrity context. Normal
sanitized/live reports are Unknown, with no established absence, expected residual,
writer or corruption claims. A later producer contract requires fresh exact-version
evidence and independent review, not just a new enum value or compatibility flag.

## Validation and future work

Domain synthetic scenarios supply explicit authoritative closed contracts. Generated
properties use 256 cases each with fixed seed `0x4d335f696e746567`, standard replay
persistence and no live homes. They weaken positive witnesses, add owners/targets,
permute facts, preserve conflicts, and test unknown compatibility/residual protection.
Provider scenarios cover observed/synthetic fixtures, duplicate pages, retained roots
and skipped links. CLI manifests compare every fixture byte before/after verify
text/JSON and test unknown test-owned roots. Explicit live verification additionally
fingerprints bounded real source state around providers/scan/inspect/verify.

Future requirements: validated namespace closures, coherent capture contracts,
producer-specific owner roles, writer evidence and residual contracts. Physical
identity/hard-link semantics and post-transaction verification remain later work.
No implementation of those future operations is included in M3.
