# M8.2.1 Desktop candidate checkpoint — 2026-11-06

**Verdict: OPEN.** Schema 4 integrates a real, narrowly scoped Desktop adapter
candidate into planning, independent approval, consumed durable intent, execution,
progress and structural reporting. It is explicitly `DesktopUnaccepted`: even a
completed candidate cutoff reports an unknown outcome and cannot yield product
Removed. This is reviewed implementation evidence, not admission or restart
acceptance. The [earlier continuation](m8-2-1-continuation-2026-11-03.md) and
[prior report](m8-2-1-report.md) remain historical receipts.

## Exact profile and prerequisites

The candidate pins Darwin arm64, a local host, Legacy primary profile, exact
storage schemas, identities and signed Desktop build 23880 (version
26.1027.51404) with the pinned CLI and stable primary worker. Current installed
build 24141 differs; it does not qualify by version similarity. The
[drift and acceptance record](../research/codex-desktop-acceptance-drift-2026-11-06.md)
contains exact hashes and the bounded search for the reviewed bundle. No current
build admission, application launch or real user-state cleanup is claimed.

The user must keep Codex and every relevant writer closed through verification.
A conservative same-UID process census considers writers across roots, and active,
unreadable, ambiguous or changing inventory refuses execution. Inventory and
boundaries are rechecked during the operation. This is an explicit cooperative
closed-writer condition, not a kernel CAS or a guarantee against an uncooperative
launch in the compare/publication gap. Production does not stop or restart Codex.

## Fixed adapter effects and verification

Schema 4 freezes a typed Desktop subsection and exact ordered actions: catalog
composite, global backup, global main, session index, primary payload and state
thread records. Existing schema 2/3 plans, hashes and approvals do not upgrade.
The catalog operation binds local host, thread and source kind, and commits its
exact target row and conditional scan suppression/revision changes together;
retained commitments protect unrelated rows and checkpoints. Global main and
backup are separate lossless, exact-membership publications; ambiguous bare-ID
ownership is refused. The index removes exact structured target records while
preserving unrelated and malformed bytes. Missing stores are frozen as absence;
new units or changed preimages require refusal/replanning. Legacy browser import
has a bounded observation/admission check, not a browser-store wipe. History is
not mutated and its projection role remains unverified.

All boundaries and preimages precede destructive work; each effect is followed by
durable progress before the next. Drift before publication is refused; uncertainty
following an effect is unknown and reconciliation is read-only, with no automatic
retry or reclaimed-byte credit. A final destination re-stat closes a reviewed
publication race; catalog and primary clean no-sidecar WAL handling were fixed.
These are independent review findings and fixes, not a claim that all remaining
races or reconstruction roles have passed acceptance. Compare-and-rename is not
an atomic compare-and-swap.

Recent, pin, project, search and archive consumers depend on catalog, global,
primary and other sources; their complete reconstruction closure is not proven.
History, browser/legacy and derived surfaces remain unverified. Same UUID on a
foreign host, same-cwd and same-project sessions, similar IDs and unknown/shared
state must survive; complete Desktop collateral preservation is not yet proven.
No auxiliary cleanup family is promoted by this checkpoint. Only actually observed
approved logical file bytes may count as reclaimed; no new real-state reclaim is
claimed.

## Acceptance and verification status

The candidate must run the real discovery, frozen manifest, independent approval,
consumed intent, adapter and structural checks in isolated owned state, then pass
two fresh actual application reconstruction cycles with positive collateral
sentinels and backup, scan and legacy challenges. No graphics acceptance cycle or
reappearance challenge has passed. The synthetic isolation canary and source
review in the drift record are not graphical acceptance; its earlier main-only
probe exited SIGTRAP. Exact bundle and reviewed confinement are prerequisites.
Local formatting, strict Clippy, workspace tests and build, architecture and
both compatibility evidence checks passed. The workspace suite ran 452 tests
successfully, with five explicit opt-ins ignored. The owned CLI mismatch test
refused installed build 24141 before opening provider evidence and left the
synthetic roots unchanged. The earlier continuation baseline at
`6abead737378aaa739fea4d3ec4c94383ace2cd3` passed Linux, macOS and Windows in
[GitHub Actions run 41549459194](https://github.com/therabo/AgentMender/actions/runs/41549459194).
The candidate's exact-head CI is a separate receipt.

## Product questions

1. **NO** — the unadmitted candidate does not establish removal from every
   required persistent reconstruction/projection source.
2. **NO** — the intended structural assessment does not use visual inspection,
   but complete success cannot yet be established.
3. **NO** — neither required isolated reconstruction cycle has passed.
4. **YES, within the candidate contract** — relevant active or uncertain writer
   inventory refuses mutation; production does not terminate or restart Codex.
5. **NO, for the complete profile** — preservation is designed into qualified
   effects, but all requested Desktop collateral sentinels and cycles are unproven.
6. **NO, for the complete profile** — candidate actions are fixed and approved;
   full maximum-safe auxiliary coverage is not yet admitted or proven.
7. **YES, conditionally** — only proven non-projecting, non-primary technical
   residuals can coexist with `RemovedWithResiduals` after all other acceptance
   conditions; no complete Desktop profile presently establishes that result.

**OPEN.** Complete source-role and writer closure and isolated reconstruction
acceptance remain. M8.3 follows only after M8.2.1 PASS.
