# Manual acceptance for Codex Desktop removal

Current status **Candidate / NotTested**. Desktop cleanup has no admitted
production profile. The first three steps can establish identity and review a
read-only plan. Resume at step 4 only after AgentMender documents structural
admission for your exact build and your plan reports Ready.

1. Choose a disposable local chat with a distinctive title and no work you
   need. Keep a second disposable chat in the same project as a preservation
   check. Exclude the chat running development work.
2. From the target chat header menu, use **Copy session ID** if available.
   Record the full UUID, exact title, project and whether it is archived or
   pinned. The title identifies what you expect to see. The UUID identifies
   which record AgentMender may act on. Stop if either is uncertain.
3. From an external terminal, use the standalone AgentMender CLI and supply
   the reviewed absolute paths for the Codex root, Desktop user data, and exact
   stable worker. Run the read-only `plan cleanup-session <UUID>` with the
   `--codex-root`, `--codex-desktop-user-data` and `--codex-executable` options.
   Verify the printed title, UUID, profile/build and blockers. A candidate or
   any other blocked plan carries zero executable operations.
4. For a future admitted profile, quit Codex manually and keep all relevant
   writers closed until structural verification finishes. Repeat the plan from
   the external terminal and verify Ready and the same title and UUID. Use the
   same provider paths for `plans prepare cleanup-session <UUID>`, plan
   inspection, independent terminal approval of the returned digest and
   transaction ID, and transaction execution. Use one private AgentMender
   state root separate from provider data. The
   [CLI workflow](scoped-session-cleanup.md#local-cli-workflow) shows the command
   forms. If interrupted or uncertain, preserve the transaction ID for
   read-only reconciliation.
5. Expect `Session removed: "<exact title>" (ID: <UUID>)` and
   `Structural cleanup: Verified`. If structural cleanup remains unknown or
   blocked, stop and record the reported outcome.
6. Open Codex yourself when convenient. Check the sidebar, recent, pinned and
   custom sections, project list, search by the distinctive title, and archived
   or all sessions where available. Confirm the peer still appears in its
   project. If a stale entry persists after a normal manual refresh or reopen,
   report which view shows it and whether it opens the target session.
7. Report build, transaction ID, result status, view, whether you refreshed or
   reopened, and whether the peer survived. Keep the exact session UUID in a
   private report if needed. Use the synthetic test title and omit transcripts,
   full JSON reports, paths, screenshots containing private data and credentials.

AgentMender determines the structural result. Only the user's actual observation
can set graphical acceptance to UserConfirmed or UserReportedFailure.
