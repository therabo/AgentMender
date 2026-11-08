# P0-C1 manual client and removal acceptance

Status on 2026-11-08: **OPEN**. The installed Desktop build 24614
(26.1029.61003) embeds Codex 0.166.0-alpha.6. It has no admitted deletion profile.
The supported primary adapter still requires the exact stable 0.166.0 Darwin
arm64 binary, Legacy payload, fixed state_5 schema and root-local WAL described
in the [operating contract](conversational-codex-removal.md). A discovery match
does not establish that contract. Fully conversational execution is unavailable;
MCP hands approval **and execution** to an external human operator.

## 1. Run the non-destructive client acceptance first

Build AgentMender and create new synthetic search data:

```sh
cd /absolute/checkout/AgentMender
cargo build --locked --workspace
python3 scripts/prepare_p0_c1_readonly_fixture.py
```

The helper accepts no paths or other arguments and writes only a fresh temporary
directory. Keep its printed `root`, `state_root`, `project` and
`protected_fixture_peer`. These are local test data. That UUID is **not your
management conversation**; this phase cannot accept caller protection or deletion.
Do not run approval or execution against this onboarding fixture.

Use an independently verified stable executable with SHA-256
`39acf979729dd290f9a24ee38b2914e036c91b65952872e28ae2a6b88a7db7c1`:

```sh
shasum -a 256 /absolute/pinned/codex-0.166.0
```

In the actual desktop application, open **Settings → MCP servers → Add server**.
Choose STDIO, name it `agentmender-p0-c1`, and use the following launcher shape.
Replace every uppercase placeholder with the printed/verified absolute value.
The same configuration can be saved using the installed CLI command:

```sh
codex mcp add agentmender-p0-c1 --env CODEX_HOME=FIXTURE_ROOT -- \
  /usr/bin/env -u CODEX_SQLITE_HOME /absolute/agentmender \
  --provider codex --codex-root FIXTURE_ROOT \
  --codex-executable PINNED_BINARY \
  --protect-codex-session FIXTURE_PEER_UUID \
  mcp --session-removal --state-root FIXTURE_STATE
```

For direct configuration, the equivalent table is:

```toml
[mcp_servers.agentmender-p0-c1]
command = "/usr/bin/env"
args = ["-u", "CODEX_SQLITE_HOME", "/absolute/agentmender",
  "--provider", "codex", "--codex-root", "FIXTURE_ROOT",
  "--codex-executable", "PINNED_BINARY",
  "--protect-codex-session", "FIXTURE_PEER_UUID",
  "mcp", "--session-removal", "--state-root", "FIXTURE_STATE"]
env = { CODEX_HOME = "FIXTURE_ROOT" }
required = true
startup_timeout_sec = 30
tool_timeout_sec = 120
```

Save this in the **client's** configuration, outside the selected fixture root.
Never put `config.toml`, a named profile or project `.codex/config.toml` inside
the selected provider/configuration boundary. The primary adapter refuses them.
The explicit environment selects the fixture for the server; the current client
retains its own home. Clearing `CODEX_SQLITE_HOME` is necessary if inherited.
Do not copy authentication files or secrets into a fixture or MCP arguments.

