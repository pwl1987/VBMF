# vendor/

This directory holds vendored Rust dependencies for **offline / reproducible BMD builds**
(Gate 6/7 build supply chain, "方案 B + cargo vendor").

It is **populated by `scripts/vendor.sh`** (run once on a crates.io-reachable host) and then
committed, so the BMD acceptance server can `cargo build --features bmd --offline` without
any access to crates.io (the BMD box must NOT depend on public package mirrors to rebuild
the Media Agent).

Until `cargo vendor` has been run, this directory stays empty and default / CI builds use
crates.io directly (see `.cargo/config.toml`).

Blackmagic DeckLink SDK headers / `libDeckLinkAPI.so` are **intentionally NOT here** — they
are private assets, injected at BMD build time via `/opt/vbmf-sdk/` or a CI secret.
