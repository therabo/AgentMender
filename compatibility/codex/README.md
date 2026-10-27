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

Physical macOS experiment: `/usr/bin/sandbox-exec` with
`(version 1)(allow default)(deny file-write*)(deny network*)` denied a fresh
fixture file-write canary and a loopback connection with EPERM. App Server under
the same profile and an empty fixture `CODEX_HOME` exited 1 before initialization:
`failed to initialize sqlite state runtime`. It emitted no protocol output and
created no fixture files. The production process helper applies at least these
write and network denials; the adapter never retries with relaxed permissions.
A usable live API read is therefore **not verified** for this installed version.
No live App Server attempt against real state was needed for this evidence.

M1 outcome: typed synthetic protocol fixture discovery is supported for the exact
schema above. Live startup failure produces `CapabilityUnavailable` diagnostics;
regular-file metadata remains visible and every artifact is protected. There is
no direct SQLite or rollout parser, no metadata repair, no temporary state-copy
infrastructure, no mutation endpoint, and no successful live session-discovery
claim. Unknown versions remain metadata audit only.

The `fixtures/codex/*/manifest.json` responses are handwritten synthetic metadata
following this generated schema. They are not captures of user threads and do
not establish a locally installed executable version. Titles and previews are
ignored by the normalized domain projection. Parent references are metadata
only; a referenced parent may be outside the discovery scope. Runtime status
belongs to the temporary App Server process and does not establish activity in
another running client. Snapshot coverage remains partial across provider stores.
