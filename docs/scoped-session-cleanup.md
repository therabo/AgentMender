# Scoped session cleanup and complete session removal

## P0-C1 protected primary workflow

The [conversational handoff](conversational-codex-removal.md) reuses historical
scoped-cleanup-v1/schema 2, with registered management exclusions committed through
the existing graph and plan. Human approval/execution remain external. Protected
conversational plans report verified primary success separately from complete
Desktop removal; legacy plans retain their original JSON/exit/assessment meanings.
No schema-3/4 effect, global JSON publication or new artifact family is enabled.


## Product-first capability boundary

[ADR-0022](adr/0022-product-first-session-lifecycle.md) separates useful primary
session removal from optional deep artifact and Desktop projection cleanup.
M8.2 remains **PASS**, integrated at `a284767157ace8769deee85b195b5b70efe374d8`
([receipt](milestones/m8-2-integration.md)). Its exact profile and immutable
encodings below remain unchanged. A primary receipt establishes only its own
payload/canonical-row and retained-state postconditions at the fenced cutoff.

M8.2.1 is **unfinished/deferred** under P1/P2 and **no longer a global blocker
for the conversational MVP**. Both schema-4 Desktop candidates (24141/23880) remain
unadmitted. Global target publication is refused because the real child-process
counterexample demonstrates lost unrelated updates despite final checks and
advisory locks. Closing Desktop does not resolve that race. See the
[writer checkpoint](milestones/m8-2-1-writer-admission-2026-11-07.md).

The stronger SessionRemovalAssessment is retained for complete persistent
projection claims. Unknown reconstruction roles, residual required surfaces,
incomplete structural evidence or collateral drift prevent that claim. User
GUI confirmation is separate under ADR-0021. Production does not control or
restart an app. These are complete-Desktop gates, not a requirement to erase
every cache before honestly reporting a separately verified primary operation.

Current CLI primary success still has an incomplete complete-removal assessment
and exit 2. This audit changes neither JSON/exit semantics nor historical results.
P0 must supply operation-scoped reporting and MCP integration in a later slice.

## Expanded primary profile (schema 3)

`scoped-cleanup-v2` uses the exact stable 0.166.0 binary/schema/WAL/root and legacy
plain-history boundaries below. It additionally removes target rows from
thread_attachments and guardian_review_feedback with exact count/preimage checks.
The retained digest excludes only those target rows; every unrelated record and
shared asset remains protected. Metadata payload values never become filesystem
deletion instructions. Guardian BLOB contents are hashed transiently and never
persisted in review or journal metadata.

Expanded-primary plans carry CompleteRemovalManifest and distinct v3 hash/binding domains,
ordered PrimaryPayload/StateThreadRecords actions and explicit PrimaryOnly profile
admission. After all fences and preimages validate, the payload is unlinked and
its progress synced, then the exact main-DB transaction commits and its progress
syncs. No later effect follows a progress failure. Unknown outcomes and crash gaps
retain partial knowledge and never authorize retries. Existing journal schema 2,
independent terminal approval, recovery and one-shot DurableIntent are reused.
Old v2 plans select v1 throughout review/approval/execution; no grant is upgraded.

All approved expanded primary effects can yield a primary transaction Succeeded,
but the product assessment remains ProjectionIncomplete and CLI exits 2. No
complete Desktop removal profile is admitted. Stable worker evidence transfers no
authority to the different embedded Desktop CLI. The current sidebar is not the
reason for this incomplete result: persistent closure and profile acceptance are
missing. Index/history/global/backup/catalog/browser/automation mutations stay
unadmitted pending exact record, physical and lifetime writer boundaries.

[ADR-0019](adr/0019-structural-removal-and-profile-acceptance.md) partially
supersedes [ADR-0018](adr/0018-complete-session-removal.md). No production restart,
shutdown or process termination is added. Reconciliation is read-only. M8.3, M10
and NLP remain unstarted.

## Historical M8.2 primary-cleanup contract

M8.1's complete provider-native Codex cleanup remains **BLOCKED BY PROVIDER**.
Its answer to complete native effect/deletion/retention proof remains **NO**.
M9 Claude lifecycle likewise remains blocked with zero mutation profiles. Their
evidence, receipts and historical ADR reasoning are retained unchanged.

M8.2 asks whether one exact session's primary state and safely attributable storage
can be removed while preserving uncertain residuals. It does not require forensic
erasure of every provider, plugin, cache or global trace. Native operations remain
preferred only when their exact effects fit the approved set. Current cascading
native delete is not dispatched; local strategy is selected before approval.

## Attribution, planning and authority

