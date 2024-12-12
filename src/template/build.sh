#!/bin/bash
cargo generate-lockfile --verbose
cargo vendor --verbose --locked crates