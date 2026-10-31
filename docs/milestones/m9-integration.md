# M9 blocker-evidence integration receipt

Verified 2026-10-31, before M8.1 research or implementation.

| Item | Direct evidence |
|---|---|
| Original main | `83f0692cdda33bc3be34146ab8ceefab5dadf1f3` |
| M9 branch | `milestone/m9-claude-lifecycle` |
| Original reviewed M9 HEAD | `53d2a485bdc2a20111ad52393a5a9f1a75ac3518` |
| Original run | [41090154046](https://github.com/therabo/AgentMender/actions/runs/41090154046), three platforms PASS |
| Documentation closeout / final M9 HEAD | `9daa237ac6eadeb3d74e71b19ba31bcd6eef97ef` |
| Exact-final-head run | [41150461236](https://github.com/therabo/AgentMender/actions/runs/41150461236) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/41150461236/job/124385511633) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/41150461236/job/124385511269) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/41150461236/job/124385511578) |
| Integration | `git merge --ff-only milestone/m9-claude-lifecycle`, ordinary push |
| Verified pushed main / M8.1 base | `9daa237ac6eadeb3d74e71b19ba31bcd6eef97ef` |
| New tracked branch | `milestone/m8-1-codex-full-cleanup` |

M9 was integrated as blocker evidence, NOT as completed lifecycle capability.
Capability status: BLOCKED BY PROVIDER. Investigation: CLOSED FOR CURRENT PROVIDER
CONTRACT. Claude lifecycle operations remain Unsupported, with zero production
mutation operations/profiles/dispatcher. The closeout decision supersedes historical
OPEN/no-merge wording without changing provider semantics. See [closeout](m9-closeout.md).

All six local gates passed on the original and final heads: fmt, locked workspace
Clippy with warnings denied, tests/build, architecture fitness and whitespace.
Final tests: 337 primary passes, seven additional nested persistence child-process
passes, zero failures, three explicit live probes ignored. Generated tests retain
256 cases per property and fixed seeds, including Claude `0x4d395f636c617564`.

The GitHub connector returned 404; no gh was installed. Authenticated Safari showed
exact commit links and successful individual Linux/macOS/Windows jobs. SSH ls-remote
independently matched branch heads before integration and pushed main afterward.
The worktree was clean at integration. History was preserved, with no force push.
Only after remote main verification was M8.1 created at the exact base and pushed
with tracking. M10 was not started.

An additional explicit old M8 native regression was attempted separately after the
ordinary gates. It failed closed at the exact binary digest check, before provider
startup or creating disposable provider state: the currently installed executable
hash is `58a91f6ea19bea477ae3a2ec48d619f0039afe75d3476260747e11d201fa7cea`,
not the historical `d84a54b031085f01e3ad915ea7c72ebea4dcf69e90f0d1e0e4a637e8280991d2`.
This is a rejected compatibility attempt, not a native acceptance pass. No installed
file/profile was changed and no real user session was mutated. The historical
profile and all deterministic M8 regressions remain unchanged.
