# AgentMender engineering constraints

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

Domain has no adapters/filesystem logic. Application obtains fresh planning inputs
through read-only provider ports and routes independent approval through explicit
approval ports. CLI
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
