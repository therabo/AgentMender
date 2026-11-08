# AgentMender — Repository Consolidation & Codebase Cleanup

Dated implementation/audit receipt, 2026-11-08 (Europe/Rome). The README owns
current capability status and the product roadmap owns sequencing. Exact final
Git/CI/deletion outcomes are recorded after commit in the external delivery
receipt under `/Users/daniel/Desktop/AgentMender-backups/2026-11-08-consolidation/`;
this avoids pretending a source document can contain its own final commit hash.
No real conversation deletion, real provider-state mutation or app restart is authorized.

## A. Git consolidation

Starting clean local/remote development HEAD:
`feature/p0-c1-real-world-adaptive-compatibility`,
`b54222b2f65a40ca56f7bd0e42f37e8369cb9213`. This includes the adaptive real-session
reader and the user's later accepted conversational metadata search. Original
local/remote main: `a284767157ace8769deee85b195b5b70efe374d8`. GitHub's original
default was `milestone/m0-m1-read-only`, verified through remote symbolic HEAD.
There was one worktree, 16 local and 16 matching remote branches, no tags, and no
commits in any branch tip outside the starting development history. No divergent
integration, cherry-pick, reset, orphan root or whole-history squash is needed.

All original refs are preserved in `original-all.bundle` outside the repository,
SHA-256 `558bfd3b712b1189a7fe87857389286d618693e9c45d163cbed37eb9744eda2b`.
`git bundle verify`, an independent mirror restoration, exact comparison of all
32 original refs and restored `git fsck --full` passed. The external
`original-refs.json` also records worktree/status/remotes. The committed
[branch manifest](2026-11-08-original-refs.json) lists every original name/SHA and
its annotated `archive/2026-11-08/<original-branch>` tag, including original main.
All 16 tags were dereferenced and compared with their original tips.

The authorized procedure is: commit cleanup on the existing development line,
fast-forward main, normal push, supported GitHub default-branch switch, exact
final Linux/macOS/Windows CI, then verified explicit retirement of the 15 ordinary
development branches locally/remotely. No extra worktree or development branch
is created. Preserve all original historical commits and tags. No wildcard
remote deletion, force push, protection weakening or deletion while CI is unknown.
The final external receipt lists actual removed/exceptional refs and clean status;
the original manifest is an inventory, not a claim that deletion has happened.

## B. Code audit

All 64 Rust source modules across eight crates are classified with consumers and
retention reasons in the [module inventory](2026-11-08-module-inventory.md).
The baseline has 88 Rust source/test files and 46,590 Rust lines. Inspection uses
module/export references, Cargo declarations, direct source uses, cfg/platform
paths, test inventory, source-fitness checks, warning-denied Clippy and Git history.
This is a maintainability audit, not a new independent security certification.

Removed `scripts/prepare_p0_c1_readonly_fixture.py` (91 lines): a standalone
synthetic onboarding generator with no CI/test caller, superseded by real-provider
onboarding and existing deterministic application/stdio fixture builders. Its
historical implementation remains recoverable at `d58311b64580bd4213bf928d64888e563aa0755f`.
No tracked generated provider root or development-only binary was found.

Removed the unnecessary module-wide `allow(dead_code)` on scoped-files safety
code: warning-denied local Clippy passed without it. Removed two redundant internal
protected-result flags, the projection's always-false argument/dead branch, and
the search's redundant admission predicate. Existing JSON fields remain compatible.
Shared search now filters registered exact-root identities before matching and
projection, rather than presenting protected records as candidates. Protected
read-only preview and missing-registration removal-mode search now refuse.

No dependency, crate, lockfile package, provider adapter, native protocol version,
approval engine, recovery schema, plan hash domain or admission profile is removed
or introduced. Every direct dependency has actual source/test/target uses. No
unused production function was demonstrated after lifting the suppression; the
obsolete launcher from ADR-0022 was already absent before this task.

