# AgentMender

AgentMender is a local browser session manager for **Codex** and **Claude Code**.
Expand registered projects to browse their main conversations. Codex shows
independent conversations in Recenti. Search projects, workspaces, titles and
session IDs. Supported Codex releases can remove a verified conversation tree, including
unarchived sessions without a writer, or the trees assigned to a native project.

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
| Codex | Storage discovery and export inventory for macOS, Linux and Windows. Registered projects, Recenti, partial main-conversation inventory and bounded transcript pages on macOS. Complete session deletion on macOS through the installed native command, followed by local index and Desktop cleanup. |
| Claude Code | Namespace groups and bounded inventory of verified local project JSONL, including Desktop-origin Code. Complete session deletion on macOS, including the session directory and checkpoints. |

Open **Storage → Export e backup** to measure and select up to 32 export groups.
On macOS and Linux, review the selected files and directories and confirm once
to delete eligible groups permanently. The result identifies preserved groups
and errors, and the inventory refreshes automatically. Export deletion creates
no backup. **Installazioni e archivi** shows the discovered roots, including the
root selected for the Session Manager.

Both providers offer one permanent deletion flow with a single confirmation.
Claude previews the scoped files, directories and history rows before deletion.
AgentMender refreshes the inventory after each attempt. Session deletion creates no backup.

[Launch options and API](docs/local-session-manager.md) ·
[Architecture](docs/architecture.md) · [Roadmap](docs/product-roadmap.md) ·
[Cleanup and validation report](docs/milestones/ui-first-cleanup.md)
