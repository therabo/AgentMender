# M9 current local Claude investigation: BLOCKED

Observed 2026-10-30. This is categorical investigation evidence, **not an enabled
compatibility profile or execution authority**. No Claude mutation adapter was
added. All project/session archive, restore and delete operations remain
Unsupported. [ADR-0015](../../../../docs/adr/0015-current-claude-purge-provider-blockers.md)
preserves ADR-0014's safety boundary and changes only the milestone assessment.
No M9 merge or M10 work is authorized by these results.

## Exact releases and provenance

The official [latest non-prerelease release](https://github.com/anthropics/claude-code/releases/tag/v2.1.314)
is **2.1.314**, published 2026-10-30. The official distribution's separately named
delayed `stable` channel serves **2.1.308**. The [setup documentation](https://code.claude.com/docs/en/setup#configure-release-channel)
explains this channel distinction. Both exact binaries were investigated separately;
neither replaced the installed Desktop 4.29.0 / bundled Code 2.1.222.

| Candidate | Distribution | Bytes | SHA-256 |
|---|---|---:|---|
| 2.1.314 | Official native darwin-arm64, latest | 278848096 | `21d6e7b9a6f07b4cfcb7df13760b3b72aa25e084e13a5354413382987ba832ed` |
| 2.1.308 | Official native darwin-arm64, delayed stable | 274675680 | `ca923c86ac84b30574a82331a05b6e066c27477a926ceaffe46a845a33e4dd37` |

[Provenance](provenance.json) records official URLs, source commitments, manifest
metadata and signatures. Downloads remained in private AgentMender temporary
storage. The official installer was read, never executed. Each binary matched its
signed manifest and passed strict macOS code-signature verification with Anthropic
team `Q6L2SF6YDW`, identifier `com.anthropic.claude-code`. Manifest RSA/SHA512
verification used a narrow OpenPGP-v4 extractor and existing OpenSSL, binding the
official documented fingerprint `31DDDE24DDFAB679F42D7BD2BAA929FF1A7ECACE`.
This was not a general OpenPGP certification/trustdb or independent notarization
check. Public manifests, detached signatures and release key are retained here;
no executable or raw user history is committed.

## Documentation, release claims and native observations

The current [directory documentation](https://code.claude.com/docs/en/claude-directory#clear-local-data)
describes path-selected local purge and retained data. Documentation does not
close the exact binary's effects. The official
[changelog](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md) was
reviewed from 2.1.223 through 2.1.314 (78 published sections): long-path handling
2.1.224; custom project-directory name 2.1.234; rename to `purge` 2.1.288;
continue/list failures/exit 1 on deletion errors 2.1.293; history separators
2.1.296; separator-bearing subfolders 2.1.300; local-only wording 2.1.311.
These are release claims until independently probed. No version bisect or broad
minimum-version authority was inferred.

Exact native `--version`, general help and purge help are captured as bounded
output and commitments in [2.1.314 help](2.1.314-native-help.json),
[2.1.308 help](2.1.308-native-help.json) and [native evidence](native-evidence.json). The current callable
candidate is `--bare purge <generated-path> --dry-run|--yes`. No session identifier
or project ID is exposed by that selector. Research fixtures are generated input,
not captured authentic histories or production acceptance tests.

## Cross-version comparison

The 2.1.222 column preserves the [historical record](../evidence.json). That older
reproducer reused mutated state within several cases, limiting phase attribution.
The new suite gives every sequential invocation fresh HOME/config/cwd/transcripts;
only the explicitly labeled concurrency case shares one newly generated fixture.

| Property | Installed 2.1.222 | Latest 2.1.314 | Stable channel 2.1.308 |
|---|---|---|---|
| Dry-run read-only | No: config writes | No: config writes | No: config writes |
| Backup during dry-run | Created | Created | Created |
| `a-b` / `a/b` selector collision | Both transcripts removed | Both removed | Both removed |
| Parent/nested cascade | Child transcript/history effects; child config retained | Same counterexample | Same counterexample |
| Denied first removal | Exit 0, no removals in historical case | Exit 1; other removals continue | Exit 1; other removals continue |
| Denied later removal | Exit 0 after six sidecar-file removals in historical nested case | Exit 1 after partial sidecar removal | Exit 1 after partial sidecar removal |
| Complete effects closure | Unproven | Unproven | Unproven |
| Lossless AgentMender project target | Unproven/non-injective selector | Non-injective selector | Non-injective selector |
| Authoritative negative proof | Unavailable | Not established | Not established |
| Complete read-only reconciliation | Unavailable | Not established | Not established |
| Production readiness | Unsupported | Unsupported | Unsupported |

The basic denial rows use independent one-target-session fixtures. Additional
fresh nested-denial controls exercise the historical shape separately. Historical
late-denial followed earlier invocations and used two nested sessions; its six
removals must not be compared directly with a basic case's count. Current native
error reporting improved, while partial effects remain.

## Identity and matching

Current 2.1.314 tests cover relative/absolute selection, `.`/`..`, trailing slash,
realpath through a symlink, case and Unicode aliases, nested and prefix projects,
encoded-only state and inconsistent transcript cwd. Case/Unicode results apply only
to the tested APFS volume, not all filesystems/platforms. A fresh `a-b` / `a/b`
counterexample already disproves injectivity. An unrelated bucket containing the
same transcript cwd survives: cwd does not provide exhaustive membership.

AgentMender's observed Project still has no native `provider_project_id`; its
cwd-based grouping remains Heuristic. Configuration entries, encoded bucket keys
and realpaths cannot be promoted to destructive identity by assumption. Custom
names and long-path cases are research only, never a newly manufactured native ID.

## Effects and retained state

This table classifies sampled effects; it is explicitly **not exhaustive closure**.
Detailed differences and raw-report commitments are in native-evidence.json.

| Namespace | Observed current-version classification |
|---|---|
| Selected project transcripts, memory, subagent/tool-results/MCP-task sentinels inside bucket | Direct bucket deletion |
| Colliding bucket's other transcripts/session stores | Cascaded effect beyond one path-derived project |
| Nested project's transcripts/session stores/history | Cascade; nested config entry retained |
| Session tasks/debug/file-history | Direct removal; can precede a later failure |
| `history.jsonl` | Shared/global file filtered; unrelated entries retained in sampled runs |
| `.claude.json` | Shared/global startup/config writes; target entry removed by apply |
| Backups | Created even on dry-run/no-state; existing backup sentinel retained in sampled runs; rotation remains relevant |
| Plans, shell snapshots, settings, plugin/cache/global sentinels, usage/feedback/jobs/session metadata outside selected bucket | Retained in sampled runs; unknown exhaustive semantics |
| Generated source files, unrelated and prefix project state | Retained in sampled runs |
| Generated external temp image/scratch sentinels | Retained; no claim that sentinels enumerate actual complete provider temp state |
| Unknown categories, concurrent writers, system-managed/remote state | Unclosed; no authority inferred |

Immediate before/after fingerprints cover every generated file, directory and
symlink without following links, excluding atime. They distinguish expected
observed changes and unexpected changes in that finite fixture. They do not create
a provider-authoritative coherent baseline or prove absence of other namespaces.

## Failure, verification and reconciliation

No-state purge exits 1 yet can create config/backup state. Config/history denial
can follow destructive effects. Controlled signals distinguish an unchanged early
sample, startup-only writes and interrupted partial removal; elapsed timing does
not establish a stable dispatch phase. Two concurrent purges can both exit 0;
this supplies no serialization or writer-fencing guarantee. Timeout/response loss
after possible startup remains OutcomeUnknown. No automatic retry is safe.

Applied requires exact native identity, complete coherent absence in every declared
target/cascade namespace, fresh retained-state comparison and valid integrity.
NotApplied/NoEffect requires positive nondispatch proof before writable startup,
or an independently validated equivalent complete no-effect certificate. Surviving
files, exit 1 or an unchanged finite sample are insufficient after possible startup.
Every unresolved started-process result remains Unknown/RecoveryRequired.

[Verification research](verification.md) reviews official local SDK helpers at an
exact source commit: transcript listing/lookup can omit existing unreadable,
filtered or metadata-only files and omits other purge namespaces. Empty output is
not authoritative absence. No complete local read-only reconciliation contract
was established. Cloud Managed Agents session APIs are a separate product.
Reconciliation must only observe; it cannot rerun purge, writing dry-run or repair.
Purge remains NotCompensatable; backups are not a validated inverse.

## Isolation, reproduction and gates

The [separate opt-in reproducer](../../../../scripts/probe_claude_current_lifecycle.py)
accepts only private temporary artifacts matching the two exact pins. It accepts
no user-state root, project selector, account, free-form command or argument.
macOS Seatbelt denies writes outside owned evidence storage, all real-home file
contents, network, forks and Mach lookups. Owned canaries attest denial. Environment
is cleared, credentials absent, stdin closed, output/time bounded, children reaped.
There is no relaxed or unsandboxed retry. Both downloaded binaries are rehashed.

```sh
python3 scripts/probe_claude_current_lifecycle.py --artifact-root /private/tmp/agentmender-OWNED-DOWNLOADS --version 2.1.314 --allow-isolated-native
```

Use `--version 2.1.308 --core-only` for the separate delayed-stable control. The root must be
private, owned, canonical and contain the exact pinned artifact basename. Acquisition
and provenance verification are separate preparation, not an installer. The
downloaded executable must be an owned regular single-link file with write bits
removed; the script rejects writable artifacts and symlinked ancestors.
Native probe counts, independent case counts, intentional interruption outcomes
and commitments are recorded in native-evidence.json. Ordinary CI needs no Claude
installation and never runs this script. Deterministic unsupported guards, fixed-seed
generated claims and architecture canaries protect the disabled production boundary;
they do not substitute for positive native integration/crash/reconciliation tests.

The retained reviewer run and independent main-agent reproduction each executed
41 native processes for 2.1.314 (39 independent sequential cases plus two deliberate
concurrent processes) and 12 fresh core/control cases for 2.1.308. All seven core
checks passed for each version in both runs: 106 checked processes, 84 synthetic
`--yes` invocations. Including 97 earlier exploratory processes, this continuation
ran 203 native invocations, 159 with `--yes`. Historical 2.1.222's 62 invocations
are separate. Ordinary locked workspace tests passed 336 primary tests and eight
additional persistence child invocations, with zero failures and three explicit
live tests ignored. The M9 generated guard runs 256 fixed-seed cases including
exact 2.1.222/2.1.308/2.1.314 version claims; it grants no authority. All six local
gates passed. No positive Claude production native integration test exists because
no operation satisfies the support gate.

## Upstream requirements

Resume M9 only with a lossless stable local project identity and exhaustive
membership, a complete versioned startup/direct/cascade/retention contract, coherent
read-only complete observation with explicit error-versus-absence semantics, and
defined partial-error/interruption/concurrency outcomes. Revalidate the exact
artifact, then implement project granularity through M6-M8 and its positive gates.
Current contract defects require these upstream changes: **BLOCKED**.
