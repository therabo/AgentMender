# Codex compatibility evidence

## P0-SR4 read-only discovery

Desktop 26.1103.60933 (build 25894) with CLI 0.167.0-alpha.3.2 loaded
`session_discover` after an AgentMender connection restart. The live
root-thread mapping excluded the independently identified current thread
and preserved unrelated compressed history and context. The supported
shape is a canonical `threadId` with an absent or equal `sessionId`, plus
an exact row in the configured Codex root. Discordant metadata and unknown
roots refuse. Management authentication and lifecycle admission remain separate.
[Acceptance](../../docs/milestones/p0-sr4-implicit-caller-discovery.md).

## P0-SR3 observed read compatibility

Desktop26.1103.60933/build25894 and CLI0.167.0-alpha.3.2 retain validated read-only
metadata and plain/Zstandard legacy/paginated sources. The five67.5–148MB compressed
sources and exact330-session cohort reach physical EOF; all logical histories stay
Partial. No mutation profile transfers to this Desktop/CLI. Updated-parent
resident traversal and default management refusal passed after the
[SR3 live refresh](../../docs/milestones/p0-sr3-live-mcp-acceptance.md).
Authenticated caller routing remains OPEN.

## P0-SR2 current history representation

Desktop26.1103.41825/build25679, CLI0.167.0-alpha.3.2: all four nominated missing
plain paths have exact compressed counterparts. Their visible messages now pass
AgentMender MCP search/context. The reader supports bounded `.jsonl.zst` legacy
and paginated inline records within the existing kernel isolation; native RPC
startup and direct history-projection SQL remain unadmitted. Partial coverage,
replay limits and caller-binding refusal remain. See [source evidence and
limitations](../../docs/native-history.md) and [SR2 acceptance](../../docs/milestones/p0-sr2-native-history.md).

## P0-SR1 current read reliability

CLI 0.167.0-alpha.3.2 is observed separately from historical mutation profiles. Missing backing sources are source_missing rather than format incompatibility. Fixed active/archive counterparts stay within existing roots and require UUID/physical validation. Paginated inline display items remain Partial. Documented non-resuming thread/read, thread/turns/list and thread/items/list do not alone establish write-denied startup or a trusted existing-client transport. No credentials are copied and no client is impersonated. Native coverage and caller routing stay OPEN. See [current status](../../docs/milestones/p0-sr1-search-reliability.md).

Current read-only retrieval and host-binding limits are documented in
[the search contract](../../docs/session-search.md). Content reads do not expand
any approval, mutation profile or recovery authority. Real updated client and
management-binding acceptance remain OPEN.

## Current real metadata versus mutation compatibility

Intelligent read-only retrieval extends P0-S1 with sequential supported transcript
traversal, structured queries, context and optional private FTS5 caching. See the
[implementation report](../../docs/milestones/intelligent-search-implementation.md)
for separate fixture, real-provider and actual-client acceptance. Automatic caller
binding remains unavailable; production file-only declarations cannot admit it.
Guaranteed management requests refuse with zero candidates. Explicit diagnostic
protections remain separate from caller authentication and mutation admission.

`codex-state-threads-columns-v1` validates required metadata columns/values inside
the protected read-only worker. Additive columns and unknown producer versions
can be readable without a binary-hash admission list. Its bounded selected-root
coverage and parent relationships remain partial. Installed native method schemas
are capability observations only. Current [alpha.6 evidence](0.166.0-alpha.6/real-world-readonly/evidence.json)
records actual metadata reads and renewed native descendant/partial-error limits.
No current native delete or raw mutation profile is admitted. The exact stable
fallback, older App Server discovery and advanced artifact contracts below remain
independent; a schema/binary match on one axis does not transfer another's authority.


## P0-C1 compatibility axes

Historical P0-C1 removal composition uses the exact stable0.166.0 metadata
profile through the protected worker. Current read-only search separately admits
validated additive metadata without transferring that destructive profile.
Readability never admits Desktop alpha.5.1/24141 or unknown producers for effects. Preview/execution retain scoped-cleanup-v1's exact binary,
Legacy/WAL/config/physical/dependency/writer gates; search eligibility is no grant.
Native delete remains unsupported. Human approval/execution remain external and
full actual-client/human acceptance remains OPEN. See
[the workflow](../../docs/conversational-codex-removal.md) and
[ADR-0023](../../docs/adr/0023-conversational-primary-removal-handoff.md).


