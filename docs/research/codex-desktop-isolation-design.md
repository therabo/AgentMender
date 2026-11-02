# Kernel-isolated Desktop composite candidate: research design

Status: PRE-LAUNCH DESIGN, 2026-11-02. No Desktop/provider process has been launched by this investigation. This design makes no GUI acceptance claim. The source supports a plausible route; remaining questions are testable. Failure to establish confinement or visibility means evidence remains OPEN, not proof that the provider makes the product impossible.

## Exact candidate and evidence

Installed ChatGPT.app is the Codex-branded Owl/Electron bundle `com.openai.codex`, Desktop `26.1027.51404`, build `23880`. Its embedded Codex is `0.166.0-alpha.5.1`; it is not the stable M8.2 binary. Full archive and selected asset hashes are in projection-provenance.json. Info.plist ElectronAsarIntegrity is archive-header integrity, not the full archive SHA256.

The initial candidate should combine this exact unmodified signed bundle with an explicit private official stable `0.166.0` executable pin `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`. That is a new composite experimental profile, not an inference that the current M8.2 profile already covers Desktop. Read the executable identity only; do not invoke its real-root --version or substitute a newly downloaded version.

Read-only `codesign -d --entitlements :- /Applications/ChatGPT.app` shows app-sandbox=false, automation.apple-events=true, network.client=true, broad team keychain access, and app groups for notifications/CUA. These entitlements make a clean HOME alone inadequate. The harness must apply a stronger inherited kernel sandbox before any executable startup and explicitly prevent keychain/Apple-event/app-group/live IPC access. Entitlements are not permission to use those resources in the experiment.

## Source-supported isolation controls

Use direct subprocess execution of `/Applications/ChatGPT.app/Contents/MacOS/ChatGPT`, not `open`, LaunchServices, an existing app process, Apple Events, CUA, or a deep link. Supply a new private `CODEX_ELECTRON_USER_DATA_PATH` and matching `--user-data-dir`; initial code derives single-instance behavior from that path. An independent process must never forward its arguments to the real running app.

Use an environment constructed from an empty mapping. Set only the owned HOME, CODEX_HOME, TMPDIR, XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME, explicit userData, LANG, minimal trusted PATH, and the exact candidate CLI override. Clear all inherited credentials, proxy settings, real CODEX_SQLITE_HOME, cloud/websocket endpoints, app-tools/worker IPC paths, SSH agents, launchd/application-group variables, and runtime config overrides. App-generated writes remain limited by the kernel even if a path ignores these variables.

Set `CODEX_CLI_PATH` to the exact private stable executable and `CODEX_APP_SERVER_FORCE_CLI=1`. In `early-bootstrap-824-3c24dda6dba622ec.js`, the override resolver (characters 164000-167800) gives this path precedence and source=override; it is not a development-only override. The transport code (characters 158000-159700) only reuses a local daemon when use-local-daemon=1, force-cli is not 1, no CLI override is set, and an owned socket resolves. Keep use-local-daemon unset and force-cli=1. Also deny real daemon sockets at the kernel boundary. No provider process is started to inspect those settings.

Do not set `--no-sandbox`, disable signature checks, modify the bundle, enable auth mocks that replace actual GUI behavior, use real auth state, or permit real keychain access. Do not weaken the stable binary's expected producer/schema guards merely to load GUI fixtures.

## Fresh owned root layout

A new 0700 canonical non-symlink directory created with mkdtemp holds home/, codex/, electron-userData/, config/, cache/, data/, tmp/, project/, bin/, evidence/, and denied-canary siblings owned by the harness. Seed only synthetic UUIDs, titles, rollouts, global records and exact schemas. Never copy existing provider state, credentials, logs or renderer stores. Copy only pinned executable/signing resources when the existing exact official pinning contract requires them, not user payloads. All writable directories are on the owned root and remain disjoint from the production provider roots and AgentMender state.

## Kernel policy: required properties before launch

The actual Seatbelt policy must be independently reviewed and canary-tested. Do not treat the outline here as a validated sandbox profile. Start deny-by-default; admit only exact app/system runtime reads and owned-root reads/writes. No broad real-home file-read allowance is needed because the app is in /Applications and the binary is under the owned root. Explicitly deny all real-home content and provider-state reads/writes, user keychain and app-group data, outgoing/incoming network, Apple Events, task ports/other-process control, and named live Unix endpoints. Do not use a global allow-mach-lookup, allow-unix-socket, allow-network, allow-user-preferences or allow-real-home exception to make startup pass.

Electron/Owl needs process-exec/process-fork, graphics IPC and child IPC that the current CLI sandbox intentionally denies. Admit process execution only for the exact app binary, its exact identified helpers/framework executable paths, the pinned stable CLI and fixed test canary helper. Children must inherit the sandbox with no process-exec sandbox reset or unsandboxed launcher. Limit graphics Mach lookups to individually reviewed services actually needed for a new window (WindowServer is a candidate, not a preapproved allowlist). No securityd/keychain, AppleEvents, distributed notification, CUA, LaunchServices app launch, browser profile bridge or existing-app command socket is admissible merely because it is encountered. Each proposed allowlist addition gets a fresh denial audit; unrelated system service access is not implicitly safe.

A separate tiny canary helper runs first under exactly this policy, through the same sandbox-exec parent and child-spawn chain. It must demonstrate: owned file read/write works; an owned sentinel outside the writable boundary cannot be written; protected owned content outside the read boundary cannot be opened; real-home contents cannot be listed/read; TCP loopback and external connect/bind/listen fail; named Unix socket outside root fails; keychain/securityd Mach lookups and Apple Events fail. It must fork/exec the allowed helper and repeat the same denials in the descendant. Use synthetic sentinel contents only; never read actual user data to prove denial. Record errno/results and policy/hash, not credentials or transcripts. An exec-success canary is necessary because this experiment requires permitted children; a fork-denied CLI canary cannot prove the graphical policy.

