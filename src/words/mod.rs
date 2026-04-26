//! Stage-zero Forth words grouped by behavior
//!
//! This module installs the classic irreducible stage-zero nucleus: the inner interpreter words,
//! minimal control words including `QUIT` and `BYE`, core stack and memory words, and the basic
//! input/output boundary words.

use crate::{
    io::ForthIo,
    vm::{Cell, ForthVm, VmError},
};

/// Arithmetic and comparison words used by the first source-driven REPL
pub mod arithmetic;
/// Source-compiler words such as `:` and `;`
pub mod compiler;
/// Control-flow and outer-interpreter words such as `QUIT` and `BYE`
pub mod control;
/// Inner-interpreter words such as `NEXT`, `DOCOL`, and `DOSEMI`
pub mod inner;
/// Input/output boundary words such as `KEY` and `EMIT`
pub mod io;
/// Memory access words such as `@`, `!`, `C@`, and `C!`
pub mod memory;
/// Data-stack words such as `DUP`, `DROP`, and `SWAP`
pub mod stack;
/// System variable words such as `BASE`
pub mod system;

/// Control the outcome produced by a primitive or threaded execution step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Continue normal threaded or outer-interpreter execution.
    Continue,
    /// Reset to the top-level outer interpreter.
    Quit,
    /// Terminate the outer interpreter and return to the host.
    Bye,
    /// Terminate the current run because a source-level abort was requested.
    Abort,
}

/// Primitive handler identifiers stored in dictionary code fields
///
/// Rust executes a primitive handler directly when its word runs. Colon definitions instead store
/// the dedicated `DOCOL` marker in their code field address (CFA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Primitive {
    /// Advance the threaded interpreter by fetching and executing the next word.
    Next = 1,
    /// Enter a colon definition by saving the current instruction pointer.
    Docol = 2,
    /// Return from a colon definition by restoring the saved instruction pointer.
    Dosemi = 3,
    /// Push the next inline literal cell onto the data stack.
    Lit = 4,
    /// Branch unconditionally to an inline threaded-code target.
    Branch = 5,
    /// Branch to an inline threaded-code target when the popped flag is zero.
    ZeroBranch = 6,
    /// Reset the outer-interpreter state to its top-level entry condition.
    Quit = 7,
    /// Exit the outer interpreter completely.
    Bye = 8,
    /// Duplicate the top data-stack cell.
    Dup = 9,
    /// Drop the top data-stack cell.
    Drop = 10,
    /// Exchange the top two data-stack cells.
    Swap = 11,
    /// Fetch one cell from virtual memory.
    Fetch = 12,
    /// Store one cell into virtual memory.
    Store = 13,
    /// Fetch one memory word from virtual memory.
    CFetch = 14,
    /// Store one memory word into virtual memory.
    CStore = 15,
    /// Read one character from the configured input backend.
    Key = 16,
    /// Write one character to the configured output backend.
    Emit = 17,
    /// Begin compiling a colon definition from source input.
    Colon = 18,
    /// Finish compiling a colon definition.
    Semicolon = 19,
    /// Add the top two stack cells.
    Add = 20,
    /// Subtract the top stack cell from the next cell.
    Subtract = 21,
    /// Compare the top two stack cells for equality.
    Equal = 22,
    /// Test whether the top stack cell is zero.
    ZeroEqual = 23,
    /// Copy the second stack cell to the top of the stack.
    Over = 24,
    /// Abort the current run.
    Abort = 25,
    /// Print the top stack cell in the current base followed by a space.
    Dot = 26,
    /// Abort the current run when the top stack flag is zero.
    QuestionAbort = 27,
    /// Push the address of the current number-conversion radix cell.
    Base = 28,
}

impl Primitive {
    /// Return the encoded code-field value used for this primitive.
    pub const fn code_field(self) -> Cell {
        self as Cell
    }

    /// Decode a primitive from a stored code-field value.
    pub const fn from_code_field(code_field: Cell) -> Option<Self> {
        match code_field {
            1 => Some(Self::Next),
            2 => Some(Self::Docol),
            3 => Some(Self::Dosemi),
            4 => Some(Self::Lit),
            5 => Some(Self::Branch),
            6 => Some(Self::ZeroBranch),
            7 => Some(Self::Quit),
            8 => Some(Self::Bye),
            9 => Some(Self::Dup),
            10 => Some(Self::Drop),
            11 => Some(Self::Swap),
            12 => Some(Self::Fetch),
            13 => Some(Self::Store),
            14 => Some(Self::CFetch),
            15 => Some(Self::CStore),
            16 => Some(Self::Key),
            17 => Some(Self::Emit),
            18 => Some(Self::Colon),
            19 => Some(Self::Semicolon),
            20 => Some(Self::Add),
            21 => Some(Self::Subtract),
            22 => Some(Self::Equal),
            23 => Some(Self::ZeroEqual),
            24 => Some(Self::Over),
            25 => Some(Self::Abort),
            26 => Some(Self::Dot),
            27 => Some(Self::QuestionAbort),
            28 => Some(Self::Base),
            _ => None,
        }
    }

    /// Execute this primitive against the supplied virtual machine.
    pub fn execute<I: ForthIo>(self, vm: &mut ForthVm<I>) -> Result<Control, VmError> {
        match self {
            Self::Next => inner::execute_next(vm),
            Self::Docol => inner::execute_docol(vm),
            Self::Dosemi => inner::execute_dosemi(vm),
            Self::Lit => inner::execute_lit(vm),
            Self::Branch => control::execute_branch(vm),
            Self::ZeroBranch => control::execute_zero_branch(vm),
            Self::Quit => control::execute_quit(vm),
            Self::Bye => control::execute_bye(vm),
            Self::Dup => stack::execute_dup(vm),
            Self::Drop => stack::execute_drop(vm),
            Self::Swap => stack::execute_swap(vm),
            Self::Fetch => memory::execute_fetch(vm),
            Self::Store => memory::execute_store(vm),
            Self::CFetch => memory::execute_c_fetch(vm),
            Self::CStore => memory::execute_c_store(vm),
            Self::Key => io::execute_key(vm),
            Self::Emit => io::execute_emit(vm),
            Self::Colon => compiler::execute_colon(vm),
            Self::Semicolon => compiler::execute_semicolon(vm),
            Self::Add => arithmetic::execute_add(vm),
            Self::Subtract => arithmetic::execute_subtract(vm),
            Self::Equal => arithmetic::execute_equal(vm),
            Self::ZeroEqual => arithmetic::execute_zero_equal(vm),
            Self::Over => stack::execute_over(vm),
            Self::Abort => control::execute_abort(vm),
            Self::Dot => io::execute_dot(vm),
            Self::QuestionAbort => control::execute_question_abort(vm),
            Self::Base => system::execute_base(vm),
        }
    }
}

/// Install the classic irreducible stage-zero word set into the dictionary.
pub fn install_stage_zero<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    inner::install(vm)?;
    control::install(vm)?;
    compiler::install(vm)?;
    stack::install(vm)?;
    arithmetic::install(vm)?;
    system::install(vm)?;
    memory::install(vm)?;
    io::install(vm)?;
    Ok(())
}
