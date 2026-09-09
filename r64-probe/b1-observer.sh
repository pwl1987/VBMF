#!/usr/bin/env bash
# B1-DIAG 外部只读内存构成 observer —— 4h 诊断配套（非 Step15 rung·不产生 verdict·只产 classification 输入）。
# 裁决契约（三项机械调整）:
#   A 生命周期状态机: exit 0=COMPLETE / 3=TARGET_GONE(覆盖不达标) / 4=PARTIAL(降级运行) /
#     10=OBSERVER_ERROR(结构性失败)。meta.json 的 observer_status 是登记唯一依据, shell 返回码仅辅助。
#   B process-domain T0 冻结: 首采点一次性枚举子树; 运行期新 child 只记 children-events.log,
#     存活>120s 记 domain_drift=true, 一律不并入观察总量。
#   C topology 三次全量快照(start/mid/end): 完整读取流式落盘, 记 bytes+行数+md5, 禁止截断;
#     单点失败= topology_status ERROR, 其余数据继续采, 不因此整体失败。
# 硬红线: live 数据源仅 /proc/<PID>/*（经 PROC_ROOT 间接, 默认 /proc）+ 两份已落盘文件
#   （发现用 <EV>/pid; 结束触发用 <EV>/stop-response.json）。本脚本不做任何网络访问、
#   不触碰业务端点、不读 GStreamer/应用内部状态; cycle/epoch/frames/watchdog 关联全部离线
#   （samples.csv + svc.log 事后离线 join, 本脚本不做）。
set -u

TAG="${TAG:-b1-diag-4h}"
INTERVAL="${INTERVAL:-30}"            # 主序列间隔(秒): status + smaps_rollup + statm 全套
LITE_INTERVAL="${LITE_INTERVAL:-10}"  # 轻序列间隔(秒): 仅 status 四字段
MID_S="${MID_S:-7200}"                # topology mid 触发时刻(秒, 自 T0)
NOMINAL_S="${NOMINAL_S:-14500}"       # 名义观察时长(秒)——覆盖率分母
WAIT_PID_S="${WAIT_PID_S:-600}"       # 等待 pid 文件上限(秒)
EXPECT_CMD="${EXPECT_CMD:-media-agent}"
PROC_ROOT="${PROC_ROOT:-/proc}"
EV_ROOT="${EV_ROOT:-$HOME/a2-8-02i-evidence}"
OUT="${OUT_DIR:-$EV_ROOT/$(date +%Y-%m-%d)-${TAG}-observer}"

mkdir -p "$OUT" 2>/dev/null || { echo "OBSERVER_ERROR: mkdir $OUT 失败" >&2; exit 10; }
note() { printf '%s %s\n' "$(date -Is)" "$*" >> "$OUT/observer-console.log"; }
ts_now() { date +%s; }
alive() { [ -d "$PROC_ROOT/$1" ]; }
cmd_of() { tr '\0' ' ' < "$PROC_ROOT/$1/cmdline" 2>/dev/null || true; }