Check protection without deliberately writing to production paths: use scratch protected sentinels and a statically audited no-real-home-allow policy. If policy validation or any canary fails, no app launch occurs. A subsequent run never retries unsandboxed. The app's native entitlements do not supersede the inherited deny policy, but that inheritance must be empirically demonstrated for exact child binaries before relying on it.

## Structured GUI observation route

The real installed preload exposes `window.electronBridge.getInitialSidebarBootstrap()`. Main's actual `getInitialSidebarBootstrap` (bundle439 characters 4147290-4151000) reads the actual catalog and supplies catalogEntries/catalogHostIds/globalStateEntries plus workspace state. It includes local host without requiring a cached authenticated principal. This is a plausible auth-free structured probe into the real UI-facing state. The preload memoizes the first returned snapshot; create a fresh renderer/window or app process for a fresh observation. Repeated calls in one renderer are not fresh verification.

The bootstrap is capped: selected/pinned/project keys plus small per-host pages, typically 10 entries each, within a fixed global bound. It cannot prove absence outside a cap. Seed fewer than that cap with distinct fixtures, prove every fixture appears before deletion, and use full structured paging for complete supported surfaces. `createAppHost` exposes `localThreadCatalog` (bundle439 character 4128553), with readStatus/readPage/readEntries. Its methods are the actual Desktop services; read-only invocation on the owned app can verify all catalog pages without inventing a replacement catalog.

A candidate observation transport is Chromium's inherited remote-debugging pipe, using parent-owned file descriptors rather than a TCP debug port. Whether the exact Owl build honors this switch is untested. If unavailable, do not enable a world-reachable debug socket. A harness could instead use the existing app-host MessageChannel RPC through the exact bundled serializer within the owned renderer. That protocol and an authentication-free catalog population path still require a bounded implementation review. Do not add arbitrary evaluate/JSON/SQL commands to production AgentMender. The research harness may inspect only its own owned renderer and fixed read-only service requests.

The preload bootstrap can establish that persistent entries drive the actual graphical process even behind a login view. It does NOT alone prove a baseline target is visible in the normal sidebar, search, recent, pinned, archive/all and project UI. Positive acceptance requires baseline structured UI views containing the target, then their absence after cleanup/reload/restart with unrelated positive sentinels. If auth-free startup never exposes those views, report that exact limitation and keep GUI acceptance pending. Do not copy real credentials or fabricate a logged-in principal to turn a hidden fixture into a GUI success claim.

## Staged experiment after design review

1. Validate canonical owned roots, exact app signature/identities, asset hashes, exact stable override pin, synthetic fixtures, and absence of inherited sensitive environment.
2. Compile/review the narrow Seatbelt policy and run parent+child denial canaries. Fail before launch on any mismatch or missing proof.
3. Launch only a new direct child in a dedicated process group under that policy. No open/deep-link/AppleEvent/real app IPC. Attach only an inherited owned observation pipe. Bound readiness and memory/time.
4. Establish its bundle, process tree, effective CODEX_HOME/userData/CLI identity via structured owned evidence; verify it did not route to an existing daemon or app process. Reject any path outside the approved owned read/write universe.
5. Query actual fresh preload and app-host read-only state. Record whether local synthetic target/unrelated/same-cwd/similar-UUID/project/pinned/recent/archive fixtures are actually visible. A login-only result is an informative negative feasibility result, not acceptance.
6. Gracefully close this test instance through its own supported window/app lifecycle. Verify every test-owned writer/helper has terminated. No production process termination and no generic killall/pkill. A timeout may terminate only the cryptographically/randomly tagged owned child process group after verifying ownership; this is test cleanup, not production coordination.
7. Only after reviewed projection mutation design exists, run the actual AgentMender production pipeline against that disposable exact fixture. Prepare/approve/consume existing DurableIntent; freeze all exact projection+artifact+coordination effects. Test harness cannot mint production authority or insert unapproved targets.
8. Relaunch the exact same isolated composite app with the same owned roots; recreate renderer; wait for actual native catalog/renderer readiness. Query all supported views plus retained positive controls. Repeat refresh/reindex and a second restart. Test malformed-main/valid-backup fallback, catalog checkpoint/scan and legacy pin migration as separate fixtures.
9. Crash cuts include before shutdown, after termination, before/after each projection publication, payload unlink, DB commit, before restart, during readiness, and after restart before final durable observation. No automatic retry after unknown effects. Reconciliation observes frozen state only.

## Feasibility questions that must be resolved empirically

- Which minimum graphics/child-IPC allowlist permits an unmodified signed Owl app without granting private brokers or real-user data access?
- Does the exact build support a parent-owned debugging pipe or another bounded fixed structured read probe?
- Can auth-free local mode populate and render all required views? The startup bootstrap has local catalog access statically, but normal UI visibility is untested.
- Does the Desktop work correctly with explicit stable0.166.0 override, rather than its embedded alpha? Equal schemas do not prove it.
- Can all old app/renderer/CLI/daemon writers be excluded and exact instance shutdown proved without touching the real application?
- What exact persisted atom/legacy browser-storage/migration records participate in reconstruction? Do main and .bak cleanup plus catalog mutation cover them under that exact profile?

These questions justify a bounded candidate experiment after review. They do not justify claiming provider impossibility, graphical support, or SessionRemoved now.
