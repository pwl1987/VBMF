#!/usr/bin/env bash
# VBMF standalone lane 版本切换 / 回滚 — STANDALONE-ENTRY-01 S6（SE-01B + SE-01B-FIX）
#
# 用法: switch-current.sh <version-tag>
#
# 行为:
#   1. 目标版本目录必须存在且带 install-manifest.json
#   2. fail-closed provenance 复核（SE-01B-FIX）：
#      a. manifest version_tag 必须与请求 tag 一致（防目录/manifest 错位）
#      b. manifest git_commit_sha 必须是 full 40-hex（无该字段的 legacy manifest 拒绝）
#      c. tag 的 short SHA 必须是 git_commit_sha 前缀（手工伪造 tag 不能成为 Authority）
#      d. bin/media-agent sha256 必须与 manifest 记录一致（篡改/半写目录拒绝切换）
#   3. 原子切换 /opt/vbmf/current（tmp symlink + mv -T）
#
# 回滚 = 本脚本指向旧版本 tag；升级后回滚不修改任何版本目录内容。
# VBMF_INSTALL_ROOT 为测试观测缝（默认 /opt/vbmf；生产不设置）。
set -euo pipefail

usage() { echo "usage: switch-current.sh <version-tag>" >&2; exit 2; }

[ $# -eq 1 ] || usage
TAG=$1
ROOT=${VBMF_INSTALL_ROOT:-/opt/vbmf}
DEST="$ROOT/$TAG"
MANIFEST="$DEST/install-manifest.json"

[ -d "$DEST" ] || { echo "version dir not found: $DEST" >&2; exit 2; }
[ -f "$MANIFEST" ] || { echo "install-manifest.json missing: $MANIFEST" >&2; exit 2; }

M_TAG=$(sed -n 's/.*"version_tag": "\([^"]*\)".*/\1/p' "$MANIFEST")
M_SHA=$(sed -n 's/.*"git_commit_sha": "\([^"]*\)".*/\1/p' "$MANIFEST")
RECORDED=$(grep -o '"media_agent_sha256": "[0-9a-f]*"' "$MANIFEST" | grep -o '[0-9a-f]\{64\}')

[ "$M_TAG" = "$TAG" ] || {
  echo "manifest version_tag '$M_TAG' != requested tag '$TAG'; refusing to switch" >&2
  exit 2
}
[[ "$M_SHA" =~ ^[0-9a-f]{40}$ ]] || {
  echo "install-manifest has no valid full git_commit_sha (legacy/tampered manifest: '${M_SHA:-missing}'); refusing to switch" >&2
  exit 2
}
SHORT_SHA=${TAG##*-}
[[ "$M_SHA" == "$SHORT_SHA"* ]] || {
  echo "tag short SHA '$SHORT_SHA' is not a prefix of manifest git_commit_sha '$M_SHA'; refusing to switch" >&2
  exit 2
}
ACTUAL=$(sha256sum "$DEST/bin/media-agent" | awk '{print $1}')
[ "$RECORDED" = "$ACTUAL" ] || {
  echo "digest mismatch for $TAG (recorded=$RECORDED actual=$ACTUAL); refusing to switch" >&2
  exit 2
}

ln -sfn "$DEST" "$ROOT/current.tmp.$$"
mv -T "$ROOT/current.tmp.$$" "$ROOT/current"
echo "current -> $TAG (provenance verified: commit=$M_SHA digest=ok)"
