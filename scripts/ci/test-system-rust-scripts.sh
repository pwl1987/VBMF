#!/usr/bin/env bash
# Focused non-root tests for the P2-C Rust pin scripts.
#
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.7 / §15.8.
# Runs entirely on the Development VM: no root, no /usr/local mutation, no
# network. Covers:
#   - syntax of pin-system-rust.sh / verify-system-rust.sh
#   - pin-system-rust.sh argument validation (the fail-closed paths that run
#     before the root gate; install logic itself is host-admin-only)
#   - verify-system-rust.sh V1-V4 gates against fixture trees, happy path and
#     each failure mode
#   - regression: caller-level/poisoned rustup env (HOME, RUSTUP_HOME,
#     CARGO_HOME, RUSTUP_TOOLCHAIN) cannot skew V2/V3/V4/V5; fixture binaries
#     emulate real rustup proxies that resolve via those env vars
#   - prepare-system-rust-bundle.sh §15.9 step A: exact-SHA export of the
#     three reviewed scripts + SHA256SUMS, happy path and fail-closed modes
#
# Usage: scripts/ci/test-system-rust-scripts.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
PIN="$HERE/pin-system-rust.sh"
VERIFY="$HERE/verify-system-rust.sh"
V="1.98.1"

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
check "bash -n pin-system-rust.sh" 0 bash -n "$PIN"
check "bash -n verify-system-rust.sh" 0 bash -n "$VERIFY"
check "bash -n self" 0 bash -n "$0"

# --- pin-system-rust.sh argument validation (pre-root, fail-closed) ---
check "pin: no args rejected" 2 "$PIN"
check "pin: bad version rejected" 2 "$PIN" --version 1.98
check "pin: rolling stable rejected" 2 "$PIN" --version stable
check "pin: unknown flag rejected" 2 "$PIN" --version "$V" --extra
check "pin: --help exits 0" 0 "$PIN" --help
if grep -q -- '--component clippy' "$PIN"; then
  ok "pin: toolchain install includes clippy component"
else
  notok "pin: clippy component missing from toolchain install"
fi
if grep -q 'cargo-clippy clippy-driver' "$PIN"; then
  ok "pin: exposes cargo-clippy and clippy-driver system-wide"
else
  notok "pin: cargo-clippy/clippy-driver exposure missing"
fi

# --- fixture builder for verify-system-rust.sh ---
FIXTURE=""
cleanup() { [ -n "$FIXTURE" ] && rm -rf "$FIXTURE"; }
trap cleanup EXIT
make_fixture() { # make_fixture <root>
  local root="$1"
  mkdir -p "$root/bin" "$root/cargo/bin" "$root/rustup/toolchains/$V-x86_64-unknown-linux-gnu"
  # Fixture binaries emulate real rustup proxies: they resolve the toolchain
  # via RUSTUP_HOME/CARGO_HOME exactly like the /usr/local/bin shims do. With
  # any other env (e.g. a caller's own rustup) they answer from "caller
  # stable" instead — the false-positive/false-fail modes seen on host1.
  cat > "$root/cargo/bin/rustc" <<EOF
#!/bin/sh
if [ "\$RUSTUP_HOME" = "$root/rustup" ] && [ "\$CARGO_HOME" = "$root/cargo" ]; then
  echo "rustc $V (abcdef123456 2026-01-01)"
else
  echo "rustc 1.97.0 (caller rustup stable)"
fi
EOF
  cat > "$root/cargo/bin/cargo" <<EOF
#!/bin/sh
if [ "\$RUSTUP_HOME" = "$root/rustup" ] && [ "\$CARGO_HOME" = "$root/cargo" ]; then
  echo "cargo $V (abcdef123 2026-01-01)"
else
  echo "cargo 1.97.0 (caller rustup stable)"
fi
EOF
  cat > "$root/cargo/bin/rustfmt" <<EOF
#!/bin/sh
if [ "\$RUSTUP_HOME" = "$root/rustup" ] && [ "\$CARGO_HOME" = "$root/cargo" ]; then
  echo "rustfmt 1.9.0-stable (abcdef12 2026-01-01)"
else
  echo "error: rustfmt not installed in caller toolchain" >&2
  exit 1
fi
EOF
  cat > "$root/cargo/bin/cargo-clippy" <<EOF2
#!/bin/sh
if [ "\$RUSTUP_HOME" = "$root/rustup" ] && [ "\$CARGO_HOME" = "$root/cargo" ]; then
  echo "clippy 0.1.$(printf '%s' "$V" | cut -d. -f2) (abcdef123456 2026-01-01)"
else
  echo "error: clippy not installed in caller toolchain" >&2
  exit 1
fi
EOF2
  cp "$root/cargo/bin/cargo-clippy" "$root/cargo/bin/clippy-driver"
  cat > "$root/cargo/bin/rustup" <<EOF
#!/bin/sh
if [ "\$1" = "default" ] && [ "\$RUSTUP_HOME" = "$root/rustup" ]; then
  cat "\$RUSTUP_HOME/default.txt"
else
  echo "rustup 1.28.1"
fi
EOF
  printf '%s\n' "$V-x86_64-unknown-linux-gnu (default)" > "$root/rustup/default.txt"
  chmod +x "$root/cargo/bin/"*
  for t in rustup rustc cargo rustfmt cargo-clippy clippy-driver; do ln -s "$root/cargo/bin/$t" "$root/bin/$t"; done
}

