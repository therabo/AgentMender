# Installed Desktop catalog source evidence — 2026-11-03

Research-only, source-derived evidence. No application/provider was started, no
real provider database or userData was read, and no repository file was modified.
This evidence is not an admitted mutation profile or restart acceptance receipt.

## Reproduce

Run `python3 extract_catalog_evidence.py /path/to/ChatGPT.app /path/to/fresh-output`
from this directory with the exact reviewed bundle and an empty owned output
directory. The extractor reads only the supplied bundle's Info.plist, main
executable and app.asar. It verifies the pinned ASAR and migration asset hashes
before deriving the schema.

The ASAR reader parses the archive header and reads selected byte ranges; it does
not extract or execute the application. From asset339 it isolates native migration
declarations and the native K manifest/checksum construction. A fresh Node VM
exposes only crypto.createHash, evaluates that bounded declaration fragment with a
one-second timeout, and returns JSON. No require/process/filesystem/Electron object
is exposed to the VM. Migrations execute only against Python SQLite `:memory:`.
No migration-ledger rows, runtime timestamps or real provider content are invented.
The source's two legacy user_version shims are reproduced, yielding 34.

`ordered-migrations.native.json` preserves native JSON serialization;
`ordered-migrations.json` is its inspectable equivalent with evidence metadata.
Both include all 36 migrations, predecessor identities, checksums and operations.

`complete-schema-projection.json` contains every sqlite_master entry, table_info,
table_xinfo, foreign_key_list, index_list, index_info and index_xinfo. Its separate
semantic projection excludes rootpage, which is page allocation rather than
schema identity. The canonical semantic JSON and its digest exclude the final LF.
The SQLite version used for disposable derivation is recorded. Source-derived
fresh schema does not prove every historical migration route; production must
validate its actual complete schema and migration ledger under the approved
physical/root/writer boundary. Reject unknown tables, indexes, triggers, views,
foreign keys and migration records rather than silently ignoring them.

`complete-schema.sql` is a convenient fresh-schema DDL rendering (tables before
indexes), not a provider migration runner. Index enumeration sequence can differ
when replaying this convenience rendering; native ordered migration execution is
the authority for derivation. `provenance.json` records exact Unicode source ranges
and UTF-8 hashes. `catalog-class.extracted.js` includes the complete relevant mU
catalog class plus the short intervening source tail before mV. The small extracted
windows independently show checkpoint format, removal and tombstone suppression.

## Superseded partial evidence

The repository file
`compatibility/codex/complete-removal-research/desktop-catalog-schema-static.json`
remains unchanged. Its final catalog column order is superseded by the actual
native manifest: `trial_conversation_type TEXT` is added **before**
`chatgpt_async_status INTEGER`. Declaration order and chronological migration IDs
are not execution order. The prior artifact was explicitly incomplete for whole
schema admission; this directory supplies the full source-derived projection.

## Findings

There are 14 tables, eight explicit indexes (seven catalog indexes and
`automations_owner_idx`), no triggers, no views, no foreign keys and no
AUTOINCREMENT sequence. The final native migration is
`20261028065000_create_active_chat_turns`; user_version is 34.

Authoritative target removal increments only its host observation_sequence,
conditionally creates/updates the exact removed scan tombstone when a checkpoint
exists (closed process means no in-memory activeScan), deletes only host+thread,
and increments shared catalog_revision only for removal of a visible row.
Shared checkpoint JSON and unrelated rows remain intact. Checkpoints contain
cursors, phases, watermarks and anchor tuples, not catalog row payloads. Native
resumed-scan upsert skips removed tombstones. Fresh scans reset bookkeeping and
must prove canonical primary absence before non-reappearance can be accepted.

Source evidence alone does not supply actual two-cycle isolated reconstruction
acceptance. All production approval, DurableIntent, preimage, physical, schema,
closed-writer, structural and collateral gates still apply.
