//! Integration tests for the current `run_forth_steps` interpreter scaffold.
//!
//! These tests pin the visible startup, echo, line-buffering, and token output behavior while the
//! VM internals are introduced behind the existing runner API.

use rforth::{io::ForthIo, run_forth_steps};

/// Scripted host I/O backend for deterministic runner tests.
///
/// `key` consumes bytes from `input`, and `emit` records bytes in `output`.
struct ScriptedIo<'a> {
    /// Input bytes returned one at a time from [`ForthIo::key`].
    input: &'a [u8],
    /// Current read offset into `input`.
    input_pos: usize,
    /// Bytes captured from [`ForthIo::emit`].
    output: Vec<u8>,
}

impl<'a> ScriptedIo<'a> {
    /// Construct a scripted I/O backend with empty captured output.
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            input_pos: 0,
            output: Vec::new(),
        }
    }

    /// Return all bytes emitted by the runner.
    fn output(&self) -> &[u8] {
        &self.output
    }
}

impl ForthIo for ScriptedIo<'_> {
    /// Capture one output byte.
    fn emit(&mut self, c: u8) {
        self.output.push(c);
    }

    /// Return the next scripted input byte.
    fn key(&mut self) -> u8 {
        let c = self.input[self.input_pos];
        self.input_pos += 1;
        c
    }
}

/// Verifies startup emits the banner before any input is read.
#[test]
fn emits_startup_banner_without_reading_input() {
    let mut io = ScriptedIo::new(b"");

    run_forth_steps(&mut io, 0);

    assert_eq!(io.output(), b"OK\n");
}

/// Verifies a newline completes the input line and prints tokenized words.
#[test]
fn echoes_newline_and_outputs_tokenized_words() {
    let mut io = ScriptedIo::new(b"one two\n");

    run_forth_steps(&mut io, b"one two\n".len());

    assert_eq!(io.output(), b"OK\none two\n[one, two]\n");
}

/// Verifies carriage return is echoed and normalized with an additional newline before token output.
#[test]
fn carriage_return_echoes_newline_before_words() {
    let mut io = ScriptedIo::new(b"one two\r");

    run_forth_steps(&mut io, b"one two\r".len());

    assert_eq!(io.output(), b"OK\none two\r\n[one, two]\n");
}

/// Verifies each completed input line starts a fresh line buffer.
#[test]
fn resets_line_after_each_completed_input_line() {
    let mut io = ScriptedIo::new(b"one two\nthree\n");

    run_forth_steps(&mut io, b"one two\nthree\n".len());

    assert_eq!(io.output(), b"OK\none two\n[one, two]\nthree\n[three]\n");
}

/// Verifies bytes beyond the fixed line buffer capacity are echoed but not buffered.
#[test]
fn ignores_input_bytes_after_line_buffer_is_full() {
    let input = [b'a'; 130];
    let mut io = ScriptedIo::new(&input);

    run_forth_steps(&mut io, input.len());

    assert_eq!(
        io.output(),
        &[b"OK\n".as_slice(), input.as_slice()].concat()
    );
}
