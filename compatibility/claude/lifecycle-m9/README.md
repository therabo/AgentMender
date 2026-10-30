# M9 Claude native evidence: no enabled execution profile

Observed 2026-10-30 after [M8 formal integration](../../../docs/milestones/m8-integration.md).
This directory contains categorical investigation evidence, not caller-importable
proofs or lifecycle compatibility. **M9 is OPEN; all six operations are Unsupported.**

Exact installation identities, executable/signing-file hashes, isolation facts,
probe counts, counterexamples and raw-evidence commitments are in [evidence.json](evidence.json).
Desktop 4.29.0 (`com.anthropic.claudefordesktop`) owns Code 2.1.222
(`com.anthropic.claude-code`). Code's executable SHA-256 is
`16ad925cea3d7752af589f8a63b0d5689b2ad3e95f01797847e6034715e84e12`.
Current-version installation metadata does not identify historical producers.

## Provenance and findings

[Official sources and bounded claims](sources.json) distinguish current documentation
from exact installed help/behavior. The native candidate is `claude project purge`;
current documentation uses its later name `claude purge`. No provider schema or
native complete-verification contract was established. Existing 2.1.311 fixtures
remain SyntheticOnly and confer no authority over this installed binary.

Initial investigation ran 17 native probes; three fresh reproductions ran 15 each:
62 total, including 24 `--yes` invocations solely on generated synthetic state.
The final checked script reproduced six findings, 6 passed / 0 failed:

- Dry-run changes configuration and creates a backup before printing the plan.
- Colliding project selectors can delete two distinct projects' transcripts.
- Parent selection cascades into a nested configured project.
- Denied first removal can exit 0 without removals or completion.
- Denied later removal can exit 0 after six task/debug/file-history removals.

The first bullet accounts for two checked findings. These counterexamples disprove
read-only dry-run, path-based exclusive closure and exit-code success assumptions.
They do not establish every possible effect or authoritative negative proof.
Raw reports contain only synthetic fixtures and stay outside the repository; their
hashes are retained. No user transcript, prompt/reply/source body, raw database,
credential, token or cookie is copied into this evidence directory.

## Explicit isolated reproduction

The [opt-in script](../../../scripts/probe_claude_lifecycle.py) accepts only the
exact pinned Desktop-owned executable at its installed location and explicit opt-in.
It rejects relocated bundles and unsupported non-ASCII/control-bearing Seatbelt
paths before creating state. It creates all state/targets itself;
it accepts no provider root, project/session selector, account or arbitrary command.
It requires macOS, verified outside-write denial, cleared environment, private
HOME/config/cwd, real-home content denial except the exact binary bundle, network/
fork/Mach denial, bounded output/time and child reaping. There is no weaker retry.
`--bare` avoids the keychain subprocess denied by that policy. Evidence directories
are retained privately for inspection. No actual Desktop process is launched.

```sh
python3 scripts/probe_claude_lifecycle.py /absolute/exact/claude --allow-isolated-native
```

This is an investigation reproducer on synthetic input, not a production adapter,
CI prerequisite or lifecycle acceptance test. A new binary fails the exact pin.

## Real-state read-only checks

[Discovery smoke](claude-readonly-smoke.json) records Partial/RealPartial discovery,
Unknown integrity, seven sessions, one project and 51 unchanged monitored entries.
[Planning smoke](claude-planning-smoke.json) records all five intentions for one
session and one project: ten Unsupported plans with no executable operation,
capability or effect contract, and the same monitored entries unchanged. Atime is
excluded; neither check claims global atomic capture, liveness or provider health.

See the [operation matrix and missing proofs](../../../docs/claude-lifecycle.md)
and [ADR-0014](../../../docs/adr/0014-claude-project-lifecycle-evidence-gate.md).
