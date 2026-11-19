# ADR 0045 — Scope Codex storage capabilities independently

Accepted 2026-11-18. Extends ADR 0043 and ADR 0044.

The Codex adapter owns platform discovery and format recognition. Discovery
returns observations and capability reasons, with a separate identity for each
root. The CLI selects a single session root using the launch override, the
process configuration or the default home. Discovering a path grants no
authorization. SQLite remains inside the confined reader. Metadata observation
can run on more platforms than native mutation.

Native removal reviews the logical subtree and every physical rollout. It
revalidates recognized archival states, graph closure, locators, history bases,
revisions and writer ownership. A persisted open edge requires corroborated
storage and passes the same lock and native lifecycle checks. Both rollout
trees stay write-denied except for exact reviewed files. The release and
platform must pass the disposable native fixture.

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
