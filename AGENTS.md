# AgentMender engineering constraints

Read all docs/adr before changing boundaries. Current scope is M6: durable
AgentMender-owned transaction/recovery knowledge (ADR-0011). M5 remains a thin
read-only stdio adapter over queries, integrity and immutable lifecycle intentions.
No lifecycle mutation, arbitrary command/SQL dispatch, provider-state writes,
approval, deletion closure or integrity-repair engine is
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

Domain has no adapters/filesystem logic. Application calls read-only ports. CLI
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
Transactions, digests and durable receipts are not authorization. M7 owns approval;
M8/M9 own provider mutation. Preserve M5's eight read-only MCP tools unchanged.

Run cargo fmt --check, cargo clippy --locked --workspace --all-targets -- -D warnings,
cargo test --locked --workspace, cargo build --locked --workspace,
python3 scripts/check_architecture.py and git diff --check. Real tests are explicit,
separate from deterministic tests, and never required in CI. M6 work belongs on
milestone/m6-durable-transactions after verified M5 integration base
c426a7fe4e12c24afc2f383087912e7f54111b39. MCP is an inbound interface, never an
authorization authority. No execution, approval, repair execution, persistent MCP
state, HTTP/OAuth transport or later milestone implementation is permitted.
