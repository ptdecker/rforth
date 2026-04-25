//! Data-stack manipulation words
//!
//! These words exist as primitives because the current stage-zero substrate does not expose stack
//! pointer arithmetic directly to a Forth application. They provide the minimal conventional stack
//! surface that later words and source files rely on.

use crate::{
    io::ForthIo,
    vm::{ForthVm, VmError},
};

use super::{Control, Primitive};

/// Dictionary name for the top-cell duplication primitive.
const DUP_NAME: &str = "DUP";

/// Dictionary name for the top-cell discard primitive.
const DROP_NAME: &str = "DROP";

/// Dictionary name for the top-two-cell exchange primitive.
const SWAP_NAME: &str = "SWAP";

/// Dictionary name for the second-cell copy primitive.
const OVER_NAME: &str = "OVER";

/// Install the core stack words in dictionary order
///
/// Keeping their installation centralized here makes the primitive nucleus predictable and keeps
/// stack behavior available before higher-level source definitions depend on it.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(DUP_NAME, Primitive::Dup, 0)?;
    vm.install_primitive_word(DROP_NAME, Primitive::Drop, 0)?;
    vm.install_primitive_word(SWAP_NAME, Primitive::Swap, 0)?;
    vm.install_primitive_word(OVER_NAME, Primitive::Over, 0)?;
    Ok(())
}

/// Execute `DUP` by copying the current top-of-stack cell
///
/// `DUP` is foundational because later words often need to inspect or reuse a value without
/// consuming it.
pub(crate) fn execute_dup<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let value = vm.peek_data()?;
    vm.push_data(value)?;
    Ok(Control::Continue)
}

/// Execute `DROP` by discarding the current top-of-stack cell
///
/// `DROP` is the minimal way to discard intermediate results and keep stack depth under control.
pub(crate) fn execute_drop<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let _ = vm.pop_data()?;
    Ok(Control::Continue)
}

/// Execute `SWAP` by exchanging the top two data-stack cells
///
/// `SWAP` provides the smallest useful reordering primitive for stack-based composition.
pub(crate) fn execute_swap<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let top = vm.pop_data()?;
    let next = vm.pop_data()?;
    vm.push_data(top)?;
    vm.push_data(next)?;
    Ok(Control::Continue)
}

/// Execute `OVER` by copying the second stack cell to the top of the stack
///
/// `OVER` is included because it keeps simple source-level tests and definitions readable without
/// requiring more elaborate shuffle sequences.
pub(crate) fn execute_over<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let top = vm.pop_data()?;
    let next = vm.pop_data()?;
    vm.push_data(next)?;
    vm.push_data(top)?;
    vm.push_data(next)?;
    Ok(Control::Continue)
}
