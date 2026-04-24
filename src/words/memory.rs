//! Memory access words.
//!
//! These stage-zero words expose byte and cell fetch/store operations over the VM address space.

use crate::{
    io::ForthIo,
    vm::{Address, ForthVm, MemoryWord, VmError},
};

use super::{Control, Primitive};

const FETCH_NAME: &str = "@";
const STORE_NAME: &str = "!";
const C_FETCH_NAME: &str = "C@";
const C_STORE_NAME: &str = "C!";

/// Install the stage-zero memory words in dictionary order.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(FETCH_NAME, Primitive::Fetch, 0)?;
    vm.install_primitive_word(STORE_NAME, Primitive::Store, 0)?;
    vm.install_primitive_word(C_FETCH_NAME, Primitive::CFetch, 0)?;
    vm.install_primitive_word(C_STORE_NAME, Primitive::CStore, 0)?;
    Ok(())
}

/// Execute `@` by fetching one cell from the address on the data stack.
pub(crate) fn execute_fetch<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let address = pop_address(vm)?;
    let value = vm.read_cell(address)?;
    vm.push_data(value)?;
    Ok(Control::Continue)
}

/// Execute `!` by storing one cell to the address on the data stack.
pub(crate) fn execute_store<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let address = pop_address(vm)?;
    let value = vm.pop_data()?;
    vm.write_cell(address, value)?;
    Ok(Control::Continue)
}

/// Execute `C@` by fetching one memory word from the address on the data stack.
pub(crate) fn execute_c_fetch<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let address = pop_address(vm)?;
    let value = vm.read_memory_word(address)?;
    vm.push_data(i64::from(value))?;
    Ok(Control::Continue)
}

/// Execute `C!` by storing one memory word to the address on the data stack.
pub(crate) fn execute_c_store<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let address = pop_address(vm)?;
    let value = vm.pop_data()?;
    vm.write_memory_word(address, value as MemoryWord)?;
    Ok(Control::Continue)
}

/// Pop one address value from the data stack and validate it as a VM-visible address.
fn pop_address<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Address, VmError> {
    let raw = vm.pop_data()?;
    Address::try_from(raw).map_err(|_| VmError::InvalidAddress)
}
