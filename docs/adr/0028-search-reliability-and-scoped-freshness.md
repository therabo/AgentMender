# ADR-0028: Search reliability and query-scoped freshness

## Status

Accepted, 2026-11-09, for P0-SR1. Refines ADR-0027 without changing the eight
crates, read ports, FTS5 architecture or lifecycle admission.

## Context

The real acceptance run found missing source locators/files, a completed-message
continuation loop, unrelated activity invalidating filtered cursors, and strong
semantic rejections from incomplete evidence. Synthetic acceptance did not prove
the real client/provider coverage or caller identity.

## Decision

Keep the existing kernel-attested source reader and descriptor-relative NOFOLLOW
reads. Export only closed, path-free diagnostic codes. If a validated locator is
missing, permit only fixed active/archive counterparts derived from that same
locator and its recorded calendar prefix; never recursively discover files or read
outside the existing sessions/archived_sessions roots. Multiple counterparts
refuse. The selected file still needs an exact header UUID and physical revision.
Native App Server history availability does not authorize startup writes, copying
credentials, or borrowing the desktop client's transport. Unavailable history
remains explicit and cannot prove absence.

Refine cursor fingerprints to the eligible query cohort and metadata facts that
affect filtering, lexical ranking or ordering. Unrelated excluded-project activity
and unused display/activity fields must not invalidate an identity traversal.
Eligible identity insertion/removal and changes to relevant facts still refuse,
including changes before a keyset boundary. The bounded inventory window retains
its own validation. Source revisions for selected content remain pinned and checked.
This is a query-scoped freshness contract, not an immutable global transcript
snapshot: display-only facts are read fresh. Changed relevant membership requires
a fresh query; new matching sessions are not silently added to an old traversal.

The real retest also found the worker's independent whole-DB post-read fingerprint
gate refusing unrelated activity during a call. Keep that gate: when it changes,
search alone may obtain one additional bounded READ_ONLY/NOFOLLOW SQLite backup
under the same worker deadline. Require unchanged physical primary DB identity
and equality of query-scoped cohort/window/source facts against the staged result.
Otherwise refuse. This revalidates relevant changes rather than ignoring store
drift or retrying transcript ingestion. Context/transcript operations retain their
existing conservative post-read refusal. No atomic global history claim follows.

Reserve positive next-reference chunk offsets for unfinished messages. Completed
messages anchor forward/occurrence traversal at offset zero. The application/MCP
boundary must reject repeated no-progress continuation pairs rather than loop.

Semantic acceptance requires trusted coverage facts in the candidate handoff.
Missing evidence and incomplete sources cannot support a session-wide negative,
regardless of model confidence or syntactic JSON validity. Contradictory passages
may disprove a proposed resolution while still making the conversation relevant.
Fresh deterministic identity/filter checks remain the parent's responsibility.

Guaranteed management mode stays closed until an enforceable host binding exists.
No launch declaration, client name, cwd, process argument or model UUID grants it.
Persistent indexing remains explicit opt-in; private real receipts stay outside Git.

## Validation

Add source-error/relocation/isolation tests; multi-chunk/end/boundary/no-progress
navigation tests; unrelated-update and relevant-membership/keyset regressions;
semantic rejection tests using trusted complete versus partial coverage. Recheck
the fixed real acceptance cohort and four MISSION1 targets, separately labeling
fixture, direct-driver and refreshed actual-client results. Preserve all existing
approval/recovery and compatibility gates and exact-final three-OS CI.
