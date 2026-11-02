# M8.2 integration and M8.2.1 base

Verified in the development cycle begun 2026-11-01; recorded 2026-11-02.

| Item | Exact observation |
|---|---|
| Previous local/remote main | `6f798bcebdfd2f763e31d62ef2695c52396ca709` |
| M8.2 branch | `milestone/m8-2-scoped-session-cleanup` |
| Final local/remote M8.2 SHA | `a284767157ace8769deee85b195b5b70efe374d8` |
| Historical verdict | PASS: safe primary scoped cleanup and logical reclaim |
| Actions | [41320886468](https://github.com/therabo/AgentMender/actions/runs/41320886468), full commit link verified |
| Linux | [124890638937](https://github.com/therabo/AgentMender/actions/runs/41320886468/job/124890638937), completed successfully, 5m 25s |
| macOS | [124890638772](https://github.com/therabo/AgentMender/actions/runs/41320886468/job/124890638772), completed successfully, 3m 42s |
| Windows | [124890638987](https://github.com/therabo/AgentMender/actions/runs/41320886468/job/124890638987), completed successfully, 10m 26s |
| Integration | `git merge --ff-only`, ordinary push; no squash/force |
| Verified local/remote main after push | `a284767157ace8769deee85b195b5b70efe374d8` |
| M8.2.1 branch | `milestone/m8-2-1-complete-session-removal`, created at that exact SHA and pushed with tracking before implementation |

Actions was freshly observed in authenticated Safari GitHub accessibility state:
full commit URL, Success summary and all three completed-job links. Connector/API
returned 404 and gh was unavailable; prior reports were not substituted for live
verification. SSH ls-remote independently confirmed heads before/after integration.

All baseline gates passed on the unchanged M8.2 commit: cargo fmt --check,
locked workspace Clippy with warnings denied, locked workspace tests/build,
scripts/check_architecture.py, scripts/check_codex_lifecycle_evidence.py and
git diff --check. The workspace was clean.

The separate opt-in test
`scoped_cleanup::acceptance::exact_binary_and_sandboxed_worker_on_disposable_fixture`
passed (1 pass, 23.54s). It read the exact 0.166.0 binary and used the real protected
worker against manually seeded disposable state through the real planning,
transaction, recovery, independent test approval, DurableIntent and production
adapter pipeline. No native provider process or real user session was started
or mutated. The fixture removes 64 MiB logical rollout bytes; physical reclaim
remains unknown. This is primary-cleanup evidence, not GUI/restart evidence.

M8.2 remains PASS. [ADR-0018](../adr/0018-complete-session-removal.md) strengthens
the product claim without rewriting history. M8.2.1 is OPEN. M8.3, M10 and the
natural-language layer have not started.
