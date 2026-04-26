//! Shared integration-test support
//!
//! Each file in `tests/` is compiled as its own crate, so shared test doubles live here and are
//! imported with `mod common`. Keeping them in one module prevents the VM, stage-zero, and
//! outer-interpreter tests from growing separate copies of the same host I/O boundary.

use rforth::io::{ForthIo, InputEvent};

/// Scripted host I/O backend used by integration tests
///
/// Tests exercise the VM and outer interpreter through the same [`ForthIo`] boundary used at
/// runtime. This deterministic stand-in lets them prove `KEY`, `EMIT`, diagnostics, and
/// interactive-mode behavior without depending on a real terminal or OS device state. `key`
/// consumes bytes from `input`, `emit` records bytes in `output`, and `emit_error` records bytes in
/// `stderr`, giving each test a controlled input stream and inspectable output sinks.
pub struct ScriptedIo<'a> {
    /// Input bytes returned one at a time from the scripted input source.
    input: &'a [u8],
    /// Current read offset into `input`.
    input_pos: usize,
    /// Captured stdout bytes.
    pub output: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
    /// Whether the source should be treated as interactive.
    interactive: bool,
    /// Whether reads should report an I/O error after scripted bytes are exhausted.
    fail_after_input: bool,
}

impl<'a> ScriptedIo<'a> {
    /// Construct a scripted backend with explicit interactive-mode behavior.
    pub fn new(input: &'a [u8], interactive: bool) -> Self {
        Self {
            input,
            input_pos: 0,
            output: Vec::new(),
            stderr: Vec::new(),
            interactive,
            fail_after_input: false,
        }
    }

    /// Construct a scripted backend that reports an input failure after its bytes are consumed.
    pub fn with_read_error(input: &'a [u8], interactive: bool) -> Self {
        Self {
            input,
            input_pos: 0,
            output: Vec::new(),
            stderr: Vec::new(),
            interactive,
            fail_after_input: true,
        }
    }
}

impl ForthIo for ScriptedIo<'_> {
    /// Capture one stdout byte.
    fn emit(&mut self, c: u8) {
        self.output.push(c);
    }

    /// Capture one stderr byte.
    fn emit_error(&mut self, c: u8) {
        self.stderr.push(c);
    }

    /// Return the next scripted input byte.
    fn key(&mut self) -> u8 {
        match self.read_key() {
            InputEvent::Byte(c) => c,
            InputEvent::Eof | InputEvent::Error => {
                panic!("scripted key() should not read past EOF")
            }
        }
    }

    /// Read the next scripted input byte or report EOF.
    fn read_key(&mut self) -> InputEvent {
        if self.input_pos == self.input.len() {
            if self.fail_after_input {
                return InputEvent::Error;
            }
            return InputEvent::Eof;
        }
        let c = self.input[self.input_pos];
        self.input_pos += 1;
        InputEvent::Byte(c)
    }

    /// Report whether this scripted source is interactive.
    fn is_interactive(&self) -> bool {
        self.interactive
    }
}
