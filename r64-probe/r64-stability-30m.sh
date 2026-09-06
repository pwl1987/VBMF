#!/usr/bin/env bash
# R64-6': 30min 稳定基线重跑（真服务 media-agent·无注入·真 BMD·R65 后 bin）
# 谓词 v2（R65 裁决·2026-09-07 随 R65-A0 计划批准冻结——单独登记的验收谓词
# 修订, 非静默改脚本; v1 五项保持: fd/RSS/frames/drops/critical）:
#   线程有界振荡 max−min≤4（per-connection 模型本性——v1"恒定"字面废弃）/
#   switch_epoch 逐命令恰 +1（60 成功命令 → 终值恰 60, 且全部 executed+preserved）
#   / observed 逐命令 == target（v1 序列首尾比较废弃）/
#   watchdog tick 递增（RUST_LOG=info 使 tick/teardown 行可测——v1 warn 级
#   测不到=测量缺口）/
#   fd 末段≤首段+8 / RSS 末1/3≤首1/3+50MB 且无单调爬升 / frames 严格递增 /
#   dropped_bus_events=0 / events has_critical=0
# 纪律: bin 重建+md5 先行; PIDS 显式收集; ^锚定清理; /events 破坏性 drain 稀疏采样。
set -u

cd "$HOME/media-agent-build/services/media-agent"
EV="$HOME/a2-8-02i-evidence/$(date +%Y-%m-%d)-r64-stability-30m-v2"
rm -rf "$EV"; mkdir -p "$EV"

# ── 0. 身份钉扎（bin 重建先行——R62 纪律; R65 后代码态）──
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
  echo "predicate=v2 (R65 adjudication 2026-09-07: threads bounded<=4 / per-command epoch+1 / per-command observed==target / RUST_LOG=info watchdog)"
} > "$EV/header.txt"

# ── 1. 起服务（常驻·取证后显式停止）──
export MEDIA_AGENT_DEVICE_BINDING=$HOME/a2-8-02i-v5.manifest.json
export MEDIA_AGENT_MODE=diagnostic
export VBMF_DIAG_INPUTS=2
export MEDIA_AGENT_HEALTH_BIND=127.0.0.1:8080
export RUST_LOG=info
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
  "requested_by": "r64-stability-v2"}).encode()
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
echo "cycle,ts,rss_kb,fd,threads,dropped,clock_lost,sw_epoch,tl_epoch,frames_v,frames_a,observed,target" > "$SAMP"
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
  echo "$c,$(date +%s),$RSS,$FD,$THR,$DROP,$CLK,$SW_E,$TL_E,$FV,$FA,$OBS,$TGT" >> "$SAMP"
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
  "requested_by": "r64-stability-v2"}).encode()
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

# ── 6. 谓词 v2 分析（R65 裁决修订版——PASS/FAIL 行 + 总裁决）──
python3 - "$EV" "$TICK_MID" "$TICK_END" <<'PYEOF' > "$EV/summary.txt"
import csv, json, re, sys
ev, tick_mid, tick_end = sys.argv[1], int(sys.argv[2] or 0), int(sys.argv[3] or 0)
rows = list(csv.DictReader(open(f"{ev}/samples.csv")))
verdicts = []
def pred(name, ok, note=""):
    verdicts.append((name, ok, note))
    print(f"{'PASS' if ok else 'FAIL'} {name} {note}")
# [v2] 线程有界振荡 max−min≤4（per-connection 模型本性——"恒定"字面废弃）
ths = [int(r["threads"]) for r in rows]
pred("threads_bounded_oscillation", max(ths) - min(ths) <= 4,
     f"min={min(ths)} max={max(ths)} spread={max(ths)-min(ths)}")
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
# [v2] switch_epoch 逐命令恰 +1（首样本=1·末样本=60——60 成功命令）
sws = [int(r["sw_epoch"]) for r in rows]
per_cmd = all(sws[i+1] == sws[i] + 1 for i in range(len(sws)-1))
pred("switch_epoch_per_command_plus1", per_cmd and sws[0] == 1 and sws[-1] == 60,
     f"first={sws[0]} last={sws[-1]} consecutive_plus1={per_cmd}")
# [v2] 全部切换命令 executed + outcome=preserved（不伪装成功）
# 响应词表: status.status=executed/replayed; detail/classification 为顶层字段。
ok_cmds = 0
for line in open(f"{ev}/switch-responses.jsonl"):
    line = line.strip()
    if not line:
        continue
    j = json.loads(line)
    if (j.get("status") or {}).get("status") == "executed" and "outcome=preserved" in (j.get("detail") or ""):
        ok_cmds += 1
pred("switches_all_executed_preserved", ok_cmds == 60, f"ok={ok_cmds}/60")
# [v2] observed 逐命令 == target（回读行级核对——序列首尾比较废弃）
rb_ok, rb_total = 0, 0
pat = re.compile(r"cycle=(\d+) target=(\S+) observed=(\S+)")
for line in open(f"{ev}/readbacks.txt"):
    m = pat.search(line)
    if m:
        rb_total += 1
        if m.group(3) == m.group(2):
            rb_ok += 1
pred("observed_tracks_per_command", rb_total == 60 and rb_ok == 60,
     f"match={rb_ok}/{rb_total}")
# frames 严格递增
fv = [int(r["frames_v"]) for r in rows]
fa = [int(r["frames_a"]) for r in rows]
pred("frames_advancing", all(fv[i+1] > fv[i] for i in range(len(fv)-1)) and all(fa[i+1] > fa[i] for i in range(len(fa)-1)),
     f"v:{fv[0]}->{fv[-1]} a:{fa[0]}->{fa[-1]}")
# dropped/clock_lost 恒 0
pred("drops_zero", all(int(r["dropped"]) == 0 for r in rows) and all(int(r["clock_lost"]) == 0 for r in rows))
# [v2] watchdog tick 递增（RUST_LOG=info——v1 warn 级测量缺口消除）
pred("watchdog_ticks_advancing", tick_end > tick_mid > 0, f"mid={tick_mid} end={tick_end}")
# events has_critical=0
try:
    ev_lines = [l for l in open(f"{ev}/events.txt") if "critical" in l]
    crit_ok = all("critical=0" in l or "critical=False" in l for l in ev_lines)
    pred("events_no_critical", crit_ok, f"samples={len(ev_lines)}")
except FileNotFoundError:
    pred("events_no_critical", False, "events.txt missing")
# replay 数据行（不 gate——数量与原样性如实在案）
rp_lines = [l for l in open(f"{ev}/replays.jsonl") if l.strip()]
rp_replayed = sum(1 for l in rp_lines if '"replayed"' in l)
print(f"NOTE replays={len(rp_lines)} replayed_status={rp_replayed} (data, not gating)")
allp = all(ok for _, ok, _ in verdicts)
print(f"VERDICT {'PASS' if allp else 'FAIL'} ({sum(1 for _,ok,_ in verdicts if ok)}/{len(verdicts)} predicates v2)")
PYEOF
cat "$EV/summary.txt"
md5sum "$EV"/* > "$EV/md5s.txt"
if grep -q "^VERDICT PASS" "$EV/summary.txt"; then exit 0; else exit 2; fi
