//! Unix implementation of [`ForthIo`] for [`SystemIo`]
//!
//! Delegates character I/O to [`sys::SystemSys`] via the [`SysCalls`] trait. The terminal is
//! already in raw mode by the time these methods are called (raw mode is set up in
//! [`SystemIo::new`]).

use super::*;

use crate::sys::SysCalls;

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
    /// byte is available, without buffering or line editing. Aborts on EOF or any other error.
    fn key(&mut self) -> u8 {
        match self.read_key() {
            InputEvent::Byte(c) => c,
            // SAFETY: `abort` has no preconditions and does not return.
            InputEvent::Eof | InputEvent::Error => unsafe { libc::abort() },
        }
    }

    /// Read the next byte from stdin, distinguishing EOF from I/O failure.
    fn read_key(&mut self) -> InputEvent {
        let mut buf = [0u8; 1];
        // SAFETY: File descriptor 0 is stdin for this process, and `buf` is a valid one-byte
        // output buffer for the duration of the call.
        let n = unsafe { self.sys.sys_read(0, &mut buf) };
        if n == 1 {
            InputEvent::Byte(buf[0])
        } else if n == 0 {
            InputEvent::Eof
        } else {
            InputEvent::Error
        }
    }

    /// Report whether stdin is interactive.
    fn is_interactive(&self) -> bool {
        self.interactive
    }
}
