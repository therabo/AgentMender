# Dependency decisions

## Product-first audit dependency decision

ADR-0022 keeps the eight crates and all locked package versions. Retire the unused
provider-api `spawn_codex_read_only` entry point, superseded by owned evidence
capture, and its duplicate development declaration of the runtime `tempfile`
dependency. Tests still access the runtime dependency; Cargo.lock is unchanged.
No runtime, generic lifecycle package, crate split or feature flag is added.
Provider-native selection remains provider-specific and uses the existing
protected orchestration. Deep Desktop modules remain private and unadmitted.

The remainder records dependency decisions at their stated historical dates;
release-version claims there are not fresh package recommendations.

## Deferred M8.2.1 assessment boundary

The pure product assessment uses existing domain/serde dependencies and introduces
no executor, new runtime, generic mutation library or parallel persistence store.
Provider-specific projection schemas and GUI coordination remain in adapters and
need separate exact-profile review before any new destructive dependency is added.
See [ADR-0018](adr/0018-complete-session-removal.md).

## M8.2 scoped cleanup dependencies

M8.2 reuses the locked `rusqlite`, `sha2` and safe `rustix 1.1.6` packages.
The Codex adapter enables rustix `std`/`fs` for descriptor-relative no-follow open,
flock, stat, unlink and directory sync. No new registry package/version, unsafe
exception or MSRV change is introduced. The `application` dependency in Codex is
**development-only**, to test the genuine approval/transaction/application chain
on disposable provider state; runtime dependency direction remains unchanged.
The SQLite read worker reuses the existing kernel-denial sandbox and bounded RAM
backup/scrub boundary. Third-party cleaner implementations were researched but
not copied or introduced as dependencies. Recovery remains bounded typed metadata,
without transcript or credential backups. See [ADR-0017](adr/0017-scoped-cleanup-residual-preservation-provider-local-mutation.md).

M9 adds only the already locked `proptest` workspace package as a Claude provider
development dependency for zero-authority guards. No new registry package/version,
production dependency, runtime, MSRV or persistence schema is introduced. The opt-in
native evidence reproducer uses Python's standard library and the existing macOS
sandbox facility; production remains Rust-only and Claude read-only.

M8 reuses the locked workspace packages. Application and provider-codex add only
the existing persistence crate for ADR-0013's narrow durable handoff. Application
tests reuse existing tempfile/proptest. No new registry package, RPC/HTTP stack,
MSRV change or provider-api/MCP dependency is introduced. Native writable dispatch
is confined to the exact reviewed private Codex lifecycle launcher.

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

## M6 owned persistence

Reviewed on 2026-10-29 through registry metadata, downloaded source and official
SQLite/Rust/Windows documentation. Reuse existing rusqlite 0.41.0 (bundled SQLite
3.54.1), serde_json, thiserror and getrandom; domain reuses SHA-256. No second
database, hash, UUID or migration framework is added. Safe std file locks are
available within the existing Rust 1.89 MSRV. Unused direct serde/sha2/proptest
dependencies were removed from persistence; properties remain domain dev tests.

Unix promotes the already locked `rustix 1.1.6` (MSRV 1.65) with std/process only
to obtain the current effective UID safely. No new unsafe allowance is needed.
Existing libc supplies no-follow/nonblocking open constants.

Windows adds target-only `windows-permissions 0.2.4` for safe ACL/SID wrappers and
`winapi-util 0.1.11` for handle-based file identity/link counts unavailable through
stable std metadata. Their stable published releases and source were inspected;
they do not declare MSRV. Source uses older stable language features; current
Windows CI is required. The small transitive additions are winapi 0.3.9, its two
GNU import libraries, and bitflags 1.3.2; winapi-util reuses windows-sys. These are
low-level wrappers with limited release activity, not a claim of an active security
audit or guaranteed future maintenance. Direct reviewed wrapper calls avoid an
observed unrelated set_multiple flag bug. No generic command helper or unsafe
Rust was added. Windows runtime ACL behavior is verified by platform CI, and
ReFS/directory power-loss limits remain explicit M10 work.

Primary references: [SQLite sync modes](https://www.sqlite.org/pragma.html#pragma_synchronous),
[Rust file locks](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock),
[rustix](https://github.com/bytecodealliance/rustix),
[Windows permission wrappers](https://github.com/danieldulaney/windows-permissions-rs),
[winapi-util](https://github.com/BurntSushi/winapi-util),
[Windows security descriptors](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format).

## M7 approval authentication

Reuse already locked hmac 0.13.0 and sha2 0.11.1 in persistence, with getrandom
0.4.3 for a dedicated approval key and independent grant identities/nonces. No new
package, crypto implementation, runtime or MSRV increase is required. Rechecked
the exact cached package metadata and official RustCrypto documentation on
2026-10-29: hmac MSRV 1.85 and sha2 MSRV 1.88 fit workspace Rust 1.89. Both are
stable RustCrypto releases already used by this workspace. This is a scoped
dependency selection review, not a complete vulnerability audit. The dedicated
approval key never reuses MCP's ephemeral privacy key.

Use full-length HMAC-SHA256 tags and Mac::verify_slice for verification, following
the [maintainer's API](https://docs.rs/hmac/0.13.0/hmac/). Plain SHA-256 remains
content identity. Existing SQLite BEGIN IMMEDIATE and the store lock serialize
approval writes; [SQLite transaction semantics](https://www.sqlite.org/lang_transaction.html)
do not make external files/provider operations part of a database transaction.
