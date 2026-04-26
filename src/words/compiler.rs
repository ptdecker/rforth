//! Source-compiler words
//!
//! These words bridge source parsing and threaded execution by creating and closing colon
//! definitions whose runtime entry behavior is `DOCOL`.

use crate::{
    io::ForthIo,
    vm::{ForthVm, VmError, WORD_FLAG_IMMEDIATE},
};

use super::{Control, Primitive};

/// Dictionary name for the colon-definition start primitive.
const COLON_NAME: &str = ":";

/// Dictionary name for the colon-definition finish primitive.
const SEMICOLON_NAME: &str = ";";

/// Dictionary lookup token for the runtime colon-definition return primitive.
const DOSEMI_NAME: &[u8] = b"DOSEMI";

/// Install the source-compiler words in dictionary order.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(COLON_NAME, Primitive::Colon, WORD_FLAG_IMMEDIATE)?;
    vm.install_primitive_word(SEMICOLON_NAME, Primitive::Semicolon, WORD_FLAG_IMMEDIATE)?;
    Ok(())
}

/// Execute `:` by reading the next source token as the new definition name and creating a colon
/// definition whose code field dispatches through `DOCOL`.
pub(crate) fn execute_colon<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let mut scratch = [0u8; crate::vm::TIB_SIZE];
    let Some(name) = vm.next_tib_word(&mut scratch)? else {
        return Err(VmError::InvalidSource);
    };
    vm.begin_colon_definition(name)?;
    Ok(Control::Continue)
}

/// Execute `;` by compiling `DOSEMI`, finalizing the open definition, and returning to interpret
/// state.
pub(crate) fn execute_semicolon<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    if vm.state != crate::vm::InterpreterState::Compiling {
        return Err(VmError::InvalidSource);
    }
    let dosemi_xt = vm
        .find_word(DOSEMI_NAME)?
        .ok_or(VmError::InvalidDictionaryEntry)?;
    vm.compile_xt(dosemi_xt)?;
    let _ = vm.finish_colon_definition()?;
    Ok(Control::Continue)
}
