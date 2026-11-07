# AgentMender

Safe, evidence-driven cleanup and storage reclaim for local AI coding-agent state.
The product objective is to remove an exact session from persistent storage and
every supported provider view, remove all safely attributable exclusive state,
and preserve shared, ambiguous and unrelated technical state.

**M8.2 primary cleanup: PASS and integrated. M8.2.1 complete removal: OPEN.**
The exact M8.2 commit `a284767157ace8769deee85b195b5b70efe374d8` passed local,
isolated and Linux/macOS/Windows gates and was fast-forward integrated into main
([receipt](docs/milestones/m8-2-integration.md)). Work continues on
`milestone/m8-2-1-complete-session-removal`.

The existing `scoped-cleanup-v1` adapter supports pinned Codex 0.166.0 Darwin arm64,
an exact root-local WAL schema and one unshared Legacy plain rollout. It proves
primary removal and logical reclaim at a fenced cutoff. Its historical
RemovedWithResiduals result does not prove disappearance from Desktop views or
absence after restart. Current product reporting distinguishes those facts;
complete graphical removal has no admitted production profile yet.

AgentMender verifies exact persistent removal and collateral preservation.
The user checks graphical disappearance separately. Automated restart is not a
development gate under [ADR-0021](docs/adr/0021-structural-admission-and-manual-desktop-acceptance.md).
All relevant writers must remain closed throughout a Desktop cleanup. A
process-absence snapshot cannot fence a later launch.

New schema-3 `scoped-cleanup-v2` plans materially expand primary cleanup with exact
target attachment metadata and guardian feedback. Shared attachment payloads and
foreign records survive. Per-action journal progress stops subsequent effects when
recording fails. Historical v2 plans/grants keep their original scope.
Separate schema-4 candidates pin installed build 24141 and recovered build
23880. Both remain unadmitted pending independent structural
verification of all required sources. See [the current checkpoint](docs/milestones/m8-2-1-writer-admission-2026-11-07.md),
[the current contract](docs/adr/0020-closed-desktop-complete-removal.md),
[scoped behavior](docs/scoped-session-cleanup.md) and
[continuation report](docs/milestones/m8-2-1-continuation-2026-11-03.md).

M8.1 native complete cleanup and M9 Claude mutation retain their historical
provider blockers. M3 health meanings, independent terminal approval, durable
intent and exactly eight read-only MCP tools remain intact. M8.3 follows only
after M8.2.1 PASS; M8.3, NLP and M10 have not started.

## Existing discovery and native lifecycle foundation

The following describes discovery and the historical native v1 plan/adapter path.
Its complete native readiness restrictions do not replace the separately proven
M8.2 scoped-local contract above.

Provider-aware inventory, evidence graph, integrity reports and independently
approved native lifecycle orchestration
of AI coding-agent state. M0/M1 and the Codex compatibility refresh retain accurately
reported **partial coverage**. M2 adds graph reconstruction and explanations. M3 adds
a pure integrity evaluator and `verify`. M4 adds immutable lifecycle intentions;
M5 exposes read-only queries and planning over local stdio MCP. M6 adds a durable
AgentMender-owned transaction journal, reconciliation manifests and restart knowledge.
M7 adds independent local approval evidence. M8 adds a consumed-receipt Codex native
unarchive adapter with independent verification and read-only reconciliation.
Archive/delete execution and repair remain unsupported; native v1 real plans remain
Blocked/Unsupported. See [the exact lifecycle boundary](docs/codex-lifecycle.md).

M8.1 independently investigated official stable Codex `0.166.0`. Full cleanup is
**BLOCKED BY PROVIDER**: archive/delete lack complete preapproval effects and
complete postcondition/retention proof. New unarchive cascades, so it inherits no
historical target-only authority. No new production profile was enabled. See the
[full cleanup report](docs/codex-full-cleanup.md).

