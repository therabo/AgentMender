# Local session manager

From the repository root, use Rust 1.89 or later and Node 22 LTS.

```sh
npm ci --prefix ui
npm run build --prefix ui
cargo build --locked --workspace
./target/debug/agentmender ui
```

The command opens the default browser and prints a one-time launch URL.
`ui --no-open` prints it for manual opening. Keep it private. Restart the
server to create a fresh browser session if that ticket is used and the
browser tab is closed. Stop the listener with Ctrl+C.

Codex defaults to `CODEX_HOME` or `~/.codex`. Claude Code defaults to
`CLAUDE_CONFIG_DIR` or `~/.claude`. `--codex-root`, `--claude-root` and
`--codex-executable` override those choices at launch. For example:

```sh
./target/debug/agentmender --codex-root /absolute/path/to/codex-home ui
```

Use `--protect-codex-session <UUID>` for an exact UUID that must be kept.
`--max-entries` controls the bounded inventory acquisition, with a default
of 10,000. Refresh the inventory to pick up external changes. The date filter
uses inclusive calendar days in UTC and tests last activity. Unknown dates and
sizes sort after known values in either direction.

## Local API

The server assigns its port. `POST /api/authorize` accepts the launch
`ticket` and issues the bearer header for that process. Every later API call
requires `Authorization: Bearer …`. POST also requires exact `Origin`.
The root, key-to-identity map and provider commands are held in Rust.

| Route | Role |
|---|---|
| `GET /api/providers` | Availability, coverage and capabilities for both providers |
| `GET /api/{provider}/sessions` | Snapshot page with `search`, `state`, `since`, `until`, `sort`, `ascending`, `offset` and `limit` |
| `GET /api/{provider}/sessions/{key}` | Session metadata, observed relationships and compatibility |
| `GET /api/{provider}/sessions/{key}/conversation` | Requested Codex transcript page with optional opaque `cursor` |
| `POST /api/{provider}/refresh` | Rebuild inventory using the configured source |
| `POST /api/codex/sessions/{key}/removal-preview` | Validate eligibility and issue a two-minute one-use receipt |
| `POST /api/operations/{receipt}/confirm` | Check the `confirmation` phrase, consume the receipt and run asynchronously after fresh validation |
| `GET /api/operations/{receipt}` | Running, changed, command succeeded or uncertain outcome |

The browser exposes one selected target. The inventory shows the measured
transcript size separately from deletion outcomes. Claude is read-only. Codex
conversation reads require macOS. Codex removal additionally requires the
[exact reviewed executable and archive shape](adr/0042-local-session-manager.md).
See the [roadmap](product-roadmap.md) for later artifact attribution.

## Disposable acceptance

```sh
npm test --prefix ui
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender --native-codex /absolute/path/to/reviewed/codex
```

The native test accepts only the reviewed binary digest, creates fresh test
archives and verifies the target rollout and canonical row against retained
fixture sessions. The optional branch requires Darwin arm64.
