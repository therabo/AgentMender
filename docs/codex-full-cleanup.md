# M8.1 Codex full cleanup / lifecycle compatibility refresh

Verdict: **BLOCKED**. Can AgentMender now safely implement “remove this Codex
session”? **NO**. The reviewed current contract does not supply a complete
preapproved native effect boundary and authoritative complete deletion/retention
verification. No requirement was weakened to obtain a successful deletion.

## M9 blocker closeout

M9 branch `milestone/m9-claude-lifecycle` originally matched
`53d2a485bdc2a20111ad52393a5a9f1a75ac3518`; original main was
`83f0692cdda33bc3be34146ab8ceefab5dadf1f3`. Documentation-only closeout commit
`9daa237ac6eadeb3d74e71b19ba31bcd6eef97ef` passed all six local gates and
[exact-head Actions](https://github.com/therabo/AgentMender/actions/runs/41150461236):
[Linux PASS](https://github.com/therabo/AgentMender/actions/runs/41150461236/job/124385511633),
[macOS PASS](https://github.com/therabo/AgentMender/actions/runs/41150461236/job/124385511269),
[Windows PASS](https://github.com/therabo/AgentMender/actions/runs/41150461236/job/124385511578).

Normal fast-forward and push integrated M9 at that exact SHA; SSH independently
verified pushed main. Only then was tracked branch
`milestone/m8-1-codex-full-cleanup` created from it. See the
[integration receipt](milestones/m9-integration.md) for source review and counts.

**M9 was integrated as blocker evidence, NOT as completed lifecycle capability.**
Capability: BLOCKED BY PROVIDER. Investigation: CLOSED FOR CURRENT PROVIDER
CONTRACT. Code 2.1.222, 2.1.308 and 2.1.314 remain Unsupported. No production Claude
mutation adapter/profile/Ready operation or session-to-project widening exists.
Existing native research was internally consistent and was not reopened.

## Codex release refresh

Historical exact version: `0.165.0-alpha.1.2`. Current official stable:
[`0.166.0`](https://github.com/openai/codex/releases/tag/rust-v0.166.0), independently
confirmed through official GitHub release metadata and npm distribution. Acquisition
was private; neither the app installation nor PATH Codex was updated.

| Property | Exact new evidence |
|---|---|
| Official archive SHA-256 | `c48824462fc57681eafb0925d97b11b589010475613904f570b42fea53e240d5`, matched upstream asset digest |
| Native macOS arm64 binary SHA-256 | `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1` |
| App Server version | Native initialization reports `0.166.0`; candidate interface identifier `codex-app-server-0.166.0` is bookkeeping only |
| Official source commit | `6ff1e1ca3b30fc8b914720681fcf5fe62e95f856`; release association, no reproducible source-build identity claimed |
| Native state schema | Same 66-entry source schema as V165, `1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647` |
| Generated schemas | 321 stable / 467 experimental, independently regenerated twice with identical path sets/hashes |
| Stable schema-map fingerprint | `1a0c7ce9118772a7b2f3ca351d976bdb244e7b34633e897dbe0c68a9ff6fcfa2` |
| Experimental schema-map fingerprint | `ce6e6f326f53a36fa184cba45a1e813a38cd31d4862e6250d9700dc3595a4a43` |

[Exact provenance, selected schemas and structural comparisons](../compatibility/codex/0.166.0/lifecycle-m8-1/README.md)
keep official docs, generated schemas, official source, native behavior on synthetic
fixtures, finite filesystem measurements and inference separate. No current-release
production profile was registered. Equal wire/storage shapes are not compatibility.

Eight retained comparable stable request/response files match; list/unarchive
response definitions change. No old read-schema equality is claimed. Experimental
list filters add parentThreadId, ancestorThreadId, attachedToThreadId and projectId
relative to the retained old stable schema. Nested source spawn ancestry still
matters. Current unarchive restores descendants, a material semantic difference.

## Comparison with historical M8

Every cell cites its evidence class. Historical evidence remains unchanged.

| Property | 0.165.0-alpha.1.2 | 0.166.0 stable |
|---|---|---|
| Archive cascade closure | Hidden-child / partial-cascade counterexamples; unclosed. [old native](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/README.md) | Better relation queries, but raw-edge/thread-row namespace mismatch remains. [source](../compatibility/codex/0.166.0/lifecycle-m8-1/source-review.md) |
| Delete cascade closure | Hidden descendants and irreversible partial error. [old native](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/error-cases-summary.json) | Descendants removed before root error; complete scope still unproved. [new native](../compatibility/codex/0.166.0/lifecycle-m8-1/delete-evidence.json) |
| Hidden descendants | No-preview child omitted despite terminal list cursor. [old native](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/hidden-descendant-summary.json) | Ancestor filter reveals no-preview child; attached query recovers attached omission in finite fixture. No closure promotion. [new native](../compatibility/codex/0.166.0/lifecycle-m8-1/archive-evidence.json) |
| Exact target read | Exact UUID metadata read can outlive rollout. [old native](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/restart-missing-summary.json) | Useful exact canonical current view; stale metadata/history mismatch remains. [new native](../compatibility/codex/0.166.0/lifecycle-m8-1/delete-evidence.json) |
| Authoritative complete negative proof | Not established. [old review](adr/0013-codex-durable-native-handoff.md) | Current-view miss does not close all cleanup/history/shared namespaces. [source review](../compatibility/codex/0.166.0/lifecycle-m8-1/source-review.md) |
| Partial failure | Archive may accept partial cascade; delete may error after effects. [old native](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/README.md) | Both behaviors independently reproduced. [archive](../compatibility/codex/0.166.0/lifecycle-m8-1/archive-evidence.json), [delete](../compatibility/codex/0.166.0/lifecycle-m8-1/delete-evidence.json) |
| Unarchive scope | Target restored, descendants retained. [old native](../compatibility/codex/0.165.0-alpha.1.2/lifecycle-m8/unarchive-cases-summary.json) | Descendant restore, including previously archived children; denied-child partial restore. [new native](../compatibility/codex/0.166.0/lifecycle-m8-1/archive-evidence.json) |
| Reconciliation | Read-only positive restore observation; missing retained baseline stays Unknown/RecoveryRequired. [implemented M8](codex-lifecycle.md) | No complete protected read-only cleanup/retention proof established. [independent review](../compatibility/codex/0.166.0/lifecycle-m8-1/source-review.md) |
| Production Archive readiness | Unsupported. [ADR-0013](adr/0013-codex-durable-native-handoff.md) | Unsupported. [ADR-0016](adr/0016-current-codex-cleanup-provider-contract-gate.md) |
| Production Delete readiness | Unsupported. [ADR-0013](adr/0013-codex-durable-native-handoff.md) | Unsupported. [ADR-0016](adr/0016-current-codex-cleanup-provider-contract-gate.md) |

## Archive

**Unsupported**. Native scope includes the target and attempted spawned descendants,
including the tested hidden/attached descendants and other cwd/workspaces. Unrelated
and fork-only synthetic rollout hashes remained unchanged in tested cases, which
does not establish every retained/shared/global invariant. Archive can accept while
one child fails and deeper descendants archive. Root state or RPC success alone
cannot verify the approved entire postcondition.

Parent/ancestor queries now include empty previews. Attached+ancestor union recovers
the tested attached child; that sample alone is not an impossibility proof. Source
analysis identifies a remaining raw-spawn-edge versus joined-thread-row boundary
and no complete coherent effect/retention observation contract. The edge-only case
is source-derived, not claimed native/UI behavior. A fresh dedicated App Server's
own live registry can potentially be empty; unrelated-process registries are not
automatically added to its cascade.

Applied would require fresh authoritative archived-state proof for every promised
member, exact preapproved membership, retained invariants and acceptable recomputed
integrity. NotApplied requires positive refusal before possible writable startup
or a separately validated complete no-effect proof. All other post-start knowledge
remains Unknown/RecoveryRequired. Reconciliation must only observe under real-state
write denial. No current complete archive reconciliation contract was established.
Compensation: **NotCompensatable**, since new unarchive also restores descendants
already archived before the forward operation.

## Delete

**Unsupported**. Native delete removes descendants before root and affects rollout,
history, name/index and database state beyond a single current thread view. The
root-denial fixture returns -32603 after descendants disappear. Fresh restart still
returns their stale metadata, while history reads match a never-seeded current-view
error. Neither RPC acceptance, list absence, file absence, nor that history error
proves complete deletion. Exact metadata presence is useful but is not intact rollout
or retained-history proof.

The absence contract problem concerns namespaces, coherence and retained invariants;
lack of a typed public not-found variant alone is not decisive. The suspected initial
metadata error suppression is followed by resolver re-query and can propagate persistent
errors. No empirical false-read DB-error-as-absence claim is retained.

Applied would require authoritative absence of approved target and cascade across
the complete deletion boundary, no unexpected affected identities, preserved shared/
global/retained invariants, a coherent fresh graph and acceptable recomputed integrity,
with unchanged profile. Current evidence cannot establish all of those conditions.
After possible startup, errors/timeouts/process death or partial effects remain
Unknown; continued target presence is not NoEffect. Reconciliation creates no new
approval/intent and never redispatches. Compensation: **NotCompensatable**; no native
equivalent-scope restore exists. Recovery manifests are metadata, not backups.

## Full cleanup capability

**NO**. “Remove this Codex session” cannot yet map to a validated complete native
DeleteSession operation. No RemoveSession alias or natural-language parser was
introduced. The controlling invariant is that approval must bind the complete
provider effect boundary before dispatch and verified success must prove its full
postcondition and retained invariants. The current reviewed contract fails that gate.

## Execution pipeline preserved

No new real effect was enabled. Existing exact M8 flow remains:

1. `DiscoveryService::approval_inputs` derives graph, recomputed integrity and
   PlanningContext from trusted fresh provider inputs; domain `plan_lifecycle`
   produces an immutable Ready `LifecyclePlan` only when all predicates hold.
2. `TransactionBinding::from_plan` and `TransactionStore::create_transaction`
   create one v2 attempt. `RecoveryStore::prepare`/verification precedes Prepared.
3. Independent local CLI approval rechecks `approval_eligible`; the signed one-shot
   `ApprovalGrant` binds transaction, exact plan/state/profile/effects and recovery.
4. `application::lifecycle::execute` requires the stored exact plan, native preflight,
   independent pre-observation and immediate fresh Current revalidation.
5. `TransactionStore::record_intent` authenticates and atomically consumes approval
   with forward effect intent (`ApprovalAction::Consumed`); external journal sync
   and SQLite commit precede
   construction of private noncloneable `DurableIntent`.
6. `CodexLifecycleAdapter::dispatch` consumes that gate by value, repeats exact pin
   and expiry validation, converts to `DurableObservation`, then permits writable
   startup and the one historical native unarchive method. Startup is an effect.
7. `record_observation` journals native acceptance; fresh protected `observe`
   supplies ArtifactGraph/IntegrityReport. Application recomputes integrity and
   verifies exact native state plus the immediate retained baseline before Succeeded.
8. Unknown or failed verification protects OutcomeUnknown/RecoveryRequired.
   `application::lifecycle::reconcile` authenticates consumed history and observes
   only. It cannot recreate intent, grant, dispatch or retry; missing retained
   baseline cannot be reconstructed from hashes or poststate.

No transaction ID, digest, thread ID or ordinary DTO can construct dispatch authority.
No current stable plan is made Ready by the new evidence files.

## Tests and evidence

Ordinary tests remain provider-independent. M8.1 adds two focused nonvacuous guards
for current/neighbor version refusal, historical profile transfer and wider effect
scope, plus architecture canaries and deterministic committed schema/evidence checks.
All prior approval, post-effect verification, reconciliation, property, regression,
crash and Claude M9 guards remain intact.

Generated properties retain 256 cases per property and fixed seeds:
graph `0x4d325f6772617068`, integrity `0x4d335f696e746567`, planning
`0x4d345f706c616e73`, transaction `0x4d365f74786e7331`, approval
`0x4d375f6170707276`, lifecycle `0x4d385f6c69666531`, Claude guard
`0x4d395f636c617564`. Persisted graph/planning/lifecycle counterexamples remain.
Existing lifecycle properties include 4,096 cases; approval 5,376; transaction 2,304.
No new mutation crash contract is claimed because no operation was promoted.

Existing application restart cuts cover intent committed, possible effect before
observation, accepted observation and verification begun. Persistence child-process
abort/fault tests retain recovery, journal sync/SQLite commit, grant/revoke/consume,
observation, verification, key/migration and concurrency boundaries.

Native investigations separately repeat schemas, test B's seven archive/unarchive
cases and C's mixed/partial/restart/identity cases (11 assertions). A bounded repository
opt-in reproducer independently checks seven fresh synthetic cases and isolation
canaries. These are native effects on hand-generated state, not production positive
acceptance. The historical M8 opt-in test was attempted but rejected the now-different
installed hash before provider startup; it is reported as a compatibility rejection,
not a native pass. Historical profile bytes and deterministic regressions remain.

## Safety, Git and release gate

No real user session was mutated for milestone testing. No installation or PATH
executable was replaced. No storage-level deletion fallback exists. No production
effect bypasses independent approval, journal, recovery or fresh revalidation.
OutcomeUnknown does not auto-retry. Claude remains mutation-blocked; MCP cannot
approve and retains its eight read-only tools. No recovery backup/compensation or
secure physical erasure is claimed. Native research never accesses real sessions.

The final branch commit and exact Linux/macOS/Windows release-gate results are
reported in the final delivery and independently verified after push. The immutable
M9 integration SHA/run above are already established. No M8.1 merge into main was
requested; the reviewed final branch remains available without rewriting history.

## Verdict and upstream need

**BLOCKED**. Required upstream evidence is a versioned authoritative predispatch
closure matching native effects, including raw-edge/materialization namespaces;
defined startup/cascade/shared/global/retained effects and concurrency; protected
coherent complete post-observation with error-versus-absence semantics; and decisive
read-only reconciliation of ambiguous outcomes with retained proof. Parsing more
fields or adding a typed not-found result alone is insufficient.

Keep this outcome as a roadmap decision point. Do not weaken guarantees or begin
M10. Future UX can remain simple over strict internal guarantees once the native
contract is sufficient.
