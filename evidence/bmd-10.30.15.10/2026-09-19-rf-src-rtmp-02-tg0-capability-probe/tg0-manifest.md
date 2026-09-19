# TG-0 Capability Probe — Evidence Manifest

- Packet: `RF-SRC-RTMP-02-IMPLEMENTATION` Step 0 / TG-0 (read-only capability probe)
- Date: 2026-09-19 (BMD box clock ~15:59)
- Design authority: `docs/superpowers/plans/2026-09-19-rf-src-rtmp-02-production-rtmp-input-boundary.md` (exact commit `bf0a8e0`)
- BMD host: `lytv@10.30.15.10` (eno1 `10.30.15.10/16`), FFmpeg `/usr/local/bin/ffmpeg` `git-2026-08-23-1019f8f-+allcodec-20260824`
- rtmp present in both Input/Output protocol lists (`rtmp-options.txt` captures RTMP AVOptions)
- Isolation: all listeners/publishers used isolated ports 19351/19352/19361-19365 and lavfi-generated media only; no DeckLink argument appeared in any command; device-2 output process PID `992634` alive before/after (`gst-after.txt`); zero ffmpeg leftovers (`ffmpeg-leftover.txt` = NONE); zero listening-port residue (`ss-after.txt` vs `ss-before.txt`); remote temp dir removed after evidence fetch.

## Probe ① — non-loopback LAN listener bind + same-host push: **PASS**

- Listener bind evidence (`p1clean-bind.txt`): `LISTEN 0 1 10.30.15.10:19352 0.0.0.0:*` — **explicit LAN address binding, not wildcard, not loopback**.
- Same-host publisher (testsrc2+sine → h264+aac/flv, 3 s): `publisher_rc=0 listener_rc=0` (`p1clean-target.txt`).
- A/V reception: listener log `p1clean-listener.log` — video+audio packets muxed to mpegts, output `p1clean-out.ts` 244776 bytes.
- Note: first run `p1-*` (port 19351) proved the same substance (60 video + 174 audio packets, 332008 bytes) but its rc bookkeeping was polluted because the publisher ffmpeg lacked `-nostdin` and consumed the script stream; repeated cleanly as `p1clean-*` with `-nostdin`. Raw first-run logs retained (`p1-*.log`).

## Probe ② — path enforcement negative matrix: **"path is a routing label only"**

Listener always declared `rtmp://10.30.15.10:<port>/live/probe`; publisher URL varied:

| Case | Publisher path | Bind | publisher_rc | listener_rc | out bytes | out md5 |
|---|---|---|---|---|---|---|
| v1correct | `/live/probe` | 10.30.15.10:19361 | 0 | 0 | 244776 | `86cd3c820d796d422b5fbc26240704e5` |
| v2wrong | `/live/other` | 10.30.15.10:19362 | 0 | 0 | 244776 | `86cd3c820d796d422b5fbc26240704e5` |
| v3trailing | `/live/probe/` | 10.30.15.10:19363 | 0 | 0 | 244776 | `86cd3c820d796d422b5fbc26240704e5` |
| v4query | `/live/probe?x=1` | 10.30.15.10:19364 | 0 | 0 | 244776 | `86cd3c820d796d422b5fbc26240704e5` |
| v5encoded | `/live/pro%62e` | 10.30.15.10:19365 | 0 | 0 | 244776 | `86cd3c820d796d422b5fbc26240704e5` |

All five variants were **accepted with byte-identical A/V output** (identical md5 across the matrix). The BMD FFmpeg `-rtmp_listen 1` does not match the incoming connection's app/play path against the listener URL. Verdict per design D4: **"path is a routing label only"** — admissible under the frozen product ruling; path must never be described as publisher authentication or a security boundary.

## Binary outputs (hashes only; not committed to repo)

| File | Bytes | md5 |
|---|---|---|
| p1-out.ts | 332008 | `4842c549045844dcb965e8cef147458e` |
| p1clean-out.ts | 244776 | `86cd3c820d796d422b5fbc26240704e5` |
| v1correct-out.ts … v5encoded-out.ts | 244776 each | `86cd3c820d796d422b5fbc26240704e5` (all identical) |

MPEG-TS binaries were kept out of the repo (tooling safety scan blocks binary `*.ts` writes); their md5/byte counts above are the integrity record. Remote temp dir deleted after fetch.

## Probe ③ — `source_id` already in `SourceIntent::Rtmp` wire: **PASS**

At exact commit `bf0a8e0`, `services/media-agent/src/graph_intent.rs` defines `Rtmp { source_id: NetworkSourceId, endpoint: NetworkEndpoint }` — authorization uses the existing outer Source identity; no new wire field required.

## RTMP option observations (deferred-risk input)

`rtmp-options.txt` shows, among RTMP AVOptions: `-rtmp_listen`, `-listen`, and RTMP-side `-timeout` — "Maximum timeout (in seconds) to wait for incoming connections. -1 is infinite. Implies -rtmp_listen 1". So a publisher-wait cap is available. No post-accept handshake/idle-connection timeout option was observed for RTMP — the deferred ingress risk (half-connection holding the unique listener after accept) stands, per STATE §9.

## Verdict

**TG-0 PASS** — probe ① PASS; probe ② determined "path is a routing label only" (admissible per D4 default product ruling, wording downgrade mandatory); probe ③ PASS. Implementation packet Step 1 (TG-1) may start.
