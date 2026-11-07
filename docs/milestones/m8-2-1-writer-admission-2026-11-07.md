# M8.2.1 writer admission result, 2026-11-07

**OPEN. Structural Candidate. Graphical NotTested. Desktop admission disabled.**

The smallest demonstrated obstacle for a target with global-state references
is safe replacement of an existing shared JSON file. The current process
census, native locks, index lock, companion checks and final destination
validation cannot prevent a provider writer publishing between that validation
and rename. Closing Desktop is an operating prerequisite and supplies no
kernel exclusion for that interval.

## Reproduced failure and enforced refusal

The new `late_writer_counterexample_requires_global_publication_refusal`
test operates exclusively on a disposable fixture. The parent holds both the
index lock and an exclusive lock on the main JSON inode. A real child process,
scheduled after the last destination stat, writes a new complete provider
image by atomic rename. That image changes an unrelated peer's metadata.
The private replacement primitive then overwrites it, and its final digest
check still succeeds. The child publication receipt proves the concurrent
update happened. The retained peer metadata is the older value.

This test demonstrates a defect in the unpublished primitive's concurrency
contract. The production wrapper now independently refuses global actions with
nonzero target units. Domain admission also excludes them, and planning adds
`SharedWriterCoordinationUnavailable` with zero executable operations.
The CLI names this limitation and explains that closing Codex does not resolve
it. No test grants schema-4 execution authority. Existing plans, approvals,
journal encodings and schema-2/3 authority retain their prior meanings.

Reproduce the deterministic concurrent process test with the normal test entry.

```sh
cargo test --locked -p agentmender-provider-codex --lib \
  late_writer_counterexample_requires_global_publication_refusal
```

## Mechanisms evaluated

The previously inspected exact Desktop main process persists a whole cached
Map without a compatible external writer lock. No validated native operation
or coordination transport closes this adapter's approved global, catalog and
primary effects. Advisory locking does not exclude the child writer in the
counterexample. Atomic rename and swap do not condition replacement on the
expected old file. [Exact Desktop contract](../research/codex-desktop-role-closure.md),
[Apple rename contract](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/man/man2/rename.2).

Darwin exposes `F_SETLEASE` in the installed SDK. A separate owned C probe on
macOS 26.6.2, build 25G83, Darwin 25.6.0 returned `-1/EPERM` both for a file
write lease and a directory read lease. Apple source requires the private
`com.apple.private.vfs.file-leases` entitlement. This is no available
unprivileged exclusion mechanism for this build of AgentMender.
[Apple lease implementation](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/vfs/vfs_subr.c).

## Current capability and remaining work

The baseline was clean `d41f46fb34732548649a3ec282b8940ec5cbb00d` locally and on
origin, on `milestone/m8-2-1-complete-session-removal`. Its
[exact CI run 42316964428](https://github.com/therabo/AgentMender/actions/runs/42316964428)
passed all three platforms. Installed build 24141 and recovered build 23880
retain separate candidates. Neither is admitted. Their catalog/index component
tests and expanded primary execution evidence remain available.

The [structural coverage report](m8-2-1-structural-2026-11-07.md) still applies to
catalog/scan, main/backup, index, pins/projects, search/archive, legacy/browser
sources and independent automation/activity records. Required renderer source
coverage remains unresolved. The integrated planner protects a foreign-host
identical UUID through refusal. Shared metadata, unknown artifacts and optional
history/log/queue/goal/snapshot families remain protected. No Desktop storage
reclaim or complete schema-4 execution is claimed.

The complete approval, intent, action-receipt and structural-verification path
cannot be exercised safely for the rejected Desktop scope. Primary crash and
read-only journal recovery tests still run. A full Desktop crash matrix
requires a writer protocol that permits those effects, followed by persistent
source closure. The failed publication strategy does not prove that every
future provider-supported strategy is impossible.

The [standalone manual workflow](../codex-desktop-manual-acceptance.md) remains
read-only through the current candidate plan. It may execute only a future
admitted Ready plan. Graphical confirmation belongs to the user under ADR-0021.
Final local and exact-commit CI receipts are stored under the internal
`desktop-complete-profile/admission-blockers-2026-11-07` folder.
