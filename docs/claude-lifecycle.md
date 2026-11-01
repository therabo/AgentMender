# Claude project lifecycle investigation: M9 BLOCKED

## M8.2 scope does not enable Claude mutation

The Codex-only [scoped cleanup proposal](scoped-session-cleanup.md) is **OPEN**
under Accepted ADR-0017. It does not supersede ADR-0015's M9 evidence gate, add
Claude profiles, widen a session into project purge or permit a JSONL/storage fallback.
M9 remains BLOCKED BY PROVIDER, investigation CLOSED FOR CURRENT PROVIDER CONTRACT;
all six Claude lifecycle operations below remain Unsupported. Historical native
counterexamples retain their exact-version provenance.

M9 capability status: **BLOCKED BY PROVIDER**. Investigation status: **CLOSED FOR
CURRENT PROVIDER CONTRACT**. Claude lifecycle operations: **Unsupported**.
Integration preserves blocker evidence and safety guards; it does not make M9 PASS
or establish completed lifecycle capability. See the [closeout record](milestones/m9-closeout.md).

M9 enables **zero Claude mutation operations**. Native project purge exists in the
installed Code binary, but its target/effects/verification contract is insufficient
for AgentMender execution. No Claude mutation adapter or lifecycle compatibility
profile is installed. Discovery remains read-only. See [ADR-0014](adr/0014-claude-project-lifecycle-evidence-gate.md)
and the [native evidence record](../compatibility/claude/lifecycle-m9/README.md).

## Current official release follow-up

[ADR-0015](adr/0015-current-claude-purge-provider-blockers.md) changes the milestone
assessment to BLOCKED: the current local provider contract requires upstream
identity/effects/verification evidence before safe completion. The latest public
non-prerelease is Code 2.1.314; the separately named delayed `stable` update channel
serves 2.1.308. Both were acquired privately without installing or upgrading them.
Their [separate evidence and comparison](../compatibility/claude/lifecycle-m9/current-2026-10-30/README.md)
must not be applied retroactively to 2.1.222.

Fresh-case current-version probes confirm continued dry-run/startup writes,
colliding selectors and nested cascades. Partial deletion failures now exit 1,
while continuing other destructive effects. This improves error reporting without
proving identity, atomicity, complete effects or a deletion postcondition. There
is still no authoritative complete read-only reconciliation interface. No mutation
adapter or new production profile was added. All six operations below remain
Unsupported for all investigated exact versions.

The following installed-profile sections preserve the historical 2.1.222 findings.
Historical counts and exit-0 behavior describe that exact binary only. The older
reproducer reused state within several cases; the follow-up uses independent
fresh cases, with an explicitly marked shared concurrency experiment.

## Environment and authority

Detection on 2026-10-30 still reports Desktop 4.29.0 with independently versioned
bundled Code 2.1.222. The known Desktop metadata layout remains ReadOnlyKnown;
historical producers, exhaustive membership and lifecycle compatibility are separate
unproven facts. The synthetic 2.1.311 fixture validates no installed release.

