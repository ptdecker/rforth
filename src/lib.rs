#![no_std]

//! `rforth` — a minimal, dependency-free Forth interpreter
//!
//! The interpreter core is platform-agnostic and communicates with the outside world exclusively
//! through the [`ForthIo`] trait.

pub mod io;
pub mod sys;
pub mod tokenizer;
pub mod vm;
pub mod words;

use io::{ForthIo, InputEvent};
use vm::{Cell, ForthVm, TIB_SIZE, VmError};
use words::Control;

/// Process exit status used when interpretation completes successfully.
const EXIT_SUCCESS: i32 = 0;

/// Process exit status used for unknown words and malformed source.
const EXIT_UNKNOWN_OR_SYNTAX: i32 = 1;

/// Process exit status used for data-stack underflow or overflow.
const EXIT_DATA_STACK: i32 = 2;

/// Process exit status used for return-stack underflow or overflow.
const EXIT_RETURN_STACK: i32 = 3;

/// Process exit status used for invalid VM memory access.
const EXIT_MEMORY_ACCESS: i32 = 4;

/// Process exit status used for dictionary or terminal-input-buffer failures.
const EXIT_DICTIONARY_OR_TIB: i32 = 5;

/// Process exit status used for host input/output failures.
const EXIT_IO: i32 = 6;

/// Process exit status used when no narrower failure category applies.
const EXIT_INTERNAL: i32 = 7;

/// Dictionary lookup token used when compiling numeric literals.
const LIT_WORD_NAME: &[u8] = b"LIT";

/// Source token that starts a line comment.
const LINE_COMMENT_WORD: &[u8] = b"\\";

/// Source token that starts a parenthesized comment.
const PAREN_COMMENT_START_WORD: &[u8] = b"(";

/// Source token that ends a parenthesized comment.
const PAREN_COMMENT_END_WORD: &[u8] = b")";

/// ASCII Backspace byte produced by some terminals.
const BACKSPACE_BYTE: u8 = 0x08;

/// ASCII Delete byte commonly produced by the Backspace key in raw terminal mode.
const DELETE_BYTE: u8 = 0x7f;

/// Minimal terminal echo sequence for erasing one visible character.
const ERASE_PREVIOUS_CHARACTER: &[u8] = b"\x08 \x08";

/// Error category used for Unix exit status mapping
///
/// The runner groups failures by class instead of exposing raw internal errors so batch execution
/// can return stable process statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorCategory {
    UnknownOrSyntax,
    DataStack,
    ReturnStack,
    MemoryAccess,
    DictionaryOrTib,
    Io,
    Internal,
}

/// One source-token interpretation failure
///
/// The runner distinguishes unknown source words from VM failures because they are reported and
/// categorized differently at the batch/REPL boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterpretError {
    UnknownWord,
    Vm(VmError),
}

/// Run the Forth interpreter using `io` for all character-level I/O
///
/// Interactive terminals print prompts and echo input. Non-interactive stdin is treated as batch
/// source input, writes diagnostics to stderr, stops on the first error, and returns a nonzero exit
/// status for that failure category. This keeps the library entry point aligned with two real uses:
/// a human-facing REPL and file-or-pipe driven batch execution.
pub fn run_forth(io: &mut impl ForthIo) -> i32 {
    run_forth_loop(io, None)
}

/// Run the interpreter for a fixed number of top-level input events
///
/// This helper exists for host-side tests that need deterministic control over how much scripted
/// input the runner consumes before assertions inspect the resulting VM-visible behavior.
pub fn run_forth_steps(io: &mut impl ForthIo, keys_to_read: usize) {
    let _ = run_forth_loop(io, Some(keys_to_read));
}

