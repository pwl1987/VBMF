# RF-SRC-01 — single-input SRT source boundary and FFmpeg runtime

- Date: 2026-09-18
- Status: BLOCKED at capability gate; implementation not started
- Branch: main
- Authority: MEDIA_BACKEND_CONTRACT §1/§1.1/§4; RUNTIME_BINDING_MODEL;
  live GraphRuntimeIntent, PipelinePlan, FFmpegBackend, Session/RecoveryMonitor;
  RF-FF-01A–01F and RF-FF-03 evidence.

## 1. Selection decision

The next bounded Runtime Features packet is one SRT input path, consumed by the
existing single-input FFmpeg Session. This is intentionally narrower than the
historical “Network Sources” umbrella: one protocol, one input, one backend, one
loopback acceptance fixture.

The live source contract is still device-bound (device_id/port_id), while the
Backend contract requires canonical source semantics plus an authorized runtime
binding. Adding a URL field to the existing device source or teaching FFmpeg to
interpret GStreamer fields would create a second Runtime truth. This packet first
introduces an explicit network-source boundary and then consumes it in FFmpeg.
## 2. Candidate comparison

- Network umbrella: BACKLOG; mixes SRT/RTMP/HLS/RTSP, decoder policy, source
  health, credentials and failover. Too broad.
- PACKET/MASTER switch: requires compressed/normalized program evidence and is
  not a source-entry substitute.
- Hot-Standby: requires multiple source candidates and failover ownership.
- SRT single-input: existing FFmpeg lifecycle, canonical failure events and
  recovery monitor can be reused; loopback producer/receiver gives bounded
  acceptance. Selected.
- Warning cleanup / 24h RSS: independent debts, not a reason to widen this
  feature packet.

## 3. Allowed scope

1. Freeze a backend-neutral source model that preserves existing DeckLink JSON
   compatibility and adds a tagged Network/SRT intent without vendor fields.
2. Materialize SRT intent into an authorized network RuntimeBinding; the canonical
   plan carries source semantics/reference, never FFmpeg argv or GStreamer fields.
3. Add FFmpeg-only SRT input construction. The adapter owns argv and process
   lifecycle; no shell interpolation, guessed fallback, or automatic backend
   fallback.
4. Reuse RF-FF-01F Session/RecoveryMonitor, canonical observe/error path and
   RF-FF-03-style bounded loopback producer. Only one producer and one input.
5. Add fail-closed validation for scheme, host/port, unsupported query fields,
   empty endpoint, credentials/log redaction, and missing authorization.
6. Preserve DeckLink/GStreamer/Mock behavior byte-for-byte in meaning and keep
   the existing no-output/appsink path unchanged.
## 4. Forbidden scope

- No RTMP/HLS/RTSP/WebRTC/RTP/UDP source in this packet.
- No Network Source failover, Hot-Standby, source switching, PACKET_SWITCH or
  new MASTER_SWITCH behavior.
- No SRS ownership, public network service, recording/replay, transcoding policy,
  multi-input, composition, metadata/audio-flow expansion, or UI/control-plane
  command expansion beyond the minimum typed intent needed for SRT.
- No secrets in GraphRuntimeIntent, PipelinePlan serialization, logs, evidence or
  command output; no credential management system is introduced.
- No changes to Session/Resource/Lease ownership, Clock/Timecode, idempotency,
  output device-number 2, BMD manifest, or 24h rss_bounded claims.
- No acceptance claim based only on FFmpeg argv/unit tests; runtime producer,
  receiver, recovery and teardown evidence are required.

## 5. Implementation shape

1. Inventory all SourceIntent constructors and runtime projections; preserve the
   current DeckLink wire shape through explicit serde compatibility tests.
2. Add a canonical SRT source descriptor and an authorization-bound runtime view.
   Keep endpoint/reference semantics separate from backend-specific argv.
3. Extend materialization and preflight to reject malformed or unauthorized SRT
   sources before Session creation; make unsupported backends fail closed.
4. Extend FFmpeg command construction with a controlled SRT input branch. Keep
   the existing DeckLink and SelfTest branches unchanged.
5. Reuse the existing process monitor: initial source frames, producer restart,
   consumer recovery with a new child/generation, and clean stop.
6. Add focused software tests before any product/runtime gate. If the contract
   shape requires more than the above files or exposes a new owner, stop and
   return to PLAN REQUIRED.
## 6. Acceptance matrix

| Gate | Required result |
|---|---|
| Contract | DeckLink JSON round-trip unchanged; SRT is typed and vendor-neutral |
| Admission | malformed/unsupported/unauthorized endpoint rejected before spawn |
| Security | no password/token in serialized plan, logs, argv evidence or errors |
| Backend | FFmpeg SRT argv is owned by adapter, no shell, exact controlled URL |
| Software | focused SRT tests; default/simulation/mock/ffmpeg matrices; recovery tests |
| CI | all 7 required contexts PASS on exact implementation commit |
| Runtime | loopback SRT producer delivers video+audio to one FFmpeg input |
| Recovery | producer interruption/restart produces canonical failure and recovered generation |
| Teardown | producer/consumer/monitor exit; no orphan; existing lease/resource closure |
| Boundary | no DeckLink output device-number 2 touch; no claims for other protocols |

## 7. Capability and stop gates

Before implementation, run a read-only capability probe on the BMD/runtime image:
FFmpeg must expose SRT input support and the required mux/demux path. The probe
is not acceptance. If SRT support, safe loopback binding, or A/V source generation
is unavailable, record the blocker and do not substitute RTMP or claim Network
Source support.

Stop with UNKNOWN if the source-to-runtime binding cannot be represented without
putting backend addresses or credentials into the canonical plan, if the existing
Session/RecoveryMonitor owner must be duplicated, or if producer failure cannot
be distinguished from consumer failure in canonical events.

## 8. Required files / verification

Expected implementation files are limited to graph_intent.rs, pipeline.rs,
preflight.rs, the FFmpeg adapter/contract tests, and the existing bounded gate;
exact list is confirmed after the inventory step. No API, web, SRS or deployment
files are expected. Required output is this plan, implementation report, exact
software/CI/runtime evidence, and .project/STATE.md synchronization.
Completion requires implementation, focused/default/simulation/mock/FFmpeg
verification, exact 7/7 CI, controlled loopback runtime evidence, clean recovery
and teardown, evidence archive, and GitHub/main/STATE synchronization. This
packet does not close the historical Network Sources umbrella; it closes only
SRT single-input FFmpeg admission and lifecycle if every gate passes.
