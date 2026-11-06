# Desktop acceptance: bundle drift and remaining gates

2026-11-06. Verdict: **OPEN; no restart acceptance**.

Read-only inspection of `/Applications/ChatGPT.app/Contents/Info.plist` found
`com.openai.codex`, version `26.1027.61615`, build `24141`. SHA-256 of its main
executable is `b3783838c86c3124d4b0fd5fa5c2d2fca327b4b4f52c7c7c369df25cfa879509`,
of `Contents/Resources/app.asar` is
`bb88615052255e79b5977885f9a0cc212c268589587c79d4d3b7c4b46e881a95`,
and of `Contents/Info.plist` is
`fb72c75a6cabb64f05420aef782b2d6ac86269a6c5557a75dabfa796905ee6ef`.
The reviewed source and compiled candidate identify version `26.1027.51404`,
build `23880`, main `0c1cb804f8858c8ab059a26a1640b9a66c9a5071c48f39d6618e8698da370b56`,
asar `52e0a65f9fea51f4287bbd98c63343761f9424517bb793652bb554764bc0bcc8`,
and plist `7a3920d56d052cdf4bf803a92e2098be015eaf9263589d51e04ac4121f16eed7`.
A filename search bounded to `/tmp/agentmender*`, `/Applications` and
`~/Downloads` found no preserved matching application or installer. The official
[Codex download page](https://openai.com/codex/) offers a download; a versioned
package for build 23880 was not established by this search. This is not proof
that no such package exists. The installed app was not launched, modified, or
replaced. Requalifying the new build requires a new exact source/schema and
safety review; version similarity does not import acceptance.

Follow-up located the [official versioned arm64 archive](https://persistent.oaistatic.com/codex-app-prod/ChatGPT-darwin-arm64-26.1027.51404.zip).
The 764974997-byte archive was recovered into private scratch space. Its SHA-256
is `93993beed5ee52a61147ca973290e2052bb0fa3da117634d571267071720bb5d`.
The extracted bundle passed strict recursive code-signature verification with
the Apple anchor, team `2DC432GLL2` and identifier `com.openai.codex`. The
plist, executable, ASAR and both embedded CLI hashes matched the historical
pins. The separately pinned stable CLI was also recovered and verified. Catalog
schema, ordered migrations and the scan suppression excerpt reproduced byte
for byte from the recovered bundle. These checks were static. The installed
profile remains build 24141 and graphical reconstruction acceptance is pending.

A fresh run of the existing synthetic `isolation/run.py` canary succeeded in
parent and exec child under a newly generated default-deny profile, including
owned read/write, synthetic protected read/write denial and loopback/Unix socket
denial. Receipt: `/tmp/agentmender-m821-research/isolation/result-m821-ng8k2fl1.json`;
policy SHA-256 `da855caae9b1f4c64b8b47b22995da671c573af2bdc79cb28aec74d0c617a66a`.
Broker, real-home and Apple Event observations were permission queries, not
actual access attempts. This canary permits only its helper executable: it does
not establish graphical startup, application-child inheritance, complete broker
coverage, or acceptance. The previously reviewed main-only startup exited
SIGTRAP, and the broader helper policy remains unreviewed for startup.

## Repeatable acceptance gates

1. Obtain the exact reviewed signed application and pinned CLI without replacing
   the installed app, or separately review a new exact candidate. Verify
   signature, executable/asar/plist and helper identities, schemas and resource
   inventory immediately before each run; reject any mismatch.
2. Independently review the actual default-deny policy, explicit helper/CLI and
   graphics allowances, restricted environment, descriptor setup, and owned
   descendant supervision. Verify inherited confinement in parent and exec
   descendants, including owned synthetic protected paths, network, named Unix
   endpoints and denied brokers. Failure blocks launch; never retry unsandboxed.
3. Use fresh disjoint owned HOME, CODEX_HOME, Electron userData and workspace.
   Seed exact-profile synthetic target and unrelated, same-cwd, similar-ID,
   same-project, pinned, recent, archived and ambiguous sentinels. Establish
   baseline positive structured visibility, including every applicable persistent
   projection, before cleanup. No real credentials or provider data.
4. Invoke actual discovery, frozen manifest, transaction, independent terminal
   approval, consumed DurableIntent, production adapter and structural
   verification. Stop on drift, writer detection or unknown outcome.
5. Start and normally close only the verified owned process tree; repeat for two
   independent reconstruction cycles. Each fresh cycle must observe target
   absence and all positive sentinels retained, including backup fallback,
   checkpoint/scan and legacy migration challenges. Observe all relevant pages,
   not merely the capped bootstrap. Retain evidence bound to the exact profile,
   fixture, approval and actual execution. Missing observation prevents PASS.

The source-supported candidate read API is the actual preload
`window.electronBridge.getInitialSidebarBootstrap()` and actual app-host
`localThreadCatalog.readStatus/readPage/readEntries`. Bootstrap alone is capped
and memoized, so it cannot prove exhaustive absence or freshness; use a fresh
renderer/process and complete structured paging. A parent-owned debugging pipe
or equivalent narrowly reviewed transport remains unimplemented and unproven.
Neither a synthetic substitute nor direct database inspection alone proves
actual service reconstruction. Production never starts or stops the user's app.
