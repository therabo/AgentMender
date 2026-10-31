# Exact-source scope and observation review

Source provenance: official tag-resolved commit
`6ff1e1ca3b30fc8b914720681fcf5fe62e95f856`. Native behavior is separately labeled
in archive/delete evidence. Source analysis does not establish reproducible binary
identity or claim every possible source state was created by the UI.

| Source at exact commit | Conclusion / provenance limit |
|---|---|
| [Raw edge traversal](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/state/src/runtime/threads.rs#L314) | Lifecycle reads persisted spawn descendants without joining thread rows. Source-only edge-leaf namespace gap. |
| [Relationship listing](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/state/src/runtime/threads.rs#L1271) | Public relations join threads, apply archive/source/attachment filters, and include empty previews for relation filters. No assertion that one filter omission proves union impossible. |
| [Manager subtree](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/thread_manager.rs#L1146) | Persisted edges union this manager's local live registry. Fresh dedicated process may keep its own registry empty; unrelated processes are not automatically in that union. |
| [Native delete](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/app-server/src/request_processors/thread_delete.rs#L31) | Descendants precede root; native partial error is not NoEffect. Actual binary confirms this. |
| [Archive batch](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/archive_thread.rs#L52) | Later member failures can be warnings after success. Actual binary confirms accepted partial cascade. |
| [Unarchive batch](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/unarchive_thread.rs#L37) | Busy/failed descendants can be skipped; root success is not full restoration. |
| [Unarchive subtree fallback](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/app-server/src/request_processors/thread_processor.rs#L2195) | Failure loading subtree can fall back to root-only. Old target-only semantics cannot be generalized. |
| [Current thread view](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/app-server/src/request_processors/thread_processor.rs#L3095) | Useful exact selected-store/live view. Its miss is not a complete cleanup/retention certificate. Lack of typed NotFound alone is insufficient to reject an exact lookup. |
| [Metadata-only read](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/read_thread.rs#L357) | Initial DB errors may be suppressed; stale metadata may survive missing rollout. Do not infer full retained integrity from metadata presence. |
| [Resolver re-query](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/thread_rollout_resolver.rs#L87) | Persistent DB errors can propagate. Initial suppression is not a demonstrated error-as-absence counterexample. Selected paginated history may deliberately avoid older materializations. |
| [Cleanup namespaces](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/thread-store/src/local/delete_thread.rs#L123) | History, rollout variants, names and database cleanup extend beyond one read/list view. |
| [Read repair](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/rollout/src/list.rs#L1415) | Fallback lookup can repair metadata; writable native lookup is not protected read-only reconciliation. |
| [Startup](https://github.com/openai/codex/blob/6ff1e1ca3b30fc8b914720681fcf5fe62e95f856/codex-rs/core/src/thread_manager.rs#L560) | Migration/compression may start before lifecycle RPC. Count writable startup as an effect. |

Five key source files were fetched independently from the commit URL and compared
byte-for-byte with the acquired tag archive; see source-commit-verification.json.
Other rows were inspected in the same official tag archive. A raw edge with no
thread row is a source-supported namespace distinction, not a native/UI fixture claim.

Read-only reconciliation would need authenticated consumed history/recovery, exact
profile/identity, coherent complete observation of approved effects and retained
namespaces, and a trusted preeffect baseline or reviewed equivalent proof. It must
create no grant/intent and never dispatch, mutate, read-repair real state or retry.
No complete current contract establishes those requirements. Internal parsing,
more persisted-edge observation or better baseline storage alone cannot supply it.
