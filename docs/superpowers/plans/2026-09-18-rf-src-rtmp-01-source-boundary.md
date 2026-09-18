# RF-SRC-RTMP-01 — single-input RTMP source boundary and FFmpeg runtime

- Date: 2026-09-18
- Status: PLAN REQUIRED after boundary inventory; implementation not started
- Branch: main
- Authority: MEDIA_BACKEND_CONTRACT §1/§1.1/§4; RUNTIME_BINDING_MODEL;
  live GraphRuntimeIntent, PipelinePlan, FFmpegBackend, Session/RecoveryMonitor;
  RF-FF-01A–01F and RF-FF-03 evidence.
- Capability prerequisite: BMD FFmpeg exposes RTMP input/output; SRT is blocked
  and remains separately recorded in RF-SRC-01.

## 1. Selection decision

The next feasible bounded Runtime Features packet is one RTMP input path, consumed
by the existing single-input FFmpeg Session. It is narrower than the historical
Network Sources umbrella: one protocol, one input, one backend, one loopback
producer/consumer fixture.

The live source contract is still device-bound (device_id/port_id), while the
Backend contract requires canonical source semantics plus an authorized runtime
binding. Adding a URL to the existing device source or teaching FFmpeg to
interpret GStreamer fields would create a second Runtime truth. This packet first
introduces an explicit network-source boundary and then consumes it in FFmpeg.
## 2. Candidate comparison

- SRT source: BLOCKED by the exact BMD capability probe; no SRT protocol exposed.
- Network umbrella: BACKLOG; mixes protocols, decoder policy, credentials and
  failover. Too broad.
- RTMP single-input: the exact runtime already has RTMP protocol support; the
  existing RF-FF-03 loopback receiver/producer and RF-FF-01F monitor give a
  bounded path. Selected.
- Hot-Standby or source switching: requires multiple candidates and ownership not
  present in this packet.
- Warning cleanup / 24h RSS: independent debts, not part of source admission.
## 3. Allowed scope

1. Preserve existing DeckLink JSON compatibility and add a tagged, vendor-neutral
   Network/RTMP source intent.
2. Materialize RTMP intent into an authorized network RuntimeBinding; the canonical
   plan carries source semantics/reference, never FFmpeg argv or GStreamer fields.
3. Add FFmpeg-only RTMP input construction. The adapter owns argv and process
   lifecycle; no shell interpolation, guessed fallback, or automatic backend
   fallback.
4. Reuse RF-FF-01F Session/RecoveryMonitor, canonical observe/error path and the
   RF-FF-03-style bounded loopback producer. Only one producer and one input.
5. Validate scheme, loopback host/port for the acceptance fixture, path, unsupported
   query fields, empty endpoint, credential redaction and missing authorization.
6. Preserve DeckLink/GStreamer/Mock behavior in meaning and keep existing
   no-output/appsink behavior unchanged.
## 4. Forbidden scope

- No SRT/HLS/RTSP/WebRTC/RTP/UDP source in this packet.
- No source failover, Hot-Standby, source switching, PACKET_SWITCH or new
  MASTER_SWITCH behavior.
- No SRS ownership, public network service, recording/replay, transcoding policy,
  multi-input, composition, metadata/audio-flow expansion, or UI/API expansion
  beyond the minimum typed intent required for RTMP.
- No secrets in intent, plan serialization, logs, evidence or command output; no
  credential management system is introduced.
- No changes to Session/Resource/Lease ownership, Clock/Timecode, idempotency,
  output device-number 2, BMD manifest, or 24h rss_bounded claims.
- No acceptance based only on argv/unit tests; runtime A/V, recovery and teardown
  evidence are required.
## 5. Implementation shape

1. Inventory all SourceIntent constructors and runtime projections; preserve the
   current DeckLink wire shape through explicit serde compatibility tests.
2. Add a canonical RTMP source descriptor and authorization-bound runtime view.
   Keep endpoint/reference semantics separate from backend-specific argv.
3. Extend materialization and preflight to reject malformed or unauthorized RTMP
   sources before Session creation; unsupported backends fail closed.
4. Extend FFmpeg command construction with a controlled RTMP input branch. Keep
   existing DeckLink and SelfTest branches unchanged.
5. Reuse the process monitor: initial producer frames, producer interruption,
   consumer recovery with a new generation, and clean stop.
6. Add focused software tests before the runtime gate. If the canonical binding
   requires a new owner or a broader contract migration, stop at PLAN REQUIRED.
## 6. Acceptance matrix

| Gate | Required result |
|---|---|
| Contract | DeckLink JSON round-trip unchanged; RTMP is typed and vendor-neutral |
| Admission | malformed/unsupported/unauthorized endpoint rejected before spawn |
| Security | no password/token in serialized plan, logs, argv evidence or errors |
| Backend | FFmpeg RTMP argv is adapter-owned, no shell, exact controlled URL |
| Software | focused RTMP tests; default/simulation/mock/ffmpeg matrices; recovery tests |
| CI | all 7 required contexts PASS on exact implementation commit |
| Runtime | loopback RTMP producer delivers video+audio to one FFmpeg input |
| Recovery | producer interruption/restart yields canonical failure and recovered generation |
| Teardown | producer/consumer/monitor exit; no orphan; resource/lease closure remains |
| Boundary | no DeckLink output device-number 2 touch; no claims for other protocols |

## 7. Capability and stop gates

The BMD read-only probe has already confirmed RTMP input/output protocol support.
Before implementation, confirm the required FLV/H.264/AAC path and controlled
loopback listener behavior. These are capability checks, not acceptance.

Stop with UNKNOWN if source-to-runtime binding requires backend addresses or
credentials in the canonical plan, if Session/RecoveryMonitor ownership must be
duplicated, or if producer failure cannot be distinguished from consumer failure
in canonical events. If the loopback fixture cannot carry both A/V streams,
do not downgrade acceptance to video-only.
## 8. Required files / verification

Expected implementation files are limited to graph_intent.rs, pipeline.rs,
preflight.rs, the FFmpeg adapter/contract tests, and the existing bounded gate;
exact list is confirmed after inventory. No API, web, SRS or deployment files are
expected. Required output is this plan, implementation report, exact software/CI/
runtime evidence, and .project/STATE.md synchronization.

Completion requires implementation, focused/default/simulation/mock/FFmpeg
verification, exact 7/7 CI, controlled loopback A/V evidence, clean recovery and
teardown, evidence archive, and GitHub/main/STATE synchronization. This packet
closes only RTMP single-input FFmpeg admission/lifecycle, not Network Sources as
a whole.
