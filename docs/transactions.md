# Durable transactions and restart recovery

M6 implements AgentMender-owned persistence, with no provider executor or human
approval system. A transaction, plan digest or durable intent receipt is not
authorization. MCP's eight read-only tools are unchanged. Current provider plans
remain Blocked/Unsupported; M6's positive lifecycle tests use synthetic contracts.

## Bound attempts and deterministic knowledge

`TransactionId` is 256 bits from OS randomness, identifying one attempt rather than
one plan. Multiple attempts can bind the same `PlanDigest`. `TransactionBinding`
is private immutable content constructed from a Ready `LifecyclePlan`, retaining
its original digest/state fingerprint, categorical provider/intent/native operation,
and SHA-256 commitments to target, native identity, profile, capabilities/effects,
pre/postconditions and the one M4 operation. It never persists the rich plan or
provider prose/identifiers. Equivalent M4 set ordering has the same binding.
Recovery needs the original exact immutable plan supplied again and freshly
revalidated; commitments alone cannot reconstruct or authorize a different plan.

`JournalRecord` v1 binds transaction, per-transaction sequence beginning at zero,
binding digest, previous record digest and typed event. SHA-256 uses separate
domains for binding, commitments, recovery and journal. Journal order is significant;
timestamps are omitted. `TransactionSnapshot::replay` and `advanced` are pure,
validate every transition/chain and never read state or sample randomness/time.

Normal forward states are Created → RecoveryPrepared → Prepared → OutcomeUnknown
→ EffectObserved → Verifying → Succeeded. OutcomeUnknown starts immediately on
durable Intent, before any hypothetical dispatch. An explicit observed NoEffect
may finish Failed. An observed Unknown or failed/unknown verification instead
requires recovery. A missing observation never means success or failure.
Terminal states reject further events; no retry is inferred or attempted.

Compensation has distinct intent, unknown, observed, verification and Compensated
states. It requires a known Applied forward observation and an explicit semantic
operation/evidence commitment in the recovery manifest. Default production
manifests say NotCompensatable; no inverse is inferred and no production compensation
workflow exists. Synthetic tests exercise this state machine. Compensation is a
Saga action, not rollback of an external provider through SQLite.

## Store, schema and concurrency

The eighth crate, `agentmender-persistence`, depends internally only on domain.
It owns all new durable writes. A lifetime OS file lock enforces one writer across
processes. Readers take a shared lock; they fail busy while a writer owns the root.
Expected sequence rejects stale/duplicate advancement. SQLite BEGIN IMMEDIATE
provides a second serialization boundary. No durable status cache can disagree
with the history: all snapshots are replayed from records.

SQLite schema v1 uses application_id `0x414d3631`, user_version 1 and a fixed events
table with unique (transaction_id, sequence), explicit global position, and UPDATE/
DELETE rejection triggers. New schema initialization is atomic. Existing unknown,
unversioned nonempty, modified schema or non-DELETE journal mode fails closed before
changing persistent PRAGMAs. `integrity_check` and `foreign_key_check` run on open.

Writer configuration is DELETE rollback journal, synchronous=EXTRA, foreign_keys=ON,
trusted_schema=OFF, temp_store=MEMORY and a two-second busy timeout; values are read
back. macOS additionally uses fullfsync=ON. The bundled SQLite is 3.54.1 through
existing rusqlite 0.41.0. Low throughput favors simpler rollback files over WAL and
checkpoint handling. Readers use READ_ONLY|NOFOLLOW and query_only=ON, without
initializing files/schema. Existing rollback sidecars must be private, regular and
single-linked; WAL/SHM sidecars are rejected before SQLite access.

`transactions.journal` is an independent append-only JSONL copy of the same
records as SQLite. Append validates first, writes and syncs the external record,
then inserts/commits SQLite. Only afterward can the private, non-cloneable
`DurableIntent` be constructed. It binds transaction, sequence and intent digest;
recording the observation requires that matching receipt. Any ambiguous write or
commit error poisons that instance. Reopen requires exact histories and valid replay.
External-only, database-only or torn records stop advancement without truncation,
silent repair or automatic replay of effects. `TransactionReader::external_prefix`
can recover independently verified audit evidence if SQLite is damaged; that prefix
is explicitly not a committed snapshot or proof of provider state.

Histories have conservative bounds of 64 MiB, 100,000 records and 128 KiB per record.
Exceeding a bound fails closed; M6 introduces no pruning, rotation or retention policy.

## Recovery ordering and crash windows

`RecoveryManifest` v1 and `RecoveryPoint` describe minimal reconciliation metadata:
transaction/plan/state/provider/operation/target commitments, ReconciliationOnly
strategy and explicit compensation capability. This is not a provider backup and
does not promise restoration. No arbitrary blob API, provider database, transcript,
prompt/reply/source body, credential, token or cookie is stored.

