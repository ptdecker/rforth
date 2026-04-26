//! Control-flow and outer-interpreter words
//!
//! This module holds stage-zero words that alter threaded execution or transfer control back to the
//! top-level interpreter boundary.

use crate::{
    io::ForthIo,
    vm::{Address, CELL_SIZE, ForthVm, NO_ADDRESS, VmError},
};

use super::{Control, Primitive};

/// Dictionary name for the unconditional branch primitive.
const BRANCH_NAME: &str = "BRANCH";

/// Dictionary name for the zero-conditional branch primitive.
const ZERO_BRANCH_NAME: &str = "0BRANCH";

/// Dictionary name for the top-level interpreter reset primitive.
const QUIT_NAME: &str = "QUIT";

/// Dictionary name for the host-exit primitive.
const BYE_NAME: &str = "BYE";

/// Dictionary name for the unconditional source-abort primitive.
const ABORT_NAME: &str = "ABORT";

/// Dictionary name for the zero-flag source-abort primitive.
const QUESTION_ABORT_NAME: &str = "?ABORT";

/// Install the stage-zero control words in dictionary order.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(BRANCH_NAME, Primitive::Branch, 0)?;
    vm.install_primitive_word(ZERO_BRANCH_NAME, Primitive::ZeroBranch, 0)?;
    vm.install_primitive_word(QUIT_NAME, Primitive::Quit, 0)?;
    vm.install_primitive_word(BYE_NAME, Primitive::Bye, 0)?;
    vm.install_primitive_word(ABORT_NAME, Primitive::Abort, 0)?;
    vm.install_primitive_word(QUESTION_ABORT_NAME, Primitive::QuestionAbort, 0)?;
    Ok(())
}

/// Execute `BRANCH` by loading the inline branch target into the instruction pointer.
pub(crate) fn execute_branch<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.instruction_pointer = vm.read_address(vm.instruction_pointer)?;
    Ok(Control::Continue)
}

/// Execute `0BRANCH` by branching only when the popped flag is zero.
pub(crate) fn execute_zero_branch<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.instruction_pointer = if vm.pop_data()? == 0 {
        vm.read_address(vm.instruction_pointer)?
    } else {
        vm.instruction_pointer
            .checked_add(CELL_SIZE as Address)
            .ok_or(VmError::InvalidAddress)?
    };
    Ok(Control::Continue)
}

/// Execute `QUIT` by resetting the outer interpreter and requesting a top-level restart.
pub(crate) fn execute_quit<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.reset_outer_interpreter_state();
    Ok(Control::Quit)
}

/// Execute `BYE` by requesting interpreter termination at the host boundary.
pub(crate) fn execute_bye<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.current_definition = NO_ADDRESS;
    vm.instruction_pointer = NO_ADDRESS;
    vm.working_register = NO_ADDRESS;
    Ok(Control::Bye)
}

/// Execute `ABORT` by resetting the interpreter and requesting a failing exit at the host
/// boundary.
pub(crate) fn execute_abort<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.reset_outer_interpreter_state();
    Ok(Control::Abort)
}

/// Execute `?ABORT` by aborting when the popped flag is zero.
pub(crate) fn execute_question_abort<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    if vm.pop_data()? == 0 {
        return execute_abort(vm);
    }
    Ok(Control::Continue)
}
