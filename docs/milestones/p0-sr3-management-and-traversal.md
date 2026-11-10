# AgentMender — P0-SR3 Management Identity & Complete History Traversal

Delivery date2026-11-10; starting `main`
`12588610e5867fc821167397580685fadd166fee`. Exact final local SHA, executable hash,
private target identities/call receipts and verification logs are recorded in the
external delivery report. Implementation commit:
`9f928bf9cef295c91d1e6b74c6fc06952664f4ea`. This public report contains no private transcript text.

## Verdict and proof levels

Physical traversal is implemented and real-provider-tested. **Full milestone
acceptance remains OPEN** because authenticated host caller binding is unavailable.
Management fails closed; no self-exclusion PASS is inferred from fixture tests.
The retained conversational parent was not refreshed because the user could not
restart that connection. Its compatible small-source reads remain useful, but
updated-parent resident/default-management actual-client acceptance is OPEN.

| Capability | Implemented / unit | Fixture | Real provider | Actual client | Production boundary |
|---|---|---|---|---|---|
| Total compressed-size rejection removed | Yes / PASS | >64MiB valid source | Five targets EOF | 67.5MB target search/content PASS | Bounded supported macOS reads |
| Resident decoding and replay | Yes / PASS | >128MiB, work exhaustion, eviction, no duplicates | Five sequential navigations and cold probes PASS | Retained parent; full resident refresh OPEN | Transient connection state, declared limits |
| Start/backward/forward/occurrence/turn navigation | Yes / PASS | Large hidden gaps, chunks, source drift | Five middle/later probes, EOF and navigation PASS | Four MISSION1 targets, final chunks/continuations PASS | Supported records; oversized/unsupported gaps explicit |
| Physical/logical coverage split | Yes / PASS | Prefix/EOF/unsupported accounting | 330 physical EOF, zero logical complete | Retained parent omits new coverage fields | No external-history completeness claim |
| Authenticated management identity | No safe host admission | Typed exclusions / forged metadata PASS | No authenticated routing established | Zero-candidate refusal PASS | Management unavailable, OPEN |
| Shared/concurrent caller behavior | Typed composition / PASS | Two callers, shared reader/process, switch/reconnect/stale refs | No real two-chat attestation | OPEN | No authority from `_meta` or process identity |

Installed desktop:26.1103.60933/build25894; CLI0.167.0-alpha.3.2. The installed
schema generator ran with network, provider writes and auth/config reads denied.
Current official source exposes per-call thread correlation and connection sharing;
neither arbitrary `_meta` nor an operator assertion authenticates the route into
AgentMender. [Exact host investigation](../mcp-caller-identity.md) records the
missing per-request issuer/root/thread/epoch contract and companion-host boundary.

## Implementation

[ADR-0030](../adr/0030-resident-bounded-history-decoding.md) keeps the existing eight
crates/read ports. Raw bounded Zstandard decoding and at most two resident working
sets run inside the existing confined worker. The CLI retains its adapter; bounded
framed requests take fresh metadata/source checks and deadlines. There is no
transcript spool, external service, history SQL ingestion or default persistent
index. Ordinary frames are not randomly seekable; cold jumps replay progressively.
[Limits, counters and privacy](../progressive-history.md) describe the tradeoff.

Search now defaults to management; diagnostics require explicit purpose. No missing
identity is replaced with a heuristic. Exclusions and source-bound HMAC references
retain their existing gates. Context freshness now checks the exact target locator
and physical source after unrelated metadata changes, preserving target-change
refusal. Forward suggestions ignore inherited before-window settings; backward
navigation can cross empty intervals of hidden/tool records. Suffix EOF cannot
claim prefix/logical coverage. Unsupported parts count once per record.

## Exact cohort and oversized histories

The UUID set is exactly equal to SR2's330 identities. SR2 had325 Partial sources
and five size refusals; SR3 has330 Partial readable sources and330 verified physical
EOFs, zero size/resource refusals in this cohort and zero complete logical sessions.
All330 have unaccounted external paginated history; unsupported/suppressed records
also prevent absence proof. Full logical eligible-corpus completeness is unproven.
No unrelated changing inventory is substituted for the denominator.

