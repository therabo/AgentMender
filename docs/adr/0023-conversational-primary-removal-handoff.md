# ADR-0023: Conversational primary-removal preparation and external human execution

## Status

Accepted for the bounded implementation, 2026-11-07. Full P0-C1 client/human
acceptance remains OPEN. This extends ADR-0010's read-only MCP boundary only for
AgentMender-owned transaction preparation; ADR-0012's human authority and
ADR-0017's provider effects remain unchanged. ADR-0022 still governs sequencing.

## Context

The eight existing MCP tools hide labels and cannot hand a selected Codex session
to the supported primary-removal engine. The installed Desktop build 24141 embeds
0.166.0-alpha.5.1, which cannot inherit stable 0.166.0 authority. The exact stable
0.166.0 Darwin arm64 binary/profile remains a usable separate primary worker.
Its native delete is not admitted: retained M8.1 evidence includes hidden spawned
cascades, partial deletion, stale canonical metadata and startup effects.

The current coding agent can run shell/PTY commands and read same-user files.
Neither a terminal challenge nor an authority.key owned by that same user proves
independent human participation against that adversary. No authenticated caller
session identity is available from stdio MCP client metadata.

## Decision

1. Keep the original eight tools and privacy projections unchanged by default.
   Explicit `mcp --session-removal --state-root PATH` adds metadata search, exact
   preview, durable preparation, status and an external execution handoff. The
   inbound crate has no persistence dependency, signing key, grant or effect port.
2. Select historical schema-2 scoped primary cleanup before review. Do not start
   native delete or fall back after an uncertain operation. No Desktop global,
   index, history, new artifact family, Claude or bulk effects are admitted.
3. Add a separate metadata read port. An opt-in protected worker reads only fixed
   canonical ID/title/cwd basename/creation/activity/archive/producer columns from
   the exact schema. It never exports preview, first-user-message or transcripts.
   Bound inventory at 500 records/128 KiB; disclose Partial rather than infer absence.
   Titles and project labels are untrusted data and never establish ownership.
4. Require independently registered `--protect-codex-session UUID` identities in
   trusted launch configuration. Tools cannot supply or alter them. Registration
   is a human/host deployment responsibility, not authenticated caller discovery.
   Each registered identity must exist in the selected root's fresh inventory.
   Insert UserProtected session roots into the existing graph before planning;
   existing plan/hash/binding machinery freezes these retained identities.
5. Reject protected targets in both pure planning and the provider. Restore frozen
   protections from the original plan during external CLI approval/execution even
   if launch flags are omitted. Adding a protection invalidates old plans. Provider
   preflight/dispatch checks the frozen set, and fresh validation repeats exclusion.
   The existing exact writer/SQLite fences protect unrelated sessions. Historical
   CLI plans without registration do not acquire a caller-binding guarantee.
6. Application orchestration prepares the existing transaction and recovery
   metadata after fresh exact preview/digest validation. Identical preparations
   recover the existing attempt, including terminal or unknown knowledge; they
   never reset approval. Transaction lookup survives server restart. Incomplete
   preparation is visible for inspection; no automatic effect retry is introduced.
7. Keep human approval **and execution** in a trusted external terminal for this
   deployment. `request_removal_execution` only reads status and names that handoff,
   even if a grant exists. It cannot grant, consume or execute. Human review uses
   the exact original immutable plan/digest, including root, protected identities,
   payload/row effects, residuals and irreversibility. The human must operate this
   boundary independently; same-user PTY or whole-store/key access remains outside
   the authentication guarantee. Do not advertise OS-enforced separation here.
8. Present operation-qualified primary success from authenticated durable
   Succeeded + supported CleanupObserved postconditions. Require payload removal,
   canonical absence and retained-primary invariants at the recorded fenced cutoff.
   Disclose additional artifacts as not fully cleaned, GUI as not requested and
   physical reclaim as unknown. Preserve old outcomes, schemas, hashes and legacy
   CLI reports. Protected conversational schema-2 plans receive the new presentation
   and exit 0 only for verified primary success; legacy CLI semantics remain intact.

## Alternatives and consequences

A model-supplied caller ID, confirm flag, PTY-produced challenge or tool annotation
cannot supply human authority. A new approval engine would duplicate existing
security machinery. Universal Desktop cleanup would defer useful primary removal
without addressing this caller/approval boundary. An OS-separated broker or
provider-authenticated caller binding could support a later stronger deployment,
but is not fabricated here.

The result is a useful, bounded MCP-to-terminal workflow and actual provider
execution on disposable state. It is not production-admitted autonomous deletion
from an unrestricted coding-agent session. Actual conversational client use with
trusted independently operated approval remains an explicit acceptance gate.

## Validation

Retain historical wire/replay properties, approval expiry/revoke/consume/crash
checks, native counterexamples, empty schema-4 admission and the lost-update
regression. Add metadata collisions/partial/version/privacy tests; protected target
and stale protection refusal; exact digest/retry tests; real stdio preparation,
external CLI effect and reopened status on disposable pinned-profile fixtures.
Fixture grants must be labeled test authority, never independent human acceptance.
Architecture fitness admits preparation only in application/conversation.rs while
still rejecting grant/intent/effect authority there or persistence access in MCP.
Full locked local and exact-final-commit three-OS CI gates remain required.
