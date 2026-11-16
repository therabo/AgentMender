# AgentMender

AgentMender is a local browser session manager for **Codex** and **Claude Code**.
Browse sessions by provider, search titles, IDs and workspaces, then inspect a
selected session. On the supported Codex profile, preview and explicitly confirm
removal of one archived session.

## Run locally

From the repository root with Rust and Node 22 LTS installed:

```sh
npm ci --prefix ui
npm run build --prefix ui
cargo build --locked --workspace
./target/debug/agentmender ui
```

The last command opens the default browser. Use `ui --no-open` to print the
private launch URL. The local manager uses deterministic metadata and runs
without model services, MCP or external credentials.

| Provider | Available workflow |
|---|---|
| Codex | Partial inventory and bounded transcript pages on macOS. Native single-session removal for the [exact reviewed binary and archive profile](docs/provider-compatibility.md). |
| Claude Code | Partial read-only inventory of recognized project JSONL, including Desktop-origin Code records in the configured project archive. |

The UI distinguishes observed metadata from missing values and reports provider
availability, coverage and eligibility. Native command success leaves residual
artifacts and physical storage reclaim unverified.

[Launch options and API](docs/local-session-manager.md) ·
[Architecture](docs/architecture.md) · [Roadmap](docs/product-roadmap.md) ·
[Cleanup and validation report](docs/milestones/ui-first-cleanup.md)
