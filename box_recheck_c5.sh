#!/bin/bash
set -u
export PATH=$HOME/.cargo/bin:$PATH
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib

mkdir -p ~/media-agent-build
tar -xzf ~/media-agent-src.tar.gz -C ~/media-agent-build
cd ~/media-agent-build

run() { echo "=== $1 ==="; shift; "$@" > "$1.log" 2>&1; echo "$1_EXIT=$?"; }

run DEF_CLIPPY      cargo clippy --all-targets -- -D warnings
run SIM_CLIPPY      cargo clippy --all-targets --features simulation -- -D warnings
run MOCK_CLIPPY     cargo clippy --all-targets --features mock -- -D warnings
run GS_ONLY_CLIPPY  cargo clippy --all-targets --features gstreamer-backend -- -D warnings
run COMPAT_CLIPPY   cargo clippy --all-targets --features bmd,gstreamer -- -D warnings
run CANON_CLIPPY    cargo clippy --all-targets --features bmd-provider,gstreamer-backend -- -D warnings
run CANONMOCK_CLIPPY cargo clippy --all-targets --features bmd-provider,gstreamer-backend,mock -- -D warnings

echo "ALL_DONE_C5R" > /tmp/build_done_c5r.log
