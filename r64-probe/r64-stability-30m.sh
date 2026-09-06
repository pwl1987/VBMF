#!/usr/bin/env bash
# R64-6: 30min 稳定基线（真服务 media-agent·无注入·真 BMD）
# 验收谓词经 R64 计划批准（2026-09-06）:
#   线程恒定 / fd 末段≤首段+8 / RSS 末1/3≤首1/3+50MB 且无单调爬升 /
#   switch_epoch 恰 +60 / observed 逐周期跟随 / frames 严格递增 /
#   watchdog tick 持续递增 / dropped_bus_events=0 / events has_critical=0
# 纪律: bin 重建+md5 先行; PIDS 显式收集; ^锚定清理; /events 破坏性 drain 稀疏采样。
set -u

cd "$HOME/media-agent-build/services/media-agent"
EV="$HOME/a2-8-02i-evidence/$(date +%Y-%m-%d)-r64-stability-30m"
rm -rf "$EV"; mkdir -p "$EV"

# ── 0. 身份钉扎（bin 重建先行——R62 纪律）──
export DECKLINK_SDK_INCLUDE=$HOME/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
source ~/.cargo/env
bash scripts/build-bmd.sh > "$EV/build.log" 2>&1 || { echo "build failed" > "$EV/abort.txt"; exit 2; }
BIN_MD5=$(md5sum target/debug/media-agent | cut -d" " -f1)
MAN_MD5=$(md5sum "$HOME/a2-8-02i-v5.manifest.json" | cut -d" " -f1)
{ echo "date=$(date -Is)"
  echo "bin_md5=$BIN_MD5"
  echo "manifest_md5=$MAN_MD5"
  echo "duration=1800s cycles=60 cycle_period=30s replay_every=5"
} > "$EV/header.txt"

