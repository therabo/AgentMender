# Follow-up: concrete role closure and auth-independent observation

Research only, 2026-11-02. No app/provider startup. Source offsets refer to UTF-8-decoded Unicode character offsets of hashed exact installed assets. All additional hashes are in projection-provenance.json.

## Actual default auth gate, and two distinct fixture approaches

The installed renderer `app-runtime-aa2c0a69096f.js`, function RA around characters38200-39000, selects login when auth is loading/absent while required, or a ChatGPT-mode token is explicitly missing. Gateway/Bedrock pending state also selects login. Otherwise non-ChatGPT modes can select the ordinary app. This is actual routing, not an inference from the existence of an API.

The fresh default OpenAI provider requires auth, so an empty synthetic HOME may render login. Local catalog access nonetheless exists before account selection: main bootstrap includes local host with no cached principal; the localThreadCatalog service checks authorization only for `chatgpt:` hosts (bundle439 characters2837561+). This can prove actual GUI-facing catalog hydration even if normal sidebar is hidden by login, but it cannot alone prove all normal user-visible views.

Potential ordinary fixture mode A: exact stable account_processor.rs:1107+ derives requiresAuth from model_provider.requires_openai_auth. A real custom no-auth provider returns no auth method and requiresAuth=false. This is a genuine provider configuration path, not an app mock. HOWEVER root config.toml would be refused by historical M8.2 admission, so using that mode for an end-to-end M8.2 production acceptance needs a separately reviewed exact composite profile. Do not hide this boundary change.

Potential ordinary fixture mode B: create only an owned synthetic auth.json selecting normal ApiKey mode with a deliberately invalid, conspicuously synthetic API-key string. Exact official storage.rs:47+ defines auth_mode/OPENAI_API_KEY; manager.rs:355-360 and893-896 construct ApiKey auth from the stored nonempty key without a remote validation request; account_processor.rs reports the selected method. An invalid fixture key does not authenticate any account and all network stays kernel-denied. This mode does not modify app auth code, fabricate a ChatGPT principal/token or access real secrets. It avoids a config file admission exception. Its use must be explicit in the acceptance record: API-key-configured local UI fixture, not proof of authenticated ChatGPT/cloud views. The app may still have additional access/onboarding guards, so baseline view rendering must be observed rather than presumed. No fixture key has yet been written or used by this investigation.

Do not simply put OPENAI_API_KEY in inherited environment and assume App Server honors it. The exact auth manager distinguishes CODEX_API_KEY enablement and on-disk storage; use source-backed fixture selection and verify the actual reported auth mode without returning token material.

## Reuse actual renderer service bindings for structured reads

`connect-app-host-38c2a2481282.js` exports a connector that creates a MessageChannel, posts connect-app-host through the preload and returns the bundled RPC client. A second connector invocation also registers an app view in main, which may affect runtime ownership. It is unnecessary for a read probe when the ordinary app has already initialized services.

Both app-runtime and initialize-app-host-services import `{Vct as R/U}` from `app-shared-e711d01481fb.js`. That export is the live variable KQ; initialization assigns the real main-process services through Kwt. An owned renderer may therefore import its already-loaded exact module and read `Vct.localThreadCatalog.readStatus()`, `readEntries([{hostId:local,threadId:synthetic-id}])`, and complete `readPage` pages using the actual established app host. This is a concrete candidate structured observation path without creating another RPC connection, modifying state, or inventing a replacement catalog. It still needs an owned-process transport (inherited debugging pipe or equivalent) and readiness proof; no real current renderer is queried.

The actual renderer JN/YN in app-runtime around characters110767-113800 seeds its catalog from the actual preload bootstrap, enables the local app-server source and syncs it independently from enabling the ChatGPT source. ChatGPT source population alone is account/feature gated. Existing local catalog service reads pass the wrapper without a ChatGPT account lease. This materially supports auth-independent local catalog feasibility.

Freshness matters: preload memoizes bootstrap once; a new renderer is needed. The normal live service readEntries/readPage can be fresh. Both are actual GUI-facing state but acceptance must positively show target visibility before cleanup across each applicable ordinary view and then absence after fresh reload/restart. A JSON storage query alone is not equivalent.

## Narrow catalog invalidation service, separate from native lifecycle

The appHost service includes a semantic `localThreadCatalog.notifyThread({hostId,threadId},remove)` (bundle439 character2836110). It resolves the exact host coordinator and calls that coordinator's archived handler. The coordinator applies authoritative removal for the exact thread: catalog row removal, active/resumable scan tombstone, observation sequence/revision handling and listener publication. It does not send App Server thread/archive or thread/delete and does not enter the general deleted-thread host cleanup path.

This is an actionable narrowly scoped native-UI candidate for an owned experiment. Its static effect model is substantially smaller than cascading native delete. Nevertheless it is a private interface, and neither an external production transport, independent-approval dispatch binding, nor collateral-safe exact schema contract has been validated. It does not clear every global/backup/atom/run record by itself. Do not expose generic RPC/evaluate ports or call it against real app state during research.

## Discovered inbox/history projection

Desktop automation runs are a separate concrete visible projection, not merely a diagnostic residual. Bundle339 characters1110187-1112000 queries automation_runs joined automations, deriving title/description from run fields, archived user/assistant messages and automation metadata. It does not join state_5. A retained target run can remain visible in automation inbox/history even if canonical row, payload and thread catalog are gone. Classify it RequiredProjection when the target has an eligible current-owner run.

