#!/bin/bash
set -u
export PATH=$HOME/.cargo/bin:$PATH
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
mkdir -p ~/media-agent-build
tar -xzf ~/media-agent-src.tar.gz -C ~/media-agent-build
cd ~/media-agent-build

run() {
  local label="$1"; shift
  echo "=== $label ==="
  "$@" > "$label.log" 2>&1
  echo "${label}_EXIT=$?"
}

run DEF_CLIPPY      cargo clippy --all-targets -- -D warnings
run MOCK_CLIPPY     cargo clippy --all-targets --features mock -- -D warnings
run GS_ONLY_CLIPPY  cargo clippy --all-targets --features gstreamer-backend -- -D warnings
run CANON_CLIPPY    cargo clippy --all-targets --features bmd-provider,gstreamer-backend -- -D warnings
run CANONMOCK_BLD   cargo build --features bmd-provider,gstreamer-backend,mock
run CANONMOCK_CLIPPY cargo clippy --all-targets --features bmd-provider,gstreamer-backend,mock -- -D warnings

echo "ALL_DONE_RECHECK" > /tmp/build_done_recheck.log
