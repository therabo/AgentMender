# Claude installation and metadata evidence

## Actual installation finding

**OBSERVED LOCALLY, 2026-10-28:** Claude Desktop exists at
`/Applications/Claude.app`, bundle ID com.anthropic.claudefordesktop, version 4.29.0.
Desktop owns a separate Code binary at
`~/Library/Application Support/Claude/claude-code/2.1.222/claude.app/Contents/MacOS/claude`,
bundle ID com.anthropic.claude-code. A write/network-denied --version probe returned
`2.1.222 (Claude Code)`. No standalone PATH CLI was detected in the bounded search.
Desktop's independent Code installation is documented by Anthropic.

The previous PATH-only resolver missed that binary and incorrectly generalized
its CLI search miss into provider absence. Detection now records Desktop and
BundledCli separately, with distinct BundleMetadata/ExecutableProbe version
sources. A candidate is not presumed active; multiple viable Code candidates are
reported without choosing the newest. Unknown/conflicting Desktop versions do not
establish the observed metadata context. No reinstall, upgrade, shell or global
PATH change was performed.

## Read-only discovery contract

**DOCUMENTED:** Official structured local history helpers exist in the Python and
TypeScript SDKs. The Rust runtime does not execute a bridge or start a conversation
for inventory. CLI JSON conversation output is not a past-session inventory API.

**OBSERVED LOCALLY:** The narrow Desktop sidecar shape is
`claude-code-sessions/<opaque-uuid>/<opaque-uuid>/local_<uuid>.json`. Namespace
semantics are unknown. Both components are retained in identity; unrelated
scheduled-task files are excluded. Reads are bounded to 64 KiB per file and a
caller entry budget, reject links and duplicate recognized fields, and ignore
private/unknown values. The whitelist is sessionId, cliSessionId, cwd, originCwd,
createdAt, lastActivityAt, lastFocusedAt, isArchived and forkedFromSessionId.

Seven sidecars exposed explicit mappings to seven of eleven root transcript
locators. Only one Desktop UUID suffix equaled its CLI ID: mapping **must** use
cliSessionId. All seven archive booleans were false; this means unarchived
presentation, not a quiescent writer. One fork references another Desktop session
in its namespace; it creates References, never SpawnedBy. Missing archive status
stays Unknown. Cwd values are project/workspace observations, not ownership proof.

Historical sidecars contain no producer-version field. Their structural profile
was observed alongside Desktop 4.29.0; that current app version does not attest to
historical producers. Bounded investigative transcript metadata showed six older
Code producer labels 2.1.202/205/209/217/219/222. The live adapter does not parse
those transcript bodies. Eight subagent metadata files had no parentAgentId, so
no immediate parent relation is inferred. Four additional roots and other
histories remain unattributed protected artifacts.

**Production verification:** seven sessions, one cwd group, seven explicit CLI
mappings and one fork reference were returned, with RealPartial evidence and
Partial coverage. The 51 monitored entries (including both root directories) and
seven parsed metadata-file hashes matched before/after providers/scan/inspect;
atime was excluded. This is not a bit-for-bit global state or health claim.

## Fixtures and limits

`fixtures/claude/4.29.0-observed` contains sanitized metadata and empty locator
representations, **not transcript captures**. IDs/paths/times are replaced; no
prompts, titles, summaries, config, tokens or real account IDs are included.
Loading it produces FixtureVerified evidence, never live RealVerified evidence.
The older 2.1.311 fixture remains SyntheticOnly and never validates a real release.

Known metadata context yields ReadOnlyKnown independently from RealPartial
provenance and Partial coverage. New/unknown contexts receive metadata-only audit.
An explicit CLI state root disables default Desktop reads unless a separate
Desktop root is explicitly configured. Native process isolation is currently
verified only on macOS; other platforms remain explicit about unsupported probes.

Official sources checked:

- [Desktop's independent Code version](https://code.claude.com/docs/en/desktop#claude-code-version-in-the-code-tab)
- [SDK Python local history](https://code.claude.com/docs/en/agent-sdk/python#list_sessions)
- [SDK TypeScript](https://code.claude.com/docs/en/agent-sdk/typescript)
- [Transcript implementation-detail warning](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)

No mutation, unsupported lifecycle command, provider SQL write, ownership solver
or M2 algorithm is implemented.
