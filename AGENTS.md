# AgentMender engineering constraints

Read docs/adr before changing boundaries. The current cycle is M0/M1 only.
Do not add lifecycle mutations, arbitrary command execution, SQLite writes,
provider-file writes, approval, recovery or automatic cleanup.
Unknown provider versions and layouts remain audit-only. Fixture provenance must
not be presented as verified live compatibility. Provider subprocesses require
OS-enforced write denial; there is no unsandboxed retry.
Domain has no provider adapters or filesystem logic. Application calls provider
ports. CLI composes adapters and calls application use cases.
Run cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings,
cargo test --workspace and python3 scripts/check_architecture.py.
Real-provider tests are opt-in, strictly read-only and never required in CI.
