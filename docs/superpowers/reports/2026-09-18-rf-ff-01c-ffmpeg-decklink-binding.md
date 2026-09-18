# RF-FF-01C — FFmpeg Resolved RuntimeBinding + single-input BMD parity

Date: 2026-09-18

## Result

RF-FF-01C is complete at runtime commit `ffee3a057d7d4bebde26a7bf9a6294501047a021`.
The packet adds an adapter-local FFmpeg DeckLink input mapping without changing the canonical PipelinePlan or frozen MediaBackend SPI.

## Architecture

- FFmpeg does not consume GStreamer `device-number` as its device address.
- BMD `ffmpeg -sources decklink` reports inputs by Blackmagic DeviceHandle, e.g. `46:00000000:002e4500`.
- The FFmpeg adapter therefore derives a private `DeviceId -> DeviceHandle` view from the authorized v2 provisioning manifest plus live Blackmagic Provider identity.
- The mapping is accepted only for manifest-declared Input/Bidirectional ports and an exact single live Provider match.
- missing, ambiguous, output-only, non-SDI, malformed canonical ID, unsupported binding class, and output plans fail closed.
- no shell command string, runtime device enumeration, guessed fallback, Network Source, output path, GraphRuntimeIntent change, Session truth change, or Resource truth change is introduced.

## Implementation

- `services/media-agent/src/adapters/ffmpeg.rs`: Resolved input validation, private DeviceHandle mapping, DeckLink argv, failure-first tests, ignored BMD acceptance.
- `services/media-agent/src/adapters/mod.rs`: concrete FFmpeg constructor remains inside adapter layer.
- `services/media-agent/src/registry.rs`: vendor-neutral trait-object constructor for manifest-authorized FFmpeg backend.
- `.github/workflows/media-agent.yml`: media runner compiles `bmd-provider,ffmpeg-backend` tests.
- `services/media-agent/Cargo.toml`: feature description updated; no new FFmpeg Rust binding dependency.
## Development VM verification

- focused `ffmpeg_rt_02`: 5/5 PASS.
- full `ffmpeg-backend`: 266 PASS, 1 ignored real-hardware acceptance.
- default: 252/252 PASS.
- mock: 436/436 + integration 9/9 + 12/12 PASS.
- format, ffmpeg/mock clippy `-D warnings`, `git diff --check`: PASS.
- `check_arch_portability.py`: PASS.
- `check_remove_adapters.py`: PASS for simulation + mock.

## GitHub CI

Actions run `35339165383` at exact `ffee3a0...`: 7/7 required jobs PASS.
The `hardware-test-compile` job additionally passed `cargo test --no-run --features bmd-provider,ffmpeg-backend` on the media runner with DeckLink SDK available.

## BMD exact-commit verification

- exact archive SHA-256: `384d798791174bc851116202f401d8b41ff82dabdee24e910ce59c0da90022e7`.
- authorized manifest: `/home/lytv/a2-8-02i-v5.manifest.json`.
- explicit input: `46:00000000:002e4500` (`SDI-IN-1`).
- direct FFmpeg frame probe auto-detected 1920x1080 25.00i, embedded stereo audio, captured 50 video frames in 2.00s, rc=0.
- VBMF ignored acceptance `ffmpeg_rt_02_real_decklink_resolved_binding_lifecycle`: 1/1 PASS, covering start -> observe -> recover -> observe -> stop.
- no FFmpeg process remained after stop.
- historical DeckLink output device 2 process PID remained `992634` before and after.

Verification level: **SOFTWARE + CI + BMD HARDWARE VERIFIED** for RF-FF-01C exact runtime commit.

Evidence: `evidence/bmd-10.30.15.10/2026-09-18-rf-ff-01c-decklink/`.
## Next bounded packet

`RF-FF-01D — FFmpeg SessionManager / production composition-root single-input wiring`.

The next packet must reuse existing Session/Resource/Preflight truth. It must not bypass ResourceRegistry or introduce a second binding authority. Before implementation, reconcile the current GStreamer-specific `ResolvedDeviceBinding` / PortRegistry construction dependency and define the smallest backend-neutral authorization projection required by SessionManager materialization.
