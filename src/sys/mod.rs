//! Raw syscall wrappers for rforth
//!
//! This module defines the [`SysCalls`] trait specifying the contract every platform-specific 
//! syscall implementation must satisfy. It also re-exports the platform-appropriate [`SystemSys`]
//! struct so the I/O layer can use `crate::sys::SystemSys` without knowing which platform it is on.
//!
//! The two Unix-only helpers ([`sys_set_raw_mode`] and [`sys_restore_mode`]) are deliberately
//! excluded from the trait because they have no meaningful analogue on embedded targets.
//!
//! Platform implementations:
//! - [`unix_sys`] — thin wrappers around `libc` for Unix targets
//! - `embedded_sys` — unimplemented stubs for bare-metal targets

/// Raw syscall contract that every platform implementation must satisfy
///
/// Implement this trait on a platform-specific `SystemSys` struct to wire up a new target. The I/O
/// layer ([`crate::io`]) depends solely on this trait for character-level reads and writes, keeping
/// the interpreter core free of any platform knowledge.
///
/// All methods are `unsafe` — callers must uphold the standard preconditions for the underlying
/// syscall (valid file descriptors, valid buffer pointers and lengths, etc.).
pub trait SysCalls {
    /// Read up to `buf.len()` bytes from the file descriptor `fd` into `buf`
    ///
    /// Returns the number of bytes actually read, or a negative value on error (the raw `read(2)`
    /// return value).
    ///
    /// # Safety
    ///
    /// `fd` must be a valid, readable file descriptor. `buf` must be valid for writes of
    /// `buf.len()` bytes for the duration of the call.
    unsafe fn sys_read(&self, fd: i32, buf: &mut [u8]) -> isize;

    /// Write `buf` to the file descriptor `fd`
    ///
    /// Returns the number of bytes written, or a negative value on error (the raw `write(2)` return
    /// value). Short writes are possible but not currently retried.
    //TODO: Check for short writes and retry
    ///
    /// # Safety
    ///
    /// `fd` must be a valid, writable file descriptor. `buf` must be valid for reads of `buf.len()`
    /// bytes for the duration of the call.
    unsafe fn sys_write(&self, fd: i32, buf: &[u8]) -> isize;
}

#[cfg(unix)]
pub mod unix_sys;

#[cfg(unix)]
pub use unix_sys::{SystemSys, sys_restore_mode, sys_set_raw_mode};

#[cfg(all(not(unix), feature = "embedded"))]
pub mod embedded_sys;

#[cfg(all(not(unix), feature = "embedded"))]
pub use embedded_sys::SystemSys;
