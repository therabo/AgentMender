# M8 formal integration evidence

Verified on 2026-10-30 before M9 investigation or implementation.

| Item | Evidence |
|---|---|
| Previous main | `5cf94c01d1d14acf6034d7bb7f5521547ee470dd` |
| M8 branch | `milestone/m8-codex-lifecycle` |
| Exact local and remote M8 HEAD | `83f0692cdda33bc3be34146ab8ceefab5dadf1f3` |
| Exact-commit Actions run | [40974505377](https://github.com/therabo/AgentMender/actions/runs/40974505377) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/40974505377/job/123799090516) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/40974505377/job/123799090318) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/40974505377/job/123799090610) |
| Integration | `git merge --ff-only milestone/m8-codex-lifecycle`, ordinary push |
| Verified local/remote main and exact M9 base | `83f0692cdda33bc3be34146ab8ceefab5dadf1f3` |

The main agent read all 37 research pages and the complete milestone specification,
all repository documents and ADRs. Main and three independent Sol/high reviewers
reconstructed all source, tests, fixtures, compatibility evidence and Git history
before repository changes. Review ownership covered Codex/provider isolation,
domain/persistence/authority, and application/CLI/MCP/Claude boundaries.

All six checks passed on M8 and again on pushed main: formatting, locked workspace
Clippy with warnings denied, locked tests/build, architecture fitness and whitespace
checks. Primary test suites including compile-fail documentation tests: 332 passed,
zero failed, three explicit live probes ignored. Seven additional persistence
child-process test invocations passed. Working tree was clean at both gates.

The GitHub connector returned 404. Authenticated Safari displayed the exact run,
full commit URL and all three successful platform job conclusions. SSH initially
had no agent identities; the existing macOS keychain, selected per command with
`UseKeychain=yes`, allowed independent `git ls-remote` verification and normal
push. No credentials, access settings or Git history were changed. Historical
reports did not substitute for live remote evidence.

No blocking review finding remained. Codex unarchive stays conditional on the exact
validated profile and all existing readiness/approval/recovery/revalidation gates.
Archive/delete remain Unsupported. No raw provider DB/filesystem lifecycle fallback,
effect before durable intent, approval bypass, automatic ambiguous retry or
response-only success exists. Claude and MCP remained read-only through this gate.
One conservative verification limitation was retained: presentation-only wording
changes can fail retained-fact comparison; this cannot create false success.

Only after the repeated main checks, `milestone/m9-claude-lifecycle` was created
from this exact base and pushed with tracking before broad implementation.
