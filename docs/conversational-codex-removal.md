# Conversational Codex primary-session removal (P0-C1)

Status: **OPEN**. Start with the [real-session quick start](p0-c1-manual-acceptance.md)
and [real-world report](milestones/p0-c1-real-world-integration.md). Normal browsing
uses the installed Codex, not a synthetic root or historical binary.

Real conversational metadata search is now accepted, separately from the OPEN
deletion verdict. [Search and management identity](session-search.md) governs
bounded content evidence and guaranteed exclusion under ADR-0026. Trusted registered identities
are filtered before matching/projection in both modes and refuse preview/prepare;
automatic caller binding remains incomplete. Unregistered browsing is diagnostic
metadata access and may include its caller.

## Current real-session read mode

`agentmender --provider codex mcp --session-search` adds six conversational
tools to the original eight. It needs no transaction state root or caller UUID.
It uses the protected `codex-state-threads-columns-v1` reader, not the exact cleanup
profile: validate fixed required metadata columns/value bounds in an in-memory
snapshot under kernel source-write denial. Unknown producer versions/additive
columns can be readable. Display names, title/project basename/UUID, timestamps,
archive state and optional spawn parent are disclosed; transcripts/auth/config are
not exported by legacy metadata. Explicit bounded content search and signed
context are separate read-only requests; see [the contract](session-search.md).
Guaranteed management mode requires independently controlled registration and
refuses before provider access when absent. Coverage/relationships stay partial. Optional labels over 4096 bytes
are unavailable; the bounded inventory is at most 500 recent records/96 KiB.

Installed native methods are inspected from generated schemas under isolation.
Advertisement does not establish effects. Read-mode `preview_removal` is an exact
selected-identity blocked assessment, with no plan digest or execution authority.
Preparation/status/handoff refuse with `read_only_removal_unavailable`; this mode
has no cleanup port, approval key or owned state. Caller binding is not inferred.
[ADR-0024](adr/0024-adaptive-read-only-metadata.md) separates these capabilities.
The following exact contract applies to the **existing destructive fallback**.

## Supported boundary

