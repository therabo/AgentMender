# Run and test the local session manager

Install Rust and Node 22 LTS. From the repository root run the following commands.

```sh
npm ci --prefix ui
npm run build --prefix ui
cargo build --locked --workspace
./target/debug/agentmender ui
```

`ui` opens the default browser. `ui --no-open` prints a private one-use launch
URL for manual opening. Restart to obtain a new ticket. Stop the foreground
server with Ctrl+C.

Codex prefers `--codex-root`, then `CODEX_HOME`, then the provider default.
With the default absent, discovery can select a single corroborated alternate.
Claude Code observes `~/.claude` and a distinct `CLAUDE_CONFIG_DIR` when
set. An absolute `--claude-root` selects the primary root explicitly.
Repeat `--claude-additional-root` to include more local archives.
`--claude-desktop-root` can point at a separate directory of Desktop metadata.
Supply absolute `--codex-root`, `--codex-executable` or `--claude-executable`
overrides at launch. `--protect-codex-session <UUID>` prevents AgentMender from directly selecting that Codex UUID for deletion. `--max-entries` bounds inventory acquisition and defaults to
10,000. Use refresh to pick up external changes. Date filters use inclusive UTC
calendar days over last activity.

## Browse and remove projects

Expand any number of projects in the Codex or Claude Code inventory.
Search by project name, workspace, namespace or associated main session
metadata. Each expanded project pages its own sessions.
Codex Recenti lists conversations without a verified project association
and supports normal session actions. Counts and sizes describe observed
records and known transcript bytes.

Use **Elimina sessione** in a Codex conversation to confirm permanent
deletion. AgentMender displays the result of the installed native command.
The existing project action applies that command in sequence to the main
conversations of eligible native projects. Project registrations and
workspaces stay separate.

Open **Diagnostica inventario Claude Code** to see the unfiltered local
counts, exclusions, scan limits, source availability and the matches
between Desktop Code registrations and local transcripts.

For Claude Code, open a session to read a bounded conversation preview and
inspect its namespace, file modification time, metadata coverage and storage
dependencies. Filter by metadata, activity date and transcript size.

## Claude Code storage and deletion

Open **Export e backup → Storage Claude Code** to inspect paths and
measurements. In a session's detail, choose **Standard** for recognized
session files or **Approfondita** to include scoped checkpoints. Review the
file list and total size, then select **Elimina**.

Session deletion is permanent. AgentMender creates no backup and offers no
restore. Existing backups on disk are left untouched. Close Claude before
confirming to reduce concurrent writes. Errors can leave some files or
history rows in place. AgentMender displays the result and refreshes the
inventory. Physical disk space reclaimed can differ from the logical size.

## Storage and export deletion

Choose **Storage → Export e backup**. The list supports search, size/date
sorting, pagination and selection of up to 32 groups. Expand a
row to see its format, manifest, timestamps, observed links and protection.
The first-level file or directory remains a filesystem group unless the
known exporter format identifies a publication.

**Anteprima selezione** refreshes the inventory within its entry and depth
limits. It shows file and directory counts, logical size, observed allocation
when available, and eligibility for each selected group. On macOS and Linux, **Elimina
selezionati** opens one confirmation listing the eligible groups, including any
explicitly selected independent backups or user outputs. Confirming removes
those groups permanently without creating backups. Shared references, active
leases and partial coverage are shown for review. AgentMender preserves export
roots and unlinks selected links without following their targets. Groups that
changed after preview are reported individually, as are filesystem errors.
An error or a scan limit can leave part of a selected directory in place. The
inventory refreshes and the selection clears after the attempt. Windows supports
inventory and preview. **Installazioni e archivi** shows the launch-time
discovery separately. Restart after a change to root or executable overrides.

## Local API

The server chooses an IPv4 loopback port. `POST /api/authorize` consumes the
ticket from the URL fragment and returns a per-launch bearer value. API
requests require that value in the Authorization header. Mutations also
require exact Origin. All requests require the exact local Host.

| Route | Role |
|---|---|
| `GET /api/storage/codex` | Read separate installation and storage observations |
| `GET /api/storage/exports` | Search, sort and page bounded export metadata |
| `POST /api/storage/exports/preview` | Resolve opaque group keys and issue a single-use receipt when some groups are eligible |
| `POST /api/storage/exports/refresh` | Invalidate the export inventory and pending receipt |
| `POST /api/storage/exports/operations/{key}/confirm` | Delete eligible groups bound to the receipt and return the result for each selected group |
| `GET /api/{provider}/projects` | Search, filter, sort and page provider project groups |
| `GET /api/{provider}/projects/{key}` | Read an observed group and its coverage and capability |
| `GET /api/{provider}/projects/{key}/sessions` | Page verified main conversations within a group |
| `POST /api/codex/projects/{key}/delete` | Confirm permanent deletion of sessions in an eligible Codex project |
| `GET /api/codex/recents` | Page verified main conversations without a verified project association |
| `GET /api/{provider}/sessions` | Search, filter, sort and page over verified main conversations |
| `GET /api/{provider}/sessions/{key}` | Details of a verified main conversation |
| `GET /api/{provider}/sessions/{key}/conversation` | Bounded selected Codex transcript page with optional opaque cursor |
| `POST /api/{provider}/refresh` | Reacquire the provider snapshot |
| `POST /api/claude/sessions/{key}/removal-preview` | List exact Claude targets for `standard` or `deep` on macOS |
| `POST /api/{provider}/sessions/{key}/delete` | Delete the selected session after explicit confirmation and return the result |

The JSON confirmation is `{"confirm":true}`. Claude also passes the opaque
`review` returned by its preview. Browser payloads carry no paths or commands.

## Test with disposable archives

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
npm test --prefix ui
python3 scripts/check_architecture.py
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender
```

On Darwin arm64, run the native fixture with a [supported Codex release](provider-compatibility.md).

```sh
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender --native-codex /absolute/path/to/codex
```

The suite uses disposable archives and checks both command results and the
refreshed inventory. The optional native Claude fixture exercises deletion
of a transcript generated by the installed engine.