Current Desktop schema-4 evidence is in the [structural checkpoint](../../docs/milestones/m8-2-1-writer-admission-2026-11-07.md).

## Product-first support axes

The [roadmap](../../docs/product-roadmap.md) separates exact read-only discovery,
native lifecycle, scoped primary cleanup and optional Desktop/deep cleanup.
A fingerprint recognized on one axis grants no authority on another. Native-first
selection requires exact operation scope, not merely a matching protocol schema.
The first conversational slice must address the gap between historical inventory
profiles and the stable 0.166.0 primary-cleanup profile. See the
[audit matrix](../../docs/audits/2026-11-07-product-first-audit.md).

## Deferred M8.2.1 complete-removal compatibility

M8.2 primary cleanup is PASS and [integrated](../../docs/milestones/m8-2-integration.md)
at a284767157ace8769deee85b195b5b70efe374d8. M8.2.1 is unfinished/deferred, no
longer a global P0 blocker. Its stronger complete-removal claim remains governed by
[ADR-0019](../../docs/adr/0019-structural-removal-and-profile-acceptance.md), with zero admitted
complete graphical removal profiles. The [installed 24141 candidate](desktop-complete-removal-24141-v1/README.md)
and [recovered 23880 candidate](desktop-complete-removal-v1/catalog/README.md)
are separate. Both embed CLI 0.166.0-alpha.5.1. A stable 0.166.0 primary worker
override still requires exact composite acceptance.
Exact static projection/source hashes, runtime evidence gaps, app coordination and
restart acceptance must be considered independently. Historical v1 pins/receipts
below remain primary-cleanup evidence and do not promise UI disappearance.

## Expanded primary profile

[scoped-cleanup-v2](scoped-cleanup-v2/README.md) is separately reviewed schema-3
primary cleanup with exact attachment/guardian metadata actions. Old v2 plans and
grants cannot reach it. It is not a graphical profile. Current removal authority
is structural persistent state under [ADR-0019](../../docs/adr/0019-structural-removal-and-profile-acceptance.md);
exact persistent-source closure governs admission. Graphical acceptance is manual
under [ADR-0021](../../docs/adr/0021-structural-admission-and-manual-desktop-acceptance.md).

## M8.2: separate scoped-local compatibility axis

M8.1 is formally integrated at `6f798bcebdfd2f763e31d62ef2695c52396ca709`
([receipt](../../docs/milestones/m8-1-integration.md), recorded in `c9dd04d`).
M8.2 local verification, independent code/security review and exact-head three-OS
CI passed before formal integration. ADR-0017 accepts a separate registered
[scoped-cleanup-v1 profile](scoped-cleanup-v1/README.md). Historical read-only and
exact native lifecycle profiles remain unchanged; source/schema equality cannot
transfer authority.

The local profile is exact 0.166.0 Darwin arm64 with a plain Legacy rollout,
selected root-local exact WAL schema, existing safe WAL/SHM and no root-local
thread-history DB. Native writer/dependency and actual SQLite main-file exclusive
fences protect the verified cutoff. Unknown primary identity blocks cleanup;
unrelated unknown stores remain protected residuals. See
[scoped scope/status](../../docs/scoped-session-cleanup.md).

The separate [0.166.0 M8.1 investigation](0.166.0/lifecycle-m8-1/README.md) retains
exact release/binary/schema/native evidence without enabling a production profile.
Full cleanup remains BLOCKED BY PROVIDER; changed unarchive cascades do not inherit
old target-only authority. Historical evidence below is unchanged.

M8 separately validates a conditional unarchive dispatch profile; read-only
compatibility below is never mutation authority. See [exact native evidence and
counterexamples](0.165.0-alpha.1.2/lifecycle-m8/README.md) and
[the execution boundary](../../docs/codex-lifecycle.md). Archive/delete remain
execution-unsupported and current real planning effects remain unclosed.

Exact executable observed on 2026-10-28: `codex-cli 0.164.0-alpha.11.1`.
No stable release or version range is inferred from this prerelease.

