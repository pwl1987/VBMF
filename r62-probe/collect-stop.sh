#!/usr/bin/env bash
# R62 收尾: stop_session（会话级 teardown 链实测）→ 进程停止 → 证据收集 + md5。
set -u
BASE=http://127.0.0.1:8080
OUT="$HOME/a2-8-02i-evidence/2026-09-06-a2-8-05-r62-cp-safety-probe"
exec > >(tee -a "$OUT/run.log") 2>&1

echo "--- [7] stop_session（会话级 teardown） ---"
HEX=$(curl -sf "$BASE/api/v1/runtime" | grep -o '"id":"session-[^"]*"' | head -1 | sed 's/.*session-//;s/"$//')
if [ -n "${HEX:-}" ]; then
  SID=$(printf '%s' "$HEX" | sed 's/\(.\{8\}\)\(.\{4\}\)\(.\{4\}\)\(.\{4\}\)\(.\{12\}\)/\1-\2-\3-\4-\5/')
  s=$(date +%s.%3N)
  curl -s --max-time 60 -w '\nSTOP-HTTPCODE=%{http_code} STOP-TIME=%{time_total}\n' \
    -H 'Content-Type: application/json' -X POST "$BASE/api/v1/commands" \
    -d "{\"command_id\":\"r62-stop-$(date +%s%N)\",\"kind\":\"stop_session\",\"target\":{\"target_type\":\"session_by_id\",\"session_id\":\"$SID\"},\"requested_by\":\"r62-probe\"}"
  echo "STOP-WALL|start=$s"
else
  echo "no-session-row（runtime 面不可用或无会话）"
fi
sleep 2

echo "--- [8] 进程停止 + 日志收取 ---"
PID=$(pgrep -f '^\./target/debug/media-agent' | head -1)
if [ -n "${PID:-}" ]; then
  kill "$PID" 2>/dev/null; sleep 1; kill -9 "$PID" 2>/dev/null
fi
sleep 1
pgrep -f '^\./target/debug/media-agent' >/dev/null && echo "WARN: 进程仍在" || echo "process-dead"
cp /tmp/r62-svc.log "$OUT/svc.log"
grep -c 'watchdog' "$OUT/svc.log" || true
grep 'watchdog' "$OUT/svc.log" | tail -5 > "$OUT/watchdog-tail.txt" || true
cp "$OUT/watchdog-tail.txt" /dev/null 2>/dev/null || true

echo "--- [9] md5 ---"
cd "$OUT" && md5sum ./* > md5s.txt && cat md5s.txt
