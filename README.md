# AgentMender

AgentMender is a local session manager for **Codex** and **Claude Code**. Use a
browser to browse sessions by provider, search by metadata and inspect a
selected session. On a supported Codex installation, review and confirm removal
of one archived session. The local manager runs without AI models.

## Run locally

```sh
npm ci --prefix ui
npm run build --prefix ui
cargo build --locked --workspace
./target/debug/agentmender ui
```

The last command opens a loopback URL in the default browser. Use
`ui --no-open` to print the URL for manual opening. See
[setup, archive overrides and the local API](docs/local-session-manager.md).

| Provider | Local capabilities |
|---|---|
| Codex | Inventory and on-request conversation preview through the macOS confined reader. Removal for the [exact reviewed binary and archive profile](docs/adr/0042-local-session-manager.md). |
| Claude Code | Read-only project transcript metadata. Removal stays disabled until an individual-session operation is validated. |

The inventory reports partial or unavailable coverage explicitly. Transcript
size is the measured source size. Native command outcomes report what Codex
returned and whether further effects remain uncertain.

## Architecture and roadmap

The eight Rust crates retain domain, application, provider, persistence and
interface boundaries. The new loopback surface serves React, TypeScript and
Vite assets and uses existing discovery and Codex dispatch paths. The
[browser and provider boundaries](docs/adr/0042-local-session-manager.md) and
[architecture audit](docs/audits/2026-11-16-local-session-manager.md)
record the design and provenance of the change.

The [product roadmap](docs/product-roadmap.md) prioritizes the local
single-session workflow, followed by provider coverage and later artifact
attribution. Existing [milestone receipts](docs/milestones/README.md) retain
the evidence and scope of the historical CLI and MCP features. The
[local UI acceptance report](docs/milestones/local-session-manager-acceptance.md)
records the browser, HTTP and native fixture checks.

## Optional MCP search and lifecycle

Build with `cargo build --locked --workspace`, then register a STDIO server whose
command is the absolute AgentMender executable and arguments are:

```text
--provider codex mcp --session-search
```

See [session search](docs/session-search.md) for examples and query semantics.
The [private index](docs/full-text-index.md) is an explicit cache for independent search.
The resolver scans the captured cohort live and uses
[Luna Medium](docs/luna-session-relevance.md) for bounded evidence assessment.

### Conversational search and native deletion

Register the compiled executable globally in Codex and use absolute paths
for the executable, Codex home and installed Codex CLI. For example:

```toml
[mcp_servers.agentmender]
command = "/absolute/path/to/agentmender"
args = ["--provider", "codex", "--codex-root", "/absolute/path/to/.codex", "--codex-executable", "/absolute/path/to/codex", "mcp", "--session-search"]
```

The autonomous host runs on macOS with the system Python 3 and a signed-in
Codex CLI with a private file credential in the configured root. AgentMender
embeds the workflow and role profiles in its binary. Refresh the AgentMender
MCP connection after replacing the binary and verify that the new catalog
advertises `session_find` and `session_delete`. See [connection and job behavior](docs/intent-aware-resolution.md).

[Setup, current limits and human acceptance](docs/p0-c1-manual-acceptance.md).

## Development and regression fixtures

`main` is the ordinary development branch. Original branch tips are preserved in
annotated `archive/2026-11-08/*` tags and the
[consolidation manifest](docs/audits/2026-11-08-original-refs.json).
The dated [consolidation audit](docs/audits/2026-11-08-repository-consolidation.md)
records code/test retention decisions and the verification boundary.

Current documentation: this README for capability status; [architecture](docs/architecture.md)
for boundaries; [roadmap](docs/product-roadmap.md) for sequencing;
[search and management identity](docs/session-search.md) for the implemented incremental read-only
contract; [MCP](docs/mcp.md) for tools; [development](docs/development.md) for checks.
Dated audits, milestone receipts and research are historical evidence, not competing
roadmaps. [ADR-0025](docs/adr/0025-bounded-search-and-management-exclusion.md)
records the search/exclusion requirement; [ADR-0026](docs/adr/0026-bounded-conversation-reader-and-independent-registration.md) records the historical bounded reader, extended by
[ADR-0027](docs/adr/0027-intelligent-session-retrieval.md) for the current implementation.

Use Rust 1.89 or newer and the checked-in Cargo.lock. CI also tests current stable
on Linux, macOS and Windows. Live provider isolation and mutation have narrower
platform/version support than the fixture test matrix.

```sh
cargo build --locked --workspace
cargo run --locked -- --provider codex --fixture fixtures/codex/known scan --json
cargo run --locked -- --provider claude --fixture fixtures/claude/4.29.0-observed --desktop-fixture scan --json
cargo run --locked -- --provider codex --fixture fixtures/codex/known verify --json
```

Use an ID returned by `scan` with `inspect <id>`. To connect the current read-only
MCP server, configure a client to launch `/absolute/path/to/agentmender mcp`.
See [MCP setup, tools and limits](docs/mcp.md) for configuration and fixture mode.

Development checks:

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
python3 scripts/check_codex_lifecycle_evidence.py
python3 scripts/check_complete_removal_evidence.py
git diff --check
```

Normal tests use sanitized/disposable fixtures. Real provider probes are separate
opt-ins. Development never deletes real user sessions or restarts the application.
Read [AGENTS.md](AGENTS.md) before changing safety boundaries. Detailed contracts:
[planning](docs/planning.md), [integrity](docs/integrity.md),
[approval](docs/approvals.md), [transactions/recovery](docs/transactions.md),
[scoped cleanup](docs/scoped-session-cleanup.md),
[Codex](docs/codex-lifecycle.md), [Claude](docs/claude-lifecycle.md),
[compatibility](compatibility/codex/README.md), and [ADRs](docs/adr/README.md).
