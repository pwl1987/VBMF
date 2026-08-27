#!/usr/bin/env bash
# HW-IDENT-02 跨轮比对: 比较各轮 TSV, 判定 DeviceHandle 是否跨冷启动稳定 + 绑定闭环是否逐轮成立。
# 用法: ./scripts/hw-ident-02-collate.sh [outdir]
set -euo pipefail
OUTDIR="${1:-/tmp/hw-ident-02}"
cd "$OUTDIR"
shopt -s nullglob
rounds=($(printf '%s\n' round-*.tsv 2>/dev/null | sort))
if [ ${#rounds[@]} -eq 0 ]; then echo "无轮次数据: $OUTDIR"; exit 1; fi

echo "=== HW-IDENT-02 跨轮比对 (共 ${#rounds[@]} 轮) ==="
echo

echo "[1] 每轮 bmd_device_handle 集合 (跨冷启动稳定性):"
prev_set=""
handle_stable=1
for r in "${rounds[@]}"; do
  set=$(tail -n +2 "$r" | cut -f1 | sort | tr '\n' ' ')
  echo "  $r -> [$set]"
  if [ -n "$prev_set" ] && [ "$prev_set" != "$set" ]; then handle_stable=0; fi
  prev_set="$set"
done

echo
echo "[2] 逐设备 match_kind 跨轮一致性:"
inconsistent=0
all_handles=$(for r in "${rounds[@]}"; do tail -n +2 "$r" | cut -f1; done | sort -u)
while IFS= read -r h; do
  [ -z "$h" ] && continue
  kinds=""
  for r in "${rounds[@]}"; do
    k=$(awk -F'\t' -v h="$h" '$1==h {print $5}' "$r" | head -1)
    kinds="$kinds [$r=$k]"
  done
  ucount=$(for r in "${rounds[@]}"; do awk -F'\t' -v h="$h" '$1==h {print $5}' "$r"; done | sort -u | wc -l)
  echo "  handle=$h :$kinds"
  if [ "$ucount" -gt 1 ]; then inconsistent=1; fi
done <<< "$all_handles"

echo
echo "[3] 判定:"
if [ "$handle_stable" -eq 1 ] && [ "$inconsistent" -eq 0 ]; then
  echo "RESULT=PASS : DeviceHandle 跨冷启动稳定, 绑定闭环(match_kind)逐轮一致"
  exit 0
else
  echo "RESULT=FAIL : 冷启动间 DeviceHandle 或 match_kind 发生变化 (见上)"
  exit 2
fi
