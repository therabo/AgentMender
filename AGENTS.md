# AgentMender engineering constraints

Read all docs/adr before changing boundaries. Current scope is M0/M1 only.
No lifecycle mutation, arbitrary command/SQL dispatch, provider-state writes,
approval, recovery, ownership resolution or M2 graph algorithm is permitted.

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

Domain has no adapters/filesystem logic. Application calls read-only ports. CLI
composes adapters and invokes use cases, with a fixed internal capture-worker
entry point only. Do not add a general SQL or file-write interface.

Run cargo fmt --check, cargo clippy --locked --workspace --all-targets -- -D warnings,
cargo test --locked --workspace, cargo build --locked --workspace,
python3 scripts/check_architecture.py and git diff --check. Real tests are explicit,
separate from deterministic tests, and never required in CI. Do not merge into main
automatically or begin M2 during the closeout cycle.
