# RF-NORM-01 — explicit RAW Normalize execution

- Date: 2026-09-18
- Status: READY / PLAN FROZEN
- Branch: main
- Authority: .project/STATE.md §3.38/§4–§6; CANONICAL_MEDIA_MODEL.md; ARCHITECTURE_V0.2.md §3.3–§3.4; MEDIA_BACKEND_CONTRACT.md; live code and tests.

## 1. Comprehension proof

The live repository has three different meanings that must remain separate:

1. normalize.rs is a pure Raw → Canonical descriptor function. It does not build or mutate a media graph.
2. TimelinePolicy and program_timeline.rs describe/execute Program timestamp mapping. They are not format normalization.
3. The GStreamer Program graph currently feeds raw source branches into input-selector; FrameSwitch is the only executable switch policy. PacketSwitch and MasterSwitch fail closed.

The missing capability is therefore RAW video/audio format normalization before switching, not another clock or timestamp patch.

## 2. Objective

Add one explicit, backend-neutral Normalize execution contract and one bounded GStreamer implementation path for a fixed, documented Program RAW target. The packet must prove that the target was supplied, the conversion chain was materialized, and the resulting media was observed at the selector boundary.

This packet does not activate MASTER_SWITCH; it creates the prerequisite execution capability and keeps unsupported policies fail-closed.

## 3. Initial bounded target

The first target is the already exercised SDI acceptance shape, expressed without vendor fields:

- Video: video/x-raw, I420, 1920×1080, 25/1, interleaved.
- Audio: audio/x-raw, S16LE, 2 channels, 48000 Hz.

This is a bounded runtime target, not a claim that all formats or all color-management cases are implemented. No caller may silently receive this target through a missing/default field; production construction must pass it explicitly, and absence must fail closed.

## 4. Allowed scope

- Add typed target/plan/evidence domain objects with explicit video/audio planes.
- Materialize GStreamer conversion/caps elements before each selector input.
- Keep video and audio normalization independent while sharing the program target identity.
- Add mock/simulation tests for target absence, unsupported/missing element, caps mismatch, and successful evidence.
- Add a real GStreamer acceptance path for the canonical dual-input gate after software verification.

## 5. Forbidden scope

- No Network Source, SRS ownership, Recording/Replay, UI, SDK, or control-plane work.
- No FFmpeg adapter changes in this packet.
- No Packet/Master switch enablement, hot-standby, or multi-output expansion.
- No changes to Clock/Timecode, RH-FLOW-01, RH-IDEM-01, Session/Resource/Lease truth, or 24h RSS claims.
- No drop-backwards workaround, wall-clock PTS repair, or hidden target fallback.

## 6. Execution shape

The intended execution order is:

1. Explicit target validation in the runtime plan; missing target is an error.
2. Per-source video chain: source → video normalize elements → target caps → selector.
3. Per-source audio chain: source → audio normalize elements → target caps → selector.
4. Existing selector/timeline/fence/switch observation remains the owner of switch and timeline truth.
5. Normalize evidence records target identity and observed caps at the selector input boundary; evidence is not inferred from intent.

No Normalize completion fact may be emitted merely because elements were constructed.

## 7. Acceptance

- Software tests prove the target is explicit and both planes fail closed independently.
- Simulation proves the normalized branches negotiate the exact target caps and continue producing frames/samples.
- Existing default, simulation, mock, format, clippy, architecture, and remove-adapters checks remain green.
- CI required contexts are 7/7 green.
- Because the canonical GStreamer path changes, BMD exact-commit dual-input acceptance must be rerun before claiming the packet complete; output device-number 2 remains untouched.
- MASTER_SWITCH remains rejected until a later packet consumes the normalized evidence and defines its own acceptance.

## 8. Stop conditions

Stop and record UNKNOWN instead of guessing if the target cannot be represented by the existing canonical intent, if either media plane cannot negotiate the target, or if hardware/CI evidence is unavailable. Do not widen the packet to solve those conditions by adding new source types or external ownership.## 9. Implementation order

- Phase A: add the typed target and evidence model, wire explicit construction, and prove failure-first behavior.
- Phase B: add the smallest GStreamer element chain and simulation acceptance.
- Phase C: run full software/CI checks and inspect the diff against this packet.
- Phase D: build the exact commit on BMD, run the dual-input gate, archive evidence, and only then update the packet/STATE to COMPLETE.

The packet is frozen as one bounded unit. No second feature may be started until its verification and evidence are adjudicated.