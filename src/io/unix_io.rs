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
        unsafe {
            self.sys.sys_write(1, &[c]);
        }
    }

    /// Block on [`SysCalls::sys_read`] from stdin (fd 0) and return the byte read
    ///
    /// Because the terminal is in raw mode (`VMIN=1`, `VTIME=0`), this returns as soon as a single
    /// byte is available, without buffering or line editing.  Retries automatically on `EINTR`;
    /// aborts on EOF or any other error.
    fn key(&mut self) -> u8 {
        let mut buf = [0u8; 1];
        loop {
            let n = unsafe { self.sys.sys_read(0, &mut buf) };
            if n == 1 {
                return buf[0];
            }
            // EINTR: interrupted by signal — retry
            if n == -(libc::EINTR as isize) {
                continue;
            }
            // EOF or other error — abort
            unsafe { libc::abort() }
        }
    }
}
