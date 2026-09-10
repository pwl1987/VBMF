#!/usr/bin/env bash
# C2-O1 Allocator/Arena Mapping-Closure observer —— 2.5h 定点诊断配套
# (非 Step15 rung·无 verdict·O1 只证 mapping 不证 arena)。
# 基座: b1-observer.sh 契约 A/B/C 原样继承(生命周期状态机 / domain T0 冻结 /
#   拓扑快照完整性 bytes+行数+md5 不截断)。C2-O1 扩展(v2.1 冻结口径):
#   双时间轴   RUN_T0_ABS = btime + starttime/clk_tck (服务启动绝对时刻;
#              四元组 btime/pid_starttime_ticks/clk_tck/run_t0_epoch_s 落
#              identity+meta 供任何人重算); RUN_REL_S 为主相对轴; absolute
#              epoch 仅跨文件对齐; wall clock 不作事件主时钟(DST/NTP 披露)。
#   触发器     单一共享 last_trigger_baseline; pending = RssAnon_now - baseline;
#              |pending| >= TRIG_KB 触发(累计制: 多个 <1MB 小步也能累计触发);
#              触发后 baseline=now; 正负共用一个 baseline; 上限 MAX_TRIG——
#              超限后不再捕获但逐条记录 trigger-log(file=-)并置
#              trigger_limit_reached=true / observer_status=DEGRADED(不退出)。
#   捕获链     PRE(最近先行全量快照) -> TRIGGER(trig-n) -> POST-OBSERVATION
#              (+POST_DELAY_S; 语义=post-observation 非稳定态——双子步相隔可
#              仅 10s, POST 可能落在下一事件前)。
#   快照原因   capture_reason ∈ startup/shutdown/fallback/base_periodic/
#              window_periodic/trigger_positive/trigger_negative/post_observation。
#   判定阶梯   CONFIRMED-DIRECT / CONFIRMED-PARTIAL / REFUTED(归因主导制: 事件
#              的主导 RSS 增长由新映射/映射替换/VmSize 增长/heap 形态解释) /
#              INCONCLUSIVE——全部由离线分析器执行, 本脚本只取证。smaps 为逐
#              映射顺序观测非进程级原子快照, 小额 accounting 差异归 PARTIAL
#              不作机制证据(固定披露)。
# 硬红线(继承并细化允许清单): live 数据源仅 /proc/<PID>/{status,smaps,
#   smaps_rollup,statm,stat,cmdline,exe} + task/<tid>/comm + 两份已落盘文件
#   (<EV>/pid 发现 · <EV>/stop-response.json 结束触发)。零网络·零业务端点·
#   零应用内部状态·零 allocator 工具(malloc_info/gdb/pmap/perf/eBPF/
#   LD_PRELOAD/GStreamer debug/allocator tracing/jemalloc/glibc tunables 均禁)。
#   本脚本自哈希与 /proc/<PID>/exe 一次性 md5 仅作 identity 元数据。
# 观察者扰动声明: read-only procfs 但非数学零开销; 其影响按诊断扰动对待,
#   不作因果证据; 本 run 未观测到目标事件 = INCONCLUSIVE, 不等于事件消失。
set -u

TAG="${TAG:-c2o1-2p5h}"
INTERVAL="${INTERVAL:-30}"              # 主序列(秒): status+smaps_rollup+statm
LITE_INTERVAL="${LITE_INTERVAL:-10}"    # 轻序列(秒): status 4 字段
FAST_INTERVAL="${FAST_INTERVAL:-2}"     # 快序列(秒): watch 窗内 status 9 字段
WATCH_START_S="${WATCH_START_S:-6000}"  # 窗口起(RUN_REL_S 主轴)
WATCH_END_S="${WATCH_END_S:-8700}"      # 窗口止(RUN_REL_S)
GUARD_S="${GUARD_S:-120}"               # fast 序列守卫余量
BASE_TOPO_INTERVAL="${BASE_TOPO_INTERVAL:-300}"  # 全程基线全量 smaps 周期
WIN_TOPO_INTERVAL="${WIN_TOPO_INTERVAL:-60}"     # 窗内密集全量 smaps 周期
TRIG_KB="${TRIG_KB:-1024}"              # 触发阈值(kB·累计制)
MAX_TRIG="${MAX_TRIG:-20}"              # 触发捕获上限
POST_DELAY_S="${POST_DELAY_S:-15}"      # POST-OBSERVATION 延迟
NOMINAL_S="${NOMINAL_S:-9200}"          # 名义观察时长(秒)——覆盖率分母
WAIT_PID_S="${WAIT_PID_S:-600}"
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
    if [ "$t" -gt "$bt" ]; then bt="$t"; best="$d"; fi
  done
  printf '%s' "$best"
}

