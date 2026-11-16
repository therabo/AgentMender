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
| `application` | Deterministic normalized metadata queries, stable ordering, paging and observed child lookup. |
| `provider-api` | Bounded filesystem reads, fixed version probes, isolated worker and native command launch, kernel write-denial attestation. |
| `provider-codex` | Fixed SQLite and rollout observations, bounded compressed transcript parsing, exact native admission and the existing guarded success marker. |
| `provider-claude` | Bounded read-only project transcript inventory and installation detection. |
| `cli` | Launch configuration, embedded Vite assets, browser authorization, provider routing, one-use confirmation and operation results. |

## Read path

Discovery returns provider-scoped observations. The application holds a bounded
in-memory metadata index. Search uses Unicode normalization and full case
folding. State and UTC date filters run before sorting and paging. Unknown sort
values follow known ones. An opaque browser key incorporates provider, root,
namespace and native identity.

Codex opens SQLite only inside a protected macOS worker. It copies a bounded
snapshot into memory and runs fixed projections. The bundled SQLite build
omits the retired FTS5 extension. The worker resolves the
selected canonical row to its rollout, checks scope and physical identity, then
reads only a bounded transcript page. Compressed rollouts are incrementally
decoded in the worker. Replay state is bound to the source and finite. Message
suppression, unknown records and external paginated history are reflected in
partial coverage. Browser cursors hold neither a path nor provider internals.

Claude uses its existing bounded project JSONL metadata profile. Roots,
entrypoints and matching embedded IDs govern observation. Conflicting workspace
values remain unknown. There is no Claude mutation operation.

## Removal path

The browser reviews one Codex session. The provider checks its pinned executable,
canonical root and SQLite row, an unshared archived rollout with a matching
header, and known spawn edges. CLI stores an expiring receipt. Confirmation
consumes it under lock, then obtains fresh evidence immediately before the
fixed native command. The status endpoint tracks this user-triggered command
through completion, including disconnection of the initiating request.

The provider confines native writes to the configured root and blocks network
access. Native coordination handles writer and reference checks. A successful
native return may request the prior guarded Desktop marker reset. Pre-dispatch
refusals, native success and uncertain outcomes have distinct statuses.
See the [provider compatibility contract](provider-compatibility.md).
