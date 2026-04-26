#!/usr/bin/env sh
#
# Purpose:
#   Validate formatting, tests, and clippy for every supported VM I/O feature combination.
#   This keeps the default direct-MMIO profile, UART MMIO profile, direct port-I/O profile, and
#   UART port-I/O profile from drifting apart.
#
# Usage:
#   scripts/check-ci.sh
#
# Notes:
#   The script assumes it is run from the repository root and that the stable Rust toolchain is
#   installed. Set CARGO_BUILD_RUSTC_WRAPPER= when a local rustc wrapper such as sccache should be
#   bypassed. It also runs the stage-zero Forth source self-test file through the rforth binary so
#   the batch stdin path is validated in addition to the Rust test suite.
set -eu

# Keep formatting independent of feature selection; one pass is enough.
cargo fmt

# Default VM profile: direct memory-mapped I/O.
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --quiet < tests/stage0_self_test.fth > /tmp/rforth-stage0-self-test.stdout 2> /tmp/rforth-stage0-self-test.stderr
test "$(cat /tmp/rforth-stage0-self-test.stdout)" = "66 X"
test ! -s /tmp/rforth-stage0-self-test.stderr

# UART over memory-mapped I/O.
cargo test --features vm-uart
cargo clippy --all-targets --features vm-uart -- -D warnings
cargo run --quiet --features vm-uart < tests/stage0_self_test.fth > /tmp/rforth-stage0-self-test.stdout 2> /tmp/rforth-stage0-self-test.stderr
test "$(cat /tmp/rforth-stage0-self-test.stdout)" = "66 X"
test ! -s /tmp/rforth-stage0-self-test.stderr

# Direct port-mapped I/O.
cargo test --features vm-port-io
cargo clippy --all-targets --features vm-port-io -- -D warnings
cargo run --quiet --features vm-port-io < tests/stage0_self_test.fth > /tmp/rforth-stage0-self-test.stdout 2> /tmp/rforth-stage0-self-test.stderr
test "$(cat /tmp/rforth-stage0-self-test.stdout)" = "66 X"
test ! -s /tmp/rforth-stage0-self-test.stderr

# UART registers over port-mapped I/O.
cargo test --features vm-port-io,vm-uart
cargo clippy --all-targets --features vm-port-io,vm-uart -- -D warnings
cargo run --quiet --features vm-port-io,vm-uart < tests/stage0_self_test.fth > /tmp/rforth-stage0-self-test.stdout 2> /tmp/rforth-stage0-self-test.stderr
test "$(cat /tmp/rforth-stage0-self-test.stdout)" = "66 X"
test ! -s /tmp/rforth-stage0-self-test.stderr
