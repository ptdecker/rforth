# Changelog

This changelog was reconstructed from the Git history. Minor versions track functional additions to
the interpreter; test- and CI-only changes are recorded as maintenance notes without consuming a
minor version.

## 0.8.0 - 2026-04-24

- Added the stage-zero `BYE` word so the outer interpreter can terminate cleanly instead of only
  resetting to the top-level prompt.
- Kept `QUIT` as the classic outer-interpreter reset word and clarified the distinction between
  `QUIT` and `BYE` in tests and runner behavior.
- Added runner and VM tests covering `BYE` process-exit behavior.

## 0.7.0 - 2026-04-24

- Connected the top-level runner to the stage-zero dictionary, so parsed input tokens are now looked
  up and executed instead of being echoed back as a token list.
- Added line-level outer-interpreter behavior for successful execution, unknown-word reporting, and
  `QUIT` abandoning the rest of the current input line.
- Added runner integration tests covering unknown-word reporting, `QUIT` line handling, and
  runner-driven `KEY` / `EMIT` execution.

## 0.6.0 - 2026-04-24

- Added the classic irreducible stage-zero word set: `NEXT`, `DOCOL`, `DOSEMI`, `LIT`,
  `BRANCH`, `0BRANCH`, `QUIT`, `DUP`, `DROP`, `SWAP`, `@`, `!`, `C@`, `C!`, `KEY`, and `EMIT`.
- Added a `words/` module with grouped submodules for inner-interpreter, control, stack, memory,
  and input/output primitives.
- Added explicit virtual machine `W` work-register and `P` compile-pointer state, plus
  dictionary-entry helpers and threaded word execution.
- Added tests covering stage-zero word installation, colon-definition execution, stack words,
  memory words, input/output words, and `QUIT`.

## 0.5.0 - 2026-04-22

- Added a typed flat-memory Forth VM with `MemoryWord`, `Cell`, `Address`, and address-derived
  `MEMORY_SIZE`.
- Added VM-resident dictionary pointer, terminal input buffer metadata, opposing data/return stacks,
  checked stack operations, checked memory access, and checked dictionary allocation.
- Added direct MMIO, UART MMIO, direct port I/O, and UART port I/O feature variants.
- Added VM behavior tests and an all-variant formatting/test/clippy script used by CI.
- Preserved the existing `run_forth`/`run_forth_steps` runner behavior while instantiating the VM
  behind the runner.

## 0.4.0 - 2026-04-19

- Refactored the interpreter into a reusable `no_std` library crate plus a small Unix binary entry
  point.
- Added host-side tests for tokenizer behavior against the reusable library.
- Kept platform syscall and I/O boundaries separate from interpreter-core code.

## 0.3.0 - 2026-04-19

- Added allocation-free token parsing for ASCII whitespace-separated Forth input.
- Added fixed-capacity token storage for parsed word slices.

## 0.2.0 - 2026-04-11

- Added platform-agnostic character I/O through the `ForthIo` trait.
- Added Unix raw terminal I/O backed by syscall wrappers.
- Added embedded syscall and I/O stubs behind the `embedded` feature.
- Added the initial interactive scaffold that prints `OK`, echoes input, and processes completed
  lines.

## 0.1.0 - 2026-04-11

- Added the initial Cargo package, no-std binary scaffold, README, and repository metadata.

## Maintenance - 2026-04-19

- Added coverage monitoring and coverage badge generation.
- Increased test coverage for the existing tokenizer and runner scaffolding.
