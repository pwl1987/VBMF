#!/usr/bin/env bash
# R63-A real-machine normal-path regression smoke (single-shot, evidence-producing).
# Preconditions: hw bin rebuilt via build-bmd (md5 pinned in header), manifest v5 present.
# Service model: diagnostic agent stays resident until killed (session teardown != process exit).
set -u
cd "$HOME/media-agent-build/services/media-agent"
EV="$HOME/a2-8-02i-evidence/2026-09-06-r63a-recovery"
rm -rf "$EV"; mkdir -p "$EV"
BIN_MD5=$(md5sum target/debug/media-agent | awk '{print $1}')
MAN_MD5=$(md5sum "$HOME/a2-8-02i-v5.manifest.json | awk '{print $1}'" 2>/dev/null || md5sum "$HOME/a2-8-02i-v5.manifest.json" | awk '{print $1}')
{
  echo "=== R63-A recovery smoke $(date -Is)"
  echo "bin_md5=$BIN_MD5"
  echo "manifest_md5=$MAN_MD5"
} > "$EV/header.txt"
echo "bin_md5=$BIN_MD5 manifest_md5=$MAN_MD5"

export MEDIA_AGENT_DEVICE_BINDING="$HOME/a2-8-02i-v5.manifest.json"
export MEDIA_AGENT_MODE=diagnostic
export VBMF_DIAG_INPUTS=2
export MEDIA_AGENT_HEALTH_BIND=127.0.0.1:8080
export RUST_LOG=info
nohup ./target/debug/media-agent > "$HOME/r63a-svc.log" 2>&1 &
echo $! > "$HOME/r63a-svc.pid"

for i in $(seq 1 60); do
  curl -sf http://127.0.0.1:8080/health >/dev/null 2>&1 && break
  sleep 0.25
done
curl -s http://127.0.0.1:8080/health > "$EV/health.json"
echo "health: $(cat "$EV/health.json")"

curl -s http://127.0.0.1:8080/api/v1/runtime > "$EV/runtime-before.json"
SID_HEX=$(jq -r '.sessions[0].id' "$EV/runtime-before.json" | sed 's/^session-//')
SID=$(python3 -c "h='$SID_HEX'.replace('-','');print(f'{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}')")
A=$(jq -r '.program_switch.observed_active' "$EV/runtime-before.json")
B=$(jq -r ".sessions[0].inputs[] | select(.id != \"$A\") | .id" "$EV/runtime-before.json")
echo "SID=$SID A=$A B=$B" | tee "$EV/session.txt"

rb() {
  curl -s http://127.0.0.1:8080/api/v1/runtime \
    | jq -c '{observed: .program_switch.observed_active, v_pts_state: .program_switch.program_video_pts_state, timeline: .program_switch.timeline}'
}

sw() {
  local t=$1 cid
  cid=$(python3 -c 'import uuid;print(uuid.uuid4())')
  curl -s -X POST http://127.0.0.1:8080/api/v1/commands \
    -H 'content-type: application/json' \
    -d "{\"command_id\":\"$cid\",\"kind\":\"switch_program\",\"target\":{\"target_type\":\"switch_program\",\"session_id\":\"$SID\",\"target_device\":\"$t\"},\"requested_by\":\"r63a-smoke\"}" \
    | tee -a "$EV/switch-responses.jsonl"
  echo
}

echo "== switch 1: $A -> $B"
sw "$B"
sleep 0.6
echo "readback1: $(rb)" | tee -a "$EV/readbacks.txt"
echo "== switch 2: $B -> $A"
sw "$A"
sleep 0.6
echo "readback2: $(rb)" | tee -a "$EV/readbacks.txt"

curl -s http://127.0.0.1:8080/api/v1/runtime > "$EV/runtime-after.json"

CID=$(python3 -c 'import uuid;print(uuid.uuid4())')
curl -s -X POST http://127.0.0.1:8080/api/v1/commands \
  -H 'content-type: application/json' \
  -d "{\"command_id\":\"$CID\",\"kind\":\"stop_session\",\"target\":{\"target_type\":\"session_by_id\",\"session_id\":\"$SID\"},\"requested_by\":\"r63a-smoke\"}" \
  > "$EV/stop-response.json"
echo "stop: $(cat "$EV/stop-response.json")"
sleep 2
# teardown chain verification (service stays resident by design — kill after verify)
TEARDOWN_OK=$(grep -c "Program Execution Runtime teardown 完成" "$HOME/r63a-svc.log" || true)
WATCHDOG_OK=$(grep -c "停止旗置位, 观测线程退出" "$HOME/r63a-svc.log" || true)
echo "teardown_line=$TEARDOWN_OK watchdog_exit_line=$WATCHDOG_OK (service resident by design)" | tee -a "$EV/header.txt"
if pgrep -f '^\./target/debug/media-agent' >/dev/null; then
  echo "service-resident-after-session-teardown (expected for diagnostic mode)" | tee -a "$EV/header.txt"
fi
kill "$(cat "$HOME/r63a-svc.pid")" 2>/dev/null
sleep 1
if pgrep -f '^\./target/debug/media-agent' >/dev/null; then
  pkill -f '^\./target/debug/media-agent'
  sleep 1
fi
echo "process-stopped-after-evidence (explicit)" | tee -a "$EV/header.txt"

cp "$HOME/r63a-svc.log" "$EV/svc.log"
grep -c "watchdog 活体观测行" "$EV/svc.log" > "$EV/watchdog-tick-count.txt" || true
md5sum "$EV"/* > "$EV/md5s.txt"
echo "=== evidence files ==="
cat "$EV/md5s.txt"
