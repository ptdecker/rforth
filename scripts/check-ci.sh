#!/usr/bin/env sh
#
# Purpose:
#   Validate formatting, tests, and clippy for every supported VM I/O feature combination.
#   This keeps the default direct-MMIO profile, UART MMIO profile, direct port-I/O profile, and
#   UART port-I/O profile from drifting apart.
#
# Usage:
#   scripts/check-ci.sh
#   scripts/check-ci.sh --ci-runner
#
# Notes:
#   The script assumes it is run from the repository root and that the stable Rust toolchain is
#   installed. Set CARGO_BUILD_RUSTC_WRAPPER= when a local rustc wrapper such as sccache should be
#   bypassed. It also runs the stage-zero Forth source self-test file through the rforth binary so
#   the batch stdin path is validated in addition to the Rust test suite.
set -eu

CI_MODE=0
for arg in "$@"; do
    if [ "$arg" = "--ci-runner" ]; then
        CI_MODE=1
    fi
done

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT HUP INT TERM
SELF_TEST_STDOUT="$TMPDIR/stage0-self-test.stdout"
SELF_TEST_STDERR="$TMPDIR/stage0-self-test.stderr"

# Keep formatting independent of feature selection; one pass is enough.
if [ "$CI_MODE" = "1" ]; then
    cargo fmt --check
else
    cargo fmt
fi

# Default VM profile: direct memory-mapped I/O.
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --quiet < tests/stage0_self_test.fth > "$SELF_TEST_STDOUT" 2> "$SELF_TEST_STDERR"
test "$(cat "$SELF_TEST_STDOUT")" = "66 X"
test ! -s "$SELF_TEST_STDERR"

# UART over memory-mapped I/O.
cargo test --features vm-uart
cargo clippy --all-targets --features vm-uart -- -D warnings
cargo run --quiet --features vm-uart < tests/stage0_self_test.fth > "$SELF_TEST_STDOUT" 2> "$SELF_TEST_STDERR"
test "$(cat "$SELF_TEST_STDOUT")" = "66 X"
test ! -s "$SELF_TEST_STDERR"

# Direct port-mapped I/O.
cargo test --features vm-port-io
cargo clippy --all-targets --features vm-port-io -- -D warnings
cargo run --quiet --features vm-port-io < tests/stage0_self_test.fth > "$SELF_TEST_STDOUT" 2> "$SELF_TEST_STDERR"
test "$(cat "$SELF_TEST_STDOUT")" = "66 X"
test ! -s "$SELF_TEST_STDERR"

# UART registers over port-mapped I/O.
cargo test --features vm-port-io,vm-uart
cargo clippy --all-targets --features vm-port-io,vm-uart -- -D warnings
cargo run --quiet --features vm-port-io,vm-uart < tests/stage0_self_test.fth > "$SELF_TEST_STDOUT" 2> "$SELF_TEST_STDERR"
test "$(cat "$SELF_TEST_STDOUT")" = "66 X"
test ! -s "$SELF_TEST_STDERR"
