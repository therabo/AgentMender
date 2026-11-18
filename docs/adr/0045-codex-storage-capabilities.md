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
An exporter contract must establish ownership, resource use and a confined
removal primitive before a group can obtain deletion authorization. Independent
backups, unknown groups and user output have protected states. Preview receipts
belong to their own operation namespace and remain bound to server-side scope.
No database or session deletion path authorizes an export operation.

Validation uses isolated fixtures for platform candidates, linked and modified
entries, provider layout and rollout variants, writer locks, shared references,
and both browser workflows.