The installed native interface is `claude project purge [path]`. Current
[official documentation](https://code.claude.com/docs/en/claude-directory#clear-local-data)
calls it `claude purge` and records the rename in 2.1.288. Its selector is a project
path, not a session UUID, Desktop sidecar ID, cloud project ID or ArtifactId.
AgentMender's existing cwd-derived Project has no provider_project_id and cannot
serve as native deletion authority.

## Operation matrix

Unsupported means unsupported by AgentMender under the reviewed installed profile;
it does not assert that no Claude product offers the feature.

| Request | Native evidence | AgentMender | Target, scope and missing proof |
|---|---|---|---|
| Project archive | No exact installed local Code contract validated | Unsupported | Local project identity, complete effects and positive archived-state proof unavailable |
| Project restore | No exact installed local Code contract validated | Unsupported | No native restore/inverse with equivalent scope established |
| Project delete/purge | Code 2.1.222 project purge help and isolated behavior | Unsupported | Path selector can include colliding/nested project state; no closed identity/effect contract or authoritative deletion proof |
| Session archive | Current Desktop documentation describes UI archive | Unsupported | Exact callable 4.29.0 contract and full worktree/remote/startup effects not established |
| Session restore | Current Desktop documentation describes UI Unarchive | Unsupported | Resume is not unarchive; no exact contract or complete compensation proof |
| Session delete | No exact installed local contract validated | Unsupported | No session-level purge or authoritative complete negative proof; no JSONL fallback |

The [Desktop documentation](https://code.claude.com/docs/en/desktop#archive-a-session)
also describes worktree removal and possible loss of uncommitted work around archive.
UI availability therefore cannot justify a target-only session effect assumption.
Session requests never expand to project purge. Project requests remain explicit
Unsupported plans, with the existing session-only planner limitation visible.

## Actual isolated findings

The initial 17 installed-Code probes used fresh private HOME/config/cwd, no credentials,
network denial, real-home content denial outside the exact executable bundle, and
write denial outside disposable state. Three independent 15-probe reproductions brought
the total to 62 probes, including 24 `--yes` invocations affecting only hand-made
synthetic fixtures. The final checked script reproduced six findings, all passing.
These are native behavior observations on synthetic inputs,
separate from synthetic fake-port tests and real-history acceptance evidence.

- Ordinary startup attempted a denied keychain helper. `--bare` allowed the native
  probes without relaxing isolation.
- Even `--dry-run` changed configuration and created a backup before printing its
  plan. Writable dry-run/startup cannot precede durable intent on real state.
- A simple purge removed planned project/session stores and retained several
  unrelated/global categories. That sample does not close every effect namespace.
- Distinct selectors `a-b` and `a/b` shared a native storage key; purging one removed
  both sessions' transcripts and associated stores while other metadata survived.
- Purging a parent also affected nested project transcripts/session stores/history,
  while the nested configuration entry survived.
- Denial of the first removal produced exit 0 without completion and without the
  requested removals. A later denial produced exit 0 after six session-store files
  had already been removed. Native failure is not atomic; exit 0 is not Applied.

No real user project was purged, archived, restored or used for destructive testing.
No new provider was installed and no existing installation was upgraded.

## Verification, recovery and execution boundary

No production Claude dispatcher exists. There is consequently no Claude path around
DurableIntent, independent approval or fresh revalidation. No command accepts a raw
Claude path as mutation authority. Existing Claude execute/reconcile CLI selection
is rejected before durable state access. MCP retains its eight read-only tools.

If a future contract is validated, reuse `TransactionBinding`,
`TransactionStore::record_intent`, `DurableIntent`, `DurableObservation`, independent
approval and verified recovery. Explicit project operation/identity/effects and
post-observation types would be necessary; current `application::lifecycle::execute`
and `reconcile` implement session unarchive and cannot dispatch project purge.
Do not expose an ungated native process while those proofs are missing.

| Point | Required future interpretation |
|---|---|
| Before provider startup | Positive refusal may prove NeverDispatched/NoEffect |
| Durable intent committed, crash before startup | OutcomeUnknown; no inferred retry permission |
| Startup, dispatch, response loss, timeout or native rejection | OutcomeUnknown once startup may have happened |
| Native completion before durable observation | Crash remains OutcomeUnknown |
| Durable accepted response | EffectObserved only; fresh independent verification still needed |
| After observation/before verification | No automatic success; verify exact deleted/retained scope |
| Reconciliation | Observe only; no dispatch, intent reconstruction, new approval or retry |

Applied would require an independently validated complete native project postcondition
and retained-state proof. None was established. Continued presence after possible
dispatch cannot prove NotApplied; partial effects/recreation/reversal remain possible.
Missing files, list entries, sidecars or project folders cannot prove deletion.
Native dry-run against writable state is not a read-only reconciliation mechanism.
Unproven results remain Unknown/RecoveryRequired.

Purge is NotCompensatable: no exact native inverse was demonstrated. Other proposed
operations have unknown compensation contracts and are not executable. M6 recovery
artifacts contain reconciliation metadata, not conversations or a restoration backup.

## Real-state smoke and remaining work

Read-only providers/scan/inspect/verify observed seven Desktop sessions, one project,
Partial/RealPartial evidence and Unknown integrity with no established findings.
Fifty-one monitored entries, including parsed metadata hashes, matched before/after;
atime is excluded. A separate ten-plan smoke covered all five intents on one session
and one project: all Unsupported, zero operations/capabilities/effect contracts,
and the same 51 monitored entries unchanged. This is bounded preservation evidence,
not a global atomic snapshot or complete provider-health claim.

Guard tests cover both fixture classes, project/session granularity, all intentions,
unknown or claimed profiles, changing storage observations, unsupported preparation,
and rejection before durable-root access. Architecture fitness rejects Claude
write/transport/authority additions and capability-producing planning overrides.
Existing Codex and MCP tests remain unchanged. These gates protect the
provider-blocked state;
they do not replace native lifecycle execution/crash/reconciliation acceptance tests.

M9 requires an upstream lossless native project identity and exhaustive membership,
a versioned closed effects/retention contract including startup and concurrency,
and authoritative coherent read-only observations proving absence and retained
invariants after ambiguous outcomes. Only then can an explicit project handoff
reuse M6-M8 and receive positive native acceptance tests. No M10 work is started
or recommended by this investigation. The 2026-10-31 closeout authorizes
history-preserving integration of evidence and guards only, subject to exact-head
CI, local gates and clean-tree verification.
