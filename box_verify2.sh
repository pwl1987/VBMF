#!/bin/bash
set -u
export PATH=$HOME/.cargo/bin:$PATH
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
cd ~/media-agent-build

echo "=== default clippy ==="
cargo clippy --all-targets -- -D warnings > /tmp/cl_def2.log 2>&1
echo "CLDEF_EXIT=$?" >> /tmp/cl_def2.log

echo "=== gstreamer build (bmd,gstreamer) ==="
cargo build --features bmd,gstreamer > /tmp/b_gst2.log 2>&1
echo "BLD_EXIT=$?" >> /tmp/b_gst2.log

echo "=== gstreamer clippy (bmd,gstreamer) ==="
cargo clippy --all-targets -- -D warnings --features bmd,gstreamer > /tmp/cl_gst2.log 2>&1
echo "CLGST_EXIT=$?" >> /tmp/cl_gst2.log

echo "ALL_DONE2" > /tmp/build_done2.log
