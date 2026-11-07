# ADR-0021: Structural admission and manual Desktop acceptance

Accepted 2026-11-07. Supersedes the automated graphical acceptance prerequisites
in [ADR-0019](0019-structural-removal-and-profile-acceptance.md) and
[ADR-0020](0020-closed-desktop-complete-removal.md).

Structural admission requires exact provider identity, independent structural
verification of the primary and every required persistent reconstruction
source, collateral preservation, and verification of the maximum safe approved
cleanup. Use real provider-state readers and typed effects on disposable data.
Source analysis must account for reconstruction roles and support the checks.
Unresolved roles, incomplete inventories, missing receipts and uncertainty
after an effect prevent a complete structural result.

The user verifies graphical disappearance separately, with a manual refresh or
restart at their convenience. That observation can confirm product acceptance
or expose a reconstruction defect. It has no execution authority and cannot
repair a failed structural result. AgentMender never controls the running app
to observe its UI. Automated graphical tests are optional future evidence.

The independent approval, immutable manifest, durable intent, per-action
observations and read-only reconciliation in ADR-0020 still govern execution.
The user may run a future admitted profile from an external terminal with all
relevant writers closed until verification finishes. Keep the writer census,
native locks, physical identity checks, preimages and retained postimages.
Recheck both global images and the census at each publication. This detects
additional interference but leaves a window against a writer ignoring the
remain-closed prerequisite. No compatible lifetime lock or filesystem CAS has
been established for the global-state writer.

Track structural implementation and graphical acceptance independently.
Only structural evidence and reviewed compiled profile admission can yield
structural success. The complete product milestone also requires the user's
actual graphical confirmation. This decision changes neither schema-2/3
authority nor any serialized manifest, approval or journal encoding.
