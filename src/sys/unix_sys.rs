//! Unix syscall wrappers used by the rforth I/O layer
//!
//! [`SystemSys`] implements [`super::SysCalls`] using thin, `unsafe` wrappers around the
//! corresponding `libc` calls. Error return values are currently unchecked; this is intentional for
//! the initial scaffolding and can be tightened once the interpreter core is more complete.
//TODO: Implement error handling for syscalls

/// Zero-sized token that carries the Unix [`super::SysCalls`] implementation
///
/// Construct one with `SystemSys` (unit struct literal) and call [`super::SysCalls`] methods on it
/// to perform raw I/O. The two Unix-only terminal helpers ([`sys_set_raw_mode`] and
/// [`sys_restore_mode`]) are free functions rather than trait methods because they have no
/// cross-platform equivalent.
pub struct SystemSys;

impl super::SysCalls for SystemSys {
    /// Read up to `buf.len()` bytes from the file descriptor `fd` via raw `read(2)`
    ///
    /// # Safety
    ///
    /// `fd` must be a valid, readable file descriptor. `buf` must be valid for writes of
    /// `buf.len()` bytes for the duration of the call.
    unsafe fn sys_read(&self, fd: i32, buf: &mut [u8]) -> isize {
        unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) as isize }
    }

    /// Write `buf` to the file descriptor `fd` via raw `write(2)`
    ///
    /// # Safety
    ///
    /// `fd` must be a valid, writable file descriptor. `buf` must be valid for reads of
    /// `buf.len()` bytes for the duration of the call.
    unsafe fn sys_write(&self, fd: i32, buf: &[u8]) -> isize {
        unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) as isize }
    }
}

/// Switch file descriptor `fd` to raw (non-canonical, no-echo) terminal mode and return the
/// previous [`libc::termios`] so it can be restored later
///
/// Concretely, this clears `ICANON` and `ECHO` from `c_lflag`, sets `VMIN=1` and `VTIME=0` so that
/// reads return after exactly one byte, and applies the new settings with `TCSAFLUSH`.
///
/// # Safety
///
/// `fd` must be an open file descriptor that refers to a terminal.
pub unsafe fn sys_set_raw_mode(fd: i32) -> libc::termios {
    unsafe {
        let mut orig = core::mem::zeroed::<libc::termios>();
        let rc = libc::tcgetattr(fd, &mut orig);
        assert_eq!(rc, 0, "libc::tcgetattr failed: unable to get terminal attributes");
        let mut raw = orig;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        let rc = libc::tcsetattr(fd, libc::TCSAFLUSH, &raw);
        assert_eq!(rc, 0, "libc::tcsetattr failed: unable to set terminal attributes for raw mode");
        orig
    }
}

/// Restore the terminal attributes of `fd` to `orig` using `TCSAFLUSH`
///
/// This is the inverse of [`sys_set_raw_mode`] and is called from the [`Drop`] impl of `SystemIo` to
/// leave the terminal in a usable state after the interpreter exits.
///
/// # Safety
///
/// `fd` must be an open file descriptor that refers to a terminal, and `orig` must have been
/// obtained from a prior call to `tcgetattr` (or [`sys_set_raw_mode`]) on the same terminal.
pub unsafe fn sys_restore_mode(fd: i32, orig: &libc::termios) {
    unsafe {
        libc::tcsetattr(fd, libc::TCSAFLUSH, orig);
    }
}
