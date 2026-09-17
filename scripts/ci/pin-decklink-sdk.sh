#!/usr/bin/env bash
# Pin the DeckLink SDK headers used by the VBMF media self-hosted runner.
# Contract: docs/architecture/CI_RUNNER_STRATEGY.md §15.3 (adjudication B).
#
# Installs the SDK Linux/include headers to the canonical host path
#   /usr/local/share/decklink-sdk/include
# and records the exact version in
#   /usr/local/share/decklink-sdk/VERSION
# Idempotent, fail-closed, zero network. NEVER prints header contents or any
# hash of the proprietary payload (version + file count only, per the evidence
# discipline in the P2-M1 brief).
#
# Usage (root, on the media runner host):
#   pin-decklink-sdk.sh --tarball <sdk.tar.gz|sdk.tar|extracted-sdk-dir> --version 16.0.0
# The tarball/dir must contain exactly one .../Linux/include/DeckLinkAPI.h.
set -euo pipefail

TARBALL=""
VERSION=""
while [ $# -gt 0 ]; do
  case "$1" in
    --tarball) TARBALL="${2:-}"; shift 2 ;;
    --version) VERSION="${2:-}"; shift 2 ;;
    -h|--help)
      echo "usage: $0 --tarball <path> --version <x.y.z>"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[ -n "$TARBALL" ] || { echo "--tarball is required" >&2; exit 2; }
[ -n "$VERSION" ] || { echo "--version is required" >&2; exit 2; }
printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
  || { echo "invalid SDK version: $VERSION" >&2; exit 2; }
[ "$(id -u)" -eq 0 ] || { echo "must run as root" >&2; exit 1; }
[ -e "$TARBALL" ] || { echo "tarball/dir not found: $TARBALL" >&2; exit 1; }

DEST="/usr/local/share/decklink-sdk"
REQUIRED_HEADERS="DeckLinkAPI.h DeckLinkAPIConfiguration.h DeckLinkAPIDispatch.h DeckLinkAPIModes.h"

STAGE="$(mktemp -d /tmp/vbmf-sdk-pin.XXXXXX)"
trap 'rm -rf "$STAGE"' EXIT

# Locate the include dir: exactly one .../Linux/include/DeckLinkAPI.h.
if [ -d "$TARBALL" ]; then
  INC_DIR="$(cd "$TARBALL" && find . -type f -path '*/Linux/include/DeckLinkAPI.h' | sed 's|^\./||' | head -1)"
  [ -n "$INC_DIR" ] || { echo "no Linux/include/DeckLinkAPI.h under dir: $TARBALL" >&2; exit 1; }
  INC_REL="$(dirname "$INC_DIR")"
  N_HITS="$(cd "$TARBALL" && find . -type f -path '*/Linux/include/DeckLinkAPI.h' | wc -l)"
  [ "$N_HITS" -eq 1 ] || { echo "ambiguous SDK layout: $N_HITS Linux/include/DeckLinkAPI.h found" >&2; exit 1; }
  mkdir -p "$STAGE/$INC_REL"
  cp -a "$TARBALL/$INC_REL/." "$STAGE/$INC_REL/"
else
  N_HITS="$(tar -tf "$TARBALL" | grep -Ec '(^|/)Linux/include/DeckLinkAPI\.h$' || true)"
  [ "$N_HITS" -eq 1 ] \
    || { echo "tarball must contain exactly one Linux/include/DeckLinkAPI.h (found $N_HITS)" >&2; exit 1; }
  INC_REL="$(tar -tf "$TARBALL" | grep -E '(^|/)Linux/include/DeckLinkAPI\.h$' | head -1 | xargs dirname)"
  tar -xf "$TARBALL" -C "$STAGE" "$INC_REL"
fi

# Fail closed BEFORE touching the canonical path: all required headers present.
for h in $REQUIRED_HEADERS; do
  [ -f "$STAGE/$INC_REL/$h" ] || { echo "missing required header in SDK: $h" >&2; exit 1; }
done
COUNT="$(find "$STAGE/$INC_REL" -maxdepth 1 -name '*.h' | wc -l)"

# Atomic swap into the canonical location (unversioned dir; the version lock
# lives in VERSION + verify-decklink-sdk.sh + the manifest, not in the path).
mkdir -p "$DEST"
rm -rf "$DEST/include.next"
mv "$STAGE/$INC_REL" "$DEST/include.next"
rm -rf "$DEST/include"
mv "$DEST/include.next" "$DEST/include"
chmod -R a+rX "$DEST/include"
printf '%s\n' "$VERSION" > "$DEST/VERSION"

echo "DeckLink SDK pinned: version=$VERSION headers=$COUNT path=$DEST/include"
printf 'PINNED_SDK_VERSION=%s\n' "$VERSION"