The flow is discovery → ArtifactGraph → exact target → CleanupSet and RetainedSet
→ reclaim estimate → immutable LifecyclePlan → durable transaction and recovery
→ independent local approval → fresh validation → consumed DurableIntent → exact
provider-local effects → fenced postcondition verification → bytes and residuals.

`CleanupManifest` freezes target/root/profile, primary payloads, exact metadata
preimages/counts, optional index actions, physical identities, categorical binding
evidence and retained baselines/residuals. It is review data, not an imported SQL,
command or arbitrary path program. `PlanDigest`, `StateFingerprint` and
`TransactionBinding` bind it to the existing M7 approval and one-shot durable gate.
Executed effects must be a subset of approved effects. New candidates require a
new plan; unsafe optional candidates may be retained without expanding authority.

| Classification | Cleanup treatment |
|---|---|
| Exact target-exclusive binding under admitted rule | Eligible within the frozen manifest |
| Shared / Global | Retain |
| Referenced | Retain unless an independently proven exact relation mutation is approved |
| Unknown / ambiguous / unsupported / future store | Retain and report |
| Owned by another session | Preserve |
| Derived / Orphaned / same cwd / filename proximity | Descriptive only; grants no deletion permission |

Uncertainty about primary identity, sharing, trusted root, physical file identity,
schema or writer exclusion blocks the target. Unrelated unknown residuals do not.
M3 integrity remains its own read-only global/scoped health interpretation; cleanup
success must not relabel an unclosed M3 report Healthy.

## Initial exact Codex contract

The implemented exact-profile target is official `0.166.0`, Darwin arm64 executable
SHA-256 `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`,
state schema SHA-256
`1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`,
adapter `scoped-cleanup-v1`, root-local state DB, WAL, legacy plain history.
Read-only discovery, native lifecycle and scoped-local compatibility are separate
axes. An equal schema hash on a different version transfers no cleanup authority.

The requested identity includes the selected canonical root. The adapter rejects
`CODEX_SQLITE_HOME`, a conflicting `CODEX_HOME`, root `config.toml`, `*.config.toml`
or `config.d`, the three `/etc/codex/{config,managed_config,requirements}.toml`
files, and any target-cwd ancestor's `.codex/config.toml`. Missing/uncertain or
symlinked configuration ancestry refuses admission. Configuration contents are not
parsed. These observable guards are repeated under the execution fence; they do
not establish MDM/cloud/other-process argument or historical configuration. Those
unsearched external stores are disclosed residual scope. Exact Legacy readers
require the matching rollout; an external projection alone cannot restore intact
Legacy history after the selected-root primary cut.

The first profile requires a unique canonical UUID, canonical DB rollout path,
matching bounded payload metadata, safe regular single-link physical object and
stable identities. It excludes compressed/reverted/paginated/ambiguous primary
representations, independent thread-history projections and retained history-base
or attached primary dependencies. Children are not implicitly deleted. A copied
legacy fork is independent; reference-backed history is not.

Current primary payloads are deleted exactly; optional index actions are represented
but the first adapter retains the shared index without rewriting it. There is no
index/history rewrite in this implementation, and no index-rewrite safety claim.
The fixed SQL deletes the exact target read receipt and canonical thread row;
the reviewed foreign key removes its dynamic-tool rows. Target attachment,
guardian-feedback and incident-spawn rows must be zero. All other logical rows in
all 17 exact-schema tables are committed as a retained projection and reverified.

Primary success requires all approved primary replay payloads and the canonical
thread row absent at a coherent fenced verification cutoff, with retained-session
primary state unchanged. The shared name index is a title cache; exact name removal
may be optional. Command history, logs, memories, global caches, Desktop catalogs,
unknown images and unsupported stores can remain explicit residuals. Their presence
alone is not an intact resumable legacy session.

## Concurrency and mutation

Read-only source SQLite inspection stays inside the protected worker/kernel write-
denial boundary. No provider process is launched for local cleanup. Executable
identity is read-only hashed. Planning does not create provider lock files.

After DurableIntent, the adapter acquires the exact maintenance/coordination lock
namespace, verifies all existing writer locks idle, and revalidates all primary
objects. A fresh SQLite connection sets `locking_mode=EXCLUSIVE` before its first
database access and must actually obtain main-file exclusion. WAL connections
already attached, even idle, cause refusal; new reads are blocked while exclusion
is held. `BEGIN IMMEDIATE`, `BEGIN EXCLUSIVE` alone or a pragma echo is insufficient.
The same exclusive connection captures poststate after commit. Lock contention is
bounded; there is no weaker retry path or forced process termination.