The UI native deletion wrapper (bundle339 around743700-744405) deletes its target automation_runs row after the App Server result. The general thread/deleted receive path (around917792) calls ri (around811309), which also finds/deletes heartbeat automations targeting the thread, their run records, and goes on to other host/worktree cleanup. Therefore injecting a general fake deleted event is not a bounded invalidate-only shortcut. Every applicable run/heartbeat/worktree target must be proven exclusive and frozen if cleanup needs it.

`active_chat_turns` has another explicit reconstruction route: bundle439 characters1502371-1504200 writes profile/account/user/thread/launch IDs; `takeActiveChatsFromPreviousLaunch` returns and deletes previous-launch thread IDs for that profile/principal. This is app restart activity state, not a static never-read log. Its caller and cache-rehydrate behavior still require exact closure; do not call it non-projecting by default.

Voice `thread_timeline_ledger` is read by exact host/thread/sequence in bundle439 around1344500-1346000. The record reader does not by itself enumerate thread catalogs, but it is actual UI timeline content. `live_visualization_suggestions` contains exact account/user/host/thread and turn IDs in migration schema; its complete current runtime consumer was not established in this pass. These are precise remaining role questions, not evidence of harmless residuals.

## Small metadata distinction

The actual ThreadMetadataService (bundle439 around4272554) reads a thread-keyed object with known optional labelColor and loose unknown fields. Its semantic update removes only labelColor and deletes the exact object only if then empty; unknown fields are preserved. This is evidence that wholesale target subtree removal could discard unsupported fields and that a known small exact record has native record-level semantics. Host ambiguity of bare UUID records remains a separate proof obligation.

## Sandbox-check primary source

Local public SDK `/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/sandbox.h` has deprecated sandbox_init declarations but no sandbox_check/filter enum. WebKit's Apple-authored primary `Source/WTF/wtf/spi/darwin/SandboxSPI.h` defines external-SDK fallback enum: NONE0, PATH1, GLOBAL_NAME2, PREFERENCE_DOMAIN6, XPC_SERVICE_NAME12, IOKIT_CONNECTION13, SYSCALL_NUMBER14, and declares sandbox_check with a varargs filter parameter. SANDBOX_CHECK_NO_REPORT is an extern enum symbol, not a numeric constant to guess. Primary URL inspected: https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WTF/wtf/spi/darwin/SandboxSPI.h . The ABI query must use the right filter and require exactly the documented denial result, supplemented by actual descendant IO/socket canaries. Source declarations do not independently prove complete graphical confinement.

## Further activity closure

The exact persisted atom is `electron-persisted-atom-state.update-resume-state-v1` (early-bootstrap.js around224694). It contains accountId/userId/threadIds/expiresAtMs according to main's schema. Main consumes and clears it with a flushed global write; renderer only auto-resumes it for the matching ChatGPT account/user and a nonexpired timestamp. ApiKey-only acceptance cannot establish the ChatGPT update-resume/reminder branch.

Renderer active-chat candidates are validated by kN (app-runtime around99177): actual listThreadTurns must return a latest interrupted turn; old candidates also need recent actual readThread metadata. Promise.allSettled drops failed/missing candidates. This inspected reminder path therefore does not construct a full catalog entry from the activity ID alone. However current-profile absence must still be observed, and exact safely attributable records should be removed when support is established. Account/profile-qualified rows belonging to another principal remain protected.

## 2026-11-03 persistent writer closure

[ADR-0019](../adr/0019-structural-removal-and-profile-acceptance.md) now separates
persistent runtime verification from isolated profile restart acceptance. Pure
currently rendered presentation may be stale; a cached Map capable of persisting
old records is a writer/reconstruction surface and is not harmless ephemeral UI.

The exact static bundle revalidation is retained in
[persistent-contract-2026-11-03.json](../../compatibility/codex/complete-removal-research/persistent-contract-2026-11-03.json).
Nine Unicode-window SHA-256 commitments independently matched the installed ASAR.
Bundle339 characters150650–150900 contains the packaged/isMacOS/explicit-userData
singleton selector; characters132450–132900 contains its startup use. Default
packaged macOS launches bypass Electron singleton locking. An external singleton
holder therefore cannot fence default-root launches. Explicit isolated userData
uses singleton locking, but no external maintenance-holder protocol is validated.

Bundle133 characters12500–17000 loads the global Map and queues whole-Map writes
to current and backup without a reviewed cross-process CAS or stable lock. An app
launched after an absence check can read pre-cleanup data, wait on SQLite, then
repersist old records after cleanup. Process census plus native rollout/SQLite
locks is insufficient. No global/catalog/browser production mutation is admitted.
This is an OPEN implementation/evidence gap, not proof of provider impossibility.

Catalog authoritative removal increments only the exact host observation sequence,
conditionally records an exact removed scan tombstone if a scan/checkpoint exists,
deletes the exact host/thread row, and advances catalog revision only for a visible
removed row. Shared checkpoints store cursors/anchors, not catalog payloads; keep
them. The exact removed tombstone suppresses scan resurrection and may be required
non-projecting technical state. Never reset a whole host/catalog.

Pinned-content-tabs records have account/host/thread identity and related exact
presentation/order entries. A local-kind legacy custom-section item key is not
execution-host identity: the separate threadHostIds map can designate another
host. Native legacy pin resolution requires one unique successful host read.
The exact build 23880 bootstrap asset `.vite/build/early-bootstrap-439-a0f25ba32c465de1.js`
stores the supplied host ID verbatim in the project membership map (Unicode
offset 3634798). The pinned-thread migration identity combines that host ID
with its Codex home (offset 1956068). The local profile therefore binds the
membership map to `local` and the migration map to `local:<CODEX_HOME>`.
Legacy localStorage custom sections are imported on startup; cleaning global
atoms alone does not close that reconstruction route. Unknown or ambiguous target
legacy/browser memberships block complete-profile admission.
