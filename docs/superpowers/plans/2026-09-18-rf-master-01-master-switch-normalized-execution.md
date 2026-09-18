# RF-MASTER-01 — bounded normalized MasterSwitch execution

- Date: 2026-09-18
- Status: PLAN FROZEN
- Branch: main
- Authority: A2-1 Canonical SwitchPolicy design/verify; RF-NORM-01 spec and
  Phase B report; live switch execution SPI and GStreamer adapter.

## 1. Comprehension and decision

A2-1 defines MASTER_SWITCH as normalize → one common output format → switch.
The current domain and adapters deliberately reject it. RF-NORM-01 now supplies
the missing prerequisite: explicit video/audio target, materialized per-source
Normalize chain, and observed exact caps at the selector boundary.

This packet enables only the bounded form above on the existing GStreamer
Program graph. It does not claim heterogeneous source support, packet switching,
automatic failover, or a public control-plane command.

## 2. Allowed scope

- Let the internal ExecutionGroup produce a MASTER_SWITCH plan while keeping
  PACKET_SWITCH fail-closed.
- Keep the existing dual-plane selector topology and switch epoch/observation
  ownership unchanged.
- In the GStreamer adapter, require an explicit Normalize plan and exact V+A
  evidence immediately before a MASTER_SWITCH pad flip.
- Reuse the existing normalized graph for the actual paired video/audio cutover.
- Add simulation and focused fail-closed tests for missing, incomplete, and exact
  Normalize evidence.
- Extend the canonical dual-input gate with a bounded MasterSwitch acceptance
  after the existing L2c evidence is present.

## 3. Forbidden scope

- No PACKET_SWITCH implementation.
- No new source types, Network Source, SRS, Recording/Replay, Hot-Standby, or
  automatic failover.
- No output device-number 2, FFmpeg, Session/Resource/Lease owner, wire schema,
  command-plane policy exposure, Clock/Timecode, Flow, Idempotency, or 24h work.
- No inference of Normalize readiness from intent, element construction, or a
  successful prior FrameSwitch; exact V+A selector-boundary evidence is required.
- No hiding or reclassifying the existing non-fatal GStreamer warning debt.

## 4. Implementation shape

1. Update SwitchError wording/variant so incomplete Normalize evidence is
   observable and fail-closed.
2. Allow MasterSwitch in ExecutionGroup::plan_switch; retain the adapter as
   the authority for backend-specific readiness.
3. In GStreamerSwitchAdapter::switch, validate explicit Normalize plan and
   NormalizeEvidence::complete() before mutating either selector. Return the
   evidence states on failure.
4. Leave SwitchExecutionAdapter topology-neutral; Mock remains unsupported for
   MasterSwitch unless its own evidence model is explicitly added later.
5. Add simulation tests proving: no plan → reject; missing/incomplete evidence
   → reject with zero selector/bookkeeping mutation; exact V+A → paired switch,
   observed target, monotonic epoch.
6. Gate acceptance runs FrameSwitch baseline plus one MasterSwitch transition
   only after L2c exact evidence; teardown and failure isolation remain required.

## 5. Acceptance

- Existing software suites, format, clippy, architecture/remove-adapters and
  diff-check remain green.
- Focused MasterSwitch tests cover both policy/domain and GStreamer readiness.
- CI required contexts: 7/7.
- BMD exact commit: dual-input gate proves L2c exact V+A, one MasterSwitch
  transition, observed video/audio target agreement, failure isolation/recovery,
  teardown, and unchanged output PID/manifest.
- Evidence archives include source SHA, binary SHA, manifest, gate output and
  warning observations.

## 6. Stop conditions

Stop with UNKNOWN and do not widen scope if exact V+A evidence cannot be
observed before the MasterSwitch attempt, if either plane diverges, or if the
existing gate cannot separate normalized readiness from switch execution.


## 7. Required files

- services/media-agent/src/switch_execution.rs
- services/media-agent/src/adapters/gstreamer/switch_graph.rs
- services/media-agent/src/gates/dual_input.rs
- Focused tests adjacent to the above implementation.
- This plan, the RF-MASTER report, evidence archive, and .project/STATE.md.

No other subsystem is in scope.