FIXTURE="$(mktemp -d /tmp/vbmf-rust-pin-test.XXXXXX)"
GOOD="$FIXTURE/good"
make_fixture "$GOOD"

# --- verify-system-rust.sh: happy path + failure modes ---
check "verify: no args rejected" 2 "$VERIFY"
check "verify: bad version rejected" 2 "$VERIFY" --expect-version latest
check "verify: --help exits 0" 0 "$VERIFY" --help
check "verify: healthy fixture passes" 0 "$VERIFY" --expect-version "$V" --root "$GOOD"

# Noncanonical-root success must be visibly TEST-ONLY, not acceptance evidence.
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

BAD_RUSTC="$FIXTURE/bad-rustc";      make_fixture "$BAD_RUSTC"
sed -i "s/rustc $V/rustc 1.97.0/" "$BAD_RUSTC/cargo/bin/rustc"
check "verify: rustc version drift fails" 1 "$VERIFY" --expect-version "$V" --root "$BAD_RUSTC"

BAD_CARGO="$FIXTURE/bad-cargo";      make_fixture "$BAD_CARGO"
sed -i "s/cargo $V/cargo 1.97.0/" "$BAD_CARGO/cargo/bin/cargo"
check "verify: cargo version drift fails" 1 "$VERIFY" --expect-version "$V" --root "$BAD_CARGO"

NO_FMT="$FIXTURE/no-rustfmt";        make_fixture "$NO_FMT"
rm "$NO_FMT/bin/rustfmt" "$NO_FMT/cargo/bin/rustfmt"
check "verify: missing rustfmt fails" 1 "$VERIFY" --expect-version "$V" --root "$NO_FMT"

BAD_CLIPPY="$FIXTURE/bad-clippy";      make_fixture "$BAD_CLIPPY"
CLIPPY_GOOD="clippy 0.1.$(printf '%s' "$V" | cut -d. -f2)"
sed -i "s/$CLIPPY_GOOD/clippy 0.1.97/" "$BAD_CLIPPY/cargo/bin/cargo-clippy"
check "verify: clippy version drift fails" 1 "$VERIFY" --expect-version "$V" --root "$BAD_CLIPPY"

NO_CLIPPY="$FIXTURE/no-clippy";        make_fixture "$NO_CLIPPY"
rm "$NO_CLIPPY/bin/cargo-clippy" "$NO_CLIPPY/cargo/bin/cargo-clippy"
check "verify: missing cargo-clippy fails" 1 "$VERIFY" --expect-version "$V" --root "$NO_CLIPPY"

