//! Input/output boundary words.
//!
//! These words route stage-zero console input/output through the active VM device model.

use crate::{
    io::ForthIo,
    vm::{Address, Cell, ForthVm, VmError},
};

use super::{Control, Primitive};

const KEY_NAME: &str = "KEY";
const EMIT_NAME: &str = "EMIT";

#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
const ACTIVE_KEY_ADDRESS: Address = crate::vm::DIRECT_MMIO_KEY_ADDR;
#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
const ACTIVE_EMIT_ADDRESS: Address = crate::vm::DIRECT_MMIO_EMIT_ADDR;

#[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
const ACTIVE_KEY_ADDRESS: Address = crate::vm::UART_RBR_THR_ADDR;
#[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
const ACTIVE_EMIT_ADDRESS: Address = crate::vm::UART_RBR_THR_ADDR;

#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
const ACTIVE_KEY_PORT: Address = crate::vm::DIRECT_KEY_PORT;
#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
const ACTIVE_EMIT_PORT: Address = crate::vm::DIRECT_EMIT_PORT;

#[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
const ACTIVE_KEY_PORT: Address = crate::vm::UART_RBR_THR_PORT;
#[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
const ACTIVE_EMIT_PORT: Address = crate::vm::UART_RBR_THR_PORT;

/// Install the stage-zero input/output words in dictionary order.
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(KEY_NAME, Primitive::Key, 0)?;
    vm.install_primitive_word(EMIT_NAME, Primitive::Emit, 0)?;
    Ok(())
}

/// Execute `KEY` by reading one character through the active VM input model.
pub(crate) fn execute_key<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    #[cfg(feature = "vm-port-io")]
    let value = vm.port_in(ACTIVE_KEY_PORT);

    #[cfg(not(feature = "vm-port-io"))]
    let value = vm.read_cell(ACTIVE_KEY_ADDRESS)?;

    vm.push_data(value)?;
    Ok(Control::Continue)
}

/// Execute `EMIT` by writing the low byte of the top cell through the active VM output model.
pub(crate) fn execute_emit<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let value = vm.pop_data()?;
    write_output(vm, value);
    Ok(Control::Continue)
}

/// Write one output cell through the active port-based device model.
#[cfg(feature = "vm-port-io")]
fn write_output<I: ForthIo>(vm: &mut ForthVm<I>, value: Cell) {
    vm.port_out(ACTIVE_EMIT_PORT, value);
}

/// Write one output cell through the active memory-mapped device model.
#[cfg(not(feature = "vm-port-io"))]
fn write_output<I: ForthIo>(vm: &mut ForthVm<I>, value: Cell) {
    let _ = vm.write_cell(ACTIVE_EMIT_ADDRESS, value);
}
