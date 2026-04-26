//! Input/output boundary words
//!
//! These words route stage-zero console input/output through the active VM device model.

use crate::{
    io::ForthIo,
    vm::{self, Address, Cell, ForthVm, VmError},
};

use super::{Control, Primitive};

/// Dictionary name for the input-byte primitive.
const KEY_NAME: &str = "KEY";

/// Dictionary name for the output-byte primitive.
const EMIT_NAME: &str = "EMIT";

/// Dictionary name for the signed decimal output primitive.
const DOT_NAME: &str = ".";

#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
/// Active memory-mapped input address for the direct I/O backend.
const ACTIVE_KEY_ADDRESS: Address = vm::DIRECT_MMIO_KEY_ADDR;
#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
/// Active memory-mapped output address for the direct I/O backend.
const ACTIVE_EMIT_ADDRESS: Address = vm::DIRECT_MMIO_EMIT_ADDR;

#[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
/// Active memory-mapped input address for the UART-backed I/O backend.
const ACTIVE_KEY_ADDRESS: Address = vm::UART_RBR_THR_ADDR;
#[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
/// Active memory-mapped output address for the UART-backed I/O backend.
const ACTIVE_EMIT_ADDRESS: Address = vm::UART_RBR_THR_ADDR;

#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
/// Active input port for the direct port-I/O backend.
const ACTIVE_KEY_PORT: Address = vm::DIRECT_KEY_PORT;
#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
/// Active output port for the direct port-I/O backend.
const ACTIVE_EMIT_PORT: Address = vm::DIRECT_EMIT_PORT;

#[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
/// Active input port for the UART-backed port-I/O backend.
const ACTIVE_KEY_PORT: Address = vm::UART_RBR_THR_PORT;
#[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
/// Active output port for the UART-backed port-I/O backend.
const ACTIVE_EMIT_PORT: Address = vm::UART_RBR_THR_PORT;

/// Install the stage-zero input/output words in dictionary order.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(KEY_NAME, Primitive::Key, 0)?;
    vm.install_primitive_word(EMIT_NAME, Primitive::Emit, 0)?;
    vm.install_primitive_word(DOT_NAME, Primitive::Dot, 0)?;
    Ok(())
}

/// Execute `KEY` by reading one character through the active VM input model.
pub(crate) fn execute_key<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    #[cfg(feature = "vm-port-io")]
    let value = vm.port_in(ACTIVE_KEY_PORT)?;

    #[cfg(not(feature = "vm-port-io"))]
    let value = vm.read_cell(ACTIVE_KEY_ADDRESS)?;

    vm.push_data(value)?;
    Ok(Control::Continue)
}

/// Execute `EMIT` by writing the low byte of the top cell through the active VM output model.
pub(crate) fn execute_emit<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let value = vm.pop_data()?;
    write_output(vm, value)?;
    Ok(Control::Continue)
}

/// Execute `.` by printing the top stack cell in signed decimal followed by a space.
pub(crate) fn execute_dot<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let value = vm.pop_data()?;
    emit_decimal(&mut vm.io, value);
    vm.io.emit(b' ');
    Ok(Control::Continue)
}

/// Write one output cell through the active port-based device model.
#[cfg(feature = "vm-port-io")]
fn write_output<I: ForthIo>(vm: &mut ForthVm<I>, value: Cell) -> Result<(), VmError> {
    vm.port_out(ACTIVE_EMIT_PORT, value);
    Ok(())
}

/// Write one output cell through the active memory-mapped device model.
#[cfg(not(feature = "vm-port-io"))]
fn write_output<I: ForthIo>(vm: &mut ForthVm<I>, value: Cell) -> Result<(), VmError> {
    vm.write_cell(ACTIVE_EMIT_ADDRESS, value)
}

/// Emit one signed decimal cell without allocating.
fn emit_decimal(io: &mut impl ForthIo, value: Cell) {
    let mut buf = [0u8; 20];
    let negative = value < 0;
    let mut magnitude = value.unsigned_abs();
    let mut len = 0;

    loop {
        buf[len] = b'0' + (magnitude % 10) as u8;
        len += 1;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }

    if negative {
        io.emit(b'-');
    }

    while len > 0 {
        len -= 1;
        io.emit(buf[len]);
    }
}
