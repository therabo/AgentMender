# Discovery test boundaries

Normal cargo test runs use sanitized/synthetic fixtures or fresh test-owned
locations. Before/after manifests compare paths and bytes. They require no local
provider and do not scan real homes or app bundles implicitly.

The same byte manifests cover graph/explain as well as scan/inspect. Graph tests
exercise sanitized observed Codex and Claude fixtures, unknown versions, partial
coverage and raw/effective rule provenance. Domain properties use 256 generated
cases per property and fixed seed `0x4d325f6772617068`; run them independently with
`cargo test --locked -p agentmender-domain --test graph_properties`. They cover
positive Shared/Referenced/Global assessments, uncertainty, conflicts, roots,
canonical ordering and lossless provenance/path serialization. They never access
real provider homes.

Two older ignored Rust probes wrap the entire CLI in independent global write
and network denial. They test safe fallback, not the nominal disposable-mirror
path: nested/global sandbox restrictions can prevent provider initialization.
They are not used to claim M1 discovery support.

Production-path live verification is explicit and separate:

```sh
python3 scripts/verify_live_discovery.py --allow-real --provider codex \
  --binary /absolute/agentmender --state-root /absolute/codex-home \
  --executable /absolute/codex
python3 scripts/verify_live_discovery.py --allow-real --provider claude \
  --binary /absolute/agentmender --state-root /absolute/claude-state \
  --desktop-root /absolute/AppSupport/Claude --app-bundle /absolute/Claude.app
```

The script runs providers/scan/inspect, emits only aggregate report information,
and compares DB/WAL/SHM fingerprints for Codex, or bounded Desktop/project-tree
metadata plus parsed-file hashes for Claude. Access time is excluded. Matching
fingerprints cover those observed files and intervals, not all OS/provider state.
Independent concurrent changes must not be attributed automatically to AgentMender.
Provider subprocesses retain the production isolation boundary throughout.
