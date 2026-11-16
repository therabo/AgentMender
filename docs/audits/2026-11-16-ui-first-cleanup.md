# UI-first cleanup audit

The checkout started clean on `main` at
`0b8ec50859771480fc142d095d3c55143faee3ad`, matching the brief and the
`origin/main` tracking ref. Implementation uses `refactor/ui-first-cleanup`.

## Runtime reachability at the baseline

`ui/src/main.tsx`, `detail.tsx` and `api.ts` call the bearer exchange,
provider inventory, detail, refresh, transcript, preview, confirmation and
operation-status routes in `cli/src/ui`. The HTTP inventory calls the
deterministic `application::local_inventory` index, with observations from
`provider-codex::content::local` and `provider-claude::local`. Both provider
adapters also supply detection. CLI invokes the OS browser opener. Its build
script embeds Vite's HTML, JavaScript and CSS.

The Codex UI calls the local inventory, source and removal worker operations.
The transcript route then uses only `SessionSearchReadPort::transcript_page`
in diagnostic mode, with an empty exclusion set. The transitive reader
includes exact-row and locator resolution, the bounded SQLite snapshot,
regular-file descriptor traversal, revision checks, Zstandard resident
decoding, JSONL format recognition and visible message suppression. Its
temporary evidence directories, Seatbelt profile and kernel attestation
remain necessary. Search ranking, cohort management, AST filtering,
context windows and application search services are orthogonal to this path.

Deletion reaches the one-use UI receipt store, fresh Codex removal
observations and the confined native launch. The successful-command branch
can request the existing exact Desktop reconciliation marker reset. The
durable plan, approval, journal, projection, manual cleanup and recovery
engines do not enter this path. The old unconfined native entry point
belongs to the retired MCP caller.

| Area | Classification | Reachability decision |
|---|---|---|
| React UI, Vite build and browser authorization | Keep | Entry point and reviewed HTTP surface. |
| Deterministic inventory application | Keep | Used for list/detail selection, normalized filters, stable order and paging. |
| Domain observations and identity | Refactor | Two nested metadata structures and artifact graph identity reduce to typed provider/root/session identity and observed fields. Graph, integrity, approval and planning types have no required consumer. |
| Provider filesystem and process boundaries | Refactor | Retain bounded reads, bundle/version probes, evidence isolation and fixed native launch. Remove observer probes, old provider ports, unconstrained native launch, coordinator and lifecycle workers. |
| Codex content and metadata | Refactor | Extract only snapshot, exact row, transcript and preflight observations. Preserve bounded locator resolution and decoder state. Remove FTS, semantic metadata worker, cohort and search DTOs. |
| Claude project reader and detection | Refactor | Retain the bounded JSONL reader and supported installation evidence. The synthetic scanner, Desktop sidecar graph, and native compatibility research serve retired commands. |
| Codex exact native removal and existing marker reset | Investigated → retain | The browser invokes both. Keep the exact digest, root, archive, UUID, spawn-edge and revision checks with the same native command, timeout and success-only marker. |
| Manual cleanup, transactions and Desktop candidates | Investigated → remove | Reachable only through retired CLI/MCP entry points and fixtures. Neither the UI review nor the native command consumes their authority or state. |
| MCP, Luna, independent index and persistence | Remove | Their only reachability is through removed conversational or external-maintenance commands. The browser holds its own receipts in memory. |
| Scripts, evidence and documentation | Refactor | Preserve live UI tests, migrate relevant native fixture helpers and decoder/sandbox tests, replace historical CI guards. Historical source and artifacts remain available in Git. |

## Reference review

Reviewed [Session Steward at ac80fc3](https://github.com/mallikcheripally/session-steward/tree/ac80fc31f7977a00c80f5be22841a6049bb0bcf1)
under its [MIT license](https://github.com/mallikcheripally/session-steward/blob/ac80fc31f7977a00c80f5be22841a6049bb0bcf1/LICENSE).
The provider stores own recognition and compatibility, and the HTTP layer
sends provider-specific listing results. The Codex store separates canonical
thread metadata from transcript observations. The Claude store recognizes
project JSONL and distinguishes missing and unreadable state. The UI keeps
filtering, paging, inspection and cleanup review visible as separate actions.
Session Steward recomputes the cleanup fingerprint and protections before
execution and uses a distinct backup and verification lifecycle. AgentMender
keeps the existing reviewed single-session native boundary and draws only on
the separation of discovery, review and dispatch. The refactor uses original
AgentMender code.
