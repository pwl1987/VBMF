# RF-FF-01D — Backend-neutral Session Binding Authorization

Date: 2026-09-18

## Result

RF-FF-01D is complete at runtime commit `dd0eb8ce5f6dd7925731b23cc44d542805ed43e3`.

This packet removes concrete backend runtime addresses from Session/Preflight/materialization truth. Session-level authorization now answers only whether a canonical Device has an accepted production-grade physical binding. GStreamer `device-number/persistent-id` and FFmpeg DeviceHandle remain concrete adapter/runtime-binding data.

No frozen Architecture, MediaBackend SPI, GraphRuntimeIntent, command/wire, Session ownership, or Resource ownership contract changed.

## Implementation

- `resolver::BindingAuthorization` carries only confidence/match semantics plus a persistent-identity evidence bit; it contains no backend runtime address.
- `authorizations_from_runtime_bindings` projects existing GStreamer RuntimeBinding into the neutral authorization view without copying addresses.
- `collect_authorizations_from_manifest` authorizes only exact live `RealBmd` + `blackmagic` DeviceHandle matches against the v2 provisioning manifest.
- Manifest authorization uses `DeviceHandleExact`; it deliberately does **not** claim `ManifestVerified`, which remains concrete GStreamer RuntimeBinding verification semantics.
- missing live Provider, ambiguous live identity, foreign Provider identity, weak authorization, and Production without authorization fail closed.
- `PortRegistry::build_authorized` supplies canonical Port/Resource facts without fabricating a GStreamer runtime address or signal probe.
- `SessionManager`, `PreflightInputs`, canonical runtime-state projection, and `materialize` consume only `BindingAuthorization`.
- Production Preflight requires authorization; Diagnostic mode may explicitly WARN and use its pre-existing diagnostic fallback.
- GStreamer gates keep their concrete bindings only at adapter execution points and project a neutral view into SessionManager.
- `check_arch_portability.py` now rejects any future `ResolvedDeviceBinding` dependency in `session.rs`, `preflight.rs`, or `runtime_state.rs`.

## Development VM verification

- focused RF-FF-01D: **6/6 PASS**.
- Session focused under mock: **31/31 PASS**.
- default: **258/258 PASS**.
- mock: **436/436** + integration **9/9 + 12/12 PASS**.
- ffmpeg-backend: **272 PASS + 1 ignored BMD hardware acceptance**.
- format check: PASS.
- clippy default/mock/ffmpeg-backend with `-D warnings`: PASS.
- architecture lexical gate: PASS.
- remove-adapters simulation + mock proof: PASS.
- `git diff --check`: PASS.

One implementation compile gap was caught during full regression: the new `PreflightInputs.require_authorization` field was missing at several test construction sites. It was fixed by explicitly preserving Diagnostic=false semantics; no test or safety gate was weakened.
## GitHub CI

Actions run `35342501850` at exact `dd0eb8c...`: **7/7 required jobs PASS**:

- rust-format
- gstreamer-build
- hardware-test-compile
- architecture-portability
- rust-clippy
- session-lifecycle
- rust-test-matrix

## BMD exact-commit regression

- exact archive SHA-256: `e622f08337477fbb7206a8353399279f926a6b8a6ed787ad70366850ed549875`.
- manifest: `/home/lytv/a2-8-02i-v5.manifest.json`, MD5 `7521d17e7fd02e50eb2b0a84374a43dd`.
- native `bmd,gstreamer` media-agent-gates build: PASS.
- A2-8 dual-input gate: **10/10 ALL PASS / rc=0**.
- production-grade input bindings: **2/2**; both signals locked.
- Session dual-input, MediaTap/Bridge, Program advancement, L4 switch/timeline, L5 failure isolation/recovery, and teardown: PASS.
- historical output DeckLink device-number 2 process PID remained `992634` before/after.
- leftover gate process: NONE.

Observed GStreamer interlace converter and pad-unlink CRITICAL diagnostics are preserved in evidence. They did not produce a gate verdict, frame/PTS, recovery, teardown, or process-leak failure in this exact run and are not reclassified here as an RF-FF-01D failure.

Verification level: **SOFTWARE + CI + BMD HARDWARE VERIFIED**.

Evidence: `evidence/bmd-10.30.15.10/2026-09-18-rf-ff-01d-binding-authorization/`.

## Next bounded packet

`RF-FF-01E — FFmpeg SessionManager / production composition-root single-input wiring`.

01E must reuse this neutral authorization projection plus existing Resource/Lease/Session owners, wire the existing FFmpeg adapter into the production composition root for one authorized SDI input, and prove real Session create/start/actual-state/stop on BMD. It must not expand to Network Source, output, or Program multi-input.
