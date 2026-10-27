# AgentMender

A provider-aware lifecycle, forensic and integrity foundation for AI coding-agent
state. This implementation exposes **read-only inventory only**.

```sh
cargo run -- providers
cargo run -- scan --provider codex --json
cargo run -- --provider codex --fixture fixtures/codex/known scan --json
cargo run -- --provider claude --fixture fixtures/claude/2.1.311-synthetic scan --json
cargo run -- --provider codex --fixture fixtures/codex/known inspect codex:session:019a0000-0000-7000-8000-000000000001 --json
```

Use an ID returned by `scan` for `inspect`. Provider roots default to `CODEX_HOME`
and `CLAUDE_CONFIG_DIR`, otherwise `$HOME/.codex` and `$HOME/.claude`. Override
with `--codex-root` / `--claude-root` and, when needed, absolute
`--codex-executable` / `--claude-executable` paths. Live roots and `--project`
scopes must be absolute. `--max-entries` bounds traversal; `--active-only` works
only where archive membership is known. Fixture mode is explicit and never
asserts an installed provider version.

JSON reports go to stdout and structured errors to stderr. Exit 0 means the
requested report is complete for its declared boundary; exit 2 means invalid
input, missing capability, partial inventory or failure. Inspecting an entity
from incomplete discovery remains partial. Incomplete scans cannot prove absence.
Reports include local paths and identifiers; transcript bodies are excluded.
UTF-8 paths are strings; non-UTF-8 paths use a lossless native encoding object.

## Current evidence and release gates

| Provider | Verified evidence | Current live behavior |
|---|---|---|
| Codex | Exact generated schema for installed `0.164.0-alpha.11.1`; synthetic protocol fixtures | Write-denied App Server attempt; protected file metadata fallback when startup fails |
| Claude Code | Official SDK/storage documentation; synthetic `2.1.311` parser contract | Metadata audit only; no live transcript parsing |

M0/M1 infrastructure is implemented, but **their full release gates are pending**.
Codex App Server failed to initialize SQLite in an empty write-denied fixture.
Successful live thread enumeration is not assumed. Claude is not installed in the
validation environment, and no exact-release capture validates its live parser.
The smallest next step is to establish these provider evidence gates before M2.
See [Codex evidence](compatibility/codex/README.md) and
[Claude evidence](compatibility/claude/README.md). Unknown versions never inherit
compatibility by semantic-version proximity. Synthetic fixtures are not real
provider captures.

## Safety boundary

No archive, delete, purge, repair, provider SQL, filesystem writes, cleanup
execution or approval capability exists in production code. All artifacts are
protected; paths never prove ownership. Version probes and Codex App Server run
under macOS OS-enforced file-write, network, process-fork and Mach lookup denial.
Failure never triggers an unsandboxed retry. Linux and Windows process integration
remains unavailable until equivalent isolation is verified; metadata audit works.

Only initialize, initialized and thread/list are sent to Codex; list always sets
useStateDbOnly:true, because its default behavior can repair metadata. Lifecycle
success and provider health are never inferred from this inventory.
The guarantee covers application-requested provider content/metadata writes.
Filesystem reads can affect access timestamps and OS caches; unrelated provider
writers can continue changing state. These observations are not atomic snapshots,
TOCTOU-proof deletion boundaries or future ownership evidence. Cross-platform
path/reparse and concurrent replacement hardening remain later work.

## Development

Six crates enforce ports and adapters: `domain`, `provider-api`, `application`,
`provider-codex`, `provider-claude`, and `cli`. CLI composes adapters and calls
application use cases. Application imports only domain and provider ports.
See [ADR-0001](docs/adr/0001-read-only-foundation.md) and
[dependency decisions](docs/dependencies.md).

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_architecture.py
```

Normal tests need no installed provider and never target real provider homes.
Two live smoke tests are ignored by default and require explicit root/executable
environment variables; see [test instructions](crates/cli/tests/README.md).
CI checks Linux, macOS and Windows; configuring that matrix is not evidence that
all remote jobs have passed. Deterministic ordering applies to complete fixed
observations; a truncated directory sample can vary with filesystem enumeration.
