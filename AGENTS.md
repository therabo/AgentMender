# AgentMender engineering constraints

M9 blocker evidence is formally integrated at
`9daa237ac6eadeb3d74e71b19ba31bcd6eef97ef`; capability stays BLOCKED BY PROVIDER.
Current scope is M8.1 on `milestone/m8-1-codex-full-cleanup`. Investigate current
Codex in private isolated state only. Preserve exact historical M8 pins and
independent approval/durable-intent/revalidation gates. Archive/delete need complete
pre-dispatch effects and authoritative postconditions, otherwise Unsupported.
No NLP layer or M10 work. See docs/milestones/m9-integration.md.

M8 is formally integrated at `83f0692cdda33bc3be34146ab8ceefab5dadf1f3`.
M9 capability status: BLOCKED BY PROVIDER. Its investigation is CLOSED FOR CURRENT
PROVIDER CONTRACT on `milestone/m9-claude-lifecycle`; integration retains blocker
evidence and permanent guards, not a completed lifecycle capability. ADR-0015
keeps zero Claude mutation operations/profiles, no dispatcher or storage fallback.
Installed Code 2.1.222 recognizes project purge, but isolated native dry-run itself
writes configuration/backups. Never invoke it against real state before durable
intent. Existing heuristic projects do not supply native purge identity or closed
effects. Current official Code 2.1.314 (latest) and 2.1.308 (delayed stable channel)
are independently investigated research candidates, not installed or enabled profiles.
Their evidence is in compatibility/claude/lifecycle-m9/current-2026-10-30/.
Do not widen session targets, introduce a fake project executor, or change
Codex semantics. Evidence/guard tests do not establish M9 lifecycle completion.
Historical milestone constraints below remain context except where later ADRs apply.

Current development scope is M8 on `milestone/m8-codex-lifecycle`, after M7 was
formally integrated at `5cf94c01d1d14acf6034d7bb7f5521547ee470dd`.
ADR-0013 supersedes the historical M7-only execution prohibitions below only for
its narrow Codex unarchive handoff. Preserve the older milestone boundaries as
context. Archive/delete execution remains Unsupported due hidden descendant scope;
Claude and MCP remain read-only. Current real plans retain all readiness blockers.
Only application lifecycle orchestration may advance consumed transactions toward
the private, receipt-gated Codex adapter. No effect before durable intent, verified
recovery, independent approval and immediate fresh validation. No raw commands,
provider SQL writes or filesystem lifecycle emulation. Reconciliation is read-only
and never reconstructs dispatch authority. Unknown outcomes never trigger retries.
Only the exact reviewed private executable pin may write fixed files under a fresh
owned evidence sandbox. Native process startup counts as a possible effect.

Read all docs/adr before changing boundaries. Current scope is M7: independent
local approval (ADR-0012), following formally integrated M6 at
f2268d2cdcff961a91e079af79314571fb3752d7. M5 remains a thin
read-only stdio adapter over queries, integrity and immutable lifecycle intentions.
No lifecycle mutation, arbitrary command/SQL dispatch, provider-state writes,
deletion closure or integrity-repair engine is
permitted. Persistence writes are allowed only through the explicit private owned
root capability in persistence, disjoint from protected provider state. No effect
executor or generic command/SQL/file-write payload exists. Provider-native authority
is not replaced by graph inference. Integrity
reports never decide deletability. See ADR-0008 for negative-proof rules and
ADR-0009 for canonical identity, complete effects contracts and future validation.
Plans are not authorization. Ready is not approved. Only semantic native session
intentions may be represented; no file/SQL/command mutation primitive is allowed.
Current real profiles have no closed effects or integrity contract and cannot Ready.

Unknown versions/layouts remain protected/audit-only. Installation presence,
compatibility, evidence status and coverage are separate facts. A PATH miss is
NotDetected, never proof the provider is absent. Current app versions do not prove
which historical binaries produced metadata. Fixtures retain explicit provenance.

Provider processes require OS-enforced real-state write denial; there is no
unsandboxed retry. Temporary writes are permitted only to the opaque fresh owned
evidence sandbox described by ADR-0003. Source SQLite is read-only and only opened
in the isolated worker after kernel-denial verification (ADR-0004). Mirror SQL is
in-memory/disposable only; this is not recovery or provider repair.

Workspace unsafe_code remains forbidden. The provider-api crate has one explicitly
reviewed macOS C-ABI query module; do not expand that exception without an ADR and
safety review. Source fitness checks reject other unsafe code and allowances.

Observation is not inference. Paths are not ownership. Partial discovery never
proves exclusivity or orphanhood. Preserve provenance, uncertainty, conflicting
observations and missing physical identity explicitly. Graph transformations must
be deterministic for a fixed snapshot and must not read state or sample the clock.
No current provider rule proves exclusive lifecycle ownership; all graph nodes
remain protected. Provider-specific rule interpretation belongs in adapters.

Domain has no adapters/filesystem logic and owns shared approval eligibility.
Application obtains fresh planning inputs through read-only provider ports. CLI
composes the independent human broker and protected persistence adapter, which
enforces the domain predicate again before signing. CLI
composes adapters and invokes use cases, with a fixed internal capture-worker
entry point only. Do not add a general SQL or file-write interface.
Planning-context hooks and helpers are pure. Never infer exhaustive effects from
graph reachability, unrelated authoritative evidence or native protocol schemas.
Bind dedicated effects rules to the exact capability. Preserve conflicting
descriptors before validation; malformed claims cannot disappear.
Plan identity binds structured evidence, typed scope and original conflicts, not
presentation prose. Fresh validation replans; a matching hash is not authority.

Journal intent and flush before any future effect; observe before recording results.
A missing outcome is neither failure nor success. RecoveryRequired/OutcomeUnknown
never trigger automatic retries or compensation. Required recovery precedes intent.
Transactions and digests are not authorization. M7 binds authenticated, short-lived,
one-shot approval to an exact plan, v2 transaction and recovery digest. Legacy v1
attempts remain inspection-only. V2 consumption and durable intent form one signed
event; receipts never prove provider effects. M8/M9 own provider mutation.
Preserve M5's eight read-only MCP tools unchanged. MCP cannot request grant,
revocation, consumption, persistent preparation or key access. Only the independent
local interactive approval composition may issue grants; no boolean, --yes/force,
JSON or noninteractive bypass. Terminal possession is not proof of personal identity
against a same-user shell/PTY adversary. Approval never overrides blockers or fresh
execution-time validation. Exact review plans may persist as bounded typed metadata
inside v2 Created records; no graph/snapshot/provider bodies or arbitrary imports.

Run cargo fmt --check, cargo clippy --locked --workspace --all-targets -- -D warnings,
cargo test --locked --workspace, cargo build --locked --workspace,
python3 scripts/check_architecture.py and git diff --check. Real tests are explicit,
separate from deterministic tests, and never required in CI. M7 work belongs on
milestone/m7-independent-approval after verified M6 integration base
f2268d2cdcff961a91e079af79314571fb3752d7. MCP is an inbound interface, never an
authorization authority. No provider execution, repair execution, persistent MCP
state, HTTP/OAuth transport or later milestone implementation is permitted.
