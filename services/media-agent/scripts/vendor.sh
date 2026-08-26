#!/usr/bin/env bash
# Bootstrap offline / reproducible build artifacts (Gate 6/7 build supply chain).
#
# Run ONCE on a host with crates.io reachability, then COMMIT both outputs:
#   - ./vendor        (vendored crate sources)
#   - ./Cargo.lock    (pinned, reproducible resolution)
#
# After that, BMD can build fully offline:
#   cargo build --features bmd --offline \
#     --config 'source.crates-io.replace-with="vendored-sources"'
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo vendor (populates ./vendor, updates ./Cargo.lock)"
cargo vendor --locked

echo "==> done. Commit ./vendor and ./Cargo.lock."
echo "    Then build on BMD offline with --features bmd and the --config replace-with flag."
