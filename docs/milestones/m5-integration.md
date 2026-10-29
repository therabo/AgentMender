# M5 formal integration evidence

Verified on 2026-10-29 before M6 implementation.

| Item | Evidence |
|---|---|
| Previous main | `139291761975c1dca50492adcb5972cbfe7233f5` |
| M5 branch | `milestone/m5-readonly-mcp` |
| Exact local and remote M5 HEAD | `c426a7fe4e12c24afc2f383087912e7f54111b39` |
| Exact-commit Actions run | [40854074866](https://github.com/therabo/AgentMender/actions/runs/40854074866) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/40854074866/job/123380539082) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/40854074866/job/123380538503) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/40854074866/job/123380539089) |
| Integration | `git merge --ff-only milestone/m5-readonly-mcp`, normal push |
| Verified local/remote main and exact M6 base | `c426a7fe4e12c24afc2f383087912e7f54111b39` |

GitHub connector returned 404 and no gh binary was installed. Authenticated Safari
displayed the exact run, full commit link, branch and all three successful matrix
jobs. SSH ls-remote independently confirmed branch identities and pushed main.
No historical report substituted for current remote evidence.

All six checks passed before integration and again after pushing main: fmt,
locked workspace Clippy with warnings denied, locked tests/build, architecture
fitness and git diff --check. Tests: 204 passed, zero failed, two explicit live
probes ignored. Worktree clean at both gates. Three Sol/high reviews found no
blocking M5 safety defect: MCP remains read-only, no execution or approval API,
no provider lifecycle mutation, transaction executor or recovery system; Unknown
is protected and Partial cannot prove destructive exclusivity.

Only after this gate, milestone/m6-durable-transactions was created from exact
main and pushed with tracking before broad implementation. No history rewrite.