Select the exact official stable Codex **0.166.0**, Darwin arm64 executable SHA-256
`39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`, exact
state_5 schema, root-local WAL and one unshared plain Legacy rollout. P0-C1 selects
`scoped-cleanup-v1` (plan schema 2), with no new artifact effects. Root/config,
physical identity, dependency and writer restrictions in the
[primary contract](scoped-session-cleanup.md#initial-exact-codex-contract) all apply.
The pinned executable is hashed, not launched, for this local fallback.

Desktop observed on 2026-11-08 is 24614/26.1029.61003 with alpha.6 (the prior
24141/alpha.5.1 observation is historical). These are separate identities.
Unknown/alpha releases, Linux/Windows mutation, shared histories,
spawn dependencies and uncertain writers refuse. General historical App Server
inventory and this stable exact-schema metadata reader have separate compatibility.
Current native `thread/delete` is not dispatched; see retained
[native evidence](../compatibility/codex/0.166.0/lifecycle-m8-1/README.md).

## Advanced fallback setup and management protection

A human or trusted host must independently obtain the exact management UUID and
register it before launching the server. Registration is an exclusion contract,
not proof that MCP authenticated the current caller. A title/cwd/Active label or
model-provided caller ID is insufficient. This initial profile requires the
registered identity in the selected root. A host whose management session lives
elsewhere needs a separately verified binding; do not register an arbitrary peer
and claim caller protection. Missing/incorrect registration is an acceptance gap.

Use explicit canonical root/executable paths and a private, disjoint AgentMender
state directory. The human controls the executable, configuration, provider root,
registration and external terminal. If the coding agent can alter that configuration,
read the authority key or synthesize terminal input, same-user authentication does
not enforce independence. This local development environment has those capabilities;
its tests do not establish a strongly isolated production deployment.

A separately configured stable-profile fallback launcher (not normal read-only onboarding):

```toml
[mcp_servers.agentmender]
command = "/absolute/path/to/agentmender"
args = [
  "--provider", "codex",
  "--codex-root", "/absolute/canonical/disposable-codex-root",
  "--codex-executable", "/absolute/pinned/codex-0.166.0",
  "--protect-codex-session", "EXACT-MANAGEMENT-UUID",
  "mcp", "--session-removal",
  "--state-root", "/absolute/private-agentmender-state"
]
```

The block above documents the launcher shape. **Do not save config.toml inside
the selected provider root**: this primary profile deliberately refuses root-local
configuration. For a same-root disposable management client, supply equivalent
`-c mcp_servers.agentmender.command=...` and `-c mcp_servers.agentmender.args=...`
overrides on its trusted launch command, then resume the independently verified
management UUID. The server's inherited CODEX_HOME must agree with the selected
root. A config-only `mcp add/list` check in a different isolated home validates
syntax, not this complete provider/caller deployment. Any paginated history store
or incompatible live writer introduced by the real client still refuses; that
exact-client gate remains unaccepted.

Codex supports stdio launchers in `mcp_servers` and `codex mcp add/list`;
[official MCP setup](https://learn.chatgpt.com/docs/extend/mcp) describes the client
configuration. Do not restart the running development application. The default
`agentmender mcp` continues to expose only the original eight read-only tools.

## Search and selection

`session_search` accepts `{query, include_archived}`. Query is at most 256 bytes;
every whitespace-separated term must occur in title, project basename or exact
UUID, case-insensitively. The calling agent interprets natural language; this is
neither semantic nor transcript search. An empty query enumerates bounded metadata.

Results disclose provider-scoped opaque target and root namespace, canonical UUID,
title or reliable UUID label, project basename, creation/activity Unix seconds,
archive state, exact profile and producer compatibility. Unavailable fields are
null. `can_request_preview` describes eligibility to ask for a fresh assessment;
it does not mean Ready, approved or writer-safe. Non-archived Active is a provider
presentation label, not proof of a live writer or caller identity.

Titles/project names are untrusted data. Full paths, transcript bodies, previews,
first-user-message and raw databases are excluded. Metadata is opt-in because
labels can themselves contain private text. Existing eight-tool privacy is unchanged.
The adaptive reader bounds inventory at 500 records/96 KiB; Partial results cannot prove
absence. Responses over the MCP budget fail explicitly; narrow the query.

Return all unprotected matching candidates, including duplicate/similar titles, archives and
same-project peers. Ask the user to choose an exact UUID/root namespace. The same
UUID/title in another root is a different identity. Opaque references expire on
restart and never authorize removal. Search does not infer ownership.

## Preview, preparation and human handoff

1. `preview_removal({target})` freshly resolves the exact identity and protection,
   then invokes the existing primary planner. Review includes original digest and
   fingerprint, exact version/binary/schema/profile, affected session set, payload
   count, exact row-set counts, retained commitments, protected refs, bytes,
   postconditions and residual disclosure. The fallback is already selected.
2. `prepare_removal({target, expected_plan_digest})` repeats fresh validation and
   requires the reviewed digest. It prepares the existing durable transaction and
   reconciliation metadata. It creates no approval. Identical retries return the
   original attempt; use `removal_status({})` if a preparation response was lost.
3. `request_removal_execution({transaction_id})` reports status and
   `execution_channel: trusted_external_terminal`. **It does not execute.**
4. The human independently runs the commands below, reviews the original plan and
   exact digest and completes the terminal challenge. Never have the model type
   that challenge or access/sign with the authority key.

```sh
agentmender --provider codex --codex-root ROOT --codex-executable PINNED_BINARY plans inspect DIGEST --transaction-id TX --state-root STATE
agentmender --provider codex --codex-root ROOT --codex-executable PINNED_BINARY approve DIGEST --transaction-id TX --state-root STATE
agentmender --provider codex --codex-root ROOT --codex-executable PINNED_BINARY transactions --state-root STATE execute TX
```

The CLI restores protected identities from the immutable original plan even when
flags are omitted. Additional `--protect-codex-session` registrations invalidate the
old scope rather than weaken protection. If registered management state disappears
or is unsafe, fresh planning refuses. Do not close/restart Codex automatically:
writer contention returns refusal. The human can choose a compatible deployment.

The authority chain is original plan → protected recovery → independent terminal
grant → fresh same-plan revalidation → consumed one-shot DurableIntent → exact
adapter effects → durable observations → structural primary verification.
Approval lasts at most 300 seconds. Denial issues no grant; expiry/revocation needs
a fresh attempt, and MCP never silently creates one under the old approval.

## Status, interruption and results

`removal_status({transaction_id})` reads authenticated durable knowledge;
`removal_status({})` lists matching protected primary attempts for the configured
root. It works after disconnect/restart without an ephemeral target reference.
No grants, signatures, recovery paths or raw review data are exported.
`primary_verified` requires durable Succeeded plus the exact primary observation:
payload removed, canonical target row absent and retained primary state unchanged.
Its basis is `recorded_fenced_cutoff`, not a new eternal nonexistence claim.

Example verified presentation:

```text
Session: Codex session EXACT-UUID
Primary session removal: Verified
Other sessions: Preserved at the fenced cutoff
Additional technical artifacts: Not fully cleaned
Graphical confirmation: Not requested
Recovery: reconciliation metadata only; no payload restore
```

The prior title remains in the user's selection; the durable report uses a reliable
UUID label because it does not store a new title index. Complete Desktop cleanup,
physical reclaim and secure erasure are not implied. No new global outcome enum,
plan version, journal version or historical Removed meaning is introduced.

Before intent, failure grants no dispatch. After intent, missing effect/response
knowledge remains OutcomeUnknown/RecoveryRequired. Inspect the original attempt;
read-only `transactions ... reconcile TX` cannot redispatch or restore payloads.
Client/server cancellation drains admitted workers; a lost response is recovered
by status, never a second deletion. New dependencies or changed scope need a new
preview and independent review. Unknown effects never trigger native/local fallback.

Specific MCP errors `protected_management_session`, `management_protection_required`
and `removal_plan_changed` require choosing an unprotected target, verifying trusted
registration or reviewing a fresh plan, respectively. `capability_unavailable` or
`compatibility_blocked` requires checking the exact profile/config/writer contract
in the external terminal; no bypass flag exists.

## Acceptance and reproduction

Normal workspace tests cover deterministic metadata/authority/protection behavior.
The opt-in test below uses the actual stdio server, real protected worker, exact
pinned binary, MCP preparation, a **fixture grant**, CLI provider effects, independent
structural verification, retry refusal and reopened MCP status. It preserves the
management peer, duplicate-title same-cwd peer and an entire other root.

```sh
AGENTMENDER_CODEX166_BINARY=/absolute/pinned/codex-0.166.0 cargo test --locked -p agentmender --test conversational_removal -- --ignored
```

This is real protocol/provider integration, not an actual Codex conversation or
independent human approval. A passing P0-C1 still requires a real management client,
verified registration, human selection/review/challenge outside model control,
actual supported execution and verified unrelated-session preservation. No fixture
grant, CI success or manual GUI observation can substitute for that gate.
