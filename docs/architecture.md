# Architecture

```text
React / TypeScript / Vite
        |
        | exact-loopback HTTP, browser session, opaque keys
        v
agentmender CLI -> application -> domain
        |
        +-> provider-codex  -> provider-api -> confined SQLite / rollout reader
        |                   -> Codex native deletion or scoped Claude file removal
        |
        +-> provider-claude -> provider-api -> bounded project JSONL reads
```

| Crate | Responsibility |
|---|---|
| `domain` | Provider/root/session and project identity, coverage-neutral metadata and display safety. |
| `application` | Verified main-conversation filtering, project grouping, normalized metadata queries and stable paging. |
| `provider-api` | Bounded filesystem reads, fixed version probes, isolated worker and native command launch, kernel write-denial attestation. |
| `provider-codex` | Fixed SQLite and rollout observations, bounded compressed transcript parsing, platform discovery, bounded export observations, native release and storage admission, and the guarded success marker. |
| `provider-claude` | Bounded project transcript inventory, installation detection and offline session cleanup on admitted macOS layouts. |
| `cli` | Launch configuration, embedded Vite assets, browser authorization, provider routing, one-use confirmation and operation results. |

## Read path

Discovery returns provider-scoped observations. The application filters for
verified main conversations before constructing its bounded metadata index,
which supplies provider totals and opaque key lookups. Search uses Unicode normalization and full case
folding. State and UTC date filters run before sorting and paging. Unknown sort
values follow known ones. An opaque browser key incorporates provider, root,
namespace and native identity. Project keys bind provider, root and the
registered Codex ID or observed Claude namespace. Native SQLite and recognized
Desktop assignments resolve Codex membership inside the provider observation.
Recenti collects main conversations without a verified project association.
Session keys stay rooted in the original provider observation.

The project index groups the full bounded observation before paging. Project
and session filters use the same normalized metadata query. Group totals
count visible main conversations. Known transcript bytes can include
attributable hidden descendants, and coverage flags qualify observed totals.
The browser pages sessions independently inside each expanded project and
in Recenti. Coverage flags qualify the three observed counts.

Codex opens SQLite only inside a protected macOS worker. It copies a bounded
snapshot into memory and runs fixed projections. Inbound edge checks query
the SQLite snapshot directly, including parents beyond the inventory limit.
Codex also checks source, fork and attachment fields against available rollout
identity metadata. The bundled SQLite build
omits the retired FTS5 extension. The worker resolves the
selected canonical row to its rollout, checks scope and physical identity, then
reads only a bounded transcript page. Compressed rollouts are incrementally
decoded in the worker. Replay state is bound to the source and finite. Message
suppression, unknown records and external paginated history are reflected in
partial coverage. Browser cursors hold neither a path nor provider internals.

Claude observes bounded project JSONL across configured roots, with
revision-keyed metadata and revalidated conversation windows. Root, namespace,
entrypoint and embedded IDs govern observation. Desktop registrations supply
metadata when they corroborate an unambiguous transcript identity. Nested
agents, sidechains and recognized forks stay outside the main index. Storage
diagnostics report candidate dependencies.
[ADR 0046](adr/claude-session-cleanup.md).

## Removal path

The authenticated browser confirms permanent deletion by opaque session key.
Codex dispatches its native command for the selected UUID. The CLI uses the
same archive as the session inventory. After native success, the adapter
rewrites the two JSONL indexes, prunes the Desktop catalog and state, and
requests sidebar reconciliation. It reports auxiliary errors as partial
success. The project action repeats the workflow serially for observed main
conversations and stops on a native command failure.

On macOS, Claude builds a complete plan for the verified transcript, its
namespace-scoped directory and associated artifacts, including checkpoints.
The server retains the reviewed targets until confirmation. Claude removes
entries and directories without following links, then rewrites applicable
history rows under Claude's renewed directory lock. Each provider's result
invalidates the inventory. The export controls have separate authorization. [ADR 0046](adr/claude-session-cleanup.md).

## Storage discovery and exports

`provider-codex/discovery` gathers OS-specific installation and root evidence.
It selects one launch root for the existing session manager. The browser can
inspect other roots independently. Discovery never grants a native writer
capability. Storage records differentiate presence, format evidence, ownership
and supported operations.

`provider-codex/exports.rs` enumerates metadata within entry and depth limits
and projects only recognized manifest fields. The CLI caches the
inventory for pagination and produces a new observation before a selection
preview. Export keys bind the observed root, group path and metadata revision.
The router applies the existing loopback browser and Origin boundaries.
Export confirmation consumes a single-use receipt in its independent namespace.
The provider re-observes each eligible group and unlinks the observed entries
relative to the verified export-root descriptor. A completed attempt invalidates
the export inventory. [ADR 0045](adr/0045-codex-storage-capabilities.md)


Claude's storage view builds bounded measurements under declared provider
roots. It reports directory groups and ownership evidence and projects the
existing catalog for transcript rankings. The per-session storage read
revalidates transcript and reconciled record revisions before returning an
storage diagnostic. The Codex export
inventory retains its own observation and authorization boundary.
[ADR 0046](adr/claude-session-cleanup.md).
