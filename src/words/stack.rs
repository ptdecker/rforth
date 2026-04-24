//! Data-stack manipulation words.
//!
//! These stage-zero words operate directly on the VM-resident data stack.

use crate::{
    io::ForthIo,
    vm::{ForthVm, VmError},
};

use super::{Control, Primitive};

const DUP_NAME: &str = "DUP";
const DROP_NAME: &str = "DROP";
const SWAP_NAME: &str = "SWAP";

/// Install the stage-zero stack words in dictionary order.
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(DUP_NAME, Primitive::Dup, 0)?;
    vm.install_primitive_word(DROP_NAME, Primitive::Drop, 0)?;
    vm.install_primitive_word(SWAP_NAME, Primitive::Swap, 0)?;
    Ok(())
}

/// Execute `DUP` by copying the current top-of-stack cell.
pub(crate) fn execute_dup<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let value = vm.peek_data()?;
    vm.push_data(value)?;
    Ok(Control::Continue)
}

/// Execute `DROP` by discarding the current top-of-stack cell.
pub(crate) fn execute_drop<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let _ = vm.pop_data()?;
    Ok(Control::Continue)
}

/// Execute `SWAP` by exchanging the top two data-stack cells.
pub(crate) fn execute_swap<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let top = vm.pop_data()?;
    let next = vm.pop_data()?;
    vm.push_data(top)?;
    vm.push_data(next)?;
    Ok(Control::Continue)
}
