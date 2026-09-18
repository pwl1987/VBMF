# RF-SRC-RTMP-01 — Network Source boundary design

- Date: 2026-09-18
- Status: DESIGN FROZEN; implementation packet READY after review
- Authority: IMPLEMENTATION_BOUNDARIES §3–§5.2; MEDIA_BACKEND_CONTRACT §1/§1.1/§4;
  RUNTIME_BINDING_MODEL; RUNTIME_RESOURCE_MODEL; RUNTIME_SESSION_MODEL;
  live SourceIntent/PipelinePlan/Resource/Lease/SessionManager and RF-FF evidence.
- Scope: one RTMP source, one FFmpeg input, one bounded loopback fixture.

## 1. Problem and decision

The live runtime uses device_id as four different things: source identity,
resource owner lookup, device lease key and SessionInput identity. A network URL
cannot occupy that field. The design therefore introduces explicit canonical
source/resource/input references and keeps backend runtime addresses private.

DeckLink intent JSON remains semantically and structurally compatible. Existing
device source constructors are migrated to the typed Device variant. RTMP uses a
NetworkSourceId and a validated endpoint value; it never creates a fake DeviceId.
## 2. Canonical types

1. SourceRef:
   Device(DeviceId) | NetworkSource(NetworkSourceId) | SelfTest.
2. SourceIntent:
   a tagged source variant. Device preserves kind/device_id/port_id JSON;
   Network carries kind=rtmp, source_id and endpoint without credentials.
3. PipelinePlan:
   carries CanonicalSource plus SourceBindingClass. It never carries
   device-number, DeckLink handle, FFmpeg argv, or GStreamer properties.
4. ResourceOwner:
   Device(DeviceId) | NetworkSource(NetworkSourceId). ResourceRegistry remains
   the only owner of Resource state and derives network input resources explicitly.
5. LeaseKey:
   Device(DeviceId) | NetworkSource(NetworkSourceId). LeaseManager remains the
   only owner of exclusive claims; existing DeviceLease behavior is preserved
   through the Device key path.
6. SessionInput:
   source_ref: SourceRef + opaque PipelineHandle. No device_id field is used
   for network input.
## 3. Binding and lifecycle

- RuntimeBindingRef points from SourceRef to an authorized RuntimeResourceRef;
  it contains no backend runtime address in canonical state.
- The composition root registers the NetworkSource resource and authorized
  binding before SessionManager::create. Backend must not discover or allocate it.
- Preflight resolves SourceRef → ResourceOwner → ResourceId and rejects missing,
  duplicate, wrong-kind or unavailable resources without side effects.
- Create acquires the LeaseKey and Reservation for the exact source/resource;
  Device and NetworkSource claims cannot collide accidentally.
- Start materializes the canonical source, instantiates the existing FFmpeg
  backend, allocates the exact claim, starts the child, and stores SessionInput.
- Stop/recover use the same SessionManager journal and release path; no network
  special-case cleanup exists outside the owner.
- Canonical events carry SourceRef/ResourceId facts; no URL, password or token is
  emitted in logs, health projections or evidence.
## 4. Compatibility and migration rules

- Existing DeckLink serialized intent must round-trip with the same fields and
  values; omitted optional port_id remains equivalent to None.
- Existing Session/Program consumers migrate from SessionInput.device_id to
  SessionInput.source_ref. Device callers receive the same DeviceId projection.
- Resource and Lease public snapshots retain a compatibility projection for
  device-backed entries during this packet; NetworkSource entries are explicit
  and never represented as nil-device fallbacks.
- No endpoint is accepted from an untyped string at the FFmpeg adapter boundary.
  Parsing, scheme/host/port/path validation and credential rejection happen before
  RuntimeBinding creation.
- RTMP acceptance uses loopback only. Public targets, SRS ownership, credentials,
  and multi-source policy are not introduced.
## 5. Implementation packet

Allowed files are limited to:
- graph_intent.rs and adjacent contract tests;
- pipeline.rs and materialization tests;
- resource.rs, lease.rs and session.rs ownership adapters;
- preflight.rs and canonical source/resource validation;
- FFmpeg adapter and existing bounded RTMP lifecycle gate;
- necessary module exports, architecture tests and STATE/evidence documents.

Forbidden:
- RTMP/HLS/SRT/RTSP source variants beyond the one RTMP variant;
- control-plane API, UI, SRS, public credential store, multi-input, failover,
  switch policy, Clock/Flow/Idempotency, output device-number 2 or 24h work;
- changing existing Device/Port identity semantics or adding backend fields to
  canonical types.
## 6. Acceptance

| Gate | Required result |
|---|---|
| Wire compatibility | existing DeckLink intent semantic round-trip unchanged |
| Type boundary | NetworkSourceId cannot enter DeviceId/PortId/DeviceLease paths |
| Resource | explicit Network resource owner, reservation/allocation/release lifecycle |
| Lease | NetworkSource lease conflict/release/renew uses canonical LeaseKey |
| Session | SessionInput carries SourceRef; start/stop/recover has no orphan |
| Security | endpoint validation; no credential leakage in state/log/evidence |
| Backend | FFmpeg owns RTMP argv; no shell or fallback; existing paths regress zero |
| Runtime | loopback RTMP delivers video and audio to one input |
| Recovery | producer failure and consumer recovery produce canonical events |
| CI | all 7 required contexts PASS on exact implementation commit |
| Boundary | no other network protocol, SRS or output-device claims |

## 7. Stop conditions

Return to PLAN REQUIRED if preserving DeckLink JSON requires changing its meaning,
if Resource/Lease ownership cannot be made explicit without a second state store,
if SessionManager must be bypassed, or if runtime evidence needs a fake success
path. No code is accepted until focused type/ownership tests pass.
## 8. Ready packet handoff

This design is the sole authority for the next implementation packet:
RF-SRC-RTMP-01-IMPLEMENTATION. The implementation starts with contract and
ownership tests, then materialization/preflight, then FFmpeg argv/lifecycle,
then loopback/recovery/teardown. Every stage must keep the working tree and
STATE synchronized; no parallel Runtime Features packet is opened.
