# Development and verification

## Real-session versus regression acceptance

Normal onboarding starts with the existing installation and
`agentmender --provider codex mcp --session-search`; see the
[quick start](p0-c1-manual-acceptance.md). The standalone synthetic onboarding
generator was retired during consolidation: no CI/test consumer, and its search
invariants are exercised by application and disposable stdio tests. Its original
bytes remain in commit `d58311b64580bd4213bf928d64888e563aa0755f` and archival refs.
Preserve all deterministic CI/security fixtures. The normal macOS stdio test proves readable unknown binaries, blocked preview,
preparation refusal, no owned-state creation and unchanged fixture bytes. Real
metadata checks are separate from private-data-free CI and from actual model use,
human approval, real deletion and optional GUI confirmation.


## P0-C1 verification

The [conversational primary handoff](conversational-codex-removal.md) is implemented
with destructive acceptance still OPEN. Real conversational metadata search is
accepted; current [intelligent content search](session-search.md) is implemented
with updated client and real caller-binding acceptance still open. Normal tests retain all prior safety regressions.
The explicit `conversational_removal` CLI integration test uses disposable exact
provider state and real stdio/worker/CLI effects; its grant is fixture authority.
Do not count it as an actual Codex conversation or independent human acceptance.
The final report records those gates separately. P0-S1 adds bounded content
fixture/stdio/isolation tests and 5,000-session paging. The current
[search quick start](session-search.md) defines advanced requests, refusal and the
failure behavior. Production file-only declarations are not caller proof. Do not
infer binding from model UUID selection or treat a protocol driver as conversational use. No test deletes real sessions.


Start with [AGENTS.md](../AGENTS.md), [architecture](architecture.md),
[product roadmap](product-roadmap.md) and the [audit](audits/2026-11-07-product-first-audit.md).
M8.2.1 and historical M9 reports remain evidence; they do not set the next product
milestone. Both initial providers are required, with one supported vertical slice
delivered first. The audit baseline stopped before features; the current P0-C1 work follows ADR-0023.

Use Rust >=1.89 and Cargo.lock. The eight-crate workspace has no build.rs or
application frontend. Python scripts use the standard library. CI's bundled SQLite
and provider fixtures do not require installed Codex/Claude. Current live process
isolation is macOS-specific; three-OS fixture PASS does not admit mutation on all
platforms.

Run relevant targeted checks after a refactor, followed by:

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

Important reusable regression entry points:

```sh
cargo test --locked -p agentmender-domain --test scoped_cleanup historical_v2_plan_binding_and_journal_retain_their_exact_encoding
cargo test --locked -p agentmender-provider-codex --lib late_writer_counterexample_requires_global_publication_refusal
cargo test --locked -p agentmender-provider-claude --test lifecycle_guards
cargo test --locked -p agentmender-persistence
cargo test --locked -p agentmender-mcp-server --test protocol
```

These are subsets of the workspace gate; rerun separately only for a meaningful
change or diagnostic need. Preserve proptest seeds and stored crash regressions.
Never remove a valid failing safety test to pass CI.

`check_architecture.py` executes its source-fitness canaries, inspects all production
modules including code after test modules, and checks Cargo metadata. It permits
only the reviewed authority and mutation boundaries. The two evidence validators
check committed pins, schemas, inventories and counterexamples without network,
provider startup or live state access. Evidence probes (`probe_*`, live discovery,
capture scripts and ignored installed-binary tests) are separate explicit opt-ins.
Read their headers and contracts before use. This audit uses disposable fixtures
and static validators; it does not authorize provider-state writes or restarts.

Before a removal, inspect references, public exports, cfg/platform branches,
features, tests, historical wire/replay consumers and provider contracts. A private
unadmitted advanced module is not automatically dead. Avoid a new generic executor,
crate or outcome model where the existing boundary is sufficient.

Work from a verified `main` baseline, commit coherently and push normally. Preserve
historical tips through the [archival manifest](audits/2026-11-08-original-refs.json)
and verified external bundle; do not recreate long-lived milestone branches.
Check GitHub Actions against the
**final exact SHA**, including Linux/macOS/Windows jobs; an earlier green commit
does not satisfy this gate. No whole-history squash, history rewrite or force
push. Keep historical receipts immutable and record current status in the
canonical roadmap/audit. Final verification receipts can be retained externally
to bind the final commit without a self-referential commit hash.

## Intelligent search development

Read [ADR-0027](adr/0027-intelligent-session-retrieval.md) and the
[current search/index contracts](session-search.md). New deterministic tests cover
query/DST/unknown logic, streaming progress and middle messages, chunk/context
navigation, source mutation, private cache/rebuild and missing host binding.
The [benchmark receipt](research/intelligent-search-benchmark.md) includes exact
reproduction and limitations. Run the unchanged locked workspace gates plus:

```sh
cargo test --locked -p agentmender-domain --test session_query
cargo test --locked -p agentmender-application --test indexed_search --test context_navigation
cargo test --locked -p agentmender-provider-codex content::tests
cargo test --locked -p agentmender --test search_index
cargo run --locked -p agentmender-persistence --example search_benchmark
```

On disk-constrained local machines use `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0` and `CARGO_INCREMENTAL=0`; these reduce generated
artifacts without changing checked-in profiles or weakening CI. Live read probes
are explicit and aggregate-only, separate from portable sanitized tests.

CI also runs `python3 scripts/test_semantic_assessment.py` for the versioned
model-output contract and exact candidate/evidence allowlist.