Retain distinct native/scoped/expanded handoffs and outcomes because they carry
different approved effects, old serialization and replay semantics. Keep Desktop
catalog/global/legacy/index candidates for exact compatibility evidence and P1/P2
reuse, with empty schema-4 admission and target global publication refusal. The
global-state lost-update child process, physical/ownership fences and Windows
permissions wrappers are significant safety consumers, not obsolete abstractions.

## C. Tests and fixtures

| Group | Behavior protected / decision |
|---|---|
| Domain unit/property tests and seeds | Exact identity, graph evidence, immutable plans, scope, approval, replay and old wire encodings; retain |
| Application integration tests | Exact selection, supported orchestration, stale/unknown outcomes and no retry; retain; update search expectations and add two protection regressions |
| Persistence unit/crash/reopen tests | Authenticated grants, one-shot consumed intent, journal durability, recovery/unknown knowledge and platform permissions; retain |
| Codex provider tests and actual SQLite fences | Exact schema/version/ownership/writer admission, retained-session preservation, native descendant effects and global lost-update refusal; retain |
| Claude fixtures and lifecycle guards | Literal identity, traversal, sidecars, partial/unknown data and zero mutation profiles; retain |
| CLI/stdio/MCP protocol tests | Tool catalog, bounds/privacy, strict arguments, no authority bypass, real worker isolation and disposable approved effects; retain; replace protected-candidate expectation with exclusion/refusal |
| 44 provider fixture files | Known/unknown/partial/duplicate/malformed contracts and provenance; useful deterministic consumers, retain |
| 133 compatibility files and schema snapshots | Historical exact pins, native counterexamples, validators and immutable evidence hashes; retain byte-for-byte |
| Synthetic onboarding generator | No automated consumer; useful invariants already covered by application/stdio tests; retire generator, preserve history |
| Native probes/capture/live validators | Separate opted-in reproducibility and non-mutation evidence, not redundant CI; retain without running destructive probes on real data |

No test or regression fixture file is deleted. No valid security property is
weakened to reduce counts. The old search assertion that management appeared as
`protected: true` is obsolete under the new requirement; the stdio test now proves
absence and uses a retained-plan reference to prove preview refusal independently.
New application cases exercise blank/keyword/UUID/archive searches in both modes,
same-UUID foreign-root preservation, protected preview/prepare refusal and missing
registration. The existing read-only stdio test also checks registered exclusion
with an unsupported executable, identical fixture bytes and no approval store.

macOS list inventory: **471 → 473** tests (including 8 ignored opt-ins).
Final normal suite: **465 top-level passed, 0 failed, 8 ignored**, plus 8 successful
nested child-process checks; raw output therefore totals 473 pass summaries.
Three additional selected ignored exact-profile tests pass separately: sandboxed
primary worker, provider conversational workflow and stdio/external execution/
restart status. All effects are confined to their disposable test providers;
fixture grants are not independent human acceptance.

Full fmt, locked workspace Clippy/test/build, architecture, native/complete evidence
validators and whitespace gates pass locally. An initial manual opt-in invocation
omitted `AGENTMENDER_HELPER` and refused before provider effects; supplying the
documented built helper fixed the invocation without a test/code relaxation.
Exact final CI is required separately and recorded by full SHA in the delivery
receipt, never inferred from these local results or baseline CI.

## D. Search architecture

The [search contract](../session-search.md) is canonical. Current real metadata
matching is title/display name, project basename and UUID; timestamps, archives
and parent data are observations. Results retain duplicates, opaque root namespace,
bounded metadata and Partial coverage. The accepted historical model query found
37 MISSION1 records. The cleanup-time model query found 36, still Partial, with
`authenticated_caller_binding: false`; changing live counts are not a regression
or proof of absence. The installed observed provider remains alpha.6.

Existing Codex attested SQLite and exact rollout-header readers provide safe
primitives, not a transcript search capability. Claude's validated synthetic prefix
and sidecar parser is metadata-only; unknown live transcripts remain unsupported.
Installed `thread/list.searchTerm` searches titles. Native history APIs advertise
read access, but safe initialization and exact store-format support still need
proof under source-write denial. Do not resume sessions or retry unsandboxed.

