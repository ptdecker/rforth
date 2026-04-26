//! Integration tests for the source-driven outer interpreter
//!
//! These tests cover interactive prompt behavior, batch stdin processing, stderr diagnostics, and
//! source-level compilation of numeric literals and colon definitions.

use rforth::{run_forth, run_forth_steps, vm::TIB_SIZE};

mod common;

use common::ScriptedIo;

/// Verifies startup emits the prompt only for interactive sources.
#[test]
fn emits_startup_banner_only_for_interactive_mode() {
    let mut interactive = ScriptedIo::new(b"", true);
    let mut batch = ScriptedIo::new(b"", false);

    run_forth_steps(&mut interactive, 0);
    run_forth_steps(&mut batch, 0);

    assert_eq!(
        interactive.output.as_slice(),
        b"OK\n",
        "interactive startup should emit the standard OK prompt"
    );
    assert!(
        batch.output.is_empty(),
        "batch startup should not emit prompts before any source is processed"
    );
}

/// Verifies interactive mode still echoes input and writes unknown-word diagnostics to stderr.
#[test]
fn interactive_mode_echoes_input_and_reports_errors_to_stderr() {
    let mut io = ScriptedIo::new(b"one two\n", true);

    run_forth_steps(&mut io, b"one two\n".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\none two\nOK\n",
        "interactive mode should echo the typed line and print a fresh prompt after the error"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"one ?\n",
        "interactive diagnostics should be written to stderr"
    );
}

/// Verifies recoverable interactive errors do not affect a later clean BYE exit status.
#[test]
fn interactive_mode_returns_success_after_recovering_from_error() {
    let mut io = ScriptedIo::new(b"bad\n1 .\nBYE\n", true);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 0,
        "a recovered interactive source error should not make a later BYE return failure"
    );
    assert_eq!(
        io.output.as_slice(),
        b"OK\nbad\nOK\n1 .\n1 \nOK\nBYE\n",
        "interactive mode should continue normally after the recovered error"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"bad ?\n",
        "the recovered error should still be reported to stderr"
    );
}

/// Verifies interactive backspace deletes the previous buffered input byte.
#[test]
fn interactive_mode_backspace_deletes_previous_input_byte() {
    let mut io = ScriptedIo::new(b"AB\x7fC\n", true);

    run_forth_steps(&mut io, b"AB\x7fC\n".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\nAB\x08 \x08C\nOK\n",
        "interactive delete should erase one visible character and leave the edited line"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"AC ?\n",
        "the interpreted token should reflect the line after backspace editing"
    );
}

/// Verifies an interactive backspace at the start of a line is ignored.
#[test]
fn interactive_mode_ignores_backspace_when_line_is_empty() {
    let mut io = ScriptedIo::new(b"\x08A\n", true);

    run_forth_steps(&mut io, b"\x08A\n".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\nA\nOK\n",
        "backspace at an empty line should not emit an erase sequence"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"A ?\n",
        "the later input should still be interpreted normally"
    );
}

/// Verifies batch mode suppresses prompts and echo while still reporting diagnostics to stderr.
#[test]
fn batch_mode_suppresses_prompts_and_echo() {
    let mut io = ScriptedIo::new(b"one two\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "an unknown word in batch mode should return the unknown-or-syntax exit code"
    );
    assert!(
        io.output.is_empty(),
        "batch mode should not echo source input or print interactive prompts"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"one ?\n",
        "batch mode should report unknown words to stderr"
    );
}

/// Verifies interactive input-source failures are fatal and return the I/O exit code.
#[test]
fn interactive_mode_returns_io_exit_code_on_input_failure() {
    let mut io = ScriptedIo::with_read_error(b"", true);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 6,
        "an interactive input failure should return the I/O exit code"
    );
    assert_eq!(
        io.output.as_slice(),
        b"OK\n",
        "interactive mode should emit only the startup prompt before the fatal read failure"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"io-error ?\n",
        "fatal input failures should be reported to stderr"
    );
}

/// Verifies carriage return is echoed and normalized only in interactive mode.
#[test]
fn carriage_return_echoes_newline_before_prompt_in_interactive_mode() {
    let mut io = ScriptedIo::new(b"QUIT\r", true);

    run_forth_steps(&mut io, b"QUIT\r".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\nQUIT\r\nOK\n",
        "interactive carriage return should be echoed and normalized before the next prompt"
    );
    assert!(
        io.stderr.is_empty(),
        "QUIT should not produce any stderr diagnostics"
    );
}

