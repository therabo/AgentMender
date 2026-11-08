# ADR-0024: Adaptive read-only metadata separate from destructive compatibility

## Status

Accepted, 2026-11-08, by the real-world P0-C1 integration request. Extends
ADR-0003/0004 and ADR-0022/0023. Supersedes the requirement for the exact stable
cleanup binary merely to use conversational search. Existing mutation contracts,
historical evidence and serialized authority remain unchanged.

## Context

P0-C1's metadata reader reused the exact stable 0.166.0 cleanup profile. This made
browsing existing sessions depend on a historical executable and synthetic
onboarding. The installed alpha.6 Desktop exposes native lifecycle interfaces,
but interface presence does not establish deletion scope or retained-state proof.
Its App Server cannot initialize the real state DB under source-write denial.
Allowing provider startup writes merely to browse would breach the read boundary.

## Decision

- Keep the eight crates. Add `mcp --session-search`, explicitly opting into useful
  metadata while retaining the original eight-tool default. Both opt-in modes
  use the existing thirteen-tool registry. Search mode needs neither a protected
  caller UUID nor a transaction state root. It cannot prepare transactions, read
  approval keys, consume intent or dispatch effects. The separate existing
  `--session-removal` mode keeps its exact stable/protection/owned-state gates.
- Detect the existing bundled Codex executable for this explicit real-session
  mode when PATH lacks it. Do not change isolated historical scans into implicit
  live-installation probes. Explicit executable/root launch configuration wins.
- Use the existing opaque EvidenceSandbox, fixed protected worker, kernel write
  denial, source SQLite READ_ONLY/NOFOLLOW, query_only and bounded in-memory Backup
  API snapshot. No auth, config or rollout body is read by this worker. Validate
  the required metadata table/column affinities and runtime value/identity bounds;
  additive columns and unknown producer versions do not require new binary hashes
  for reading. Incompatible required shapes fail without mutation.
- The fixed queries select IDs, title/display name, cwd basename, creation/activity,
  archive state, optional producer version and available spawn-parent metadata.
  Prefer the provider's display name over its first-prompt title. Oversized optional
  labels become unavailable, never invented or silently truncated. Preserve UUIDs,
  duplicate titles, root namespaces and partial relationship/coverage knowledge.
  The bounded inventory is the most recent at most 500 records/96 KiB of metadata.
  MCP's existing response budget still refuses oversize projections explicitly.
- Inspect the installed implementation's generated ClientRequest schema inside
  fresh write-confined scratch. Advertised `thread/list/read/archive/delete` names
  are observations, not requests, effect closure or mutation admission. Failure
  of this optional probe leaves native availability Unknown without preventing a
  validated metadata read. Provider-api owns the confined process; provider-codex
  interprets the schema. No new JSON dependency or generalized process API.
- Return the independent metadata/native/scoped-fallback/additional-cleanup facts
  in the search projection. Unknown versions may be readable. No new global
  compatibility state machine or permission is derived from readability.
- A read-mode `preview_removal` is an exact-identity **blocked assessment** with no
  immutable plan digest or dispatch authority. It reports unverified descendant
  effects, concurrency/postcondition requirements, residual uncertainty and missing
  caller/approval separation. It never manufactures a Ready native plan. Other
  removal tools return `read_only_removal_unavailable` in this mode.

## Safety and consequences

Native-first remains the policy once a complete operation contract fits the
independently approved set. Fresh isolated alpha.6 tests still demonstrate native
descendant deletion and partial effects before error. No native deletion is
admitted by this decision; no unknown native result triggers fallback. Exact raw
local mutation, schema-4 refusal, lost-update regressions and historical plan/
journal/hash meanings remain. The old stable fallback remains available.

Management identity is still independently registered for destructive admission;
MCP does not authenticate its caller. The real read-only launch deliberately has
no approval store or claimed caller binding. Same-user shell/PTY/key access cannot
establish independent human authorization. Current destructive acceptance stays
OPEN until an actual eligible case and independent deployment are demonstrated.

Direct fixed-column reads are coupled to a small validated structural contract,
not an entire version/schema catalog. They may miss externally selected stores,
new metadata representations or relationships. Partial results cannot establish
absence, ownership or a complete deletion set. No claim of universal compatibility.

## Validation

Test additive/unknown versions, incompatible required affinity, duplicate IDs,
display names, missing/oversized labels, archives, parent relationships and bounds.
Exercise actual stdio/worker search without a supported binary or approval root;
prove blocked assessment, preparation refusal, no state creation and unchanged
source bytes. Retain the exact stable disposable execution/recovery opt-ins and
the entire locked suite. Verify real installed-provider metadata under kernel
denial separately from actual model/client invocation, human approval, real
deletion and GUI acceptance. Ship only with exact-final-commit three-OS CI.
