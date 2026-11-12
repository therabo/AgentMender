# Autonomous MCP session finding

## Baseline and implementation

The baseline checkout and public `main` were
`d024ce521fb0b9f6ea991a8fca4e690f8749711d`. The checkout was clean and the
macOS baseline workspace suite passed 581 tests with eight ignored.
Implementation is on the local branch `feature/autonomous-session-find`.
[ADR-0037](../adr/0037-autonomous-mcp-session-find.md) records the decision.

| Area | Changes |
| --- | --- |
| MCP catalog and transport | `crates/mcp-server/src/{find,protocol,conversation,lib}.rs` |
| Job registry and process lifetime | `crates/cli/src/session_find.rs`, `crates/cli/src/session_find/host.rs`, CLI composition and Cargo manifest |
| Typed resolver and caller validation | `crates/application/src/{conversation,session_resolution}.rs`, `crates/cli/src/conversation.rs` |
| Fixed launch and embedded assets | `crates/provider-api/src/process.rs`, `scripts/resolve_with_luna.py`, existing SearchIntent schema and role profiles |
| Guards and verification | `scripts/check_architecture.py`, `scripts/test_autonomous_session_find.py`, CI matrix |
| User contract | README, AGENTS.md, AgentMender lookup skill, MCP and search docs, architecture, roadmap and ADRs |

`session_find` accepts only the natural query. `session_find_status` reads
progress, `session_find_result` waits up to 60 seconds and pages verified rows,
and `session_find_cancel` cancels the job. Progress continues between calls.
The same MCP signer and typed resolver serve diagnostic and autonomous paths.
The private stdio bridge carries bounded messages and uses no nested MCP
connection. Source data is still read by the confined Codex worker.

The Rust host embeds the existing Python workflow and owns the coordinator
and App Server handles. It uses separate private Codex homes for the two
roles, a fixed verified executable, pinned GPT-6 Luna Medium, and no model
fallback. The Planner exits before cohort retrieval. The Evaluator is started
only when retrieval produced candidates and exits before publication. Scoped
turn events detect reroutes, unexpected tools and extra threads. Job expiry,
limits, caller metadata and connection lifetime are documented in the
[workflow](../intent-aware-resolution.md).

## Observed Codex runs

- **Desktop, external workspace, Terra High.** The exact request “Trova tutte
  le sessioni in cui ho parlato di carte.” made one `session_find` call and
  status and result calls. The originating turn did not spawn a subagent.
  AgentMender verified two distinct Luna Medium threads with one Planner
  turn and 19 Evaluator turns. Rust inspected 543 of 543 snapshot sources
  and evaluated 73 of 73 selected candidates. Eight confirmed rows were
  signed and published. The 65 unconfirmed judgments produced zero
  published plausible alternatives. The client rendered two tables.
  The terminal status was `interrupted` with `source_unavailable` after
  publication. The worker reported 542 partial sources, one unavailable
  source and one freshness failure. Candidate evaluation was complete.
  The same acceptance also exposed that the client chooses the localized
  headers. The empty group was rendered with two columns.
- **First Desktop attempt.** The machine logged clamshell sleep during
  evaluation for about seven minutes. The pending Evaluator turn later
  timed out at 40 of 135 candidates. This attempt established discovery and
  lifecycle, and the subsequent awake run established publication.
- **MISSION1 and GTT Desktop.** Narrow absent-term requests completed via the
  new tool with Terra High and Luna Medium origin models respectively. Both
  started one Planner and no Evaluator. The GTT recheck used the build with
  bounded waiting and used two AgentMender calls. A prior GTT turn had
  stopped polling while its job was still retrieving.
- **Codex CLI and reconnection.** A disposable GPT-6.1 Sol chat started in the
  AgentMender checkout, then resumed with the same chat UUID and a newly
  started MCP connection after replacing the old executable. The resumed
  chat invoked `session_find` and received a completed result. This is a
  CLI test. The existing work chat still advertised its earlier Desktop catalog.
  A fresh connection is required to get the current catalog. Existing user chats were left intact.
- **Standalone MCP, real App Server.** The exact historical 539-source
  fixture produced the same 43 confirmed UUIDs as the controlled host
  result, with zero duplicates and zero foreign UUIDs. Both runtime
  roles resolved to GPT-6 Luna Medium. The Planner ran once and the
  Evaluator handled nine turns on one thread.

