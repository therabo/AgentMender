# ADR-0013: Exact Codex unarchive through a consumed durable handoff

**M8.2 cross-reference (2026-10-31):** [ADR-0017](0017-scoped-cleanup-residual-preservation-provider-local-mutation.md)
partially supersedes native-only rules for its exact Codex scoped-local contract.
Semantic direction is accepted; implementation remains OPEN. The historical
native/read-only contract and evidence below remain unchanged; see
[current scoped-cleanup status](../scoped-session-cleanup.md).

## Status

Accepted, 2026-10-29. Extends ADR-0009/0011/0012 after verified M7 integration at
`5cf94c01d1d14acf6034d7bb7f5521547ee470dd`. M8 is Codex only.

## Context and evidence

M7 creates an authenticated one-shot durable intent but cannot dispatch. The full
research and milestone specification require native effects, independent approval,
fresh validation, journal-before-effect and independent post-effect verification.
The M8 specification does not require MCP execution; its eight tools remain read-only.

Fresh isolated investigation of installed Codex `0.165.0-alpha.1.2` regenerated
320 schemas. Compared initialization/list/lifecycle schemas and the exact 66-entry
source schema match the retained V165 profile. The binary SHA-256 is
`d84a54b031085f01e3ad915ea7c72ebea4dcf69e90f0d1e0e4a637e8280991d2`.
Experiments used only explicitly synthetic rollouts in private disposable homes,
with real-home content, outside writes, network, forks and Mach lookup denied.
Fixtures were prepared separately; native lifecycle effects were not emulated.
Current [official documentation](https://learn.chatgpt.com/docs/app-server)
corroborates the scopes but does not attest to exact binary implementation identity.

Actual-binary counterexamples prevent archive/delete execution support:

- Archive can return success while a denied child stays active and a grandchild
  archives. Delete can fail after removing a child rollout.
- A metadata-only spawned child is omitted from complete-looking unfiltered native
  listings, yet native archive/delete affects it. List pagination cannot close the
  cascade boundary. The historical normalized parent field misses nested spawn data.
- Direct metadata read can succeed after its rollout was removed. Generic native
  errors do not establish authoritative absence or no effect.

Unarchive restores the target while descendants stay archived. Positive native
active-list membership can establish logical active presentation for that exact ID.
Some restored no-preview targets remain unlisted, which is Unknown. Unarchive is
not an inverse of cascading archive. No provider-global backing-storage closure,
deletion negative proof or native restoration backup was established.

## Decision

Support only the exact validated unarchive dispatch contract, conditionally on every
M3/M4/M6/M7 prerequisite. Archive and delete execution are Unsupported. Separate
execution compatibility from read-only compatibility and informational protocol
descriptors. Pin executable content, signed bundle metadata, version, interface,
source schema, protocol evidence, target-only semantic contract and adapter version.
Other binary/profile/platform combinations fail closed. Current real discovery
remains Partial, with no complete effects or integrity contract, so it cannot Ready,
gain approval or dispatch. This milestone does not relabel partial observations.

Keep the eight crates. The handoff port lives in persistence beside the private
receipt constructor. Application and provider-codex may depend on persistence for
this narrow boundary. Persistence still depends internally only on domain and never
calls providers. Provider-api stays read-only. Provider-codex may use handoff types,
not TransactionStore, recovery writers, signing keys or authority methods. Application
owns orchestration; CLI only composes it. MCP has no durable writer or lifecycle port.

Dispatch consumes noncloneable, nonserializable `DurableIntent` by value. Only the
trusted store creates it after matching original plan/binding, fresh Current replan,
verified recovery, active authenticated grant, atomic signed consumption, external
journal sync and SQLite commit. Conversion produces a noncloneable observation-only
handle with no constructor or route back to dispatch. Observed-result recording
accepts that handle. Transaction IDs, digests and ordinary DTOs cannot start effects.

The execution coordinator checks the exact prepared v2 transaction and unarchive
operation, runs read-only native preflight, obtains immediate trusted planning
inputs, records intent and hands off immediately. No public raw method, JSON,
command, provider write token or arbitrary callback exists. No native filesystem
or SQLite lifecycle fallback is allowed.

### Startup, process and executable boundary

Writable provider startup is the first possible provider effect. It follows durable
intent, even if initialization or the later lifecycle request fails. A private
macOS launcher takes an unforgeable handoff token and permits writes only to the
exact configured provider root plus its fresh owned scratch directory. HOME,
temporary paths and cwd use scratch; CODEX_HOME is the approved root. Environment
secrets/loader variables are cleared; network, forks and Mach lookup stay denied.
No unsandboxed retry exists. The sole native lifecycle method is `thread/unarchive`.

The observed signed binary requires its minimal app-bundle context when relocated.
The validated pin consists of the executable and exactly three bounded public
signing files: Info.plist, CodeResources and embedded.provisionprofile. Each has a
fixed digest/size. Private create-new copies inside owned scratch preserve the
original bytes; no provider transcript or configuration is copied. A standalone
copy without this signing context was killed; the minimal signed skeleton ran
version and synthetic unarchive with original bundle reads denied. Launch the
validated private image. Source identity/hash checks and private paths reduce the
race; this is not a claim of global TOCTOU elimination or protection against an
arbitrary same-user process controlling AgentMender memory/owned files.

### Results, observation and verification

Return only categorical outcomes/reasons. Before any writable provider process
starts, a proved failure may be NeverDispatched/NoEffect. Every failure after spawn
is Unknown, including initialization rejection, timeout, pipe/process failure,
malformed/wrong response and native error. Never infer rollback. Ambiguous dispatch
leaves OutcomeUnknown; it is not automatically retried or prematurely made terminal.
A validated unarchive acceptance is observed native response evidence, not success.
Journal it before independent verification.

One fresh read-only observation returns native presentation, ArtifactGraph and
IntegrityReport from the same protected snapshot. Recompute integrity; independently
check exact unique identity, profile, root and graph/native-state agreement. Do not
combine independent captures to manufacture coherence. Use the existing kernel-
attested, scrubbed read-only mirror and native positive filtered-list membership.
Direct native read needs rollout contents and fails under that boundary; M8 does
not relax privacy or copy transcript headers. List absence remains Unknown.

Successful same-run verification requires the immediate pre-intent graph baseline,
complete coherent authoritative evidence and recomputed healthy integrity without
findings/blockers/conflicts. Only the target's archived-to-active state and its
corresponding retention-root change may differ. All other semantic facts, native
identities, activity metadata, locators, relationships, rules and protection roots
remain bound. Exclude only ADR-0009 presentation fields. RPC acceptance alone,
incomplete capture, missing baseline or uncertain retained state cannot Succeed.
Unexpected established retained changes fail verification. Unknown/failed
verification protects RecoveryRequired.

### Restart and history

Reconciliation authenticates consumed v2 history, exact original binding, recovery
and sequence. It performs only read-only observation. A narrow store method can
record positive Applied evidence without rebuilding DurableIntent or regranting
approval. A still-archived target after possible dispatch is Unknown, not proof of
NoEffect. It may have changed and been reversed. Reconcile never retries.

M7 plans do not persist retained payload baselines. After restart that missing
baseline cannot be reconstructed from a hash or poststate. Positive native restore
may be observed, but retained verification remains Unknown/RecoveryRequired. Pending
EffectObserved/Verifying attempts resume observation without redispatch or duplicate
verification-start events. No durable schema migration is required; old history,
approval keys and recovery bytes retain their encoding. V1 remains inspection-only.

All compensation remains NotCompensatable. Recovery is reconciliation metadata,
not a transcript backup or promise to restore deleted state. M9 must independently
establish Claude contracts and cannot inherit Codex authority or semantics.

## Alternatives and consequences

Moving the gate or adding a public constructor weakens durable ordering. Coupling
provider-api to persistence expands every read adapter's dependencies. A new generic
executor framework is unnecessary. Unrestricted real-root observation violates the
read-only recovery boundary. List-derived cascade closure, native-error absence,
response-only success, reconstructed baselines and inferred compensation are rejected.

The deliberate cost is conservative support: real plans remain blocked, some
positive restores cannot be proved by listing, and restart cannot establish all
retained invariants. These limits are visible rather than turned into permission.

## Validation

Retain all prior gates and assertions. Add owned-gate construction/move failures,
exact profile and unsupported-operation tests, journal/approval/recovery execution
tests, same-capture post-verification and retained-state properties, crash cuts,
read-only reconciliation, bounded transport/error/privacy tests, isolated native
evidence and exact-final-commit Linux/macOS/Windows CI. Fitness permits only the
reviewed private lifecycle launcher and fixed owned binary pin writes; rejects
provider storage mutation, generic dispatch, Claude effects and MCP authority.
