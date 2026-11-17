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
| `domain` | Provider/root/session identity, coverage-neutral metadata and display safety. |
| `application` | Verified main-conversation filtering, normalized metadata queries and stable paging. |
| `provider-api` | Bounded filesystem reads, fixed version probes, isolated worker and native command launch, kernel write-denial attestation. |
| `provider-codex` | Fixed SQLite and rollout observations, bounded compressed transcript parsing, native release and storage admission and the existing guarded success marker. |
| `provider-claude` | Bounded read-only project transcript inventory and installation detection. |
| `cli` | Launch configuration, embedded Vite assets, browser authorization, provider routing, one-use confirmation and operation results. |

## Read path

Discovery returns provider-scoped observations. The application filters for
verified main conversations before constructing its bounded metadata index,
which supplies provider totals and opaque key lookups. Search uses Unicode normalization and full case
folding. State and UTC date filters run before sorting and paging. Unknown sort
values follow known ones. An opaque browser key incorporates provider, root,
namespace and native identity.

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

Claude uses its existing bounded project JSONL metadata profile. Roots,
entrypoints and matching embedded IDs govern observation. Nested agent and
sidechain records stay outside the main-conversation index. Conflicting workspace
values remain unknown. There is no Claude mutation operation.

## Removal path

The browser reviews one verified main UUID. The Codex worker owns the full
relationship observation. It indexes incoming and outgoing spawn edges and
walks the outgoing edges iteratively into a unique bounded set, retaining
descendant IDs with missing canonical rows. Duplicate or conflicting edges,
cycles, unknown coverage and unsafe or external references block review.
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
