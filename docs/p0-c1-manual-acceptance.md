# Use AgentMender with real Codex sessions

P0-C1 is **OPEN**. Real read-only discovery works with the installed macOS Codex;
actual model/client invocation and independently approved deletion are separate
acceptance gates. No synthetic provider root or historical binary is needed to
browse. Synthetic fixtures remain development tests only.

## Connect the existing installation

Build once from the checkout:

```sh
cargo build --locked --workspace
```

In the **actual Codex client's local MCP settings**, add a STDIO server named
`agentmender`. Use the absolute AgentMender executable and these arguments:

```text
--provider codex mcp --session-search
```

The selected root defaults to `CODEX_HOME` or `~/.codex`. This explicit read mode
finds the existing bundled executable under `/Applications/ChatGPT.app` or
`~/Applications/ChatGPT.app` when `codex` is absent from PATH. It reads metadata
under a kernel-attested worker and inspects the executable's advertised native
schema in private scratch. It does not change the application bundle or auth.
Use explicit canonical root/executable paths for a particular installation:

```toml
[mcp_servers.agentmender]
command = "/absolute/AgentMender/target/debug/agentmender"
args = ["--provider", "codex", "--codex-root", "/absolute/your-codex-home",
  "--codex-executable", "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
  "mcp", "--session-search"]
```

Replace both `/absolute/...` placeholders. For this read-only mode, the normal
client `config.toml` can remain in its home; the historical cleanup profile's
absence-of-config restriction does not apply to browsing. If your deployment
selects an external SQLite home, explicitly select the database root you intend
AgentMender to inspect; do not assume a default local DB covers other stores.

The existing embedded CLI can save the registration without any PATH install:

```sh
"/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex" \
  mcp add agentmender -- /absolute/AgentMender/target/debug/agentmender \
  --provider codex --codex-root /absolute/your-codex-home \
  --codex-executable /Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex \
  mcp --session-search
```

Refresh **only this MCP server connection** in the Codex client. Do not quit or
restart the application. The [official MCP documentation](https://learn.chatgpt.com/docs/extend/mcp)
describes local `mcp_servers` and client server controls. A server added in a
web ChatGPT Plugins screen is not evidence that the current Codex chat has it.
`mcp add/get/list` verifies configuration, not actual model use.

For this workstation, the real read-only registration has been saved in the
Codex config. The obsolete generated fixture was removed after provenance checks.
The UI tool prohibits access to the Codex application, and the user could not
refresh/verify its tools during development. That client gate remains OPEN.

## Search and choose an exact identity

In a dedicated management conversation, ask:

> Use only AgentMender session_search to find my Codex sessions associated with
> MISSION1. Include archived results. Show title, UUID, project/namespace, creation
> and last activity, archive state, available parent relationship and compatibility
> limits. Keep duplicate titles separate. Do not prepare or execute removal.

Require an actual `session_search` call and grounded output. The configured server
exposes thirteen tools; the original default launch still exposes eight.
Search uses all-term title/display-name/project-basename/UUID matching, not full
transcript search. Native display names take precedence over first-prompt titles.
Missing or oversized labels are null/UUID fallbacks, not invented titles. Times
are Unix seconds and may be unavailable. `active` means non-archived in this DB;
it does not prove runtime activity. Parent metadata is partial, not descendant
closure. Names/paths and relationships never establish destructive ownership.

The most recent at most 500 records and 96 KiB of metadata are observed. Coverage
is explicitly partial and limited to the selected root DB. A response over MCP's
budget fails rather than silently dropping matches; narrow the query. Absence
under partial coverage is not proof of absence. Opaque targets and namespace refs
are process-local; reacquire after server restart. Native UUID/root identity and
later durable transaction IDs have different purposes.

## Inspect the compatibility assessment

Select one exact UUID and namespace, then request `preview_removal` with its opaque
`target`. In `--session-search` mode this returns a **blocked read-only assessment**:
exact selected metadata, native method availability, incomplete affected set,
descendant uncertainty, irreversible-effect/residual/concurrency disclosures,
postconditions and independent approval requirements. It has no Ready plan or
plan digest and permits no execution. Do not mistake interface presence or
`can_request_preview` for mutation compatibility.

On installed alpha.6, native `thread/delete` is advertised, but isolated tests
still show descendant deletion and partial effects before error. No current native
delete is admitted. `prepare_removal`, `removal_status` and
`request_removal_execution` return `read_only_removal_unavailable` in this mode;
no approval store is created or read. Existing historical provider scan capability
and this metadata reader are separate contracts.

## Stop before real destructive acceptance

A real session is never disposable merely because it is old. The human must
choose its precise UUID and approve the complete affected set independently.
Current read mode intentionally cannot turn that request into authority.

The existing stable 0.166.0 Darwin arm64 Legacy/WAL scoped fallback remains
available through the separately configured `mcp --session-removal` mode. Its exact
binary/schema/configuration/physical/dependency/writer gates remain unchanged;
readability of an alpha or unknown producer does not make that fallback eligible.
See the [operating contract](conversational-codex-removal.md) for its original
preview/durable preparation/terminal approval/execution/status commands.

Before any destructive admission, a trusted host/operator must independently
identify/register the actual management UUID in the correct provider/root. A
model-supplied UUID, title, cwd, Active label or arbitrary peer is not caller
binding. This MCP connection does not authenticate the conversation's identity.

The model must also lack access to approval secrets and control of the human
terminal. Same-user shell/PTY/key access does not enforce independence. If the
required separation or operation contract is unavailable, leave removal blocked.
Do not let the model type a challenge or issue a fixture grant. No real user
session is deleted by this development task.

If an admitted profile requires a writer to close, only the human may close and
later reopen Codex. Closing does not admit an unsupported version; it may stale a
plan and require fresh review. After a future authorized operation, retrieve its
durable status, independently prove primary absence and unrelated preservation,
and report residuals/unknown physical reclaim. GUI disappearance is a separate
optional user check. Unknown effects never permit retry or native/local fallback.
