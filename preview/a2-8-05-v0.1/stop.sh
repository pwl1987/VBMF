#!/usr/bin/env bash
# v0.1 停止脚本 —— 先经 API stop_session（会话级 teardown: Program Stop→Tap Detach→
# Input Stop→资源归还→Released）, 再 kill 常驻进程（缺口④如实: 无信号处理）。
set -euo pipefail
cd "$(dirname "$0")"

BIND="${MEDIA_AGENT_HEALTH_BIND:-127.0.0.1:8080}"
PID_FILE=media-agent.pid
[ -f "${PID_FILE}" ] || { echo "no pid file (未在运行?)"; exit 1; }
PID="$(cat "${PID_FILE}")"

# 1) 会话级停止（若 runtime 面可用）。
#    runtime 会话行 id 为展示形 "session-<32hex>"; CommandById 须 canonical UUID。
HEX="$(curl -sf "http://${BIND}/api/v1/runtime" 2>/dev/null \
  | grep -o '"id":"session-[^"]*"' | head -1 | sed 's/.*session-//;s/"$//')"
if [ -n "${HEX}" ]; then
  SID="$(printf '%s' "${HEX}" | sed 's/\(.\{8\}\)\(.\{4\}\)\(.\{4\}\)\(.\{4\}\)\(.\{12\}\)/\1-\2-\3-\4-\5/')"
  RESP="$(curl -s -m 10 -X POST "http://${BIND}/api/v1/commands" \
    -H 'Content-Type: application/json' \
    -d "{\"command_id\":\"v01-stop-$(date +%s)\",\"kind\":\"stop_session\",\"target\":{\"target_type\":\"session_by_id\",\"session_id\":\"${SID}\"},\"requested_by\":\"v0.1-stop.sh\"}" || true)"
  echo "stop_session ${SID} -> ${RESP:-<empty>}"
else
  echo "no active session found (runtime 面不可用或无会话) — 直接进程停止"
fi

# 2) 进程停止（常驻形态·无信号处理 → kill）
kill "${PID}" 2>/dev/null || true
sleep 1
kill -9 "${PID}" 2>/dev/null || true
rm -f "${PID_FILE}"
echo "process stopped (pid=${PID})"