This is cooperative exact-profile exclusion, not protection against malicious
same-user processes ignoring locks. A later metadata-only reappearance is new
residual state; it cannot authorize reuse of an old receipt. It does not restore
removed legacy history. A new payload/projection is primary reappearance, not a
small metadata residue. The success receipt is a cutoff observation, not an eternal
nonexistence guarantee.

Only private static exact-key SQL inside the Codex adapter is permitted. Full
schema/FK/trigger checks, preimages, affected counts and retained semantic row
projections protect other sessions. No cwd/title/LIKE ownership predicates,
subtree deletion, global VACUUM, database replacement or sidecar removal. Files
require stable safe identity immediately before relative deletion. Optional index
rewrite, if admitted, uses structural exact IDs, the native stable index lock,
non-target byte/order preservation, a private synced temporary and atomic publication.

## Outcomes, recovery and space

| Cleanup outcome | Meaning |
|---|---|
| Removed | Required primary removal and all approved optional actions verified; no retained residual known |
| RemovedWithResiduals | Required primary removal verified; protected residuals or optional failures remain; successful cleanup |
| Blocked | Essential primary safety prerequisites unavailable before effects |
| OutcomeUnknown | Possible effects with incomplete/conflicting required or retained verification |
| Failed | Decisive failure; primary removal is not claimed |

These outcomes are distinct from M6 transaction states. A provider return value
does not establish success. Payload-first ordering keeps the exact metadata
transaction prepared/validated before irreversible unlink, then commits after
payload/optional-index effects. A crash between stores remains incomplete until
observation proves the frozen required postconditions. No automatic retry,
compensation or whole-store restoration.

Recovery stores bounded typed reconciliation metadata, not a multi-GB payload
backup. Independent approval must disclose irreversibility, primary actions,
optional actions, estimated logical reclaim and retained residuals. After restart,
inspect each frozen action; never delete a replacement object or expand the set.
Missing evidence cannot reconstruct a dispatch receipt or retained baseline.

Accounting distinguishes known eligible/planned logical bytes, actually unlinked
approved logical bytes, retained known bytes and unknown residual categories.
Already-missing, failed or skipped files add no executed reclaim; aliases are not
double-counted. SQLite freelists/WAL sizes, sparse allocation, APFS clones/snapshots
and global free-space variation do not prove physical reclaim. Physical bytes stay
unknown unless independently established. Same-filesystem quarantine without final
unlink and retained payload backups do not reclaim the payload's storage.

## Verification and remaining scope

Required evidence includes seeded preservation/subset/accounting properties,
exact SQL count/rollback/schema tests, index byte/order/publication tests for every
rewritten index, root/link/substitution cases, idle/active/new writer contention,
fault cuts at payload/index/DB/journal boundaries, read-only restart reconciliation,
and an opt-in full positive disposable production path through genuine plan,
transaction, recovery, independent test approval and DurableIntent. Destructive
acceptance never uses real user sessions. All standard local gates and exact-final-
commit Linux/macOS/Windows Actions remain mandatory.

P3 may later compose frozen exact session sets using each supported primary
contract and sum independently verified results. Optional complete-Desktop or
deep-cleanup claims retain their additional gates; project association never
authorizes recursive file deletion. M8.3/bulk, local NLP, broader platform
mutation, Claude local mutation, recovery redesign and repair are not implemented
by M8.2. M8.3 is not the next priority; see the canonical roadmap.

## Local CLI workflow

Use the exact canonical UUID (a `codex:session:` prefix is accepted by the CLI).
Set an explicit canonical `--codex-root` and exact `--codex-executable` when the
defaults do not select the reviewed profile. A project or active-only filter cannot
be used for this destructive scope. A fixture flag never enables production cleanup.

```sh
agentmender --provider codex plan cleanup-session UUID --json
agentmender --provider codex plans prepare cleanup-session UUID --state-root /absolute/private-agentmender-state
agentmender plans inspect PLAN_DIGEST --transaction-id TRANSACTION_ID --state-root /absolute/private-agentmender-state
agentmender approve PLAN_DIGEST --transaction-id TRANSACTION_ID --state-root /absolute/private-agentmender-state
agentmender transactions --state-root /absolute/private-agentmender-state execute TRANSACTION_ID
agentmender transactions --state-root /absolute/private-agentmender-state reconcile TRANSACTION_ID
```

Preparation stores review/recovery metadata. `approve` requires an independent
interactive terminal challenge; JSON, `--yes` and noninteractive approval shortcuts
are unavailable. Execution consumes the grant once and performs immediate fresh
validation. Reconcile only observes frozen objects; an incomplete crash does not
resume deletion. Output includes required removal, exact logical bytes, optional
actions and retained residuals. Read-only planning and inspection never authorize
execution.
