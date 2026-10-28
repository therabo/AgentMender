# AgentMender engineering constraints

Read all docs/adr before changing boundaries. Current scope is M2: a read-only,
evidence-derived graph reconstructed from discovery observations.
No lifecycle mutation, arbitrary command/SQL dispatch, provider-state writes,
approval, recovery, cleanup plans, deletion closure or integrity-repair engine is
permitted. Provider-native authority is not replaced by graph inference.

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

Run cargo fmt --check, cargo clippy --locked --workspace --all-targets -- -D warnings,
cargo test --locked --workspace, cargo build --locked --workspace,
python3 scripts/check_architecture.py and git diff --check. Real tests are explicit,
separate from deterministic tests, and never required in CI. M2 work belongs on
milestone/m2-artifact-graph; do not begin M3 or integrate later milestones.
