#!/usr/bin/env bash
# C2-O3-2 / E3A source-family differential harness (existing MEDIA_AGENT_SELFTEST path).
# Question (frozen): with DeckLink source/plugin/hardware removed but the same GStreamer
# controller + raw caps + tee + queue + appsink + same process allocator, does the target
# reservation-boundary morphology still appear?
# Diagnostic-only: NOT a Step15 rung, no PASS/FAIL, 24h FAIL unchanged, 45min fixed
# (no extension regardless of result). No trigger counter, no per-TID observer.
# Offline classification reuses the O1/O2 reservation-envelope classifier verbatim on
# adjacent smaps pairs; "trig_positive" is never used.
set -uo pipefail

TAG="${TAG:-c2o3-e3a-selftest-45m}"
WARMUP_S=300
TOTAL_S=2700
STATUS_INTERVAL=30
SMAPS_INTERVAL=300
REQBY="${REQBY:-c2o3-e3a-selftest}"
EV_ROOT="${EV_ROOT:-$HOME/a2-8-02i-evidence}"
ROOT="${ROOT:-$HOME/media-agent-build}"
AGENT_DIR="$ROOT/services/media-agent"
EV="$EV_ROOT/$(date +%Y-%m-%d)-$TAG"

mkdir -p "$EV"
cd "$AGENT_DIR"

export DECKLINK_SDK_INCLUDE="$HOME/decklink-sdk-include"
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
source "$HOME/.cargo/env"
bash scripts/build-bmd.sh > "$EV/build.log" 2>&1 || { echo "build failed" > "$EV/abort.txt"; exit 2; }
BIN_MD5=$(md5sum target/debug/media-agent | awk '{print $1}')
SCRIPT_MD5=$(md5sum "$ROOT/r64-probe/c2o3-e3a-selftest.sh" | awk '{print $1}')

cat > "$EV/header.txt" <<HDR
run_kind=C2-O3-2/E3A source-family differential (selftest)
source_family=videotestsrc/audiotestsrc (DeckLink source/plugin/hardware removed)
step15_verdict=INAPPLICABLE
warmup_s=$WARMUP_S
total_s=$TOTAL_S
smaps_interval_s=$SMAPS_INTERVAL
status_interval_s=$STATUS_INTERVAL
bin_md5=$BIN_MD5
harness_md5=$SCRIPT_MD5
frozen_question=does target reservation-boundary morphology persist without DeckLink source family?
interpretation=SELFTEST_MORPHOLOGY_OBSERVED / NOT_OBSERVED / OBSERVED+WORKLOAD_SENSITIVE / INCONCLUSIVE only
trig_positive_used=false
HDR

# Same process environment family as E2; only MEDIA_AGENT_SELFTEST added (source family swap).
export MEDIA_AGENT_DEVICE_BINDING="$HOME/a2-8-02i-v5.manifest.json"
export MEDIA_AGENT_MODE=diagnostic
export MEDIA_AGENT_SELFTEST=1
export MEDIA_AGENT_HEALTH_BIND=127.0.0.1:8080
export RUST_LOG=info
nohup ./target/debug/media-agent > "$EV/svc.log" 2>&1 &
SVC_PID=$!
echo "$SVC_PID" > "$EV/pid"