M9 capability status is **BLOCKED BY PROVIDER**. Its investigation is **CLOSED FOR
CURRENT PROVIDER CONTRACT**; integration retains blocker evidence and permanent
safety guards, not completed lifecycle capability. Claude lifecycle operations
remain **Unsupported**, pending an upstream identity/effects/verification contract.
Separate investigation of current official Code 2.1.314
and delayed-stable 2.1.308 preserves the unsupported installed 2.1.222 profile.
Purge error reporting has improved, but writable dry-run, ambiguous selectors and
incomplete negative proof prevent execution support. Claude remains read-only with
zero lifecycle profiles. See [Claude evidence and guarded limits](docs/claude-lifecycle.md).

```sh
cargo run -- providers --json
cargo run -- scan --provider codex --json
cargo run -- scan --provider claude --json
cargo run -- --provider claude --fixture fixtures/claude/4.29.0-observed --desktop-fixture scan --json
cargo run -- --provider codex --fixture fixtures/codex/observed-snapshot scan --json
cargo run -- --provider codex --fixture fixtures/codex/observed-snapshot-0.165.0-alpha.1.2 graph --json
cargo run -- --provider codex --fixture fixtures/codex/observed-snapshot-0.165.0-alpha.1.2 verify --json
cargo run -- --provider claude --fixture fixtures/claude/4.29.0-observed --desktop-fixture verify
cargo run -- --provider codex --fixture fixtures/codex/observed-snapshot-0.165.0-alpha.1.2 plan archive codex:session:019a0000-0000-7000-8000-000000000001 --json
cargo run -- --provider codex --fixture fixtures/codex/observed-snapshot-0.165.0-alpha.1.2 explain codex:session:019a0000-0000-7000-8000-000000000001 --json
```

Use an ID returned by `scan` with `inspect <id>`. JSON reports go to stdout;
structured errors go to stderr. Exit 0 means a report is complete for its declared
boundary; exit 2 means partial/unavailable inventory or an input/provider error.
For `verify`, exit 0 requires Healthy or HealthyWithExpectedResiduals for every
returned provider, with no failures; Unknown, Degraded and Failed return 2.
Partial inventory never proves absence, ownership, liveness or safe deletion.
Reports include local paths/identifiers but exclude conversation bodies.

## Verified local evidence

| Provider | Installation | Compatibility / evidence | Discovery |
|---|---|---|---|
| Codex | Bundled CLI `0.165.0-alpha.1.2` | Separately validated exact protocol + state schema; SnapshotVerified when source is stable | 166 native summaries in the recorded refresh observation; partial across stores |
| Claude | Desktop `4.29.0`, bundled Code `2.1.222` | Observed Desktop metadata context; RealPartial | Seven Desktop sessions and explicit CLI mappings; other history remains protected |

These counts are observations, not future test expectations. Claude Desktop owns
its Code binary outside PATH; not finding `PATH/claude` never establishes provider
absence. Every installation has its own surface and version provenance. A
candidate binary is not presumed to be the active runtime. Compatibility,
evidence provenance and collection completeness are separate fields.

The historical exact Codex `0.164.0-alpha.11.1` profile is retained. The refresh
binds each executable version to its independently captured source-schema hash;
neither version accepts the other's layout. A later sampling run observed source
WAL/SHM changes and retains RealPartial/SnapshotStale provenance. Compatibility
never establishes current liveness, complete discovery, or authority to mutate.

See [Codex evidence](compatibility/codex/README.md),
[Claude evidence](compatibility/claude/README.md), and [ADRs](docs/adr/README.md).
Observed fixtures are sanitized samples; synthetic fixtures remain labeled
SyntheticOnly. Neither is presented as fresh live evidence.

## Evidence graph and explanations

`graph` reconstructs typed nodes, relationships, rules and protected roots from one
discovery snapshot per provider. `explain <id>` shows the original facts, directed
relationships, rule descriptions, raw and effective confidence, provenance and
protection reasons. JSON includes the full structured provider/root/class identity;
an ambiguous local ID is rejected. These commands retain scan's scope flags,
fixture mode and exit 2 for partial coverage.

Relationships are References, SpawnedBy, MapsTo and AssociatedWith. Cwd grouping
is an inference capped at Heuristic. Claude's explicit CLI mappings and fork
references retain their existing Heuristic strength and unknown historical producer.
Codex snapshot metadata remains Strong. Missing timestamps stay absent. Different
same-identity observations and contradictory immediate-parent claims remain visible
as conflicts. Unknown rules have Unknown effective confidence.