## Performance

Single runs on the same 539-source fixture with real App Server inference.

| Path | Time | Planner | Retrieval | Evaluator | Publication | Confirmed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Prior controlled host | 82.05 s | 5.15 s | 0.20 s | 76.63 s | 0.07 s | 43 |
| MCP-native | 83.16 s | 5.60 s | 0.31 s | 76.76 s | 0.28 s | 43 |

The controlled host issued 28 resolver MCP calls to its separate server.
The native run made 28 internal resolver operations and the acceptance
client sent 24 external job calls, including polling and result pagination.
The final bounded-wait fixture tests observed two external calls to terminal
before additional result pages. These are different measurements. In the
real native fixture, observed token counts were 12,446 input and 58 output
for the Planner and 154,035 input and 3,318 output for the Evaluator, including
4,864 and 138,240 cached input tokens respectively.

Synthetic model responses isolate Rust and bridge overhead. Each row evaluated
43 candidates and checked the exact UUID set and a signed context reference.
RSS samples cover the MCP process and the largest direct child separately.

| Sources | Total time | Rust retrieval | MCP RSS | Child RSS | CPU user/system | Peak cohort spool |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 105 | 0.71 s | 0.03 s | 20.4 MiB | 20.7 MiB | 0.52/0.31 s | 232 KiB |
| 539 | 0.79 s | 0.11 s | 20.5 MiB | 20.8 MiB | 0.57/0.33 s | 364 KiB |
| 1,000 | 0.92 s | 0.21 s | 20.3 MiB | 20.8 MiB | 0.65/0.37 s | 480 KiB |
| 10,000 | 3.90 s | 2.91 s | 17.6 MiB | 22.1 MiB | 2.24/1.63 s | 2.98 MiB |

The native result spool reached 204 KiB in the real 539-source run and
in these fixture runs. Corpus traversal and candidate classification
remain distinct from logical history completeness.

## Safety and limits

The fixture acceptance covers concurrent callers, cross-scope refusal,
duplicate submissions, protected UUIDs, model or effort mismatch, reroute,
quota, unexpected tools, unavailable executable, cancellation, MCP EOF and
child cleanup. It also checks modified, missing and out-of-root sources and
the signed context path. The existing lifecycle and provider guards remain
the separate authority boundary.

Caller metadata is forgeable and carries only apparent read-only scope.
Job IDs are randomly generated and process-local. The account-backed
App Server path requires macOS process isolation, the system Python and
private file credentials. Linux and Windows can compile the runtime
but fail closed for this launcher. Full lexical traversal cannot prove
semantic absence when a source is missing or a transcript is partial.

Real user history, job receipts and the Desktop result tables are excluded
from Git. The only installation change was replacing the configured
AgentMender executable. The app was not restarted.

## Platform verification

- **macOS arm64.** The locked workspace suite passed 581 Rust tests with
  eight ignored. Clippy with warnings denied, build, rustfmt, architecture,
  lifecycle and complete-removal guards passed. The Luna host suite, the
  semantic validator and six semantic regressions passed. The new MCP
  acceptance suite passed with real Rust retrieval and disposable App
  Server responses. Real Codex runs are recorded separately above.
- **Linux arm64.** Clippy, the locked build, architecture, lifecycle and semantic guards
  passed in a Debian Bookworm container with Rust 1.97. The workspace suite encountered 15 existing
  provider-Codex failures. The same 15 test names failed on the archived
  baseline in the same container with tmpfs. A non-root provider rerun
  passed 155 tests, with three ignored and the existing
  `same_length_stage_edit_cannot_replace_destination` failure remaining.
- **Windows x86-64.** `cargo check --locked --workspace --all-targets`
  passed with the `x86_64-pc-windows-gnu` target and MinGW. This verifies
  compilation from Linux. Native Windows execution was unavailable.
- **GitHub Actions.** The candidate was tested locally. The historical
  baseline's [GitHub run](https://github.com/therabo/AgentMender/actions/runs/43256264930)
  is separate from this branch. The branch is local and `main` remains
  at the baseline.
