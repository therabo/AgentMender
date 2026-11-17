# AgentMender

AgentMender is a local browser session manager for **Codex** and **Claude Code**.
Browse verified main conversations by provider and search titles, IDs and
workspaces. On supported Codex releases, preview and confirm irreversible
removal of one archived main conversation and its verified spawned descendants.

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
| Codex | Partial main-conversation inventory and bounded transcript pages on macOS. Native cascade removal under the [verified release, command and storage contract](docs/provider-compatibility.md). |
| Claude Code | Partial read-only inventory of verified main conversations in project JSONL, including Desktop-origin Code records. |

The review lists the main UUID, verified descendants and expected session count.
After dispatch, AgentMender checks canonical rows, rollout files and spawn edges.
Desktop visibility, other provider artifacts and reclaimed disk space remain
outside those checks.

[Launch options and API](docs/local-session-manager.md) ·
[Architecture](docs/architecture.md) · [Roadmap](docs/product-roadmap.md) ·
[Cleanup and validation report](docs/milestones/ui-first-cleanup.md)
