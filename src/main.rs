#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

//! `rforth` Unix binary entry point
//!
//! The library owns the interpreter and platform-agnostic execution loop. This binary supplies the
//! minimal `no_std` process boundary for Unix: exported runtime symbols, a panic handler, terminal
//! I/O construction, and the call into [`run_forth`].

#[cfg(all(unix, not(test)))]
use rforth::{io::SystemIo, run_forth};

/// Exception-handling personality stub required by the precompiled `libcore`
///
/// Even with `panic = "abort"` in the Cargo profile, the precompiled `core` crate retains a
/// reference to `rust_eh_personality` because it was compiled with unwinding support.  The linker
/// therefore demands the symbol even though it can never be reached at runtime.  We satisfy the
/// linker with this stub; the body aborts immediately to catch any surprise invocation during
/// debugging.
#[cfg(all(unix, not(test)))]
// SAFETY: This intentionally exports the exact symbol name required by the linker.
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    // SAFETY: `abort` has no preconditions and does not return.
    unsafe { libc::abort() }
}

/// Panic handler for `no_std` Unix builds
///
/// Without `std`, the runtime no longer provides a panic handler, so we must supply one.  On Unix
/// the simplest correct behavior is an immediate process abort via `libc::abort`, which terminates
/// the process immediately without flushing stdio buffers or running `atexit` handlers — safer
/// than looping forever or issuing a raw `SIGABRT`.
#[cfg(all(unix, not(test)))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // SAFETY: `abort` has no preconditions and does not return.
    unsafe { libc::abort() }
}

/// Binary entry point
///
/// Declared as `extern "C"` with `#[no_mangle]` so the C runtime (`crt0`/`libc`) can find and call
/// it directly.  With `#![no_std]` the standard library's runtime wrapper around `main` is absent,
/// so we take responsibility for the symbol ourselves.
///
/// Constructs the platform-appropriate [`SystemIo`] instance (which puts the terminal into raw
/// mode) and hands control to [`run_forth`].  The `#[cfg(unix)]` guard here mirrors the
/// compile-time platform selection used throughout the codebase.
#[cfg(all(unix, not(test)))]
// SAFETY: This exports the process entry symbol expected by the C runtime.
#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut io = SystemIo::new();
    run_forth(&mut io)
}

#[cfg(all(not(unix), feature = "embedded"))]
compile_error!("Embedded entry point not yet implemented");

#[cfg(all(not(unix), not(feature = "embedded")))]
compile_error!("Unsupported target");
