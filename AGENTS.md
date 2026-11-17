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
- Allow only provider-verified main conversations into search, sorting,
  pagination, counts and opaque key lookup. Keep descendant evidence inside
  provider observations and inspect the full Codex graph for removal.
- Admit native removal through a confined version probe, a write-denied
  command probe and an exercised release and storage contract. Pin the actual
  executable and root identities. Review the complete archived subtree,
  including edge-only descendants, and check configured protections, writer
  ownership, rollout identity and external references.
- Rebuild the bound scope immediately before the single native request and
  constrain rollout writes to reviewed paths. Keep one-use receipts and the
  single in-flight guard. Verify rows, edges and rollouts through the read-only
  worker. Ambiguous outcomes stay uncertain and are never retried automatically.
- Keep Claude Code read-only. Decline bulk, project-wide and unrelated artifact
  cleanup. Keep the guarded Desktop marker and distinguish verified session
  evidence from other provider artifacts or physical storage reclaim.
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
