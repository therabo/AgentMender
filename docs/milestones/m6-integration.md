# M6 formal integration evidence

Verified on 2026-10-29 before M7 implementation.

| Item | Evidence |
|---|---|
| Previous main | `c426a7fe4e12c24afc2f383087912e7f54111b39` |
| M6 branch | `milestone/m6-durable-transactions` |
| Original expected local/remote HEAD | `6508a81d3b9d11426ed219406fa1bb7dc9500b71` |
| Original exact run | [40868667746](https://github.com/therabo/AgentMender/actions/runs/40868667746), Linux/macOS/Windows successful |
| Corrected final local/remote HEAD | `f2268d2cdcff961a91e079af79314571fb3752d7` |
| Corrected exact run | [40891496279](https://github.com/therabo/AgentMender/actions/runs/40891496279), successful, all three matrix jobs completed |
| Integration | Fast-forward merge, ordinary push, no history rewrite |
| Verified local/remote main and exact M7 base | `f2268d2cdcff961a91e079af79314571fb3752d7` |

All six required checks passed on the original head, corrected head and pushed
main: fmt, locked workspace Clippy with warnings denied, locked tests/build,
architecture fitness and whitespace checks. Local totals: 251 passed, zero failed,
two explicit live probes ignored. Working tree clean at the integration gates.

Three independent Sol/high reviews covered all domain modules/properties,
provider/isolation/MCP/application/CLI, and persistence/recovery/crash tests.
Review found that a regex silently skipped the nested TransactionEvent declaration
in the raw-payload fitness check. The follow-up commit replaces it with balanced
declaration parsing, fails on missing declarations and adds nested mutation
canaries. Independent review rechecked the correction. No Rust behavior, provider,
MCP, fixture, assertion or safety requirement was weakened.

GitHub connector reads returned 404 and gh was unavailable. Authenticated Safari
showed the exact original commit and three successful individual platform jobs;
the corrected run showed its full commit link and successful three-job conclusion.
SSH ls-remote independently verified both branches and pushed main. Historical
reports were not substituted for live Git/CI evidence.

Plans remained immutable, persistence/receipts were not authorization, required
recovery preceded durable intent, OutcomeUnknown never implied success/failure,
and no provider state was guessed or mutated. MCP retained its eight read-only
tools. Only after this gate was milestone/m7-independent-approval created from
the exact main and pushed with tracking before implementation.
