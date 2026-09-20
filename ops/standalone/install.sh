#!/usr/bin/env bash
# VBMF standalone lane installer — STANDALONE-ENTRY-01 S6/S7（SE-01B）
#
# 用法: install.sh <archive.tar.gz> <version-tag> [ci-run-id]
#   version-tag 必须形如 <version>-<short-sha>（拒绝 floating ref）
#   VBMF_INSTALL_FEATURES 覆盖构建 feature 集（默认 bmd,ffmpeg-backend）
#   DECKLINK_SDK_INCLUDE 覆盖 SDK 头路径（默认 /home/lytv/decklink-sdk-include）
#
# 行为（frozen S6）:
#   1. 拒绝覆盖已存在的版本目录 —— 禁止原地 mutation 升级
#   2. 展开到 /opt/vbmf/<version-tag>/（git archive = exact-commit 源）
#   3. 版本目录内 release 构建，binary 复制到 <dir>/bin/
#   4. 写 <dir>/install-manifest.json（tag / archive sha256 / binary sha256 / features / ci-run-id / 时间）
#   5. 原子切换 /opt/vbmf/current（tmp symlink + mv -T）
#   6. 确保 /var/lib/vbmf 存在（预留，不伪造用途）
#
# 边界: 绝不触碰 /opt/vbmf-dev；绝不修改既有版本目录内容；不读取/不复制 Runtime 状态
# （Runtime owns truth —— 本脚本只做 文件/构建/symlink，无任何健康或状态判断）。
set -euo pipefail

usage() { echo "usage: install.sh <archive.tar.gz> <version-tag> [ci-run-id]" >&2; exit 2; }

[ $# -ge 2 ] || usage
ARCHIVE=$1
TAG=$2
CI_RUN_ID=${3:-none}
FEATURES=${VBMF_INSTALL_FEATURES:-bmd,ffmpeg-backend}
SDK_INCLUDE=${DECKLINK_SDK_INCLUDE:-/home/lytv/decklink-sdk-include}
ROOT=/opt/vbmf
DEST="$ROOT/$TAG"

[ -f "$ARCHIVE" ] || { echo "archive not found: $ARCHIVE" >&2; exit 2; }
[[ "$TAG" =~ ^[0-9][0-9A-Za-z._-]*-[0-9a-f]{7,40}$ ]] || {
  echo "version-tag must be <version>-<commit-sha> (floating refs forbidden): $TAG" >&2
  exit 2
}
[ -e "$DEST" ] && { echo "refusing to overwrite existing version dir (no in-place mutation): $DEST" >&2; exit 2; }
mkdir -p "$ROOT"
ARCHIVE_SHA=$(sha256sum "$ARCHIVE" | awk '{print $1}')

mkdir "$DEST"
tar xzf "$ARCHIVE" -C "$DEST"
cd "$DEST/services/media-agent"
export PATH="$HOME/.cargo/bin:$PATH"
DECKLINK_SDK_INCLUDE="$SDK_INCLUDE" cargo build --release --features "$FEATURES" --bins
mkdir -p "$DEST/bin"
cp target/release/media-agent target/release/media-agent-gates "$DEST/bin/"
BIN_SHA=$(sha256sum "$DEST/bin/media-agent" | awk '{print $1}')
GATES_SHA=$(sha256sum "$DEST/bin/media-agent-gates" | awk '{print $1}')

cat > "$DEST/install-manifest.json" <<EOF
{
  "lane": "standalone",
  "version_tag": "$TAG",
  "archive_sha256": "$ARCHIVE_SHA",
  "media_agent_sha256": "$BIN_SHA",
  "media_agent_gates_sha256": "$GATES_SHA",
  "features": "$FEATURES",
  "ci_run_id": "$CI_RUN_ID",
  "installed_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

# 预留运行时目录（S7: 当前 Runtime 无持久状态；只建立并保持属主，不伪造用途）
# /var/lib 归 root —— 经 sudo 建立（目标主机现实：BMD 免密 sudo；无 sudo 主机可预建目录）
if [ ! -d /var/lib/vbmf ]; then
  sudo mkdir -p /var/lib/vbmf
fi

# 原子切换 current（tmp symlink + rename；读侧任一时刻看到完整旧或新版本）
ln -sfn "$DEST" "$ROOT/current.tmp.$$"
mv -T "$ROOT/current.tmp.$$" "$ROOT/current"
echo "installed $TAG"
echo "  archive_sha256=$ARCHIVE_SHA"
echo "  media_agent_sha256=$BIN_SHA"
echo "  current -> $ROOT/current"
