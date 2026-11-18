# Architecture

```text
React / TypeScript / Vite
        |
        | exact-loopback HTTP, browser session, opaque keys
        v
agentmender CLI -> application -> domain
        |
        +-> provider-codex  -> provider-api -> confined SQLite / rollout reader
        |                   -> reviewed Codex native removal + marker request
        |
        +-> provider-claude -> provider-api -> bounded project JSONL reads
```

| Crate | Responsibility |
|---|---|
| `domain` | Provider/root/session and project identity, coverage-neutral metadata and display safety. |
| `application` | Verified main-conversation filtering, project grouping, normalized metadata queries and stable paging. |
| `provider-api` | Bounded filesystem reads, fixed version probes, isolated worker and native command launch, kernel write-denial attestation. |
| `provider-codex` | Fixed SQLite and rollout observations, bounded compressed transcript parsing, platform discovery, bounded export observations, native release and storage admission, and the guarded success marker. |
| `provider-claude` | Bounded read-only project transcript inventory and installation detection. |
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
diagnostics report candidate dependencies and a removal blocker without issuing
receipts. [ADR 0046](adr/claude-session-cleanup.md).

## Removal path

The browser can review one verified persisted main UUID or the session trees explicitly
assigned to a verified native Codex project.
The Codex worker owns the full
relationship observation. It indexes incoming and outgoing spawn edges and
walks the outgoing edges iteratively into a unique bounded set, retaining
descendant IDs with missing canonical rows. Duplicate or conflicting edges,
cycles, unknown coverage and unsafe or external references block review. The
worker resolves each physical rollout ID, validates its placement in either
rollout tree and checks the history chains and native fallback paths.
Provider-specific classification and graph logic stay in the adapter. The CLI
keeps the full review server-side and returns counts with an expiring receipt.

Confirmation consumes the receipt under the existing operation lock. The
provider rebuilds the subtree, compares membership and evidence revisions,
checks executable and root identity, then dispatches the fixed native command
once. The provider scopes writes to the configured root, limits rollout writes
to reviewed paths and denies network access. Codex enforces writer locks and
checks references during native deletion.

A fresh read-only observation verifies expected row, edge and rollout absence
and compares unrelated canonical rows and rollout file revisions. The status
endpoint records verified deletion, pre-dispatch changes, residual evidence
or uncertainty. Verified deletion can request the existing guarded Desktop
marker reset. The [compatibility contract](provider-compatibility.md) lists the
admitted releases and observation limits.


Project review reopens the complete bounded archive independently of the UI
entry budget, builds the existing graph and rollout index once per scan, then
validates native assignments, trees and references before issuing a receipt.
Each native step starts from a freshly rebuilt plan. The verified previous
tree's unrelated evidence is the fingerprint expected for the next remaining
plan. A final scan verifies the remaining state and project registration.
The same native dispatch serves individual and bulk session removal.

The operation status exposes each verified, undispatched or uncertain tree.
An observed failure stops dispatch. Outcomes and capabilities expire with the
server process. The project inventory is reobserved on restart and the user
can authorize a newly verified scope explicitly. See [ADR 0044](adr/0044-provider-projects.md).

## Storage discovery and exports

`provider-codex/discovery` gathers OS-specific installation and root evidence.
It selects one launch root for the existing session manager. The browser can
inspect other roots independently. Discovery never grants a native writer
capability. Storage records differentiate presence, format evidence, ownership
and supported operations.

`provider-codex/exports.rs` enumerates metadata within entry, depth and time
budgets and projects only recognized manifest fields. The CLI caches the
inventory for pagination and produces a new observation before a selection
preview. Export keys bind the observed root, group path and metadata revision.
The router applies the existing loopback browser and Origin boundaries.
Export confirmation has an independent namespace and is unavailable until
there is an exercised exporter write contract. [ADR 0045](adr/0045-codex-storage-capabilities.md)


Claude's storage view builds bounded measurements under declared provider
roots. It reports directory groups and ownership evidence and projects the
existing catalog for transcript rankings. The per-session storage read
revalidates transcript and reconciled record revisions before returning an
informational preflight. The route issues no receipt. The Codex export
inventory retains its own observation and authorization boundary.
[ADR 0046](adr/claude-session-cleanup.md).