| Private target label | Physical bytes | Decoded EOF position | Search calls | Visible message ordinals navigated | Coverage |
|---|---:|---:|---:|---:|---|
| A | 85,162,746 | 141,182,626 | 13 | 543 | EOF / Partial |
| B | 71,272,933 | 109,369,761 | 9 | 265 | EOF / Partial |
| C | 77,759,980 | 114,012,818 | 12 | 80 | EOF / Partial |
| D | 67,502,706 | 104,900,622 | 10 | 123 | EOF / Partial |
| E | 147,957,003 | 206,933,372 | 19 | 137 | EOF / Partial |

Each target was searched to EOF, navigated from the first available message through
all returned supported chunks, checked near the middle and later in the source,
and probed in other directions. No repeated cursors/chunks were accepted. Exact
source identity/revision metadata matched before/after each run. This is not a
claim of byte-identical whole-provider state while the host was active.

The 91-session MISSION1 query returns81 positives and10 UNKNOWN candidates, all
Partial, versus SR2's78/13 with three size refusals. Calendar October10–20 remains31;
Rome October25 remains18 and equals its explicit25-hour UTC interval. The gRPC
regression grows naturally from1,757,847 to2,322,307 physical bytes and now finishes
in41 advancing context pages instead of20. Its first20 message chunks agree with
the earlier traversal; later source content is not a continuation regression.

## Resources and client evidence

Search-only wait4 maximum reader-child RSS: small plain26.1MiB, medium compressed
64.0MiB, just-over64MiB119.5MiB, largest116.6MiB. Largest-source sequential search
uses19 calls,6.29s,150,447,371 compressed bytes fetched,207,400,316 decoded bytes,
4,612 inspected records and22,586 decoded replay bytes. Header/read-ahead costs
explain work above the206,933,372-byte EOF position. This demonstrates bounded
memory and near-linear sequential decoding in the tested range, not an arbitrary
file-size SLO. Context/jump suites sample higher memory and deliberately incur
replay. [Benchmark guidance](../development.md) separates measurement methods.

A256MiB highly compressible synthetic source uses47.2MiB maximum RSS; a144MiB
valid decoded fixture whose compressed size exceeds64MiB uses131.7MiB while also
checking backward/cold navigation; corrupt/window/deadline cases use21.6MiB and
refuse. Persistent index/checkpoint disk overhead is zero by default. Cold jumps
and backwards cache eviction cost replay; nearby context uses resident bytes.

All four nominated MISSION1 histories pass actual conversational search and
context. D's long message finishes through chunk continuations. The actual C
forward test exposed inherited-before navigation retreat; its preserved failed
attempt is followed by a fixed six-call traversal to EOF. Actual chat tools also
search the67,502,706-byte target in10 calls and retrieve real text. The retained
registration remains unchanged; these tests prove compatible worker retrieval,
not refreshed-parent admission or authenticated caller identity.

## Safeguards and validation

No real session was deleted, archived, moved or modified. No approval/key store,
provider credentials, default persistent index, application restart or new effect
operation was introduced. Descriptor-relative NOFOLLOW, ordinary single-link
identity, source/root ownership, kernel write denial, network/auth restrictions,
privacy suppression, immutable evidence/HMAC tokens, UUID checks, approval,
transaction/recovery, no unknown-effect retry and global lost-update refusal remain.

Locked local fmt, Clippy with denied warnings, workspace tests/build, architecture,
Codex/complete-removal evidence checks, semantic validator and diff checks pass.
Workspace harnesses print563 passes including nested harnesses, zero failures;
eight explicit opt-in tests remain ignored. Approval/recovery/privacy/lifecycle,
Boolean/AND/OR/NOT/UNKNOWN, dates/DST, scoped pagination, source/index freshness,
FTS5 and incomplete Luna-evidence regressions remain in those gates. A fresh Luna
model/custom-agent route or deletion authority is not claimed.

Remote verification/push is blocked by repository authentication: SSH public-key
and HTTPS credential access fail; the connected GitHub account cannot resolve the
repository. The prior exact SR2 workflow was blocked by account billing/spending
before jobs. That historical failure is not final-commit remote CI evidence; the
current remote status cannot be verified. No job reruns or billing changes occurred.

Remaining OPEN: authenticated host routing and cross-provider caller integration,
actual shared-host/switch acceptance, refreshed MCP parent, remote CI, external
paginated hydration and logical/corpus completeness. Read-only physical traversal
is supported within the documented macOS source/format/resource boundary.
