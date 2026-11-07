# AgentMender

AgentMender is an **MCP-native session lifecycle manager for AI coding agents**.
Its initial target providers are **OpenAI Codex and Anthropic Claude Code**.
It aims to help users find and safely manage old conversations through a
natural-language request to their coding agent.

The calling agent interprets the request. AgentMender supplies structured
operations, exact provider-specific identity, independent human approval and
verified results. It protects other sessions and shared state throughout.

## Intended conversational workflow

> “Find the old Codex session where we worked on the MISSION1 README.”
>
> AgentMender returns matching metadata and resolves ambiguity. The user selects
> one exact session, previews removal, and approves its immutable plan through
> the trusted human channel. After execution and structural verification, the
> agent explains the result and any remaining artifacts.

P0-C1 is **OPEN**. An opt-in Codex workflow now searches metadata, previews an
exact primary removal, prepares its durable transaction and recovers status.
The human approves and executes through the existing external-terminal path.
Actual conversational client use with independent human approval remains an
acceptance gate. See [the workflow and limits](docs/conversational-codex-removal.md).
Cross-provider deletion is not implemented.

## What works today

| Capability | Status |
|---|---|
| Provider discovery, scan and inspection | Implemented with explicit partial coverage and exact compatibility limits |
| Codex inventory | Historical exact App Server profiles; unknown versions receive metadata audit only |
| Claude inventory | Partial observed Desktop sidecar metadata; synthetic Code fixtures; no general live SDK inventory |
| MCP | Eight default read-only tools; five opt-in Codex search/preview/preparation/status/external-handoff tools |
| Session search | Opt-in exact stable-profile title/project/UUID metadata search; duplicates retained; no transcript search |
| Human approval and durable transactions | Implemented local terminal review, one-shot authenticated grants, journal and read-only reconciliation |
| Codex primary cleanup | M8.2 PASS for exact 0.166.0/macOS arm64, unshared Legacy plain history and reviewed WAL schema |
| Native Codex mutation | Conditional historical unarchive handoff; real native plans remain blocked; archive/delete unsupported |
| Claude mutation | No enabled adapter/profile; investigated project-purge contracts remain unsafe for session removal |
| Complete Desktop removal | Unfinished/deferred; schema-4 candidates unadmitted; unsafe shared global writes refused |
| Bulk management / broad artifact cleanup | Deferred |

M8.2's verified primary removal and logical unlinked bytes do not prove every
artifact is gone, immediate GUI refresh, physical storage reclaim or secure
erasure. Schema-3 expanded primary cleanup has disposable execution evidence;
its stronger Desktop assessment remains incomplete. See the
[capability audit and MVP gaps](docs/audits/2026-11-07-product-first-audit.md).

## Architecture and safety

Eight Rust crates keep the domain, read-only provider ports, application services,
Codex/Claude adapters, persistence, CLI and MCP interface separate.
[Architecture](docs/architecture.md) describes the current dependency boundaries.
Core session lifecycle and optional deep artifact cleanup share the existing
protected execution engine; exhaustive Desktop cleanup is not an MVP prerequisite.

- Exact provider/root/session identity and fresh immutable plans bound scope.
- Native provider operations are preferred when their exact-version effects fit
  approval. Otherwise only a separately supported scoped fallback is eligible.
- The calling model cannot supply independent destructive approval. The existing
  local terminal channel has a documented same-user shell/PTY limitation.
- Durable intent precedes effects; interrupted or uncertain operations are never
  automatically retried. Recovery metadata is not a transcript backup.
- Shared, global, ambiguous and unknown state stays protected. Cwd and project
  membership are discovery evidence, never destructive ownership.

## Product roadmap

The [canonical roadmap](docs/product-roadmap.md) defines measurable phase gates:

1. **P0:** conversational single-session management, first one complete removal
   path, then both Codex and Claude Code.
2. **P1:** native lifecycle operations and broader exact provider coverage.
3. **P2:** additional artifact cleanup and storage reclamation.
4. **P3:** project and multi-session lifecycle.

M8.2.1 remains **unfinished/deferred**, with its safety findings preserved. It is
**no longer a global blocker for the conversational MVP**. M8.3 bulk cleanup is
not the next implementation priority. [ADR-0022](docs/adr/0022-product-first-session-lifecycle.md)
records the change; [milestone receipts](docs/milestones/README.md) retain history.

## Build and try read-only fixtures

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
