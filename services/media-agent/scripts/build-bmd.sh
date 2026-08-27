#!/usr/bin/env bash
# BMD 真机 (10.30.15.10, lytv) 本地编译脚本 —— 复用已装工具链/GStreamer dev/libclang, 不依赖 CI.
#
# 用法:
#   ./scripts/build-bmd.sh                 # 默认 --features bmd,gstreamer (debug)
#   ./scripts/build-bmd.sh --release       # release 构建
#   MEDIA_AGENT_SELFTEST=1 ./scripts/build-bmd.sh   # 构建后由调用方以 selftest 运行
#
# env (有默认值, 可被外部环境变量覆盖):
#   DECKLINK_SDK_INCLUDE  DeckLink SDK Linux/include 路径 (已软链去空格)
#   LIBCLANG_PATH         libclang 所在目录 (bindgen 需要)
set -euo pipefail

cd "$(dirname "$0")/.."

DECKLINK_SDK_INCLUDE="${DECKLINK_SDK_INCLUDE:-/home/lytv/decklink-sdk-include}"
LIBCLANG_PATH="${LIBCLANG_PATH:-/usr/lib/llvm-21/lib}"

if [ ! -f "${DECKLINK_SDK_INCLUDE}/DeckLinkAPI.h" ]; then
  echo "ERROR: DECKLINK_SDK_INCLUDE 指向无 DeckLinkAPI.h: ${DECKLINK_SDK_INCLUDE}" >&2
  echo "       请先软链 SDK include 到无空格路径, 见 docs/phase-0.6/BMD-LOCAL-BUILD.md §4" >&2
  exit 1
fi

export DECKLINK_SDK_INCLUDE
export LIBCLANG_PATH

echo "== BMD local build: features=bmd,gstreamer =="
echo "   DECKLINK_SDK_INCLUDE=${DECKLINK_SDK_INCLUDE}"
echo "   LIBCLANG_PATH=${LIBCLANG_PATH}"
cargo build --features bmd,gstreamer "$@"