The store creates a private random staging file, writes and syncs it, and publishes
the digest name atomically with a same-filesystem hard link that cannot replace an
existing artifact. It checks held/name identity and link counts, removes only the
temporary staging link, syncs the directory and reopens/verifies canonical bytes.
Existing final artifacts are synced and verified again. Interrupted stages remain;
interruption with both links present yields protected recovery, never silent cleanup.
Final recovery artifacts are never overwritten or automatically deleted.

`prepare_recovery` must publish and verify the manifest before journaling
RecoveryPrepared. `record_intent` takes the original plan plus fresh graph, integrity
report and planning context; it calls M4 `validate_against` and compares the complete
binding inside that operation, then verifies recovery bytes again. The Intent event
binds the matching plan/state validation. Prepared alone never admits an effect.
There is no production effect port, closure, shell/SQL command or provider binding.
The test-only fake effect consumes the real receipt and independently checks intent
in both stores before changing a test marker.

| Crash or failure | Durable interpretation |
|---|---|
| Before intent | No effect is considered attempted; prior committed knowledge remains |
| After intent, before hypothetical dispatch | OutcomeUnknown; no assumption that dispatch did or did not occur |
| During fake effect or after fake effect before observation | OutcomeUnknown, regardless of test marker |
| After observed result commit | Replay preserves that explicit result; verification is still required |
| During compensation intent/outcome window | CompensationOutcomeUnknown; no guessed rollback |
| Between external sync and SQLite commit, torn/corrupt journal | Protected error/divergence; verified external prefix only for diagnosis |
| Missing/corrupt recovery artifact | Journal knowledge remains inspectable with protected availability; advancement blocked |

The application `RecoveryCoordinator` reports incomplete/protected durable knowledge.
It never invokes a provider, retries, reconciles or fabricates an outcome. Fresh
provider observations and independent authorization belong to M7–M9.

## Explicit owned root and local inspection

The library requires an explicit absolute `OwnedStateRoot` plus protected provider
roots. No default persistent root is created. Creating the root is a trusted local
composition decision; normal providers/scan/plan/MCP commands never initialize it.
Root overlap in either direction, traversal, static case/Unicode/symlink aliases,
dangling protected links, reparse points, nonregular/multiple-linked files, changed
identity and broad permissions are rejected. Private child names are fixed or
derived from validated digests/OS random staging IDs.

Unix uses 0700 directories and 0600 files owned by the current effective UID.
Windows applies a protected current-owner-only inheritable DACL before creating
any content; existing directories and inherited file ACLs are checked, together
with handle-based volume/file identity and link counts. No new unsafe Rust or
unsandboxed process dispatch is introduced.

Read-only CLI commands inspect an existing root, without provider discovery:

```sh
agentmender transactions --state-root /absolute/agentmender-state list
agentmender --json transactions --state-root /absolute/agentmender-state inspect <transaction-id>
agentmender --json transactions --state-root /absolute/agentmender-state journal <transaction-id>
```

JSON says `mode: durable_knowledge_only`, `provider_state_verified: false` and
`execution_authorized: false`. Recovery availability and protection remain explicit.
CLI checks standard homes, configured/env provider roots, Desktop stores and app
bundles for overlap. Missing/empty state is never initialized by inspection.
No create/execute/resume/retry/rollback/approval command exists.

## Durability and security limits

File sync, directory sync on Unix and SQLite commit are implemented, with errors
propagated. Actual power-loss persistence depends on OS/filesystem/device honoring
those requests. Windows directory-entry power-loss synchronization, ReFS 128-bit
identity guarantees, malicious same-user ancestor replacement and cross-filesystem
provider content staging require M10. Use local filesystems; network filesystems
are outside this contract. Tests prove process interruption and reopened committed
knowledge, not hardware fault tolerance. macOS external files use File::sync_all;
SQLite's fullfsync does not upgrade those file flushes to hardware guarantees.

There is no encryption or strong confidentiality-at-rest claim. Hash chains detect
corruption/inconsistency, not a same-user attacker rewriting both copies. M11 must
define keys, tamper resistance and retention. Recovery data is deliberately minimal
and conservatively retained until those policies exist.

## Validation

Nine generated transaction properties use 256 cases each, fixed seed
`0x4d365f74786e7331` (2,304 generated cases), plus four focused regressions. They
cover valid/invalid replay, ordering, duplicate/reordered/corrupt envelopes,
immutable commitments, recovery-before-intent, missing outcomes, all terminal
states, independent verification and compensation uncertainty.

Persistence tests abort child processes before/after writes, external flush,
SQLite insertion/commit across every forward transition, recovery publication,
fake effect and compensation intent. Deterministic error injection simulates
write/flush/commit failures without filling disks. Additional tests cover schema,
triggers, corruption/prefix recovery, reopens, stale validation, wrong receipts,
cross-process locking, concurrent manifest publication, private paths/permissions,
static aliases, sidecars and content integrity. CLI byte manifests prove inspection
does not modify state. All previous graph/integrity/planning/MCP tests remain intact.

See [ADR-0011](adr/0011-durable-transaction-knowledge.md) for alternatives and the
accepted failure model. This substrate does not authorize or implement mutation.
