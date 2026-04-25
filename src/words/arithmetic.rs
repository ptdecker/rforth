//! Arithmetic and comparison words needed for the first source-driven REPL
//!
//! These words are present so source files can do real work instead of only exercising the inner
//! interpreter. They are the minimum numeric substrate needed for self-tests, branching conditions,
//! and ordinary stack calculations.

use crate::{
    io::ForthIo,
    vm::{Cell, ForthVm, VmError},
};

use super::{Control, Primitive};

/// Dictionary name for the wrapping addition primitive.
const ADD_NAME: &str = "+";

/// Dictionary name for the wrapping subtraction primitive.
const SUBTRACT_NAME: &str = "-";

/// Dictionary name for the equality comparison primitive.
const EQUAL_NAME: &str = "=";

/// Dictionary name for the zero-test comparison primitive.
const ZERO_EQUAL_NAME: &str = "0=";

/// Install the arithmetic and comparison words in dictionary order
///
/// Source-driven tests depend on these words existing before `.fth` input is interpreted.
//noinspection DuplicatedCode
pub(crate) fn install<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<(), VmError> {
    vm.install_primitive_word(ADD_NAME, Primitive::Add, 0)?;
    vm.install_primitive_word(SUBTRACT_NAME, Primitive::Subtract, 0)?;
    vm.install_primitive_word(EQUAL_NAME, Primitive::Equal, 0)?;
    vm.install_primitive_word(ZERO_EQUAL_NAME, Primitive::ZeroEqual, 0)?;
    Ok(())
}

/// Execute `+` by adding the top two stack cells
///
/// Addition is the baseline numeric operation that later arithmetic and address calculations build
/// on.
pub(crate) fn execute_add<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let right_hand_operand = vm.pop_data()?;
    let left_hand_operand = vm.pop_data()?;
    vm.push_data(left_hand_operand.wrapping_add(right_hand_operand))?;
    Ok(Control::Continue)
}

/// Execute `-` by subtracting the top stack cell from the next stack cell
///
/// Subtraction complements `+` and keeps simple comparisons and offset calculations expressible.
pub(crate) fn execute_subtract<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let right_hand_operand = vm.pop_data()?;
    let left_hand_operand = vm.pop_data()?;
    vm.push_data(left_hand_operand.wrapping_sub(right_hand_operand))?;
    Ok(Control::Continue)
}

/// Execute `=` by comparing the top two stack cells for equality
///
/// Equality is needed immediately for source-driven self-checks and conditional control flow.
pub(crate) fn execute_equal<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let right_hand_operand = vm.pop_data()?;
    let left_hand_operand = vm.pop_data()?;
    vm.push_data(boolean_cell(left_hand_operand == right_hand_operand))?;
    Ok(Control::Continue)
}

/// Execute `0=` by testing whether the top stack cell is zero
///
/// Zero-testing is the conventional bridge between arithmetic results and Forth flag semantics.
pub(crate) fn execute_zero_equal<I: ForthIo>(vm: &mut ForthVm<I>) -> Result<Control, VmError> {
    let value = vm.pop_data()?;
    vm.push_data(boolean_cell(value == 0))?;
    Ok(Control::Continue)
}

/// Return the conventional Forth boolean cell representation
///
/// Forth uses all-bits-set for true and zero for false, so source code can feed flags directly into
/// conditional words and bitwise-style reasoning.
fn boolean_cell(flag: bool) -> Cell {
    if flag { -1 } else { 0 }
}
