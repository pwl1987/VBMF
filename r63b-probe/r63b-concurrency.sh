#!/usr/bin/env bash
# R63-B real-machine concurrency validation (B5 + slow-reader isolation + B6 normal chain).
# Preconditions: hw bin rebuilt via build-bmd (md5 pinned in header), manifest v5 present.
set -u
cd "$HOME/media-agent-build/services/media-agent"
EV="$HOME/a2-8-02i-evidence/2026-09-06-r63b-concurrency"
rm -rf "$EV"; mkdir -p "$EV"
BIN_MD5=$(md5sum target/debug/media-agent | awk '{print $1}')
MAN_MD5=$(md5sum "$HOME/a2-8-02i-v5.manifest.json" | awk '{print $1}')
{
  echo "=== R63-B concurrency probe $(date -Is)"
  echo "bin_md5=$BIN_MD5"
  echo "manifest_md5=$MAN_MD5"
} > "$EV/header.txt"
echo "bin_md5=$BIN_MD5 manifest_md5=$MAN_MD5"
LAT="$EV/latencies.txt"; : > "$LAT"

export MEDIA_AGENT_DEVICE_BINDING="$HOME/a2-8-02i-v5.manifest.json"
export MEDIA_AGENT_MODE=diagnostic
export VBMF_DIAG_INPUTS=2
export MEDIA_AGENT_HEALTH_BIND=127.0.0.1:8080
export RUST_LOG=info
nohup ./target/debug/media-agent > "$HOME/r63b-svc.log" 2>&1 &
echo $! > "$HOME/r63b-svc.pid"
for i in $(seq 1 60); do
  curl -sf http://127.0.0.1:8080/health >/dev/null 2>&1 && break
  sleep 0.25
done
curl -s http://127.0.0.1:8080/health > "$EV/health.json"
curl -s http://127.0.0.1:8080/api/v1/runtime > "$EV/runtime-before.json"
SID_HEX=$(jq -r '.sessions[0].id' "$EV/runtime-before.json" | sed 's/^session-//')
SID=$(python3 -c "h='$SID_HEX'.replace('-','');print(f'{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}')")
A=$(jq -r '.program_switch.observed_active' "$EV/runtime-before.json")
B=$(jq -r ".sessions[0].inputs[] | select(.id != \"$A\") | .id" "$EV/runtime-before.json")
echo "SID=$SID A=$A B=$B" | tee "$EV/session.txt"

t_probe() { # t_probe <label> <path>
  curl -s -o /dev/null -w "$1 %{http_code} %{time_total}s\n" "http://127.0.0.1:8080$2" >> "$LAT"
}
sw() { # sw <cid> <target> -> response body to stdout
  curl -s -X POST http://127.0.0.1:8080/api/v1/commands \
    -H 'content-type: application/json' \
    -d "{\"command_id\":\"$1\",\"kind\":\"switch_program\",\"target\":{\"target_type\":\"switch_program\",\"session_id\":\"$SID\",\"target_device\":\"$2\"},\"requested_by\":\"r63b\"}"
}
rb() {
  curl -s http://127.0.0.1:8080/api/v1/runtime \
    | jq -c '{observed: .program_switch.observed_active, seg: .program_switch.timeline.segment_id, disc: .program_switch.timeline.discontinuity_state, vc: .program_switch.timeline.video_continuity}'
}
CID() { python3 -c 'import uuid;print(uuid.uuid4())'; }

echo "== B5-1: bg switch A->B + concurrent probes"
SW_START=$(date +%s.%N)
sw "$(CID)" "$B" > "$EV/switch1.json" &
SWPID=$!
sleep 0.02
t_probe "b5_1_health_during" /health
t_probe "b5_1_runtime_during" /api/v1/runtime
t_probe "b5_1_events_during" /api/v1/events/projection
t_probe "b5_1_health_during2" /health
wait $SWPID
SW1=$(cat "$EV/switch1.json"); echo "switch1: $SW1"
sleep 0.4
echo "readback1: $(rb)" | tee -a "$EV/readbacks.txt"

