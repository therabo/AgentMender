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
| Codex | Storage discovery and export inventory for macOS, Linux and Windows. Registered projects, Recenti, partial main-conversation inventory and bounded transcript pages on macOS. Native cascade removal under the [verified release, command and storage contract](docs/provider-compatibility.md). |
| Claude Code | Namespace groups and bounded inventory of verified local project JSONL, including Desktop-origin Code. Offline cleanup and recovery for admitted macOS sessions. |

Open **Storage → Export e backup** to measure export groups, inspect why they
are protected and review a multi-selection. An export location or manifest
does not authorize mutation. Installed exporters have no verified removal
contract in this release. **Installazioni e archivi** shows the discovered
roots separately, including the root selected for the Session Manager.

A project review lists the main conversations, descendants and exclusions.
Codex removes each reviewed session tree in sequence. The project
registration and its source files stay outside the removal scope. AgentMender verifies canonical rows, rollouts and spawn edges after
each command and stops if the next scope changes or a result is uncertain.
Those checks measure archive state. Desktop visibility and physical disk space
reclaimed can differ.

[Launch options and API](docs/local-session-manager.md) ·
[Architecture](docs/architecture.md) · [Roadmap](docs/product-roadmap.md) ·
[Cleanup and validation report](docs/milestones/ui-first-cleanup.md)