Identity separates providers, logical state roots and entity kinds. Physical file
identity is explicitly unavailable. Paths and one observed reference cannot prove
ownership. Assessments may describe Global, Shared, Referenced or Unknown;
Exclusive, Derived and Orphaned have no current proof rule. Every M2 node remains
protected, including retained active/archived sessions, user-protected projects,
global state and unresolved references. Active presentation does not establish
writer activity. No deletion closure is computed. See ADR-0006/0007.

## Integrity verification

`verify` discovers once per provider, builds the same graph, and evaluates normalized
facts without further provider I/O. Reports distinguish established violations,
suspicious inconsistencies, verification blockers and explicit residual candidates.
JSON retains affected identities, raw/effective evidence, rule descriptors,
supporting observations/relationships, graph diagnostics and proof context. Text
summarizes blocker counts and samples; use `--json` for every fact.

Current live profiles have no closed absence/owner/residual contract. Their normal
status is **Unknown** with visible partial coverage, never a global health claim.
Unattributed transcripts are protected uncertainty. Database/list/file count
differences are not orphan findings. Concurrent activity is an observation blocker,
not corruption. Significant conflicting provider facts may instead be Degraded.

The generic engine verifies scoped synthetic contracts for required targets and
owners; Partial relevant coverage removes negative proof. No finding or residual
classification means removable. See [integrity semantics and support](docs/integrity.md)
and [ADR-0008](docs/adr/0008-read-only-integrity-proofs.md).

## Immutable lifecycle intentions

`plan <intent> <id>` accepts archive, unarchive, soft-delete, purge or repair and
resolves an observed identity inside the selected provider inventory. JSON uses
`mode: planning_only`. Blocked/Unsupported plans return exit 2 with structured
reasons; only Ready returns 0. A plan is not authorization; Ready is not approved.
No operation is executed.

Plans bind exact provider/state/evidence, typed scope, recomputed M3 integrity,
native capability and exhaustive effects assumptions using SHA-256. All native v1
real/sanitized provider requests remain blocked or unsupported because effects,
ownership or integrity are unproven. Claude session purge remains unsupported.
Current physical reclaim estimates are unknown. See [planning semantics](docs/planning.md)
and [ADR-0009](docs/adr/0009-immutable-evidence-bound-planning.md).

## Read-only MCP

Launch `agentmender mcp` as a local stdio server. Configure provider roots and
executables with the existing launch flags. Eight tools expose providers,
workspace_scan, inspect, graph, explain, verify_integrity, plan_lifecycle and
repair_plan. Repair intentions remain Unsupported; no repair executes.

MCP uses the same application layer as CLI. Its typed privacy views replace local
paths/IDs and untrusted prose with opaque process-scoped references while preserving
coverage, compatibility, evidence, integrity and plan blockers. Results are paginated
and bounded. A plan is not authorization; Ready is not approved. No MCP execution,
approval or arbitrary command/filesystem/SQL capability exists.

See [launch instructions, schemas, privacy, pagination and limits](docs/mcp.md)
and [ADR-0010](docs/adr/0010-readonly-mcp-boundary.md).

## Durable transaction knowledge

M6 stores immutable plan commitments, ordered hash-chained records in SQLite and
an independent journal, and typed recovery manifests inside an explicit private
AgentMender root. An ambiguous crash remains OutcomeUnknown; no provider state is
guessed, retried or changed. A transaction is not authorization. Native v1 real plans
remain blocked. The CLI can inspect an existing root read-only:

```sh
agentmender --json transactions --state-root /absolute/agentmender-state list
agentmender --json transactions --state-root /absolute/agentmender-state inspect <transaction-id>
```

Normal discovery/planning/MCP commands create no persistent state. M7's explicit
local preparation workflow creates new attempts; M8 separately executes or
reconciles an independently approved exact transaction.
See [durability, recovery and
limitations](docs/transactions.md) and [ADR-0011](docs/adr/0011-durable-transaction-knowledge.md).

## Independent local approval

