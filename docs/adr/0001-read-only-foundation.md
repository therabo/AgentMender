# ADR-0001: A physically read-only hexagonal foundation

**M8.2 cross-reference (2026-10-31):** [ADR-0017](0017-scoped-cleanup-residual-preservation-provider-local-mutation.md)
partially supersedes native-only rules for its exact Codex scoped-local contract.
Semantic direction is accepted; implementation remains OPEN. The historical
native/read-only contract and evidence below remain unchanged; see
[current scoped-cleanup status](../scoped-session-cleanup.md).

## Status
Accepted foundation, 2026-10-28. Process and evidence policy refined by
ADR-0002/0003; the narrow OS query boundary is documented in ADR-0004.

## Context
AgentMender inventories valuable coding-agent state. The complete internal research
(37 pages) and milestone specification were reviewed before repository changes.
M0/M1 must have no provider-state mutation capability, including incidental process
startup writes. Official Codex thread/list repairs metadata by default.

## Decision
Use six small Rust crates: domain, provider-api, application, provider-codex,
provider-claude and cli. Domain depends only on serialization and typed errors.
Application depends only on domain and read-only provider ports. Adapters translate
external evidence; CLI is the composition root and invokes application use cases.
No MCP, graph ownership solver, persistence, recovery, approval or lifecycle methods.

Provider subprocesses must run under an OS-enforced filesystem write and network
deny policy, without an unsandboxed retry. Unsupported isolation means an explicit
unavailable capability. Codex queries use only initialize, initialized and thread/list
with useStateDbOnly:true and explicit source filters. Startup failure is visible;
no repair is attempted. Version checks use the same process boundary. Filesystem
fallback is internal forensic evidence, exact-version pinned, bounded and excludes
symlinks. Unknown versions receive metadata-only audits and protected artifacts.
Provider paths and observations never establish destructive ownership.

Snapshots describe collection completeness and diagnostics; partial discovery is
never reported as a verified provider health check or a consistent atomic snapshot.
Fixture provenance distinguishes synthetic contract examples from real captures.

## Document discrepancies
The research recommends early encrypted recovery; milestones defer security to M11
and this task prohibits recovery. Defer all recovery, including encryption.
The research roadmap places MCP after mutation; milestones place read-only MCP at
M5. Follow milestones; no MCP in M0/M1. M1 parent/child inventory precedes M2 graph:
record evidence-bearing references only; do not implement ownership inference.
Research Full compatibility is intentionally absent: this binary cannot authorize
or execute mutations. Exact known interfaces are ReadOnlyKnown only.

## Consequences
Native provider startup may be unavailable under strict isolation. This is a visible
coverage limitation, not grounds to weaken the no-write guarantee. Cross-platform
process integration remains unavailable until equivalent isolation is verified.
A new provider release never inherits parser compatibility by semver proximity.

## Validation
Cargo dependency tests enforce layer direction. Safety fitness checks reject
provider mutation APIs, filesystem writes and arbitrary process dispatch. Adapter
fixtures cover unknown versions, malformed records, partial inventory and links.
Real-provider tests are opt-in and use the same write-denied process boundary.

The no-write boundary covers application-requested provider content and metadata
changes. Filesystem reads may update access timestamps or OS caches; independent
provider writers are outside this guarantee. Complete fixed observations are
normalized deterministically; truncated directory sample membership is not
stable across filesystem enumeration orders.

Observed project/cwd paths retain the provider platform's absolute-path syntax
(including POSIX, drive-rooted Windows and UNC forms) without filesystem access.
Live state roots and executables require native absolute paths. This keeps
portable synthetic fixtures distinct from actual local read boundaries. A native
project appearing in several cwd/worktrees does not acquire the first observed
workspace as its canonical root.
