#!/usr/bin/env bash
# C2-O3-1 / E2 single-input differential diagnostic harness.
# Diagnostic-only: no Step15 verdict, no production source changes, no threshold changes.
set -uo pipefail

TAG="${TAG:-c2o3-e2-single-input-2p5h}"
CYCLES=315
DWELL=28
NOMINAL_S=9200
REQBY="${REQBY:-c2o3-e2-single-input}"
EV_ROOT="${EV_ROOT:-$HOME/a2-8-02i-evidence}"
ROOT="${ROOT:-$HOME/media-agent-build}"
AGENT_DIR="$ROOT/services/media-agent"
EV="$EV_ROOT/$(date +%Y-%m-%d)-$TAG"
OBS_OUT="$EV_ROOT/$(date +%Y-%m-%d)-${TAG}-observer"

mkdir -p "$EV"
cd "$AGENT_DIR"

export DECKLINK_SDK_INCLUDE="$HOME/decklink-sdk-include"
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
source "$HOME/.cargo/env"
bash scripts/build-bmd.sh > "$EV/build.log" 2>&1 || { echo "build failed" > "$EV/abort.txt"; exit 2; }
BIN_MD5=$(md5sum target/debug/media-agent | awk '{print $1}')
MAN="$HOME/a2-8-02i-v5.manifest.json"
MAN_MD5=$(md5sum "$MAN" | awk '{print $1}')
cat > "$EV/header.txt" <<HDR
run_kind=C2-O3-1/E2 single-input differential diagnostic
step15_verdict=INAPPLICABLE
inputs=1
cycles=$CYCLES
dwell_s=$DWELL
nominal_observer_s=$NOMINAL_S
bin_md5=$BIN_MD5
manifest_md5=$MAN_MD5
frozen_question=dual-input/dual-instance condition necessary for target allocation-path event?
interpretation=differential evidence only; never causal conclusion
rss_threshold_reference=+50MB unchanged; not used to re-adjudicate prior 24h FAIL
HDR

export MEDIA_AGENT_DEVICE_BINDING="$MAN"
export MEDIA_AGENT_MODE=diagnostic
export VBMF_DIAG_INPUTS=1
export MEDIA_AGENT_HEALTH_BIND=127.0.0.1:8080
export RUST_LOG=info
nohup ./target/debug/media-agent > "$EV/svc.log" 2>&1 &
SVC_PID=$!
echo "$SVC_PID" > "$EV/pid"

