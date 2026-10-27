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
| tempfile (test only) | 3.27.0 | Isolated disposable fixture trees |

Maintenance was verified through crates.io cargo info metadata, official docs and
upstream repositories. This is a selection review, not a dependency security audit.
The libc prerelease 1.0 line was deliberately excluded. No async runtime, SQL,
network SDK, MCP SDK, crypto or recovery dependencies are required for M0/M1.
