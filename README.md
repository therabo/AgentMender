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
| Codex | Storage discovery and export inventory for macOS, Linux and Windows. Registered projects, Recenti, partial main-conversation inventory and bounded transcript pages on macOS. Permanent deletion through the installed native command. |
| Claude Code | Namespace groups and bounded inventory of verified local project JSONL, including Desktop-origin Code. Permanent Standard and Approfondita deletion on macOS. |

Open **Storage → Export e backup** to measure and select up to 32 export groups.
On macOS and Linux, review the selected files and directories and confirm once
to delete eligible groups permanently. The result identifies preserved groups
and errors, and the inventory refreshes automatically. Export deletion creates
no backup. **Installazioni e archivi** shows the discovered roots, including the
root selected for the Session Manager.

Session deletion requires explicit confirmation and creates no backup.
Codex uses its native command. Claude lists the recognized local files
before deletion. Both providers refresh the inventory after the attempt.

[Launch options and API](docs/local-session-manager.md) ·
[Architecture](docs/architecture.md) · [Roadmap](docs/product-roadmap.md) ·
[Cleanup and validation report](docs/milestones/ui-first-cleanup.md)
