# Local Claude deletion verification: contract unavailable

Reviewed 2026-10-30. No production Claude observer, dispatcher or project operation
was added. These conclusions concern local Code/Desktop state. Managed Agents
cloud session APIs do not supply authority for it.

## Existing pipeline

Reconstruction covered domain approval/transaction types and properties,
persistence intent/observation, recovery, ownership and crash tests, application
execution/verification/reconciliation, CLI approval composition and integration
tests, MCP authority rejection, and the exact Codex adapter/native opt-in test.
M6 commits the exact Ready plan, typed target/native identity/profile/effects and
fresh state; recovery is verified before intent. M7's independent one-shot grant
is consumed atomically with the signed intent. External journal sync precedes
SQLite commit and construction of the private noncloneable DurableIntent.

M8's workflow and native observations are explicitly session-unarchive scoped:
Active/Archived/Unknown, with no project Deleted observation. The adapter consumes
DurableIntent by value. Accepted response records an observation, then fresh
integrity and retained-state verification determine success. Reconciliation cannot
reconstruct dispatch authority. Restart without the retained baseline remains
Unknown/RecoveryRequired. None of these types may simply be relabeled as purge.

## Official local observation research

The [local CLI reference](https://code.claude.com/docs/en/cli-reference),
[application-data documentation](https://code.claude.com/docs/en/claude-directory),
[TypeScript SDK reference](https://code.claude.com/docs/en/agent-sdk/typescript#listsessions)
and [SDK sessions documentation](https://code.claude.com/docs/en/agent-sdk/sessions)
were reviewed for project identity, complete observation and deletion proof.
They expose useful local transcript/session information, without an authoritative
coherent closure over purge's target, cascades and retained namespaces.

Official Python SDK source was inspected without importing or executing it,
pinned to commit `1ea8cb74ddcdadbfaa9b20dfa8268e18047851e7`:

| Official source | SHA-256 |
|---|---|
| [sessions.py](https://github.com/anthropics/claude-agent-sdk-python/blob/1ea8cb74ddcdadbfaa9b20dfa8268e18047851e7/src/claude_agent_sdk/_internal/sessions.py) | `5faa6011b0559bb2ccbe0e54ec9dea029418001da04013a67c6d107ebd192419` |
| [session_store.py](https://github.com/anthropics/claude-agent-sdk-python/blob/1ea8cb74ddcdadbfaa9b20dfa8268e18047851e7/src/claude_agent_sdk/_internal/session_store.py) | `7c65f885209a5abc5c9d07d0a2fb5be3189fdd0c9207782f7b5ed46ddd1931a1` |

Source observations on that exact SDK commit:

- Session listing/lookup inspect UUID-named main JSONL files with bounded head/tail
  metadata reads. Sidechain, empty, metadata-only, unreadable or unparseable files
  can be omitted. Directory scan errors can produce an empty list.
- A present file without an extractable summary can return no session info.
  Message lookup can return an empty list for missing/unreadable files or no
  visible messages. Those results cannot distinguish absence from failed coverage.
- Worktree lookup and long-path fallback affect lookup scope. Project key derivation
  normalizes realpath/NFC and replaces nonalphanumerics with hyphens; short keys
  lack an injective identity commitment. This is SDK evidence, not proof that every
  internal 2.1.314 code path uses identical implementation.
- SessionStore transcript mirroring does not enumerate all purge namespaces,
  provide retained-state coherence or certify project absence.

Thus an SDK empty list/None/empty messages cannot prove a negative postcondition.
The native counterexamples independently demonstrate writable observation attempts
and selector ambiguity. Finite synthetic fingerprints are empirical measurements,
not an exhaustive provider observation contract.

## Required outcome semantics

| Outcome | Required positive proof | Current local purge |
|---|---|---|
| Applied | Lossless approved identity; fresh coherent authoritative absence across all declared target/cascade namespaces; exact retained-state invariants; valid integrity | Not established by CLI, SDK or exit status |
| NotApplied / NoEffect | Positive refusal before writable startup, or an independently validated equivalent complete no-effect certificate/history | Possible only at the pre-start gate; surviving target or exit 1 after startup is insufficient |
| Unknown | Possible startup/effects without the complete proof above, including timeout/signal/partial error/stale baseline/incomplete reads/concurrency | Must remain protected; never infer success, failure or retry permission |

An intent committed before an interrupted launch is still OutcomeUnknown after
restart unless positive authenticated outcome evidence exists. An unchanged early
test sample does not create a general post-start NoEffect rule. Native acceptance
is not transaction success. Backups are not a validated equivalent restore;
purge remains NotCompensatable.

## Observation-only reconciliation

A future implementation must authenticate consumed transaction/recovery metadata,
obtain a fresh coherent complete read-only observation under the exact validated
profile, and check the same identity/deletion/cascade/retained invariants. It may
not rerun purge, writing dry-run, repair state or remove remaining files.

No complete current local reconciliation strategy was established. Required
upstream evidence is a lossless native project identity and exhaustive membership,
version-bound startup/direct/cascade/retention closure, read-only complete coherent
observation with explicit read-error-versus-absence semantics, writer fencing or
equivalent coherence, and interruption/partial-error outcome semantics. More
successful fixture deletions cannot replace these contracts. M9 is BLOCKED pending
that provider change; all six operations remain Unsupported.
