# AgentMender

## Find the conversation that answers a question

Ask Codex “Find the MISSION1 conversation where we implemented and validated
the Earth View camera limits.” The read-only `session_find` tool takes
that request as its only required argument. AgentMender owns the search job
and uses [SearchIntent v1](docs/intent-aware-resolution.md) and source-bound evidence.
One Luna Medium Planner interprets the request and closes before Rust scans
the full frozen UUID cohort. A distinct Luna Medium Evaluator assesses
candidates requiring semantic judgment on one thread. Rust verifies metadata
matches without opening their transcripts and signs fresh results after any
Evaluator closes. A topical project mention
leaves cross-project discovery open. The default returns all supported
matches within the reported coverage.

Use `session_discover` for deterministic inventory and possible deletion
candidates. Use `session_context` with a native UUID to open or refresh a
Codex transcript independently of a search run. Discovery excludes the
apparent calling chat and produces read-only results.
See the [resolution workflow](docs/intent-aware-resolution.md),
[search contract](docs/session-search.md) and
[autonomous search acceptance](docs/milestones/autonomous-session-find.md).

## Delete one selected Codex session

Ask Codex to delete a session from the search results. AgentMender checks the
selected UUID and signed evidence, then the MCP client asks once for confirmation,
showing the title, UUID and possible deletion of spawned subagents. On acceptance,
AgentMender runs `codex delete <UUID> --force` using the configured executable
and original Codex home. The result is `succeeded` or `failed` with the UUID.
These states report the command outcome. A failure can follow partial effects.
See the [MCP client requirements](docs/mcp.md#native-deletion-request).

## P0-SR3 progressive history

Compressed sources are no longer refused because their total size exceeds 64 MiB.
The existing isolated worker retains bounded decoder state across requests. All
five known oversized histories and the same 330 identities reach physical EOF;
all 330 remain logically Partial. Start/forward/backward/occurrence navigation,
verified jumps and explicit replay continuations use the existing tools.
See [traversal and operational limits](docs/progressive-history.md) and
[live Desktop MCP acceptance](docs/milestones/p0-sr3-live-mcp-acceptance.md).

`session_search` defaults to management and refuses with zero candidates
until an authenticated host route exists. Diagnostics require explicit
`purpose:"diagnostic"`; they are not manageable-session lists. Real caller
identification remains **OPEN**; [the missing host contract](docs/mcp-caller-identity.md)
is explicit and separate from native delete selection and consent.

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

P0-C1 deletion is **OPEN**. Real conversational metadata search was accepted in
P0-S1. Intelligent retrieval now adds typed Boolean/date/project queries,
three-valued negatives, complete supported transcript traversal with continuation,
source-bound context navigation, FTS5 and opt-in private indexing. Fixture and
real-provider/client acceptance are separate; see the
[implementation report](docs/milestones/intelligent-search-implementation.md).
Automatic caller identity remains **OPEN**. Production guaranteed management
search returns `caller_identity_required` with zero candidates because the installed
MCP transport does not attest exclusive chat routing. A privileged declaration
alone cannot admit it. Diagnostic browsing is not a manageable deletion list.

Read-only search remains independent of destructive compatibility. The native
Codex delete MVP is separate from historical durable removal and its acceptance
criteria. There is no Claude mutation profile.

## What works today

| Capability | Status |
|---|---|
| Provider discovery, scan and inspection | Implemented with explicit partial coverage and exact compatibility limits |
| Codex inventory | Real metadata search with validated column/value contract; historical App Server scans remain separate |
| Claude inventory | Partial observed Desktop sidecar metadata; synthetic Code fixtures; no general live SDK inventory |
| MCP | Eight default read-only tools. Explicit `--session-search` adds caller-scoped search and context, plus human-confirmed native Codex deletion. |
| Session search | Typed Boolean/time/project filters, unknown-safe negatives, FTS5 passages and bounded continuation. The read-only resolver uses a frozen UUID cohort, Luna assessments and source-bound publication. |
| Management-session exclusion | Guaranteed real mode refuses with zero candidates until enforceable host binding exists; typed fixture exclusion precedes counts/ranking/paging/context; launch-protected diagnostic identities stay excluded |
| Human approval and durable transactions | Implemented local terminal review, one-shot authenticated grants, journal and read-only reconciliation |
| Codex primary cleanup | M8.2 PASS for exact 0.166.0/macOS arm64, unshared Legacy plain history and reviewed WAL schema |
| Native Codex mutation | Single-session delete after MCP client confirmation; separate historical unarchive handoff |
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
Historical session lifecycle and artifact cleanup use the protected execution
engine below. The native delete MVP uses the separate
[ADR-0038 decision](docs/adr/0038-native-codex-delete-admission.md#mvp-decision--2026-11-13).

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

## Connect the existing Codex installation

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
