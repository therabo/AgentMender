# ADR 0045 — Scope Codex storage capabilities independently

Accepted 2026-11-18. Extends ADR 0043 and ADR 0044.

The Codex adapter owns platform discovery and format recognition. Discovery
returns observations and capability reasons, with a separate identity for each
root. The CLI selects a single session root using the launch override, the
process configuration or the default home. Discovering a path grants no
authorization. Native session SQLite reads remain inside the confined
reader. Metadata observation can run on more platforms than native mutation.

The native delete command owns the provider-managed conversation files and
databases. After success the Codex adapter cleans the selected UUID from the
local JSONL indexes and supported Desktop stores. The Desktop writer uses
fixed statements on the two recognized catalog paths. The local thread,
scan tombstone and revision counters change atomically in each catalog.
Catalog writes wait up to five seconds for contention.
AgentMender requests sidebar reconciliation after cleaning Desktop JSON
state. [ADR 0046](claude-session-cleanup.md) defines the complete session
workflow. Validate the release with the disposable native fixture.

Storage export inventory has a distinct browser surface. It lists filesystem
groups and uses bounded metadata reads. Roots confirmed through an installation
or explicit configuration are tracked separately from name-based candidates.
An explicit selection and confirmation authorize eligible first-level groups of
the discovered export roots. The confirmation identifies independent backups
and user output. Active leases, shared references and partial coverage are
reported for review. Overlapping storage stays protected. Preview receipts
belong to the export operation namespace and are bound to the selected
server-side group revisions.
Removal reuses the inventory revision and performs no-follow writes relative to
the verified root. Database and session deletion keep separate authorization.

Validation uses isolated fixtures for platform candidates, linked and modified
entries, provider layout and rollout variants, writer locks, shared references,
and both browser workflows.
