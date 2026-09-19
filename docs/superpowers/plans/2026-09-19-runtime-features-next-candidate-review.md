# Runtime Features Next Candidate Review

- Date: 2026-09-19
- Status: PLAN REQUIRED; no implementation started
- Authority: .project/STATE.md §3.49–§5.1; frozen Runtime/Backend contracts
- Precondition: RF-SRC-RTMP-01 is complete at `bbd029d`; CI `35408518440` is 7/7 PASS.

## Candidate adjudication

| Candidate | Current decision |
|---|---|
| SRT source | BLOCKED: exact BMD FFmpeg capability probe has no SRT protocol. |
| RTMP external/non-loopback expansion | Not READY: requires a new bounded security/endpoint/fixture acceptance design. |
| PACKET/Program switch | Not READY: crosses multi-input/Program ownership boundaries. |
| Hot-Standby/failover | Not READY: requires multiple source candidates and new ownership semantics. |
| Recording/Replay or SRS/Output | Not READY: external storage/service ownership is undefined. |
| RH-FLOW-01 | DEFER-UNTIL-TOUCH: only when multi-flow Audio/Metadata enters scope. |
| RH-IDEM-01 | DEFER-UNTIL-CONTROL-PLANE: persistent external command path does not exist. |
| 24h RSS stability | Verification debt, not a Runtime Features implementation packet. |

## Decision

No candidate is promoted to READY from the current evidence. The next action is to freeze one new bounded packet with explicit authority, allowed/forbidden scope, touch-gates, and acceptance before code changes. Until then, do not start Network umbrella, multi-input, Program/Switch, SRS, Recording/Replay, Flow, Idempotency, or 24h work under RF-SRC-RTMP-01.
