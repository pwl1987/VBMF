#!/usr/bin/env bash
# Focused non-root tests for the P2-M1 DeckLink SDK pin scripts.
#
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.3 (adjudication B).
# Runs entirely on the Development VM: no root, no /usr/local mutation, no
# network, proprietary payloads faked with plain-text fixtures. Covers:
#   - syntax of pin-decklink-sdk.sh / verify-decklink-sdk.sh
#   - pin-decklink-sdk.sh argument validation (fail-closed paths before the
#     root gate; install logic itself is host-admin-only)
#   - verify-decklink-sdk.sh V1-V4 against fixture trees: happy path, missing
#     dir, empty dir, version drift, missing VERSION, missing header
#   - TEST-ONLY marker for noncanonical roots (same contract as the Rust
#     verifier's fixtures)
#   - collect-toolchain.sh SDK section: present vs absent, yaml + txt
#
# Usage: scripts/ci/test-decklink-sdk-scripts.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
PIN="$HERE/pin-decklink-sdk.sh"
VERIFY="$HERE/verify-decklink-sdk.sh"
COLLECT="$HERE/collect-toolchain.sh"
V="16.0.0"

PASS_N=0
FAIL_N=0
ok()   { printf 'ok %d - %s\n' "$((++PASS_N))" "$1"; }
notok(){ printf 'not ok %d - %s\n' "$((++FAIL_N))" "$1"; }
check() { # check <desc> <expected-status> <cmd...>
  local desc="$1" want="$2" st; shift 2
  "$@" >/dev/null 2>&1 && st=0 || st=$?
  if [ "$st" = "$want" ]; then
    ok "$desc"
  else
    notok "$desc (want exit $want, got $st)"
  fi
}

# --- syntax ---
check "bash -n pin-decklink-sdk.sh" 0 bash -n "$PIN"
check "bash -n verify-decklink-sdk.sh" 0 bash -n "$VERIFY"
check "bash -n self" 0 bash -n "$0"

# --- pin-decklink-sdk.sh argument validation (pre-root, fail-closed) ---
check "pin: no args rejected" 2 "$PIN"
check "pin: missing --tarball rejected" 2 "$PIN" --version "$V"
check "pin: missing --version rejected" 2 "$PIN" --tarball /nonexistent
check "pin: bad version rejected" 2 "$PIN" --tarball /nonexistent --version 16.0
check "pin: unknown flag rejected" 2 "$PIN" --tarball /nonexistent --version "$V" --extra
check "pin: --help exits 0" 0 "$PIN" --help
# Non-root must be refused before any filesystem work (canonical host gate).
if [ "$(id -u)" -ne 0 ]; then
  check "pin: non-root rejected" 1 "$PIN" --tarball /nonexistent --version "$V"
else
  ok "pin: non-root test skipped (running as root)"
fi
# Evidence discipline: the pin script never hashes or dumps the payload.
if grep -qE 'sha256|md5|cat .*include' "$PIN"; then
  notok "pin: payload hash/dump found in script (evidence-discipline violation)"
else
  ok "pin: no payload hash/dump in script"
fi

# --- fixture builder for verify-decklink-sdk.sh ---
FIXTURE=""
cleanup() { [ -n "$FIXTURE" ] && rm -rf "$FIXTURE"; }
trap cleanup EXIT
make_fixture() { # make_fixture <root> [version]
  local root="$1" ver="${2:-$V}"
  mkdir -p "$root/include"
  printf '%s\n' "$ver" > "$root/VERSION"
  for h in DeckLinkAPI.h DeckLinkAPIConfiguration.h DeckLinkAPIDispatch.h DeckLinkAPIModes.h; do
    printf '// fixture header %s\n' "$h" > "$root/include/$h"
  done
}

FIXTURE="$(mktemp -d /tmp/vbmf-sdk-pin-test.XXXXXX)"
GOOD="$FIXTURE/good"
make_fixture "$GOOD"

# --- verify-decklink-sdk.sh: happy path + failure modes ---
check "verify: no args rejected" 2 "$VERIFY"
check "verify: bad version rejected" 2 "$VERIFY" --expect-version latest
check "verify: --help exits 0" 0 "$VERIFY" --help
check "verify: healthy fixture passes" 0 "$VERIFY" --expect-version "$V" --root "$GOOD"