Primary protocol documentation: [official App Server documentation](https://learn.chatgpt.com/docs/app-server), reached from https://developers.openai.com/codex/app-server.
The local version-specific schemas are the authority for this exact build. The
current documentation can describe fields absent from this generated build.

The checked-in schemas were generated with this executable's command:

```text
codex app-server generate-json-schema --out <temporary-directory>/schemas
```

The child received fresh, empty temporary `HOME` and `CODEX_HOME` directories,
a minimal environment, and a temporary working directory. This command did not
access real session records or credentials. The schema files contain protocol
definitions only, without user data. Reproduce generation in an isolated child
environment; never point schema experiments at a live provider home.

| File | SHA-256 |
| --- | --- |
| v1/InitializeParams.json | b55124e7c0f36a1789396daa5578217a92be67f053903696236686e59f6f212e |
| v1/InitializeResponse.json | 62ad689c2cb6379913c1d72749cfd8de5089d35760214123518eb92eef11acc9 |
| v2/ThreadListParams.json | e9da2e0bf099d21ab7982210073f927a50370c4e28be7140c314f82c524070bd |
| v2/ThreadListResponse.json | 215b560c2a27998088992b3b0521f42a58b4230fbad276b8ce19872c0add68ab |

Initialize uses `clientInfo` with name and version; title is optional. The stable
surface is selected with `capabilities.experimentalApi:false`. Requests and
responses are newline-delimited JSON without a `jsonrpc` field. An `initialized`
notification follows successful initialization. Only `initialize`, `initialized`,
and `thread/list` are emitted by this adapter. Unexpected server requests cause
failure and are never executed.

Every list request sets `useStateDbOnly:true`: omitted or false triggers JSONL
scan-and-repair according to both the schema and official documentation. This
flag is not a filesystem permission boundary. All ten source kinds are explicit:
`cli`, `vscode`, `exec`, `appServer`, `subAgent`, `subAgentReview`,
`subAgentCompact`, `subAgentThreadSpawn`, `subAgentOther`, `unknown`.
Omitted or empty source filters include only interactive sources. Active and
archived lists are separate. Cursors are opaque and repeated cursors terminate
discovery as incomplete.

## Real disposable-snapshot evidence

The earlier global write-denial experiment correctly rejected App Server startup:
Codex requires writable runtime state. ADR-0003 supersedes that restriction with
an isolated, disposable evidence home while keeping real source state write-denied.

**OBSERVED LOCALLY, exact build:** SQLite Backup API captures only state_5.sqlite
from a read-only connection under OS enforcement, including committed WAL state.
The source is first copied into RAM. Its 65-entry schema must match
[state-schema-5.json](state-schema-5.json), compact SHA-256
`5a7a23f469f17c68c9713f62a02a0ec97868e232a08cdff38881d922ea58ab4b`.
Unknown/corrupt state is rejected. Content-bearing tables are cleared, text is
redacted with NULL/empty predicates preserved, and secure_delete/VACUUM precede
first file persistence. Leading-NUL content is included in privacy regressions.
The scrubbed database is 0600 inside a fresh 0700 owned directory; no credentials,
config, rollouts, history database or log database are copied.

The fixed internal capture worker verifies kernel source-write denial before
SQLite access. A filesystem canary supplements that query; ordinary permissions
alone cannot establish sandboxing. App Server writes only in the owned home,
reads no real-home file contents, and can stat metadata. Network, forks and Mach
lookup are denied. Children are reaped before the owned home is dropped. Full
real-home read denial was tested and silently produced zero native summaries,
so metadata access is a demonstrated requirement for this build.

Recorded production verification:

| Observation | Count |
|---|---:|
| Source snapshot rows | 211 active, 2 archived |
| Nonempty previews | 162 active, 2 archived |
| Official native summaries | 164 |
| Observed cwd groups | 57 |
| Bounded filesystem artifacts | 8,499 |

The investigative run separately found 51 rollout files and 49 database paths
pointing to existing files; these were not sampled atomically with the final run.
Database rows, provider summaries and filesystem artifacts have different scopes.
Do not call their differences orphan/integrity findings or infer resumability.
The listing matched the nonempty-preview counts in this observation; this is not
a complete reverse-engineered filtering contract. Current upstream source has
path checks whose exact behavior differs from this installed prerelease.

Source DB/WAL/SHM SHA-256 and inode/size/mtime/ctime/mode matched before and after
one production providers/scan/inspect observation. A later repeat saw 212 active
rows and changes in all three monitored files; discovery correctly downgraded to
RealPartial with SnapshotStale/ConcurrentChange instead of claiming unchanged
source state. Atime was excluded. Independent activity changed row counts between
experiments; these fingerprints alone cannot attribute a concurrent writer.
A successful coherent snapshot does not
prove cross-store atomicity or current liveness; source changes produce explicit
SnapshotStale/ConcurrentChange diagnostics. Coverage remains Partial, independently
of SnapshotVerified evidence status. Cwd-derived project grouping is heuristic
when no provider-native project assignment exists.

## Fixtures and reproduction

`fixtures/codex/observed-snapshot` is a sanitized five-record normalized projection
from verified native enumeration. IDs, cwd paths and times are replaced; prompts,
previews and other content are excluded. It is FixtureVerified, never live evidence
or a complete raw App Server response. Other fixtures remain SyntheticOnly.

Explicit local tools (never normal CI):

- scripts/verify_live_discovery.py compares conservative source fingerprints around
  providers/scan/inspect and prints aggregates only.
- scripts/capture_codex_fixture.py creates a **new** sanitized sample directory from
  SnapshotVerified discovery. It refuses output within known provider roots.

The native metadata projection ignores titles/previews. Parent references are
literal metadata only, and native temporary-process status cannot establish
activity in another client. Unknown versions/layouts remain protected audit-only.
No lifecycle operation, direct source SQL mutation or recovery mechanism exists.

## Exact 0.165.0-alpha.1.2 profile

An independent local investigation on 2026-10-28 used the same kernel-attested
sandbox and bounded read-only SQLite Backup API capture. Its separately retained
[generated schemas and source schema](0.165.0-alpha.1.2/) belong only to this exact
prerelease. InitializeParams, InitializeResponse, and ThreadListParams match the
historical generated files; ThreadListResponse adds only the nested `modelNotFound`
error variant (SHA-256 `1c346f3a9174a6d68df79ea72279f7079681cf0afdb709afab818edbf355cfab`).
The 66-entry source schema has compact SHA-256
`1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`.
Its only changes are nullable `threads.attached_thread_parent_id` and its index;
the identifier is retained as opaque metadata without relationship inference.
Each compiled profile binds its exact version, interface and source schema. The
source hash is checked before scrubbing and VACUUM; the disposable mirror's
VACUUM removes `sqlite_sequence`, so the mirror need not have the source hash.

The first isolated observation returned 166 summaries over eleven pages, with
source DB/WAL/SHM fingerprints stable across capture and enumeration. A second
observation sampled the five sanitized records in
[the new fixture](../../fixtures/codex/observed-snapshot-0.165.0-alpha.1.2/),
whose [provenance](0.165.0-alpha.1.2/provenance.json) records changed WAL/SHM and
RealPartial/SnapshotStale. The coherent SQLite snapshot does not imply cross-store
atomicity. Unselected native response fields are discarded; the entire response is
not claimed content-free. Source status is presentation metadata, not evidence of
writer liveness. The normalized projection and the existing scope limits remain
unchanged. Unknown versions and mismatched schemas remain metadata-only.

## Lifecycle protocol evidence for 0.165.0-alpha.1.2

The separately captured [lifecycle schemas and evidence summary](0.165.0-alpha.1.2/lifecycle/) were generated in an OS-isolated temporary home with writes denied outside that owned directory. Their canonical protocol evidence object has SHA-256 `14a6e75b4bae9ec7b4bb41f3ed9840296f4d671295f300532efc7a7ca9b3d686`. This establishes protocol shapes only; no lifecycle operation was invoked. The older discovery profile has no corresponding lifecycle evidence.

The [official App Server documentation](https://learn.chatgpt.com/docs/app-server) describes archive as attempting spawned descendants, including success despite a failed child; delete covers the target and spawned descendants; unarchive restores the target with provider-managed state and is not a complete inverse. This current documentation does not verify exact runtime effect closure for the captured version. Accordingly, descriptors carry conservative semantic scopes and production emits no exhaustive effect boundary. Discovery remains partial and integrity lacks a closed contract, so current plans remain blocked. A future native transition may change provider-managed global database metadata; retained logical state is not a byte-identical database guarantee. Physical reclaim remains unknown.
