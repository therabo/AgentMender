# Dependency decisions

Registry metadata and maintainer documentation checked on 2026-10-28. Cargo.lock
records the resolved versions; reviewed stable release lines are constrained in
Cargo.toml. No dependencies from the research were copied without checking need.

| Dependency | Selected release | Need |
|---|---|---|
| serde | 1.0.229 | Typed JSON domain serialization |
| serde_json | 1.0.154 | Provider wire fixtures and CLI structured output |
| thiserror | 2.0.21 | Structured errors without string-only dispatch |
| clap | 4.6.7 | Small typed read-only CLI surface |
| libc (Unix) | 0.2.191 stable | O_NOFOLLOW/O_NONBLOCK constants through safe std OpenOptions; no unsafe calls |
| tempfile | 3.27.0 | Isolated fixtures and private owned evidence directories |

Maintenance was verified through crates.io cargo info metadata, official docs and
upstream repositories. This is a selection review, not a dependency security audit.
The libc prerelease 1.0 line was deliberately excluded. No async runtime, network SDK, MCP SDK or recovery system is introduced.

Closeout additions checked through crates.io/maintainer documentation:

| Dependency | Release | Need |
|---|---|---|
| rusqlite | 0.41.0 | Safe SQLite Backup API into RAM; exact-schema scrub and disposable persistence |
| libsqlite3-sys (resolved) | 0.39.1 / SQLite 3.54.1 | Bundled maintained SQLite; read-only WAL handling, consistent version and no extension-loading feature |
| sha2 | 0.11.1 | SHA-256 source and schema fingerprints |

rusqlite default features are disabled; only libsqlite3-sys, bundled, backup and
limits are enabled. tempfile is now a runtime dependency only for owned disposable
homes. These dependencies contain native/cryptographic implementation internals;
the application invokes their safe interfaces. MSRV rises from 1.85 to 1.89 for
rusqlite (sha2 requires 1.88).

No extra crate was added for macOS sandbox_check. One small C-ABI module queries
kernel write policy before source SQLite access; it matches WebKit's maintained
Darwin declaration. Workspace unsafe remains forbid; only provider-api uses deny
plus this module's explicit allowance. Architecture checks reject other unsafe
code. This OS-specific SPI fails closed on unavailable/invalid results and is not
a claim of full M10 platform hardening. See ADR-0004.

M2 adds `proptest 1.12.0` only as a domain development dependency. Registry metadata
and the maintainer's documentation were checked on 2026-10-28: this stable release
requires Rust 1.88, below the existing 1.89 MSRV. The mature project describes its
status as passive maintenance. Generated permutations, duplicate observations,
evidence combinations, and graph round trips need shrinking and reproducible
counterexamples; a custom random-test harness would duplicate this functionality.
Only `std` is enabled; subprocess forking, timeout infrastructure, bit sets and
derive macros are unnecessary. No production graph/database dependency is added.

M4 reuses the reviewed workspace `sha2 0.11.1` in domain for deterministic plan
and state identity. No new package/transitive dependency or MSRV change is needed.
No planning, rule, persistence, recovery or approval framework is introduced.

## M5 read-only MCP

Verified on 2026-10-28 against crates.io metadata, official MCP documentation and
downloaded SDK source. The stable MCP revision is 2026-07-28. The maintained official
Rust SDK is rmcp 3.8.0, published 2026-10-26; an older GitHub search result was not
used as the release authority. See ADR-0010 for protocol/privacy decisions.

| Dependency | Selected release / MSRV | Purpose |
|---|---|---|
| rmcp | exactly 3.8.0 / 1.88 | Official protocol negotiation, tool server and bounded async stdio framing |
| tokio | 1.54.1 / 1.85 | Current-thread runtime, admitted blocking query, synchronization, deadlines and I/O |
| schemars | 1.2.3 / 1.74 | Strict MCP-only typed input/output JSON Schemas |
| hmac | 0.13.0 / 1.85 | Standard HMAC-SHA256 for process-scoped identity/cursor privacy |
| getrandom | 0.4.3 / 1.85 | OS-generated process-lifetime identity key; already present transitively before M5 |

rmcp disables default features and enables only server and transport-io. Its
resolved features are schemars, server, transport-async-rw, transport-io and uuid.
No client, macros, auth, request-state or HTTP feature is selected. No reqwest,
axum, hyper, OAuth package or rmcp-macros is pulled into the runtime. The SDK
requires async primitives, futures, tracing, chrono, UUID and schema utilities;
these explain the transitive increase. No tracing subscriber logs source frames.
transport-io includes Tokio networking support internally; AgentMender opens no
network listener or remote MCP transport.

Schema dependencies remain outside domain/application/provider ports. The async
runtime is confined to the inbound composition boundary. Existing SHA-256 supplies
the HMAC hash; keys and references are ephemeral, not approval grants or durable
security state. All 130 resolved packages declare MSRV at or below existing 1.89
(target-specific packages included); Cargo.lock pins their actual versions.

Primary sources: [official SDK](https://github.com/modelcontextprotocol/rust-sdk),
[rmcp release metadata](https://crates.io/crates/rmcp/3.8.0),
[Tokio blocking semantics](https://docs.rs/tokio/1.54.1/tokio/task/fn.spawn_blocking.html),
[schemars](https://docs.rs/schemars/1.2.3),
[RustCrypto HMAC](https://docs.rs/hmac/0.13.0),
[OS entropy](https://docs.rs/getrandom/0.4.3).