/// Run the shared interpreter loop, optionally limiting the number of input reads
///
/// Both the public REPL entry point and the test helper flow through this function, so interactive,
/// batch, and scripted-execution share the same outer-interpreter behavior.
fn run_forth_loop(io: &mut impl ForthIo, max_reads: Option<usize>) -> i32 {
    let interactive = io.is_interactive();
    let mut vm = ForthVm::new(io);
    words::install_stage_zero(&mut vm)
        .expect("stage-zero and stage-one bootstrap words should install");
    let mut reads = 0usize;
    let mut line_overflowed = false;
    let mut last_error: Option<ErrorCategory> = None;

    if interactive {
        emit_ok(&mut vm.io);
    }

    loop {
        if let Some(limit) = max_reads
            && reads == limit
        {
            break;
        }
        reads += 1;

        match vm.io.read_key() {
            InputEvent::Byte(byte) => {
                if let Some(code) = process_input_byte(
                    &mut vm,
                    byte,
                    interactive,
                    &mut line_overflowed,
                    &mut last_error,
                ) {
                    return code;
                }
            }
            InputEvent::Eof => {
                return handle_end_of_input(&mut vm, interactive, line_overflowed, last_error);
            }
            InputEvent::Error => {
                let category = ErrorCategory::Io;
                emit_category_message(&mut vm.io, b"io-error");
                return finish_fatal_error(category, &mut last_error);
            }
        }
    }

    last_error.map_or(EXIT_SUCCESS, exit_code)
}

/// Process one input byte in either interactive or batch mode
///
/// The runner handles input a byte at a time, so the same code can support echoed terminal input,
/// batch stdin, terminal-input-buffer overflow handling, and end-of-line driven source parsing.
fn process_input_byte(
    vm: &mut ForthVm<&mut impl ForthIo>,
    byte: u8,
    interactive: bool,
    line_overflowed: &mut bool,
    last_error: &mut Option<ErrorCategory>,
) -> Option<i32> {
    if interactive && (byte == BACKSPACE_BYTE || byte == DELETE_BYTE) {
        if vm.remove_last_tib_byte() {
            *line_overflowed = false;
            emit_bytes(&mut vm.io, ERASE_PREVIOUS_CHARACTER);
        }
        return None;
    }

    if byte == b'\r' || byte == b'\n' {
        if interactive {
            vm.io.emit(byte);
            if byte == b'\r' {
                vm.io.emit(b'\n');
            }
        }

        if *line_overflowed {
            let category = ErrorCategory::DictionaryOrTib;
            emit_category_message(&mut vm.io, b"tib-overflow");
            vm.reset_tib();
            *line_overflowed = false;
            let code = finish_error(interactive, &mut vm.io, category, last_error);
            return if interactive { None } else { Some(code) };
        }

        return process_line(vm, interactive, last_error);
    }

    if *line_overflowed {
        return None;
    }

    match vm.append_tib_byte(byte) {
        Ok(()) => {
            if interactive {
                vm.io.emit(byte);
            }
            None
        }
        Err(VmError::TibOverflow) => {
            *line_overflowed = true;
            None
        }
        Err(error) => {
            let category = category_for_vm_error(error);
            emit_vm_error(&mut vm.io, error);
            vm.reset_tib();
            let code = finish_error(interactive, &mut vm.io, category, last_error);
            if interactive { None } else { Some(code) }
        }
    }
}

/// Handle ordinary end-of-input in either interactive or batch mode
///
/// End-of-input is normal in batch mode, but it still needs explicit handling, so an unfinished
/// source line or open definition can become a deterministic interpreter error instead of being
/// silently dropped.
fn handle_end_of_input(
    vm: &mut ForthVm<&mut impl ForthIo>,
    interactive: bool,
    line_overflowed: bool,
    mut last_error: Option<ErrorCategory>,
) -> i32 {
    if line_overflowed {
        let category = ErrorCategory::DictionaryOrTib;
        emit_category_message(&mut vm.io, b"tib-overflow");
        return finish_error(interactive, &mut vm.io, category, &mut last_error);
    }

    if vm.tib_len > 0
        && let Some(code) = process_line(vm, interactive, &mut last_error)
    {
        return code;
    }

    if vm.state == vm::InterpreterState::Compiling {
        let category = ErrorCategory::UnknownOrSyntax;
        emit_category_message(&mut vm.io, b"unexpected-eof");
        return finish_error(interactive, &mut vm.io, category, &mut last_error);
    }

    last_error.map_or(EXIT_SUCCESS, exit_code)
}