echo "== B5-2: bg switch B->A + concurrent probes"
sw "$(CID)" "$A" > "$EV/switch2.json" &
SWPID=$!
sleep 0.02
t_probe "b5_2_health_during" /health
t_probe "b5_2_runtime_during" /api/v1/runtime
t_probe "b5_2_events_during" /api/v1/events/projection
wait $SWPID
echo "switch2: $(cat "$EV/switch2.json")"
sleep 0.4
echo "readback2: $(rb)" | tee -a "$EV/readbacks.txt"

echo "== B5-3: slow reader isolation (partial request, hold ~8s)"
exec 3<>/dev/tcp/127.0.0.1/8080
printf 'GET /health HTTP/1.1\r\nHost: t\r\n' >&3
sleep 1
t_probe "b5_3_health_during_slow_reader_t1s" /health
sleep 3
t_probe "b5_3_health_during_slow_reader_t4s" /health
sleep 4
t_probe "b5_3_health_during_slow_reader_t8s" /health
exec 3<&- 2>/dev/null
exec 3>&- 2>/dev/null
sleep 0.2

echo "== B5-4: same-id replay (second POST after first completes)"
RC=$(CID)
R1=$(sw "$RC" "$B"); echo "replay-first: $R1" >> "$EV/replay.json"
R2=$(sw "$RC" "$B"); echo "replay-second: $R2" >> "$EV/replay.json"
echo "$R1 / $R2" | grep -o '"status":{"status":"[a-z]*"}' | sort | uniq -c >> "$EV/replay-summary.txt"
python3 - "$R1" "$R2" <<'PY' >> "$EV/replay-summary.txt"
import json, sys
a, b = json.loads(sys.argv[1]), json.loads(sys.argv[2])
print("detail_identical:", a.get("detail") == b.get("detail"))
print("classification_identical:", a.get("classification") == b.get("classification"))
PY
sleep 0.3
echo "readback3: $(rb)" | tee -a "$EV/readbacks.txt"

echo "== B5-5: N=8 concurrent /runtime"
PIDS=""
for i in $(seq 1 8); do
  t_probe "b5_5_runtime_concurrent_$i" /api/v1/runtime &
  PIDS="$PIDS $!"
done
for p in $PIDS; do wait "$p"; done

echo "== B6: normal chain A(active)->B->query->A->query covered above; teardown"
CIDS=$(CID)
curl -s -X POST http://127.0.0.1:8080/api/v1/commands -H 'content-type: application/json' \
  -d "{\"command_id\":\"$CIDS\",\"kind\":\"stop_session\",\"target\":{\"target_type\":\"session_by_id\",\"session_id\":\"$SID\"},\"requested_by\":\"r63b\"}" > "$EV/stop-response.json"
sleep 2
TEARDOWN_OK=$(grep -c "Program Execution Runtime teardown 完成" "$HOME/r63b-svc.log" || true)
WATCHDOG_OK=$(grep -c "停止旗置位, 观测线程退出" "$HOME/r63b-svc.log" || true)
echo "teardown_line=$TEARDOWN_OK watchdog_exit_line=$WATCHDOG_OK (service resident by design)" | tee -a "$EV/header.txt"
kill "$(cat "$HOME/r63b-svc.pid")" 2>/dev/null
sleep 1
pgrep -f '^\./target/debug/media-agent' >/dev/null && pkill -f '^\./target/debug/media-agent'
echo "process-stopped-after-evidence (explicit)" | tee -a "$EV/header.txt"

cp "$HOME/r63b-svc.log" "$EV/svc.log"
echo "=== latencies ==="
cat "$LAT"
md5sum "$EV"/* > "$EV/md5s.txt"
cat "$EV/md5s.txt"
