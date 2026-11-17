# ADR 0044 — Provider-scoped projects in the local manager

Accepted 2026-11-17. Extends ADR 0043.

## Decision

Use the exact absolute workspace recorded in recognized Codex metadata and
the directory namespace observed under Claude Code's projects tree. Both
identities are scoped to the provider and configured state root. A missing
workspace on disk can still have archive metadata. Treat the workspace
string as data and keep it out of filesystem mutation paths.

Build the project inventory from the provider observation before filtering
and paging. Keep session identity, detail and transcript access independent.
Represent unassigned conversations as a group without removal capability.

Rebuild Codex project membership and every descendant scope inside the
confined reader. The existing native dispatch and postcondition are applied
serially to the reviewed roots. The browser's launch-scoped receipt binds
the full review. Stop on change or uncertainty and report progress by
subtree. After a restart the current provider observation is authoritative
and a fresh explicit confirmation is required. Launch-scoped results do not
claim any outcome after an interruption.

Claude directories are observable project instances. Keep them read-only
while exclusive ownership, sidecars, shared references and writer
coordination lack an exercised deletion contract.

## Validation

HTTP and native fixture tests use isolated temporary roots. The architecture
check enforces the confined reader, native dispatch and read-only Claude
boundaries.
