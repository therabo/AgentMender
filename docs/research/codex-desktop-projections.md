# Codex Desktop projection surface inventory

Status: **OPEN**, static exact-bundle research as of 2026-11-02. No complete
graphical mutation profile is admitted; no GUI removal or restart acceptance is
claimed. A RequiredProjection below is an obligation for the prospective GUI
profile, not an enabled production capability. UnknownProjection is never skipped
as NotRelevant. No investigated view was proven non-existent.

See [machine inventory](../../compatibility/codex/complete-removal-research/projection-inventory.json),
[source hashes](../../compatibility/codex/complete-removal-research/projection-provenance.json),
[static schema](../../compatibility/codex/complete-removal-research/desktop-catalog-schema-static.json),
[isolation design](codex-desktop-isolation-design.md) and
[ADR-0018](../adr/0018-complete-session-removal.md).

## Projection inventory

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

## Required projection cleanup

Primary/catalog removal alone does not close the independent Desktop catalog,
scan checkpoints/tombstones, global JSON and fallback .bak, native/legacy pins,
recent/project/archive/search views, loaded runtime and persisted renderer atoms.
Unknown browser storage and other thread-specific Desktop tables need role closure
before success can be claimed. Host/thread identity is composite; another host's
same UUID is protected. Session index name enrichment is derived but remains a
maximum-safe exact-record cleanup candidate. History has a strong session field
but its full GUI reconstruction role and mutation fence remain unvalidated.

## Coordination and restart

The global store loads once into a Map and queues writes of both current and
backup state. The local catalog has independent scan checkpoints and may perform
only incremental discovery after ordinary restart. Therefore direct edits while
live can be overwritten, and restart alone is no deletion proof. Native deleted
events trigger scoped catalog and memory invalidation, but no safe public external
target-only invalidation operation covering all stores has been established.

The installed Desktop embeds alpha 0.166.0-alpha.5.1, while M8.2 pins stable
0.166.0. A source-supported stable CLI override plus isolated userData is a new
experimental composite candidate. It is not authority to transfer the old profile.
Any shutdown/relaunch must bind the exact app, state roots and processes and appear
in the frozen approval. No real app instance is stopped or restarted here.

## Session-visible verification

Sidebar, title/content search, recent, pinned/custom section, archived/all and
project views exist in inspected source. Their actual baseline visibility and
absence after reload/restart are **not tested**. The actual preload bootstrap and
app-host catalog paging provide potential structured probes, but bootstrap caches
once per renderer and caps results. A sampled list is not exhaustive absence.

No screenshot/OCR proof, synthetic list-only success or CLI restart is substituted
for the required isolated graphical acceptance. Current facts support OPEN, not
provider-impossibility BLOCKED and not PASS.
