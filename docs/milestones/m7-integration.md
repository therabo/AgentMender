# M7 formal integration evidence

Verified on 2026-10-29 before M8 implementation.

| Item | Evidence |
|---|---|
| Previous main | `f2268d2cdcff961a91e079af79314571fb3752d7` |
| M7 branch | `milestone/m7-independent-approval` |
| Exact local and remote M7 HEAD | `5cf94c01d1d14acf6034d7bb7f5521547ee470dd` |
| Exact-commit Actions run | [40912256361](https://github.com/therabo/AgentMender/actions/runs/40912256361) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/40912256361/job/123583049324) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/40912256361/job/123583049534) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/40912256361/job/123583048936) |
| Integration | `git merge --ff-only milestone/m7-independent-approval`, ordinary push |
| Verified local/remote main and exact M8 base | `5cf94c01d1d14acf6034d7bb7f5521547ee470dd` |

The main agent read all 37 research pages and the complete milestone specification,
all repository documentation and ADRs. Main and independent source reviewers
reconstructed domain, adapters, isolation, application, persistence, approval,
CLI, MCP, tests, fixtures and Git history before repository changes.

All six checks passed on M7 and again on pushed main: formatting, locked workspace
Clippy with warnings denied, locked tests/build, architecture fitness and whitespace
checks. Primary test suites: 287 passed, zero failed, two explicit live probes
ignored. Eight additional persistence child-process invocations passed. Working
tree was clean at both gates. No assertion or safety rule was weakened.

Independent Sol/high reviews found no actionable security defect. Grants bind the
exact transaction, immutable plan/state/profile/effects and recovery. Expired,
revoked or consumed grants cannot dispatch; blocked/unsupported plans cannot gain
authority. Fresh validation and verified recovery precede signed atomic consumption
and durable intent. Missing outcomes stay protected. Digests and IDs are not
authority. MCP retained its eight read-only tools and no provider mutation existed.

The GitHub connector returned 404 and no GitHub CLI was installed. Authenticated
Safari displayed the exact run, full commit link, branch and successful individual
Linux, macOS and Windows job conclusions. SSH `git ls-remote` independently verified
both branch heads and pushed main. Historical reports did not substitute for live
evidence.

Only after this gate, `milestone/m8-codex-lifecycle` was created from exact main
and pushed with tracking before implementation. History was preserved.
