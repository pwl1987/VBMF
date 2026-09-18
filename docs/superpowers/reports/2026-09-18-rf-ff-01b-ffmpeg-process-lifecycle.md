# RF-FF-01B — FFmpegBackend process lifecycle / SelfTest

Date: 2026-09-18

## Decision

RF-FF-01B is complete at `4ad8135e8c4f1daa7c73cd193a888cc29a93f830`.
The packet implements only the first concrete FFmpeg `MediaBackend` slice: process ownership and canonical SelfTest lifecycle.
It does not open DeckLink, add a network source, change GraphRuntimeIntent/wire, or move Session/Resource truth.

## Implementation

- `7ebd551`: adds `ffmpeg-backend`, `FFmpegBackend`, registry construction, lifecycle tests, and CI feature coverage.
- One backend instance owns one mutex-protected `PipelineHandle -> Child` table.
- `instantiate/start/stop/recover/observe` implement the existing frozen `MediaBackend` SPI.
- Process launch uses `std::process::Command` plus backend-owned argv; no shell command string is accepted.
- stop/recover/drop reap owned children; tests assert old PIDs disappear.
- spawn failure, abnormal exit, duplicate start, unknown/never-started recover, and recover-spawn failure fail closed.
- only canonical `PipelinePlan::self_test()` is accepted in this packet.

## Architecture RCA

The first implementation run (`35312763403` @ `7ebd551`) failed the architecture-portability gate because registry named the concrete FFmpeg adapter type.
`4ad8135` fixes this by moving concrete construction behind `adapters::build_process_media_backend()` and returning only `Arc<dyn MediaBackend>` to protected orchestration code.
No frozen Architecture/Contract change was required.

## Verification
- Development VM focused FFmpeg lifecycle: 8 passed, 0 failed, 1 ignored real-binary smoke.
- Development VM full `ffmpeg-backend`: 261 passed, 0 failed, 1 ignored.
- Development VM default: 252 passed, 0 failed.
- Development VM mock: 436 passed + integration 9/9 + 12/12.
- `cargo fmt --manifest-path services/media-agent/Cargo.toml --all -- --check`: PASS.
- `cargo clippy --manifest-path services/media-agent/Cargo.toml --all-targets --features ffmpeg-backend -- -D warnings`: PASS.
- `git diff --check`: PASS.
- GitHub Actions `35313253516` @ exact `4ad8135`: all 7 required jobs PASS.
- BMD exact-commit real FFmpeg SelfTest: 1/1 PASS using installed `/usr/local/bin/ffmpeg`.
- BMD archive SHA-256: `90c11f7983b8711f78bc37eee0b92e6ccb48b09de0a3ae47cd5220d560a2eca0`.
- Existing output device-2 process PID remained `992634` before/after the smoke.

## Verification level

`SOFTWARE + CI + BMD RUNTIME SMOKE VERIFIED` for FFmpeg SelfTest lifecycle.
This is not DeckLink hardware verification: the FFmpeg smoke used lavfi video/audio to null output and did not open any DeckLink device.

Evidence: `evidence/bmd-10.30.15.10/2026-09-18-rf-ff-01b-ffmpeg-selftest/`.

## Next bounded packet

`RF-FF-01C — FFmpeg Resolved RuntimeBinding mapping + single-input BMD parity`.
The packet must keep backend-specific device addressing outside CanonicalPipelinePlan, must consume only already-resolved/authorized runtime resources, and must not self-enumerate or silently fall back to another device.
