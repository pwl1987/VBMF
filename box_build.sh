#!/bin/bash
set -u
export PATH=$HOME/.cargo/bin:$PATH
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
cd ~/media-agent-build

echo "=== default test ==="
cargo test > /tmp/t_def.log 2>&1
echo "DEF_EXIT=$?" >> /tmp/t_def.log

echo "=== sim test ==="
cargo test --features simulation > /tmp/t_sim.log 2>&1
echo "SIM_EXIT=$?" >> /tmp/t_sim.log

echo "=== default clippy ==="
cargo clippy -D warnings > /tmp/cl_def.log 2>&1
echo "CLDEF_EXIT=$?" >> /tmp/cl_def.log

echo "=== gstreamer build (bmd,gstreamer) ==="
cargo build --features bmd,gstreamer > /tmp/b_gst.log 2>&1
echo "BLD_EXIT=$?" >> /tmp/b_gst.log

echo "=== gstreamer clippy (bmd,gstreamer) ==="
cargo clippy -D warnings --features bmd,gstreamer > /tmp/cl_gst.log 2>&1
echo "CLGST_EXIT=$?" >> /tmp/cl_gst.log

echo "ALL_DONE" > /tmp/build_done.log
