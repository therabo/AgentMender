# M8.2 production mutation audit

Audited 2026-11-01 against the integrated working tree. Test-only modules and
fixtures are excluded explicitly; source fitness still audits production after
test modules. This is a static call-site/authority review, not a claim that lexical
matching proves arbitrary Rust behavior. Independent security findings and
resolutions are in [the security review](m8-2-security-review.md).

## Provider-state destructive sites

| Site | Caller / exact action | Binding, gate and collateral protection |
|---|---|---|
| `scoped_files.rs::validate_and_unlink` | Private Codex scoped adapter; one descriptor-relative `unlinkat` for the frozen primary rollout; directory `fsync` afterward | Exact profile, canonical DB path + header ID/version, approved path/dev/inode/link/size/time commitments, no-follow ancestor descriptors, complete relevant inventory and known catalogue path coverage; native writer/maintenance/sidecar fence; consumed DurableIntent checked before entry; no recursive remove or discovered expansion |
| `scoped_storage.rs::DatabaseFence::apply` | Private adapter; static `DELETE FROM thread_read_receipts WHERE thread_id=?1` and `DELETE FROM threads WHERE id=?1` | Exact full schema/profile and canonical UUID; six frozen target row-set preimages/counts; attachment/feedback/incident-spawn counts zero; exact receipt count and one thread row; reviewed FK cascades only target dynamic tools; 17-table retained semantic projection verified before/after scratch savepoint and commit; genuine WAL exclusive lock through cutoff |
| Historical `lifecycle.rs::launch` | Private native M8 unarchive launcher; fixed `thread/unarchive`, never delete/archive | Original exact private executable pin, immutable native plan/effect contract, verified recovery, independent approval and consumed receipt; original fresh validation/postcondition contract retained. No transfer to 0.166 native delete |

There is no production provider `remove_dir`, `remove_dir_all`, recursive rm,
provider-directory rename/quarantine, index rewrite, history rewrite, generic SQL
port, native delete dispatch, fallback after ambiguous native effects, VACUUM,
checkpoint-for-reclaim or provider WAL/SHM deletion. The shared session-name index
and command history are retained. SQLite may write its fixed existing coordination
sidecars under the declared DB profile; sidecars must be regular, same-device,
single-link, no-follow objects, with stable identities held through SQLite close.
Rollback journal presence refuses admission. They are not per-session reclaim.

The adapter cannot open a writable source DB before consuming the receipt. SQL
schema/preimage/count validation is under exclusion; unknown SQL effects roll back
before payload unlink. Once unlink or writable SQLite startup may have happened,
errors remain ambiguous with typed per-action knowledge, not an automatic retry.
The actual destructive set is a subset of the reviewed manifest. Replacement or
new objects never inherit the old grant.

## Existing owned-state and evidence writes

| Surface | Allowed writes | Authority / isolation |
|---|---|---|
| Persistence `store.rs` | Initial owned metadata/key setup; `INSERT` authority metadata and append-only journal events in owned SQLite plus external JSONL; no event UPDATE/DELETE | Opaque private OwnedStateRoot disjoint from provider roots, ownership/permissions/reparse/lock checks; immutable event triggers and exact two-store agreement; grant MAC, expiry, one-shot signed consumption and durable flush before gate construction |
| Persistence `owned.rs` / `recovery.rs` | New owned transaction/recovery files, synced atomic no-replace hardlink publication; one `remove_file(&stage.path)` for the verified temporary second link | Owned-root capability and fixed names; does not delete final recovery/journal/provider state; metadata-only recovery has no transcript backup or automatic restore |
| Native lifecycle `pin_image` | Fixed create-new executable/signature copies under a fresh owned evidence sandbox | Original opaque EvidenceSandbox and exact historical hash; no provider payload removal or arbitrary destinations |
| `capture.rs` and `scoped_capture.rs` | Bounded RAM SQLite backup/scrub and owned evidence output | Source SQLite READ_ONLY/NOFOLLOW/query-only in the kernel-denied worker after request/path/write-denial validation; output is bounded metadata/commitments; no transcript bodies or credentials persist |
| Provider-api process launchers | Fixed sandboxed version/metadata workers, exact capture helper flags and existing read-only App Server evidence requests | Write denial to real provider state, cleared environment, no unsandboxed retry; scoped cleanup hashes the provider binary and does not start it |

Normal discovery, planning, integrity, MCP and independent review do not acquire
provider mutation authority. MCP retains eight read-only tools and no grant,
receipt, key, transaction-creation or execution port. Claude retains zero mutation
profiles. Read-only reconciliation inspects only frozen objects and records
knowledge in the owned journal; it cannot reconstruct DurableIntent or dispatch.

## Enforcement and reviewed exceptions

`scripts/check_architecture.py` retains dependency direction, domain purity/storage
separation, no new unsafe boundary, strict provider/CLI/MCP approval ownership,
nonconstructible/noncloneable gates and intent-before-dispatch checks. M8.2 permits
only the exact Codex private modules: it checks the two literal exact SQL deletes
inside the fixed action and the single relative unlink inside the frozen-payload
helper, while rejecting exported low-level primitives and additional relative
mutations. Unsupported-platform stubs may consume a gate only to return refusal.
Application is a Codex **test-only** dependency for full-chain disposable acceptance;
the runtime dependency graph stays unchanged.

The final review identified and closed two physical/dependency holes before
release: hardlinked WAL/SHM aliases and known catalog rows selecting rollout paths
outside the inspected inventory. Negative planning and under-fence tests preserve
the target/foreign files in both cases. No destructive acceptance uses real user
sessions. No change widens scope to descendants, projects, Claude, NLP or M10.