The next implementation uses a narrow adapter-owned read port, bounded streaming
of an evidenced representation, application exclusion/filtering/relevance, pure
domain evidence and short MCP excerpts/provenance. Explicit metadata filters can
narrow scope; title keyword matching must not gate body scanning. Start without
an index or external service. The contract defines per-request/session/record/time/
response caps, cancellation, Partial causes, source-bound cursors and a thousands-
of-sessions fixture benchmark. A later index needs measured need and prior privacy,
private storage, synchronization, invalidation and lifecycle decisions.

Extract supported user/assistant text only. Treat excerpts as untrusted quoted
data; historical commands never execute. Avoid credentials, auth/config, tool dumps,
hidden reasoning, attachments, full transcripts, raw paths, transcript persistence
and secret-bearing snippets. Unknown/malformed formats report unsupported/partial.
Missing implementation: format-qualified body readers, bounded evidence/ranking/
context DTOs, continuation/privacy rules, caller integration and real read-only
content acceptance. None is advertised as complete by this cleanup.

## E. Management-session exclusion

Existing trusted launch UUID registration is frozen as UserProtected graph roots,
restored from the immutable plan during CLI approval/execution and rechecked at
preflight/fresh validation. This task fixes conversational filtering and protected
read-only preview, preserving preparation/execution guards. The exact path audit
and required regression cases are in the search contract.

No authenticated automatic per-conversation identity is established. Neither MCP
clientInfo, a title/cwd/active flag, a recent UUID nor the historical search's
`Cerca sessioni MISSION1` record is caller proof. Current raw scan/graph/inspect
diagnostics can show the management conversation; they are not admitted guaranteed
management lists. Future content/project/paging/ranking/count/context paths must
exclude the verified identity before every limit and presentation, and refuse
stale/forged selection at preview/planning/execution.

Smallest missing integration: observe sanitized real host envelopes and verify a
host-authenticated provider/root/conversation binding, or independently register
the actual identity through an operator-controlled channel outside model control,
with conversation-switch/reconnect checks. Claude binding requires its own proof.
Official client thread-scoped APIs do not establish server-side authentication.
The required future failure is caller_identity_required with zero candidates;
the existing removal service uses management_protection_required for missing
registrations. No speculative authentication framework is implemented.

## F. Product status

| Capability | Implemented/tested/admitted status |
|---|---|
| Real MCP discovery | Metadata reader implemented, fixture-tested, real conversational search accepted with Partial coverage; macOS read isolation only |
| Content search | Complete contract/acceptance design; no production reader/ranking/context acceptance |
| Self-exclusion | Registered identities enforced/tested in conversational search/preview/prepare and existing plans/effects; automatic caller binding and all-route guaranteed management workflow incomplete |
| Native deletion | Installed methods observed; descendant/partial-effect counterexamples retained; current-version delete not admitted or accepted |
| Scoped local fallback | Historical M8.2 exact stable 0.166.0 Darwin arm64 Legacy/WAL primary contract retained; disposable integration passes; no authority inherited by alpha/unknown versions |
| Independent approval | Existing authenticated one-shot terminal/grant/intent/recovery engine tested; actual unrestricted same-user environment does not enforce human independence |
| Claude parity | Useful partial read-side evidence/guards retained; no mutation profile; content and host binding require separate evidence |
| Advanced artifacts | Existing exact expanded-primary components tested; Desktop complete candidates deferred/unadmitted; broad/bulk work deferred |

P0 conversational management, P1 provider coverage, P2 deeper artifacts and P3 bulk
remain the priorities. No Desktop-complete prerequisite is reinstated. Current
destructive client/human acceptance remains OPEN. No real-user deletion was tested.

## G. Recommended next milestone

**P0-S1: bounded content evidence with verified management exclusion**, read-only.
Establish one trustworthy caller integration, add one supported bounded Codex
conversation reader and grounded provenance/excerpts, then verify neutral-title
Earth/Cesium matches and strongest-match management exclusion through a real client.
Missing binding must refuse guaranteed workflows; unsupported Claude/store formats
remain explicit. Reuse the cleaned eight-crate architecture and existing isolation.
Do not widen destructive authority or start this next milestone automatically.
