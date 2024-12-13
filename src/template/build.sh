#!/usr/bin/env bash
CI=1
cd "$(dirname "$0")"
cargo generate-lockfile --verbose
cargo vendor --verbose --locked ./crates
mv crates/* .
if [[ "$1" != "save" ]]; then
    rm Cargo.lock Cargo.toml rust-toolchain.toml src/main.rs .cargo/config.toml build.sh
    rm -d src .cargo crates
fi