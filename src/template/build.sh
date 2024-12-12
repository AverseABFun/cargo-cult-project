#!/usr/bin/env bash
cd "$(dirname "$0")"
cargo generate-lockfile --verbose
cargo vendor --verbose --locked ./crates
mv crates/* .
rm Cargo.lock Cargo.toml rust-toolchain.toml src/main.rs .cargo/config.toml build.sh
rm -d src .cargo