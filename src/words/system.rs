//! System variable words
//!
//! These words expose VM-resident interpreter state through ordinary Forth address semantics.

use crate::{
    io::ForthIo,
    vm::{BASE_ADDRESS, Cell, ForthVm, VmError},
};

use super::{Control, Primitive};

/// Dictionary name for the number-conversion radix variable.
const BASE_NAME: &str = "BASE";

/// Install the stage-zero system variable words in dictionary order.
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(BASE_NAME, Primitive::Base, 0)?;
    Ok(())
}

/// Execute `BASE` by pushing the address of the current radix cell.
pub(crate) fn execute_base<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    vm.push_data(Cell::from(BASE_ADDRESS))?;
    Ok(Control::Continue)
}
