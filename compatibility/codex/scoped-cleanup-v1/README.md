# Codex scoped-cleanup-v1 profile evidence

## Current interpretation

This is the historical M8.2 primary-cleanup profile, PASS and integrated at
`a284767157ace8769deee85b195b5b70efe374d8`. Pins, production effects, v2 wire
identity and replay are unchanged. Under
[ADR-0018](../../../docs/adr/0018-complete-session-removal.md), its successful
primary receipt maps to product ProjectionIncomplete, with maximum-safe cleanup
unknown. This is not an admitted graphical complete-removal/restart profile.

This is a separately reviewed provider-local cleanup profile. It does not change
M8.1's blocked native delete verdict or the historical native unarchive pins.
The implementation and release remain subject to the M8.2 review and gates in
[ADR-0017](../../../docs/adr/0017-scoped-cleanup-residual-preservation-provider-local-mutation.md).

The pinned executable is official Codex 0.166.0 for Darwin arm64, 268,972,064
bytes, SHA-256 `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`.
It is read and hashed, never started, by this cleanup path. The reviewed state
schema fingerprint is `1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`,
corresponding to the complete schema captured in
[the M8.1 evidence](../0.166.0/lifecycle-m8-1/state-schema-5.json). See the
[source and coordination review](../../../docs/research/codex-0.166-storage-and-coordination.md)
for exact source provenance and limitations.

The admitted cut is one canonical legacy/plain rollout and its exact row, not
native descendant delete or global erasure. Active and archived rollout headers
and known state rows are checked for ambiguity and references. Compressed,
reference-backed, paginated, reverted or unsupported target history is refused.
Unknown nonprimary stores, the shared name index and command history remain
residuals. Other configured or historical external stores are not searched; the
bounded declared-root scope and observable configuration restrictions are
specified in ADR-0017. Known external selections are not admitted.

The mutation fence requires existing native maintenance and writer-lock
infrastructure, existing regular single-link WAL and SHM files on the same
filesystem, absent rollback journal, and a fresh SQLite connection that actually
acquires WAL exclusive locking. Those coordination files are retained. The same
fence remains held through the required absence and collateral verification.
No generic SQL or file operation is exposed by this profile.

Synthetic fixtures, explicit process-termination cuts, and the opt-in disposable
exact-binary acceptance do not operate on real user state. The evidence of a
passing test on one host does not replace exact-final-commit CI and review.
