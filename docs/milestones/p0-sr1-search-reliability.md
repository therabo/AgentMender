# AgentMender — P0-SR1 Search Reliability Hardening

Starting main: `264a45a601f5645a6b4290e1e6a63021e33f28c9`, clean and equal to
the directly observed remote branch. Final commit and exact three-OS CI are bound
by the external delivery receipt, avoiding a self-referential commit. Private real
requests, source references, latency receipts and model outputs remain outside Git.

**Verdict: reliability fixes implemented; real content coverage and production
management acceptance remain OPEN.** No new search engine, graph service,
embedding, crate, persistent-index default or lifecycle admission was introduced.
[ADR-0028](../adr/0028-search-reliability-and-scoped-freshness.md) refines ADR-0027.

## Coverage findings and safe improvement

The exact 330 identities finalized by the preceding acceptance run were replayed:
16 still have readable Partial content; the other 314 now report `source_missing`.
All 330 were finalized after the snapshot refinement. This is the same identity
cohort at a later observation time, not a coverage percentage from changing live
inventory totals. **No real readability increase is claimed.**

All four nominated MISSION1 targets remain unresolved. A complete scoped traversal
reported 91 `source_missing` sessions, zero proven content matches. The isolated
reader fails before parsing: recorded backing sources and fixed active/archive
counterparts are missing. These observations do not establish that the provider
lost the conversations; the existing native client can read the known targets.
They do establish that AgentMender cannot presently retrieve their content.

Closed diagnostics distinguish missing locator/source, outside-root, permission,
link/scope, foreign identity, ambiguous counterpart, unsupported format and
conservative unavailable cases without exporting OS error messages or paths.
Fixed active/archive locator counterparts are attempted only after ENOENT and
retain descriptor-relative NOFOLLOW, unshared regular files, exact header UUID,
physical revision and freshness checks. No recursive scanner or root widening.
Disposable tests prove relocation recovery and refusal of ambiguity, symlinks and
foreign headers; these are not claims of recovered MISSION1 data.

The documented non-resuming App Server history APIs are availability evidence,
not write-denied startup or trusted access to the existing client's transport.
The current worker denies network and auth/config reads. No unisolated native
startup, credential copy, desktop impersonation or unsupported history-store read
was used. A safe native history transport/representation remains a precise blocker.
Unknown layouts, external paginated history, unavailable sources and oversized or
suppressed records remain Partial/unsupported rather than negative proof.

## Context progress

Positive next-reference offsets now designate unfinished messages only. Once a
message is complete, the provider resets its continuation anchor offset so the
MCP projection advances forward/through occurrences. End of supported source has
no next ref. The parent projection rejects identical reference/window pairs.

The original real failing message was retrieved with fresh refs through seven
180-byte chunks, then automatically advanced to subsequent messages and reached
the supported source end after 20 pages without a repeated pair. Coverage remains
Partial because external history is not hydrated. Real middle retrieval again
returned ordinals 461/472/599, including the passage beyond the old head/tail gap.
Exact boundaries, following messages, exhausted sessions and no-progress refusal
have deterministic regressions. Source identity/revision checks are unchanged.

## Live pagination

Fingerprints pin the eligible identity cohort, predicate leaf truth, lexical facts
and ordering keys, rather than every display/activity value in unrelated rows.
Matching additions/removals, relevant truth/order changes, earlier matching keyset
insertions, exclusions and selected content revisions still refuse. Display-only
facts are read per snapshot; no immutable global transcript snapshot is claimed.
Changed matching membership requires a fresh query, not silent result merging.

The first scoped-fingerprint retest exposed a second cause: the worker's whole-DB
post-read gate rejected unrelated writes during a call. That gate remains. Search
may revalidate a changed store against one additional bounded read-only SQLite
backup under the same deadline, requiring unchanged primary DB identity and equal
relevant cohort/window/source facts. The first backup is released first; no
transcript rescan or unbounded manifest is introduced. Context retains conservative
store-change refusal. Physical replacement still refuses even with identical bytes.

The final actual-client metadata traversal completed 19 pages, 91 unique MISSION1
identities, no duplicates/skips or stale error, interleaved with ongoing unrelated
chat/tool activity. Before/after activity observations changed during the overall
test interval. Fixture tests cover writer interleaving, matching membership drift,
sort changes and >10,000-row keyset continuation. Raw-window changes and relevant
source changes may still conservatively require a new query.

## Semantic result acceptance

The validator now distinguishes JSON shape from semantic acceptance. Session-wide
negatives require parent-supplied complete coverage, fresh source/original-filter
checks, a supported evidence basis and a parent-vetted negative evidence ref.
Missing coverage is unknown. Partial excerpts, inline EOF and lack of a lexical
match cannot admit even a low-confidence negative. Insufficient results identify
the missing evidence. A contradiction can reject a proposed fix while leaving its
conversation relevant. Existing positive/insufficient v1 receipts remain readable;
historical model evidence is not rewritten. No destructive authority is added.

The original overconfident real assessment is rejected by the new guard. A fresh
explicit gpt-6-luna/medium run on the original four real candidates returned one
relevant and three insufficient on its first pass, with valid exact identities,
refs, schema and guard. Parent-mediated actual MCP retrieval supplied four bounded
validation windows (1,545 UTF-8 excerpt bytes), followed by fresh deterministic
identity revalidation. Luna did not invoke MCP directly. Native standalone custom
agent discovery/routing and feature reduction remain unverified.

## Management, privacy and acceptance boundaries

Real management requests still return `caller_identity_required`, no authenticated
binding and zero candidates before provider access. Thread metadata/session IDs
and operator files do not prove exclusive per-chat MCP routing. No trustworthy
host binding was established, and no privileged admission was reactivated.
Diagnostic results retain no automatic caller-exclusion guarantee.

No session-changing operation was requested. No approval store, credentials,
provider config, persistent index, app restart or destructive test was introduced.
Normal host activity continued; byte-identical live provider state is not claimed.
Existing approval/recovery, lifecycle compatibility and source-isolation regressions
remain required. The local locked suite, Clippy, build, architecture and compatibility
validators, semantic regressions and private real read-only checks were run; exact
final CI evidence is separate. Existing opt-in historical/live tests retain their
explicit environment requirements rather than silently running against user state.

| Component | Implemented | Fixture | Real driver | Actual client |
|---|---|---|---|---|
| Differentiated failures/fixed counterparts | Yes | Pass | Missing sources classified; targets unresolved | New worker diagnostics observed |
| Completed-message progress | Yes | Pass | 20 pages to supported EOF | 20 pages to supported EOF |
| Scoped freshness/post-read revalidation | Yes | Pass | Cohort/metadata traversals finish | 19 pages/91 identities finish |
| Incomplete-evidence acceptance guard | Yes | Pass | Historical negative replay rejected | Fresh explicit Luna + mediated evidence pass |
| Additive parent content_coverage/no-progress guard | Yes | Pass | New server tested | Existing parent needs connection refresh for new projection/schema |
| Native TOML custom-agent route | Configured | Schema/instructions only | Not claimed | OPEN |
| Guaranteed management self-exclusion | No host admission | Existing exclusion tests pass | Safe missing-binding refusal | Safe refusal; automatic binding OPEN |

Residual production blockers: inaccessible target history, no admitted complete
native history transport, unsupported/Partial provider formats, unverified native
custom-agent route, parent catalog refresh acceptance and authenticated exclusive
caller binding. No fully accepted production search claim is made.
