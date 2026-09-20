#!/usr/bin/env bash
# VBMF standalone lane 版本切换 / 回滚 — STANDALONE-ENTRY-01 S6（SE-01B）
#
# 用法: switch-current.sh <version-tag>
#
# 行为:
#   1. 目标版本目录必须存在且带 install-manifest.json
#   2. fail-closed digest 复核：bin/media-agent 的 sha256 必须与 install-manifest
#      记录一致（篡改/半写目录拒绝切换）
#   3. 原子切换 /opt/vbmf/current（tmp symlink + mv -T）
#
# 回滚 = 本脚本指向旧版本 tag；升级后回滚不修改任何版本目录内容。
set -euo pipefail

usage() { echo "usage: switch-current.sh <version-tag>" >&2; exit 2; }

[ $# -eq 1 ] || usage
TAG=$1
ROOT=/opt/vbmf
DEST="$ROOT/$TAG"
MANIFEST="$DEST/install-manifest.json"

[ -d "$DEST" ] || { echo "version dir not found: $DEST" >&2; exit 2; }
[ -f "$MANIFEST" ] || { echo "install-manifest.json missing: $MANIFEST" >&2; exit 2; }

RECORDED=$(grep -o '"media_agent_sha256": "[0-9a-f]*"' "$MANIFEST" | grep -o '[0-9a-f]\{64\}')
ACTUAL=$(sha256sum "$DEST/bin/media-agent" | awk '{print $1}')
[ "$RECORDED" = "$ACTUAL" ] || {
  echo "digest mismatch for $TAG (recorded=$RECORDED actual=$ACTUAL); refusing to switch" >&2
  exit 2
}

ln -sfn "$DEST" "$ROOT/current.tmp.$$"
mv -T "$ROOT/current.tmp.$$" "$ROOT/current"
echo "current -> $TAG (digest verified)"
