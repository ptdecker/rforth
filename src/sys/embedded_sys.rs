//! Embedded stubs for the syscall layer
//!
//! [`SystemSys`] implements [`super::SysCalls`] for bare-metal targets. Both methods panic
//! immediately if unimplemented. They exist so that the rest of the codebase can compile for
//! bare-metal targets; replace the bodies with real peripheral driver calls when bringing up a
//! new board.

/// Zero-sized token that carries the embedded [`super::SysCalls`] implementation
///
/// Both trait methods are stubs that will panic at runtime.  Swap in real peripheral driver
/// calls once a concrete board bring-up is underway.
pub struct SystemSys;

impl super::SysCalls for SystemSys {
    /// Not yet implemented for embedded targets
    ///
    /// # Panics
    ///
    /// Always panics.
    ///
    /// # Safety
    ///
    /// No safety guarantees — this function never returns normally.
    unsafe fn sys_read(&self, _fd: i32, _buf: &mut [u8]) -> isize {
        unimplemented!("sys_read not implemented for embedded target")
    }

    /// Not yet implemented for embedded targets
    ///
    /// # Panics
    ///
    /// Always panics.
    ///
    /// # Safety
    ///
    /// No safety guarantees — this function never returns normally.
    unsafe fn sys_write(&self, _fd: i32, _buf: &[u8]) -> isize {
        unimplemented!("sys_write not implemented for embedded target")
    }
}
