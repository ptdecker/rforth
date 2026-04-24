//! Inner-interpreter words.
//!
//! These words implement the stage-zero indirect-threaded execution model.

use crate::{
    io::ForthIo,
    vm::{Address, CELL_SIZE, ForthVm, VmError},
};

use super::{Control, Primitive};

const NEXT_NAME: &str = "NEXT";
const DOCOL_NAME: &str = "DOCOL";
const DOSEMI_NAME: &str = "DOSEMI";
const LIT_NAME: &str = "LIT";

/// Install the stage-zero inner-interpreter words in dependency order.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(NEXT_NAME, Primitive::Next, 0)?;
    vm.install_primitive_word(DOCOL_NAME, Primitive::Docol, 0)?;
    vm.install_primitive_word(DOSEMI_NAME, Primitive::Dosemi, 0)?;
    vm.install_primitive_word(LIT_NAME, Primitive::Lit, 0)?;
    Ok(())
}

/// Execute `NEXT` by fetching and dispatching the next execution token from threaded code.
pub(crate) fn execute_next<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.next()
}

/// Execute `DOCOL` by saving the current instruction pointer and entering the parameter field.
pub(crate) fn execute_docol<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let entry = vm.dictionary_entry(vm.w)?;
    vm.push_return(i64::from(vm.ip))?;
    vm.ip = entry.pfa;
    Ok(Control::Continue)
}

/// Execute `DOSEMI` by restoring the saved instruction pointer from the return stack.
pub(crate) fn execute_dosemi<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let restored_ip = vm.pop_return()?;
    vm.ip = Address::try_from(restored_ip).map_err(|_| VmError::InvalidAddress)?;
    Ok(Control::Continue)
}

/// Execute `LIT` by pushing the inline literal cell that follows it in threaded code.
pub(crate) fn execute_lit<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let literal = vm.read_cell(vm.ip)?;
    vm.ip = vm
        .ip
        .checked_add(CELL_SIZE as Address)
        .ok_or(VmError::InvalidAddress)?;
    vm.push_data(literal)?;
    Ok(Control::Continue)
}
