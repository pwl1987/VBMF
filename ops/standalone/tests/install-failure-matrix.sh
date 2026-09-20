#!/usr/bin/env bash
# SE-01B-FIX installer/switcher failure-injection matrix（Development VM 运行）
#
# 机械证明（用户 2026-09-20 验收要求）：
#   * bad archive（无 embedded commit id）        → 拒装，正式目录不出现
#   * provenance mismatch（shortsha 非前缀）      → 拒装，正式目录不出现
#   * build failure                              → 拒装，正式目录不出现，staging 清理
#   * manifest/provenance 篡改（switch 面）      → 拒绝切换（伪造 tag 不能成为 Authority）
#   * 全部失败路径上 current 不变、无 .staging-* 残留
#   * 成功安装后 final dir 完整（manifest/digest 逐字段复核）
#   * 重装同 tag 拒绝且旧目录字节不变
#
# 测试观测缝：VBMF_INSTALL_ROOT / VBMF_VAR_LIB_DIR 指向 mktemp 目录；cargo 用
# stub（真构建证明 = BMD exact-commit 安装）；HOME 重定向避免真实 ~/.cargo/bin
# 抢占 stub。生产部署不设置这些 env，install.sh/switch-current.sh 行为不变。
set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
INSTALL="$REPO_ROOT/ops/standalone/install.sh"
SWITCH="$REPO_ROOT/ops/standalone/switch-current.sh"

PASS=0; FAIL=0
ok()  { PASS=$((PASS+1)); echo "PASS: $*"; }
bad() { FAIL=$((FAIL+1)); echo "FAIL: $*" >&2; }
expect_fail() { local d=$1; shift; if "$@" >/dev/null 2>&1; then bad "$d (unexpected rc=0)"; else ok "$d"; fi; }
expect_ok()   { local d=$1; shift; if "$@" >/dev/null 2>&1; then ok "$d"; else bad "$d (unexpected nonzero rc)"; fi; }
expect_true() { local d=$1; shift; if "$@"; then ok "$d"; else bad "$d"; fi; }

WORK=$(mktemp -d /tmp/vbmf-se01b-fix-matrix.XXXXXX)
cleanup() { rm -rf -- "$WORK"; }
trap cleanup EXIT
ROOT="$WORK/opt-vbmf"
VARLIB_PARENT="$WORK/var-lib"
STUB="$WORK/stubbin"
FAKEHOME="$WORK/home"
mkdir -p "$ROOT" "$VARLIB_PARENT" "$STUB" "$FAKEHOME"

# --- stub cargo：成功 = 产出可执行 stub binaries；STUB_CARGO_FAIL=1 = 注入构建失败 ---
cat > "$STUB/cargo" <<'STUBEOF'
#!/usr/bin/env bash
if [ "${STUB_CARGO_FAIL:-0}" = "1" ]; then
  echo "stub cargo: injected build failure" >&2
  exit 1
fi
mkdir -p target/release
printf '#!/bin/sh\nexit 0\n' > target/release/media-agent
printf '#!/bin/sh\nexit 0\n' > target/release/media-agent-gates
chmod +x target/release/media-agent target/release/media-agent-gates
STUBEOF
chmod +x "$STUB/cargo"

TESTENV() {
  env PATH="$STUB:$PATH" HOME="$FAKEHOME" \
    VBMF_INSTALL_ROOT="$ROOT" VBMF_VAR_LIB_DIR="$VARLIB_PARENT/vbmf" "$@"
}

# --- 真实 git archive（embedded commit id = HEAD full SHA）---
FULL=$(git -C "$REPO_ROOT" rev-parse HEAD)
SHORT=${FULL:0:7}
# 一个保证不等于 FULL 前缀的 7-hex 串（首位取反）
WRONG_SHORT=""
for ((i=0; i<7; i++)); do
  c=${SHORT:$i:1}
  if [ "$c" = "0" ]; then WRONG_SHORT+="1"; else WRONG_SHORT+="0"; fi
done
TAG="0.1.0-$SHORT"
TAG2="0.2.0-$SHORT"
ARCHIVE="$WORK/real.tar.gz"
git -C "$REPO_ROOT" archive --format=tar.gz HEAD -o "$ARCHIVE"

# 与 install.sh 无关的独立 sanity：git get-tar-commit-id 可用且 archive 内嵌 == FULL
EMBEDDED=$(gzip -dc "$ARCHIVE" | git get-tar-commit-id || true)
expect_true "tool sanity: git get-tar-commit-id == rev-parse HEAD" test "$EMBEDDED" = "$FULL"

no_staging_residue() { [ -z "$(find "$ROOT" -maxdepth 1 -name '.staging-*' -print -quit)" ]; }
current_points_to() { [ "$(readlink "$ROOT/current" 2>/dev/null || true)" = "$ROOT/$1" ]; }
dir_absent() { [ ! -e "$ROOT/$1" ]; }

# --- 失败注入：坏 archive（非 git archive 产物，无 embedded commit id）---
mkdir -p "$WORK/junk"; echo not-a-git-archive > "$WORK/junk/f"
tar czf "$WORK/bad.tar.gz" -C "$WORK" junk
expect_fail "bad archive (no embedded commit id) refused" \
  TESTENV bash "$INSTALL" "$WORK/bad.tar.gz" "$TAG" ci-test
expect_true "bad archive: final dir absent" dir_absent "$TAG"
expect_true "bad archive: no staging residue" no_staging_residue
expect_true "bad archive: no current created" dir_absent current

# --- 失败注入：provenance mismatch（tag shortsha 非 embedded SHA 前缀）---
expect_fail "shortsha/prefix mismatch refused" \
  TESTENV bash "$INSTALL" "$ARCHIVE" "0.1.0-$WRONG_SHORT" ci-test
