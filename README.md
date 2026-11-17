# AgentMender

AgentMender is a local browser session manager for **Codex** and **Claude Code**.
Expand projects to browse verified main conversations. Search projects,
workspaces, titles and session IDs. On supported Codex releases, preview and
confirm removal of one archived conversation or all verified conversation
trees in a workspace group.

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
| Codex | Workspace groups, partial main-conversation inventory and bounded transcript pages on macOS. Native cascade removal under the [verified release, command and storage contract](docs/provider-compatibility.md). |
| Claude Code | Namespace groups and partial read-only inventory of verified main conversations in project JSONL, including Desktop-origin Code records. |

A project review lists the main conversations, descendants and exclusions.
For Codex, the workspace identifies a group of archive records and source
files stay outside the removal scope. Codex deletes each reviewed tree in
sequence. AgentMender verifies canonical rows, rollouts and spawn edges after
each command and stops if the next scope changes or a result is uncertain.
Those checks measure archive state. Desktop visibility and physical disk space
reclaimed can differ.

[Launch options and API](docs/local-session-manager.md) ·
[Architecture](docs/architecture.md) · [Roadmap](docs/product-roadmap.md) ·
[Cleanup and validation report](docs/milestones/ui-first-cleanup.md)
