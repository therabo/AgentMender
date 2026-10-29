# Immutable lifecycle planning

M4 describes future provider-native intentions without invoking them. A plan is
not authorization. Ready is not approved. `plan <intent> <id>` represents archive,
unarchive, soft-delete, purge and repair requests. No execute, approval, recovery
or repair system belongs to M4. M6 adds [durable transaction/recovery knowledge](transactions.md)
without provider execution. All current real/sanitized plans are Blocked/Unsupported.

## Types and flow

DiscoveryService::plan discovers once per adapter, builds its graph, obtains M3
proof context, evaluates integrity and obtains pure PlanningContext. A unique
observed EntityId resolves to full ArtifactId; paths never resolve targets.
Failed/incomplete sibling inventories cannot establish bare-ID uniqueness.
The planning scope must equal the actual discovery scope.

Pure plan_lifecycle consumes ArtifactGraph (retaining DiscoverySnapshot facts),
IntegrityReport, PlanningContext and LifecycleRequest. It recomputes M3 from graph
and proof context and rejects mismatched reports. There is no I/O or clock access.

LifecyclePlan privately owns PlanContent and PlanDigest with shared getters only.
Content retains native target ID, provider fingerprint, typed observation scope,
compatibility/evidence status, PlanIntegritySummary, LifecycleCapability,
PlanEffectContract, artifact assessments, semantic PlanOperation descriptions,
PlanPrecondition/ExpectedPostcondition sets, typed PlanBlocker values,
SpaceEstimate and future independent approval/recovery requirements.

PlanStatus is Ready, Blocked or Unsupported. Only Ready has operations:
ArchiveSession, UnarchiveSession or DeleteSession. These are descriptions, never
commands, SQL or file deletion. SoftDelete is not archive: independent recovery
semantics are unimplemented. Repair remains unsupported. Orphan/residual findings
never automatically produce lifecycle operations.

## Identity and serialization

PlanDigest is the semantic ID; no random PlanId is necessary. SHA-256 preimages
use `agentmender/state/v1` and `agentmender/plan/v1` separators followed by NUL.
Plan identity hashes every PlanContent field. State identity binds whole normalized
provider/root/installation/profile, typed scope, graph facts/variants/edges/roots/
rules, request, supplied/recomputed integrity, native capability/effects context,
original graph diagnostics and effective evidence. Unrelated observed changes can
conservatively require replanning.

Explicit projections exclude compatibility reason, human coverage wording,
diagnostic messages, rule descriptions and integrity explanations. Raw Evidence.rule,
rule identity, producer, confidence, timestamps, locators, codes, paths, complete
distinct facts and original conflicts remain bound. Description-only conflicts
cannot disappear after projection. Duplicate structural effects-claim flags remain
bound before set deduplication. Plans contain no transcript/prompt/source or
credential bodies.

Every v1 array is a declared set: recursively sort by canonical structured JSON
and exact-deduplicate; JSON object keys have deterministic ordering. Future ordered
operations need a new contract. Native paths retain lossless path_serde encoding.
Human output is never the digest input.

Versioned deserialization rejects duplicate keys (including escaped-equivalent
keys), unknown fields, malformed hashes, unsupported versions, inconsistent
readiness/roles/effects/preconditions and digest mismatch. Reordered JSON preserves
identity. Hash validity is content consistency, not authenticity: anyone can
rehash. Pure validate_against replans from fresh trusted normalized inputs and
compares legitimate content via its digest, protecting against rehashed forgeries.

Changed fingerprint/content means Stale, including newly partial observations.
Equal inputs are Current only for Ready plans, otherwise Unknown. Changes to native
ID, version/profile, capability, scope/coverage, relationships, roots, evidence,
ownership/integrity/effects or observed metadata invalidate old plans. New shared
references always change the fingerprint. Validation never edits the old plan.
M6 must revalidate immediately before effects; M4 does not eliminate TOCTOU.

## Effects and ownership

Native identity, operation effects and physical ownership are separate.
NativeEffectBoundary requires an explicit contract, exact selected capability,
dedicated admitted effects rules, authoritative exact-version observations,
coherence, Complete scope and all potentially affected identities. Malformed or
contradictory relevant descriptors block before filtering. Observed cascading
descendants cannot be omitted. Missing/unverified effects block readiness.

Every graph node retains its original OwnershipAssessment/protection reasons and
role Target, PotentialEffect or Retained. Shared/Referenced/Global/Unknown outside
a proven boundary remain retained. Retained means no independent AgentMender
artifact operation, not byte-identical provider databases after a native action.
UserProtected, ProviderGlobal/UnknownState targets/effects, native aliases,
Unknown session state, partial/filtered discovery, conflicts, stale/concurrent
observations and uncertain integrity block readiness.

No current rule proves Exclusive. Purge cannot Ready and affected artifacts cannot
enter operation sets. A nonvacuous synthetic Ready archive proves only a native
session transition with closed effects and Healthy M3, without making storage
Exclusive. Referenced/Derived/Orphaned, paths/cwd and integrity findings grant no
deletion authority. Preconditions include unchanged profile/native identity/state/
capability/effects/ownership/protected roots/integrity, no additional references,
fresh coherent complete scope, future immediate revalidation/approval/recovery.

## Provider support

Codex 0.165.0-alpha.1.2 has independently captured archive/unarchive/delete protocol
shapes bound to exact interface, verified source layout and adapter. Descriptors
preserve attempted descendant archive, target restore and descendant delete
distinctions. Schemas do not establish effects closure, inactivity, rollback,
resumability or physical reclaim. Adapters return effects=None, discovery Partial,
and M3 contexts unclosed. Current fixtures lack verified live layouts and receive
no native descriptor. Historical discovery-only and unknown profiles do not
inherit lifecycle support. Claude session purge/archive stays unsupported; no raw
JSONL fallback exists. No lifecycle operation or dry-run is invoked.

## Space, output and tests

LogicalSpaceEstimate separates known subtotal, optional total and typed unknown
reasons. Only proven affected exclusive artifact sizes contribute; IDs deduplicate,
conflicts/missing sizes/overflow stay unknown. Current native session plans have
unknown logical total and zero known subtotal: this does not assert zero affected
bytes. PhysicalReclaimEstimate stays unknown while physical identity, hard links,
provider-managed stores and SQLite allocation remain unresolved. Logical sizes do
not promise disk recovery; retained/shared/global state adds no exclusive reclaim.

JSON uses mode=planning_only and report.plan. Text displays identities, scope,
capability, integrity/effects context, assessments, blockers, pre/postconditions,
space and future requirements. Exit 0 requires Ready with no provider failure;
Blocked/Unsupported return 2. No plan-import or execution/approval path is added.

Planning properties use 256 cases each, seed `0x4d345f706c616e73` and stored replay
regressions. They cover positive archive, permutations, material mutations,
conflicts, effects, protection, unsupported requests, serialization/tampering and
staleness. Fixture manifests compare every byte after planning text/JSON. Run
`cargo test --locked -p agentmender-domain --test planning_properties`.
