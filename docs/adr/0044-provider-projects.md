# ADR 0044 — Provider-scoped projects in the local manager

The current session deletion workflow is defined by [ADR 0046](claude-session-cleanup.md).

Accepted 2026-11-17. Revised 2026-11-18 for Codex Desktop parity.
Extends ADR 0043.

## Decision

Identify a Codex project by an observed registration scoped to the configured
archive. Read native registrations and roots from `projects` and
`project_roots` in the same confined SQLite snapshot as `threads.project_id`.
Read recognized Desktop registrations and assignments from the bounded local
state file. Reconcile legacy IDs using the explicit migration map for the
configured local host. Preserve an observed standalone legacy registration
when it has no native counterpart and has never been mapped in that archive.

An observed main conversation joins a registered project when its explicit
assignment resolves without conflict. For an unset SQLite assignment, a
recognized local Desktop assignment can supply membership. Conflicts,
dangling IDs and conversations with no verified association appear in Recenti.
The recorded working directory remains context and a search field.

Build Projects and Recenti before filtering and paging. Empty registered
projects retain an identity. The separately paginated views share session
details and transcript access. Each provider root and native ID are bound into
opaque keys. Claude continues to group by the observed directory namespace.

Bulk Codex removal applies the same native UUID command serially to the
observed main conversations of a native project with complete membership.
The registration and its roots are outside this operation.

Codex `project/delete` removes the registration and clears assignments. The
browser exposes session removal only. Using the experimental project protocol to
remove a registration requires a separate verified lifecycle contract.

## Validation

Disposable HTTP and native fixtures cover empty registrations, explicit
assignments, migration conflicts, same-workspace independent conversations,
and native serial deletion. The architecture check enforces the confined
reader and native dispatch boundaries.
