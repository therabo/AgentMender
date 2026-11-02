# M8.2 integration and M8.2.1 complete-session-removal report

Current continuation: [2026-11-03 implementation/evidence report](m8-2-1-continuation-2026-11-03.md).
ADR-0019 is the current structural/visual contract; this previous-cycle receipt
preserves historical facts.


Date: 2026-11-02. **Verdict: OPEN.**

M8.2 was formally verified and integrated. M8.2.1 now has a reviewed pure product
assessment, truthful CLI/approval reporting, exact static projection/artifact
inventories, structural guards and isolated feasibility evidence. It does **not**
yet have a complete Desktop mutation profile, projection/extra-artifact executor,
app coordination or successful graphical reload/restart acceptance. This report
must not be read as milestone completion or provider-impossibility evidence.

## M8.2 integration

Previous local/remote main: `6f798bcebdfd2f763e31d62ef2695c52396ca709`.
Final M8.2 branch `milestone/m8-2-scoped-session-cleanup` and exact head:
`a284767157ace8769deee85b195b5b70efe374d8`. Authenticated GitHub UI freshly
verified [Actions 41320886468](https://github.com/therabo/AgentMender/actions/runs/41320886468),
full commit URL and Linux/macOS/Windows completed-success job links. Local gates,
deterministic evidence validation and isolated exact-profile primary acceptance
were re-run before ordinary history-preserving fast-forward/push. Local and remote
main were independently verified at a284767. M8.2 remains historical **PASS**.
See [the exact integration receipt](m8-2-integration.md).

The tracked M8.2.1 branch was created and pushed from that exact verified main
before implementation. No squash, force push, M8.3, M10 or NLP work occurred.

## Projection inventory

All entries below are investigated prospective obligations. No complete graphical
profile is enabled. RequiredProjection does not mean an admitted capability;
UnknownProjection cannot be relabeled NotRelevant. Exact references, refresh and
restart behavior are in [the full analysis](../research/codex-desktop-projections.md),
[machine inventory](../../compatibility/codex/complete-removal-research/projection-inventory.json),
[role closure](../research/codex-desktop-role-closure.md) and
[source hashes](../../compatibility/codex/complete-removal-research/projection-provenance.json).

| Surface | Classification and authority | Storage/process | Visibility and reconstruction | Proposed exact cleanup | Verification/status |
|---|---|---|---|---|---|
| canonical-active-and-archived-lists | RequiredProjection; canonical provider catalog; rollout fallback | App Server/thread-store; persistent; state_5.sqlite and active/archived rollouts | thread/list and archived/all views, metadata-only entries can survive missing rollout; restart: surviving rollouts may be discovered/backfilled | M8.2 exact row+Legacy/plain payload cut; new GUI contract separately gated | complete active+archived listings and exact read classification after restart; retained positive sentinels; primary cut supported M8.2; GUI profile not validated |
| title-index | DerivedProjection; derived title enrichment and name lookup | rollout/session_index; persistent; session_index.jsonl and permanent session_index.jsonl.lock | name enrichment and name-to-loadable-rollout lookup; restart: index persists but name lookup requires readable existing rollout | exact structured-ID record removal; preserve malformed/unrelated bytes/order; stable lock; synced atomic replacement | all target records absent, non-target bytes/order unchanged, locks and similar IDs preserved; M8.2 retains; candidate for maximum-safe exact record cleanup |
| command-history | UnknownProjection; authoritative command recall; GUI role unknown | message-history; persistent; history.jsonl | global command recall; inspected Desktop catalog source does not read it directly; complete GUI role not proven; restart: persists; replacing inode while live writers hold old file unsafe | typed lossless exact-session record rewrite only after applicable writers stopped; data-inode locking differs from index lock | exact target absence and unrelated byte preservation if admitted; otherwise prove no supported reconstruction role; strong ID field exists; safe mutation/profile/UI role not validated |
| desktop-thread-catalog | RequiredProjection; independent Desktop UI catalog | main process mU/ewY; persistent; CODEX_HOME/sqlite/codex.db or codex-dev.db; local_thread_catalog | independent sidebar/history paging, count, exact entries and project/manual paging; restart: full scan only when incomplete or last_full_reconciled_at null; ordinary restart can preserve ghosts | prefer reviewed exact native invalidation; stopped-app fixed exact row/scan/revision operation only after schema closure | fresh actual GUI preload bootstrap + catalog readEntries/readPage after launch, all relevant pages and collateral checks; statically established, no enabled AgentMender Desktop profile |
| desktop-catalog-scan-records | SharedProjection; derived scan and reconstruction coordination | mU/ewY; persistent; local_thread_catalog_scan_entries/checkpoints/sync_state/metadata | pending/resumed scan can reinsert stale thread; restart: checkpoint persists; stale page reconstruction risk | freeze exact target record preimage plus reviewed observation sequence/revision/tombstone effects; no whole host reset | target cannot upsert from scan after removal; unrelated checkpoints/rows preserved; source model established; destructive profile absent |
| desktop-global-state-and-backup | RequiredProjection; UI preferences/migration authority; fallback copy | global-state RL; persistent_and_in_memory; CODEX_HOME/.codex-global-state.json and .bak | legacy pins/project/order/state + renderer atoms; restart: fallback .bak loads if main invalid/unreadable; remaining backup can restore stale state | stopped-app exact typed record edits in relevant main+fallback backup; preserve every unknown/non-target key | valid main+bak target record role absence, fresh launch projection checks, collateral baseline; risk proven statically; complete per-key closure absent |
| desktop-recents | RequiredProjection; derived recency projection | thread-store/catalog/renderer; persistent_and_in_memory; memory arrays/summaries; catalog source_recency_at; provider recency_at_ms; renderer ordering | Recent sidebar and thread list; restart: persistent catalog/atoms can hydrate; restart alone insufficient | native invalidation if scoped-safe; otherwise exact cleanup under cooperative stop and fresh restart | fresh recent structured view after reload/restart; same-cwd and similar UUID sentinels remain; surface exists; acceptance absent |
| desktop-title-and-content-search | RequiredProjection; derived queries over provider/catalog/history | thread-store search; persistent_and_in_memory; App Server thread/list(title) and thread/search(content); renderer query state | search results; restart: fresh query can find surviving source; GUI path not runtime tested | primary/catalog cleanup and exact live invalidation or coordinated restart | fresh title/content queries after reload/restart; exact target absent and positive unrelated results; source routes established; GUI acceptance absent |
| desktop-pinned-and-custom-sections | RequiredProjection; native membership plus legacy migration/renderer order | pinned service U0 and App Server; persistent_and_in_memory; legacy pinned-thread-ids; app-server-migrated-pinned-thread-ids-by-host; threads.thread_section_id; renderer order | pinned/custom sidebar entries; restart: legacy/renderer records can hydrate; multi-host ownership ambiguous | exact host/thread native semantics preferred; primary row removal cuts native membership, legacy/atom proof separate; retain shared section | fresh pin+section views after restart; unrelated pin same-project and other-host retained; source established; safe per-record closure/runtime missing |
| desktop-archived-all | RequiredProjection; canonical archive state plus independent suppression/catalog | archive service and thread-store; persistent_and_in_memory; provider archived flag/payload plus suppression/catalog state | archived/all lists; restart: surviving archived payload/row/renderer can persist | primary+catalog exact cleanup; native delete/archive cascading remains historical blocker | fresh archived/all actual structured GUI views; unrelated archive remains; surface exists; no GUI acceptance |
| desktop-project-workspace-lists | RequiredProjection; mixed provider/legacy/catalog authority; precedence unresolved | project services + catalog; persistent_and_in_memory; provider threads.project_id/projects/roots; legacy assignments/workspace hints/projectless/order globals | project/sidebar lists and ordering; restart: global/catalog/renderer hydrate independent sets | record-only cleanup after determining current native-versus-legacy/migration precedence; preserve project/roots/shared order | all supported project views target absent, same-project same-cwd unrelated visible; exact precedence/record closure unresolved |
| desktop-renderer-atoms-and-caches | UnknownProjection; UI atom authority and derived cache; closure unresolved | renderer persisted atom Map + main host; persistent_and_in_memory; electron-persisted-atom-state in global main+bak; legacy localStorage codex:persisted-atom:* | pin/chat/custom-section order, customization snapshots; caches may activate summaries; restart: persisted atoms hydrate, legacy migration can survive | exact reviewed atom records only or supported invalidation; never whole renderer cache/global subtree | actual structured renderer state after restart; cache target cannot restore; unrelated atom records intact; not NotRelevant; reconstruction role closure missing |
| loaded-transcript-runtime | RequiredProjection; live discovery and runtime catalog | thread-discovery/cache; in_memory; in-memory conversations/summaries/runtime ownership | loaded/open thread and stale summaries; restart: requires complete old process termination plus persistent-cache proof | native delete handler if exact safe command; otherwise stop exact app+descendants with all writers fenced, restart | fresh loaded-state query, actual GUI catalog and retained thread checks; handler exists; no reviewed external invalidate-only port |
| electron-userData-browser-storage | UnknownProjection; unknown reconstruction role | Electron/Owl; persistent_and_in_memory; explicit CODEX_ELECTRON_USER_DATA_PATH else appData/Codex; browser/LevelDB/IndexedDB/cache roles not closed | potential cache/session hydration; restart: independent userData persists | native invalidate preferred; no rm-rf or whole DB clear | owned isolated userData experiment, structured source-role proof; not tested; not NotRelevant |
| remote-cloud-hosts | UnknownProjection; separate remote/cloud authority; local scope unsupported | Desktop multi-host manager; persistent_and_in_memory; remote/cloud authority and host catalogs | remote/cloud lists,pins,projects; restart: may repopulate separately | outside stable local destructive authority; protect; bare UUID deletion unsafe | local scope bound to exact root/host; preserve same UUID another host; unsupported as local mutation, not assumed absent |
| desktop-other-thread-records | UnknownProjection; role unresolved; not proof of harmless residue | Desktop app DB and voice/automation/visualization services; persistent; codex.db thread_timeline_ledger,live_visualization_suggestions,active_chat_turns | voice timeline content, visualization suggestions and activity metadata; exact standalone catalog reconstruction role not completely closed; restart: Unresolved; potential reconstruction must remain protected | record-level candidate only after exact role/sharing/host identity and full schema/writer proof; current status retain Unsupported | prove all required UI roles absent and all unrelated exact logical rows preserved under actual restart; Retain Unsupported pending exact record authority and actual relevant-mode verification; these rows are not labeled harmless technical residuals |
| desktop-automation-inbox-history | RequiredProjection; independent automation inbox and history authority | Desktop automation runs and inbox service; persistent_and_in_memory; codex.db automation_runs JOIN automations | run/inbox titles and descriptions can include archived user/assistant messages independently of state_5/thread catalog; restart: Unresolved; potential reconstruction must remain protected | exact approved target run record; heartbeat automation effects are distinct frozen targets and must not be triggered implicitly | actual inbox/history structured view absence, collateral automation/run records preserved, target run cannot persist as standalone visible record; source role established; profile/runtime mutation still unvalidated |

## Maximum safe artifact inventory

Removed below means the historical M8.2 admitted primary cut. Unsupported means
no approved mutation semantics/fence/profile, even when exact association is a
strong future candidate. Retained classes remain protected. A projecting residual
in any of these classes prevents complete product removal.

| Artifact class | Current status | Ownership / removal evidence and remaining requirement |
|---|---|---|
| Primary plain legacy rollout | Removed | One canonical regular single-link file, exact UUID first session_meta/header version/path and frozen physical identity; descriptor-relative unlink under maintenance/writer + SQLite fences. |
| Archived plain legacy rollout | Removed | Same operation can admit canonical archived path; active/archived inventory checks duplicate target/header references and retains all other files. |
| Compressed `.jsonl.zst` rollout | Unsupported | Exact official delete_thread.rs removes plain/compressed siblings, but M8.2 inventory refuses unsupported rollout names/formats. Need bounded decompression, identity/reference closure, approved physical siblings and tests. |
| Reverted/replacement rollouts | Unsupported | Provider RolloutReferenceIndex maps several rollout IDs to one thread; native deletion checks external history-base references before all variants. One UUID filename is insufficient. |
| Paginated materialized history | Unsupported | `thread_history_1.sqlite` tables thread_items/thread_realtime_items/thread_turns/thread_history_projection_state are exact thread_id candidates. Existing profile explicitly refuses any history DB or sidecars; needs complete schema/fence/materialization reference contract. |
| Attached/shared histories | RetainedShared / Unsupported | Attached children and history-base references are explicit rejection boundaries. Never erase a source needed by another session; preserve raw unresolved references. |
| Canonical `threads` row | Removed | Exact id predicate and row preimage; fixed transaction validates actual row count 1 and every retained table semantic digest. |
| Dynamic tools | Removed | Exact target thread_dynamic_tools rows are included in commitment; FK ON DELETE CASCADE is enabled; postdelete count must be zero. Tool rows are already in M8.2, not a new opportunity. |
| Read receipts | Removed | Exact target thread_id explicit delete and expected row count. |
| Attachment metadata rows | Unsupported (target refused) | Exact thread_id FK CASCADE proves record association. Promotion requires retained digest exclusion for target only and approval manifest/counts, then cross-session attachment-preservation tests. |
| Attachment payload/assets | RetainedShared / Ambiguous | Provider copies attachment metadata between source/destination threads preserving attachment_type, identity_key, payload; reverse lookup index exists. Exact record ownership does not prove asset exclusivity. Backend-neutral AttachmentStore permits externally resolved File IDs and inline data. |
| Guardian feedback | Unsupported (target refused) | Exact thread_id FK CASCADE is source/schema supported. Record-level promotion possible after explicit boundary/preimage/retained hash/manifest review; never dump BLOB contents. |
| Spawn edges | RetainedShared / Unsupported (target refused) | Edge describes two sessions and native lifecycle uses raw descendants. Removing either endpoint may destroy another session relationship/discovery. Do not adopt native cascade. |
| session_index.jsonl | Unsupported; required if projecting | Exact structured id name records are attributable inside a shared container. Official native deletion removes target name entries. Need strict lossless record rewrite, frozen byte preimage, provider-wide writer exclusion, durable replacement and exact postread. Projection investigator establishes Desktop role separately. |
| history.jsonl | Unsupported; record candidate | session_id identifies exact command history records; container shared and arbitrary input text private. Is non-primary command history in official message-history. Nonprojecting claim requires exact projection evidence, not filename. Maximum-safe profile should eventually remove exact records when fenced and approved even if optional for UI. |
| Shell snapshots | Unsupported; strong promotion candidate | core/shell_snapshot.rs 250-276 creates shell_snapshots/{session_id}.{nonce}.{sh,ps1}, and {session_id}.tmp-{nonce}; cleanup parser at 1197 associates ID. Need anchored exact grammar, single-link physical identities, all generations inventory, inactive consumer/writer proof. No prefix/substring or extension-only ownership. |
| Session env | RetainedUnknown for unidentified disk artifacts | core/exec_env.rs inject_session_env supplies CODEX_SESSION_ID and CODEX_VERSION in child process environment. This is no evidence that a disk session-env directory exists in Codex. Claude fixture paths cannot establish Codex storage. |
| Plans / todos | RetainedUnknown outside rollout | core/tools/handlers/plan.rs emits PlanUpdate event and returns Plan updated. Rollout-embedded plan/tool data disappears with payload. No standalone per-session plans directory is established by that handler. User/project files remain protected. |
| Queue records | Unsupported; exact queued_items candidate, revision rows dependency-sensitive | queue_1.sqlite queued_items have exact thread_id and native delete_thread_queue deletes those items. queued_thread_revisions is a notification/tombstone-like revision tracker whose update/delete triggers advance per-thread revisions; must preserve notification and global cursor semantics, not delete the revision key merely because its UUID matches. Separate schema/fence required. |
| Goals | Unsupported; strong row candidate | state/sqlite.rs goals_1.sqlite; goals migrations thread_goals keyed thread_id plus thread_goal_continuation_deferrals FK CASCADE. Native delete calls delete_thread_goal exact thread_id. Separate DB needs exact schema, retained digest, writer fence, preimages and durable cross-store outcomes. |
| Memory stage-1 rows/jobs | Unsupported; dependency-sensitive | memories_1.sqlite and optional memories_v2_1.sqlite stage1_outputs keyed thread_id and stage1 jobs kind+job_key. Native memory deletion schedules global consolidation if removed row was selected_for_phase2. Need model intended global job side effects before mutation; never silently broaden frozen approval. |
| Memory rollout summaries | RetainedAmbiguous / referenced | memories/write storage writes exact thread_id header but filename uses timestamp+4-character hash+slug, which is not globally unique. Summaries feed shared consolidation and may be cited. Need exact header + cross-session/reference closure before file deletion; filename alone never enough. |
| Consolidated memory/raw_memories.md | RetainedShared | raw_memories.md concatenates many Thread sections; MEMORY.md/memory_summary and phase2 workspace are shared derived state. Markdown headers do not establish a safe complete record grammar. No whole-file deletion. |
| Debug logs | RetainedShared / Unknown | Text process logs lack proven exclusive target ownership. Rollout trace bundle has explicit target references but is shared with descendant threads (see below). |
| Provider DB logs | Unsupported; strong exact row candidate | logs_2.sqlite logs.thread_id; native delete removes only exact target logs; null thread_id rows partition by process_uuid and are explicitly threadless. Preserve null/other sessions and global logs. Configured local logs home can differ from sqlite home. |
| File history / project outputs | RetainedUnknown / Shared | No Codex per-session file-history directory supported by reviewed source. Cwd, output directory hint, creation time, or a tool-mentioned path is not exclusive ownership; worktree/project/user output must survive. |
| Generated images/assets | RetainedAmbiguous / Unknown | Core default InlineAttachmentStore leaves attachment bytes in rollout; host can supply external backend. Desktop/plugin generated-image locations not established by 0.166 core source. Never delete output/cwd recursively. |
| Tool outputs | Removed if inline rollout; otherwise Unknown | Function/MCP output is persisted in rollout model history. Arbitrary output file paths may belong to user/project/shared state; no generic tool-output directory deletion. |
| MCP task state | RetainedUnknown | Core mcp_prewarm_task is in-memory JoinHandle, not proof of task-state files. Third-party MCP server persistence belongs to a different authority/root; no exact local ownership contract established. |
| Desktop catalogs | Unsupported; required projection boundary | Separate host/thread catalog is established by projection investigator's exact installed static source. Must bind host+thread, schema/version and root. A local UUID is not authority to remove remote/cloud catalog row. |
| Global state + `.bak` + temporary writes | Unsupported; shared required projection boundary | Exact recognized target keys can be record-exclusive; unknown fields/other IDs protected. Backup fallback and in-memory rewrite need coordinated handling; deleting one current JSON key alone is insufficient. |
| Recent/project/pinned indexes | Unsupported; required where present | State_5 target row contains is_pinned/section/project; independent Desktop/global projections may retain references. Preserve project roots/order/pinned unrelated sessions. |
| Search indexes | Unsupported / Unknown | DB/name indexes/desktop catalog and renderer state require exact enumerated authority. Unknown supported search reconstruction risk prevents Removed. |
| Cache records / renderer storage | RetainedShared / Unknown | No safe generic renderer-cache wipe. Prefer supported invalidation or explicitly approved exact graceful shutdown/restart with fresh structured verification. |
| Temporary/session scratch | RetainedUnknown | Only exact known shell snapshot temp grammar currently evidenced. Other `.tmp` provider files are shared coordination or unknown; age and path proximity are insufficient. |
| Native writer/maintenance locks, WAL/SHM/journals | RetainedShared | Existing lock and sidecar inodes are part of exclusion proof, not reclaim candidates. Holding stale unlinked lock inode can split writer coordination; do not remove them. |
| Agent message board | RetainedShared / Unsupported | agent_message_board_1.sqlite default, custom namespace DBs possible. Root board contains descendants' posts/subscriptions; native deletion inserts deleted_boards tombstone before scoped row deletion so delayed writes cannot resurrect data. One child ID is not parent's board ID. Do not delete shared root board or tombstone. |
| Opt-in rollout trace bundles | RetainedShared / Unknown | CODEX_ROLLOUT_TRACE_ROOT may be external. Root bundle contains manifest.json, trace.jsonl, payloads/ and reduced state.json for root plus children; source thread.rs explicitly shares writer/bundle. Exact root_thread_id alone does not authorize whole-bundle deletion for one session. |
| Auth/settings/plugins/skills/global caches/remote state | RetainedShared | Outside single-session artifact ownership; never reclaim for aesthetic completeness. |

The [full artifact research](../research/complete-session-removal-artifacts.md)
records exact official source lines and targeted follow-up of ten prior cleaners
plus ghost-session issues. It does not promote third-party heuristics to authority.

## Required projection cleanup

The prospective contract requires canonical active/archive lists, independent
Desktop catalog and scan reconstruction, current/global fallback state, recent,
search, pinned/custom-section, archived/all, project, loaded-runtime and eligible
automation inbox/history entries to be clean. Title-index exact records are also
maximum-safe candidates. Browser/atom/activity/voice/visualization roles and any
supported reconstruction contribution still need closure. Another host/account's
same UUID is protected. Exact record-level mutations and their new manifest are
pending, not silently added to the approved v1 scope.

## Desktop coordination

Installed bundle: `com.openai.codex`, Desktop26.1027.51404/build23880; embedded
CLI0.166.0-alpha.5.1. M8.2 pins stable0.166.0. A source-supported explicit stable
CLI override is a new composite candidate requiring separate validation.

Global state lives in a Map and rewrites both main JSON and .bak; catalog scans,
renderer state and native sections can independently project entries. Live direct
edits are not fenced by the existing rollout/state-DB locks. An internal exact
catalog notifyThread(remove) candidate exists, but its production transport,
approval/effects/retention contract and full projection coverage are not validated.
Generic deleted events have additional automation/worktree effects and are unsafe
as a guessed invalidate-only shortcut.

No production shutdown/restart/invalidation workflow is admitted. Future effects
must identify the exact app/roots/processes, disclose and bind graceful shutdown,
termination and any permitted relaunch before approval. An initially closed real
app is not launched merely to claim verification.

Synthetic kernel file/socket canaries passed in parent and exec child. The reviewed
fork-denied signed-main startup probe exited SIGTRAP(-5), with no output, before
verified readiness or GUI state. The child was reaped, no helper/native CLI was
permitted, and no real user state or running app was touched. No unsandboxed retry
was performed. See [the result](../research/codex-desktop-isolation-result.md).
This is not graphical acceptance or proof of provider impossibility.

## Removed predicate

The separate pure SessionRemovalAssessment requires exact matching scope and a
nonempty complete frozen surface inventory, every immediate/reload/restart/fresh-
discovery absence check, primary/catalog absence without reconstruction risk,
unchanged collateral and complete safe-unit coverage. Duplicate/foreign units,
protected approved targets, unapproved actual actions, overflow and contradictory
removed-unit risk fail closed. Every supported exclusive unit must be completed
or have explicit freshly established technical safety retention; skipped or new
unapproved safe units prevent maximum-safe completion. Unknown sizes stay unknown.

Product Removed requires all predicates and no residual. RemovedWithResiduals
requires the same predicates with technical-only, non-projecting, non-primary
residue. Projection risk/unknown relevant role cannot succeed. Incomplete,
ProjectionIncomplete or InvalidEvidence remain non-successful assessments;
existing provider Blocked/OutcomeUnknown/Failed transaction protections remain.

Historical v2 CleanupManifest/Report, plan hashes, grants, durable encoding and
replay are unchanged. Current production calls only assess_legacy_session_removal:
valid primary Removed/RemovedWithResiduals maps to ProjectionIncomplete,
projection_clean=false, product_complete=false and maximum-safe cleanup Unknown.
CLI top-level outcome is the product assessment; raw primary_cleanup_outcome and
durable Succeeded retain their qualified historical meanings. Execute/reconcile
returns exit2 while complete removal is unverified. Preparation/review/approval
explicitly disclose primary-only scope and no app restart authority.

The positive full-model witness is synthetic and has no admitted production
adapter. There is no new destructive manifest/profile or parallel authority engine.

## Session-visible verification

| View/check | Source-established existence/route | Actual removal/reload/restart acceptance |
|---|---|---|
| Sidebar/thread list | Independent catalog, preload and renderer services | Not run; startup did not reach verified GUI |
| Search | Title list and content-search routes | Not run |
| Recent | Provider/catalog recency and renderer lists | Not run |
| Pinned/custom section | Legacy migration and native membership | Not run |
| Archived/all | Provider archive and Desktop suppression/catalog | Not run |
| Project/workspace | Provider/global/catalog projections | Not run |
| Automation inbox/history | Independent automation_runs UI query | Not run |
| Restart/reappearance/fresh GUI discovery | Explicit required postconditions | Not run; not inferred from primary test or canaries |

No investigated view was proven nonexistent; none is marked NotRelevant.
ChatGPT/account-specific activity and remote/cloud roles remain unsupported or
unclosed. An API-key-configured synthetic local-mode route is only prospective.

## Maximum safe cleanup

No new provider deletion category was enabled. Current removal covers approved
plain active/archived primary rollout, canonical thread row, dynamic tools and
read receipt. Source-backed promotion candidates include exact index/history
records, snapshot generations and thread-linked attachment/feedback/log/queue/goal
records, subject to new schema/dependency/writer/profile review. Shared memory,
trace bundles, board state and user/project assets remain protected; queue revision
and board tombstone semantics cannot be erased merely because a UUID matches.

The pure model derives discovered/safely-attributable/planned/actual-removed/
already-absent and retained category counts and known/unknown bytes. Production
v1 cannot supply complete expanded coverage, so reports None/Unknown rather than
invented percentages. Actual record/file mutation and maximum-cleanup fixtures
remain required before M8.2.1 PASS.

## Reclaim

The separate exact-primary fixture successfully unlinked 67,108,864 logical rollout
bytes. Physical allocation reclaim remains unknown (the fixture is sparse;
APFS/clones/snapshots/open handles/SQLite allocation are separate concerns).
No real user session was removed. Five redundant owned test-binary copies were
reclaimed after evidence preservation; these are excluded from product reclaim.

## Collateral safety

Final independent Sol/high review found no new destructive operation, target or
authority. Provider-api/provider-codex/persistence and original domain cleanup,
planning and transaction source diffs against a284767 remain empty. The existing
reviewed exact UUID SQL, retained semantic digests, physical/header/root guards,
SQLite/WAL and writer fences, consumed intent and read-only reconciliation remain.
No new index/history/global/key/sidebar/pin/cache mutation can damage another
session because no such production action was introduced. That is not a claim
that a new GUI mutator was tested: it is deliberately unadmitted.

Pure shared-record/global-key/projection preservation checks validate claimed
collateral evidence only. Actual lossless shared-container atomicity and GUI
noninterference still need adapter/integration tests.

## Tests

All ordinary local gates passed: fmt, locked workspace Clippy warnings denied,
locked workspace tests/build, architecture fitness, exact Codex evidence,
complete-removal inventory validation and whitespace. Workspace result: **391
primary passes**, 8 nested child pass summaries (399 including those children),
zero failures, four opt-in ignores. Separate exact-primary acceptance passed
(1 test, 35.09s); it is not GUI acceptance.

New assessment suite: 11 tests, including 128 deterministic generated witnesses
and 13 failure cases per witness, seed `0x7a3948bfd14c1923`, plus explicit risk,
foreign/duplicate/protected-unit, optional retention, restart, collateral and
coverage regressions. Historical scoped model retains 4,096 cases with seed
`0x4d382e3253434f50`; a new regression proves even residue-free primary receipts
cannot establish complete removal. Existing properties/regressions remain intact.

The original transaction/primary crash cases and reconciliation tests passed.
New projection publication, app shutdown/restart, max-safe mutation and actual
GUI reappearance fault tests remain **not implemented/not passed**. Synthetic
assessment and kernel canaries do not replace them. Structural canaries reject
whole-container deletion, substring attribution, generic JSON/SQL/cache ports,
broad process termination and false legacy product promotion.

Independent reviews: Sol/high accepted the contract before Terra/high assessment
implementation; four concrete predicate/accounting findings were corrected and
regression-tested; final fail-closed foundation review found no actionable blocker.
Signed-main probe review was limited to that experiment, not GUI mutation support.

## Documentation

README, AGENTS, scoped cleanup, planning, integrity, approvals, transactions,
Codex lifecycle/full-cleanup, compatibility, MCP/dependency/scope notes and threat
research now distinguish primary cleanup from complete removal. ADR-0018 records
projection/reconstruction/restart/maximum-safe/record/coordination rules and its
relationship to ADR-0017. ADR index and M8.2 integration receipt are updated.

Internal AgentMender_Milestones.md and Scoped_Cleanup_Architecture_Addendum.md
now make M8.2.1 current and gate M8.3 on its PASS. Original PDF and historical
reports remain preserved. Reconstruction read all assigned current Rust/docs/tests
across agents; all85 compatibility JSONs were losslessly parsed and repeated
schemas structurally reconstructed as20 roots/375 unique definitions, not
misrepresented as independently rereading every duplicated generated line.

## Git / CI

Branch: milestone/m8-2-1-complete-session-removal. Verified base/main:
a284767157ace8769deee85b195b5b70efe374d8. Ordinary push only. The final exact
commit, remote SHA and Linux/macOS/Windows job evidence are recorded in the
internal final report/verification receipt after the final commit is created;
this tracked report is part of that commit and cannot contain its own SHA.
The added evidence gate remains provider-independent. Actual GUI acceptance is
separate and has not passed. Passing ordinary CI cannot turn this milestone PASS.

## Final product questions

1. Complete removal from every supported Codex graphical projection: **NO**.
2. Guaranteed absence after reload/restart under a validated graphical profile: **NO**.
3. Expanded maximum-safe artifact cleanup implemented/attempted across the required
   complete-removal profile: **NO**.
4. Technical residuals may coexist with product removal: **YES**, only when proven
   non-projecting/non-primary and safe for other sessions. The model enforces that
   rule; no complete graphical production profile is admitted yet.

## Verdict

**OPEN**. Primary integration is complete and the reporting/assessment foundation
is implemented and tested. Exact GUI role/schema/coordination closure, typed
projection and additional artifact effects, their frozen approval/crash semantics
and actual graphical restart/reappearance acceptance remain required. Missing
implementation/evidence does not prove provider-impossibility BLOCKED.

M8.3 is not recommended for implementation until M8.2.1 PASS. It must compose the
stronger per-session contract. M8.3, M10 and the natural-language layer remain
unstarted.
