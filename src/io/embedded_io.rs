//! Embedded stub implementation of [`ForthIo`] for [`SystemIo`]
//!
//! This module is compiled only when targeting a bare-metal platform (`feature = "embedded"` with a
//! non-Unix target).  Both methods panic with `unimplemented!` until real peripheral drivers are
//! wired in.

use super::*;

impl ForthIo for SystemIo {
    /// Not yet implemented for embedded targets
    ///
    /// # Panics
    ///
    /// Always panics.
    fn emit(&mut self, _c: u8) {
        unimplemented!("emit not implemented for embedded target")
    }

    /// Not yet implemented for embedded targets
    ///
    /// # Panics
    ///
    /// Always panics.
    fn emit_error(&mut self, _c: u8) {
        unimplemented!("emit_error not implemented for embedded target")
    }

    /// Not yet implemented for embedded targets
    ///
    /// # Panics
    ///
    /// Always panics.
    fn key(&mut self) -> u8 {
        unimplemented!("key not implemented for embedded target")
    }
}