cleanup() {
  touch "$EV/queries.stop" 2>/dev/null || true
  [ -n "${QPID:-}" ] && kill "$QPID" 2>/dev/null || true
  kill "$SVC_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

ok=0
for _ in $(seq 1 80); do
  code=$(curl -s -o "$EV/health0.json" -w "%{http_code}" http://127.0.0.1:8080/health || true)
  [ "$code" = "200" ] && { ok=1; break; }
  sleep 0.25
done
[ "$ok" = "1" ] || { echo "service not ready" > "$EV/abort.txt"; exit 2; }

curl -s http://127.0.0.1:8080/api/v1/runtime > "$EV/runtime-before.json"
python3 - "$EV/runtime-before.json" > "$EV/topology-check.txt" <<'PY'
import json, sys, uuid
j=json.load(open(sys.argv[1]))
sessions=j.get("sessions") or []
if len(sessions) != 1:
    raise SystemExit(f"E2_TOPOLOGY_FAIL sessions={len(sessions)} expected=1")
s=sessions[0]
inputs=s.get("inputs") or []
if len(inputs) != 1:
    raise SystemExit(f"E2_TOPOLOGY_FAIL inputs={len(inputs)} expected=1")
if j.get("program_switch") is not None:
    raise SystemExit("E2_TOPOLOGY_FAIL program_switch present; program graph must be absent")
raw=s["id"].replace("session-", "").replace("-", "")
sid=str(uuid.UUID(raw))
print("E2_TOPOLOGY_OK")
print(f"SID={sid}")
print(f"INPUT={inputs[0]['id']}")
print("PROGRAM_SWITCH=ABSENT")
PY
TOPO_RC=$?
[ "$TOPO_RC" -eq 0 ] || { echo "single-input topology assertion failed" > "$EV/abort.txt"; exit 2; }
SID=$(awk -F= '/^SID=/{print $2}' "$EV/topology-check.txt")

# Reuse the already-authorized O2 observer surface/cadence unchanged.
TAG="$TAG" NOMINAL_S="$NOMINAL_S" OUT_DIR="$OBS_OUT" \
  bash "$ROOT/r64-probe/c2o2-observer.sh" > "$EV/observer-launch.log" 2>&1 &
OBS_PID=$!

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

SAMP="$EV/samples.csv"
echo "cycle,ts,rss_kb,fd,threads,dropped,clock_lost,http_runtime" > "$SAMP"
for c in $(seq 1 "$CYCLES"); do
  RSS=$(awk '/VmRSS/{print $2}' "/proc/$SVC_PID/status")
  FD=$(ls "/proc/$SVC_PID/fd" 2>/dev/null | wc -l)
  THR=$(awk '/Threads/{print $2}' "/proc/$SVC_PID/status")
  H=$(curl -s http://127.0.0.1:8080/health)
  DROP=$(printf '%s' "$H" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("dropped_bus_events"))')
  CLK=$(printf '%s' "$H" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("clock_lost_events"))')
  RC=$(curl -s -o "$EV/runtime-last.json" -w "%{http_code}" http://127.0.0.1:8080/api/v1/runtime || true)
  echo "$c,$(date +%s),$RSS,$FD,$THR,$DROP,$CLK,$RC" >> "$SAMP"
  sleep "$DWELL"
done

touch "$EV/queries.stop"
wait "$QPID" 2>/dev/null || true
QPID=""

python3 - "$SID" "$REQBY" > "$EV/stop-response.json" <<'PY'
import sys,json,uuid,urllib.request
sid,reqby=sys.argv[1:]
body=json.dumps({"command_id":str(uuid.uuid4()),"kind":"stop_session",
 "target":{"target_type":"session_by_id","session_id":sid},"requested_by":reqby}).encode()
req=urllib.request.Request("http://127.0.0.1:8080/api/v1/commands",data=body,
 headers={"Content-Type":"application/json"},method="POST")
with urllib.request.urlopen(req,timeout=30) as r: print(r.read().decode())
PY
sleep 3
kill "$SVC_PID" 2>/dev/null || true
wait "$SVC_PID" 2>/dev/null || true
trap - EXIT INT TERM
wait "$OBS_PID"; OBS_RC=$?

echo "$OBS_RC" > "$EV/observer-exit.txt"
python3 - "$EV" "$OBS_OUT" > "$EV/e2-summary.txt" <<'PY'
import csv,json,sys,os
run,obs=sys.argv[1:]
meta_path=os.path.join(obs,"meta.json")
if not os.path.exists(meta_path):
    print("E2_RESULT=INCONCLUSIVE")
    print("reason=observer meta missing")
    raise SystemExit(0)
meta=json.load(open(meta_path))
rows=list(csv.DictReader(open(os.path.join(run,"samples.csv"))))
rss=[int(r["rss_kb"]) for r in rows]
print(f"observer_status={meta.get('observer_status')}")
print(f"observer_exit={meta.get('exit_code')}")
print(f"coverage_pct={meta.get('coverage_pct')}")
print(f"trig_positive={meta.get('trig_positive')}")
print(f"trig_negative={meta.get('trig_negative')}")
if rss:
    print(f"rss_start_kb={rss[0]}")
    print(f"rss_end_kb={rss[-1]}")
    print(f"rss_delta_kb={rss[-1]-rss[0]}")
if meta.get("observer_status") != "COMPLETE":
    print("E2_RESULT=INCONCLUSIVE")
    print("reason=incomplete observation window")
elif int(meta.get("trig_positive") or 0) > 0:
    print("E2_RESULT=SINGLE_INPUT_EVENT_OBSERVED")
    print("allowed_conclusion=dual-input is not a necessary condition; narrow toward common ingest/native allocation path")
else:
    print("E2_RESULT=SINGLE_INPUT_EVENT_NOT_OBSERVED")
    print("allowed_conclusion=dual-input condition gains weight as a necessary condition candidate only; NOT causal proof")
print("step15_verdict=UNCHANGED_24H_FAIL")
PY

cat "$EV/e2-summary.txt"
md5sum "$EV"/* > "$EV/md5s.txt" 2>/dev/null || true
exit 0