# ── 发现目标 pid(只读依赖: <EV>/pid 文件) ──
PID=""; EV_DIR=""
t_wait0=$(ts_now)
while [ $(( $(ts_now) - t_wait0 )) -lt "$WAIT_PID_S" ]; do
  ev=$(newest_ev)
  if [ -n "$ev" ] && [ -r "$ev/pid" ]; then
    c=$(tr -dc '0-9' < "$ev/pid" 2>/dev/null)
    if [ -n "$c" ] && alive "$c" ]; then
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

# ── 双时间轴: RUN_T0_ABS = btime + starttime/clk_tck ──
CLK_TCK=$(getconf CLK_TCK 2>/dev/null || echo 100)
PAGE_SIZE=$(getconf PAGESIZE 2>/dev/null || echo 4096)
BTIME=$(awk '/^btime/{print $2}' "$PROC_ROOT/stat" 2>/dev/null || true)
STAT_RAW=$(cat "$PROC_ROOT/$PID/stat" 2>/dev/null || true)
START_TICKS=""
if [ -n "$STAT_RAW" ]; then
  stat_rest="${STAT_RAW#*)}"   # 去 "pid (comm) " 后字段 3 起 → starttime(22) 为第 20 token
  START_TICKS=$(awk '{print $20}' <<< "$stat_rest" 2>/dev/null || true)
fi
OBS_T0=$(ts_now)
RUN_T0_SRC="proc_stat"
if [ -n "$BTIME" ] && [ -n "$START_TICKS" ] && [ -n "$CLK_TCK" ] && [ "$CLK_TCK" -gt 0 ]; then
  RUN_T0_ABS=$(awk -v b="$BTIME" -v t="$START_TICKS" -v hz="$CLK_TCK" 'BEGIN{printf "%d", b + t/hz}')
else
  RUN_T0_ABS=$OBS_T0; RUN_T0_SRC="obs_fallback(时钟四元组不完整——如实披露)"
fi
BIN_MD5_PROC=$(md5sum "$PROC_ROOT/$PID/exe" 2>/dev/null | cut -d' ' -f1); [ -n "$BIN_MD5_PROC" ] || BIN_MD5_PROC="-"
OBS_SELF_MD5=$(md5sum "$0" 2>/dev/null | cut -d' ' -f1); [ -n "$OBS_SELF_MD5" ] || OBS_SELF_MD5="-"

# ── T0: process-domain 一次性枚举并冻结(契约 B) ──
T0_CHILDREN=""
for c in $(list_children "$PID" | sort -u); do
  alive "$c" && T0_CHILDREN="$T0_CHILDREN $c"
done
DOMAIN_DESC="$PID$T0_CHILDREN"
DOMAIN_DRIFT=false; NEW_TRACK=""; DRIFTED=""

