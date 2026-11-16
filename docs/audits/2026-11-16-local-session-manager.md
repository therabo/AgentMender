# Local session manager architecture audit

Baseline `e0a7cc7f7e67d86885c0c21beb4bedfab9d1a844`. The local checkout
started clean on `main`, aligned with the local `origin/main` tracking ref.

| Crate | Reuse and responsibility in the new surface |
|---|---|
| `domain` | Retains provider/root/native identity, evidence and safety types. Adds a display-oriented metadata observation alongside the existing metadata type. |
| `provider-api` | Retains the bounded filesystem and process helpers, worker isolation and existing approval-free observation ports. The fixed Codex native command is shared by MCP and UI launch profiles. |
| `application` | Keeps read and lifecycle use cases. Adds normalized query, sorting, paging and child lookup over provider observations. |
| `provider-codex` | Extends the confined content worker's existing SQLite snapshot, canonical row projection, rollout locator and bounded reader. The review observes the exact unshared archived rollout, state database identity and outgoing spawn table edges. |
| `provider-claude` | Reuses the bounded filesystem inventory and project transcript shape. The local view reads explicit matching JSONL metadata prefixes. Prior synthetic and Desktop interpretation remain available to the original CLI. |
| `persistence` | Retains durable transaction, approval and owned-state recovery. Its FTS5 cache remains opt-in for historical retrieval. Browser previews carry authority for the native command only and expire with the process. |
| `mcp-server` | Retains stdio, strict projections, signed selections, host identity rules and elicitation. MCP has no control route to the browser session. |
| `cli` | Composes provider roots and adapters for both interfaces. Hosts the loopback transport, browser session and single-use native command review. |

## Retained retrieval

`cli/session_find` owns the Planner/Evaluator hosts and autonomous semantic job
orchestration. Its MCP registration, indexed retrieval, Python launch/assessment
scripts and test drivers are referenced by explicit `mcp --session-search`
and the corresponding CI checks. The normal UI path instantiates none of them.
The deterministic metadata view also avoids the search query AST, FTS cache
and cohort scan. The source worker, title/secret filtering, canonical identity
checks, provenance and bounded reader have read-only consumers on both surfaces.

## Reference review

Reviewed Session Steward at
[`ac80fc31f7977a00c80f5be22841a6049bb0bcf1`](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1),
licensed under [MIT](https://github.com/mallikcheripally/session-steward/blob/ac80fc31f7977a00c80f5be22841a6049bb0bcf1/LICENSE).

- `ui/src/main.jsx` and `ui/src/styles.css` use provider-specific views, visible
  coverage, independent filters and sorting, grouped rows and an inspector.
  AgentMender adopts those interaction patterns with its own UI code.
- `lib/providers/codex/store.mjs` and listing/scale tests distinguish canonical
  metadata and unverified file observations. The tests cover global size
  order, unknown values, date filters and large transcript handling.
  AgentMender stays on its existing bounded SQLite and rollout readers.
- `lib/providers/claude-code/store.mjs` and the current-format tests show
  project JSONL with explicit `sessionId`, `entrypoint`, `cwd`, timestamps and
  `custom-title` records. The local installed archive also had project JSONL
  using `claude-desktop` with matching explicit identities. The reader accepts
  these Code records and leaves conflicting workspace observations unknown.
  It projects only fields tied to the same observed identity and reports
  partial coverage.
- `lib/session-cleanup.mjs`, `test/server.test.mjs`, provider cleanup tests
  and `test/version-support.test.mjs` distinguish review, preflight, compatibility
  and execution. AgentMender keeps its fixed native command and exact fixture
  profile. Session Steward source and SQL are absent from the implementation.

## Compatibility

Codex's archive flag gives the UI its Active/Archived display state. It is a
retention state and does not prove whether a process is currently writing.
The UI refuses unarchived and protected identities. The approved native
macOS binary supplies writer locking and reference checks.
The pinned operation contract follows the provider implementation for
[delete coordination](https://github.com/openai/codex/blob/rust-v0.169.0-alpha.11.1/codex-rs/app-server/src/request_processors/thread_delete.rs)
and [the local thread store](https://github.com/openai/codex/blob/rust-v0.169.0-alpha.11.1/codex-rs/thread-store/src/local/delete_thread.rs).
Alias handling, absence of other residues and Desktop disappearance outside the
disposable cases remain qualified by the native provider's scope.
