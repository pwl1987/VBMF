# RF-FF-01F — FFmpeg failure-first recovery monitor

Date: 2026-09-18

## Result

RF-FF-01F passes at exact implementation commit `f868420cedca2ba38ece320ef2c9b60e48e0b6e1`.

The new backend-neutral `RecoveryMonitor` consumes `MediaBackend::observe`, maps Error/EOS to the existing canonical RuntimeEvent path, lets Supervisor make the single restart/escalate decision, revalidates Lease before recovery, and calls `MediaBackend::recover` on the same PipelineHandle. Session stop owns cancellation and join through the existing SessionStopHook.

No GStreamer symbol, device-number, vendor runtime address, second Session/Resource/Lease owner, or new wire event was added to the neutral monitor. Existing GStreamer watchdog behavior remains unchanged.

## Development VM

- default tests: 261 passed
- simulation tests: 261 passed
- mock tests: 446 passed; integration 9/9 and 12/12
- ffmpeg-backend tests: 278 passed, 1 ignored existing real-binary hardware test
- clippy default/mock/ffmpeg-backend: PASS with `-D warnings`
- `cargo fmt --all -- --check`: PASS
- ARCH-PORTABILITY-01 lexical gate: PASS
- remove-adapters compile proof: PASS
- GitHub Actions `35356652348`: 7/7 required jobs PASS, including architecture portability, session lifecycle, full test matrix, clippy, GStreamer build, and hardware compile.

## BMD exact-commit gate

Native release build with `bmd-provider,ffmpeg-backend`: PASS.
Manifest: `/home/lytv/a2-8-02i-v5.manifest.json`, MD5 `7521d17e7fd02e50eb2b0a84374a43dd`.
Explicit input: `46:00000000:002e4500`.

The gate created one real Session, started one FFmpeg child, externally terminated that child, observed the canonical failure, restarted the same PipelineHandle with a new child, then stopped and closed the Session. The final marker was `RF_FF_01F_BMD_RECOVERY_PASS`; teardown reported `Released`, `Available`, `Lease=NONE`, monitor exited, and no FFmpeg orphan.

Output device-number 2 was not touched. The stale `/opt/vbmf-dev/repo` deployment was not used or modified.

Evidence: `evidence/bmd-10.30.15.10/2026-09-18-rf-ff-01f-recovery/`.

## Adjudication

RF-FF-01F is complete and the single-input Live FFmpeg failure-first slice is accepted. No additional recovery/health follow-up is required for this slice. The next task is to formally select and decompose the next bounded Runtime Features packet; do not begin Network Source/Output/Program multi-input expansion before that packet is READY.