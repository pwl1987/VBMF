#!/usr/bin/env bash
# Prepare the P2-C host-admin Rust-pin bundle from a coordinator-confirmed
# exact commit SHA (Development VM / control machine only).
#
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.9 (step A).
# Exports exactly three reviewed scripts from the given commit:
#   pin-system-rust.sh / verify-system-rust.sh / collect-toolchain.sh
# plus a deterministic SHA256SUMS. Zero network: resolving the SHA against the
# local repository is the caller's responsibility (fetch stays out-of-band,
# coordinator-confirmed; this script never fetches and never floats to a ref).
# No sudo, no host/runner mutation, no state transition.
#
# Usage: prepare-system-rust-bundle.sh --sha <40-hex-commit> --out <dir> [--repo <path>]
set -euo pipefail

SCRIPTS=(pin-system-rust.sh verify-system-rust.sh collect-toolchain.sh)

SHA=""
OUT=""
REPO=""
while [ $# -gt 0 ]; do
  case "$1" in
    --sha)  SHA="${2:-}";  shift 2 ;;
    --out)  OUT="${2:-}";  shift 2 ;;
    --repo) REPO="${2:-}"; shift 2 ;;
    -h|--help)
      echo "usage: $0 --sha <40-hex-commit> --out <dir> [--repo <path>]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[ -n "$SHA" ] || { echo "--sha is required (coordinator-confirmed exact commit SHA; never HEAD/main/a floating ref)" >&2; exit 2; }
[ -n "$OUT" ] || { echo "--out is required (bundle output directory)" >&2; exit 2; }
# Exact-SHA discipline is structural: anything that is not a full 40-hex
# object id (refs, prefixes, HEAD) is rejected before git ever sees it.
printf '%s' "$SHA" | grep -Eq '^[0-9a-f]{40}$' \
  || { echo "invalid commit SHA (want full 40-hex, got: $SHA)" >&2; exit 2; }

HERE="$(cd "$(dirname "$0")" && pwd)"
[ -n "$REPO" ] || REPO="$(git -C "$HERE" rev-parse --show-toplevel)"

# The SHA must resolve to a commit object in this repository.
git -C "$REPO" rev-parse --verify --quiet "$SHA^{commit}" >/dev/null \
  || { echo "SHA does not resolve to a commit in $REPO: $SHA (fetch/verify out-of-band first)" >&2; exit 1; }

# Fail closed on a pre-existing non-empty destination: never mix bundle
# contents with anything the caller left behind.
if [ -e "$OUT" ]; then
  [ -d "$OUT" ] || { echo "output path exists and is not a directory: $OUT" >&2; exit 1; }
  [ -z "$(ls -A "$OUT")" ] || { echo "output directory exists and is not empty: $OUT" >&2; exit 1; }
else
  mkdir -p "$OUT" || { echo "cannot create output directory: $OUT" >&2; exit 1; }
fi

# Export only the three reviewed scripts, byte-exact from the given commit.
for s in "${SCRIPTS[@]}"; do
  git -C "$REPO" cat-file -e "$SHA:scripts/ci/$s" 2>/dev/null \
    || { echo "script missing at SHA $SHA: scripts/ci/$s" >&2; exit 1; }
  git -C "$REPO" show "$SHA:scripts/ci/$s" > "$OUT/$s"
  chmod 755 "$OUT/$s"
done

# Deterministic checksum manifest (sorted, ./-prefixed — matches the §15.9
# host-side `sha256sum -c SHA256SUMS` gate).
( cd "$OUT" && printf '%s\n' "${SCRIPTS[@]}" | LC_ALL=C sort | sed 's|^|./|' | xargs sha256sum > SHA256SUMS ) \
  || { echo "checksum generation failed" >&2; exit 1; }
[ -s "$OUT/SHA256SUMS" ] || { echo "checksum generation produced empty SHA256SUMS" >&2; exit 1; }

# Evidence: exact source SHA and output path.
printf 'BUNDLE_SOURCE_SHA=%s\n' "$SHA"
printf 'BUNDLE_DIR=%s\n' "$(cd "$OUT" && pwd)"