/// Verifies interactive dot output stays standard and prompt placement is handled by the REPL.
#[test]
fn interactive_dot_output_keeps_trailing_space_before_prompt_newline() {
    let mut io = ScriptedIo::new(b"1 . 2 .\n", true);

    run_forth_steps(&mut io, b"1 . 2 .\n".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\n1 . 2 .\n1 2 \nOK\n",
        "dot should emit only trailing spaces while the REPL puts the prompt on the next line"
    );
    assert!(
        io.stderr.is_empty(),
        "valid dot output should not emit diagnostics"
    );
}

/// Verifies a line that exactly fills the terminal input buffer is still accepted in batch mode.
#[test]
fn accepts_a_line_that_exactly_fills_the_terminal_input_buffer() {
    let mut input = vec![b'a'; TIB_SIZE];
    input.push(b'\n');
    let mut io = ScriptedIo::new(&input, false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "a full TIB line containing an unknown word should still complete line processing and exit with an error"
    );
    assert!(
        io.output.is_empty(),
        "batch mode should not echo or prompt while processing a full TIB line"
    );
    let mut expected_stderr = vec![b'a'; TIB_SIZE];
    expected_stderr.extend(b" ?\n");
    assert_eq!(
        io.stderr.as_slice(),
        expected_stderr.as_slice(),
        "a line whose length exactly matches TIB_SIZE should be accepted and then diagnosed normally"
    );
}

/// Verifies overflow characters are ignored until a newline and the overflow is reported to stderr.
#[test]
fn suppresses_execution_after_terminal_input_buffer_overflow() {
    let mut input = vec![b'a'; TIB_SIZE];
    input.extend_from_slice(b"bc\nQUIT\n");
    let mut io = ScriptedIo::new(&input, false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 5,
        "a terminal input buffer overflow should return the dictionary-or-TIB exit code in batch mode"
    );
    assert!(
        io.output.is_empty(),
        "batch mode should not emit stdout while discarding an overflowed line"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"tib-overflow ?\n",
        "overflowed input should be discarded and reported to stderr once the newline arrives"
    );
}

/// Verifies interactive backspace can recover after a terminal input buffer overflow.
#[test]
fn interactive_backspace_after_overflow_accepts_more_input() {
    let mut input = vec![b' '; TIB_SIZE - 3];
    input.extend_from_slice(b"1 .x\x7f.\n");
    let mut io = ScriptedIo::new(&input, true);

    let exit = run_forth(&mut io);

    let mut expected_output = b"OK\n".to_vec();
    expected_output.extend(core::iter::repeat_n(b' ', TIB_SIZE - 3));
    expected_output.extend_from_slice(b"1 .\x08 \x08.\n1 \nOK\n");

    assert_eq!(
        exit, 0,
        "backspace should clear overflow state once the TIB is below capacity"
    );
    assert_eq!(
        io.output.as_slice(),
        expected_output.as_slice(),
        "interactive input should echo the recovered line and execute it"
    );
    assert!(
        io.stderr.is_empty(),
        "recovering from the overflow before Enter should avoid a TIB diagnostic"
    );
}

/// Verifies `BYE` terminates the runner without another prompt and returns success when no earlier
/// error occurred.
#[test]
fn bye_stops_the_runner_cleanly() {
    let mut io = ScriptedIo::new(b"BYE\nQUIT\n", true);

    run_forth_steps(&mut io, b"BYE\nQUIT\n".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\nBYE\n",
        "BYE should terminate the interactive runner before later input is processed"
    );
    assert!(io.stderr.is_empty(), "BYE should not emit any diagnostics");
}

/// Verifies batch mode can compile and execute a colon definition from source text.
#[test]
fn batch_mode_compiles_and_executes_a_colon_definition() {
    let mut io = ScriptedIo::new(b": ONE 1 ; ONE .\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 0,
        "a valid colon definition and execution in batch mode should exit successfully"
    );
    assert_eq!(
        io.output.as_slice(),
        b"1 ",
        "dot should emit the top stack value to stdout after executing the compiled word"
    );
    assert!(
        io.stderr.is_empty(),
        "successful batch compilation and execution should not emit diagnostics"
    );
}

/// Verifies source-level ABORT reports a user-facing failure instead of an internal error.
#[test]
fn batch_mode_reports_abort_as_source_failure() {
    let mut io = ScriptedIo::new(b"ABORT\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "ABORT should return the unknown-or-syntax exit code"
    );
    assert!(
        io.output.is_empty(),
        "ABORT should not emit normal output in batch mode"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"abort ?\n",
        "ABORT should emit a user-facing diagnostic"
    );
}

