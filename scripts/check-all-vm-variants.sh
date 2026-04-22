#!/usr/bin/env sh
#
# Purpose:
#   Validate formatting, tests, and clippy for every supported VM I/O feature combination.
#   This keeps the default direct-MMIO profile, UART MMIO profile, direct port-I/O profile, and
#   UART port-I/O profile from drifting apart.
#
# Usage:
#   scripts/check-all-vm-variants.sh
#
# Notes:
#   The script assumes it is run from the repository root and that the stable Rust toolchain is
#   installed. Set CARGO_BUILD_RUSTC_WRAPPER= when a local rustc wrapper such as sccache should be
#   bypassed.
set -eu

# Keep formatting independent of feature selection; one pass is enough.
cargo fmt --check

# Default VM profile: direct memory-mapped I/O.
cargo test
cargo clippy --all-targets -- -D warnings

# UART over memory-mapped I/O.
cargo test --features vm-uart
cargo clippy --all-targets --features vm-uart -- -D warnings

# Direct port-mapped I/O.
cargo test --features vm-port-io
cargo clippy --all-targets --features vm-port-io -- -D warnings

# UART registers over port-mapped I/O.
cargo test --features vm-port-io,vm-uart
cargo clippy --all-targets --features vm-port-io,vm-uart -- -D warnings