KERNEL=$(uname -r)
HOSTNAME_STR=$(hostname 2>/dev/null || echo "-")
printf '{"host":"%s","kernel":"%s","page_size":%s,"clk_tck":%s,"btime":%s,"pid":%s,"pid_starttime_ticks":%s,"run_t0_epoch_s":%s,"run_t0_source":"%s","obs_t0_epoch_s":%s,"tag":"%s","binary_md5_proc_exe":"%s","observer_self_md5":"%s","note":"RUN_T0_ABS=btime+starttime/clk_tck; RUN_REL_S is the primary relative axis; absolute epoch is for cross-file alignment only; wall clock is not the event master clock (DST/NTP disclosed). runner-side identity (script hash, cycles) derived offline from header; live sources remain procfs plus two evidence files"}\n' \
  "$HOSTNAME_STR" "$KERNEL" "$PAGE_SIZE" "$CLK_TCK" "${BTIME:-null}" "$PID" "${START_TICKS:-null}" "$RUN_T0_ABS" "$RUN_T0_SRC" "$OBS_T0" "$TAG" "$BIN_MD5_PROC" "$OBS_SELF_MD5" > "$OUT/identity.json"

# ── 首采: 字段可用性 + topology start(startup) ──
FIELDS_AVAIL=""
for k in Rss Pss Pss_Anon Pss_File Pss_Shmem Private_Clean Private_Dirty Shared_Clean Shared_Dirty Anonymous AnonHugePages Swap SwapPss; do
  grep -q "^$k:" "$PROC_ROOT/$PID/smaps_rollup" 2>/dev/null && FIELDS_AVAIL="$FIELDS_AVAIL$k,"
done

# ── 全量快照(契约 C + capture_reason) ──
take_smaps() { # $1=label $2=reason → topology-$1.smaps + 索引 + 捕获点 status
  local label="$1"
  local reason="$2"
  local out="$OUT/topology-$label.smaps"
  local bytes="-" lines="-" sum="-" ok=ERROR now orr rr st
  now=$(ts_now); orr=$((now-OBS_T0)); rr=$((now-RUN_T0_ABS))
  if cat "$PROC_ROOT/$PID/smaps" > "$out" 2>/dev/null && [ -s "$out" ]; then
    bytes=$(wc -c < "$out"); lines=$(wc -l < "$out"); sum=$(md5sum "$out" 2>/dev/null | cut -d' ' -f1); ok=OK
  else
    rm -f "$out"
  fi
  st=$(status_fields "$PROC_ROOT/$PID/status")
  printf '%s|%s|%s|%s|%s|%s|%s|%s|%s\n' "$now" "$orr" "$rr" "$label" "$reason" "$ok" "$bytes" "$lines" "$sum" >> "$OUT/topology-index.txt"
  printf '%s|%s|%s|%s|%s|%s\n' "$now" "$orr" "$rr" "$label" "$reason" "$st" >> "$OUT/captures-status.tsv"
  [ "$ok" = OK ]
}
threads_snapshot() { # $1=label → threads-$1.txt(tid+comm 静态清单·O2 素材)
  local label="$1"
  local out="$OUT/threads-$label.txt"
  local t d
  printf '#ts=%s obs_rel=%s run_rel=%s\n' "$(ts_now)" "$(( $(ts_now)-OBS_T0 ))" "$(( $(ts_now)-RUN_T0_ABS ))" > "$out"
  for d in "$PROC_ROOT/$PID/task"/*; do
    [ -d "$d" ] || continue
    t=$(cat "$d/comm" 2>/dev/null) || t="-"
    printf '%s\t%s\n' "${d##*/}" "$t" >> "$out"
  done
  return 0
}