BAD_LINK="$FIXTURE/bad-link";        make_fixture "$BAD_LINK"
rm "$BAD_LINK/bin/rustc"
cp "$BAD_LINK/cargo/bin/rustc" "$BAD_LINK/bin/rustc"   # regular file, not symlink
check "verify: non-symlink exposure fails" 1 "$VERIFY" --expect-version "$V" --root "$BAD_LINK"

ROLLING="$FIXTURE/rolling";          make_fixture "$ROLLING"
printf '%s\n' "stable-x86_64-unknown-linux-gnu (default)" > "$ROLLING/rustup/default.txt"
check "verify: rolling stable default fails" 1 "$VERIFY" --expect-version "$V" --root "$ROLLING"

NO_DEFAULT="$FIXTURE/no-default";    make_fixture "$NO_DEFAULT"
: > "$NO_DEFAULT/rustup/default.txt"
check "verify: unset default toolchain fails" 1 "$VERIFY" --expect-version "$V" --root "$NO_DEFAULT"

# --- regression: poisoned caller rustup env must not skew V2/V3/V4 ---
# Old verifier ran V2/V3/V4 without explicit RUSTUP_HOME/CARGO_HOME, so the
# /usr/local/bin rustup proxies resolved whatever caller/user rustup state
# was in effect (host1: caller stable rustc/cargo masked as 1.98.1, caller
# rustfmt missing failed V4). Fixture shims reproduce exactly that proxy
# behavior, so the checks below fail against the old verifier.
POISON_HOME="$FIXTURE/poison-home"
POISON_RUSTUP="$POISON_HOME/.rustup"
POISON_CARGO="$POISON_HOME/.cargo"
mkdir -p "$POISON_RUSTUP" "$POISON_CARGO"
printf '%s\n' "stable-x86_64-unknown-linux-gnu (default)" > "$POISON_RUSTUP/default.txt"

# Fixture shims really do answer from caller state when env points elsewhere
# (this is what made the old verifier's evidence untrustworthy).
if env RUSTUP_HOME="$POISON_RUSTUP" CARGO_HOME="$POISON_CARGO" \
     "$GOOD/cargo/bin/rustc" --version 2>/dev/null | grep -q 'rustc 1.97.0'; then
  ok "regression fixture: proxy shim answers from poisoned env (old bug reproduced)"
else
  notok "regression fixture: shim did not emulate caller-rustup resolution"
fi
if ! env RUSTUP_HOME="$POISON_RUSTUP" CARGO_HOME="$POISON_CARGO" \
       "$GOOD/cargo/bin/rustfmt" --version >/dev/null 2>&1; then
  ok "regression fixture: rustfmt proxy fails under poisoned env (old V4 false fail)"
else
  notok "regression fixture: rustfmt shim unexpectedly ran under poisoned env"
fi

# Verifier must still fully PASS under a fully poisoned caller environment.
check "verify: poisoned caller rustup env still passes" 0 \
  env HOME="$POISON_HOME" RUSTUP_HOME="$POISON_RUSTUP" CARGO_HOME="$POISON_CARGO" \
    "$VERIFY" --expect-version "$V" --root "$GOOD"
# RUSTUP_TOOLCHAIN is a caller override of the audited default; must be ignored.
check "verify: poisoned RUSTUP_TOOLCHAIN env still passes" 0 \
  env HOME="$POISON_HOME" RUSTUP_HOME="$POISON_RUSTUP" CARGO_HOME="$POISON_CARGO" \
    RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu" \
    "$VERIFY" --expect-version "$V" --root "$GOOD"

# And drift inside the audited root must still fail under a poisoned caller
# env: the fix must not make the verifier blind to real regressions.
POISON_DRIFT="$FIXTURE/bad-rustc-poisoned"; make_fixture "$POISON_DRIFT"
sed -i "s/rustc $V/rustc 1.97.0/" "$POISON_DRIFT/cargo/bin/rustc"
check "verify: rustc drift still fails under poisoned env" 1 \
  env HOME="$POISON_HOME" RUSTUP_HOME="$POISON_RUSTUP" CARGO_HOME="$POISON_CARGO" \
    "$VERIFY" --expect-version "$V" --root "$POISON_DRIFT"