cleanup() {
  [ -n "${QPID:-}" ] && kill "$QPID" 2>/dev/null || true
  kill "$SVC_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# fail-closed startup: process alive + selftest pipeline startup log within 120s
ok=0
for _ in $(seq 1 48); do
  [ -d "/proc/$SVC_PID" ] || break
  grep -q "self-test 管线启动" "$EV/svc.log" 2>/dev/null && { ok=1; break; }
  sleep 2.5
done
[ "$ok" = "1" ] || { echo "selftest pipeline not started" > "$EV/abort.txt"; exit 2; }

# process identity (spec item 4)
CLK_TCK=$(getconf CLK_TCK)
BTIME=$(awk '/^btime/{print $2}' /proc/stat)
START_TICKS=$(awk '{print $22}' "/proc/$SVC_PID/stat" 2>/dev/null)
RUN_T0=$((BTIME + START_TICKS / CLK_TCK))
{
  echo "pid=$SVC_PID"
  echo "cmdline=$(tr '\0' ' ' < /proc/$SVC_PID/cmdline)"
  echo "exe_md5=$(md5sum /proc/$SVC_PID/exe | cut -d' ' -f1)"
  echo "uname=$(uname -r)"
  echo "clk_tck=$CLK_TCK pagesize=$(getconf PAGESIZE) btime=$BTIME starttime_ticks=$START_TICKS"
  echo "run_t0_epoch=$RUN_T0"
  ldd --version 2>&1 | head -1
  echo "GNU_LIBC_VERSION=$(getconf GNU_LIBC_VERSION 2>/dev/null || echo '-')"
  echo "env:MEDIA_AGENT_SELFTEST=1 MEDIA_AGENT_MODE=diagnostic VBMF_DIAG_INPUTS unset"
} > "$EV/identity.txt"

status_fields() {
  awk '/^VmSize:/{a=$2} /^VmRSS:/{b=$2} /^VmData:/{c=$2} /^VmPeak:/{d=$2} /^VmSwap:/{e=$2}
       /^RssAnon:/{f=$2} /^RssFile:/{g=$2} /^RssShmem:/{h=$2} /^Threads:/{i=$2}
       END{printf "%s,%s,%s,%s,%s,%s,%s,%s,%s",a,b,c,d,e,f,g,h,i}' "$1"
}
rollup_fields() {
  awk '/^Rss:/{a=$2} /^Pss:/{b=$2} /^Pss_Anon:/{c=$2} /^Pss_File:/{d=$2} /^Pss_Shmem:/{e=$2}
       /^Private_Clean:/{f=$2} /^Private_Dirty:/{g=$2} /^Shared_Clean:/{h=$2} /^Shared_Dirty:/{i=$2}
       /^Anonymous:/{j=$2} /^AnonHugePages:/{k=$2} /^Swap:/{l=$2} /^SwapPss:/{m=$2}
       END{printf "%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s",a,b,c,d,e,f,g,h,i,j,k,l,m}' "$1"
}
take_smaps() { # $1=label
  local rel=$(( $(date +%s) - RUN_T0 ))
  cat "/proc/$SVC_PID/smaps" > "$EV/topology-$1.smaps" 2>/dev/null || return 1
  printf '%s|%s|%s\n' "$(date +%s)" "$rel" "$1" >> "$EV/captures-index.txt"
  printf '%s|%s|%s|%s\n' "$(date +%s)" "$rel" "$1" "$(status_fields /proc/$SVC_PID/status)" >> "$EV/captures-status.tsv"
  curl -s "http://127.0.0.1:8080/health" > "$EV/health-$1.json" 2>/dev/null || true
}

printf '#ts|run_rel_s|label\n' > "$EV/captures-index.txt"
printf '#ts|run_rel_s|label|VmSize,VmRSS,VmData,VmPeak,VmSwap,RssAnon,RssFile,RssShmem,Threads\n' > "$EV/captures-status.tsv"
printf 'ts,run_rel_s,VmSize,VmRSS,VmData,VmPeak,VmSwap,RssAnon,RssFile,RssShmem,Threads,Rss,Pss,Pss_Anon,Pss_File,Pss_Shmem,Private_Clean,Private_Dirty,Shared_Clean,Shared_Dirty,Anonymous,AnonHugePages,Swap,SwapPss\n' > "$EV/status.csv"

take_smaps start

# 2s /health query loop (management-plane background parity with E2)
rm -f "$EV/queries.stop"
( while [ ! -f "$EV/queries.stop" ]; do
    curl -s -o /dev/null -w "$(date +%H:%M:%S) health %{http_code} %{time_total}s\n" \
      "http://127.0.0.1:8080/health" >> "$EV/latencies.txt"
    sleep 2
  done ) &
QPID=$!

# 45min fixed: 30s status/rollup ticks; full smaps every 300s
START=$(date +%s)
n=0
while :; do
  now=$(date +%s); rel=$((now - RUN_T0)); elapsed=$((now - START))
  [ "$elapsed" -ge "$TOTAL_S" ] && break
  [ -d "/proc/$SVC_PID" ] || { echo "service gone at rel=$rel" > "$EV/abort.txt"; break; }
  printf '%s,%s,%s,%s,%s\n' "$now" "$rel" "$(status_fields /proc/$SVC_PID/status)" \
    "$(rollup_fields /proc/$SVC_PID/smaps_rollup)" >> "$EV/status.csv"
  n=$((n+1))
  if [ $((n * STATUS_INTERVAL % SMAPS_INTERVAL)) -eq 0 ]; then
    take_smaps "base-$((n * STATUS_INTERVAL))"
  fi
  sleep "$STATUS_INTERVAL"
done

touch "$EV/queries.stop"
wait "$QPID" 2>/dev/null || true
QPID=""

take_smaps end || true
sleep 3
kill "$SVC_PID" 2>/dev/null || true
wait "$SVC_PID" 2>/dev/null || true
trap - EXIT INT TERM

grep -c "A+B+C" "$EV/svc.log" > "$EV/rt01-acceptance-count.txt" 2>/dev/null || true
md5sum "$EV"/* > "$EV/md5s.txt" 2>/dev/null || true
echo "E3A_RUN_DONE elapsed_target=${TOTAL_S}s abort=$([ -f "$EV/abort.txt" ] && echo yes || echo no)" | tee "$EV/done.txt"
exit 0
