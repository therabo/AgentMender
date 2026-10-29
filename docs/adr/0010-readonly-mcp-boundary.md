# ADR-0010: Read-only stdio MCP with private bounded views

## Status

Accepted, 2026-10-28, following three independent Sol/high design reviews. M4
integration base is `139291761975c1dca50492adcb5972cbfe7233f5`.

## Context

M5 exposes existing queries and pure planning to model clients. MCP is an inbound
interface, not the core or an authorization authority. Domain identities contain
local paths, including paths embedded inside EntityId strings. Existing CLI JSON
therefore cannot be reused verbatim as a model-facing export format. Provider ports
are synchronous and intentionally do not require Send/Sync. Their existing process,
evidence-directory and source-denial guarantees must survive asynchronous requests.

The full research, milestone specification, ADRs, repository source/tests/fixtures,
Git history and actual remote CI were reconstructed before implementation.

## Decision

Add one `mcp-server` library crate depending internally only on application/domain.
CLI remains the composition root and exposes one `agentmender mcp` entry point.
Trusted launch configuration selects roots, executables and fixtures. No MCP input
can change them. Application owns dispatch and target resolution; MCP owns protocol,
strict DTO validation, privacy projection and presentation pagination.

Use official `rmcp =3.8.0` with defaults disabled and only server/transport-io;
Tokio 1.54.1 with the runtime, macro, synchronization and time features needed by
stdio orchestration. SDK MSRV 1.88 fits existing Rust 1.89. Keep schema derives in
MCP DTOs, using schemars 1.2.3. No HTTP/auth/web framework/client transport. Registry
metadata and downloaded upstream source were checked, rather than copying research
versions. HMAC 0.13 with existing SHA-256 and OS entropy via getrandom 0.4.3 protect
ephemeral references/cursors; these are lookup/presentation mechanisms, never grants.

Use the SDK's bounded AsyncRwTransport with a 64 KiB input-frame ceiling. Support
current 2026-07-28 discover/per-request negotiation and legacy initialize through
the SDK. Application version, negotiated MCP revision and provider versions remain
separate. Advertise tools only. Stdout is exclusively protocol traffic; install no
raw-request tracing subscriber. Errors/logs use fixed catalogs without source prose.

### Public tools

The fixed surface is providers, workspace_scan, inspect, graph, explain,
verify_integrity, plan_lifecycle and repair_plan. Discovery/graph/verification call
their existing application use cases. Target queries use a pure application matcher
against the same fresh observation used for explanation/planning. CLI bare-ID methods
retain their behavior. Multiple matches and incomplete absence remain explicit.

Lifecycle intentions are archive, unarchive, soft_delete and purge. repair_plan maps
only to the already implemented pure Repair intention, currently Unsupported. This
satisfies the actual M5 specification without inventing a repair planner. Blocked,
Unsupported and Unknown are successful observations with visible structured reasons.
No execute, approve, restore, repair execution, provider command or filesystem/SQL
primitive exists. Read-only/non-destructive annotations are hints, never enforcement.

### Privacy and structured results

Use positive-allowlist typed DTOs with explicit input/output JSON Schemas and strict
unknown-field rejection. No arbitrary JSON payload, shell string, caller proof,
imported plan, path target, root or executable parameter. Observation budgets are
server-bounded (default 1000, maximum 10000); page sizes default 50, maximum 100.

An entity reference contains provider, class and a process-keyed token over the full
lossless ArtifactId, including the hidden root. Recompute tokens over fresh state;
there is no token registry/database. Tokens correlate within one process and expire
on restart. They are not capabilities or authorization. Distinct private variants
remain countable and conflict flags survive even when visible metadata is identical.

Export categorical provider/compatibility/coverage/evidence facts, safe metadata,
opaque identities, structured graph provenance, ownership protections, integrity
findings/blockers/residuals, and immutable plan descriptions. Replace all paths,
native/local IDs, timestamps, diagnostic/error prose and evidence locators with
opaque references, presence flags or fixed catalog meanings. Recognized rule names
and versions/profile IDs use explicit allowlists; unknown descriptors remain
opaque/unrecognized. Never copy transcript/prompt/response/source bodies, secrets,
configuration, raw databases or arbitrary provider strings into results or logs.

PlanView is a presentation DTO, not LifecyclePlan wire format. Preserve the original
PlanDigest and StateFingerprint with explicit
`digest_basis: original_unredacted_domain_plan_v1`. Hashing a privacy view does not
reproduce those commitments. Keep planning_only=true, executed=false and independent
approval required. Digest equality is content identity, never authenticity or approval.

Typed top-level records paginate graph/entity/proof/plan collections. Nested evidence
and observation lists stay typed. Repeated scalar headers preserve original context,
statuses, collection counts and blockers. Every complete tool response is at most
256 KiB. A single oversized record/context returns a structured size error. No record
is silently truncated. Presentation completeness is separate from discovery coverage.
Process-keyed cursors bind normalized arguments, tool, whole view identity and offset.
Continuation recomputes fresh state; changed views fail with stale_cursor.

### Cancellation and lifetime

Supply an application request/control/response dispatch seam and an immutable CLI
execution closure. Each admitted blocking worker constructs its adapters locally.
Keep provider ports unchanged. Admit one worker with no waiting queue; the worker
owns the semaphore permit until actual cleanup, even if the request is cancelled.
Check cooperative cancellation before/between observation boundaries and after
returns. SDK request cancellation and a fixed deadline stop delivery; tracked work
drains on shutdown, preserving child reaping and evidence-directory ownership.

An in-progress synchronous provider call finishes under existing bounds. Started
blocking jobs cannot be killed with JoinHandle::abort; arbitrary filesystem syscalls
are not instantly interruptible. Cancellation must not be advertised as stronger.

## Alternatives

Direct CLI report serialization leaks paths/prose and future added fields. A token
registry or resource cache introduces hidden state. Unkeyed pathname hashes permit
dictionary correlation. Domain schema derives couple the core to this adapter.
Raw JSON-RPC implementation duplicates maintained protocol negotiation/framing.
HTTP/OAuth and persistent plan resources create unnecessary trust/state boundaries.
All are rejected for M5. Tools suffice for fresh computation and bounded inspection;
no resources, subscriptions or fake persistent plan store are introduced.

## Consequences and security implications

Views deliberately disclose less than local CLI JSON. Restarts invalidate tokens and
cursors; concurrent provider changes may require restarting pagination. Cancellation
is cooperative at synchronous boundaries. Current real plans remain blocked or
unsupported and current incomplete integrity remains Unknown. None of these limits
permits weakening provider compatibility or adding mutation authority.

## Validation

Enforce seven-crate direction, unchanged read-only ports, no MCP provider imports,
filesystem/SQL/process access or execution/approval primitives. Test modern and legacy
protocol lifecycles, schemas/strict inputs, ambiguity and partial absence, fixture
calls, synthetic Ready/Blocked/Unsupported plans, Unknown/findings, privacy canaries,
stable serialization, pagination/stale cursors/size bounds, cancellation/busy/draining,
protocol-safe stdout and unchanged provider bytes. Retain all CLI/property gates and
require exact-final-commit Linux/macOS/Windows CI.

Primary references:

- https://modelcontextprotocol.io/specification/2026-07-28
- https://modelcontextprotocol.io/specification/2026-07-28/server/tools
- https://modelcontextprotocol.io/specification/2026-07-28/server/resources
- https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio
- https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/cancellation
- https://github.com/modelcontextprotocol/rust-sdk/tree/rmcp-v3.8.0
- https://docs.rs/tokio/1.54.1/tokio/task/fn.spawn_blocking.html
