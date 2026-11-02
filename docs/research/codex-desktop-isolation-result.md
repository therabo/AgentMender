# Isolated Desktop startup feasibility result

Date: 2026-11-02. Product verdict **OPEN**.

The synthetic kernel canary passed actual owned read/write, protected owned-file
read/write denial and loopback/Unix socket denial in parent and successfully
exec'ed child under the same reviewed policy. Strict permission-only queries
reported denial for global Apple Events, selected security/pasteboard/accessibility
broker names and real-home/keychain/app-group paths. No sensitive broker call,
Apple Event or actual real-user file access was performed. These tests do not
establish comprehensive broker coverage or graphical containment.

Sol/high context_review independently approved exactly one bounded signed-main
startup probe: launcher SHA256
`905fc99be8f55fd7a144904e724f73cc79eab9d2e05603a3049ba451449818fb`,
policy SHA256
`711dbcdffafb893d00f6e9ae320d0d28b6389af130ccbdc73b2d6489f9c3ca34`.
The same policy denied actual fork and posix_spawn canaries. Only canary and
signed main executable execution was permitted; helper/CLI execution, network,
Mach/Apple Events and real-home access were denied. The launcher used an empty
constructed environment, unique owned HOME/CODEX_HOME/userData, owned cwd,
no debug descriptors, an 11-second startup/15-second total bound and 128 KiB
output cap, with exact unreaped child identity checks and finally cleanup.

The direct startup attempt exited with **SIGTRAP (return -5)** before any GUI or
readiness observation. Both captured streams were empty. The original child was
reaped; no helper/native CLI was permitted to start. No real running application
was stopped or restarted. No real session, auth state or UI store was accessed.
The cause of this early exit is not established by an empty diagnostic stream.
There was no unsandboxed retry or graphical-policy widening.

This is a failed startup-feasibility observation, not a complete GUI acceptance,
not a session deletion test and not evidence of provider impossibility. A working
reviewed graphical isolation/observation/coordination contract remains necessary
before admitting any complete-removal profile. The source-supported structured
probes and normal API-key fixture route remain prospective and were not executed.

[Machine receipt](../../compatibility/codex/complete-removal-research/isolation-result.json)
binds results and hashes. Full source snapshots, policies, canary receipts,
stdout/stderr and review metadata are preserved in the internal
AgentMender_M8_2_1_Verification/isolation evidence folder. Five redundant test-owned
stable-binary copies were removed after preservation; those bytes are not product
reclaim, and the original pinned M8.2 evidence binary was preserved. Historical
launch scripts reference those experiment roots and require fresh preparation.
