# rforth

![CI](https://github.com/ptdecker/rforth/actions/workflows/ci.yml/badge.svg)
![coverage](assets/coverage.svg)

A minimal, portable, Forth language interpreter that is implemented in Rust. The interpreter core is
`no_std` and platform-agnostic; platform selection happens at compile time via `cfg` attributes and
the `embedded` Cargo feature flag.

## Status

Early scaffolding. The interpreter prints `OK` on startup and echoes every keystroke — the I/O
layer and syscall wrappers are in place; the Forth engine itself is not yet implemented.

## Building and running

```bash
cargo build          # build
cargo run            # run the interpreter (Unix only)
cargo test           # run tests
cargo clippy         # lint
scripts/check-all-vm-variants.sh  # format, test, and lint all VM variants
```

## Testing

Tests are host-side Rust tests that exercise the reusable library crate. The tokenizer tests cover
allocation-free word parsing, capacity handling, and `WordVec` behavior. The interpreter tests use a
scripted `ForthIo` implementation to drive the core input loop without touching the terminal or raw
syscalls.

Run the full test suite with:

```bash
cargo test
```

Run the full VM feature matrix with:

```bash
scripts/check-all-vm-variants.sh
```

## Coverage

CI measures test coverage with
[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov), generates `assets/coverage.svg`,
and commits the updated badge on pushes to `main`.

The badge intentionally tracks the testable interpreter core and tokenizer:

- included: `src/lib.rs`, `src/tokenizer.rs`, and tests under `tests/`
- excluded: `src/main.rs`, `src/io/*`, and `src/sys/*`

The excluded files are the runtime entrypoint, terminal I/O, and raw syscall glue. Those paths need
platform or integration tests rather than host-side unit tests, so excluding them keeps the badge
focused on the code currently covered by automated tests.

To measure badge coverage locally, install `cargo-llvm-cov` and run:

```bash
cargo llvm-cov --lib --tests --ignore-filename-regex 'src/(main.rs|io/.*|sys/.*)' --lcov --output-path lcov.info
scripts/generate-coverage-badge.sh lcov.info assets/coverage.svg
```

For an unscoped coverage report across all instrumented source files, omit the ignored regex:

```bash
cargo llvm-cov --lib --tests --lcov --output-path lcov.info
```

## Platform support

| Target                           | Status        | Notes                                     |
|----------------------------------|---------------|-------------------------------------------|
| Unix (default)                   | Working       | Raw-mode terminal I/O via `libc` syscalls |
| Embedded (`--features embedded`) | Stub          | Not yet implemented                       |
| Windows                          | Not Supported | Not Supported                             |

## Architecture

```
lib.rs          — no_std interpreter core and reusable modules
main.rs         — no_std / no_main entry point; constructs SystemIo and calls run_forth()
io/             — ForthIo trait + SystemIo struct; platform impls in unix_io.rs / embedded_io.rs
sys/            — SysCalls trait + raw syscall wrappers; unix_sys.rs / embedded_sys.rs
vm.rs           — flat-memory Forth VM state, stacks, memory access, and I/O dispatch
```

The interpreter core (`run_forth`) is platform-agnostic and communicates with the outside world
exclusively through the `ForthIo` trait (`emit` / `key`). The `io` layer depends on the `sys`
layer for the actual syscalls, keeping the two concerns separate.

`SystemIo` puts stdin into raw mode (no canonical processing, no echo, `VMIN=1 VTIME=0`) on
construction and restores the original terminal settings automatically when it is dropped.

The VM uses typed aliases for `MemoryWord`, `Cell`, and `Address`; `MEMORY_SIZE` is derived from
the address type. All VM layout and I/O values are named constants. The default VM profile is
direct memory-mapped I/O; `vm-uart`, `vm-port-io`, and their combination selects the alternate UART
and port-mapped models.

## Wiki

A [wiki](https://github.com/joshua-maros/rforth/wiki) is available containing supporting
documentation.

## Dependencies

Intentionally near-zero. The [`libc`](https://crates.io/crates/libc) is the only external crate,
restricted to `[target.'cfg(unix)'.dependencies]`.

## License

This project is licensed under the [Unlicense](https://unlicense.org). See the [LICENSE](LICENSE)
file for details.
