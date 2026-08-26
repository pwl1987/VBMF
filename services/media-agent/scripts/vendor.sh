#!/usr/bin/env bash
# Offline / reproducible build bootstrap for BMD / air-gapped targets (Gate 6/7).
#
# Build mode is driven entirely by command-line flags, so engineers NEVER hand-edit
# .cargo/config.toml. See the --offline step below.
#
# Step 1 (needs crates.io, run ONCE): populate ./vendor + ./Cargo.lock, then COMMIT
#         both so BMD can build fully offline.
# Step 2 (BMD, offline): build the real FFI using the vendored sources via --config.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo vendor (populates ./vendor, updates ./Cargo.lock)"
cargo vendor --locked vendor

if [[ "${1:-}" == "--offline" ]]; then
  echo "==> offline build (bmd) using vendored sources via --config"
  cargo build --locked --offline \
    --config 'source.crates-io.replace-with="vendored-sources"' \
    --config 'source.vendored-sources.directory="vendor"' \
    --features bmd
fi

echo "==> Commit ./vendor and ./Cargo.lock."
echo "    Then build on BMD offline with:"
echo "      cargo build --locked --offline \\"
echo "        --config 'source.crates-io.replace-with=\"vendored-sources\"' \\"
echo "        --config 'source.vendored-sources.directory=\"vendor\"' \\"
echo "        --features bmd"
