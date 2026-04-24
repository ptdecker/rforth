#![no_std]

//! `rforth` — a minimal, dependency-free Forth interpreter.
//!
//! The interpreter core (`run_forth`) is platform-agnostic and communicates with the outside
//! world exclusively through the `ForthIo` trait. Platform selection happens at compile time
//! via `cfg` attributes and the `embedded` Cargo feature flag; see the `io` and `sys` modules for
//! the concrete implementations.
//!
//! Terminal input is gathered directly into the VM terminal input buffer (TIB). Device input still
//! arrives one character at a time, but the outer interpreter no longer keeps a separate host-side
//! line buffer.

pub mod io;
pub mod sys;
pub mod tokenizer;
pub mod vm;
pub mod words;

use io::ForthIo;
use vm::{ForthVm, TIB_SIZE, VmError};
use words::Control;

/// Run the Forth interpreter, using `io` for all character-level I/O.
///
/// This function is platform-agnostic: it never touches file descriptors, terminal state, or any OS
/// primitive directly. All such concerns are encapsulated in the [`ForthIo`] implementation that
/// is passed in.
///
/// The current runner prints `OK\n`, reads a line of input, tokenizes it, looks up each token in
/// the VM dictionary, and executes recognized words. Unknown tokens and VM execution failures are
/// reported to the terminal. The function returns `0` when the user executes `BYE`.
pub fn run_forth(io: &mut impl ForthIo) -> i32 {
    run_forth_loop(io)
}

/// Run the top-level read-evaluate-print loop until `BYE` requests termination.
fn run_forth_loop(io: &mut impl ForthIo) -> i32 {
    let mut vm = ForthVm::new(io);
    words::install_stage_zero(&mut vm).expect("stage-zero words should install into the VM");
    emit_ok(&mut vm.io);
    let mut line_overflowed = false;

    loop {
        let control = process_key(&mut vm, &mut line_overflowed);
        if control == Control::Bye {
            return 0;
        }
    }
}

/// Run the interpreter for a fixed number of input bytes.
///
/// This uses the same loop body as [`run_forth`], but returns after `keys_to_read` top-level input
/// iterations. Executed words such as `KEY` may perform additional [`ForthIo::key`] calls. It is
/// intended for host-side tests and scripted harnesses.
pub fn run_forth_steps(io: &mut impl ForthIo, keys_to_read: usize) {
    let mut vm = ForthVm::new(io);
    words::install_stage_zero(&mut vm).expect("stage-zero words should install into the VM");
    emit_ok(&mut vm.io);
    let mut line_overflowed = false;

    for _ in 0..keys_to_read {
        let control = process_key(&mut vm, &mut line_overflowed);
        if control == Control::Bye {
            break;
        }
    }
}

/// Process one input byte and return any control transfer requested by line execution.
///
/// Completed lines are accumulated directly in the VM terminal input buffer. When the active line
/// overflows that buffer, additional non-newline characters are ignored without echo until the line
/// terminator arrives and reports the overflow.
fn process_key(vm: &mut ForthVm<&mut impl ForthIo>, line_overflowed: &mut bool) -> Control {
    let c = vm.io.key();

    if c == b'\r' || c == b'\n' {
        vm.io.emit(c);
        if c == b'\r' {
            vm.io.emit(b'\n');
        }

        if *line_overflowed {
            emit_bytes(&mut vm.io, b"tib-overflow");
            emit_line(&mut vm.io, b" ?");
            vm.reset_tib();
            *line_overflowed = false;
            return Control::Continue;
        }

        return process_line(vm);
    }

    if *line_overflowed {
        return Control::Continue;
    }

    match vm.append_tib_byte(c) {
        Ok(()) => vm.io.emit(c),
        Err(VmError::TibOverflow) => *line_overflowed = true,
        Err(error) => {
            emit_vm_error(&mut vm.io, error);
            vm.reset_tib();
        }
    }

    Control::Continue
}

/// Execute one completed input line through dictionary lookup and stage-zero word dispatch.
fn process_line(vm: &mut ForthVm<&mut impl ForthIo>) -> Control {
    let mut scratch = [0u8; TIB_SIZE];
    vm.input_pos = 0;

    loop {
        let word = match vm.next_tib_word(&mut scratch) {
            Ok(Some(word)) => word,
            Ok(None) => break,
            Err(error) => {
                emit_vm_error(&mut vm.io, error);
                vm.reset_tib();
                return Control::Continue;
            }
        };

        match vm.find_word(word) {
            Ok(Some(xt)) => {
                let control = match vm.run_word(xt) {
                    Ok(control) => control,
                    Err(error) => {
                        emit_vm_error(&mut vm.io, error);
                        vm.reset_tib();
                        return Control::Continue;
                    }
                };
                match control {
                    Control::Continue => {}
                    Control::Quit => {
                        vm.reset_tib();
                        emit_ok(&mut vm.io);
                        return Control::Continue;
                    }
                    Control::Bye => return Control::Bye,
                }
            }
            Ok(None) => {
                emit_unknown_word(&mut vm.io, word);
                vm.reset_tib();
                return Control::Continue;
            }
            Err(error) => {
                emit_vm_error(&mut vm.io, error);
                vm.reset_tib();
                return Control::Continue;
            }
        }
    }

    vm.reset_tib();
    emit_ok(&mut vm.io);
    Control::Continue
}

/// Emit the standard top-level prompt.
fn emit_ok(io: &mut impl ForthIo) {
    emit_line(io, b"OK");
}

/// Emit the standard unknown-word diagnostic for one token.
fn emit_unknown_word(io: &mut impl ForthIo, word: &[u8]) {
    emit_bytes(io, word);
    emit_line(io, b" ?");
}

/// Emit one byte slice exactly as provided.
fn emit_bytes(io: &mut impl ForthIo, bytes: &[u8]) {
    for byte in bytes {
        io.emit(*byte);
    }
}

/// Emit one byte slice followed by a newline.
fn emit_line(io: &mut impl ForthIo, line: &[u8]) {
    emit_bytes(io, line);
    io.emit(b'\n');
}

/// Translate one checked virtual-machine error into the outer interpreter's textual error form.
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
    };
    emit_bytes(io, message);
    emit_line(io, b" ?");
}
