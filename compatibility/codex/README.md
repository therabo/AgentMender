# Codex compatibility evidence

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
