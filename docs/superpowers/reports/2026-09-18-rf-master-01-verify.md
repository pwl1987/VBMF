# RF-MASTER-01 验证报告

- 日期：2026-09-18
- Status：**COMPLETE / SOFTWARE + CI + BMD HARDWARE VERIFIED；WARNING DEBT REGISTERED**
- Exact commit：`9981273a`
- Plan：`docs/superpowers/plans/2026-09-18-rf-master-01-master-switch-normalized-execution.md`

## 1. Delivered boundary

RF-MASTER-01 consumes RF-NORM-01 selector-boundary Normalize evidence and permits
a bounded MASTER_SWITCH transition only when an explicit Normalize plan exists and
both video/audio evidence are `ObservedExact`. The existing paired selector,
epoch, observation, timeline, recovery and teardown owners remain unchanged.

PACKET_SWITCH remains fail-closed. No command/wire exposure, Network Source,
Hot-Standby, automatic failover, Recording/Replay, SRS/Output expansion, FFmpeg,
Clock/Flow/Idempotency, Session/Resource/Lease, output device-number 2 or 24h
stability scope was added.

## 2. Implementation and software

- `switch_execution.rs`: MASTER plan admitted at the internal domain boundary;
  PACKET remains rejected; explicit Normalize failure variants are observable.
- `switch_graph.rs`: GStreamer adapter gates MASTER before selector mutation on
  explicit Normalize evidence; missing/incomplete V+A evidence is fail-closed.
- `dual_input.rs`: canonical BMD L4 transition now exercises MASTER_SWITCH after
  L2c exact V+A; existing L1-L3/L5/teardown checks remain in the chain.
- Focused simulation tests cover missing plan, incomplete evidence with unchanged
  active selector, and exact V+A paired switch.
- Development VM: default **269/269**, mock **455/455**, integration **9/9 + 12/12**;
  fmt, clippy `-D warnings`, diff-check PASS.

## 3. CI and BMD exact acceptance

- GitHub Actions run `35400579632`: **7/7 required contexts PASS**; feature
  compile jobs `hardware-test-compile` and `gstreamer-build` PASS.
- Source archive SHA-256:
  `6fa541ba1b0bbba52e1c39ff03d0a051e9a3de7b3193114b66601087375d2d0d`.
- Native BMD build `bmd,gstreamer`: PASS; binary SHA-256:
  `3179fdd9807fd2174f3ccf25abd0b4ece83dd7130edd7231867b3ee0c449b499`.
- Manifest MD5 before/after：`7521d17e7fd02e50eb2b0a84374a43dd` unchanged.
- Inputs：`46:00000000:002e4500` / device 0 and
  `46:00000000:002e4400` / device 1; output device-number 2 PID `992634`
  and command unchanged before/after.
- Gate：**11/11 PASS**；L2c exact V+A；L4
  `MASTER_SWITCH timing/switch+timeline(A→B)` PASS with observed target and
  timeline preserved；L5 failure isolation/recovery, Supervisor role and Teardown
  PASS；no orphan/residue.

Evidence：
`evidence/bmd-10.30.15.10/2026-09-18-rf-master-01/`

## 4. Warning debt and boundary

The exact BMD run still emitted non-fatal `gst_video_converter_*` assertions at
graph setup/recovery and four `gst_pad_unlink` assertions during teardown. They
did not alter the 11/11 verdict, exact V+A evidence, MasterSwitch result, recovery,
or release state; they are explicitly archived and remain follow-up hardening debt.
This report does not claim warning-free runtime.

RF-MASTER-01 does not close the historical 24h `rss_bounded` debt.
