# RF-SRC-RTMP-01 Verification Report

Date: 2026-09-19
Implementation: `ffd8889`
CI: `35407985671` — 7/7 required checks PASS.
Scope: one loopback RTMP source, one FFmpeg input, one Session lifecycle.

## Software

- mock: 463/463 passed.
- FFmpeg backend: 298 passed, 1 ignored.
- cargo check --all-targets: PASS.
- BMD feature build: PASS, no warnings; final binary SHA256 `17b55178cc9c6168eae86a710a6f9f3db1476187c770eeb32829d63aba4901e7`.

## BMD runtime

- Provider/backend: blackmagic/ffmpeg; manifest MD5 `7521d17e7fd02e50eb2b0a84374a43dd`.
- Loopback publisher delivered H.264/AAC; source Session reached Running.
- Final-code consumer recovery created a new process (`3436259` -> `3436455`) after publisher termination.
- Supervisor settled Recovered; teardown reached Released, Resource Available, Lease NONE, monitor exited, publisher orphan NONE.
- Output device 2 was not touched: PID `992634` and the exact command were unchanged after the run.
- Evidence: `evidence/bmd-10.30.15.10/2026-09-18-rf-src-rtmp-01/`.

## Boundary

- The BMD runtime fixture is bounded loopback ingress with HLS observation; no SRT, SRS, public RTMP server, multi-input, UI/API, output-device, or 24h claim is included.
