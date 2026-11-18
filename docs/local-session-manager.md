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
overrides at launch. `--protect-codex-session <UUID>` protects a UUID in the configured Codex root, including when it
is a descendant of the selected conversation. `--max-entries` bounds inventory acquisition and defaults to
10,000. Use refresh to pick up external changes. Date filters use inclusive UTC
calendar days over last activity.

## Browse and remove projects

Expand any number of projects in the Codex or Claude Code inventory.
Search by project name, workspace, namespace or associated main session
metadata. Each expanded project pages its own sessions.
Codex Recenti lists conversations without a verified project association
and supports normal session actions. Counts and sizes describe observed
records and known transcript bytes.

On a supported Codex installation, open **Rimuovi sessioni del progetto**
from an eligible project header. Confirm with `RIMUOVI SESSIONI DEL PROGETTO`.
The operation processes the reviewed trees in sequence. The native registration,
workspace and software repository are outside the scope. Desktop-backed projects
support individual session reviews.

For Claude Code, open a session to read a bounded conversation preview and
inspect its namespace, file modification time, metadata coverage and storage
dependencies. Filter by metadata, activity date and transcript size. Standard
and deep cleanup are disabled until the writer and ownership contracts can
be verified. The Storage and dependencies section explains the blocker.

On partial completion, inspect the verified, undispatched and uncertain
subtree IDs. A restart discards receipts and live operation results. Refresh
and request a fresh review to inspect the remaining provider records.
AgentMender never resumes removal automatically. SQLite storage reuse,
snapshots, backups and provider artifacts can affect physical bytes reclaimed.

## Storage and export previews

Choose **Storage → Export e backup**. The list supports search, size/date
sorting, pagination and up to 32 groups in a diagnostic selection. Expand a
row to see its format, manifest, timestamps, observed links and protection.
The first-level file or directory remains a filesystem group unless the
known exporter format identifies a publication.

Preview re-enumerates within the inventory budget. A changed group invalidates
its key. The preview shows total file count, logical size, observed allocation
when available and the protection of each selected group. With no verified
export writer contract the preview issues no receipt. Export inspection never
joins a session or project removal plan. **Installazioni e archivi** shows
the launch-time discovery separately. Restart after a change to root or
executable overrides.

## Local API

The server chooses an IPv4 loopback port. `POST /api/authorize` consumes the
ticket from the URL fragment and returns a per-launch bearer value. API
requests require that value in the Authorization header. Mutations also
require exact Origin. All requests require the exact local Host.

| Route | Role |
|---|---|
| `GET /api/storage/codex` | Read separate installation and storage observations |
| `GET /api/storage/exports` | Search, sort and page bounded export metadata |
| `POST /api/storage/exports/preview` | Recompute a diagnostic selection from opaque keys |
| `POST /api/storage/exports/refresh` | Invalidate the export measurement |
| `POST /api/storage/exports/operations/{key}/confirm` | Reject while no export writer contract is admitted |
| `GET /api/{provider}/projects` | Search, filter, sort and page provider project groups |
| `GET /api/{provider}/projects/{key}` | Read an observed group and its coverage and capability |
| `GET /api/{provider}/projects/{key}/sessions` | Page verified main conversations within a group |
| `POST /api/{provider}/projects/{key}/removal-preview` | Review native Codex project membership and issue a session-removal receipt |
| `GET /api/codex/recents` | Page verified main conversations without a verified project association |
| `GET /api/{provider}/sessions` | Search, filter, sort and page over verified main conversations |
| `GET /api/{provider}/sessions/{key}` | Details of a verified main conversation |
| `GET /api/{provider}/sessions/{key}/conversation` | Bounded selected Codex transcript page with optional opaque cursor |
| `POST /api/{provider}/refresh` | Reacquire the provider snapshot |
| `POST /api/codex/sessions/{key}/removal-preview` | Review one main conversation and its spawned subtree, then issue a two-minute one-use receipt |
| `POST /api/operations/{receipt}/confirm` | Consume the receipt with the explicit confirmation phrase |
| `GET /api/operations/{receipt}` | Read operation state and verified project progress |

Session keys bind the provider, root and exact identity. Transcript cursors,
receipts and outcome tracking are process-local. Hidden descendants have no
browser entry or selectable key. The irreversible deletion review reports the
main title and UUID, the verified descendant count and the total number of
sessions affected. The project review supplies verified counts, the project
key, the list of reviewed trees and explicit exclusions. A browser cannot
supply root IDs or a replacement project scope. Session removal uses
`RIMUOVI`. The project session review uses `RIMUOVI SESSIONI DEL PROGETTO`.
Ineligible targets show the reason and receive no receipt.

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

The suite initializes disposable archives, checks the release and command
contract, and exercises the same confined command boundary as the browser.
The fixture covers unarchived and archived trees, open and closed edges,
plain and compressed variants, internal and external history references,
and a byte-for-byte unchanged independent export. Install `zstd` to exercise
the compressed base. Run it once per binary when validating a release.
