//! Integration tests for the current `run_forth_steps` outer interpreter.
//!
//! These tests pin the visible startup banner, character echo, terminal-input-buffer accumulation,
//! dictionary lookup, stage-zero word execution, and line-level error reporting exposed through the
//! runner API.

use rforth::{io::ForthIo, run_forth_steps, vm::TIB_SIZE};

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

    assert_eq!(
        io.output(),
        b"OK\n",
        "runner startup should emit a single OK prompt before any input is consumed"
    );
}

/// Verifies a newline completes the input line and reports the first unknown word.
#[test]
fn reports_unknown_words_after_newline() {
    let mut io = ScriptedIo::new(b"one two\n");

    run_forth_steps(&mut io, b"one two\n".len());

    assert_eq!(
        io.output(),
        b"OK\none two\none ?\n",
        "an unknown first token should be echoed and then reported with the standard ? marker"
    );
}

/// Verifies carriage return is echoed and normalized before unknown-word reporting.
#[test]
fn carriage_return_echoes_newline_before_error_output() {
    let mut io = ScriptedIo::new(b"one two\r");

    run_forth_steps(&mut io, b"one two\r".len());

    assert_eq!(
        io.output(),
        b"OK\none two\r\none ?\n",
        "carriage return should be echoed with a normalized newline before error reporting"
    );
}

/// Verifies each completed input line starts a fresh terminal input buffer and ends with a fresh
/// `OK`.
#[test]
fn resets_line_after_each_completed_input_line() {
    let mut io = ScriptedIo::new(b"QUIT\nQUIT\n");

    run_forth_steps(&mut io, b"QUIT\nQUIT\n".len());

    assert_eq!(
        io.output(),
        b"OK\nQUIT\nOK\nQUIT\nOK\n",
        "each completed line should restart at a fresh top-level prompt"
    );
}

/// Verifies a line that exactly fills the terminal input buffer is still accepted.
#[test]
fn accepts_a_line_that_exactly_fills_the_terminal_input_buffer() {
    let mut input = vec![b'a'; TIB_SIZE];
    input.push(b'\n');
    let mut expected = b"OK\n".to_vec();
    expected.extend(vec![b'a'; TIB_SIZE]);
    expected.push(b'\n');
    expected.extend(vec![b'a'; TIB_SIZE]);
    expected.extend(b" ?\n");
    let mut io = ScriptedIo::new(&input);

    run_forth_steps(&mut io, input.len());

    assert_eq!(
        io.output(),
        expected.as_slice(),
        "a line whose length exactly matches TIB_SIZE should be echoed and executed normally"
    );
}

/// Verifies overflow characters are ignored without echo until a newline reports the error.
#[test]
fn suppresses_echo_and_execution_after_terminal_input_buffer_overflow() {
    let mut input = vec![b'a'; TIB_SIZE];
    input.extend_from_slice(b"bc\nQUIT\n");
    let mut expected = b"OK\n".to_vec();
    expected.extend(vec![b'a'; TIB_SIZE]);
    expected.push(b'\n');
    expected.extend(b"tib-overflow ?\nQUIT\nOK\n");
    let mut io = ScriptedIo::new(&input);

    run_forth_steps(&mut io, input.len());

    assert_eq!(
        io.output(),
        expected.as_slice(),
        "once the TIB is full, later characters on that line should be ignored without echo and the line should fail with tib-overflow"
    );
}

/// Verifies `QUIT` abandons the rest of the current input line and returns to the outer loop.
#[test]
fn quit_skips_remaining_tokens_on_the_same_line() {
    let mut io = ScriptedIo::new(b"QUIT one\n");

    run_forth_steps(&mut io, b"QUIT one\n".len());

    assert_eq!(
        io.output(),
        b"OK\nQUIT one\nOK\n",
        "QUIT should ignore the remaining buffered tokens on its input line"
    );
}

/// Verifies `BYE` stops the outer interpreter instead of returning another prompt.
#[test]
fn bye_stops_the_runner() {
    let mut io = ScriptedIo::new(b"BYE\nQUIT\n");

    run_forth_steps(&mut io, b"BYE\nQUIT\n".len());

    assert_eq!(
        io.output(),
        b"OK\nBYE\n",
        "BYE should terminate the runner before later input is processed"
    );
}

/// Verifies runner-driven stage-zero word execution can consume input and emit output.
#[test]
fn key_and_emit_execute_from_the_runner() {
    let mut io = ScriptedIo::new(b"KEY EMIT\nZ");

    run_forth_steps(&mut io, b"KEY EMIT\n".len());

    assert_eq!(
        io.output(),
        b"OK\nKEY EMIT\nZOK\n",
        "KEY should consume the next input byte and EMIT should write it before the next prompt"
    );
}