M7 keeps approval outside MCP. A separate local terminal workflow prepares an exact
durable review and issues a short-lived one-shot grant bound to the immutable plan,
transaction, state and recovery manifest. Grant/revoke history is authenticated and
append-only. Consumption and durable intent form one event, with no provider effect.
Read-only commands inspect approval status/history. M8's separate local execution
command requires an already independently approved exact transaction.

Existing native v1 provider contracts cannot produce Ready plans, so real approval attempts
remain blocked. The terminal boundary proves local process access, not personal
identity against a same-user shell/PTY adversary. See [commands, trust assumptions,
expiry, migration and handoff](docs/approvals.md) and [ADR-0012](docs/adr/0012-independent-local-approval.md).

## Roots and scope

CLI state roots default to CODEX_HOME / CLAUDE_CONFIG_DIR, otherwise
`$HOME/.codex` / `$HOME/.claude`. `--codex-root`, `--claude-root`, and executable
overrides require absolute local paths. On macOS, default Claude discovery also
checks known app bundles and `$HOME/Library/Application Support/Claude`.
An explicit Claude state root or CLAUDE_CONFIG_DIR disables implicit Desktop
reads. Opt in separately with `--claude-desktop-root <AppSupport/Claude>` and
`--claude-app-bundle <Claude.app>` when using explicit roots.

`--max-entries` bounds enumeration. `--project` matches an absolute observed
provider path, and `--active-only` filters only known archive presentation state.
Archive presentation never implies a quiescent writer. UTF-8 paths serialize as
strings; non-UTF-8 paths use a lossless native encoding object.

## Safety boundary

Discovery, planning, verification and MCP never write real provider state. M8's
separate Codex adapter can invoke only exact-profile unarchive after independent
approval, fresh validation, verified recovery and durable intent. Read-only ports expose only
kind/detect/discover, pure snapshot-to-graph construction and pure integrity/planning-context
interpretation. All artifacts remain
protected, and paths do not prove ownership.
No arbitrary SQL, provider command, native archive/delete/purge/repair execution
or MCP approval tool exists. M8.2 adds only its fixed receipt-gated local cleanup
actions; see the scoped contract above. Codex uses a consistent SQLite Backup API snapshot, scrubs content in RAM
before persisting a private disposable copy, and runs official thread/list there.
Real state remains write-denied. The native process cannot read real-home file
contents, but may stat metadata required by this exact build. No credentials,
configuration, history/log database or rollout contents are copied.

Provider subprocess writes are confined to a fresh AgentMender-owned `0700` evidence
home; its database is `0600`. M6 library persistence writes only to a separately
validated private owned root disjoint from provider state. Child processes are reaped before disposal. The
capture worker verifies kernel source-write denial before SQLite access. Network,
process-fork and Mach lookup are denied. Failure never relaxes permissions.
A narrow macOS C-ABI policy query is isolated and reviewed in ADR-0004; other Rust
code remains forbidden from unsafe operations. Linux/Windows native-process
isolation is unsupported and falls back explicitly to protected metadata audit.

The guarantee concerns application-requested provider content/metadata changes.
OS access times/caches and independent writers are outside it. A SQLite snapshot
is not an atomic capture of all stores, a recovery point or proof of provider
health. Truncated directory samples can vary with enumeration order.

## Development and verification

Rust 1.89+; eight crates retain ports and adapters. Application depends on domain,
provider ports and the narrow persistence handoff; CLI composes adapters and calls application use cases. MCP depends
internally only on application/domain and contains no provider storage logic.
Persistence depends internally only on domain; CLI transaction inspection uses a
read-only application port. Domain transaction/replay logic remains pure.
Only the Codex lifecycle module consumes persistence handoff types; it cannot
access transaction writers or approval keys. See ADR-0013 for the explicit boundary.
Dependencies and the scoped unsafe boundary are documented in
[dependency decisions](docs/dependencies.md).

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
```

Normal tests use isolated fixtures and require no installed provider. Explicit
live fingerprint verification and the older outer-denial probes are documented in
[test instructions](crates/cli/tests/README.md). CI has a Linux/macOS/Windows matrix;
local results do not establish that remote jobs have passed. Graph properties use
proptest with fixed seed `0x4d325f6772617068`; failing cases can also be replayed
through its standard regression persistence. No test uses real provider homes.
