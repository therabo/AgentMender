# Local session manager acceptance

Validation date 2026-11-16 on macOS arm64. The starting checkout was clean at
`e0a7cc7` on `main`. The [ADR](../adr/0042-local-session-manager.md)
records the browser and provider boundaries.

| Check | Result |
|---|---|
| Rust formatting, locked workspace Clippy with warnings denied, tests and build | Passed. 592 tests passed and eight existing tests were ignored. |
| Source architecture, independent approval and pinned Codex evidence guards | Passed. The existing complete Desktop removal research verdict remains open. |
| Existing MCP autonomous and hybrid fixture drivers, Luna host and semantic assessment tests | Passed. The drivers used disposable state and simulated model responses. |
| Frontend locked install, TypeScript, Vite production build and Vitest | Passed. Three frontend tests covered selection, missing values and pagination. |
| Loopback API fixture | Passed with 3,000 Codex sessions, stable pages, normalized Unicode search, UTC and state filters, missing metadata, invalid archive layout and provider absence. Claude Code CLI and Desktop-origin project JSONL were read-only. |
| Browser authorization | Rejected missing or forged credentials, wrong Host, wrong or missing Origin, replay of the launch ticket, arbitrary path parameters and forged approval fields. |
| Exact Codex native fixture | A stale preview, protected UUID, unarchived session and observed parent with children were refused. The confirmed deletion removed the disposable target rollout and canonical row and preserved all retained fixture rollouts byte for byte. Replay was rejected. A native nonzero exit stayed uncertain and the API refused further previews for that UUID in the same launch. |
| Bounded Codex conversation preview | The fixture verified a continuation, bound it to the selected session and confirmed suppression of a credential marker in the selected source. |
| Chrome desktop and mobile | Exercised provider switching, paging, search, transcript preview, the disabled Claude action and the typed confirmation view. Axe reported zero violations on inventory, confirmation and mobile states. Browser console reported zero errors. |
| Read-only local installations | Both archive inventories returned recognized sessions and partial coverage. Only sanitized provider and count summaries were inspected. |

The native fixture used the installed `0.169.0-alpha.11.1` executable pinned
by SHA-256 in the ADR and a new isolated archive. Real sessions were left
untouched. Exact-final remote CI on macOS, Linux and Windows remains unrun
for these local changes. Provider support varies by platform.

See [reproduction commands](../local-session-manager.md#disposable-acceptance).
