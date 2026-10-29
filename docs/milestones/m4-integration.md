# M4 formal integration evidence

Verified on 2026-10-28 before M5 implementation.

| Item | Evidence |
|---|---|
| Original main | `fa291282bea9257cf2fdb57a3827aac005a678e7` |
| M4 branch | `milestone/m4-immutable-planning` |
| Expected original local/remote HEAD | `146a4b34f3edfd51d5a0619a7a0137c6ad82e166` |
| Original exact-head run | [40765005617](https://github.com/therabo/AgentMender/actions/runs/40765005617), all three platforms passed |
| Corrected final local/remote M4 HEAD | `139291761975c1dca50492adcb5972cbfe7233f5` |
| Final exact-head run | [40768042036](https://github.com/therabo/AgentMender/actions/runs/40768042036) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/40768042036/job/123097428279) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/40768042036/job/123097427905) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/40768042036/job/123097427558) |
| Integration | `git merge --ff-only milestone/m4-immutable-planning`, normal push |
| Verified local/remote main and exact M5 base | `139291761975c1dca50492adcb5972cbfe7233f5` |

The original expected branch, SHA, clean worktree and CI were verified directly.
Independent review then found one deterministic test could inspect the real
`/Applications/Claude.app` bundle on macOS despite its temporary HOME. A small
test-only follow-up supplies explicit temporary Codex and Claude roots, disabling
implicit Desktop discovery. No production behavior or existing assertion changed.
All gates were repeated on that replacement head before integration.

The GitHub connector returned 404. Authenticated Safari displayed the exact commit
URL, run and successful individual matrix job conclusions. SSH `git ls-remote`
independently verified branch identities and pushed main. Historical reports were
not substituted for actual remote state.

All six checks passed before integration and again on pushed main: formatting,
locked workspace clippy with warnings denied, locked tests and build, architecture
fitness and whitespace checks. Local totals: 179 passed, 0 failed, 2 explicit live
probes ignored. Worktree clean at both gates.

Three independent Sol/high reviews covered domain and properties, provider ports
and both adapters including isolation/fixtures, and application/CLI/architecture.
The test-isolation finding was resolved and independently rechecked. Immutable
plans, deterministic evidence-bound identity, protected unknown/partial state and
unsupported provider intentions remain intact. Ready is not approved. No executor,
approval broker, recovery/journal or provider lifecycle invocation exists.

Only after this gate, `milestone/m5-readonly-mcp` was created from exact main and
pushed with tracking before broad implementation. Fast-forward preserved history.
