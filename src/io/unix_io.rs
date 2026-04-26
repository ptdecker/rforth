//! Unix implementation of [`ForthIo`] for [`SystemIo`]
//!
//! Delegates character I/O to [`sys::SystemSys`] via the [`SysCalls`] trait. The terminal is
//! already in raw mode by the time these methods are called (raw mode is set up in
//! [`SystemIo::new`]).

use super::*;

use crate::sys::SysCalls;

/// Maximum number of retry attempts after an interrupted stdin read.
const EINTR_READ_RETRY_LIMIT: usize = 8;

/// Delay between interrupted stdin read retry attempts, in microseconds.
const EINTR_READ_RETRY_DELAY_MICROS: libc::useconds_t = 1_000;

/// Byte returned by the legacy blocking `key` API when no input byte is available.
const KEY_ERROR_SENTINEL: u8 = 0x00;

impl ForthIo for SystemIo {
    /// Write a character `c` to stdout (fd 1) via [`SysCalls::sys_write`]
    fn emit(&mut self, c: u8) {
        // SAFETY: File descriptor 1 is stdout for this process, and `[c]` is a valid one-byte
        // buffer for the duration of the call.
        unsafe {
            self.sys.sys_write(1, &[c]);
        }
    }

    /// Write a character `c` to stderr (fd 2) via [`SysCalls::sys_write`].
    fn emit_error(&mut self, c: u8) {
        // SAFETY: File descriptor 2 is stderr for this process, and `[c]` is a valid one-byte
        // buffer for the duration of the call.
        unsafe {
            self.sys.sys_write(2, &[c]);
        }
    }

    /// Block on [`SysCalls::sys_read`] from stdin (fd 0) and return the byte read
    ///
    /// Because the terminal is in raw mode (`VMIN=1`, `VTIME=0`), this returns as soon as a single
    /// byte is available, without buffering or line editing. EOF and I/O errors return a NUL
    /// sentinel; VM and runner code should prefer [`ForthIo::read_key`] when it needs structured
    /// end-of-input handling.
    fn key(&mut self) -> u8 {
        match self.read_key() {
            InputEvent::Byte(c) => c,
            InputEvent::Eof | InputEvent::Error => KEY_ERROR_SENTINEL,
        }
    }

    /// Read the next byte from stdin, distinguishing EOF from I/O failure.
    fn read_key(&mut self) -> InputEvent {
        let mut buf = [0u8; 1];
        let mut interrupted_reads = 0usize;

        loop {
            // SAFETY: File descriptor 0 is stdin for this process, and `buf` is a valid one-byte
            // output buffer for the duration of the call.
            let n = unsafe { self.sys.sys_read(0, &mut buf) };
            if n == 1 {
                return InputEvent::Byte(buf[0]);
            }
            if n == 0 {
                return InputEvent::Eof;
            }
            if current_errno() != libc::EINTR {
                return InputEvent::Error;
            }
            if interrupted_reads == EINTR_READ_RETRY_LIMIT {
                return InputEvent::Error;
            }
            interrupted_reads += 1;
            sleep_after_interrupted_read();
        }
    }

    /// Report whether stdin is interactive.
    fn is_interactive(&self) -> bool {
        self.interactive
    }
}

/// Return the thread-local Unix errno value for the current target.
#[cfg(any(target_os = "macos", target_os = "freebsd"))]
fn current_errno() -> libc::c_int {
    // SAFETY: `__error` returns a valid pointer to the calling thread's errno storage on these
    // targets, and reading the pointed-to integer does not mutate it.
    unsafe { *libc::__error() }
}

/// Return the thread-local Unix errno value for the current target.
#[cfg(any(
    target_os = "linux",
    target_os = "emscripten",
    target_os = "hurd",
    target_os = "redox",
    target_os = "dragonfly"
))]
fn current_errno() -> libc::c_int {
    // SAFETY: `__errno_location` returns a valid pointer to the calling thread's errno storage on
    // these targets, and reading the pointed-to integer does not mutate it.
    unsafe { *libc::__errno_location() }
}

/// Return the thread-local Unix errno value for the current target.
#[cfg(any(
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "android",
    target_os = "cygwin"
))]
fn current_errno() -> libc::c_int {
    // SAFETY: `__errno` returns a valid pointer to the calling thread's errno storage on these
    // targets, and reading the pointed-to integer does not mutate it.
    unsafe { *libc::__errno() }
}

/// Return the thread-local Unix errno value for the current target.
#[cfg(any(target_os = "solaris", target_os = "illumos"))]
fn current_errno() -> libc::c_int {
    // SAFETY: `___errno` returns a valid pointer to the calling thread's errno storage on these
    // targets, and reading the pointed-to integer does not mutate it.
    unsafe { *libc::___errno() }
}

/// Return the thread-local Unix errno value for the current target.
#[cfg(target_os = "haiku")]
fn current_errno() -> libc::c_int {
    // SAFETY: `_errnop` returns a valid pointer to the calling thread's errno storage on this
    // target, and reading the pointed-to integer does not mutate it.
    unsafe { *libc::_errnop() }
}

/// Sleep briefly before retrying a read interrupted by a signal.
fn sleep_after_interrupted_read() {
    // SAFETY: `usleep` has no pointer or aliasing preconditions. The delay is a small constant
    // expressed in microseconds and intentionally ignores interruption of the sleep itself.
    unsafe {
        libc::usleep(EINTR_READ_RETRY_DELAY_MICROS);
    }
}
