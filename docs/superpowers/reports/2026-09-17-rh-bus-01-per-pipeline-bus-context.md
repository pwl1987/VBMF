# RH-BUS-01 — Per-pipeline GLib MainContext / Bus-watch isolation closure

Date: 2026-09-17

Task: `RH-BUS-01`

Implementation commit: `9b32004fa6b9d5f1d6492bd37b15070339f7b22d`
Status: **COMPLETE / SOFTWARE + BMD HARDWARE VERIFIED**

## 1. Problem and authority

STAB-O4 E4-1 exposed a production multi-input defect: `build_pipeline()` used the process-wide `glib::MainContext::default()` for every Bus-watch thread. With two concurrent pipelines, only one thread could own the default context; the other watch could disappear silently. BMD evidence showed the concrete symptom: one input had advancing `bus_msgs_total`, the second stayed at zero.

This packet is intentionally narrow. It restores per-pipeline Bus-watch isolation and lifecycle only. It does **not** change Execution Group Bus consumption, Supervisor policy, frozen `MediaBackend` SPI, command vocabulary, or recovery policy. Those remain `RH-BUS-02` scope.

## 2. Implementation

`services/media-agent/src/adapters/gstreamer/controller.rs` now gives every pipeline Bus thread a private `glib::MainContext::new()`.

Both GLib sources are explicitly attached to that same private context:

- `Bus::create_watch(...).attach(Some(&ctx))`
- `timeout_source_new(...).attach(Some(&ctx))`

The stop poll therefore drives the same private `MainLoop`, so `stop` / `recover` can terminate and rebuild the watch without a default-context dependency.

An intermediate attempt that only replaced `MainContext::default()` with `MainContext::new()` was rejected before commit: `Bus::add_watch()` and `timeout_add_local()` still attached to the default context, so the private loop had no sources and `stop` could hang on join. The committed form removes that half-fix.

## 3. Software verification

On the BMD host `/tmp` exact source tree, non-hardware self-test paths were executed before the hardware run:

- `cargo fmt --all -- --check` — PASS
- `cargo test --features gstreamer rh_bus_` — **2/2 PASS**
- `cargo test --features gstreamer` — **268/268 PASS**

The focused tests cover dual concurrent pipelines receiving independent Bus evidence, handle attribution, recover rebuilding the watch, and stop leaving no residual watch/channel state.

## 4. BMD exact-commit hardware acceptance

The Development VM produced `git archive 9b32004`; BMD verified archive SHA-256 `08f278e57e3a6511fd4f540eb897eb88bf5babdbbd0229324535a8198f3a255d` and built it with `--features bmd,gstreamer` in an isolated `/tmp` tree.

Runtime identity:

- exact implementation commit: `9b32004fa6b9d5f1d6492bd37b15070339f7b22d`
- manifest v5 MD5: `7521d17e7fd02e50eb2b0a84374a43dd`
- GStreamer: `1.28.2`
- DeckLink inputs: manifest device-number `0` and `1`
- existing manual output on device-number `2`: left running and observed alive after the test

A diagnostic two-input session reached `Capturing`. One 30 s `E4_INGEST_ANATOMY` snapshot contained both input handles:

| Handle | video buffers | audio buffers | pts_backward | bus_msgs_total |
|---|---:|---:|---:|---:|
| 1 | 722 | 725 | 0 | **65** |
| 2 | 722 | 725 | 0 | **65** |

This directly closes the historical same-shape symptom where the second handle had `bus_msgs_total=0`.

Lifecycle acceptance also passed: `stop_session` returned `executed`; Program Execution teardown and group-watchdog stop lines were present; `Bus watch` / `MainContext` failure matches were zero.

## 5. CI

GitHub Actions run `35214894696` at `9b32004…` completed **7/7 success**:

`session-lifecycle`, `rust-format`, `rust-test-matrix`, `hardware-test-compile`, `gstreamer-build`, `rust-clippy`, `architecture-portability`.

CI `hardware-test-compile` remains capability/build verification only; the BMD run above is the hardware authority for this packet.

## 6. Evidence and residual boundary

Evidence directory: `evidence/bmd-10.30.15.10/2026-09-17-rh-bus-01-per-pipeline-context/`; `sha256sums.txt` fingerprints the retained artifacts. The BMD canonical deployment checkout under `/opt/vbmf-dev/repo` was **not** reset or overwritten because it is old and contains local ops changes; acceptance used an exact-commit isolated `/tmp` build instead.

`RH-BUS-01` proves only that every pipeline has a live, independently dispatchable Bus watch with bounded lifecycle. Execution Group still does not drain each `(device_id, handle)` Bus stream into canonical fault handling, and fatal overflow fallback is still global. Those are the explicit entry conditions for `RH-BUS-02`.