expect_true "mismatch: final dir absent" dir_absent "0.1.0-$WRONG_SHORT"
expect_true "mismatch: no staging residue" no_staging_residue

# --- 失败注入：构建失败（stub cargo exit 1）---
expect_fail "build failure refused" \
  TESTENV STUB_CARGO_FAIL=1 bash "$INSTALL" "$ARCHIVE" "$TAG" ci-test
expect_true "build failure: final dir absent" dir_absent "$TAG"
expect_true "build failure: no staging residue" no_staging_residue

# --- 成功安装：final dir 完整性逐字段复核 ---
expect_ok "valid install succeeds" TESTENV bash "$INSTALL" "$ARCHIVE" "$TAG" ci-35494022684
M="$ROOT/$TAG/install-manifest.json"
fld() { sed -n "s/.*\"$1\": \"\\([^\"]*\\)\".*/\\1/p" "$M"; }
expect_true "manifest git_commit_sha == archive embedded FULL" test "$(fld git_commit_sha)" = "$FULL"
expect_true "manifest archive_sha256 == recomputed" \
  test "$(fld archive_sha256)" = "$(sha256sum "$ARCHIVE" | awk '{print $1}')"
expect_true "manifest media_agent_sha256 == recomputed" \
  test "$(fld media_agent_sha256)" = "$(sha256sum "$ROOT/$TAG/bin/media-agent" | awk '{print $1}')"
expect_true "manifest var_lib owner/mode recorded" test -n "$(fld var_lib_owner_mode)"
expect_true "success: current -> tag" current_points_to "$TAG"
expect_true "success: no staging residue" no_staging_residue

# --- 重装同 tag：拒绝且旧目录字节不变 ---
MAN_SHA_BEFORE=$(sha256sum "$M" | awk '{print $1}')
BIN_SHA_BEFORE=$(sha256sum "$ROOT/$TAG/bin/media-agent" | awk '{print $1}')
expect_fail "reinstall same tag refused" TESTENV bash "$INSTALL" "$ARCHIVE" "$TAG" ci-other
expect_true "reinstall refusal: manifest bytes unchanged" \
  test "$MAN_SHA_BEFORE" = "$(sha256sum "$M" | awk '{print $1}')"
expect_true "reinstall refusal: binary bytes unchanged" \
  test "$BIN_SHA_BEFORE" = "$(sha256sum "$ROOT/$TAG/bin/media-agent" | awk '{print $1}')"
expect_true "reinstall refusal: current unchanged" current_points_to "$TAG"

# --- 第二个完整安装（同 commit、不同 version 段）：作为切换/回滚与篡改基线 ---
expect_ok "second valid install (0.2.0) succeeds" TESTENV bash "$INSTALL" "$ARCHIVE" "$TAG2" ci-35494022684
expect_ok "switch to 0.2.0" TESTENV bash "$SWITCH" "$TAG2"
expect_true "current -> 0.2.0" current_points_to "$TAG2"

# --- 切换面篡改矩阵（手工伪造 tag/manifest 不能成为 Authority）---
# digest 篡改
cp -a "$ROOT/$TAG2" "$ROOT/tamper-digest"
printf 'X' >> "$ROOT/tamper-digest/bin/media-agent"
expect_fail "switch refused: binary digest tampered" TESTENV bash "$SWITCH" tamper-digest
expect_true "digest refusal: current unchanged" current_points_to "$TAG2"
# git_commit_sha 篡改（40-hex 全零：格式合法但前缀不符）
cp -a "$ROOT/$TAG2" "$ROOT/tamper-sha"
ZEROS=$(printf '0%.0s' $(seq 1 40))
sed -i "s/\"git_commit_sha\": \"[0-9a-f]*\"/\"git_commit_sha\": \"$ZEROS\"/" "$ROOT/tamper-sha/install-manifest.json"
expect_fail "switch refused: git_commit_sha not prefix of tag" TESTENV bash "$SWITCH" tamper-sha
expect_true "sha refusal: current unchanged" current_points_to "$TAG2"
# legacy manifest（无 git_commit_sha 字段）
cp -a "$ROOT/$TAG2" "$ROOT/tamper-legacy"
sed -i '/"git_commit_sha"/d' "$ROOT/tamper-legacy/install-manifest.json"
expect_fail "switch refused: legacy manifest without git_commit_sha" TESTENV bash "$SWITCH" tamper-legacy
expect_true "legacy refusal: current unchanged" current_points_to "$TAG2"
# manifest version_tag 与目录 tag 错位
cp -a "$ROOT/$TAG2" "$ROOT/tamper-tag"
sed -i "s/\"version_tag\": \"[^\"]*\"/\"version_tag\": \"9.9.9-$SHORT\"/" "$ROOT/tamper-tag/install-manifest.json"
expect_fail "switch refused: manifest version_tag mismatch" TESTENV bash "$SWITCH" tamper-tag
expect_true "tag refusal: current unchanged" current_points_to "$TAG2"
# 不存在的版本
expect_fail "switch refused: nonexistent version" TESTENV bash "$SWITCH" "9.9.9-$SHORT"

# --- 回滚（provenance-verified 双向）---
expect_ok "rollback switch to 0.1.0" TESTENV bash "$SWITCH" "$TAG"
expect_true "current -> 0.1.0 after rollback" current_points_to "$TAG"
expect_ok "switch forward to 0.2.0 again" TESTENV bash "$SWITCH" "$TAG2"

echo
echo "=== SE-01B-FIX failure matrix: PASS=$PASS FAIL=$FAIL (HEAD=$FULL tag=$TAG) ==="
[ "$FAIL" -eq 0 ]
