# rforth

[![codecov](https://codecov.io/gh/ptdecker/rforth/branch/main/graph/badge.svg)](https://codecov.io/gh/ptdecker/rforth)

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
```

## Coverage

CI measures test coverage with
[`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) and uploads the LCOV report to
Codecov using GitHub Actions OIDC authentication, which updates the badge above for the `main`
branch.

To measure coverage locally, install `cargo-llvm-cov` and run:

```bash
cargo llvm-cov --workspace --lcov --output-path lcov.info
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
```

The interpreter core (`run_forth`) is platform-agnostic and communicates with the outside world
exclusively through the `ForthIo` trait (`emit` / `key`). The `io` layer depends on the `sys`
layer for the actual syscalls, keeping the two concerns separate.

`SystemIo` puts stdin into raw mode (no canonical processing, no echo, `VMIN=1 VTIME=0`) on
construction and restores the original terminal settings automatically when it is dropped.

## Wiki

A [wiki](https://github.com/joshua-maros/rforth/wiki) is available containing supporting
documentation.

## Dependencies

Intentionally near-zero. The [`libc`](https://crates.io/crates/libc) is the only external crate,
restricted to `[target.'cfg(unix)'.dependencies]`.

## License

This project is licensed under the [Unlicense](https://unlicense.org). See the [LICENSE](LICENSE)
file for details.
