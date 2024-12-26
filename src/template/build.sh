#!/usr/bin/env bash
# shellcheck shell=bash
cd "$(dirname "$0")" || exit
cargo generate-lockfile --verbose
"$CARGO_BIN_FILE_CARGO_LOCAL_REGISTRY" local-registry --sync Cargo.lock crates >/dev/null 2>&1
# ^^^ is super jank, but it expects that the first argument is local-registry and acts strangely otherwise

mv crates/* .
if [[ "$1" != "save" ]]; then
    rm Cargo.lock Cargo.toml rust-toolchain.toml src/main.rs .cargo/config.toml build.sh
    rm -d src .cargo crates
fi
