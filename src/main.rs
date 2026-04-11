#![no_std]
#![no_main]

//! `rforth` — a minimal, dependency-free Forth interpreter.
//!
//! The interpreter core ([`run_forth`]) is platform-agnostic and communicates with the outside
//! world exclusively through the [`ForthIo`] trait.  Platform selection happens at compile time via
//! `cfg` attributes and the `embedded` Cargo feature flag; see the `io` and `sys` modules for the
//! concrete implementations.

mod io;
mod sys;

use io::ForthIo;
#[cfg(unix)]
use io::SystemIo;

/// Run the Forth interpreter, using `io` for all character-level I/O
///
/// This function is platform-agnostic: it never touches file descriptors, terminal state, or any OS
/// primitive directly.  All such concerns are encapsulated in the [`ForthIo`] implementation that
/// is passed in.
///
/// Currently, the interpreter prints `OK\n` on startup and then echoes every keystroke back to the
/// terminal.  This is the initial scaffolding; a full Forth engine will be built on top of this
/// loop.
fn run_forth(io: &mut impl ForthIo) {
    io.emit(b'O');
    io.emit(b'K');
    io.emit(b'\n');
    loop {
        let c = io.key();
        io.emit(c);
    }
}

/// Exception-handling personality stub required by the precompiled `libcore`
///
/// Even with `panic = "abort"` in the Cargo profile, the precompiled `core` crate retains a
/// reference to `rust_eh_personality` because it was compiled with unwinding support.  The linker
/// therefore demands the symbol even though it can never be reached at runtime.  We satisfy the
/// linker with this stub; the body aborts immediately to catch any surprise invocation during
/// debugging.
#[cfg(unix)]
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    unsafe { libc::abort() }
}

/// Panic handler for `no_std` Unix builds
///
/// Without `std`, the runtime no longer provides a panic handler, so we must supply one.  On Unix
/// the simplest correct behavior is an immediate process abort via `libc::abort`, which terminates
/// the process immediately without flushing stdio buffers or running `atexit` handlers — safer
/// than looping forever or issuing a raw `SIGABRT`.
#[cfg(unix)]
#[panic_handler]
// RustRover's linter shows a false positive "Found duplicate lang item `panic_impl`"
//TODO: Investigate more deeply to see if this is a false positive can be resolved
fn panic(_info: &core::panic::PanicInfo) -> ! {
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
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    run_forth(&mut SystemIo::new());
    0
}

#[cfg(all(not(unix), feature = "embedded"))]
compile_error!("Embedded entry point not yet implemented");

#[cfg(all(not(unix), not(feature = "embedded")))]
compile_error!("Unsupported target");
