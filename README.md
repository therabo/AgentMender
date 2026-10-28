# AgentMender

Provider-aware, strictly read-only inventory of AI coding-agent state. The local
M0/M1 closeout is verified with accurately reported **partial coverage**. No M2
algorithms, lifecycle mutations, plans, approvals, repair or recovery are present.

```sh
cargo run -- providers --json
cargo run -- scan --provider codex --json
cargo run -- scan --provider claude --json
cargo run -- --provider claude --fixture fixtures/claude/4.29.0-observed --desktop-fixture scan --json
cargo run -- --provider codex --fixture fixtures/codex/observed-snapshot scan --json
```

Use an ID returned by `scan` with `inspect <id>`. JSON reports go to stdout;
structured errors go to stderr. Exit 0 means a report is complete for its declared
boundary; exit 2 means partial/unavailable inventory or an input/provider error.
Partial inventory never proves absence, ownership, liveness or safe deletion.
Reports include local paths/identifiers but exclude conversation bodies.

## Verified local evidence

| Provider | Installation | Compatibility / evidence | Discovery |
|---|---|---|---|
| Codex | Bundled CLI `0.165.0-alpha.1.2` | Separately validated exact protocol + state schema; SnapshotVerified when source is stable | 166 native summaries in the recorded refresh observation; partial across stores |
| Claude | Desktop `4.29.0`, bundled Code `2.1.222` | Observed Desktop metadata context; RealPartial | Seven Desktop sessions and explicit CLI mappings; other history remains protected |

These counts are observations, not future test expectations. Claude Desktop owns
its Code binary outside PATH; not finding `PATH/claude` never establishes provider
absence. Every installation has its own surface and version provenance. A
candidate binary is not presumed to be the active runtime. Compatibility,
evidence provenance and collection completeness are separate fields.

The historical exact Codex `0.164.0-alpha.11.1` profile is retained. The refresh
binds each executable version to its independently captured source-schema hash;
neither version accepts the other's layout. A later sampling run observed source
WAL/SHM changes and retains RealPartial/SnapshotStale provenance. Compatibility
never establishes current liveness, complete discovery, or authority to mutate.

See [Codex evidence](compatibility/codex/README.md),
[Claude evidence](compatibility/claude/README.md), and [ADRs](docs/adr/README.md).
Observed fixtures are sanitized samples; synthetic fixtures remain labeled
SyntheticOnly. Neither is presented as fresh live evidence.

## Roots and scope

CLI state roots default to CODEX_HOME / CLAUDE_CONFIG_DIR, otherwise
`$HOME/.codex` / `$HOME/.claude`. `--codex-root`, `--claude-root`, and executable
overrides require absolute local paths. On macOS, default Claude discovery also
checks known app bundles and `$HOME/Library/Application Support/Claude`.
An explicit Claude state root or CLAUDE_CONFIG_DIR disables implicit Desktop
reads. Opt in separately with `--claude-desktop-root <AppSupport/Claude>` and
`--claude-app-bundle <Claude.app>` when using explicit roots.

`--max-entries` bounds enumeration. `--project` matches an absolute observed
provider path, and `--active-only` filters only known archive presentation state.
Archive presentation never implies a quiescent writer. UTF-8 paths serialize as
strings; non-UTF-8 paths use a lossless native encoding object.

## Safety boundary

The application never writes real provider state. Read-only ports expose only
detect/discover. All artifacts remain protected, and paths do not prove ownership.
No arbitrary SQL, provider command, archive/delete/purge/repair or approval tool
exists. Codex uses a consistent SQLite Backup API snapshot, scrubs content in RAM
before persisting a private disposable copy, and runs official thread/list there.
Real state remains write-denied. The native process cannot read real-home file
contents, but may stat metadata required by this exact build. No credentials,
configuration, history/log database or rollout contents are copied.

The only writable runtime directory is a fresh AgentMender-owned `0700` evidence
home; its database is `0600`. Child processes are reaped before disposal. The
capture worker verifies kernel source-write denial before SQLite access. Network,
process-fork and Mach lookup are denied. Failure never relaxes permissions.
A narrow macOS C-ABI policy query is isolated and reviewed in ADR-0004; other Rust
code remains forbidden from unsafe operations. Linux/Windows native-process
isolation is unsupported and falls back explicitly to protected metadata audit.

The guarantee concerns application-requested provider content/metadata changes.
OS access times/caches and independent writers are outside it. A SQLite snapshot
is not an atomic capture of all stores, a recovery point or proof of provider
health. Truncated directory samples can vary with enumeration order.

## Development and verification

Rust 1.89+; six crates retain ports and adapters. Application depends only on domain
and provider ports; CLI composes adapters and calls application use cases.
Dependencies and the scoped unsafe boundary are documented in
[dependency decisions](docs/dependencies.md).

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
```

Normal tests use isolated fixtures and require no installed provider. Explicit
live fingerprint verification and the older outer-denial probes are documented in
[test instructions](crates/cli/tests/README.md). CI has a Linux/macOS/Windows matrix;
local results do not establish that remote jobs have passed. M2 has not begun.
