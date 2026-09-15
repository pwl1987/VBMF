#!/usr/bin/env bash
# VBMF system Rust pin verification (read-only, non-mutating, no root needed).
#
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.7 / §15.8.
# Independently audits the state produced by scripts/ci/pin-system-rust.sh:
#   V1 rustup/rustc/cargo/rustfmt exposed under <root>/bin and symlinked into
#      <root>/cargo/bin (install authority stays <root>/rustup + <root>/cargo)
#   V2 rustup default resolves to the exact pinned x.y.z toolchain
#   V3 rustc --version and cargo --version match the exact pinned version
#   V4 rustfmt present and executable (rustfmt's own version scheme never
#      equals the rustc version; presence + execution is the check)
#
# /usr/local/bin/{rustup,rustc,cargo,rustfmt} are rustup proxies: whichever
# RUSTUP_HOME/CARGO_HOME the caller carries decides which toolchain they
# resolve. Every probe below therefore runs with explicit
# RUSTUP_HOME=<root>/rustup and CARGO_HOME=<root>/cargo so caller/user rustup
# state can never be mistaken for evidence about the audited system pin.
#
# This script NEVER installs, writes, or links anything.
#
# Usage:
#   verify-system-rust.sh --expect-version 1.98.1
#   verify-system-rust.sh --expect-version 1.98.1 --root /usr/local
#   --root exists so the logic can be exercised against fixtures on the
#   Development VM; the real runner check always uses the default /usr/local.
#   A successful verification at any root other than /usr/local is explicitly
#   marked TEST-ONLY and is never acceptance evidence.
set -euo pipefail

EXPECT_VERSION=""
ROOT="/usr/local"
while [ $# -gt 0 ]; do
  case "$1" in
    --expect-version) EXPECT_VERSION="${2:-}"; shift 2 ;;
    --root)           ROOT="${2:-}"; shift 2 ;;
    -h|--help)
      echo "usage: $0 --expect-version <x.y.z> [--root /usr/local]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[ -n "$EXPECT_VERSION" ] || { echo "--expect-version is required" >&2; exit 2; }
printf '%s' "$EXPECT_VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
  || { echo "invalid Rust version: $EXPECT_VERSION" >&2; exit 2; }
[ -d "$ROOT" ] || { echo "root directory not found: $ROOT" >&2; exit 2; }

BIN_DIR="$ROOT/bin"
CARGO_HOME="$ROOT/cargo"
RUSTUP_HOME="$ROOT/rustup"

# Run one command against the audited <root>/rustup + <root>/cargo authority,
# ignoring caller rustup state. RUSTUP_TOOLCHAIN is also cleared: it is a
# caller-supplied override that would make proxies resolve non-audited state.
audit() {
  env -u RUSTUP_TOOLCHAIN RUSTUP_HOME="$RUSTUP_HOME" CARGO_HOME="$CARGO_HOME" "$@"
}

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1"; FAILED=1; }
FAILED=0

echo "== System Rust Pin Gate (root=$ROOT, expect=$EXPECT_VERSION) =="

# V1: exposure + install authority (read-only lstat/readlink, no mutation)
for tool in rustup rustc cargo rustfmt; do
  link="$BIN_DIR/$tool"
  target="$CARGO_HOME/bin/$tool"
  if [ ! -e "$link" ] || [ ! -x "$link" ]; then
    fail "V1: $link missing or not executable"
    continue
  fi
  # Cygwin/WSL edge cases aside, pin-system-rust.sh always creates symlinks;
  # a regular file here means the pin path was bypassed.
  if [ -L "$link" ]; then
    if [ "$(readlink "$link")" = "$target" ]; then
      pass "V1: $link -> $target (symlink into CARGO_HOME)"
    else
      fail "V1: $link -> $(readlink "$link") (want $target)"
    fi
  else
    fail "V1: $link is not a symlink into $CARGO_HOME (pin path bypassed)"
  fi
done

# V2: rustup default resolves to the exact pinned toolchain (read-only query)
RUSTUP_BIN="$CARGO_HOME/bin/rustup"
if [ -x "$RUSTUP_BIN" ]; then
  DEFAULT_TOOLCHAIN="$(audit "$RUSTUP_BIN" default 2>/dev/null || true)"
  case "$DEFAULT_TOOLCHAIN" in
    "$EXPECT_VERSION"-*) pass "V2: rustup default=$DEFAULT_TOOLCHAIN (exact pin, no rolling stable)" ;;
    "") fail "V2: rustup default not set (want $EXPECT_VERSION-<target>)" ;;
    *)  fail "V2: rustup default=$DEFAULT_TOOLCHAIN (want $EXPECT_VERSION-<target>; rolling/other toolchain is not accepted)" ;;
  esac
else
  fail "V2: $RUSTUP_BIN missing; cannot query default toolchain"
fi

# V3: exact version readback from the exposed binaries (rustup proxies:
# must resolve via the audited RUSTUP_HOME/CARGO_HOME, not caller state)
RUSTC_ACTUAL="$(audit "$BIN_DIR/rustc" --version 2>/dev/null || true)"
case "$RUSTC_ACTUAL" in
  "rustc $EXPECT_VERSION "*) pass "V3: rustc version exact ($RUSTC_ACTUAL)" ;;
  *) fail "V3: rustc version mismatch: got '${RUSTC_ACTUAL:-<none>}' want 'rustc $EXPECT_VERSION'" ;;
esac
CARGO_ACTUAL="$(audit "$BIN_DIR/cargo" --version 2>/dev/null || true)"
case "$CARGO_ACTUAL" in
  "cargo $EXPECT_VERSION "*) pass "V3: cargo version exact ($CARGO_ACTUAL)" ;;
  *) fail "V3: cargo version mismatch: got '${CARGO_ACTUAL:-<none>}' want 'cargo $EXPECT_VERSION'" ;;
esac

# V4: rustfmt present and executes (its version scheme is independent of rustc;
# proxy must resolve via the audited RUSTUP_HOME/CARGO_HOME, not caller state)
RUSTFMT_ACTUAL="$(audit "$BIN_DIR/rustfmt" --version 2>/dev/null || true)"
case "$RUSTFMT_ACTUAL" in
  rustfmt*) pass "V4: rustfmt executable ($RUSTFMT_ACTUAL)" ;;
  *) fail "V4: rustfmt missing or not executable: got '${RUSTFMT_ACTUAL:-<none>}'" ;;
esac

echo "=="
if [ "$FAILED" -eq 0 ]; then
  if [ "$ROOT" = "/usr/local" ]; then
    echo "RESULT: PASS"
    printf 'PINNED_RUST_VERSION=%s\n' "$EXPECT_VERSION"
  else
    # Noncanonical root is a Development-VM test seam only. This output is
    # never acceptance evidence for the runner-host pin gate (§15.9).
    echo "RESULT: PASS (TEST-ONLY, root=$ROOT is not /usr/local; NOT acceptance evidence)"
    printf 'PINNED_RUST_VERSION=%s\n' "$EXPECT_VERSION"
  fi
else
  echo "RESULT: FAIL"
  exit 1
fi
