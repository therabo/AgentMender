# Product roadmap

The local manager is the primary surface. Current work focuses on predictable
main-conversation inventory by project and verified outcomes for individual
Codex trees and verified native project assignments.

## Delivered within the storage workflow

- Discovery separates platform installations and archives from session and
  export authorizations.
- Supported macOS arm64 releases review persisted unarchived trees and
  compressed variants with the same kernel and writer checks as archived trees.
- Export inventory and preview support confirmed permanent removal of explicit
  selections on macOS and Linux, with separate receipts from session removal.

## Coverage and compatibility

- Exercise Linux reader confinement with read-only mounts or Landlock write
  denial plus an independent syscall and network boundary. Verify an isolated
  SQLite working set, including live WAL behavior, before enabling session reads.
- Exercise Windows handle-relative scope, reparse rejection and a real OS
  isolation boundary for provider processes. AppContainer or a restricted
  broker requires Windows-native fixtures before enabling a writer.
- Qualify additional Codex releases and storage revisions against the native
  cascade, writer-lock, reference and unrelated-session preservation fixtures.
- Improve Claude format coverage with fixtures for observed missing metadata.
  Qualify a project removal contract with a fixed namespace, writer
  coordination and verification of state outside the project directory.

## Follow-on work

Evaluate per-session artifact attribution after provider coverage has improved.
Keep attributable sizes separate from logical or physical space reclaimed.
Evaluate durable outcome history if users need to inspect confirmed removals
across application restarts. Keep future confirmation separate from history.
Additional Desktop registry removal requires a verified provider contract.
