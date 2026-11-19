# AgentMender

AgentMender is a local browser app for browsing and cleaning up Codex and Claude Code sessions. It brings conversations, projects, and storage into one place so you can find old work and review what a cleanup would remove.

## Features

- Discover local installations and archives. Search and filter main conversations by project, activity, state, and size, then open their transcripts.
- Review a Codex session and its verified descendants before deleting the tree. Where project membership is complete, delete a registered project's sessions in bulk while keeping the project and workspace. After native deletion, AgentMender cleans up scoped history, indexes, and recognized Codex Desktop catalog entries.
- Review and delete a Claude Code session, including verified session files, checkpoints, and matching shared history entries.
- Use Storage Manager to inspect Claude Code disk usage and review individual Codex export and backup groups before removing them.

## Platform support

| Capability | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Codex installation, archive, and export inventory | Yes | Yes | Yes |
| Codex projects, sessions, transcripts, and descendant review | Yes | Unavailable | Unavailable |
| Codex session and project-session deletion | Requires a supported native Codex executable | Unavailable | Unavailable |
| Codex export deletion | Yes | Yes, with `openat2` required for directories | Unavailable |
| Claude Code projects, sessions, transcripts, and storage | Yes | Yes | Yes |
| Claude Code session deletion | Yes | Unavailable | Unavailable |

Inventory and transcript coverage depend on the archive format, available metadata, and filesystem access. The app reports partial coverage where it detects gaps.

## Build and run locally

Install Rust 1.89 or later with Cargo, a C compiler for bundled SQLite on macOS, and Node.js 22 with npm. From the repository root, build the React frontend first so Cargo can embed it in the executable.

```sh
npm ci --prefix ui
npm run build --prefix ui
cargo build --locked --workspace
./target/debug/agentmender ui
```

On Windows, launch `target\debug\agentmender.exe ui`. The app serves the UI on IPv4 loopback and opens your default browser. Use `ui --no-open` to print the private launch URL or `--help` to see the options for selecting archives and executables.

## Permanent deletion

AgentMender requires an explicit confirmation before deleting sessions or exports. **Deletion is permanent. No backups are created.** Review the affected items before confirming. A cleanup can partially succeed if files change or become inaccessible. The result reports failures and the inventory refreshes after the attempt.