# --- prepare-system-rust-bundle.sh (§15.9 step A; non-root, zero network) ---
PREPARE="$HERE/prepare-system-rust-bundle.sh"
check "bash -n prepare-system-rust-bundle.sh" 0 bash -n "$PREPARE"

REPO="$(git -C "$HERE" rev-parse --show-toplevel)"
HEAD_SHA="$(git -C "$REPO" rev-parse HEAD)"

# --- prepare-system-rust-bundle.sh: argument / SHA validation (fail-closed) ---
check "prepare: no args rejected" 2 "$PREPARE"
check "prepare: missing --out rejected" 2 "$PREPARE" --sha "$HEAD_SHA"
check "prepare: floating ref HEAD rejected" 2 "$PREPARE" --sha HEAD --out "$FIXTURE/prep-ref"
check "prepare: short SHA rejected" 2 "$PREPARE" --sha "${HEAD_SHA:0:8}" --out "$FIXTURE/prep-short"
check "prepare: unknown SHA rejected" 1 "$PREPARE" --sha 0000000000000000000000000000000000000000 --out "$FIXTURE/prep-ghost"

# A commit that predates scripts/ci must fail on missing script, not silently
# fall back to the working tree.
FIRST_WITH_SCRIPT="$(git -C "$REPO" rev-list --reverse HEAD -- scripts/ci/pin-system-rust.sh | head -1)"
if PRE_SHA="$(git -C "$REPO" rev-parse --verify --quiet "${FIRST_WITH_SCRIPT}^")"; then
  check "prepare: SHA without scripts rejected" 1 "$PREPARE" --sha "$PRE_SHA" --out "$FIXTURE/prep-pre"
else
  ok "prepare: no pre-script commit exists (root has scripts); skipped"
fi

mkdir -p "$FIXTURE/prep-dirty" && : > "$FIXTURE/prep-dirty/leftover"
check "prepare: non-empty output dir rejected" 1 "$PREPARE" --sha "$HEAD_SHA" --out "$FIXTURE/prep-dirty"

# --- prepare-system-rust-bundle.sh: happy path + deterministic evidence ---
B1="$FIXTURE/prep-good1"
if "$PREPARE" --sha "$HEAD_SHA" --out "$B1" >"$FIXTURE/prep1.log" 2>&1; then
  ok "prepare: happy path exits 0"
else
  notok "prepare: happy path exited non-zero"
fi
if grep -q "^BUNDLE_SOURCE_SHA=$HEAD_SHA\$" "$FIXTURE/prep1.log"; then
  ok "prepare: prints exact source SHA"
else
  notok "prepare: source SHA line missing"
fi
if grep -q "^BUNDLE_DIR=" "$FIXTURE/prep1.log"; then
  ok "prepare: prints output path"
else
  notok "prepare: output path line missing"
fi
for s in pin-system-rust.sh verify-system-rust.sh collect-toolchain.sh; do
  if [ -x "$B1/$s" ]; then
    ok "prepare: bundle has executable $s"
  else
    notok "prepare: bundle missing executable $s"
  fi
done
check "prepare: SHA256SUMS verifies" 0 sh -c "cd '$B1' && sha256sum -c SHA256SUMS"

B2="$FIXTURE/prep-good2"
"$PREPARE" --sha "$HEAD_SHA" --out "$B2" >/dev/null 2>&1
if diff -r "$B1" "$B2" >/dev/null 2>&1; then
  ok "prepare: bundle contents deterministic across runs"
else
  notok "prepare: bundle contents differ across runs"
fi

printf '1..%d\n' "$((PASS_N + FAIL_N))"
if [ "$FAIL_N" -eq 0 ]; then echo "RESULT: PASS ($PASS_N)"; else echo "RESULT: FAIL ($FAIL_N)"; exit 1; fi