/// Interpret or compile the current completed source line
///
/// The outer interpreter works line-by-line through the terminal input buffer, so source parsing
/// can be stateful without allocating host-side token storage.
fn process_line(
    vm: &mut ForthVm<&mut impl ForthIo>,
    interactive: bool,
    last_error: &mut Option<ErrorCategory>,
) -> Option<i32> {
    let mut scratch = [0u8; TIB_SIZE];
    vm.input_pos = 0;
    let mut line_emitted_output = false;

    loop {
        let token = match vm.next_tib_word(&mut scratch) {
            Ok(Some(token)) => token,
            Ok(None) => break,
            Err(error) => {
                let category = category_for_vm_error(error);
                emit_vm_error(&mut vm.io, error);
                vm.reset_tib();
                let code = finish_error(interactive, &mut vm.io, category, last_error);
                return if interactive { None } else { Some(code) };
            }
        };

        let starts_line_comment = token == LINE_COMMENT_WORD;
        let starts_parenthesized_comment = token == PAREN_COMMENT_START_WORD;

        if starts_line_comment {
            break;
        }

        if starts_parenthesized_comment {
            match skip_parenthesized_comment(vm, &mut scratch) {
                Ok(()) => continue,
                Err(error) => {
                    let category = category_for_vm_error(error);
                    emit_vm_error(&mut vm.io, error);
                    vm.reset_tib();
                    let code = finish_error(interactive, &mut vm.io, category, last_error);
                    return if interactive { None } else { Some(code) };
                }
            }
        }

        let token_may_emit_output =
            vm.state == vm::InterpreterState::Interpreting && (token == b"." || token == b"EMIT");

        match interpret_token(vm, token) {
            Ok(Control::Continue) => {}
            Ok(Control::Quit) => {
                if interactive {
                    emit_ok(&mut vm.io);
                }
                return None;
            }
            Ok(Control::Bye) => {
                return Some(last_error.map_or(EXIT_SUCCESS, exit_code));
            }
            Ok(Control::Abort) => {
                let category = ErrorCategory::UnknownOrSyntax;
                emit_vm_error(&mut vm.io, VmError::Abort);
                let code = finish_error(interactive, &mut vm.io, category, last_error);
                return if interactive { None } else { Some(code) };
            }
            Err(InterpretError::UnknownWord) => {
                emit_unknown_word(&mut vm.io, token);
                vm.reset_tib();
                let code = finish_error(
                    interactive,
                    &mut vm.io,
                    ErrorCategory::UnknownOrSyntax,
                    last_error,
                );
                return if interactive { None } else { Some(code) };
            }
            Err(InterpretError::Vm(error)) => {
                let category = category_for_vm_error(error);
                emit_vm_error(&mut vm.io, error);
                vm.reset_tib();
                let code = finish_error(interactive, &mut vm.io, category, last_error);
                return if interactive { None } else { Some(code) };
            }
        }

        line_emitted_output |= token_may_emit_output;
    }

    vm.reset_tib();
    if interactive && vm.state == vm::InterpreterState::Interpreting {
        if line_emitted_output {
            vm.io.emit(b'\n');
        }
        emit_ok(&mut vm.io);
    }
    None
}

/// Skip source tokens until the end of a parenthesized comment
///
/// Stage-zero source comments are whitespace-tokenized, so `( comment )` is accepted while
/// `(comment)` remains an ordinary token. They are also line-local because the parser only scans
/// the current terminal input buffer contents; `( ... )` comments must open and close on the same
/// source line. An unterminated comment is malformed source code.
fn skip_parenthesized_comment(
    vm: &mut ForthVm<&mut impl ForthIo>,
    scratch: &mut [u8; TIB_SIZE],
) -> Result<(), VmError> {
    loop {
        match vm.next_tib_word(scratch)? {
            Some(token) if token == PAREN_COMMENT_END_WORD => return Ok(()),
            Some(_) => {}
            None => return Err(VmError::InvalidSource),
        }
    }
}

/// Interpret or compile one token according to the current interpreter state
///
/// This is the point where the outer interpreter chooses between immediate execution, compilation,
/// and numeric-literal handling.
fn interpret_token(
    vm: &mut ForthVm<&mut impl ForthIo>,
    token: &[u8],
) -> Result<Control, InterpretError> {
    if let Some(xt) = vm.find_word(token).map_err(InterpretError::Vm)? {
        let is_immediate = vm.word_is_immediate(xt).map_err(InterpretError::Vm)?;
        if vm.state == vm::InterpreterState::Interpreting || is_immediate {
            return vm.run_word(xt).map_err(InterpretError::Vm);
        }
        vm.compile_xt(xt).map_err(InterpretError::Vm)?;
        return Ok(Control::Continue);
    }

    let base = vm.validated_base().map_err(InterpretError::Vm)?;
    match parse_single_cell_number(token, base) {
        Ok(Some(value)) => compile_or_push_literal(vm, value).map_err(InterpretError::Vm),
        Ok(None) => Err(InterpretError::UnknownWord),
        Err(error) => Err(InterpretError::Vm(error)),
    }
}

