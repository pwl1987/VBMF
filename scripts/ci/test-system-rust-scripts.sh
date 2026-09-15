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

# --- fixture builder for verify-system-rust.sh ---
FIXTURE=""
cleanup() { [ -n "$FIXTURE" ] && rm -rf "$FIXTURE"; }
trap cleanup EXIT
make_fixture() { # make_fixture <root>
  local root="$1"
  mkdir -p "$root/bin" "$root/cargo/bin" "$root/rustup/toolchains/$V-x86_64-unknown-linux-gnu"
  cat > "$root/cargo/bin/rustc" <<EOF
#!/bin/sh
echo "rustc $V (abcdef123456 2026-01-01)"
EOF
  cat > "$root/cargo/bin/cargo" <<EOF
#!/bin/sh
echo "cargo $V (abcdef123 2026-01-01)"
EOF
  cat > "$root/cargo/bin/rustfmt" <<EOF
#!/bin/sh
echo "rustfmt 1.9.0-stable (abcdef12 2026-01-01)"
EOF
  cat > "$root/cargo/bin/rustup" <<EOF
#!/bin/sh
[ "\$1" = "default" ] && cat "\$RUSTUP_HOME/default.txt" || echo "rustup 1.28.1"
EOF
  printf '%s\n' "$V-x86_64-unknown-linux-gnu (default)" > "$root/rustup/default.txt"
  chmod +x "$root/cargo/bin/"*
  for t in rustup rustc cargo rustfmt; do ln -s "$root/cargo/bin/$t" "$root/bin/$t"; done
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
