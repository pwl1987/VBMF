# RH-BUS-02 — Multi-input Bus event consumption / fault isolation closure

Date: 2026-09-18

Task: `RH-BUS-02`

Implementation commit: `92d60075161ef080b599ce69a89a482b3798786b`
Status: **COMPLETE / SOFTWARE + BMD HARDWARE VERIFIED**

## 1. Problem and authority

RH-BUS-01 restored an independently live GStreamer Bus watch for every pipeline, but the Execution Group watchdog still did not drain each input handle. Fatal overflow evidence also used one process-global sticky slot. In a multi-input session this left two correctness gaps: Bus Error/EOS could fail to enter the canonical RuntimeEvent→Supervisor path, and one input could overwrite another input's fatal overflow evidence.

This packet closes those gaps without changing Supervisor policy, switch policy, frozen `MediaBackend` SPI, command vocabulary, or recovery policy. Runtime remains the single truth owner; raw GStreamer Bus facts are observations only and cannot become a second decision path.

## 2. Implementation

Commit `92d6007` changes three Runtime files: `controller.rs`, `pipeline.rs`, and `watchdog.rs`.

- Every `GstInstance` owns a `fatal_fallback` slot; successful Bus channel sends do not write the slot, and only Error/EOS overflow writes it.
- `poll_bus(handle)` drains that handle's channel, then atomically takes only that handle's fallback. Stop/recover destroys and recreates the slot with the instance.
- Execution Group watchdog calls `ctrl.observe(handle)` once per `(device_id, handle)` per tick and rejects handle mismatches fail-closed instead of re-attributing them.
- Error/EOS are normalized through `bus_fatal_observation()` and existing `Supervisor::ingest`, producing canonical exact-device `PipelineFault` events before the intake drain.
- Supervisor decision candidates are derived only from drained canonical RuntimeEvents plus existing fold actions; raw Bus events cannot directly call `report_failure`.
- Per-device union/dedup guarantees one device gets at most one `report_failure` per tick while independent A+B failures remain independent decisions.
- ClockLost keeps the frozen degraded/no-auto-restart policy; Warning/StateChanged remain observation-only.

## 3. Software verification

BMD `/tmp` exact source tree verification for commit `92d6007`:

- `cargo fmt --all -- --check` — PASS
- RH-BUS focused GStreamer tests — **12/12 PASS**
- full `cargo test --features gstreamer` — **278/278 PASS**
- coordinator-side default regression — PASS
- coordinator-side mock regression — PASS

The deterministic RH-BUS-02 tests cover Error/EOS exact-device canonicalization, ClockLost no-restart semantics, mismatched-handle rejection, canonical-event ownership, same-device dedup, independent A/B failure candidates, Bus counters/acceptance folding, per-handle fatal overflow fallback, successful-send no-duplicate semantics, and nonfatal overflow behavior.

## 4. BMD exact-commit hardware acceptance

The hardware acceptance used an isolated `git archive` of implementation commit `92d60075161ef080b599ce69a89a482b3798786b`, not the old `/opt/vbmf-dev/repo` deployment checkout.

Identity evidence:

- archive SHA-256: `1007ac91de95a861382488abfc5051f660eb96b56e981951d94faa403504536d`
- manifest v5 MD5: `7521d17e7fd02e50eb2b0a84374a43dd`
- GStreamer: `1.28.2`
- Rust/Cargo on BMD: `1.98.0`
- DeckLink inputs: manifest device-number `0` and `1`
- pre-existing manual output on device-number `2` remained running after acceptance

The existing A2-8 dual-input gate completed **10/10 PASS** through L1–L5 + teardown: both inputs were bound/locked and advancing; Program advanced; A→B switch/timeline continuity passed; A failure left B alive and Program advancing; A recovery restored its bridge; B failure left A alive; fault-domain attribution did not cross inputs; final teardown released the session and resources.

A separate production `media-agent` run proved the group watchdog itself consumed both live Bus streams: the first-observation log appeared for handle 1 and handle 2, and the same 30 s E4 snapshot reported `bus_msgs_total=65/65` with both video/audio streams advancing and `pts_backward=0`.

`stop_session` returned `executed`. The post-stop runtime snapshot showed the session in `released`, both input resources `available`, inputs/outputs empty, and `program_switch=null`. Program teardown and group-watchdog exit were present in the service log.

## 5. CI infrastructure incident during closure

The first CI closure attempt did not reveal a Runtime regression. Self-hosted runners repeatedly failed in `Set up job` while downloading immutable Action archives from `codeload.github.com`; one representative failure was `actions/cache` timing out at 100 seconds on three consecutive attempts.

The mitigation was hardened rather than hidden by reruns:

- external Actions in `media-agent.yml` are pinned to immutable commit SHAs;
- self-hosted runners now use the GitHub Runner-supported `ACTIONS_RUNNER_ACTION_ARCHIVE_CACHE` layout `<owner_repo>/<resolved_sha>.tar.gz`;
- dispatch-only `ci-runner-action-cache.yml` contains zero external Actions and can repair/prewarm the cache even during a codeload outage;
- `ci-infra-probe.yml` validates all four immutable archives and permits only direct mode or credential-free loopback HTTP proxy fallback;
- all three runners were populated and restart/probe verified; `vbmf-ci-02` passed in direct mode using the local archive cache.

Runtime implementation commit `92d6007` remains the BMD hardware authority. Later CI-infrastructure commits do not inherit or rewrite that hardware evidence.

## 6. Evidence

Evidence directory: `evidence/bmd-10.30.15.10/2026-09-17-rh-bus-02-multi-input-fault-isolation/`.

`sha256sums.txt` fingerprints 14 retained artifacts including focused/full software logs, dual-input gate output, production watchdog output, runtime/health snapshots, stop request/response, exact identity, and device-2 preservation evidence.

## 7. Required CI closure

After the runner archive-cache hardening was deployed and verified on all three self-hosted runners, GitHub Actions run `35248227420` at tree commit `94bf34e659fadc8c8b58a3a252fd36b53fc177db` completed **7/7 success**: `rust-format`, `rust-test-matrix`, `rust-clippy`, `session-lifecycle`, `hardware-test-compile`, `architecture-portability`, and `gstreamer-build`.

The previously failing `rust-test-matrix` path crossed `Set up job`, resolved the pinned `checkout`, `rust-toolchain`, and `cache` Actions from the local immutable archive cache, and completed default/simulation/mock builds and tests plus artifact upload successfully.

No Runtime source changed after implementation commit `92d6007` before this CI closure. Commits `f05ec60`, `86669b7`, and `94bf34e` are CI/infrastructure-only. Therefore BMD hardware evidence remains attributed to exact Runtime commit `92d6007`; run `35248227420` proves the current main tree retains the required software/CI gates without re-labeling later commits as hardware-verified.
