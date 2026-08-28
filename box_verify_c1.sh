#!/bin/bash
set -u
export PATH=$HOME/.cargo/bin:$PATH
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib

rm -rf ~/media-agent-build
mkdir -p ~/media-agent-build
tar -xzf ~/media-agent-src.tar.gz -C ~/media-agent-build
cd ~/media-agent-build

echo "=== default test ==="
cargo test > /tmp/t_def.log 2>&1; echo "DEF_TEST_EXIT=$?"

echo "=== sim test ==="
cargo test --features simulation > /tmp/t_sim.log 2>&1; echo "SIM_TEST_EXIT=$?"

echo "=== default clippy (--all-targets --features <none> -- -D warnings) ==="
cargo clippy --all-targets -- -D warnings > /tmp/cl_def.log 2>&1; echo "DEF_CLIPPY_EXIT=$?"

echo "=== build bmd,gstreamer (compat alias) ==="
cargo build --features bmd,gstreamer > /tmp/b_compat.log 2>&1; echo "COMPAT_BLD_EXIT=$?"

echo "=== clippy bmd,gstreamer (compat) ==="
cargo clippy --all-targets --features bmd,gstreamer -- -D warnings > /tmp/cl_compat.log 2>&1; echo "COMPAT_CLIPPY_EXIT=$?"

echo "=== build bmd-provider,gstreamer-backend (CANONICAL) ==="
cargo build --features bmd-provider,gstreamer-backend > /tmp/b_canon.log 2>&1; echo "CANON_BLD_EXIT=$?"

echo "=== clippy bmd-provider,gstreamer-backend (CANONICAL) ==="
cargo clippy --all-targets --features bmd-provider,gstreamer-backend -- -D warnings > /tmp/cl_canon.log 2>&1; echo "CANON_CLIPPY_EXIT=$?"

echo "ALL_DONE_C1" > /tmp/build_done_c1.log
