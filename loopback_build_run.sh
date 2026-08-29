#!/bin/bash
set -u
export PATH=$HOME/.cargo/bin:$PATH
export DECKLINK_SDK_INCLUDE=/home/lytv/decklink-sdk-include
export LIBCLANG_PATH=/usr/lib/llvm-21/lib
cd ~/media-agent-build
tar -xzf ~/media-agent-src.tar.gz -C ~/media-agent-build
cargo build --features bmd,gstreamer > /tmp/b_gst3.log 2>&1
echo "BLD_EXIT=$?" >> /tmp/b_gst3.log
echo "=== loopback run (v2 manifest) ==="
# 释放可能占用 DeckLink 设备的残留实例, 避免 probe/render 争用
pkill -x media-agent 2>/dev/null || true
sleep 1
VBMF_LOOPBACK=1 \
  MEDIA_AGENT_DEVICE_BINDING=$HOME/loopback-manifest-v2.json \
  VBMF_FIXTURES_DIR=$HOME/loopback_fixtures \
  ./target/debug/media-agent 2>&1 | tee /tmp/loopback_run.log