# ── 字段提取(缺失字段输出空位, 逗号对齐不塌缩) ──
status_fields() { # 9: VmSize,VmRSS,VmData,VmPeak,VmSwap,RssAnon,RssFile,RssShmem,Threads
  awk '
    /^VmSize:/{a=$2} /^VmRSS:/{b=$2} /^VmData:/{c=$2} /^VmPeak:/{d=$2} /^VmSwap:/{e=$2}
    /^RssAnon:/{f=$2} /^RssFile:/{g=$2} /^RssShmem:/{h=$2} /^Threads:/{i=$2}
    END{printf "%s,%s,%s,%s,%s,%s,%s,%s,%s",a,b,c,d,e,f,g,h,i}
  ' "$1" 2>/dev/null || printf ',,,,,,,,'
}
lite_fields() { # 4: VmRSS,RssAnon,RssFile,Threads
  awk '
    /^VmRSS:/{b=$2} /^RssAnon:/{f=$2} /^RssFile:/{g=$2} /^Threads:/{i=$2}
    END{printf "%s,%s,%s,%s",b,f,g,i}
  ' "$1" 2>/dev/null || printf ',,,,'
}
rollup_fields() { # 13: Rss,Pss,Pss_Anon,Pss_File,Pss_Shmem,Private_Clean,Private_Dirty,Shared_Clean,Shared_Dirty,Anonymous,AnonHugePages,Swap,SwapPss
  awk '
    /^Rss:/{a=$2} /^Pss:/{b=$2} /^Pss_Anon:/{c=$2} /^Pss_File:/{d=$2} /^Pss_Shmem:/{e=$2}
    /^Private_Clean:/{f=$2} /^Private_Dirty:/{g=$2} /^Shared_Clean:/{h=$2} /^Shared_Dirty:/{i=$2}
    /^Anonymous:/{j=$2} /^AnonHugePages:/{k=$2} /^Swap:/{l=$2} /^SwapPss:/{m=$2}
    END{printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s",a,b,c,d,e,f,g,h,i,j,k,l,m}
  ' "$1" 2>/dev/null || printf ',,,,,,,,,,,,'
}
list_children() { # 首选 task/<pid>/children, 缺失则 ppid 扫描兜底
  local p="$1" f c d pp
  f="$PROC_ROOT/$p/task/$p/children"
  if [ -r "$f" ]; then cat "$f"; return 0; fi
  for d in "$PROC_ROOT"/[0-9]*; do
    [ -d "$d" ] || continue
    pp=$(awk '{print $4}' "$d/stat" 2>/dev/null) || continue
    [ "$pp" = "$p" ] && printf '%s\n' "${d##*/}"
  done
  return 0
}
newest_ev() { # 最新匹配 *-$TAG 的证据目录(mtime)
  local best="" bt=0 d t
  for d in "$EV_ROOT"/*-"$TAG"; do
    [ -d "$d" ] || continue
    t=$(stat -c %Y "$d" 2>/dev/null) || continue
    if [ "$t" -gt "$bt" ]; then bt=$t; best=$d; fi
  done
  printf '%s' "$best"
}

# ── 拓扑快照(调整 C): 全量读、流式写、记 bytes/行数/md5、不截断、失败如实记 ERROR ──
TOPO_START=SKIPPED; TOPO_MID=SKIPPED; TOPO_END=SKIPPED
take_topology() {
  local label="$1"
  local out="$OUT/topology-$label.smaps"
  local bytes="-" lines="-" sum="-" ok=ERROR
  if cat "$PROC_ROOT/$PID/smaps" > "$out" 2>/dev/null && [ -s "$out" ]; then
    bytes=$(wc -c < "$out"); lines=$(wc -l < "$out"); sum=$(md5sum "$out" 2>/dev/null | cut -d' ' -f1); ok=OK
  else
    rm -f "$out"
  fi
  printf '%s|%s|%s|%s|%s|%s\n' "$(ts_now)" "$label" "$ok" "$bytes" "$lines" "$sum" >> "$OUT/topology-index.txt"
  case "$label" in
    start) TOPO_START=$ok ;; mid) TOPO_MID=$ok ;; end) TOPO_END=$ok ;;
  esac
  [ "$ok" = OK ]
}

# ── 发现目标 pid(只读依赖: <EV>/pid 文件) ──
PID=""; EV_DIR=""
t_wait0=$(ts_now)
while [ $(( $(ts_now) - t_wait0 )) -lt "$WAIT_PID_S" ]; do
  ev=$(newest_ev)
  if [ -n "$ev" ] && [ -r "$ev/pid" ]; then
    c=$(tr -dc '0-9' < "$ev/pid" 2>/dev/null)
    if [ -n "$c" ] && alive "$c"; then
      case "$(cmd_of "$c")" in *"$EXPECT_CMD"*) PID=$c; EV_DIR=$ev; break ;; esac
    fi
  fi
  sleep 2
done
if [ -z "$PID" ]; then
  note "OBSERVER_ERROR: $WAIT_PID_S 秒内未发现目标 pid ($EXPECT_CMD)"
  printf '{"observer_status":"OBSERVER_ERROR","exit_code":10,"reason":"pid_not_found"}\n' > "$OUT/meta.json"
  exit 10
fi
note "target pid=$PID ev=$EV_DIR"

# ── T0: process-domain 一次性枚举并冻结(调整 B) ──
T0=$(ts_now)
T0_CHILDREN=""
for c in $(list_children "$PID" | sort -u); do
  alive "$c" && T0_CHILDREN="$T0_CHILDREN $c"
done
DOMAIN_DESC="$PID$T0_CHILDREN"
DOMAIN_DRIFT=false; NEW_TRACK=""; DRIFTED=""

# ── 首采: 字段可用性 + topology start ──
KERNEL=$(uname -r)
FIELDS_AVAIL=""
for k in Rss Pss Pss_Anon Pss_File Pss_Shmem Private_Clean Private_Dirty Shared_Clean Shared_Dirty Anonymous AnonHugePages Swap SwapPss; do
  grep -q "^$k:" "$PROC_ROOT/$PID/smaps_rollup" 2>/dev/null && FIELDS_AVAIL="$FIELDS_AVAIL$k,"
done
take_topology start && note "topology start OK" || note "topology start ERROR(其余继续)"

# ── 表头 ──
M="$OUT/main.csv"; L="$OUT/lite.csv"; CCSV="$OUT/children.csv"; CEV="$OUT/children-events.log"
printf 'ts,pid,VmSize_kb,VmRSS_kb,VmData_kb,VmPeak_kb,VmSwap_kb,RssAnon_kb,RssFile_kb,RssShmem_kb,Threads,Rss_kb,Pss_kb,Pss_Anon_kb,Pss_File_kb,Pss_Shmem_kb,Private_Clean_kb,Private_Dirty_kb,Shared_Clean_kb,Shared_Dirty_kb,Anonymous_kb,AnonHugePages_kb,Swap_kb,SwapPss_kb,statm_size_pg,statm_resident_pg,statm_shared_pg,statm_text_pg,statm_lib_pg,statm_data_pg,statm_dt_pg\n' > "$M"
printf 'ts,pid,VmRSS_kb,RssAnon_kb,RssFile_kb,Threads\n' > "$L"
printf 'ts,pid,VmSize_kb,VmRSS_kb,VmData_kb,VmPeak_kb,VmSwap_kb,RssAnon_kb,RssFile_kb,RssShmem_kb,Threads\n' > "$CCSV"
: > "$CEV"

# ── 计数器 ──
MAIN_N=0; LITE_N=0; CHILD_N=0; INTERNAL_ERRORS=0; ROLLUP_FAIL=0; FIRST_TS=""; LAST_TS=""

sample_main() {
  local now st ro m1 m2 m3 m4 m5 m6 m7 row
  now=$(ts_now)
  st=$(status_fields "$PROC_ROOT/$PID/status")
  if [ -z "${st%%,*}" ]; then INTERNAL_ERRORS=$((INTERNAL_ERRORS+1)); return 0; fi
  ro=$(rollup_fields "$PROC_ROOT/$PID/smaps_rollup")
  if [ -z "${ro%%,*}" ]; then ROLLUP_FAIL=$((ROLLUP_FAIL+1)); fi
  m1=""; m2=""; m3=""; m4=""; m5=""; m6=""; m7=""
  read -r m1 m2 m3 m4 m5 m6 m7 < "$PROC_ROOT/$PID/statm" 2>/dev/null || true
  row="$now,$PID,$st,$ro,$m1,$m2,$m3,$m4,$m5,$m6,$m7"
  printf '%s\n' "$row" >> "$M"
  MAIN_N=$((MAIN_N+1))
  [ -z "$FIRST_TS" ] && FIRST_TS=$now
  LAST_TS=$now
  # T0 域内 child 采样(仅记录, 不并入主序列)
  for c in $T0_CHILDREN; do
    alive "$c" || continue
    cst=$(status_fields "$PROC_ROOT/$c/status")
    [ -z "${cst%%,*}" ] && continue
    printf '%s,%s,%s\n' "$now" "$c" "$cst" >> "$CCSV"
    CHILD_N=$((CHILD_N+1))
  done
  # 运行期新 child: 只记事件, >120s 记 drift, 不入域(调整 B)
  local cur ent cn ct0
  cur=$(list_children "$PID" | sort -u)
  for cn in $cur; do
    case " $T0_CHILDREN " in *" $cn "*) continue ;; esac
    if ! printf '%s' "$NEW_TRACK" | grep -q " $cn="; then
      NEW_TRACK="$NEW_TRACK $cn=$now"
      printf '%s|%s|first_seen|%s\n' "$now" "$cn" "$(cmd_of "$cn" | cut -c1-80)" >> "$CEV"
    fi
  done
  for ent in $NEW_TRACK; do
    cn=${ent%%=*}; ct0=${ent##*=}
    if [ "$cn" != "$ent" ] && [ $((now-ct0)) -gt 120 ] && ! printf '%s' "$DRIFTED" | grep -qw "$cn"; then
      DRIFTED="$DRIFTED $cn"; DOMAIN_DRIFT=true
      printf '%s|%s|drift_alive_gt_120s\n' "$now" "$cn" >> "$CEV"
    fi
  done
}
sample_lite() {
  local now lit
  now=$(ts_now)
  lit=$(lite_fields "$PROC_ROOT/$PID/status")
  [ -z "${lit%%,*}" ] && return 0
  printf '%s,%s,%s\n' "$now" "$PID" "$lit" >> "$L"
  LITE_N=$((LITE_N+1))
}

# ── 主循环(1s tick; 主/轻按各自间隔触发; stop-response 落盘即抢采 end 拓扑) ──
last_main=0; last_lite=0; TERMINAL=""
note "T0=$T0 domain=[$DOMAIN_DESC] interval=${INTERVAL}s lite=${LITE_INTERVAL}s nominal=${NOMINAL_S}s"
while :; do
  now=$(ts_now)
  if ! alive "$PID"; then TERMINAL=gone; break; fi
  case "$(cmd_of "$PID")" in *"$EXPECT_CMD"*) ;; *) TERMINAL=cmd_mismatch; break ;; esac
  if [ $((now-last_main)) -ge "$INTERVAL" ]; then sample_main; last_main=$now; fi
  if [ $((now-last_lite)) -ge "$LITE_INTERVAL" ]; then sample_lite; last_lite=$now; fi
  if [ "$TOPO_MID" = SKIPPED ] && [ $((now-T0)) -ge "$MID_S" ]; then
    take_topology mid && note "topology mid OK" || note "topology mid ERROR(其余继续)"
  fi
  if [ -e "$EV_DIR/stop-response.json" ] && [ "$TOPO_END" = SKIPPED ]; then
    take_topology end && note "topology end OK(stop 触发)" || note "topology end ERROR(stop 触发)"
    alive "$PID" && sample_main
  fi
  if [ $((now-T0)) -gt $((NOMINAL_S+1800)) ]; then TERMINAL=timeout_cap; break; fi
  sleep 1
done
# 进程消失后回退补采一次 end(大概率失败——如实记录)
if [ "$TOPO_END" = SKIPPED ]; then
  take_topology end && note "topology end OK(回退)" || note "topology end ERROR(回退)"
fi

# ── 覆盖率/空洞/状态判定(调整 A) ──
END_TS=$(ts_now)
SPAN=0; [ -n "$FIRST_TS" ] && [ -n "$LAST_TS" ] && SPAN=$((LAST_TS-FIRST_TS))
COVERAGE=0; [ "$NOMINAL_S" -gt 0 ] && COVERAGE=$((SPAN*100/NOMINAL_S))
GAP_MAX=$(awk -F, 'BEGIN{m=0} NR>1{d=$1-p; if(d>m)m=d} {p=$1}' "$M" 2>/dev/null); GAP_MAX=${GAP_MAX:-0}
STATUS=""; CODE=0
RATIO=0; [ "$MAIN_N" -gt 0 ] && RATIO=$((ROLLUP_FAIL*100/MAIN_N))
if [ "$INTERNAL_ERRORS" -gt 0 ] || [ "$RATIO" -gt 10 ]; then
  STATUS=PARTIAL; CODE=4
elif [ "$COVERAGE" -ge 95 ] && [ "$GAP_MAX" -le 90 ]; then
  STATUS=COMPLETE; CODE=0
else
  STATUS=TARGET_GONE; CODE=3
fi
printf '{"observer_status":"%s","exit_code":%d,"terminal_reason":"%s","tag":"%s","pid":"%s","domain":"%s","domain_frozen_at":%d,"domain_drift":%s,"main_samples":%d,"lite_samples":%d,"child_samples":%d,"rollup_fail":%d,"internal_errors":%d,"span_s":%d,"nominal_s":%d,"coverage_pct":%d,"gap_max_s":%d,"first_ts":%s,"last_ts":%s,"topology":"start=%s,mid=%s,end=%s","rollup_fields_available":"%s","kernel":"%s","interval_s":%d,"lite_interval_s":%d,"proc_root":"%s","red_line":"live sources: procfs only plus two evidence files (pid discovery, stop trigger); no network, no business endpoints, no app internals","workload_note":"diagnostic run - not a Step15 rung - no verdict produced"}\n' \
  "$STATUS" "$CODE" "${TERMINAL:-unknown}" "$TAG" "$PID" "$DOMAIN_DESC" "$T0" "$DOMAIN_DRIFT" \
  "$MAIN_N" "$LITE_N" "$CHILD_N" "$ROLLUP_FAIL" "$INTERNAL_ERRORS" "$SPAN" "$NOMINAL_S" "$COVERAGE" "$GAP_MAX" \
  "${FIRST_TS:-0}" "${LAST_TS:-0}" "$TOPO_START" "$TOPO_MID" "$TOPO_END" "$FIELDS_AVAIL" "$KERNEL" \
  "$INTERVAL" "$LITE_INTERVAL" "$PROC_ROOT" > "$OUT/meta.json"
md5sum "$M" "$L" "$CCSV" "$CEV" "$OUT/topology-index.txt" "$OUT/meta.json" "$OUT/observer-console.log" \
  "$OUT"/topology-*.smaps > "$OUT/md5s.txt" 2>/dev/null || true
note "observer 生命周期结束 status=$STATUS code=$CODE terminal=$TERMINAL coverage=${COVERAGE}% gap_max=${GAP_MAX}s main=$MAIN_N lite=$LITE_N"
exit "$CODE"
