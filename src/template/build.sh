#!/usr/bin/env bash
cd "$(dirname "$0")"
cargo generate-lockfile --verbose
cargo vendor --verbose --locked ./crates