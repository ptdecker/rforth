//! Control-flow and outer-interpreter words.
//!
//! This module holds stage-zero words that alter threaded execution or transfer control back to the
//! top-level interpreter boundary.

use crate::{
    io::ForthIo,
    vm::{Address, CELL_SIZE, ForthVm, InterpreterState, NO_ADDRESS, VmError},
};

use super::{Control, Primitive};

const BRANCH_NAME: &str = "BRANCH";
const ZERO_BRANCH_NAME: &str = "0BRANCH";
const QUIT_NAME: &str = "QUIT";
const BYE_NAME: &str = "BYE";

/// Install the stage-zero control words in dictionary order.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(BRANCH_NAME, Primitive::Branch, 0)?;
    vm.install_primitive_word(ZERO_BRANCH_NAME, Primitive::ZeroBranch, 0)?;
    vm.install_primitive_word(QUIT_NAME, Primitive::Quit, 0)?;
    vm.install_primitive_word(BYE_NAME, Primitive::Bye, 0)?;
    Ok(())
}

/// Execute `BRANCH` by loading the inline branch target into the instruction pointer.
pub(crate) fn execute_branch<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.ip = vm.read_address(vm.ip)?;
    Ok(Control::Continue)
}

/// Execute `0BRANCH` by branching only when the popped flag is zero.
pub(crate) fn execute_zero_branch<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.ip = if vm.pop_data()? == 0 {
        vm.read_address(vm.ip)?
    } else {
        vm.ip
            .checked_add(CELL_SIZE as Address)
            .ok_or(VmError::InvalidAddress)?
    };
    Ok(Control::Continue)
}

/// Execute `QUIT` by resetting the outer interpreter and requesting a top-level restart.
pub(crate) fn execute_quit<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.reset_return_stack();
    vm.reset_tib();
    vm.state = InterpreterState::Interpreting;
    vm.input_pos = 0;
    vm.ip = NO_ADDRESS;
    vm.w = NO_ADDRESS;
    Ok(Control::Quit)
}

/// Execute `BYE` by requesting interpreter termination at the host boundary.
pub(crate) fn execute_bye<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.ip = NO_ADDRESS;
    vm.w = NO_ADDRESS;
    Ok(Control::Bye)
}
