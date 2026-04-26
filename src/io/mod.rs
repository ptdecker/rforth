//! Platform-agnostic I/O layer for rforth
//!
//! This module defines the [`ForthIo`] trait — the only I/O interface the interpreter core ever
//! sees — and the [`SystemIo`] struct that provides a concrete implementation for the current
//! compiler target.
//!
//! Platform implementations live in the submodules:
//! - [`unix_io`] — raw-mode terminal I/O via `libc` syscalls (Unix)
//! - [`embedded_io`] — stub implementation for bare-metal targets
//!
//! The correct submodule is selected at compile time; attempting to build for an unsupported target
//! combination is a hard compiler error.

use crate::sys;

/// Result of attempting to read one input byte from the active source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// One input byte was read successfully.
    Byte(u8),
    /// The current input source reached the end of the input.
    Eof,
    /// The input source failed in a way that is not an ordinary EOF.
    Error,
}

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

    /// Write a single diagnostic byte to the error output channel
    ///
    /// Implementations that do not support a distinct error stream may forward diagnostics to the
    /// same destination as [`ForthIo::emit`].
    fn emit_error(&mut self, c: u8) {
        self.emit(c);
    }

    /// Block until one byte is available from the input channel and return it
    ///
    /// On Unix this reads from stdin (fd 0) with the terminal in raw mode, so each keystroke is
    /// returned immediately without waiting for a newline.
    fn key(&mut self) -> u8;

    /// Read the next byte from the input source or report end-of-input or failure
    ///
    /// The default implementation preserves the older blocking single-byte contract by forwarding
    /// to [`ForthIo::key`].
    fn read_key(&mut self) -> InputEvent {
        InputEvent::Byte(self.key())
    }

    /// Return whether the active input source should be treated as interactive
    ///
    /// Interactive inputs get prompts and local echo; batch inputs such as files and pipes do not.
    fn is_interactive(&self) -> bool {
        true
    }
}

/// Forward [`ForthIo`] calls through mutable references
///
/// This lets higher-level structures, such as the VM, own a borrowed I/O object while still being
/// generic over `I: ForthIo`.
impl<T: ForthIo + ?Sized> ForthIo for &mut T {
    fn emit(&mut self, c: u8) {
        (**self).emit(c);
    }

    fn emit_error(&mut self, c: u8) {
        (**self).emit_error(c);
    }

    fn key(&mut self) -> u8 {
        (**self).key()
    }

    fn read_key(&mut self) -> InputEvent {
        (**self).read_key()
    }

    fn is_interactive(&self) -> bool {
        (**self).is_interactive()
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
    /// Saved terminal attributes, restored in [`Drop::drop`] for interactive terminals.
    orig_termios: Option<libc::termios>,
    /// Whether stdin is a terminal and should be treated as interactive.
    interactive: bool,
    /// Platform syscall implementation used by the [`ForthIo`] methods.
    pub(super) sys: sys::SystemSys,
}

#[cfg(unix)]
impl SystemIo {
    /// Construct a new `SystemIo`, putting stdin into raw (non-canonical, no-echo) mode
    ///
    /// # Panics
    ///
    /// Panics if stdin is a terminal, but its attributes cannot be read or switched to raw mode.
    pub fn new() -> Self {
        // SAFETY: `isatty` only inspects the supplied file descriptor and has no Rust aliasing or
        // lifetime requirements. File descriptor 0 is stdin for this process.
        let interactive = unsafe { libc::isatty(0) == 1 };
        let orig_termios = if interactive {
            // SAFETY: `interactive` confirms stdin is a terminal, satisfying `sys_set_raw_mode`'s
            // file descriptor precondition.
            Some(unsafe { sys::sys_set_raw_mode(0) })
        } else {
            None
        };
        SystemIo {
            orig_termios,
            interactive,
            sys: sys::SystemSys,
        }
    }
}

#[cfg(unix)]
impl Default for SystemIo {
    /// Construct the default Unix system I/O backend.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(unix)]
impl Drop for SystemIo {
    /// Restore the terminal to the settings that were in effect before [`SystemIo::new`] was used.
    fn drop(&mut self) {
        if let Some(orig_termios) = &self.orig_termios {
            // SAFETY: `orig_termios` is only stored after successfully putting stdin into raw mode,
            // so it belongs to the same terminal file descriptor being restored here.
            unsafe {
                sys::sys_restore_mode(0, orig_termios);
            }
        }
    }
}
