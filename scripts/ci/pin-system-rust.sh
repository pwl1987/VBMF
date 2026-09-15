#!/usr/bin/env bash
# Pin the Rust toolchain used by VBMF general self-hosted runners.
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.5 / §15.7.
set -euo pipefail

VERSION=""
while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="${2:-}"; shift 2 ;;
    -h|--help) echo "usage: $0 --version <x.y.z>"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[ -n "$VERSION" ] || { echo "--version is required" >&2; exit 2; }
printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
  || { echo "invalid Rust version: $VERSION" >&2; exit 2; }
[ "$(id -u)" -eq 0 ] || { echo "must run as root" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "curl is required" >&2; exit 1; }

export RUSTUP_HOME=/usr/local/rustup
export CARGO_HOME=/usr/local/cargo
CURL_OPTS=(--proto '=https' --tlsv1.2 -fL --retry 3 --connect-timeout 15 --max-time 600)
if [ -n "${VBMF_CI_PROXY:-}" ]; then
  CURL_OPTS+=(--proxy "$VBMF_CI_PROXY")
fi
run_provisioning_command() {
  if [ -n "${VBMF_CI_PROXY:-}" ]; then
    env http_proxy="$VBMF_CI_PROXY" https_proxy="$VBMF_CI_PROXY" \
      no_proxy="localhost,127.0.0.1,10.0.0.0/8,169.254.169.254" "$@"
  else
    "$@"
  fi
}

mkdir -p "$RUSTUP_HOME" "$CARGO_HOME"
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  tmp="$(mktemp /tmp/vbmf-rustup-init.XXXXXX)"
  trap 'rm -f "${tmp:-}"' EXIT
  curl "${CURL_OPTS[@]}" https://sh.rustup.rs -o "$tmp"
  run_provisioning_command sh "$tmp" -y --no-modify-path --profile minimal --default-toolchain none
  rm -f "$tmp"
  trap - EXIT
fi

# Idempotent: rustup toolchain install is a no-op when already installed;
# default re-set to the exact version on every run (never rolling stable).
run_provisioning_command "$CARGO_HOME/bin/rustup" toolchain install "$VERSION" --profile minimal --component rustfmt
run_provisioning_command "$CARGO_HOME/bin/rustup" default "$VERSION"

# Fail closed on exact-version semantics BEFORE exposing anything system-wide.
default_toolchain="$("$CARGO_HOME/bin/rustup" default)"
case "$default_toolchain" in
  "$VERSION"-*) ;;
  *) echo "rustup default not exact: got '$default_toolchain', want '$VERSION-<target>'" >&2; exit 1 ;;
esac
for tool in rustup rustc cargo rustfmt; do
  src="$CARGO_HOME/bin/$tool"
  [ -x "$src" ] || { echo "missing installed tool: $src" >&2; exit 1; }
done
toolchain_rustc="$("$CARGO_HOME/bin/rustc" --version)"
case "$toolchain_rustc" in
  "rustc $VERSION "*) ;;
  *) echo "toolchain rustc version mismatch: got '$toolchain_rustc', want '$VERSION'" >&2; exit 1 ;;
esac
toolchain_cargo="$("$CARGO_HOME/bin/cargo" --version)"
case "$toolchain_cargo" in
  "cargo $VERSION "*) ;;
  *) echo "toolchain cargo version mismatch: got '$toolchain_cargo', want '$VERSION'" >&2; exit 1 ;;
esac

for tool in rustup rustc cargo rustfmt; do
  ln -sfn "$CARGO_HOME/bin/$tool" "/usr/local/bin/$tool"
done

# Post-exposure verification of the system-visible toolchain (V1-V4 of
# verify-system-rust.sh; this re-checks through /usr/local/bin itself).
actual="$(/usr/local/bin/rustc --version)"
case "$actual" in
  "rustc $VERSION "*) ;;
  *) echo "rustc version mismatch: got '$actual', want '$VERSION'" >&2; exit 1 ;;
esac
actual="$(/usr/local/bin/cargo --version)"
case "$actual" in
  "cargo $VERSION "*) ;;
  *) echo "cargo version mismatch: got '$actual', want '$VERSION'" >&2; exit 1 ;;
esac
rustfmt_actual="$(/usr/local/bin/rustfmt --version)"
case "$rustfmt_actual" in
  rustfmt*) ;;
  *) echo "rustfmt missing or not executable: got '${rustfmt_actual:-<none>}'" >&2; exit 1 ;;
esac
/usr/local/bin/rustup --version
printf 'PINNED_RUST_VERSION=%s\n' "$VERSION"
