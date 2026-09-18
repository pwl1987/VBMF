# RF-FF-01E — FFmpeg production Session composition

Date: 2026-09-18

## Result

RF-FF-01E is complete at exact runtime commit `96f8055e1a9fa791cdbed8146fbd2b8eedfc9495`.

The production composition root now constructs the existing canonical Session path for FFmpeg:

`Device/Port authorization → ResourceRegistry → Lease/Preflight → SessionManager → MediaBackend(FFmpeg)`.

No new Session, Resource, Lease, command, or Runtime truth owner was created. FFmpeg DeviceHandle remains adapter-private. Production construction starts no media; media execution still requires an explicit Session intent.

No frozen Architecture/Contract, CanonicalPipelinePlan, GraphRuntimeIntent/wire, or MediaBackend SPI changed.

## Implementation

- Added `bootstrap::build_ffmpeg_session_composition` as the single production dependency factory consumed by both the production binary and BMD acceptance gate.
- The factory fail-closes on missing/invalid manifest, machine mismatch when runtime identity is available, and missing/ambiguous live Provider authorization.
- It builds `PortRegistry::build_authorized`, derives Resources, injects the existing FFmpeg backend, and constructs the existing `SessionManager` in Production mode.
- `media-agent` constructs the FFmpeg composition in production but does not auto-select or auto-start a device.
- Added `VBMF_FFMPEG_SESSION` acceptance gate. It requires an explicit DeviceHandle and explicit canonical port_id and sends one real Session intent through the production factory.
## Development VM verification

Before the acceptance-dispatch fix:

- focused RF-FF-01E factory tests: **3/3 PASS**.
- ffmpeg-backend full: **275 PASS / 1 ignored hardware**.
- default: **258/258 PASS**.
- mock: **443/443 + integration 9/9 + 12/12 PASS**.
- ffmpeg/default/mock clippy `-D warnings`: PASS.
- architecture lexical gate: PASS.
- remove-adapters proof: PASS.
- `git diff --check`: PASS.

At final exact `96f8055`, the focused RF-FF-01E 3/3 suite, ffmpeg-feature clippy, fmt and diff check were rerun and PASS. Exact-commit full regression is additionally covered by GitHub CI below.

## Acceptance RCA

The initial runtime implementation commit `2656678` reached the complete real Session lifecycle on BMD:

`create → Leased → start → Running → stop → Released`.

However, the acceptance gate returned after printing PASS and then fell through to the generic no-gate footer, which exits 2. This made the process verdict fail despite the Runtime lifecycle succeeding.

Minimal fix `96f8055`:
- successful FFmpeg Session gate exits 0;
- bmd+ffmpeg-only unused diagnostic variable is cfg-gated;
- a stale GStreamer-specific startup log is replaced with backend-neutral Session/MediaBackend ownership wording.

No Runtime contract or lifecycle semantics changed.
## GitHub CI

Actions `35344678284` at exact `96f8055...`: **7/7 required jobs PASS**.

- rust-format
- rust-clippy
- architecture-portability
- session-lifecycle
- rust-test-matrix
- gstreamer-build
- hardware-test-compile

The media runner therefore also compiled the `bmd-provider,ffmpeg-backend` path at the final SHA.

## BMD exact-commit verification

Exact archive SHA-256:
`880400611a073d8a2b5715c12e090d52365a250f19e80c1783f52765ed38561d`.

Manifest:
`/home/lytv/a2-8-02i-v5.manifest.json`, MD5 `7521d17e7fd02e50eb2b0a84374a43dd`.

Explicit input:
- DeviceHandle `46:00000000:002e4500`
- canonical DeviceId `4fa33dcb-5f76-5f76-aea8-330df7ada03e`
- canonical PortId `e43d8f5a-94f5-5c67-8ed3-cddee15df04a`

Production composition smoke:
- `bmd-provider,ffmpeg-backend` binaries built natively: PASS.
- production binary reported provider=blackmagic, backend=ffmpeg, authorized_devices=2, authorized_ports=2.
- production explicitly reported no automatic media start.
- no target FFmpeg process appeared during construction-only smoke.
Real SessionManager → FFmpeg gate:
- create: **Leased PASS**
- start: **Running PASS**
- backend actual-state observation: **alive / no terminal event**
- Resource: **Allocated while Running**
- stop: **Released PASS**
- Resource after stop: **Available**
- target Lease after stop: **NONE**
- close: Session removed
- gate rc: **0**
- FFmpeg leftover: **NONE**
- historical output device-number 2 PID: `992634 → 992634`

Final marker: `RF_FF_01E_BMD_ALL_PASS`.

Verification level: **SOFTWARE + CI + BMD HARDWARE VERIFIED**.

Evidence:
`evidence/bmd-10.30.15.10/2026-09-18-rf-ff-01e-session-composition/`.

## Next adjudication

The next correctness gap is not another device-binding slice. FFmpeg already implements canonical `observe/recover`, but the existing single-input ingest watchdog is compiled only for GStreamer and also owns GStreamer-specific `HEALTH_ARCS/appsink` acceptance bookkeeping.

Next bounded packet:
**RF-FF-01F — backend-neutral canonical event/recovery monitor extraction + FFmpeg failure-first recovery**.

01F must isolate the vendor-neutral `MediaBackend::observe → canonical event → Supervisor decision → lease revalidation → MediaBackend::recover` chain from GStreamer-specific media-health counters. It must preserve the existing GStreamer watchdog behavior and prove on BMD that a deliberately terminated FFmpeg child is detected, classified, recovered, and later cleanly stopped without orphan or resource/lease divergence.