Save/refresh the MCP server using the application's server controls; `/mcp` shows
connected servers. Do not quit or restart Codex as part of this procedure.
The [official MCP instructions](https://learn.chatgpt.com/docs/extend/mcp)
describe these client controls and shared CLI/Desktop configuration.

Open a dedicated **new management conversation** manually and ask:

> Using only AgentMender's session_search, find Codex sessions associated with
> MISSION1. Include archived results. Show each title, last activity, exact UUID
> and root namespace. Preserve duplicate matches and distinguish discovery from
> removal compatibility. Do not prepare, approve or execute anything.

Require an actual `session_search` tool call and its result, not a configuration
listing. Expected fixture observations: six matches, two identical `MISSION1
README` titles, one archived match, one unavailable title with a UUID fallback,
one incompatible alpha producer, and one protected fixture peer. The common
project basename makes all six matches relevant. Last activity is Unix second
1767312000 (2026-01-02 00:00:00 UTC). Never invent a missing title or export a
transcript. `can_request_preview` only permits requesting a fresh assessment.

Verify the catalog contains the original eight tools plus `session_search`,
`preview_removal`, `prepare_removal`, `removal_status` and
`request_removal_execution`. Duplicate titles require exact UUID/namespace
selection. `partial` means a bounded scan, not absence; another root is another
identity. General historical App Server inventory is a separate discovery path.
The new metadata reader itself requires the pinned stable binary/schema; pointing
it at alpha/unknown state does not provide general current-Desktop search.

If using a separate isolated CLI home instead of Desktop, the human must first
authenticate that isolated client with `CODEX_HOME=CLIENT_HOME codex login`.
No credential was copied in development: its natural-language run stopped with
401 after loading all 13 tool schemas. A catalog load alone is not acceptance.

## 2. Establish a genuine management exclusion before preparing removal

Stop the read-only onboarding here unless a real eligible disposable case and a
verified management identity are available. The current alpha Desktop does not
satisfy the stable deletion profile. Development's safely isolated stable
`thread/start` also failed configuration synchronization, so it did not establish
a real eligible session. Do not modify producer versions, SQL, history layout or
compatibility gates to make a case eligible.

For a future supported same-root case, the human/host must independently identify
the exact management UUID using the trusted client's session identity or a local
operator's inspection of its canonical record and matching session metadata.
Do not use a title, cwd, Active label or an identifier suggested by the model.
Confirm the root too. One reproducible CLI binding is for the human to select a
known UUID independently and explicitly launch `PINNED_BINARY resume UUID` in
that disposable home. The known UUID being resumed is then the registration;
do not replace it with a name or `--last`. Read-only verification must still
establish that this is the actual same-root management conversation.

For that same-root deployment, provide MCP settings as launch overrides instead
of writing a root-local config. The installed CLI supports this syntax:

```sh
CODEX_HOME=ELIGIBLE_ROOT PINNED_BINARY \
  -c 'mcp_servers.agentmender.command="/usr/bin/env"' \
  -c 'mcp_servers.agentmender.args=["-u","CODEX_SQLITE_HOME","/absolute/agentmender","--provider","codex","--codex-root","ELIGIBLE_ROOT","--codex-executable","PINNED_BINARY","--protect-codex-session","VERIFIED_MANAGEMENT_UUID","mcp","--session-removal","--state-root","PRIVATE_STATE"]' \
  resume VERIFIED_MANAGEMENT_UUID
```

Replace placeholders inside the TOML strings as well. This is a launch template,
not evidence that a current client produces supported Legacy history. Refuse any
incompatible history, external DB selection, active writer, shared/attached
history or spawn dependency. A management conversation in a different root cannot
be represented by registering an arbitrary peer in the target root.

Register the verified UUID through trusted `--protect-codex-session` launch
configuration. Confirm the search result is protected, its preview refuses, and
the target's review retains it. The original plan freezes this exclusion, and
approval/execution restore and freshly recheck it. MCP does not authenticate its
caller; independent registration remains a deployment requirement.

## 3. Review and prepare; stop before the human's destructive action

Select one genuinely disposable, noncritical target by exact UUID and root.
Require `preview_removal({target})`, inspect the exact binary/version/schema,
`scoped-cleanup-v1`, Ready status, affected UUID set, required payload/row counts,
protected peers, retained commitments, residuals and irreversible consequences.
Shared cwd/project/title never widens the target set. Unknown evidence refuses.

Only then request `prepare_removal({target, expected_plan_digest})` with the
unchanged `sha256:` digest from preview. Record the returned durable
`transaction_id` and `plan_digest`. Preparation is not approval. A repeat returns
the same attempt; expired/revoked attempts are not renewed automatically.
`request_removal_execution({transaction_id})` returns a handoff, not execution.

**The agent stops here. The following steps are operated by the human.**
This development task does not authorize removal of any real user conversation.

## 4. Human approval and execution in the declared trust boundary

An unrestricted same-user coding agent can access `authority.key`, control a
PTY, call the local approval machinery and execute a grant. A same-user terminal
therefore does not enforce human independence. For acceptance, the operator must
establish a deployment where the model is limited to the bounded MCP interface
and cannot read authority material, alter the trusted launch, or control the
approval/execution terminal. If that separation cannot be established, record
human approval as **OPEN**. A cooperative manual ceremony is only an explicitly
constrained intermediate mode, not an adversarial isolation proof.

In that external terminal, use the original provider root, pinned executable and
private state root. Clear conflicting environment selection for every command.

```sh
env -u CODEX_SQLITE_HOME CODEX_HOME=ELIGIBLE_ROOT /absolute/agentmender \
  --provider codex --codex-root ELIGIBLE_ROOT --codex-executable PINNED_BINARY \
  plans inspect DIGEST --transaction-id TX --state-root PRIVATE_STATE

env -u CODEX_SQLITE_HOME CODEX_HOME=ELIGIBLE_ROOT /absolute/agentmender \
  --provider codex --codex-root ELIGIBLE_ROOT --codex-executable PINNED_BINARY \
  approve DIGEST --transaction-id TX --state-root PRIVATE_STATE

env -u CODEX_SQLITE_HOME CODEX_HOME=ELIGIBLE_ROOT /absolute/agentmender \
  --provider codex --codex-root ELIGIBLE_ROOT --codex-executable PINNED_BINARY \
  transactions --state-root PRIVATE_STATE execute TX
```

Review the exact original plan and type the challenge yourself. Never ask the
model to type it, pipe it, create a fixture grant or handle the key. The one-shot
grant is plan/transaction/recovery bound and lasts at most 300 seconds. Wrong-plan,
stale, expired, revoked or consumed approval cannot authorize execution.

If writer checks refuse, stop. The human may manually close the relevant client
and choose when to reopen it; AgentMender must never quit/restart it. Changes
during close may stale the plan and require fresh review/preparation. The primary
profile requires compatible lifetime fences, not blanket Desktop shutdown.
Closing an application does not admit unsupported profiles or global JSON writes.

## 5. Retrieve results and record separate acceptance levels

After execution, ask the management client for
`removal_status({transaction_id: TX})`. After a server restart use the durable TX,
or `removal_status({})` to list attempts, rather than an old opaque target token.
Before any attempt is prepared there may be no owned store to inspect.

Require Succeeded, `primary_verified: true`, retained sessions preserved at the
recorded fenced cutoff, approval Consumed and `retry_permitted: false`. Independently
inspect the target canonical row/payload absence and unchanged protected/unrelated
session rows/payloads. Keep any proof local; do not export full transcripts.
For an interrupted/unknown result, inspect/reconcile the original attempt:

```sh
env -u CODEX_SQLITE_HOME CODEX_HOME=ELIGIBLE_ROOT /absolute/agentmender \
  --provider codex --codex-root ELIGIBLE_ROOT --codex-executable PINNED_BINARY \
  transactions --state-root PRIVATE_STATE reconcile TX
```

Reconciliation is read-only and cannot redispatch or restore payloads. Never retry
after unknown effects or switch to native deletion as a fallback. Recovery metadata
is not a transcript backup. Report additional artifacts as not fully cleaned,
physical reclaimed bytes as unknown, and GUI confirmation separately. Reopen the
client manually when convenient and inspect disappearance only if requested;
neither presence nor absence in its list overrides structural evidence.

Record implementation, actual client invocation, management binding, independent
human authorization, actual eligible deletion/collateral proof and optional GUI
confirmation separately. Fixture PASS and three-OS CI cannot close those user gates.
