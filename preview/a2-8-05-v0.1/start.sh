#!/usr/bin/env bash
# v0.1 启动脚本 —— 诊断模式双输入自启（零代码形态; runbook 见 README.md）
set -euo pipefail
cd "$(dirname "$0")"

: "${MEDIA_AGENT_DEVICE_BINDING:?需要 manifest 路径 (export MEDIA_AGENT_DEVICE_BINDING=...; 见 env.sample)}"
[ -x ./media-agent ] || { echo "ERROR: ./media-agent 不存在或不可执行 (按 IDENTITY 部署)" >&2; exit 1; }
[ -f "${MEDIA_AGENT_DEVICE_BINDING}" ] || { echo "ERROR: manifest 不存在: ${MEDIA_AGENT_DEVICE_BINDING}" >&2; exit 1; }

export MEDIA_AGENT_MODE=diagnostic
export VBMF_DIAG_INPUTS=2
export RUST_LOG="${RUST_LOG:-info}"
LOG="${V0_1_LOG:-media-agent-v0.1.log}"

nohup ./media-agent >"${LOG}" 2>&1 &
echo $! > media-agent.pid
echo "started pid=$(cat media-agent.pid) log=${LOG} health=http://${MEDIA_AGENT_HEALTH_BIND:-127.0.0.1:8080}/health"
