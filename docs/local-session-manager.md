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

Codex defaults to `CODEX_HOME` or `~/.codex`. Claude Code defaults to
`CLAUDE_CONFIG_DIR` or `~/.claude`. Supply absolute `--codex-root`,
`--claude-root`, `--codex-executable` or `--claude-executable` overrides at
launch. `--protect-codex-session <UUID>` protects a session in the configured
Codex root. `--max-entries` bounds inventory acquisition and defaults to
10,000. Use refresh to pick up external changes. Date filters use inclusive UTC
calendar days over last activity.

## Local API

The server chooses an IPv4 loopback port. `POST /api/authorize` consumes the
ticket from the URL fragment and returns a per-launch bearer value. API
requests require that value in the Authorization header. Mutations also
require exact Origin. All requests require the exact local Host.

| Route | Role |
|---|---|
| `GET /api/{provider}/sessions` | Page over metadata with search, state, date, sort and paging fields |
| `GET /api/{provider}/sessions/{key}` | Session details and observed relationships |
| `GET /api/{provider}/sessions/{key}/conversation` | Bounded selected Codex transcript page with optional opaque cursor |
| `POST /api/{provider}/refresh` | Reacquire the provider snapshot |
| `POST /api/codex/sessions/{key}/removal-preview` | Preflight and issue a two-minute one-use receipt |
| `POST /api/operations/{receipt}/confirm` | Consume the receipt with the explicit confirmation phrase |
| `GET /api/operations/{receipt}` | Read running, changed, command success or uncertain outcome |

Session keys bind the provider, root and exact identity. Transcript cursors,
receipts and outcome tracking are process-local.

## Test with disposable archives

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
npm test --prefix ui
python3 scripts/check_architecture.py
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender
```

On Darwin arm64 with the [reviewed executable](provider-compatibility.md), also
run the exact native fixture.

```sh
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender --native-codex /absolute/path/to/reviewed/codex
```

The optional test rejects a different executable digest. It initializes new
private archives and uses the same confined command boundary as the browser.
