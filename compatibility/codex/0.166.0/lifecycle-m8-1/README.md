# Exact current Codex lifecycle refresh: BLOCKED

Observed 2026-10-31. These are **research-only compatibility claims**, not an enabled
production profile or imported planning proof. Archive/delete and new cascading
unarchive remain Unsupported. Historical exact M8 support is unchanged.

## Provenance

Official [GitHub stable release](https://github.com/openai/codex/releases/tag/rust-v0.166.0)
and npm `@openai/codex/latest` independently identify `0.166.0`. The release is
non-prerelease, published 2026-10-29T19:47:53Z. No installer was run.

| Artifact / boundary | Exact evidence |
|---|---|
| Official macOS arm64 archive | `codex-aarch64-apple-darwin.tar.gz`, upstream and acquired SHA-256 `c48824462fc57681eafb0925d97b11b589010475613904f570b42fea53e240d5` |
| Extracted native executable | SHA-256 `39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1` |
| Native CLI / App Server | `0.166.0`; initialization also returns private codexHome, unix/macos platform metadata |
| Official source | Annotated tag `593389380740f7b4762771573c447ccfdc59742f`, commit `6ff1e1ca3b30fc8b914720681fcf5fe62e95f856` |
| Source archive | SHA-256 `b32b28ac8dfd6653a153fd2e93a810d499378b03c4e341d8ea9952ae2b78d57d` |
| Source identity limit | Official release association; no reproducible source-to-binary build identity claimed. Tag reported unsigned. Five key files independently matched exact commit bytes. |
| Adapter / platform | AgentMender `0.1.0`, macOS arm64 investigation only |
| Native state layout | 66 entries, compact SHA-256 `1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`, exactly V165 |

[binary-provenance.json](binary-provenance.json), [profile.json](profile.json) and
[source-commit-verification.json](source-commit-verification.json) retain separate
distribution, native, schema and source provenance. No binary/source archives or
user state are committed. Sanitized summaries omit hostnames, runtime installation
IDs, private paths and unrelated notifications; fixture UUIDs are deliberately synthetic.

## Schema evidence

Exact binary generation produced 321 stable and 467 experimental JSON schemas.
Both modes were repeated in independent fresh runtimes with identical complete
path sets/hashes. [schema-manifest.json](schema-manifest.json) records both sets;
[selected-schema-manifest.json](selected-schema-manifest.json) binds 24 retained
request/response files under `schemas/stable` and `schemas/experimental`.

Canonical SHA-256 over sorted full path-to-file-hash maps:

- Stable: `1a0c7ce9118772a7b2f3ca351d976bdb244e7b34633e897dbe0c68a9ff6fcfa2`.
- Experimental: `ce6e6f326f53a36fa184cba45a1e813a38cd31d4862e6250d9700dc3595a4a43`.

Eight of ten comparable retained stable schemas are identical to V165. List and
unarchive responses change CodexErrorInfo/ThreadItem definitions. No old committed
read schema exists, so no historical read equality is claimed. Experimental list
parameters add parentThreadId, ancestorThreadId, attachedToThreadId and projectId
relative to the retained old stable schema. These are separate experimental claims.
Stable list parameters do not expose those fields. See [structural comparison](schema-comparison.json)
and [compact fingerprints](schema-comparison-summary.json).

SessionSource/SubAgentSource/ThreadId/AgentPath definitions match retained V165.
Actual spawn ancestry remains `source.subAgent.thread_spawn.parent_thread_id`;
top-level parentThreadId can remain null. Matching wire/storage structures are
not equivalent native semantics: new unarchive restores spawned descendants.

## Native findings and limits

[Archive evidence](archive-evidence.json) records seven native cases: hidden
child/deep tree/multiple children, mixed archive states, root/child denial, cascading
unarchive denial and attached descendants. Ordinary list omission persists, while
ancestor filtering reveals the tested no-preview child. An attached spawned child
is omitted from ordinary/parent/ancestor lists, but attached filtering reveals it.
The tested union recovers that child; this is not proof union closure is impossible.

Archive can accept while a denied child stays active and a grandchild archives.
New unarchive restores descendants, can accept while a denied child remains archived,
and restores a child archived before forward archive. No equivalent compensation.

[Delete evidence](delete-evidence.json) and [11 checked assertions](delete-checks.json)
record active/archived mixed-tree removal, hidden metadata-only child, retained
unrelated/fork-only rollout hashes, shared-name entries, partial error and restart,
and full UUID alias behavior. Root removal denial returns -32603 after descendants
are removed. Metadata-only reads then retain stale rows, while history reads return
the same current-view missing error as a never-seeded UUID, including after restart.
That history error cannot prove entity/metadata absence. Matching sampled retained
rollout hashes are finite fixture measurements, not complete global invariants.

The initial metadata `.ok().flatten()` is followed by a resolver re-query which can
propagate persistent DB errors. No persistent DB-error-to-not-found counterexample
is claimed. Lack of a typed not-found response alone is not the blocking argument.
See [source and reconciliation review](source-review.md) for actual namespace limits.

Fixtures were hand-generated JSONL metadata before native operations, not histories
created by network/model turns. Native effects were actual provider calls. An
accepted synthetic attached/spawn combination is not claimed to be UI-generated.
Raw-edge-only missing-thread-row closure is source-derived, not native fixture evidence.

## Reproduction and isolation

Acquire the exact official archive privately and verify both archive and extracted
binary hashes above. Do not install it or replace any app/PATH executable. Run:

```sh
python3 scripts/probe_codex_current_lifecycle.py --allow-isolated-native --binary /absolute/private/codex-aarch64-apple-darwin
```

The standalone reproducer accepts no provider root. It creates fresh 0700 state,
independent HOME/CODEX_HOME/TMPDIR/cwd and explicit synthetic fixtures, verifies the
exact binary before launch, clears inherited environment, and kernel-tests outside
write, actual account-home read, loopback egress and fork denial. Mach lookup is
explicitly denied in policy. Both output streams/time/buffers are bounded and
children are killed/reaped on failure. No weaker retry. Private runtime reads can
write only disposable state; they are not claimed to be real-state reconciliation.

Seven fixed cases independently reproduce key findings; evidence is categorical
and contains no raw notifications. It does not prove complete lifecycle success.
Original A/B/C investigations additionally retain their private raw artifacts and
exact-source review. Normal CI validates committed schemas/evidence and guard
tests without Codex, network, user homes or native provider execution.

## Promotion gate

The [formal decision](../../../../docs/adr/0016-current-codex-cleanup-provider-contract-gate.md)
and [full report](../../../../docs/codex-full-cleanup.md) keep complete preapproval
effects, authoritative complete negative proof, retained invariants and protected
read-only reconciliation mandatory. Schema/hash knowledge never grants authority.
No production profile, archive/delete adapter, RemoveSession operation or natural
language layer was added. Claude remains blocked and M10 is not started.
