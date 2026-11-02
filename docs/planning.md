# Immutable lifecycle planning

## M8.2.1 current planning boundary

M8.2 is historical primary-cleanup PASS and integrated; M8.2.1 is OPEN under
[ADR-0018](adr/0018-complete-session-removal.md). The pure product assessment
requires a frozen nonempty required-projection inventory, all four verification
phases, exact root/profile/target identity, collateral preservation and complete
safe-unit accounting. Unknown supported reconstruction risk prevents success.
The existing v2 manifest/digest remains primary-only and gains no extra action.
A future separately versioned complete-removal manifest must bind exact projection
records, every additional safe artifact and all app shutdown/restart effects before
approval. That destructive manifest/profile is not yet implemented or admitted.
Newly discovered unapproved artifacts remain retained and require a new plan.

## M8.2 scoped planning contract

M8.1 is integrated; M8.2 scoped planning is implemented under accepted [ADR-0017](adr/0017-scoped-cleanup-residual-preservation-provider-local-mutation.md).
The native v1 planning contract below remains implemented and unchanged. A separately
versioned scoped plan was accepted in ADR-0017; M8.2's exact-head release checks
passed and its primary contract is now integrated.
MCP remains read-only and exposes no cleanup execution authority.

The scoped manifest binds exact session/root/profile and required primary
payload/catalog actions, optional exact index artifacts, retained categories,
per-action evidence, physical identities, expected row sets/counts and logical
sizes into PlanDigest and StateFingerprint. Only approved manifest members may be
mutated; fresh uncertainty can retain optional artifacts but cannot add actions.
Shared primary history, uncertain identity/root/schema/physical state or an unproved
writer fence blocks readiness. Unrelated protected residuals may coexist with scoped
success; this does not weaken M3 or turn graph reachability into ownership.

The first candidate is exact Codex0.166.0 on Darwin arm64, plain legacy history,
selected root-local WAL DB and no root-local thread-history DB. Native writer/
maintenance exclusion and actual fresh-connection SQLite main-file EXCLUSIVE
locking protect the cutoff. BEGIN IMMEDIATE alone is insufficient. The dedicated
read port can produce Ready only for its verified exact profile; native v1 plans
retain their previous Blocked/Unsupported behavior. See
[the scoped profile, outcomes and command status](scoped-session-cleanup.md).

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