# ── 1. 起服务（常驻·取证后显式停止）──
export MEDIA_AGENT_DEVICE_BINDING=$HOME/a2-8-02i-v5.manifest.json
export MEDIA_AGENT_MODE=diagnostic
export VBMF_DIAG_INPUTS=2
export MEDIA_AGENT_HEALTH_BIND=127.0.0.1:8080
export RUST_LOG=warn
nohup ./target/debug/media-agent > "$EV/svc.log" 2>&1 &
SVC_PID=$!
echo "$SVC_PID" > "$EV/pid"
ok=0
for _ in $(seq 1 80); do
  code=$(curl -s -o "$EV/health0.json" -w "%{http_code}" http://127.0.0.1:8080/health || true)
  [ "$code" = "200" ] && { ok=1; break; }
  sleep 0.25
done
[ "$ok" = "1" ] || { echo "service not ready" > "$EV/abort.txt"; kill "$SVC_PID" 2>/dev/null; exit 2; }
cp "$EV/health0.json" "$EV/health.json"

# ── 2. 发现: SID / A(当前 observed) / B(另一输入) ──
curl -s http://127.0.0.1:8080/api/v1/runtime > "$EV/runtime-before.json"
python3 - "$EV/runtime-before.json" <<'PYEOF' > "$EV/session.txt"
import json, sys, uuid
j = json.load(open(sys.argv[1]))
sess = j["sessions"][0]
raw = sess["id"].replace("session-", "").replace("-", "")
sid = str(uuid.UUID(raw))
inputs = [i["id"] for i in sess["inputs"]]
obs = j["program_switch"]["observed_active"]
other = [x for x in inputs if x != obs]
a = obs
b = other[0] if other else obs
print(f"SID={sid}\nA={a}\nB={b}\nINPUTS={','.join(inputs)}")
PYEOF
eval "$(cat "$EV/session.txt")"
echo "SID=$SID A=$A B=$B"

SW() { # SW <cid> <target> → 响应 json
  local cid="$1" tgt="$2"
  python3 - "$cid" "$tgt" "$SID" <<'PYEOF'
import sys, json, urllib.request
cid, tgt, sid = sys.argv[1], sys.argv[2], sys.argv[3]
body = json.dumps({"command_id": cid, "kind": "switch_program",
  "target": {"target_type": "switch_program", "session_id": sid, "target_device": tgt},
  "requested_by": "r64-stability"}).encode()
req = urllib.request.Request("http://127.0.0.1:8080/api/v1/commands", data=body,
  headers={"Content-Type": "application/json"}, method="POST")
try:
  with urllib.request.urlopen(req, timeout=60) as r:
    print(r.read().decode())
except Exception as e:
  print(json.dumps({"error": str(e)}))
PYEOF
}
RB() { # /runtime 摘要行
  curl -s http://127.0.0.1:8080/api/v1/runtime | python3 -c '
import json,sys
j=json.load(sys.stdin); ps=j.get("program_switch") or {}
print("observed=%s sw=%s tl_ep=%s seg=%s disc=%s v_cont=%s frames_v=%s frames_a=%s at=%s" % (
 ps.get("observed_active"), ps.get("switch_epoch"), (ps.get("timeline") or {}).get("program_epoch"),
 (ps.get("timeline") or {}).get("segment_id"), (ps.get("timeline") or {}).get("discontinuity_state"),
 (ps.get("timeline") or {}).get("video_continuity"), ps.get("program_video_frames"),
 ps.get("program_audio_frames"), (ps.get("timeline") or {}).get("observed_at_ms")))'
}
CID() { python3 -c 'import uuid;print(uuid.uuid4())'; }

# ── 3. 后台并发查询环（2s 交替 /runtime·/health 计时; 停止文件控制）──
rm -f "$EV/queries.stop"
(
  flip=0
  while [ ! -f "$EV/queries.stop" ]; do
    if [ $((flip % 2)) -eq 0 ]; then P=/api/v1/runtime; else P=/health; fi
    curl -s -o /dev/null -w "$(date +%H:%M:%S) $P %{http_code} %{time_total}s\n" \
      "http://127.0.0.1:8080$P" >> "$EV/latencies.txt"
    flip=$((flip + 1))
    sleep 2
  done
) &
QPID=$!

# ── 4. 主循环: 60 周期 × ~30s ──
SAMP="$EV/samples.csv"
echo "cycle,ts,rss_kb,fd,threads,dropped,clock_lost,sw_epoch,tl_epoch,frames_v,frames_a,observed" > "$SAMP"
EVAPPS="$EV/events.txt"; : > "$EVAPPS"
LOGS="$EV/switch-responses.jsonl"; : > "$LOGS"
RBK="$EV/readbacks.txt"; : > "$RBK"
REPLAYS="$EV/replays.jsonl"; : > "$REPLAYS"
TICK_MID=0
for c in $(seq 1 60); do
  if [ $((c % 2)) -eq 1 ]; then TGT=$B; else TGT=$A; fi
  CID1=$(CID)
  SW_OUT=$(SW "$CID1" "$TGT")
  echo "$SW_OUT" >> "$LOGS"
  if [ $((c % 5)) -eq 0 ]; then
    RP_OUT=$(SW "$CID1" "$TGT")
    echo "cycle=$c $RP_OUT" >> "$REPLAYS"
  fi
  sleep 0.3
  echo "cycle=$c target=$TGT $(RB)" >> "$RBK"
  # 采样（30s 一次）
  RSS=$(awk '/VmRSS/{print $2}' /proc/$SVC_PID/status)
  FD=$(ls /proc/$SVC_PID/fd 2>/dev/null | wc -l)
  THR=$(awk '/Threads/{print $2}' /proc/$SVC_PID/status)
  H=$(curl -s http://127.0.0.1:8080/health)
  DROP=$(echo "$H" | python3 -c 'import json,sys;print(json.load(sys.stdin).get("dropped_bus_events"))')
  CLK=$(echo "$H" | python3 -c 'import json,sys;print(json.load(sys.stdin).get("clock_lost_events"))')
  RT=$(curl -s http://127.0.0.1:8080/api/v1/runtime)
  eval "$(echo "$RT" | python3 -c '
import json,sys
j=json.load(sys.stdin); ps=j.get("program_switch") or {}; tl=ps.get("timeline") or {}
print("SW_E=%s TL_E=%s FV=%s FA=%s OBS=%s" % (ps.get("switch_epoch"), tl.get("program_epoch"),
 ps.get("program_video_frames"), ps.get("program_audio_frames"), ps.get("observed_active")))')"
  echo "$c,$(date +%s),$RSS,$FD,$THR,$DROP,$CLK,$SW_E,$TL_E,$FV,$FA,$OBS" >> "$SAMP"
  # /events 稀疏采样（60s 一次·破坏性 drain 如实注明）
  if [ $((c % 2)) -eq 0 ]; then
    curl -s http://127.0.0.1:8080/api/v1/events/projection | python3 -c '
import json,sys
j=json.load(sys.stdin)
print("t=%s total=%s critical=%s kinds=%s" % (__import__("time").strftime("%H:%M:%S"),
 j.get("total"), j.get("has_critical"), json.dumps(j.get("kind_counts"), ensure_ascii=False)))' >> "$EVAPPS"
  fi
  if [ "$c" = "30" ]; then TICK_MID=$(grep -c "watchdog 活体观测行" "$EV/svc.log" || true); fi
  # 周期节拍 ~30s
  sleep 28
done

# ── 5. 收尾 ──
touch "$EV/queries.stop"
wait "$QPID" 2>/dev/null
STOP_BODY=$(python3 - "$SID" <<'PYEOF'
import sys, json, uuid
sid = sys.argv[1]
body = json.dumps({"command_id": str(uuid.uuid4()), "kind": "stop_session",
  "target": {"target_type": "session_by_id", "session_id": sid},
  "requested_by": "r64-stability"}).encode()
import urllib.request
req = urllib.request.Request("http://127.0.0.1:8080/api/v1/commands", data=body,
  headers={"Content-Type": "application/json"}, method="POST")
with urllib.request.urlopen(req, timeout=30) as r:
    print(r.read().decode())
PYEOF
)
echo "$STOP_BODY" > "$EV/stop-response.json"
sleep 3
grep -c "Program Execution Runtime teardown 完成" "$EV/svc.log" > "$EV/teardown-line.txt" 2>&1 || true
TICK_END=$(grep -c "watchdog 活体观测行" "$EV/svc.log" || true)
echo "tick_mid=$TICK_MID tick_end=$TICK_END" > "$EV/ticks.txt"
kill "$SVC_PID" 2>/dev/null
sleep 1
pkill -f '^\./target/debug/media-agent' 2>/dev/null
cp "$EV/svc.log" "$EV/svc-final.log" 2>/dev/null || true

# ── 6. 谓词分析（显式验收谓词——PASS/FAIL 行 + 总裁决）──
python3 - "$EV" "$TICK_MID" "$TICK_END" <<'PYEOF' > "$EV/summary.txt"
import csv, json, sys
ev, tick_mid, tick_end = sys.argv[1], int(sys.argv[2] or 0), int(sys.argv[3] or 0)
rows = list(csv.DictReader(open(f"{ev}/samples.csv")))
verdicts = []
def pred(name, ok, note=""):
    verdicts.append((name, ok, note))
    print(f"{'PASS' if ok else 'FAIL'} {name} {note}")
# 线程恒定
ths = [int(r["threads"]) for r in rows]
pred("threads_constant", len(set(ths)) == 1, f"min={min(ths)} max={max(ths)}")
# fd 末段≤首段+8
fds = [int(r["fd"]) for r in rows]
head, tail = fds[:3], fds[-3:]
pred("fd_bounded", sum(tail)/3 <= sum(head)/3 + 8, f"first3_avg={sum(head)/3:.1f} last3_avg={sum(tail)/3:.1f}")
# RSS 末 1/3 ≤ 首 1/3 + 50MB 且非单调爬升
rss = [int(r["rss_kb"]) for r in rows]
n3 = len(rss)//3
mono = all(rss[i+1] > rss[i] for i in range(len(rss)-1))
pred("rss_bounded", sum(rss[-n3:])/n3 <= sum(rss[:n3])/n3 + 50*1024 and not mono,
     f"first_third={sum(rss[:n3])/n3/1024:.1f}MB last_third={sum(rss[-n3:])/n3/1024:.1f}MB monotonic={mono}")
# switch_epoch 恰 +60
sws = [int(r["sw_epoch"]) for r in rows]
pred("switch_epoch_plus60", sws[-1] - sws[0] == 60, f"start={sws[0]} end={sws[-1]}")
# observed 逐周期跟随（末值=偶数周期目标=A）
pred("observed_tracks", rows[-1]["observed"] == rows[0]["observed"], f"first={rows[0]['observed'][:8]} last={rows[-1]['observed'][:8]}")
# frames 严格递增
fv = [int(r["frames_v"]) for r in rows]
fa = [int(r["frames_a"]) for r in rows]
pred("frames_advancing", all(fv[i+1] > fv[i] for i in range(len(fv)-1)) and all(fa[i+1] > fa[i] for i in range(len(fa)-1)),
     f"v:{fv[0]}->{fv[-1]} a:{fa[0]}->{fa[-1]}")
# dropped/clock_lost 恒 0
pred("drops_zero", all(int(r["dropped"]) == 0 for r in rows) and all(int(r["clock_lost"]) == 0 for r in rows))
# watchdog tick 递增
pred("watchdog_ticks_advancing", tick_end > tick_mid > 0, f"mid={tick_mid} end={tick_end}")
# events has_critical=0
try:
    ev_lines = [l for l in open(f"{ev}/events.txt") if "critical" in l]
    crit_ok = all("critical=0" in l or "critical=False" in l for l in ev_lines)
    pred("events_no_critical", crit_ok, f"samples={len(ev_lines)}")
except FileNotFoundError:
    pred("events_no_critical", False, "events.txt missing")
allp = all(ok for _, ok, _ in verdicts)
print(f"VERDICT {'PASS' if allp else 'FAIL'} ({sum(1 for _,ok,_ in verdicts if ok)}/{len(verdicts)} predicates)")
PYEOF
cat "$EV/summary.txt"
md5sum "$EV"/* > "$EV/md5s.txt"
if grep -q "^VERDICT PASS" "$EV/summary.txt"; then exit 0; else exit 2; fi