/// Push or compile one numeric literal according to the current interpreter state
///
/// Numeric tokens need their own path so the source interpreter can accept plain literals without
/// requiring predeclared dictionary words for every constant.
fn compile_or_push_literal(
    vm: &mut ForthVm<&mut impl ForthIo>,
    value: Cell,
) -> Result<Control, VmError> {
    if vm.state == vm::InterpreterState::Interpreting {
        vm.push_data(value)?;
        return Ok(Control::Continue);
    }

    let lit_xt = vm
        .find_word(LIT_WORD_NAME)?
        .ok_or(VmError::InvalidDictionaryEntry)?;
    vm.compile_xt(lit_xt)?;
    vm.compile_cell(value)?;
    Ok(Control::Continue)
}

/// Parse one signed single-cell numeric literal token
///
/// Stage-zero follows the single-cell portion of classic Forth text interpreter behavior: a token
/// that is not found in the dictionary may be converted with the current `BASE`, with an optional
/// leading minus sign. Double-cell punctuation and floating-point input are intentionally deferred.
fn parse_single_cell_number(token: &[u8], base: u32) -> Result<Option<Cell>, VmError> {
    if token.is_empty() {
        return Ok(None);
    }

    let mut index = 0usize;
    let mut negative = false;
    if token[0] == b'-' {
        negative = true;
        index = 1;
    }

    if index == token.len() {
        return Err(VmError::InvalidNumber);
    }

    let mut value: i128 = 0;
    let mut saw_digit = false;
    for byte in &token[index..] {
        let Some(digit) = digit_value(*byte) else {
            return if saw_digit {
                Err(VmError::InvalidNumber)
            } else {
                Ok(None)
            };
        };
        if digit >= base {
            return if saw_digit || byte.is_ascii_digit() {
                Err(VmError::InvalidNumber)
            } else {
                Ok(None)
            };
        }
        saw_digit = true;
        value = value
            .checked_mul(i128::from(base))
            .and_then(|v| v.checked_add(i128::from(digit)))
            .ok_or(VmError::InvalidNumber)?;
    }

    if !saw_digit {
        return Ok(None);
    }

    if negative {
        value = -value;
    }

    Cell::try_from(value)
        .map(Some)
        .map_err(|_| VmError::InvalidNumber)
}

/// Convert one ASCII digit character to its numeric value for bases up to 36.
fn digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'A'..=b'Z' => Some(u32::from(byte - b'A') + 10),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 10),
        _ => None,
    }
}

/// Convert one VM error into a runner exit category
///
/// The VM reports detailed internal causes; the runner collapses them into stable external exit
/// classes that make sense to shell callers.
fn category_for_vm_error(error: VmError) -> ErrorCategory {
    match error {
        VmError::InvalidAddress | VmError::UnalignedCell => ErrorCategory::MemoryAccess,
        VmError::StackOverflow(vm::StackKind::Data)
        | VmError::StackUnderflow(vm::StackKind::Data) => ErrorCategory::DataStack,
        VmError::StackOverflow(vm::StackKind::Return)
        | VmError::StackUnderflow(vm::StackKind::Return) => ErrorCategory::ReturnStack,
        VmError::TibOverflow | VmError::DictionaryOverflow => ErrorCategory::DictionaryOrTib,
        VmError::EndOfInput | VmError::IoError => ErrorCategory::Io,
        VmError::InvalidDictionaryEntry | VmError::UnknownPrimitive => ErrorCategory::Internal,
        VmError::Abort | VmError::InvalidSource | VmError::InvalidNumber => {
            ErrorCategory::UnknownOrSyntax
        }
    }
}

/// Map one error category to the external Unix exit status
///
/// Keeping the mapping in one place makes the process contract explicit and stable.
fn exit_code(category: ErrorCategory) -> i32 {
    match category {
        ErrorCategory::UnknownOrSyntax => EXIT_UNKNOWN_OR_SYNTAX,
        ErrorCategory::DataStack => EXIT_DATA_STACK,
        ErrorCategory::ReturnStack => EXIT_RETURN_STACK,
        ErrorCategory::MemoryAccess => EXIT_MEMORY_ACCESS,
        ErrorCategory::DictionaryOrTib => EXIT_DICTIONARY_OR_TIB,
        ErrorCategory::Io => EXIT_IO,
        ErrorCategory::Internal => EXIT_INTERNAL,
    }
}

