# AgentMender engineering constraints

The local browser session manager is the product. Use
[ADR 0043](docs/adr/0043-ui-first-runtime.md), the [architecture](docs/architecture.md)
and [provider compatibility](docs/provider-compatibility.md) as the current
contracts.

- Keep the browser listener on IPv4 loopback. Require exact Host, same-origin
  mutations, a per-launch browser credential and explicit confirmation for deletion.
  Scope session IDs and transcript cursors to exact provider/root identities.
  URLs and request bodies carry neither file paths nor commands.
- Keep Codex SQLite inside the attested read-only worker. Retain fixed queries,
  descriptor-relative no-follow reads, unshared rollout identity, bounded
  snapshots and decoder budgets, and exact locator, UUID and revision checks.
  Preserve secret suppression and honest partial coverage. Unsafe code is
  confined to the existing macOS kernel attestation boundary.
- Allow only provider-verified main conversations into search, sorting,
  pagination, counts and opaque key lookup.
- Dispatch Codex session deletion to the native executable using the selected
  archive and UUID. Resolve bulk Codex membership from a complete native
  project observation. Keep project registrations and workspaces outside it.
- Keep Claude deletion in the provider under ADR 0046. Scope removals to
  recognized session files and prune only empty directories. Use the provider's
  directory lock for shared history. Session deletion is permanent and creates
  no backup.
- Keep export discovery, inventory and authorization separate from session
  removal. Bind each receipt to explicitly reviewed groups. Preserve export
  roots, links, shared references, active leases and incomplete groups. Identify
  independent backups and user outputs in the confirmation. Perform writes
  relative to the verified export root without following links.
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

Run the disposable native fixture with any available supported release.
Use coherent commits and preserve Git history and archival refs.
