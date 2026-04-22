#![no_std]

//! `rforth` — a minimal, dependency-free Forth interpreter.
//!
//! The interpreter core (`run_forth`) is platform-agnostic and communicates with the outside
//! world exclusively through the `ForthIo` trait. Platform selection happens at compile time
//! via `cfg` attributes and the `embedded` Cargo feature flag; see the `io` and `sys` modules for
//! the concrete implementations.

pub mod io;
pub mod sys;
pub mod tokenizer;
pub mod vm;

use io::ForthIo;
use vm::ForthVm;

const MAX_LINE_BYTES: usize = 128;
const MAX_WORDS: usize = 32;

/// Run the Forth interpreter, using `io` for all character-level I/O
///
/// This function is platform-agnostic: it never touches file descriptors, terminal state, or any OS
/// primitive directly. All such concerns are encapsulated in the [`ForthIo`] implementation that
/// is passed in.
///
/// Currently, the interpreter prints `OK\n`, reads a line of input, tokenizes it into words, and
/// prints those words back as a vector. This is early scaffolding; a full Forth engine will be
/// built on top of this loop.
pub fn run_forth(io: &mut impl ForthIo) -> ! {
    io.emit(b'O');
    io.emit(b'K');
    io.emit(b'\n');

    run_forth_loop(io)
}

fn run_forth_loop(io: &mut impl ForthIo) -> ! {
    let mut vm = ForthVm::new(io);
    let mut line = [0u8; MAX_LINE_BYTES];
    let mut line_len = 0;

    loop {
        process_key(&mut vm.io, &mut line, &mut line_len);
    }
}

/// Run the interpreter for a fixed number of input bytes.
///
/// This uses the same loop body as [`run_forth`], but returns after `keys_to_read` calls to
/// [`ForthIo::key`]. It is intended for host-side tests and scripted harnesses.
pub fn run_forth_steps(io: &mut impl ForthIo, keys_to_read: usize) {
    io.emit(b'O');
    io.emit(b'K');
    io.emit(b'\n');

    let mut vm = ForthVm::new(io);
    let mut line = [0u8; MAX_LINE_BYTES];
    let mut line_len = 0;

    for _ in 0..keys_to_read {
        process_key(&mut vm.io, &mut line, &mut line_len);
    }
}

fn process_key(io: &mut impl ForthIo, line: &mut [u8; MAX_LINE_BYTES], line_len: &mut usize) {
    let c = io.key();
    io.emit(c);

    if c == b'\r' || c == b'\n' {
        if c == b'\r' {
            io.emit(b'\n');
        }
        output_words(io, &line[..*line_len]);
        *line_len = 0;
    } else if *line_len < line.len() {
        line[*line_len] = c;
        *line_len += 1;
    }
}

fn output_words(io: &mut impl ForthIo, line: &[u8]) {
    let words = tokenizer::parse_words::<MAX_WORDS>(line);

    io.emit(b'[');
    for (index, word) in words.as_slice().iter().enumerate() {
        if index != 0 {
            io.emit(b',');
            io.emit(b' ');
        }
        for c in word.iter() {
            io.emit(*c);
        }
    }
    io.emit(b']');
    io.emit(b'\n');
}
