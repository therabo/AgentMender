# Complete graphical removal: research only

Status: **OPEN**, zero admitted complete-removal production profiles.

These files bind static evidence for Desktop 26.1027.51404/build 23880,
`com.openai.codex`, embedded CLI 0.166.0-alpha.5.1, and the separately pinned
official stable 0.166.0 source used for M8.2. They do not transfer the stable
primary-cleanup profile to Desktop or to a stable-override composite candidate.

- `projection-inventory.json`: 17 investigated surfaces, actual known mechanisms,
  classifications, proposed exact mutations, refresh/restart risks and gaps.
- `projection-provenance.json`: exact archive/assets/binary/source hashes.
- `desktop-catalog-schema-static.json`: selected SQL extracted from exact app code
  and derived disposable schema. It is explicitly not a full runtime DB capture,
  trigger/FK/index closure or destructive capability.

No user database, session, credential or renderer store was read. No app/provider
was launched to obtain this static evidence. Prospective RequiredProjection entries
are obligations, not supported capabilities; unknown surfaces are not NotRelevant.
GUI disappearance/restart acceptance remains pending.

The separate `isolation-result.json` records successful synthetic kernel canaries
and one fork-denied signed-main startup attempt that exited SIGTRAP before GUI
readiness. It is not graphical removal acceptance. See
[the exact experiment result](../../../docs/research/codex-desktop-isolation-result.md).

Run `python3 scripts/check_complete_removal_evidence.py` for provider-independent
inventory/provenance checks. Optional `--installed-bundle /Applications/ChatGPT.app`
also hashes selected static installed files without launching them or reading user
state. These checks cannot prove live projection absence.

See [ADR-0018](../../../docs/adr/0018-complete-session-removal.md),
[projection analysis](../../../docs/research/codex-desktop-projections.md),
[role closure and probe details](../../../docs/research/codex-desktop-role-closure.md),
[artifact inventory](../../../docs/research/complete-session-removal-artifacts.md)
and [isolation design](../../../docs/research/codex-desktop-isolation-design.md).
