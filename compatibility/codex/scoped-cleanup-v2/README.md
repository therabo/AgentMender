# Codex expanded primary scoped cleanup v2

This profile expands the exact stable Codex 0.166.0 Darwin arm64 primary cleanup
boundary to exact target `state_5` attachment metadata and guardian feedback
rows. It retains shared attachment payloads, spawn edges and unknown or unrelated
state. Existing exact binary/schema/WAL/native writer and physical identity
requirements still apply. See [v1](../scoped-cleanup-v1/README.md) for the
underlying pin and [continuation report](../../../docs/milestones/m8-2-1-continuation-2026-11-03.md)
for evidence and pending gates.

Expanded-primary preparation uses this adapter with a version 3 outer plan
manifest. The separate schema-4 Desktop candidate is described in the
[checkpoint](../../../docs/milestones/m8-2-1-desktop-candidate-2026-11-06.md). Historical schema-2 reviews select v1, without automatic upgrade; old
approvals cannot authorize expanded effects. Independent approval, consumed
DurableIntent and the existing journal v2/approval MAC govern execution. The
rollout unlink and state DB commit have separate durable action knowledge; an
unrecorded effect remains unknown and is not retried automatically.

This is **not a complete Desktop removal profile**. No catalog, index, history,
global-state or other Desktop projection mutation is admitted. A primary receipt
therefore maps to product `ProjectionIncomplete` (CLI exit 2) until the required
persistent closure and independently accepted exact-profile two-cycle restart
proof exist. There is no accepted Desktop restart/reappearance evidence and no
imported acceptance proof. The current open sidebar is not the success authority;
production does not restart Codex for visual verification.

## Verified continuation evidence

The [isolated receipt](isolated-acceptance.json) records a passing exact-binary
hash and actual sandboxed-worker test on disposable manually seeded state, plus
approved production-path metadata deletion and preservation checks. The native
Codex executable was not started. Real process crash/reopen tests cover the two
commit boundaries and preserve durable action prefixes without retries. Shared
attachment identity keys and the physical referenced asset survive. This is
primary compatibility evidence, never actual Desktop restart acceptance.