GOOD_OUT="$("$VERIFY" --expect-version "$V" --root "$GOOD" 2>&1)"
if printf '%s\n' "$GOOD_OUT" | grep -q 'RESULT: PASS (TEST-ONLY'; then
  ok "verify: noncanonical-root PASS marked TEST-ONLY"
else
  notok "verify: noncanonical-root PASS not marked TEST-ONLY (got: $(printf '%s\n' "$GOOD_OUT" | grep '^RESULT:'))"
fi
if printf '%s\n' "$GOOD_OUT" | grep -q 'NOT acceptance evidence'; then
  ok "verify: noncanonical-root PASS disclaims acceptance evidence"
else
  notok "verify: noncanonical-root PASS missing acceptance-evidence disclaimer"
fi
if printf '%s\n' "$GOOD_OUT" | grep -q '^PINNED_SDK_VERSION='; then
  ok "verify: prints machine-readable version line"
else
  notok "verify: machine-readable version line missing"
fi

NO_DIR="$FIXTURE/no-dir"
check "verify: missing include dir fails" 1 "$VERIFY" --expect-version "$V" --root "$NO_DIR"

EMPTY_DIR="$FIXTURE/empty-dir"; make_fixture "$EMPTY_DIR"
rm "$EMPTY_DIR"/include/*.h
check "verify: empty include dir fails" 1 "$VERIFY" --expect-version "$V" --root "$EMPTY_DIR"

DRIFT="$FIXTURE/drift"; make_fixture "$DRIFT" 15.9.9
check "verify: version drift fails" 1 "$VERIFY" --expect-version "$V" --root "$DRIFT"

NO_VERSION="$FIXTURE/no-version"; make_fixture "$NO_VERSION"
rm "$NO_VERSION/VERSION"
check "verify: missing VERSION fails" 1 "$VERIFY" --expect-version "$V" --root "$NO_VERSION"

NO_HEADER="$FIXTURE/no-header"; make_fixture "$NO_HEADER"
rm "$NO_HEADER/include/DeckLinkAPIModes.h"
check "verify: missing required header fails" 1 "$VERIFY" --expect-version "$V" --root "$NO_HEADER"

UNREADABLE="$FIXTURE/unreadable"; make_fixture "$UNREADABLE"
chmod 000 "$UNREADABLE/include/DeckLinkAPI.h"
check "verify: unreadable header fails" 1 "$VERIFY" --expect-version "$V" --root "$UNREADABLE"
chmod 644 "$UNREADABLE/include/DeckLinkAPI.h"

# --- collect-toolchain.sh SDK section ---
MBASE="$FIXTURE/manifests"
run_collect() { "$COLLECT" --name test-runner --base "$FIXTURE/collect-base" "$@" >/dev/null 2>&1; }

# Absent case (no SDK at canonical path on the Development VM).
if [ ! -d /usr/local/share/decklink-sdk ]; then
  run_collect
  if grep -q 'version: "absent"' "$FIXTURE/collect-base/manifests/test-runner/toolchain.yaml"; then
    ok "collect: absent SDK recorded in yaml"
  else
    notok "collect: absent SDK not recorded in yaml"
  fi
  if grep -q 'decklink sdk : ABSENT' "$FIXTURE/collect-base/manifests/test-runner/toolchain.txt"; then
    ok "collect: absent SDK recorded in txt"
  else
    notok "collect: absent SDK not recorded in txt"
  fi
else
  ok "collect: absent-SDK test skipped (canonical path exists on this machine)"
fi

# Present case: run with the canonical path faked via a chroot-free seam —
# collect reads the fixed /usr/local path, so the present case is exercised
# only when the SDK actually exists (media host). Structural check instead:
if grep -q 'decklink_sdk:' "$COLLECT" && grep -q 'header_count' "$COLLECT"; then
  ok "collect: SDK section present in script (yaml fields)"
else
  notok "collect: SDK section missing from script"
fi
if grep -q 'ABSENT (general tier)' "$COLLECT"; then
  ok "collect: general-tier absent marker present"
else
  notok "collect: general-tier absent marker missing"
fi
# Evidence discipline: version + count only, never payload hashes.
if grep -qE 'sha256|md5' "$COLLECT"; then
  notok "collect: payload hash found in script"
else
  ok "collect: no payload hash in script"
fi

printf '1..%d\n' "$((PASS_N + FAIL_N))"
if [ "$FAIL_N" -eq 0 ]; then echo "RESULT: PASS ($PASS_N)"; else echo "RESULT: FAIL ($FAIL_N)"; exit 1; fi
