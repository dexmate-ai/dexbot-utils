#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo 'Install coverage tooling: cargo install cargo-llvm-cov --version 0.8.5 --locked' >&2
    echo 'Install LLVM tools: rustup component add llvm-tools-preview' >&2
    exit 1
fi

cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s tools/tests
cargo llvm-cov --workspace --locked --fail-under-lines 90 --summary-only
