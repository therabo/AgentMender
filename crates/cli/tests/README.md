# Discovery test boundaries

Normal cargo test runs use static synthetic fixtures or temporary directories.
Before/after fixture manifests compare paths and bytes. Providers are not required;
PATH is cleared in CLI tests. No normal test scans a real provider home.

The two ignored macOS smoke tests wrap the entire CLI in an independent
filesystem-write and network-denied sandbox and require explicit installation paths:

```sh
AGENTMENDER_CODEX_ROOT=/absolute/codex/home \
AGENTMENDER_CODEX_EXECUTABLE=/absolute/path/to/codex \
cargo test -p agentmender --test read_only live_codex_read_only -- --ignored --exact
```

Claude uses AGENTMENDER_CLAUDE_ROOT and AGENTMENDER_CLAUDE_EXECUTABLE with
live_claude_read_only. These smoke tests prove safe report handling, not successful
complete session enumeration or live parser compatibility. They print no provider
content, and never relax isolation to make a provider initialize.
