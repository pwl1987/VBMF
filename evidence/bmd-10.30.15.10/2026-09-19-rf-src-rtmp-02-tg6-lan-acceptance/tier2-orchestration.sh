#!/bin/bash
# RF-SRC-RTMP-02 TG-6 Tier 2 orchestration (runs ON BMD) — evidence copy of
# the exact script executed for the successful gate4 run (see tg6-manifest.md).
set -u
G=/tmp/tg6-tier2-gate4.log
cd /tmp/vbmf-tg6-58fd33d/services/media-agent || exit 9
VBMF_MACHINE_ID=bmd-10-30-15-10 \
VBMF_FFMPEG_RTMP_SOURCE=1 \
VBMF_FFMPEG_RTMP_SOURCE_LAN=1 \
VBMF_FFMPEG_RTMP_SOURCE_EXTERNAL=1 \
VBMF_FFMPEG_RTMP_SOURCE_URL=rtmp://10.30.15.10:19351/live/tier2 \
VBMF_FFMPEG_RTMP_SOURCE_HLS_DIR=/tmp/tg6-tier2-hls \
nohup ./target/debug/media-agent-gates > "$G" 2>&1 &
GATE=$!
echo "GATE:$GATE"
sleep 5
docker run -d --name tier2pub1 mwader/static-ffmpeg:7.1 \
  -hide_banner -loglevel warning -nostdin -re \
  -f lavfi -i testsrc2=size=320x180:rate=25 \
  -re -f lavfi -i sine=frequency=1000:sample_rate=48000 \
  -map 0:v:0 -map 1:a:0 -c:v libx264 -preset ultrafast -tune zerolatency \
  -c:a aac -f flv rtmp://10.30.15.10:19351/live/tier2 >/dev/null
for i in $(seq 1 20); do
  grep -q "external-leg A/V verified" "$G" && break
  sleep 1
done
echo "MARKER:$(grep -c 'external-leg A/V verified' "$G")"
echo "PEER1:$(ss -tn | grep 19351 | head -1)"
docker stop -t 1 tier2pub1 >/dev/null && echo DISCONNECTED
sleep 3
docker run -d --name tier2pub2 mwader/static-ffmpeg:7.1 \
  -hide_banner -loglevel warning -nostdin -re \
  -f lavfi -i testsrc2=size=320x180:rate=25 \
  -re -f lavfi -i sine=frequency=1000:sample_rate=48000 \
  -map 0:v:0 -map 1:a:0 -c:v libx264 -preset ultrafast -tune zerolatency \
  -c:a aac -f flv rtmp://10.30.15.10:19351/live/tier2 >/dev/null
echo PUSH2-STARTED
echo "PEER2:$(ss -tn | grep 19351 | head -1)"
for i in $(seq 1 30); do
  grep -q "RF_SRC_RTMP_01_BMD_SOURCE_RECOVERY_PASS" "$G" && break
  grep -qE "RF-FF-02 FAIL" "$G" && break
  sleep 1
done
docker rm -f tier2pub1 tier2pub2 >/dev/null 2>&1
echo "===RESULT"
grep -E "PASS|FAIL|external-leg" "$G"
echo "===leftovers"
pgrep -c -x ffmpeg || echo no-ffmpeg
ps -p 992634 -o pid,etime --no-headers
