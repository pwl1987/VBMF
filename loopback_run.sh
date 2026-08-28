#!/bin/bash
export PATH=$HOME/.cargo/bin:$PATH
cd ~/media-agent-build
VBMF_LOOPBACK=1 \
  MEDIA_AGENT_DEVICE_BINDING=$HOME/loopback-manifest-v2.json \
  VBMF_FIXTURES_DIR=$HOME/loopback_fixtures \
  ./target/debug/media-agent
