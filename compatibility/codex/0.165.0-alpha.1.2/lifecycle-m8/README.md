# AgentMender M8 isolated Codex native-contract investigation

Observed 2026-10-29. No repository production files were changed. No real user state, credentials, conversations, or network-backed model turn was used. All lifecycle effects were actual installed-binary operations on explicitly synthetic, AgentMender-owned disposable fixtures.

## Exact binary and schema provenance

The binary at `/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex` reports `codex-cli 0.165.0-alpha.1.2` under sandbox-exec. Its SHA-256 is recorded in binary-provenance.json. A stable full schema generation produced 320 JSON files under schemas/. All ten compared initialization/list/archive/unarchive/delete request/response schemas exactly match committed V165 files. Native fresh state has 66 schema entries, compact SHA-256 `1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647`, identical to the exact V165 layout. No release-source-to-binary identity was established; no upstream-source inference is used.

Each native runtime had a dedicated private HOME/CODEX_HOME/TMPDIR/cwd. Sandbox policy denies filesystem writes outside that owned runtime, all network, process forks and Mach lookup, plus all real-home file data reads. An outside-write canary was denied and remained absent. No weaker retry was used. thread/start failed managed-preference loading under this policy; evidence therefore uses explicit synthetic JSONL metadata fixtures, not claimed natively generated histories. Fixtures were prepared before lifecycle calls. No direct provider SQLite mutation was used; only native scan/indexing, native lifecycle RPCs, and read-only source-schema queries.

## Native request/response and observation

All three lifecycle requests require threadId:string. Archive/delete return empty objects; unarchive returns thread. IDs are UUID parser inputs. Invalid UUID, missing rollout and read-not-found outcomes all use broad error codes, principally -32600, and storage failures use -32603; codes do not provide a typed authoritative absence taxonomy. Repeat archive on an archived target and repeat unarchive on an active target error, rather than behaving as idempotent success. Repeat delete after complete removal errors; delete of a missing rollout with retained metadata can succeed.

Successful ordinary operations had observable final state by the next sequential native request. That empirical ordering does not establish crash durability or make a response transaction success. Notifications can follow responses. Failed cascading delete emitted no deletion notifications despite a removed child rollout. No notification absence is a NoEffect proof.

Native thread/read metadata summaries have path/status/source, but no archive boolean. The exact native path changed from sessions/ to archived_sessions/ after archive and back after unarchive. Positive native list membership under archived:false or archived:true provides logical presentation-state evidence for the matched ID. A completed list, an absent match, or an error does not provide deletion/descendant absence proof.

## Cascade findings: archive/delete must remain unsupported for production closure

A root, child and grandchild were archived/deleted together. A forkedFromId-only fork and an unrelated thread remained. Spawn ancestry appears in actual responses as source.subAgent.thread_spawn.parent_thread_id; the historical normalized decoder's optional top-level parentThreadId does not retain that actual field.

Archive root success can mean root and grandchild archived while a denied child remains active. Descendant failure does not stop attempting deeper descendants. The response was {} and notifications omitted the denied child.

Delete with root-rollout write denial removed the child rollout before returning -32603; root remained. No deleted notification was emitted. Metadata-only thread/read(child) continued returning a successful stale row/path, including after process restart, while includeTurns:true failed. Thus RPC error is not NoEffect and metadata read success is not intact rollout proof.

A child with session_meta only and no user preview was omitted from fully paginated active thread/list, with nextCursor:null, in both scanning and DB-only modes. Exact read knew the child, and archive(root) affected it. Exhaustive list pagination therefore cannot establish native descendant closure. This is a concrete counterexample against upgrading current partial discovery/effect contracts to exhaustive effects.

## Unarchive-specific findings

Ordinary archived root restored to active; archived child/grandchild remained unchanged. Fresh exact native read showed active path; positive DB-only active listing included target. A no-preview target was restored successfully but omitted from active list. The latter must produce Unknown if positive membership is required by verification.

A fixture with active and archived physical copies of the same identity yielded `no archived rollout found` after native indexing preferred the active identity, despite the archived fixture still existing. Fixture bytes remained unchanged. That error cannot prove absence. A write-denied root returned -32603 and remained archived in the observed case; errors still require fresh observation.

These observations support target-only logical unarchive semantics, consistent with current official documentation, without making unarchive a compensation inverse of cascading archive. They do not establish a closed provider-global backing-storage effect contract.

## Strict read-only mirror observation

mirror_observation.py uses the existing AgentMender internal capture worker, including kernel source-write denial, scrubbed RAM backup and exact V165 profile. No rollouts are copied. Source fingerprints matched. Native AppServer runs against the disposable mirror with source file-content reads denied.

DB-only active list positively returned the unarchived target; archived list returned the retained child. thread/read(includeTurns:false) failed -32603 because this binary tried to read source rollout session metadata. Do not silently add thread/read to this observer by relaxing privacy or copying transcript headers. Positive filtered-list identity membership is available; absence must remain Unknown.

## Reproducibility and evidence

experiment.py is the bounded sandboxed transport; synthetic_probe.py prepares explicitly synthetic rollouts. contract_cases.py, error_cases.py, hidden_descendant.py, restart_missing.py, unarchive_cases.py and mirror_observation.py reproduce the cases in their fixed owned evidence locations. Existing case directories deliberately prevent accidental reruns except the explicit restart case. These are investigation harnesses, not production adapters or normal CI tests. Before promoting them to repository opt-in scripts, preserve explicit isolated-only invocation, fresh root creation and non-user-state guarantees.

Each transcript and summary records only synthetic fixture identities/paths/content and native responses. Generated runtime stores/system skills are not part of the evidence manifest. evidence-manifest.json hashes the top-level scripts/results and all generated schemas. check_evidence.py validates 15 recorded factual assertions: 15 passed, 0 failed. Source-preserving sandbox and runtime evidence are separate from deterministic fake-server evidence.

Official documentation fetched and opened: https://learn.chatgpt.com/docs/app-server . It describes target-plus-attempted-descendant archive, descendant delete, target unarchive, missing-file handling and asynchronous notifications. Those current docs corroborate observed behavior; they do not attest to complete exact-version runtime effects or source identity.


The adjacent summaries are sanitized empirical evidence. They are not caller-importable planning proofs. The original investigation scripts are retained outside the repository; the opt-in production-adapter test reproduces the supported dispatch path in newly generated disposable roots.
