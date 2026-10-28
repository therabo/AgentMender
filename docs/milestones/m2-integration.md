# M2 formal integration evidence

Verified on 2026-10-28 before M3 implementation.

| Item | Evidence |
|---|---|
| Original main | `462b35b9a8b889eb095aa67a20ebd918c0d2de28` |
| M2 branch | `milestone/m2-artifact-graph` |
| M2 exact local and remote HEAD | `b96c31710758942bfb85e3527c17cebe2ef9a40a` |
| Exact-head Actions run | [40723249873](https://github.com/therabo/AgentMender/actions/runs/40723249873) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/40723249873/job/122939559814) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/40723249873/job/122939559327) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/40723249873/job/122939559629) |
| Integration | `git merge --ff-only milestone/m2-artifact-graph`, then normal push |
| Integrated main / M3 base | `b96c31710758942bfb85e3527c17cebe2ef9a40a` |

The GitHub connector returned repository-not-found for this new repository.
Authenticated Safari displayed the exact run, commit URL and three successful
platform job conclusions. SSH `git ls-remote` independently verified branch SHAs.
No previous report was used as a substitute for live remote verification.

Before integration and again on pushed main, all checks passed: cargo fmt,
workspace clippy with warnings denied, locked workspace tests, locked workspace
build, architecture fitness and git diff whitespace checks. Tests: 117 passed,
0 failed, 2 explicit live probes ignored. The ten generated M2 properties each
used 256 cases and fixed seed `0x4d325f6772617068` (Unix also exercises native paths).
Working tree was clean at both gates. Fast-forward preserved every M2 commit.

Independent Sol/high safety review passed: pure graph/builders, protected Global
and Unknown, no Partial exclusivity/orphanhood, all conflicts/raw provenance kept,
no deletion closure, integrity repair or lifecycle port, and sanitized fixtures.
Existing disposable evidence writes remained confined to their reviewed boundary.

Production sandboxed detection reverified Codex `0.165.0-alpha.1.2`, Claude Desktop
`4.29.0`, and bundled Claude Code `2.1.222`. Current installation versions do not
establish historical transcript producers. No compatibility profile was expanded.

Only after this gate, `milestone/m3-integrity-forensics` was created from exact main
and pushed with tracking before implementation.
