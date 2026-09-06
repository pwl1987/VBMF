#!/usr/bin/env bash
# R62 Control Plane Safety Probe — 真机 leg
#   16-0A: 正常切换全程 POST 耗时（编排 ⓪-⑩ 排空确认+证据+settle 实测）
#   16-0B: Transport 并发（accept 串行性/切换在途查询阻塞/反向并发/停滞读者）
# 前提: media-agent 已在 127.0.0.1:8080 运行（diagnostic 双输入·R61 bin 90186bb9）。
# 只读探针: 不改任何代码; 只产证据（tee 落证据盒 run.log）。
set -u
BASE=http://127.0.0.1:8080
OUT="$HOME/a2-8-02i-evidence/2026-09-06-a2-8-05-r62-cp-safety-probe"
mkdir -p "$OUT"
exec > >(tee -a "$OUT/run.log") 2>&1

now_ms() { date +%s.%3N; }

echo "===== R62 probe start $(date -Is) ====="
md5sum "$HOME/media-agent-build/services/media-agent/target/debug/media-agent" \
  "$HOME/a2-8-02i-v5.manifest.json"

curl -sf "$BASE/api/v1/runtime" > "$OUT/runtime-before.json"
SID=$(jq -r '.program_switch.session_id' "$OUT/runtime-before.json")
A=$(jq -r '.program_switch.observed_active' "$OUT/runtime-before.json")
B=$(jq -r ".sessions[0].inputs | map(select(.id != \"$A\")) | .[0].id" "$OUT/runtime-before.json")
[ "$A" != "null" ] && [ "$B" != "null" ] || { echo "FATAL: program_switch/inputs 缺席"; exit 1; }
echo "PROBE|SID=$SID|A(active)=$A|B(target)=$B"

# 计时探针: TIMING|label|start=<epoch_s>|code_total=<http_code> <time_total_s>
t() {
  local label="$1"; shift
  local s; s=$(now_ms)
  local r; r=$(curl -s -o /tmp/r62-t-body -w '%{http_code} %{time_total}' --max-time 120 "$@")
  echo "TIMING|$label|start=$s|code_total=$r"
}

# sw <cmd_id> <target_device> <outfile-stem>: POST switch; code+time 落 <stem>, body 落 <stem>.body
sw() {
  curl -s --max-time 120 -o "$3.body" -w '%{http_code} %{time_total}' \
    -H 'Content-Type: application/json' -X POST "$BASE/api/v1/commands" \
    -d "{\"command_id\":\"$1\",\"kind\":\"switch_program\",\"target\":{\"target_type\":\"switch_program\",\"session_id\":\"$SID\",\"target_device\":\"$2\"},\"requested_by\":\"r62-probe\"}" > "$3"
  echo "SWITCH|$1|target=$2|code_time=$(cat "$3")|body=$(cat "$3.body")"
}

readback() {
  curl -sf "$BASE/api/v1/runtime" | jq -c '{observed: .program_switch.observed_active, epoch: .program_switch.switch_epoch, seg: .program_switch.timeline.segment_id, video_continuity: .program_switch.timeline.video_continuity, discontinuity: .program_switch.timeline.discontinuity_state}'
}

echo "--- [1] 串行基线 ---"
for i in 1 2 3; do t "serial-health-$i"   "$BASE/health"; done
for i in 1 2 3; do t "serial-runtime-$i"  "$BASE/api/v1/runtime"; done
t "serial-events-1" "$BASE/api/v1/events/projection"

echo "--- [2] 16-0A 真机: 正常切换全程耗时 ---"
CID1="r62-serial-a2b-$(date +%s%N)"
sw "$CID1" "$B" /tmp/r62-sw1
readback
CID2="r62-serial-b2a-$(date +%s%N)"
sw "$CID2" "$A" /tmp/r62-sw2
readback
# 自适应在途窗口: 取串行切换耗时的一半（0.05s~3s 夹紧）
SW1_T=$(awk '{print $2}' /tmp/r62-sw1)
DELAY=$(awk -v t="$SW1_T" 'BEGIN{d=t/2; if(d<0.05)d=0.05; if(d>3)d=3; printf "%.2f", d}')
echo "PROBE|serial_switch_time=${SW1_T}s|during_window_delay=${DELAY}s"

echo "--- [3] 16-0B: 切换在途并发查询 ---"
CID3="r62-conc-a2b-$(date +%s%N)"
( sw "$CID3" "$B" /tmp/r62-sw3 > /tmp/r62-sw3.log 2>&1 ) &
SWPID=$!
sleep "$DELAY"
t "during-switch-health-1"   "$BASE/health"
t "during-switch-health-2"   "$BASE/health"
t "during-switch-runtime-1"  "$BASE/api/v1/runtime"
t "during-switch-events-1"   "$BASE/api/v1/events/projection"
# 同 command_id replay 在途并发（幂等面在 accept 串行下排队）
s=$(now_ms)
r=$(curl -s -o /tmp/r62-replay-body -w '%{http_code} %{time_total}' --max-time 120 \
  -H 'Content-Type: application/json' -X POST "$BASE/api/v1/commands" \
  -d "{\"command_id\":\"$CID3\",\"kind\":\"switch_program\",\"target\":{\"target_type\":\"switch_program\",\"session_id\":\"$SID\",\"target_device\":\"$B\"},\"requested_by\":\"r62-probe\"}")
echo "TIMING|during-switch-replay|start=$s|code_total=$r|body=$(cat /tmp/r62-replay-body)"
wait $SWPID
cat /tmp/r62-sw3.log
readback

echo "--- [4] 16-0B: 双并发反向切换 ---"
CID4="r62-opp-a2b-$(date +%s%N)"
CID5="r62-opp-b2a-$(date +%s%N)"
( sw "$CID4" "$B" /tmp/r62-sw4 > /tmp/r62-sw4.log 2>&1 ) &
P4=$!
( sw "$CID5" "$A" /tmp/r62-sw5 > /tmp/r62-sw5.log 2>&1 ) &
P5=$!
wait $P4 $P5
cat /tmp/r62-sw4.log /tmp/r62-sw5.log
readback

echo "--- [5] 16-0B: 停滞读者（未完成请求占住唯一 accept） ---"
exec 3<>/dev/tcp/127.0.0.1/8080
printf 'GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\n' >&3   # 无空行 = 永不完整
sleep 0.3
t "stalled-during-health-1" "$BASE/health"
t "stalled-during-health-2" "$BASE/health"
sleep 12
exec 3<&- 3>&-
t "post-stall-health" "$BASE/health"

echo "--- [6] 收尾快照 ---"
curl -sf "$BASE/api/v1/runtime" > "$OUT/runtime-after.json"
readback
echo "===== R62 probe done $(date -Is) ====="