# ── 表头 ──
M="$OUT/main.csv"; L="$OUT/lite.csv"; F="$OUT/fastlite.csv"; CCSV="$OUT/children.csv"; CEV="$OUT/children-events.log"
TIDX="$OUT/topology-index.txt"; CSTAT="$OUT/captures-status.tsv"; TRIG="$OUT/trigger-log.txt"
printf 'ts,pid,VmSize_kb,VmRSS_kb,VmData_kb,VmPeak_kb,VmSwap_kb,RssAnon_kb,RssFile_kb,RssShmem_kb,Threads,Rss_kb,Pss_kb,Pss_Anon_kb,Pss_File_kb,Pss_Shmem_kb,Private_Clean_kb,Private_Dirty_kb,Shared_Clean_kb,Shared_Dirty_kb,Anonymous_kb,AnonHugePages_kb,Swap_kb,SwapPss_kb,statm_size_pg,statm_resident_pg,statm_shared_pg,statm_text_pg,statm_lib_pg,statm_data_pg,statm_dt_pg\n' > "$M"
printf 'ts,pid,VmRSS_kb,RssAnon_kb,RssFile_kb,Threads\n' > "$L"
printf 'ts,run_rel_s,pid,VmSize_kb,VmRSS_kb,VmData_kb,VmPeak_kb,VmSwap_kb,RssAnon_kb,RssFile_kb,RssShmem_kb,Threads\n' > "$F"
printf 'ts,pid,VmSize_kb,VmRSS_kb,VmData_kb,VmPeak_kb,VmSwap_kb,RssAnon_kb,RssFile_kb,RssShmem_kb,Threads\n' > "$CCSV"
: > "$CEV"
printf '#ts|obs_rel_s|run_rel_s|label|reason|ok|bytes|lines|md5\n' > "$TIDX"
printf '#ts|obs_rel_s|run_rel_s|label|reason|VmSize_kb,VmRSS_kb,VmData_kb,VmPeak_kb,VmSwap_kb,RssAnon_kb,RssFile_kb,RssShmem_kb,Threads\n' > "$CSTAT"
printf '#ts|obs_rel_s|run_rel_s|kind|pending_kb|baseline_kb|now_kb|smaps_file\n' > "$TRIG"

# 表头就绪后方可取 startup 快照(顺序约束: 先建索引/状态表头再首采)
take_smaps start startup && note "topology start OK(startup)" || note "topology start ERROR(startup·其余继续)"

# ── 计数器与触发器状态 ──
MAIN_N=0; LITE_N=0; FAST_N=0; CHILD_N=0; INTERNAL_ERRORS=0; ROLLUP_FAIL=0; FIRST_TS=""; LAST_TS=""
LAST_RSSANON=""; TRIG_BASELINE=""; TRIG_COUNT=0; TRIG_POS=0; TRIG_NEG=0; TRIG_LIMIT=false; POST_QUEUE=""
TOPO_END=SKIPPED; THREADS_START_DONE=false; WIN_NOTED=false

maybe_trigger() { # 累计制: |RssAnon_now − baseline| ≥ TRIG_KB; 正负共用 baseline
  [ -n "$LAST_RSSANON" ] || return 0
  if [ -z "$TRIG_BASELINE" ]; then TRIG_BASELINE=$LAST_RSSANON; return 0; fi
  local pending=$(( LAST_RSSANON - TRIG_BASELINE ))
  local abs=$pending neg=0
  [ $pending -lt 0 ] && { neg=1; abs=$((-pending)); }
  [ "$abs" -ge "$TRIG_KB" ] || return 0
  local now orr rr kind file lbl n
  now=$(ts_now); orr=$((now-OBS_T0)); rr=$((now-RUN_T0_ABS))
  if [ "$neg" -eq 1 ]; then kind=negative; else kind=positive; fi
  file="-"
  if [ "$TRIG_COUNT" -lt "$MAX_TRIG" ]; then
    TRIG_COUNT=$((TRIG_COUNT+1)); n=$TRIG_COUNT; lbl="trig-$n"
    if take_smaps "$lbl" "trigger_$kind"; then file="topology-$lbl.smaps"; else file="ERROR"; fi
    [ "$kind" = positive ] && TRIG_POS=$((TRIG_POS+1)) || TRIG_NEG=$((TRIG_NEG+1))
    POST_QUEUE="$POST_QUEUE $n:$((now+POST_DELAY_S))"
    note "trigger #$n kind=$kind pending=${pending}kB → $file (PRE=最近先行全量快照, POST=+${POST_DELAY_S}s)"
  else
    TRIG_LIMIT=true
    note "trigger cap ($MAX_TRIG) reached: kind=$kind pending=${pending}kB 仅记日志不捕获(观测能力截断·如实入账)"
  fi
  printf '%s|%s|%s|%s|%s|%s|%s|%s\n' "$now" "$orr" "$rr" "$kind" "$pending" "$TRIG_BASELINE" "$LAST_RSSANON" "$file" >> "$TRIG"
  TRIG_BASELINE=$LAST_RSSANON
}

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
  LAST_RSSANON=$(printf '%s' "$st" | cut -d, -f6)
  maybe_trigger
  # T0 域内 child 采样(仅记录, 不并入主序列)
  for c in $T0_CHILDREN; do
    alive "$c" || continue
    cst=$(status_fields "$PROC_ROOT/$c/status")
    [ -z "${cst%%,*}" ] && continue
    printf '%s,%s,%s\n' "$now" "$c" "$cst" >> "$CCSV"
    CHILD_N=$((CHILD_N+1))
  done
  # 运行期新 child: 只记事件, >120s 记 drift, 不入域(契约 B)
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
  LAST_RSSANON=$(printf '%s' "$lit" | cut -d, -f2)
  maybe_trigger
}
sample_fast() {
  local now rr st
  now=$(ts_now); rr=$((now-RUN_T0_ABS))
  st=$(status_fields "$PROC_ROOT/$PID/status")
  [ -z "${st%%,*}" ] && return 0
  printf '%s,%s,%s,%s\n' "$now" "$rr" "$PID" "$st" >> "$F"
  FAST_N=$((FAST_N+1))
  LAST_RSSANON=$(printf '%s' "$st" | cut -d, -f6)
  maybe_trigger
}

