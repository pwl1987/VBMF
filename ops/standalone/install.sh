#!/usr/bin/env bash
# VBMF standalone lane installer — STANDALONE-ENTRY-01 S6/S7（SE-01B + SE-01B-FIX）
#
# 用法: install.sh <archive.tar.gz> <version-tag> [ci-run-id]
#   version-tag 必须形如 <version>-<short-sha>（拒绝 floating ref）；
#   <short-sha> 必须是 archive 内嵌 git commit SHA（git get-tar-commit-id）的前缀，
#   否则拒绝安装 —— commit identity 只信 archive 本身，绝不从 filename/tag 猜 SHA。
#   VBMF_INSTALL_FEATURES 覆盖构建 feature 集（默认 bmd,ffmpeg-backend）
#   DECKLINK_SDK_INCLUDE 覆盖 SDK 头路径（默认 /home/lytv/decklink-sdk-include）
#   VBMF_INSTALL_ROOT / VBMF_VAR_LIB_DIR 为测试观测缝（默认 /opt/vbmf、/var/lib/vbmf；
#   生产部署不设置，行为与 SE-01B 相同）
#
# 行为（frozen S6 + SE-01B-FIX）:
#   1. fail-closed provenance：git get-tar-commit-id 读取 archive embedded full 40-hex
#      commit SHA；version-tag 的 short SHA 必须为其前缀；archive 无法证明 commit
#      identity（非 git archive 产物 / 无 embedded id）一律拒装
#   2. 拒绝覆盖已存在的版本目录 —— 禁止原地 mutation 升级
#   3. 全程同文件系统 staging（.staging-<tag>-<pid>）：archive 验证 → extract →
#      release 构建 → bin/ → install-manifest（含 git_commit_sha）→ digest/provenance
#      self-check；任一步失败 trap 自动清理 staging，正式 <tag> 目录不出现，current 不变
#   4. 全部成功后同文件系统 mv -T 原子落成最终 <tag>（落成前再查目标不存在，
#      rename 对已存在空目录会静默成功，必须显式 fail-closed），再原子切 current
#   5. 确保 /var/lib/vbmf 存在（S7 预留，不伪造用途；owner/mode 记入 manifest）
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
ROOT=${VBMF_INSTALL_ROOT:-/opt/vbmf}
VAR_LIB_DIR=${VBMF_VAR_LIB_DIR:-/var/lib/vbmf}
DEST="$ROOT/$TAG"
STAGING="$ROOT/.staging-$TAG-$$"
INSTALL_DONE=0

