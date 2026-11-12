# ADR-0037 — Autonomous MCP session finding

Status: accepted for implementation. Supersedes the client orchestration in ADR-0036.

## Decision

The read-only Codex MCP mode exposes `session_find` and scoped job status, result
and cancellation tools. The caller supplies a natural-language query. The MCP
connection owns the apparent caller binding and signs published references with
its existing key. It never grants lifecycle or approval authority.

The CLI owns the jobs. It embeds the existing Python Two-Stage coordinator, role
instructions, validator and SearchIntent schema. A private stdio bridge invokes
the existing typed Rust resolver in process. The Rust host owns both subprocess
handles and permits one fixed Planner followed by at most one fixed Evaluator.
It reaps the Planner before retrieval and the Evaluator before publication.
There is no second MCP connection in the normal runtime.

On macOS the existing evidence sandbox supplies a private directory. A distinct
fixed App Server profile permits model network access, denies provider writes
and process creation, and uses a private Codex home with a read-only credential
link. Account credential refresh that needs provider writes must fail closed.
The coordinator has no network, credentials, or process dispatch. The source
worker keeps its existing sandbox. Unsupported platforms refuse.

Jobs are in-memory and results are paged from a private bounded SQLite spool. Jobs
have absolute deadlines, process-local opaque random IDs, bounded concurrency
and expiry. Closing the MCP connection cancels and reaps active jobs. A restart
invalidates jobs and references. Caller metadata remains spoofable and can
only scope read-only lookup, never authenticate an identity.
