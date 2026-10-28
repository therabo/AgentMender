# M3 formal integration evidence

Verified on 2026-10-28 before M4 implementation.

| Item | Evidence |
|---|---|
| Original main | `b96c31710758942bfb85e3527c17cebe2ef9a40a` |
| M3 branch | `milestone/m3-integrity-forensics` |
| Exact local and remote M3 HEAD | `fa291282bea9257cf2fdb57a3827aac005a678e7` |
| Exact commit Actions run | [40734142052](https://github.com/therabo/AgentMender/actions/runs/40734142052) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/40734142052/job/122978017719) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/40734142052/job/122978018356) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/40734142052/job/122978018730) |
| Integration | `git merge --ff-only milestone/m3-integrity-forensics`, normal push |
| Verified local/remote main and exact M4 base | `fa291282bea9257cf2fdb57a3827aac005a678e7` |

The GitHub connector returned 404 for the repository/run. Authenticated Safari
displayed the exact commit URL, run, and three individual successful matrix jobs.
SSH `git ls-remote` independently confirmed both branches before integration and
main after push. Historical reports were not substituted for remote evidence.

All six gates passed before integration and again on pushed main: cargo fmt,
locked workspace clippy with warnings denied, locked workspace tests/build,
architecture fitness, and git whitespace checks. Tests: 147 passed, 0 failed,
2 explicit live probes ignored. The working tree was clean at both gates.

Independent Sol/high reviews covered Codex, Claude, provider ports, application,
CLI, tests, fixtures, isolation, and architecture fitness. Main reviewed the pure
domain graph/integrity evaluator and generated properties. No blocking safety
defect remained. Partial cannot prove orphanhood/missing targets, unknown remains
protected, conflicts remain visible, concurrent change is not corruption, and
orphan/residual findings do not grant deletion permission. No mutation or repair
port exists. Existing disposable evidence infrastructure is unchanged.

Only after this gate, `milestone/m4-immutable-planning` was created from exact main
and pushed with tracking before implementation. Fast-forward preserved all M3
commits and provenance.
