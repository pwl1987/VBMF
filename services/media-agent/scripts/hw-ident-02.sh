#!/usr/bin/env bash
# HW-IDENT-02: 多轮冷启动复核 DeviceHandle 身份稳定性 + Manifest 绑定闭环
#
# 背景 (见 device.rs / resolver.rs / abda19f 证据):
#   - device.rs 明示 "DeviceHandle 非跨重启永久稳定"; canonical device_id = UUIDv5(handle)。
#   - Manifest 绑定契约按 bmd_device_handle 索引, 清单 lookup 失败 -> 整条目 Unresolved (失败闭合)。
#   - 因此 HW-IDENT-02 的核心判定: 多次冷启动后, 本机全部 DeckLink 的 bmd_device_handle 集合
#     是否逐轮一致 (若漂移, canonical=DeviceHandle 决策失效, 须 V0.3 返工)。
#
# 本脚本在【每次冷启动后】运行一次, 提取 C1 Resolver Evidence, 记录每个设备的:
#   bmd_device_handle / device_id / gst_device_number / gst_hw_serial_number / match_kind
# 供 hw-ident-02-collate.sh 跨轮比对。
#
# 用法 (每次冷启动后执行一次):
#   ./scripts/hw-ident-02.sh <round> [manifest_path]
#     round:          冷启动轮次 (1,2,3,...) — 字符串, 仅用于命名
#     manifest_path:  默认 /tmp/manifest.json
#
# 产物 (目录 $HW_IDENT_OUT, 默认 /tmp/hw-ident-02):
#   round-<N>-<ts>.txt      完整 stdout (含三段标记)
#   round-<N>-evidence.json 规范化后的 Resolver Evidence (按 bmd_device_handle 排序)
#   round-<N>.tsv           一行一设备: handle/device_id/gst_device_number/serial/match_kind
set -euo pipefail

ROUND="${1:?用法: hw-ident-02.sh <round> [manifest]}"
MANIFEST="${2:-/tmp/manifest.json}"
BINARY="${MEDIA_AGENT_BIN:-./target/debug/media-agent}"
OUTDIR="${HW_IDENT_OUT:-/tmp/hw-ident-02}"
mkdir -p "$OUTDIR"

ts=$(date -u +%Y%m%dT%H%M%SZ)
runfile="$OUTDIR/round-${ROUND}-${ts}.txt"
jsonfile="$OUTDIR/round-${ROUND}-evidence.json"
tsvfile="$OUTDIR/round-${ROUND}.tsv"

VBMF_RESOLVER=1 \
MEDIA_AGENT_MODE=diagnostic \
VBMF_MACHINE_ID=10.30.15.10 \
MEDIA_AGENT_DEVICE_BINDING="$MANIFEST" \
  "$BINARY" > "$runfile" 2>&1

# 抽取 C1 Resolver Evidence JSON 段 -> 规范化(按 bmd_device_handle 排序) -> json + tsv
python3 - "$runfile" "$jsonfile" "$tsvfile" <<'PY'
import sys, json, re
runfile, jsonfile, tsvfile = sys.argv[1], sys.argv[2], sys.argv[3]
txt = open(runfile, encoding="utf-8", errors="replace").read()
m = re.search(r"=== C1 Resolver Evidence.*===\n(.*?)\n=== C1 Resolved Bindings", txt, re.S)
if not m:
    sys.exit("ERROR: 未找到 C1 Resolver Evidence 段, 检查二进制是否以 --features bmd,gstreamer 构建")
ev = json.loads(m.group(1))
ev_sorted = sorted(ev, key=lambda d: (d.get("bmd_device_handle") or ""))
open(jsonfile, "w", encoding="utf-8").write(json.dumps(ev_sorted, indent=2, ensure_ascii=False))
with open(tsvfile, "w", encoding="utf-8") as f:
    f.write("bmd_device_handle\tdevice_id\tgst_device_number\tgst_hw_serial_number\tmatch_kind\n")
    for d in ev_sorted:
        gn = d.get("gst_device_number")
        f.write("\t".join([
            str(d.get("bmd_device_handle") or ""),
            str(d.get("device_id") or ""),
            str(gn if gn is not None else ""),
            str(d.get("gst_hw_serial_number") or ""),
            str(d.get("match_kind") or ""),
        ]) + "\n")
PY

echo "ROUND $ROUND 已捕获:"
echo "  stdout   -> $runfile"
echo "  evidence -> $jsonfile"
echo "  tsv      -> $tsvfile"
echo "  handle 快照 (handle | match_kind):"
tail -n +2 "$tsvfile" | cut -f1,5 | sed 's/^/    /'
