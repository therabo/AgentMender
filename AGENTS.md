# AgentMender engineering constraints

The local browser session manager is the product. Use
[ADR 0043](docs/adr/0043-ui-first-runtime.md), the [architecture](docs/architecture.md)
and [provider compatibility](docs/provider-compatibility.md) as the current
contracts.

- Keep the browser listener on IPv4 loopback. Require exact Host, same-origin
  mutations, a per-launch browser credential and a server-held one-use receipt.
  Scope session IDs and transcript cursors to exact provider/root identities.
  URLs and request bodies carry neither file paths nor commands.
- Keep Codex SQLite inside the attested read-only worker. Retain fixed queries,
  descriptor-relative no-follow reads, unshared rollout identity, bounded
  snapshots and decoder budgets, and exact locator, UUID and revision checks.
  Preserve secret suppression and honest partial coverage. Unsafe code is
  confined to the existing macOS kernel attestation boundary.
- Admit Codex removal only under the reviewed executable and root profile.
  Check archive state, configured protections, exact rollout and spawn edges,
  including edges without canonical child rows. Recheck immediately before
  native dispatch. Only one confirmed operation can be in flight per launch.
  Ambiguous native outcomes remain uncertain and are never retried automatically.
- Keep Claude Code read-only. Decline bulk, project-wide and unrelated artifact
  cleanup. The existing success-only Desktop marker has a narrow guarded
  contract. Native command success leaves residuals and physical reclaim
  unverified.
- Use temporary, isolated provider fixtures. Leave real provider session data,
  credentials and installed provider configuration untouched during development.

Build frontend assets before exercising the embedded UI. Required local checks:

```sh
npm ci --prefix ui
npm test --prefix ui
npm run build --prefix ui
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
python3 scripts/test_local_ui.py --agentmender target/debug/agentmender
git diff --check
```

Run the optional exact native fixture when its supported binary is available.
Use coherent commits and preserve Git history and archival refs.
