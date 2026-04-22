//! Platform-agnostic I/O layer for rforth
//!
//! This module defines the [`ForthIo`] trait — the only I/O interface the interpreter core ever
//! sees — and the [`SystemIo`] struct that provides a concrete implementation for the current
//! compiler target.
//!
//! Platform implementations live in the submodules:
//! - [`unix_io`] — raw-mode terminal I/O via `libc` syscalls (Unix)
//! - `embedded_io` — stub implementation for bare-metal targets
//!
//! The correct submodule is selected at compile time; attempting to build for an unsupported target
//! combination is a hard compiler error.

use crate::sys;

#[cfg(unix)]
pub mod unix_io;

#[cfg(all(not(unix), feature = "embedded"))]
pub mod embedded_io;

#[cfg(all(not(unix), not(feature = "embedded")))]
compile_error!("Unsupported target");

/// Platform-agnostic character I/O required by the Forth interpreter
///
/// Implementors are responsible for all platform-specific details (terminal mode, peripheral
/// drivers, etc.).  The interpreter core only ever calls [`emit`](ForthIo::emit) and
/// [`key`](ForthIo::key), so porting rforth to a new target means providing a new implementation
/// of this trait.
pub trait ForthIo {
    /// Write a single byte to the output channel
    ///
    /// On Unix this writes to stdout (fd 1).  On embedded targets this would typically write to a
    /// UART or similar peripheral.
    fn emit(&mut self, c: u8);

    /// Block until one byte is available from the input channel and return it
    ///
    /// On Unix this reads from stdin (fd 0) with the terminal in raw mode, so each keystroke is
    /// returned immediately without waiting for a newline.
    fn key(&mut self) -> u8;
}

/// Forward [`ForthIo`] calls through mutable references.
///
/// This lets higher-level structures, such as the VM, own a borrowed I/O object while still being
/// generic over `I: ForthIo`.
impl<T: ForthIo + ?Sized> ForthIo for &mut T {
    fn emit(&mut self, c: u8) {
        (**self).emit(c);
    }

    fn key(&mut self) -> u8 {
        (**self).key()
    }
}

/// Concrete I/O implementation for the current compiler target
///
/// On Unix, `SystemIo::new` switches the controlling terminal to raw mode so that keystrokes are
/// delivered one character at a time.  The original terminal settings are stored and restored
/// automatically when the `SystemIo` is dropped.
///
/// Character-level reads and writes are delegated to `sys` via the [`sys::SysCalls`] trait,
/// keeping all platform syscall logic inside the `sys` module.
#[cfg(unix)]
pub struct SystemIo {
    /// Saved terminal attributes, restored in [`Drop::drop`].
    orig_termios: libc::termios,
    /// Platform syscall implementation used by the [`ForthIo`] methods.
    pub(super) sys: sys::SystemSys,
}

#[cfg(unix)]
#[allow(clippy::new_without_default)]
impl SystemIo {
    /// Construct a new `SystemIo`, putting stdin into raw (non-canonical, no-echo) mode.
    ///
    /// # Panics
    ///
    /// Does not panic, but the underlying `tcsetattr` call is unchecked. Passing a file descriptor
    /// that is not a terminal will silently do nothing on most platforms.
    pub fn new() -> Self {
        let orig_termios = unsafe { sys::sys_set_raw_mode(0) };
        SystemIo {
            orig_termios,
            sys: sys::SystemSys,
        }
    }
}

#[cfg(unix)]
impl Drop for SystemIo {
    /// Restore the terminal to the settings that were in effect before [`SystemIo::new`] was used.
    fn drop(&mut self) {
        unsafe {
            sys::sys_restore_mode(0, &self.orig_termios);
        }
    }
}