# ── 主循环(1s tick) ──
last_main=0; last_lite=0; last_fast=0; last_base=$(ts_now); last_win=$(ts_now); TERMINAL=""
note "T0(obs)=$OBS_T0 run_t0=$RUN_T0_ABS(src=$RUN_T0_SRC) domain=[$DOMAIN_DESC] main=${INTERVAL}s lite=${LITE_INTERVAL}s fast=${FAST_INTERVAL}s watch=[${WATCH_START_S},${WATCH_END_S}](run_rel)+${GUARD_S}s guard base=${BASE_TOPO_INTERVAL}s win=${WIN_TOPO_INTERVAL}s trig=${TRIG_KB}kB×max${MAX_TRIG} nominal=${NOMINAL_S}s"
while :; do
  now=$(ts_now)
  if ! alive "$PID"; then TERMINAL=gone; break; fi
  case "$(cmd_of "$PID")" in *"$EXPECT_CMD"*) ;; *) TERMINAL=cmd_mismatch; break ;; esac
  rr=$((now-RUN_T0_ABS))
  if [ $((now-last_main)) -ge "$INTERVAL" ]; then sample_main; last_main=$now; fi
  if [ $((now-last_lite)) -ge "$LITE_INTERVAL" ]; then sample_lite; last_lite=$now; fi
  if [ $rr -ge $((WATCH_START_S-GUARD_S)) ] && [ $rr -le $((WATCH_END_S+GUARD_S)) ] && [ $((now-last_fast)) -ge "$FAST_INTERVAL" ]; then
    sample_fast; last_fast=$now
  fi
  if [ "$WIN_NOTED" = false ] && [ "$rr" -ge "$WATCH_START_S" ]; then
    WIN_NOTED=true; note "watch window entered (run_rel=${rr}s)——fast×${FAST_INTERVAL}s + 密集拓扑×${WIN_TOPO_INTERVAL}s"
  fi
  if [ $((now-last_base)) -ge "$BASE_TOPO_INTERVAL" ]; then
    take_smaps "base-$rr" base_periodic || note "topology base ERROR(run_rel=${rr}s·其余继续)"
    last_base=$now
  fi
  if [ "$rr" -ge "$WATCH_START_S" ] && [ "$rr" -le "$WATCH_END_S" ] && [ $((now-last_win)) -ge "$WIN_TOPO_INTERVAL" ]; then
    take_smaps "win-$rr" window_periodic || note "topology win ERROR(run_rel=${rr}s·其余继续)"
    last_win=$now
  fi
  if [ "$THREADS_START_DONE" = false ] && [ $((now-OBS_T0)) -ge 120 ]; then
    threads_snapshot start; THREADS_START_DONE=true; note "threads start snapshot taken"
  fi
  if [ -n "$POST_QUEUE" ]; then
    newq=""; ent=""; n=""; due=""
    for ent in $POST_QUEUE; do
      n=${ent%%:*}; due=${ent##*:}
      if [ "$now" -ge "$due" ]; then
        take_smaps "post-$n" post_observation || note "topology post-$n ERROR(其余继续)"
      else
        newq="$newq $ent"
      fi
    done
    POST_QUEUE="$newq"
  fi
  if [ -e "$EV_DIR/stop-response.json" ] && [ "$TOPO_END" = SKIPPED ]; then
    take_smaps end shutdown && note "topology end OK(shutdown 触发)" || note "topology end ERROR(shutdown 触发)"
    TOPO_END=DONE
    threads_snapshot end
    alive "$PID" && sample_main
  fi
  if [ $((now-OBS_T0)) -gt $((NOMINAL_S+1800)) ]; then TERMINAL=timeout_cap; break; fi
  sleep 1
done
# 进程消失后回退补采一次 end(大概率失败——如实记录)
if [ "$TOPO_END" = SKIPPED ]; then
  take_smaps end fallback && note "topology end OK(fallback)" || note "topology end ERROR(fallback)"
  threads_snapshot end
fi

# ── 覆盖率/空洞/状态判定(契约 A + DEGRADED 扩展) ──
END_TS=$(ts_now)
SPAN=0; [ -n "$FIRST_TS" ] && [ -n "$LAST_TS" ] && SPAN=$((LAST_TS-FIRST_TS))
COVERAGE=0; [ "$NOMINAL_S" -gt 0 ] && COVERAGE=$((SPAN*100/NOMINAL_S))
GAP_MAX=$(awk -F, 'BEGIN{m=0} NR>1{d=$1-p; if(d>m)m=d} {p=$1}' "$M" 2>/dev/null); GAP_MAX=${GAP_MAX:-0}
RATIO=0; [ "$MAIN_N" -gt 0 ] && RATIO=$((ROLLUP_FAIL*100/MAIN_N))
REASON_COUNTS=$(awk -F'|' 'NR>1 && $6=="OK"{c[$5]++} END{for(k in c) printf "%s=%d ",k,c[k]}' "$TIDX" 2>/dev/null)
STATUS=""; CODE=0
if [ "$INTERNAL_ERRORS" -gt 0 ] || [ "$RATIO" -gt 10 ]; then
  STATUS=PARTIAL; CODE=4
elif [ "$TRIG_LIMIT" = true ]; then
  STATUS=DEGRADED; CODE=4
elif [ "$COVERAGE" -ge 95 ] && [ "$GAP_MAX" -le 90 ]; then
  STATUS=COMPLETE; CODE=0
else
  STATUS=TARGET_GONE; CODE=3
fi
printf '{"observer_status":"%s","exit_code":%d,"terminal_reason":"%s","tag":"%s","pid":"%s","domain":"%s","domain_frozen_at":%d,"domain_drift":%s,"main_samples":%d,"lite_samples":%d,"fastlite_samples":%d,"child_samples":%d,"rollup_fail":%d,"internal_errors":%d,"span_s":%d,"nominal_s":%d,"coverage_pct":%d,"gap_max_s":%d,"first_ts":%s,"last_ts":%s,"run_t0_epoch_s":%s,"run_t0_source":"%s","obs_t0_epoch_s":%s,"btime":%s,"pid_starttime_ticks":%s,"clk_tck":%s,"page_size":%s,"watch_start_run_rel_s":%d,"watch_end_run_rel_s":%d,"guard_s":%d,"trigger_kb":%d,"max_trig":%d,"trig_positive":%d,"trig_negative":%d,"trigger_limit_reached":%s,"capture_ok_by_reason":"%s","binary_md5_proc_exe":"%s","clock_axis_note":"RUN_REL_S (event_epoch - RUN_T0_ABS) is the primary relative axis; RUN_T0_ABS = btime + starttime/clk_tck recomputable from btime/pid_starttime_ticks/clk_tck; absolute epoch for cross-file alignment only; wall clock is not the event master clock (DST/NTP disclosed)","perturbation_note":"observer is read-only procfs but not mathematically zero-overhead; its impact is treated as diagnostic perturbation, not causal evidence; absence of the target event in this run is recorded as INCONCLUSIVE, not as event disappearance","event_contract":"E01=first positive quantum event (primary); E02..En=subsequent >=1MB positive/negative events (secondary validation); verdict ladder CONFIRMED-DIRECT/CONFIRMED-PARTIAL/REFUTED(attribution-dominance)/INCONCLUSIVE is decided by the offline analyzer; smaps is a sequential procfs observation, not an atomic process-wide memory snapshot - small accounting differences are classified as PARTIAL rather than treated as mechanism evidence","topology":"start/end 见索引","rollup_fields_available":"%s","kernel":"%s","interval_s":%d,"lite_interval_s":%d,"fast_interval_s":%d,"proc_root":"%s","red_line":"live sources: /proc/<pid>/{status,smaps,smaps_rollup,statm,stat,cmdline,exe} + task/<tid>/comm + two evidence files (pid discovery, stop trigger); no network, no business endpoints, no app internals, no allocator tooling (malloc_info/gdb/pmap/perf/eBPF/LD_PRELOAD/GStreamer-debug/jemalloc/glibc-tunables all banned); O1 proves mapping only, never arena","workload_note":"C2-O1 diagnostic run - fixed-point diagnostic window covering the first known event plus confirmation interval - NOT a Step15 rung - no verdict - periodicity proof out of scope"}\n' \
  "$STATUS" "$CODE" "${TERMINAL:-unknown}" "$TAG" "$PID" "$DOMAIN_DESC" "$OBS_T0" "$DOMAIN_DRIFT" \
  "$MAIN_N" "$LITE_N" "$FAST_N" "$CHILD_N" "$ROLLUP_FAIL" "$INTERNAL_ERRORS" "$SPAN" "$NOMINAL_S" "$COVERAGE" "$GAP_MAX" \
  "${FIRST_TS:-0}" "${LAST_TS:-0}" "$RUN_T0_ABS" "$RUN_T0_SRC" "$OBS_T0" "${BTIME:-null}" "${START_TICKS:-null}" "$CLK_TCK" "$PAGE_SIZE" \
  "$WATCH_START_S" "$WATCH_END_S" "$GUARD_S" "$TRIG_KB" "$MAX_TRIG" "$TRIG_POS" "$TRIG_NEG" "$TRIG_LIMIT" "${REASON_COUNTS:-}" \
  "$BIN_MD5_PROC" "$FIELDS_AVAIL" "$KERNEL" "$INTERVAL" "$LITE_INTERVAL" "$FAST_INTERVAL" "$PROC_ROOT" > "$OUT/meta.json"
md5sum "$M" "$L" "$F" "$CCSV" "$CEV" "$TIDX" "$CSTAT" "$TRIG" "$OUT/identity.json" "$OUT/meta.json" "$OUT/observer-console.log" \
  "$OUT"/topology-*.smaps "$OUT"/threads-*.txt > "$OUT/md5s.txt" 2>/dev/null || true
note "observer 生命周期结束 status=$STATUS code=$CODE terminal=$TERMINAL coverage=${COVERAGE}% gap_max=${GAP_MAX}s main=$MAIN_N lite=$LITE_N fast=$FAST_N trig=($TRIG_POS+/$TRIG_NEG-)limit=$TRIG_LIMIT"
exit "$CODE"