/// Verifies ?ABORT reports the same user-facing failure when its flag is zero.
#[test]
fn batch_mode_reports_question_abort_as_source_failure() {
    let mut io = ScriptedIo::new(b"0 ?ABORT\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "?ABORT should return the unknown-or-syntax exit code when its flag is zero"
    );
    assert!(
        io.output.is_empty(),
        "?ABORT should not emit normal output in batch mode"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"abort ?\n",
        "?ABORT should emit the same diagnostic as ABORT"
    );
}

/// Verifies KEY reports EOF through the runner instead of aborting the host process.
#[test]
fn batch_mode_reports_key_end_of_input() {
    let mut io = ScriptedIo::new(b"KEY\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 6,
        "KEY at EOF should return the I/O exit code in batch mode"
    );
    assert!(
        io.output.is_empty(),
        "KEY at EOF should not emit normal output"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"end-of-input ?\n",
        "KEY at EOF should report the structured VM input error"
    );
}

/// Verifies batch mode ignores backslash comments through the end of the current line.
#[test]
fn batch_mode_ignores_backslash_comments() {
    let mut io = ScriptedIo::new(b"\\ ignored words\n2 .\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(exit, 0, "a backslash comment should not fail batch input");
    assert_eq!(
        io.output.as_slice(),
        b"2 ",
        "source after the commented line should still execute normally"
    );
    assert!(
        io.stderr.is_empty(),
        "ignored backslash comment text should not emit diagnostics"
    );
}

/// Verifies batch mode ignores whitespace-delimited parenthesized comments.
#[test]
fn batch_mode_ignores_parenthesized_comments() {
    let mut io = ScriptedIo::new(b"1 ( ignored words ) .\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 0,
        "a complete parenthesized comment should not fail batch input"
    );
    assert_eq!(
        io.output.as_slice(),
        b"1 ",
        "dot should see the value before the parenthesized comment"
    );
    assert!(
        io.stderr.is_empty(),
        "ignored parenthesized comment text should not emit diagnostics"
    );
}

/// Verifies source comments are ignored while compiling a colon definition.
#[test]
fn comments_are_ignored_while_compiling() {
    let mut io = ScriptedIo::new(b": ONE ( ignored words ) 1 ; ONE .\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 0,
        "comments inside a colon definition should not affect compilation"
    );
    assert_eq!(
        io.output.as_slice(),
        b"1 ",
        "the compiled definition should execute only its real source tokens"
    );
    assert!(
        io.stderr.is_empty(),
        "ignored compile-time comment text should not emit diagnostics"
    );
}

/// Verifies an unterminated parenthesized comment is treated as malformed source code
#[test]
fn batch_mode_reports_unterminated_parenthesized_comment() {
    let mut io = ScriptedIo::new(b"1 ( missing terminator\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "an unterminated parenthesized comment should return the unknown-or-syntax exit code"
    );
    assert!(
        io.output.is_empty(),
        "unterminated comments should not emit normal output"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"invalid-source ?\n",
        "batch mode should report unterminated comments as malformed source"
    );
}

/// Verifies interactive backslash comments still echo input and print a fresh prompt.
#[test]
fn interactive_mode_ignores_backslash_comments() {
    let mut io = ScriptedIo::new(b"\\ ignored words\n", true);

    run_forth_steps(&mut io, b"\\ ignored words\n".len());

    assert_eq!(
        io.output.as_slice(),
        b"OK\n\\ ignored words\nOK\n",
        "interactive mode should echo the commented line and print the next prompt"
    );
    assert!(
        io.stderr.is_empty(),
        "ignored interactive comment text should not emit diagnostics"
    );
}

/// Verifies batch mode rejects an unterminated definition at the end of input.
#[test]
fn batch_mode_reports_unexpected_eof_while_compiling() {
    let mut io = ScriptedIo::new(b": ONE 1\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "unexpected EOF while compiling should return the unknown-or-syntax exit code"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"unexpected-eof ?\n",
        "batch mode should report unterminated definitions to stderr at EOF"
    );
}

/// Verifies a batch-mode syntax error latches a nonzero exit and stops on the first failure.
#[test]
fn batch_mode_stops_on_first_error() {
    let mut io = ScriptedIo::new(b"1 .\nmissing\n2 .\n", false);

    let exit = run_forth(&mut io);

    assert_eq!(
        exit, 1,
        "batch mode should stop on the first source error and return its exit category"
    );
    assert_eq!(
        io.output.as_slice(),
        b"1 ",
        "batch mode should keep stdout output that happened before the first error"
    );
    assert_eq!(
        io.stderr.as_slice(),
        b"missing ?\n",
        "batch mode should stop after the first unknown-word diagnostic"
    );
}