[ -f "$ARCHIVE" ] || { echo "archive not found: $ARCHIVE" >&2; exit 2; }
[[ "$TAG" =~ ^[0-9][0-9A-Za-z._-]*-[0-9a-f]{7,40}$ ]] || {
  echo "version-tag must be <version>-<commit-sha> (floating refs forbidden): $TAG" >&2
  exit 2
}
SHORT_SHA=${TAG##*-}
[ -e "$DEST" ] && { echo "refusing to overwrite existing version dir (no in-place mutation): $DEST" >&2; exit 2; }

# --- fail-closed provenance（SE-01B-FIX 缺陷3）：commit identity 只从 archive 读取 ---
GIT_COMMIT_SHA=$(gzip -dc "$ARCHIVE" | git get-tar-commit-id || true)
[[ "$GIT_COMMIT_SHA" =~ ^[0-9a-f]{40}$ ]] || {
  echo "archive does not embed a git commit id (not a git-archive product): $ARCHIVE" >&2
  exit 2
}
[[ "$GIT_COMMIT_SHA" == "$SHORT_SHA"* ]] || {
  echo "version-tag short SHA '$SHORT_SHA' is not a prefix of archive embedded commit '$GIT_COMMIT_SHA'; refusing install" >&2
  exit 2
}

# --- staging 原子安装协议（SE-01B-FIX 缺陷4）：失败绝不留下半安装正式目录 ---
cleanup() {
  rc=$?
  if [ "$INSTALL_DONE" != "1" ] && [ -n "${STAGING:-}" ] && [[ "$STAGING" == "$ROOT/.staging-"* ]]; then
    rm -rf -- "$STAGING" || true
    echo "install aborted (rc=$rc); staging removed, final dir '$DEST' untouched: $STAGING" >&2
  fi
}
trap cleanup EXIT
trap 'exit 2' HUP INT TERM

mkdir -p "$ROOT"
mkdir "$STAGING"
ARCHIVE_SHA=$(sha256sum "$ARCHIVE" | awk '{print $1}')

tar xzf "$ARCHIVE" -C "$STAGING"
cd "$STAGING/services/media-agent"
export PATH="$HOME/.cargo/bin:$PATH"
DECKLINK_SDK_INCLUDE="$SDK_INCLUDE" cargo build --release --features "$FEATURES" --bins
mkdir -p "$STAGING/bin"
cp target/release/media-agent target/release/media-agent-gates "$STAGING/bin/"
BIN_SHA=$(sha256sum "$STAGING/bin/media-agent" | awk '{print $1}')
GATES_SHA=$(sha256sum "$STAGING/bin/media-agent-gates" | awk '{print $1}')

# 预留运行时目录（S7: 当前 Runtime 无持久状态；只建立并记录属主，不伪造用途；
# 未来 Runtime 需要写入该目录必须另过 Authority，不在此偷偷赋权）。
# /var/lib 归 root —— 无权限时经 sudo 建立（目标主机现实：BMD 免密 sudo）。
if [ ! -d "$VAR_LIB_DIR" ]; then
  mkdir -p "$VAR_LIB_DIR" 2>/dev/null || sudo mkdir -p "$VAR_LIB_DIR"
fi
VAR_LIB_STAT=$(stat -c '%U:%G %a' "$VAR_LIB_DIR")

cat > "$STAGING/install-manifest.json" <<EOF
{
  "lane": "standalone",
  "version_tag": "$TAG",
  "git_commit_sha": "$GIT_COMMIT_SHA",
  "archive_sha256": "$ARCHIVE_SHA",
  "media_agent_sha256": "$BIN_SHA",
  "media_agent_gates_sha256": "$GATES_SHA",
  "features": "$FEATURES",
  "ci_run_id": "$CI_RUN_ID",
  "var_lib_path": "$VAR_LIB_DIR",
  "var_lib_owner_mode": "$VAR_LIB_STAT",
  "installed_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

# --- digest/provenance self-check（fail-closed；防 manifest 半写/中途替换）---
SC_TAG=$(sed -n 's/.*"version_tag": "\([^"]*\)".*/\1/p' "$STAGING/install-manifest.json")
SC_SHA=$(sed -n 's/.*"git_commit_sha": "\([^"]*\)".*/\1/p' "$STAGING/install-manifest.json")
SC_ASHA=$(sed -n 's/.*"archive_sha256": "\([^"]*\)".*/\1/p' "$STAGING/install-manifest.json")
SC_BSHA=$(sed -n 's/.*"media_agent_sha256": "\([^"]*\)".*/\1/p' "$STAGING/install-manifest.json")
SC_ACTUAL_BSHA=$(sha256sum "$STAGING/bin/media-agent" | awk '{print $1}')
SC_ARCHIVE_SHA=$(gzip -dc "$ARCHIVE" | git get-tar-commit-id || true)
{
  [ "$SC_TAG" = "$TAG" ] &&
  [ "$SC_SHA" = "$GIT_COMMIT_SHA" ] &&
  [ "$SC_ASHA" = "$ARCHIVE_SHA" ] &&
  [ "$SC_BSHA" = "$SC_ACTUAL_BSHA" ] &&
  [ "$SC_ARCHIVE_SHA" = "$GIT_COMMIT_SHA" ]
} || {
  echo "install-manifest digest/provenance self-check failed; refusing to finalize" >&2
  exit 2
}

# --- 原子落成（promote）：rename 前显式 fail-closed（rename(2) 对已存空目录会成功）---
[ ! -e "$DEST" ] || {
  echo "final version dir appeared during install (concurrent writer?); refusing: $DEST" >&2
  exit 2
}
mv -T "$STAGING" "$DEST"
INSTALL_DONE=1

# 原子切换 current（tmp symlink + rename；读侧任一时刻看到完整旧或新版本）
ln -sfn "$DEST" "$ROOT/current.tmp.$$"
mv -T "$ROOT/current.tmp.$$" "$ROOT/current"
echo "installed $TAG"
echo "  git_commit_sha=$GIT_COMMIT_SHA"
echo "  archive_sha256=$ARCHIVE_SHA"
echo "  media_agent_sha256=$BIN_SHA"
echo "  var_lib=$VAR_LIB_STAT"
echo "  current -> $ROOT/current"
