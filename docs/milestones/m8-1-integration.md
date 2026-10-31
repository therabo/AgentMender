# M8.1 blocker-evidence integration receipt

Verified 2026-10-31 before M8.2 external research or implementation.

| Item | Direct evidence |
|---|---|
| Previous main | `9daa237ac6eadeb3d74e71b19ba31bcd6eef97ef` |
| M8.1 branch | `milestone/m8-1-codex-full-cleanup` |
| Exact local/remote M8.1 HEAD | `6f798bcebdfd2f763e31d62ef2695c52396ca709` |
| Exact-head Actions run | [41189341451](https://github.com/therabo/AgentMender/actions/runs/41189341451) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/41189341451/job/124502694460) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/41189341451/job/124502694372) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/41189341451/job/124502694547) |
| Integration | `git merge --ff-only milestone/m8-1-codex-full-cleanup`, normal push |
| Verified local/remote main and exact M8.2 base | `6f798bcebdfd2f763e31d62ef2695c52396ca709` |
| New tracked branch | `milestone/m8-2-scoped-session-cleanup` |

The main agent reconstructed all 37 research pages, the complete internal milestone
specification, README/AGENTS, all 16 ADRs, current documentation and integration
receipts, Git history and the 45-run Actions history. Three Sol/high read-only
reviews covered all eight crates, tests, regressions, scripts, fixtures, profiles,
retained provider evidence and 13 internal historical reports. Repeated generated
schema definitions were structurally parsed and deduplicated; existing public
signature artifacts were read, without claiming a fresh cryptographic verification.
All 41 M8.1 evidence-manifest member hashes matched.

The complete M8.1 diff is 58 files, 32,317 insertions and one deletion. Both changed
Rust source files have identical production tokens after removing test modules.
Changes are evidence/schemas, documentation/ADR-0016, tests/refusal guards, isolated
research reproducer, architecture canaries and deterministic evidence validation
in CI. No broad production SQL/filesystem cleanup capability was introduced.

All six required local gates passed at the exact integrated commit: fmt, locked
all-target Clippy with warnings denied, locked workspace tests/build, architecture
fitness and whitespace checks. The additional committed evidence check passed.
This run reported 338 primary passes including compile-fail doctests, zero failures,
three explicit opt-in probes ignored, and eight additional nested child-test passes.
These are the current observed totals, separate from older report counts.

The connector returned 404 for this repository and gh was unavailable. Authenticated
Safari independently displayed the exact commit/run and each successful platform
job; SSH `ls-remote` with existing Keychain-backed authentication verified branch
heads before integration and pushed main afterward. Worktree was clean. No force
push, squash, credential change or history rewrite occurred. Only then was M8.2
created at exact main and pushed with tracking, before broad implementation.

**M8.1 historical provider-native complete-cleanup capability remains BLOCKED;
M8.2 introduces a different scoped-cleanup success model.** M8.1 answered NO to
complete native effect/deletion/retention proof. Its evidence is unchanged. The new
product question is precise removal of one session's primary and strongly
attributable state while preserving shared, ambiguous, unknown and unrelated state.
M9 likewise remains BLOCKED BY PROVIDER CONTRACT, with zero Claude mutations.

No real user session was mutated. M10, M8.3 and natural-language work were not started.
