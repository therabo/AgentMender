# ADR-0009: Immutable evidence-bound lifecycle intentions

## Status

Accepted, 2026-10-28, after M3 integration and independent Sol/high reviews of
identity, preconditions, ownership, provider semantics, and space accounting.
Extends ADR-0006/0007/0008; does not grant ownership or provider mutation authority.

## Context

M4 describes intentions for future independent approval and execution. Incorrect
plans could later authorize destructive effects. Current graphs prove no Exclusive
ownership; current provider integrity contexts have no closed contract. Exact
discovery compatibility alone does not validate lifecycle semantics. Native Codex
archive/delete may cascade to descendants and modify provider-managed metadata.

## Decision

Keep six crates. Add a pure domain planner consuming ArtifactGraph (which retains
the discovery snapshot facts), IntegrityReport, typed DiscoveryScope, and a pure
adapter-produced PlanningContext. The context describes exact native protocol
capabilities separately from an exhaustive coherent NativeEffectBoundary. It is
Serialize-only input; there is no proof-import CLI. Provider ports gain only a
pure planning-context interpretation hook. Domain performs no I/O or clock reads.

LifecycleRequest uses structured ArtifactId, never an arbitrary path. Represent
Archive, Unarchive, SoftDelete, Purge and Repair. SoftDelete and Repair have no
current operation contract. PlanOperation describes native session transitions
only, never file removal, SQL, commands, repair, approval, or recovery execution.

LifecyclePlan owns private fixed content, exposed through shared getters. Its
versioned wire representation rejects unknown fields, malformed digests, changed
content, and inconsistent status/operation structure. A SHA-256 digest is content
consistency, not authenticity, provider authority, authorization or approval.
Ready means eligible for future independent approval, not approved or executed.
Only Ready plans describe operations. Blocked/Unsupported plans retain the request
and structured reasons without executable instructions.

### Canonical identity and state boundary

Use SHA-256 with distinct versioned domain separators for PlanDigest and
StateFingerprint. Bind the complete normalized provider/root/installation/profile,
typed scope/completeness/evidence status, graph facts/variants/edges/roots/rules,
original graph conflicts/effective evidence, recomputed integrity/proof context,
and native capability/effects context. Effects identify their exact capability,
contract and admitted purpose-specific rules; generic identity evidence cannot
substitute. Observed potentially cascading descendants must not be omitted.
Contradictory or malformed matching descriptors block before validity filtering.
This intentionally over-invalidates when
unrelated provider state changes. Narrower boundaries require a future reviewed
closure proof. No persistent scan ID or time is manufactured.

Explicit projection excludes only presentation: compatibility reason, inventory
coverage wording, diagnostic messages, EvidenceRule description, and integrity
issue/blocker/residual explanations. Preserve structured diagnostic codes/paths,
raw Evidence.rule, producer versions, locators, observation/activity timestamps,
confidence ceilings and effective confidence. Derive original conflict markers
before stripping descriptions: otherwise two conflicting rule definitions could
collapse to one projected rule. Bind duplicate structural effects-claim flags
before set deduplication too; malformed duplicate claims cannot share the same
state fingerprint as a valid boundary. Native paths retain lossless encoding.

Canonicalization recursively orders object keys and declared set-like collections
by complete projected structured value, exact-deduplicating semantic duplicates.
The v1 operation set has no sequential execution meaning. Future ordered operations
must use a new explicit ordering contract/version. Human output is not hashed.
Plan identity binds every plan field except its own digest. Serialized plans
canonicalize through the same validation path; JSON key order is immaterial.

Recompute evaluate_integrity(graph, supplied_report.proof_context); compare semantic
results rather than trusting public Healthy/status fields. Mismatched reports
block planning. Pure validate_against replans from fresh normalized inputs and
compares the whole legitimate plan, preventing a forged rehashed plan from being
accepted solely for a matching state fingerprint. Changed inputs/content are
Stale, including newly incomplete observations. Equal inputs yield Current only
for Ready plans, otherwise Unknown. Neither means execution-time safety.

### Effects, ownership, and preconditions

Retain original graph ownership assessments. There is no caller-selected Exclusive
classification, new filesystem ownership rule, or deletion closure. Every graph
node appears as target/potential native effect or a retained Shared, Referenced,
Global, Unknown or Protected constraint. Retention means no independent artifact
operation; it does not promise byte-identical provider databases after a future
native transition. Native semantic changes and provider-managed backing storage
are separate. Paths/cwd and integrity orphan/residual findings prove no ownership.

Readiness requires exact matched native capability, authoritative native identity,
complete coherent exhaustive effects evidence, Healthy bounded M3 verification,
and no conflicts/stale/concurrent/protected-target uncertainty. Ordinary active/
archived retention roots do not prohibit describing an archive transition, while
UserProtected, ProviderGlobal and UnknownState targets do. An authoritative
target-only synthetic archive can be Ready without making any file Exclusive.
Purge remains blocked without proven ownership of every potentially affected
entity. Shared/Global/Unknown within potential effects block destruction; outside
an exhaustive effects boundary they remain retained constraints.

Preconditions include bound state/profile/native identity/capability/effects,
complete authoritative scope, no additional references within that boundary,
unchanged ownership/protected roots/integrity, fresh coherent observation, and
future immediate revalidation/independent approval/recovery where required.
Postconditions describe conditional native lifecycle state and retained logical
state, never physical erasure. Planning validation does not eliminate TOCTOU;
M6 must revalidate immediately before effects. M7 owns grants.

### Provider and space evidence

Codex 0.165.0-alpha.1.2 lifecycle protocol shapes are independently generated under
OS isolation. A fixed exact profile binds schema evidence and documented cascading
scope; this proves protocol representation, not exact semantic effect closure.
Current real/sanitized plans therefore remain blocked. Historical discovery-only
profiles do not inherit lifecycle support. Unknown profiles remain protected.
Claude fine-grained session purge/archive remains unsupported. Do not invoke
native dry-run, archive, unarchive, delete or purge to produce M4 plans.

Logical byte estimates require explicit affected identities and consistent sizes;
unknown/incomplete scopes stay unknown with only clearly labeled known subtotals.
Deduplicate logical identities. Retained/shared/global state contributes no
exclusive reclaim. Physical identity is currently unavailable; hard links,
provider-managed storage and SQLite page behavior prevent deriving physical
reclaim from logical sizes. Report physical reclaim Unknown, never fake precision.

## Alternatives

Random semantic IDs, hashing text output, trusting imported Healthy labels,
path-based ownership, treating archive as recoverable SoftDelete, raw deletion
fallbacks, automatic orphan repair and mutable plans are rejected. Adding a rule
framework, persistent plan database, transaction journal or approval system would
prematurely implement later milestones. Reuse existing sha2; no new package.

## Consequences and safety implications

Conservative current plans explain blockers rather than inventing readiness.
All observed state may invalidate a plan, so unrelated activity can require
replanning. Synthetic positive tests exercise an explicit native archive contract
without promoting fixture/provider metadata into ownership authority. Deserialized
hash-valid data still needs trusted fresh-input replanning and future approval.

## Validation

Generated tests cover canonical permutations, duplicate/conflicting evidence,
wording independence, typed scope, material mutations/staleness, forged reports,
round trips/tampering, protected/shared/global/unknown state, partial coverage,
unsupported operations, provider changes and logical/physical accounting.
Architecture fitness enforces purity, fixed read-only ports, no low-level mutation
operation, no execution/approval/recovery/MCP, and existing isolation boundaries.
Fixture byte manifests cover text/JSON planning without live provider homes.
