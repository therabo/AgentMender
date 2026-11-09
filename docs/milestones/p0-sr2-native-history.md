# AgentMender — P0-SR2 Native History Retrieval & Coverage

Investigation/delivery2026-11-09–10 on `main`, starting at
`4cc64a9cab5b7c16191c44ba883ae8f0a596cf9b`. Exact final SHA and private real
receipts are in the external SR2 report; no private conversation is committed.

## Result

The exact330 identities now yield325 partially readable sessions, versus16
previously. **309 previously unavailable histories are recovered.** The remaining
five compressed files exceed the explicit64MiB physical-size bound, rather than
being missing. All330 identities finalized; no live-inventory denominator is
substituted. The separate91-session MISSION1 query yields88 Partial sources and
three size refusals;78 positive topic matches and13 UNKNOWN candidates. Readability
and query relevance are different counts. No complete-history/negative proof follows.

The cohort has16 plain-only and314 compressed-only representations, no both-path
files. Native host RPC was corroborated for the four targets, not all330; its
remaining326-session availability is unmeasured. All four are readable through
both the host and AgentMender's compressed path. No native-only count is inferred.

| Target | Own MCP retrieval and context |
|---|---|
| A, Cesium/FPS | PASS,3 search pages; real visible IDs, surrounding/forward messages and two later FPS occurrences |
| B, Earth camera/workload | PASS,2 search pages; forward navigation reaches supported EOF |
| C, telemetry/PostgreSQL | PASS,2 search pages; surrounding and subsequent messages; host older-turn paging separately demonstrated |
| D, database/bootstrap | PASS,2 search pages; neighboring message and long final-message chunks at0/3764/7860, ending at supported EOF |

All are Partial. Twenty-three actual context chunks match native message/turn
identity and visible text after the existing control-character projection. Target
physical revisions agree across direct and actual-chat reads. Native corroboration
does not supply AgentMender ingestion authority.

## Source and implementation

Expected plain locators are stale representations: exact `.jsonl.zst` counterparts
exist. Local projected history also exists. [Source investigation](../native-history.md)
and [ADR-0029](../adr/0029-compressed-rollout-history.md) distinguish observed local
bytes, host paging and uninstrumented host backend selection. The exact installed
desktop is26.1103.41825/build25679, CLI0.167.0-alpha.3.2.

Add a bounded read/seek decoder inside provider-codex's existing attested worker;
retain the same port/application/MCP/FTS5/cache flow. Fixed locator counterparts,
header identity, unshared no-follow opens, revalidation, privacy suppression and
UNKNOWN handling are preserved. Compressed provenance tags and decoded offsets
are explicit. No provider materialization, history SQL reader, host import, new
database or lifecycle authority is added. `zstd`0.13.3 is locked without optional
default features. Signed cursor input now accepts32KiB so real emitted checkpoints
can round-trip; process/kind authentication and frame/output bounds remain.

Fresh owned-state experiments prove this installed App Server creates SQLite and
other startup files, and exits under provider-root write denial. Its default backend
also rejects item pagination. No live startup/credential borrowing is admitted.
Official TUI Markdown export is evaluated, not invented or implemented as trusted
input. Native host text cannot grant caller/deletion/negative-proof authority.

## Verification and limits

Fixture tests cover both compressed message formats, decoded-offset continuation,
chunks/occurrences/context, corruption/truncation, expansion/window/file/deadline
bounds, identity/links/ambiguity and physical replacement. Existing source/privacy,
Boolean/date/UNKNOWN, query-scoped paging, FTS5/cache, lifecycle/approval/recovery
and Luna regressions remain in the workspace suite. Local formatting, Clippy,
locked workspace tests/build, architecture and compatibility checks pass; the
external report binds exact logs and final status. A local disk-full interruption
was resolved by clearing regenerable build artifacts; successful reruns are retained.

The real fixed cohort used152 calls in92.70s, median541.88ms, no query errors.
MISSION1 topic retrieval used99 calls in56.16s, median518.09ms. The original context
regression still terminates in20 pages,3.91s total. Calendar dates return31 matches;
Rome DST's18 match the explicit25-hour UTC interval. Byte counters exclude decoder
replay and metadata work; no production SLO or all-platform live admission follows.

Actual chat tools recover all four targets through newly built workers. The chat
still reports the previous MCP registration after a user-reported refresh. Its
exact parent revision remains unverified; new parent projection and larger cursor
acceptance are proven by fresh direct stdio/fixtures only. Actual search disabled
excerpts to stay within the retained parent's token limit, then fetched real text
through context. A disk-full metadata call failed before recovery; target reads
and navigation afterward pass. Management returns `caller_identity_required` and
zero candidates. Luna configuration/routing is preserved without a new agent run.
The final direct replay also retained one C context `stale_cursor` refusal under
the existing conservative freshness policy; a fresh query/context traversal passes.
No freshness gate was relaxed and the refusal is preserved in the private receipts.

Production supported: bounded macOS read-only plain/compressed local discovery
within the declared formats. Production management, full-parent conversational
acceptance, general history hydration and non-macOS live isolation remain OPEN.
Prior exact CI run79 was blocked before all three OS jobs by billing/spending
limits. Final-commit remote status is recorded after normal push in the external
report; local gates are not remote PASS. No billing settings/reruns are changed.

One auxiliary SR1 `target-diagnostics.json` was accidentally overwritten by a
reused driver's fixed output path. Its replacement is copied into SR2 and excluded
from baseline analysis. Canonical SR1 cohort/actual-client receipts and the original
330-identity acceptance remain intact. The private report records this evidence
handling exception; no reconstructed original is presented as authentic.

Next: establish a refreshed parent in the real chat, restore final three-OS CI,
then investigate bounded access for the five oversized sources without raising
limits blindly. Caller protection remains mandatory before deletion development.
