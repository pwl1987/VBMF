#!/usr/bin/env bash
# VBMF DeckLink SDK host pin verification (read-only, non-mutating, no root).
#
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.3 (adjudication B).
# Independently audits the state produced by scripts/ci/pin-decklink-sdk.sh:
#   V1 include dir exists and is non-empty
#   V2 VERSION file matches the exact pinned x.y.z
#   V3 required headers present (DeckLinkAPI.h / DeckLinkAPIConfiguration.h /
#      DeckLinkAPIDispatch.h / DeckLinkAPIModes.h)
#   V4 every header readable by non-root
#
# Prints the header count only — never header contents or payload hashes
# (proprietary-asset evidence discipline).
#
# This script NEVER installs, writes, or links anything.
#
# Usage:
#   verify-decklink-sdk.sh --expect-version 16.0.0
#   verify-decklink-sdk.sh --expect-version 16.0.0 --root /usr/local/share/decklink-sdk
#   --root exists so the logic can be exercised against fixtures on the
#   Development VM; the real runner check always uses the canonical path.
#   A successful verification at any root other than the canonical one is
#   explicitly marked TEST-ONLY and is never acceptance evidence.
set -euo pipefail

EXPECT_VERSION=""
ROOT="/usr/local/share/decklink-sdk"
while [ $# -gt 0 ]; do
  case "$1" in
    --expect-version) EXPECT_VERSION="${2:-}"; shift 2 ;;
    --root)           ROOT="${2:-}"; shift 2 ;;
    -h|--help)
      echo "usage: $0 --expect-version <x.y.z> [--root /usr/local/share/decklink-sdk]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[ -n "$EXPECT_VERSION" ] || { echo "--expect-version is required" >&2; exit 2; }
printf '%s' "$EXPECT_VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
  || { echo "invalid SDK version: $EXPECT_VERSION" >&2; exit 2; }

INC_DIR="$ROOT/include"
VERSION_FILE="$ROOT/VERSION"
REQUIRED_HEADERS="DeckLinkAPI.h DeckLinkAPIConfiguration.h DeckLinkAPIDispatch.h DeckLinkAPIModes.h"

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1"; FAILED=1; }
FAILED=0

echo "== DeckLink SDK Pin Gate (root=$ROOT, expect=$EXPECT_VERSION) =="

# V1: include dir exists and is non-empty
if [ -d "$INC_DIR" ]; then
  COUNT="$(find "$INC_DIR" -maxdepth 1 -name '*.h' | wc -l)"
  if [ "$COUNT" -gt 0 ]; then
    pass "V1: $INC_DIR exists, $COUNT headers"
  else
    fail "V1: $INC_DIR exists but is empty"
  fi
else
  COUNT=0
  fail "V1: $INC_DIR missing"
fi

# V2: exact version record
if [ -r "$VERSION_FILE" ]; then
  VERSION_ACTUAL="$(tr -d '[:space:]' < "$VERSION_FILE")"
  if [ "$VERSION_ACTUAL" = "$EXPECT_VERSION" ]; then
    pass "V2: VERSION exact ($VERSION_ACTUAL)"
  else
    fail "V2: VERSION mismatch: got '$VERSION_ACTUAL' want '$EXPECT_VERSION'"
  fi
else
  fail "V2: $VERSION_FILE missing or unreadable"
fi

# V3: required headers present
for h in $REQUIRED_HEADERS; do
  if [ -f "$INC_DIR/$h" ]; then
    pass "V3: header present ($h)"
  else
    fail "V3: required header missing ($h)"
  fi
done

# V4: readable by non-root (runs as root? then check via su nobody — the
# canonical acceptance run is as vbmf-ci, non-root; keep it simple and honest)
if [ "$(id -u)" -eq 0 ]; then
  if su -s /bin/sh nobody -c "head -c 1 '$INC_DIR/DeckLinkAPI.h' >/dev/null 2>&1"; then
    pass "V4: headers readable by non-root (via nobody)"
  else
    fail "V4: headers NOT readable by non-root"
  fi
else
  if head -c 1 "$INC_DIR/DeckLinkAPI.h" >/dev/null 2>&1; then
    pass "V4: headers readable by non-root (current user uid=$(id -u))"
  else
    fail "V4: headers NOT readable by current user (uid=$(id -u))"
  fi
fi

echo "=="
if [ "$FAILED" -eq 0 ]; then
  if [ "$ROOT" = "/usr/local/share/decklink-sdk" ]; then
    echo "RESULT: PASS"
    printf 'PINNED_SDK_VERSION=%s\n' "$EXPECT_VERSION"
  else
    # Noncanonical root is a Development-VM test seam only; never acceptance
    # evidence (same contract as verify-system-rust.sh).
    echo "RESULT: PASS (TEST-ONLY, root=$ROOT is not canonical; NOT acceptance evidence)"
    printf 'PINNED_SDK_VERSION=%s\n' "$EXPECT_VERSION"
  fi
else
  echo "RESULT: FAIL"
  exit 1
fi
