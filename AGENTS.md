# AgentMender engineering constraints

## Current product direction

Read [ADR-0022](docs/adr/0022-product-first-session-lifecycle.md), the
[canonical roadmap](docs/product-roadmap.md), [architecture](docs/architecture.md)
and [audit](docs/audits/2026-11-07-product-first-audit.md) before changing scope.
The product is MCP-native conversational session lifecycle management, initially
for Codex and Claude Code. The calling agent interprets language; AgentMender
provides structured reliable operations. Provider identity is independent of the
host agent. Codex→Claude and Claude→Codex remain target scenarios.

Priority order: P0 conversational single-session workflow; P1 native operations
and provider coverage; P2 additional artifact cleanup; P3 project/bulk lifecycle.
First prove one complete vertical removal path, then both initial providers.
M8.3 multi-session cleanup is not the next implementation priority.

M8.2 primary cleanup remains historical PASS at main
`a284767157ace8769deee85b195b5b70efe374d8`. M8.2.1 is **unfinished/deferred**, not
PASS and **no longer a global blocker for the conversational MVP**. Preserve its
branch `milestone/m8-2-1-complete-session-removal` and historical evidence.
Schema-4 builds 24141/23880 remain separate unadmitted candidates with an empty
compiled admission list. Never enable unsafe Desktop global JSON publication;
preserve the child-process lost-update regression and zero-operation refusal.

M8.1 native full cleanup and M9 project-purge blockers retain their investigated
version/operation scope. They do not prove all Codex or Claude session management
impossible. Current Claude has zero mutation profiles. Do not transfer historical
Codex unarchive or stable primary-cleanup authority to a newer binary/Desktop.

## This audit's implementation boundary

The authorized cycle is reconstruction, audit, small proven simplification,
documentation and verification on `refactor/product-first-roadmap`. Stop with a
readiness assessment before beginning new features. Do not implement new MCP
execution tools, native dispatch, Claude deletion, resolution/search services,
bulk cleanup, artifact families or Desktop mutation in this cycle. No real-user
session deletion, provider global-state/index/history edits or app restart.

Current MCP has exactly eight read-only tools. It cannot prepare durable plans,
grant/revoke/consume approval, execute, access signing keys or import proofs.
Future P0 work may propose a reviewed execution handoff; this roadmap grants no
current production authority. Preserve existing MCP schemas/privacy until that
work defines safe human-readable labels and exact result semantics.

## Architecture

Keep the eight crates. Domain is pure and owns shared approval eligibility.
Provider-api has read-only ports and protected observation helpers. Provider
interpretation belongs in adapters. Application obtains fresh trusted inputs and
owns lifecycle orchestration. Persistence owns private authority construction,
journal/recovery and authenticated history; it never calls a provider. CLI composes
concrete adapters and the independent local human broker. MCP is a thin inbound
interface over application/domain, never an authorization authority.

Keep native, primary cleanup, complete Desktop assessment and durable transaction
results semantically distinct. Consolidate only genuine duplication; do not merge
security or serialization boundaries. Do not add crates or generic mutation ports
for appearance. Advanced private candidate modules may remain when they provide
reusable evidence without adding admission to the active path.

Prefer provider-native operations only when exact-version effects and supported
postconditions satisfy the user's approved target set. Native cascade/startup
side effects count. Otherwise evaluate a reviewed local fallback before approval;
never fallback after an unknown native outcome. Session requests never widen to
project purge. Additional cleanup is optional and separately scoped, using the
same approval/transaction engine. Preserve M3 health/negative-proof meanings.

## Mandatory safety invariants

- Verify exact provider/root/native identity, aliases, relevant dependencies,
  physical identity, compatibility, schema and writer boundary. Unknown versions
  remain audit-only. Presence, evidence quality, compatibility and coverage differ.
- Observation is not inference. Project paths, cwd, time proximity and filenames
  never establish destructive ownership. Preserve conflicting observations and
  partial/unknown evidence. Graph/planning transformations are deterministic and
  perform no I/O or clock sampling. Integrity findings grant no deletion authority.
- Freeze exact required/optional/retained effects. Actual effects must be a subset
  of approved effects. New units or changed scope require a fresh plan. Preserve
  every unrelated session and shared/referenced/global/ambiguous/unknown unit.
- Use existing immutable planning, fresh validation, independent human review,
  protected persistence, recovery and one-shot consumed DurableIntent. A model
  cannot approve its own destructive request; no boolean, --yes/force, noninteractive
  grant, imported proof or caller-chosen principal can bypass the human boundary.
  Terminal possession alone does not resist a same-user shell/PTY adversary.
- Journal and sync intent before every effect, including provider startup and lock
  creation. Record observations after effects. Missing outcomes stay unknown.
  RecoveryRequired/OutcomeUnknown never automatically retry, compensate, restore
  an entire provider store or reconstruct dispatch authority. Reconciliation reads
  only the frozen manifest. Recovery metadata does not back up deleted payloads.
- Preserve old plan/hash/approval/journal/replay semantics. V1 history remains
  inspection-only; v2/v3/v4 manifests never inherit expanded authority. Do not
  delete crash regressions or valid destructive property tests to obtain green CI.
- Native return success/list absence is not transcript-erasure proof. Report the
  verified operation, remaining artifacts, graphical status and logical bytes
  separately. Never claim physical reclaim or secure erasure without evidence.
- Protect the active management session before any future P0 destructive admission.
  Existing Active state/writer detection does not implement caller-session binding.

## Provider isolation and Desktop research

Provider processes require OS-enforced real-state write denial; no unsandboxed
retry. Temporary effects need an opaque owned evidence sandbox (ADR-0003).
Source SQLite is opened read-only only inside the kernel-attested worker
(ADR-0004). Disposable mirror SQL is not repair/recovery authority.

Workspace unsafe_code is forbidden except the existing explicitly reviewed macOS
C-ABI query module in provider-api. Do not expand that exception without an ADR
and safety review. No general SQL, file-write, command or app-control interface.

Read ADR-0017 for scoped primary fences and ADR-0020/0021 plus the projection and
artifact inventories before any future Desktop work. Complete Desktop structural
claims still require independently verified persistent reconstruction sources,
collateral preservation and admitted exact profiles. The user verifies graphical
disappearance separately. Production never starts/stops/kills/reloads/restarts
Codex for visual verification. Closing writers is a prerequisite, not a lifetime
lock or CAS. A process census plus compare/rename cannot close the demonstrated
global Map writer race. Keep global target publication refused.

## Development and release

Use sanitized/disposable fixtures. Real probes are explicit and separate from CI;
never test destruction on user state. Run relevant tests after each meaningful
refactor, then the full locked gates:

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
python3 scripts/check_architecture.py
python3 scripts/check_codex_lifecycle_evidence.py
python3 scripts/check_complete_removal_evidence.py
git diff --check
```

Verify branch/remote/main baselines; preserve unfinished milestone branches.
Use coherent commits and normal pushes. Verify the exact final commit on all
three GitHub Actions platforms. Do not force push, rewrite history, merge to main
without instruction, or substitute an earlier green run for final CI. Historical
milestone reports, compatibility evidence and ADRs retain their original verdicts;
new decisions supersede them explicitly. The current roadmap controls sequencing.