/// Record one error category and either continue interactively or return its batch exit code
///
/// Interactive use favors recovery and another prompt; batch use favors immediate failure, so pipes
/// and scripts stop at the first bad source line.
fn finish_error(
    interactive: bool,
    io: &mut impl ForthIo,
    category: ErrorCategory,
    last_error: &mut Option<ErrorCategory>,
) -> i32 {
    if interactive {
        emit_ok(io);
        EXIT_SUCCESS
    } else {
        *last_error = Some(category);
        exit_code(category)
    }
}

/// Record a fatal error category and return its external Unix exit status.
fn finish_fatal_error(category: ErrorCategory, last_error: &mut Option<ErrorCategory>) -> i32 {
    *last_error = Some(category);
    exit_code(category)
}

/// Emit the standard top-level prompt to stdout
///
/// The prompt is isolated here, so interactive-only output stays separate from batch execution.
fn emit_ok(io: &mut impl ForthIo) {
    emit_line(io, b"OK");
}

/// Emit one fixed diagnostic message to stderr followed by ` ?`
///
/// This preserves the conventional Forth diagnostic shape while keeping diagnostics off stdout.
fn emit_category_message(io: &mut impl ForthIo, message: &[u8]) {
    emit_error_bytes(io, message);
    emit_error_line(io, b" ?");
}

/// Emit the standard unknown-word diagnostic to stderr
///
/// Unknown words need the original token echoed back so source-driven failures stay actionable.
fn emit_unknown_word(io: &mut impl ForthIo, word: &[u8]) {
    emit_error_bytes(io, word);
    emit_error_line(io, b" ?");
}

/// Emit one byte slice exactly as provided to stdout
///
/// The runner keeps stdout emission primitive so higher-level helpers can compose prompts and output
/// without pulling formatting machinery into `no_std` code.
fn emit_bytes(io: &mut impl ForthIo, bytes: &[u8]) {
    for byte in bytes {
        io.emit(*byte);
    }
}

/// Emit one byte slice followed by a newline to stdout
///
/// This exists to keep newline policy consistent across prompts and normal text output.
fn emit_line(io: &mut impl ForthIo, line: &[u8]) {
    emit_bytes(io, line);
    io.emit(b'\n');
}

/// Emit one byte slice exactly as provided to stderr
///
/// Diagnostics use a separate helper, so stderr routing is impossible to confuse with normal
/// output.
fn emit_error_bytes(io: &mut impl ForthIo, bytes: &[u8]) {
    for byte in bytes {
        io.emit_error(*byte);
    }
}

/// Emit one byte slice followed by a newline to stderr
///
/// This keeps diagnostics uniform and avoids repeating byte-at-a-time stderr loops.
fn emit_error_line(io: &mut impl ForthIo, line: &[u8]) {
    emit_error_bytes(io, line);
    io.emit_error(b'\n');
}

/// Translate one checked virtual-machine error into stderr output
///
/// VM errors carry internal names; this function turns them into the user-facing diagnostic strings
/// that batch scripts and interactive users actually see.
fn emit_vm_error(io: &mut impl ForthIo, error: VmError) {
    let message = match error {
        VmError::InvalidAddress => b"invalid-address".as_slice(),
        VmError::UnalignedCell => b"unaligned-cell",
        VmError::StackOverflow(vm::StackKind::Data) => b"data-stack-overflow",
        VmError::StackOverflow(vm::StackKind::Return) => b"return-stack-overflow",
        VmError::StackUnderflow(vm::StackKind::Data) => b"data-stack-underflow",
        VmError::StackUnderflow(vm::StackKind::Return) => b"return-stack-underflow",
        VmError::TibOverflow => b"tib-overflow",
        VmError::DictionaryOverflow => b"dictionary-overflow",
        VmError::InvalidDictionaryEntry => b"invalid-dictionary-entry",
        VmError::UnknownPrimitive => b"unknown-primitive",
        VmError::EndOfInput => b"end-of-input",
        VmError::IoError => b"io-error",
        VmError::InvalidSource => b"invalid-source",
        VmError::InvalidNumber => b"invalid-number",
        VmError::Abort => b"abort",
    };
    emit_category_message(io, message);
}
