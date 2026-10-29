# Discovery test boundaries

M8 adds sixteen generated verification properties at 256 cases each, seed
`0x4d385f6c69666531` (4,096 cases), focused execution/reconciliation/retained-state
regressions and compile-fail one-shot gate checks. Reopen tests cover intent,
response-before-observation, observed-before-verification and Verifying cuts;
existing persistence child-abort tests cover durable write boundaries. Generated
properties and fake transport tests are separate from isolated real-binary evidence.
No normal test requires Codex or may use a real provider home. The ignored macOS
native lifecycle test accepts only an executable and an explicit opt-in; provider
and transaction roots are always newly generated disposable directories.

M7 adds 21 generated approval properties at 256 cases each (5,376 cases), seed
`0x4d375f6170707276`, plus strict serialization regression. Run
`cargo test --locked -p agentmender-domain --test approval_properties`.
They cover exact scope/binding, eligibility, expiry, terminal revocation/consumption,
protocol origin and independent binding-field changes. They do not authenticate
MACs; persistence separately tests 30 signed-field forgeries with recomputed public
hashes and matching copies, key loss/substitution, reopen, 14 key/schema crash cuts,
three competing-process authority races and grant/revoke/consume append crashes.
CLI tests cover non-terminal/JSON rejection, escaped bounded review, blocked
preparation without root creation and read-only exact-plan inspection. MCP protocol
regressions reject six approval/execution names and three boolean confirmation
fields before dispatch. Production MCP/provider code remains read-only.
See [approval semantics](../../../docs/approvals.md). M7 performs no provider effect.

M6 transaction tests use only private temporary roots and synthetic Ready plans.
`cargo test --locked -p agentmender-domain --test transaction_properties` runs nine
properties with 256 cases each and seed `0x4d365f74786e7331`, plus four regressions.
`cargo test --locked -p agentmender-persistence` includes schema/reopen/recovery,
cross-process writer exclusion, deterministic fault injection and child-process
abort at journal/SQLite/recovery/effect boundaries. Aborted children are expected
test fixtures; the parent assertions must pass. CLI `--test transactions` checks
byte-identical read-only inspection and protected unavailable recovery. No test
dispatches real provider lifecycle operations. See docs/transactions.md.

Normal cargo test runs use sanitized/synthetic fixtures or fresh test-owned
locations. Before/after manifests compare paths and bytes. They require no local
provider and do not scan real homes or app bundles implicitly.

M5 protocol tests run with `cargo test --locked -p agentmender-mcp-server`.
CLI stdio fixture sessions run with `cargo test --locked -p agentmender --test mcp`.
They cover known/unknown/partial Codex, synthetic/observed Claude, scan/graph,
opaque-target inspect/explain/planning, Unknown integrity, unsupported intentions,
malformed arguments, provider mismatch, private-path exclusion, protocol-only
stdout and complete recursive fixture byte manifests. No installed provider is
needed. In-memory protocol tests cover modern and legacy negotiation, duplicate
and oversized input, strict schemas, busy/cancel/deadline/EOF cleanup and safe
JSON text fallback. See docs/mcp.md for the public contract and limits.

The same byte manifests cover verify (JSON and text), graph/explain and scan/inspect. Graph tests
exercise sanitized observed Codex and Claude fixtures, unknown versions, partial
coverage and raw/effective rule provenance. Domain properties use 256 generated
cases per property and fixed seed `0x4d325f6772617068`; run them independently with
`cargo test --locked -p agentmender-domain --test graph_properties`. They cover
positive Shared/Referenced/Global assessments, uncertainty, conflicts, roots,
canonical ordering and lossless provenance/path serialization. They never access
real provider homes.

M3 integrity properties have fixed seed `0x4d335f696e746567` and 256 cases per
generated property. Run `cargo test --locked -p agentmender-domain --test integrity_properties`.
Synthetic closed contracts include positive absence/orphan witnesses and weaken
each proof prerequisite; existing provider fixtures are never promoted to complete
ownership/absence contracts. See docs/integrity.md for supported conclusions.

Two older ignored Rust probes wrap the entire CLI in independent global write
and network denial. They test safe fallback, not the nominal disposable-mirror
path: nested/global sandbox restrictions can prevent provider initialization.
They are not used to claim M1 discovery support.

M4 planning properties use 256 cases each and fixed seed `0x4d345f706c616e73`,
with a stored capability-conflict regression. Run
`cargo test --locked -p agentmender-domain --test planning_properties`.
Ready tests use explicit synthetic native effects/identity/integrity contracts;
no current provider fixture is promoted to complete or exclusive. Byte manifests
cover planning text/JSON, including duplicate and exact V165 Codex fixtures.

Production-path live verification is explicit and separate:

```sh
python3 scripts/verify_live_discovery.py --allow-real --provider codex \
  --binary /absolute/agentmender --state-root /absolute/codex-home \
  --executable /absolute/codex
python3 scripts/verify_live_discovery.py --allow-real --provider claude \
  --binary /absolute/agentmender --state-root /absolute/claude-state \
  --desktop-root /absolute/AppSupport/Claude --app-bundle /absolute/Claude.app
```

The script runs providers/scan/inspect/verify, emits only aggregate report information,
and compares DB/WAL/SHM fingerprints for Codex, or bounded Desktop/project-tree
metadata plus parsed-file hashes for Claude. Access time is excluded. Matching
fingerprints cover those observed files and intervals, not all OS/provider state.
Independent concurrent changes must not be attributed automatically to AgentMender.
Provider subprocesses retain the production isolation boundary throughout.
