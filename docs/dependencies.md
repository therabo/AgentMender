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
