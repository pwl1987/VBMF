# RF-NORM-01 Phase A verification

- Date: 2026-09-18
- Commit: 324c70a3758deca5902e9b4d3311dab90fff349c
- Scope: explicit RAW Normalize target/plan/evidence Domain only.

## Result

Phase A is COMPLETE / SOFTWARE + CI VERIFIED. It does not claim GStreamer Normalize execution or BMD hardware verification.

## Implementation

- Added normalize_execution.rs as a vendor-neutral Domain module.
- NormalizeTarget carries explicit video and audio targets; no default target is constructed.
- NormalizePlan::require rejects a missing target and invalid dimensions/rates/channels.
- NormalizeEvidence tracks video and audio independently as Unobserved, ObservedExact, or ObservedMismatch.
- Completion requires exact evidence for both planes; intent or one plane cannot close the gate.
- The existing normalize.rs descriptor layer and program_timeline.rs timestamp layer remain separate.

## Development VM verification

- Focused normalize_execution tests: 5/5 PASS.
- Default library tests: 269/269 PASS.
- cargo fmt --all -- --check: PASS.
- cargo clippy --all-targets -- -D warnings: PASS after Rustdoc correction.
- architecture portability: PASS.
- remove-adapters proof: PASS.
- git diff --check: PASS.

## CI verification

- GitHub Actions run 35389637837: 7/7 required contexts success.
- rust-test-matrix included default, simulation, mock, and ffmpeg-backend paths.
- hardware-test-compile and gstreamer-build compiled the touched tree; no hardware runtime claim is made.

## Boundary

Phase B remains: materialize the per-plane GStreamer Normalize chain and observe exact target caps at the selector boundary. MASTER_SWITCH remains fail-closed. BMD exact-commit dual-input acceptance is required after the canonical GStreamer graph is changed; output device-number 2 must remain untouched.