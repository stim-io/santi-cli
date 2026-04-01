#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
