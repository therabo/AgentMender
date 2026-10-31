# M9 provider-blocked investigation closeout

Recorded 2026-10-31 before M8.1. This is a closeout decision and integration gate;
the subsequent M8.1 integration receipt records the actual pushed baseline.

- M9 capability status: **BLOCKED BY PROVIDER**.
- M9 investigation status: **CLOSED FOR CURRENT PROVIDER CONTRACT**.
- Claude lifecycle operations: **Unsupported**.
- Claude mutation capability: **NOT IMPLEMENTED**.
- M9 research/evidence: **COMPLETE**; provider blocker: **ESTABLISHED**.

M9 is integrated as blocker evidence, NOT as completed lifecycle capability.
Green checks validate retained evidence and safety guards; they do not make M9 PASS.

## Verified source and remote evidence

| Item | Directly verified evidence |
|---|---|
| Original main | `83f0692cdda33bc3be34146ab8ceefab5dadf1f3` |
| M9 branch | `milestone/m9-claude-lifecycle` |
| Original local/remote M9 HEAD | `53d2a485bdc2a20111ad52393a5a9f1a75ac3518` |
| Original exact-head run | [41090154046](https://github.com/therabo/AgentMender/actions/runs/41090154046) |
| Linux | [PASS](https://github.com/therabo/AgentMender/actions/runs/41090154046/job/124181624632) |
| macOS | [PASS](https://github.com/therabo/AgentMender/actions/runs/41090154046/job/124181624870) |
| Windows | [PASS](https://github.com/therabo/AgentMender/actions/runs/41090154046/job/124181624333) |

SSH ls-remote independently matched both exact branch heads. The GitHub connector
returned 404 and gh was unavailable; authenticated Safari displayed the exact run,
full commit link, successful summary and individual platform conclusions. All six
local gates passed at that source head. The documentation-only closeout follow-up
must also pass all gates and exact-final-head CI before fast-forward integration.
Normal push must preserve history; verify remote main before starting M8.1.

## Retained blockers and review

Code 2.1.222, 2.1.308 and 2.1.314 remain separate Unsupported claims. ADR-0014/0015
and sanitized compatibility records preserve non-injective project selectors,
`a-b` / `a/b` collision, nested project cascade, writable dry-run/startup and backup
creation, partial destructive effects, insufficient exit-status semantics, unclosed
effects, no lossless lifecycle identity, no authoritative complete negative proof,
and no complete read-only reconciliation. Existing evidence is internally consistent;
no destructive Claude research was reopened for this closeout.

Full research/milestone and source reconstruction preceded changes. Independent
Sol/high reviews covered all adapters/isolation, domain/properties, persistence,
approval, application, CLI, MCP, architecture checks, fixtures and Git history.
The M9 diff contains ADR/documentation, sanitized evidence, opt-in synthetic-state
reproducers, guard/property tests, architecture canaries and test-only stabilization.
The sole production-source-file change is inside persistence tests and waits only
for post-crash lock release. No production Claude source behavior changed.

Zero dispatcher, lifecycle profile, Ready operation, filesystem fallback, JSONL
mutation, history/config rewrite or session-to-project widening exists. Independent
approval, durable intent, fresh validation, Unknown protection, observation-only
reconciliation and the eight read-only MCP tools remain intact. M10 is not started.
