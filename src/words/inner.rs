//! Inner-interpreter words
//!
//! These words implement the stage-zero indirect-threaded execution model.

use crate::{
    io::ForthIo,
    vm::{Address, CELL_SIZE, ForthVm, VmError},
};

use super::{Control, Primitive};

/// Dictionary name for the threaded-code dispatcher primitive.
const NEXT_NAME: &str = "NEXT";

/// Dictionary name for the colon-definition entry primitive.
const DOCOL_NAME: &str = "DOCOL";

/// Dictionary name for the colon-definition return primitive.
const DOSEMI_NAME: &str = "DOSEMI";

/// Dictionary name for the inline literal primitive.
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
    vm.execute_next_threaded_word()
}

/// Execute `DOCOL` by saving the current instruction pointer and entering the parameter field.
pub(crate) fn execute_docol<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let entry = vm.dictionary_entry(vm.working_register)?;
    vm.push_return(i64::from(vm.instruction_pointer))?;
    vm.instruction_pointer = entry.parameter_field_address;
    Ok(Control::Continue)
}

/// Execute `DOSEMI` by restoring the saved instruction pointer from the return stack.
pub(crate) fn execute_dosemi<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.instruction_pointer =
        Address::try_from(vm.pop_return()?).map_err(|_| VmError::InvalidAddress)?;
    Ok(Control::Continue)
}

/// Execute `LIT` by pushing the inline literal cell that follows it in threaded code.
pub(crate) fn execute_lit<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let literal = vm.read_cell(vm.instruction_pointer)?;
    vm.instruction_pointer = vm
        .instruction_pointer
        .checked_add(CELL_SIZE as Address)
        .ok_or(VmError::InvalidAddress)?;
    vm.push_data(literal)?;
    Ok(Control::Continue)
}
