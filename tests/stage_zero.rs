//! Stage-zero word installation and execution tests.
//!
//! These tests verify the classic irreducible core: dictionary-resident primitive installation,
//! threaded execution, `QUIT`, `BYE`, core stack manipulation, memory access, and basic
//! input/output.

use rforth::io::ForthIo;
use rforth::vm::{
    Address, CELL_SIZE, Cell, DICTIONARY_NAME_BYTES_OFFSET, ForthVm, InterpreterState, NO_ADDRESS,
    VmError, WORD_FLAG_PRIMITIVE,
};
use rforth::words::{Control, install_stage_zero};

/// Scratch address used by memory word tests.
const TEST_MEMORY_ADDR: Address = 0x0200;

/// Sample first data-stack value.
const FIRST_VALUE: Cell = 11;

/// Sample second data-stack value.
const SECOND_VALUE: Cell = 22;

/// Sample output character written by `EMIT`.
const OUTPUT_BYTE: Cell = b'Z' as Cell;

/// Scripted host input/output backend used by stage-zero tests.
struct ScriptedIo<'a> {
    /// Input bytes returned one at a time by `KEY`.
    input: &'a [u8],
    /// Current input read position.
    input_pos: usize,
    /// Captured output bytes written by `EMIT`.
    output: Vec<u8>,
}

impl<'a> ScriptedIo<'a> {
    /// Construct a scripted backend with empty captured output.
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            input_pos: 0,
            output: Vec::new(),
        }
    }
}

impl ForthIo for ScriptedIo<'_> {
    /// Capture one emitted byte.
    fn emit(&mut self, c: u8) {
        self.output.push(c);
    }

    /// Return the next scripted input byte.
    fn key(&mut self) -> u8 {
        let value = self.input[self.input_pos];
        self.input_pos += 1;
        value
    }
}

/// Verifies stage-zero installation registers the expected words in dictionary order.
#[test]
fn installs_stage_zero_words_in_expected_order() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let latest_xt = vm.latest;
    let latest_name =
        word_name(&mut vm, latest_xt).expect("latest dictionary word should decode cleanly");
    assert_eq!(
        latest_name, "EMIT",
        "the last installed stage-zero word should be EMIT"
    );

    let emit_entry = vm
        .dictionary_entry(vm.latest)
        .expect("latest dictionary word should have a readable entry");
    let previous_xt = vm
        .read_address(emit_entry.link_addr)
        .expect("latest dictionary link field should store the previous execution token");
    let key_name =
        word_name(&mut vm, previous_xt).expect("linked dictionary word should decode cleanly");
    assert_eq!(
        key_name, "KEY",
        "EMIT should link back to KEY in installation order"
    );

    let docol_xt =
        find_word(&mut vm, "DOCOL").expect("DOCOL should be installed into the dictionary");
    let docol_entry = vm
        .dictionary_entry(docol_xt)
        .expect("DOCOL dictionary entry should be readable");
    assert_eq!(
        docol_entry.flags & WORD_FLAG_PRIMITIVE,
        WORD_FLAG_PRIMITIVE,
        "DOCOL should be marked as a primitive word in the dictionary"
    );
}

/// Verifies `DOCOL`, `LIT`, and `DOSEMI` support a minimal colon definition.
#[test]
fn runs_a_minimal_colon_definition() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let lit_xt = find_word(&mut vm, "LIT").expect("LIT should be installed");
    let dosemi_xt = find_word(&mut vm, "DOSEMI").expect("DOSEMI should be installed");
    let colon_xt = vm
        .install_colon_word(
            "ONE",
            &[Cell::from(lit_xt), FIRST_VALUE, Cell::from(dosemi_xt)],
            0,
        )
        .expect("colon word installation should succeed");

    assert_eq!(
        vm.run_word(colon_xt)
            .expect("running a simple colon definition should succeed"),
        Control::Continue,
        "colon definitions should normally complete with a continue control result"
    );

    assert_eq!(
        vm.pop_data()
            .expect("colon definition should leave a literal on the stack"),
        FIRST_VALUE,
        "LIT inside a colon definition should push its inline literal value"
    );
    assert_eq!(
        vm.rp,
        rforth::vm::RETURN_STACK_BASE,
        "colon execution should unwind the return stack back to its base"
    );
    assert_eq!(
        vm.ip, NO_ADDRESS,
        "top-level colon execution should finish with an invalid instruction pointer"
    );
}

/// Verifies `NEXT` advances the threaded-code stream, loads `W`, and dispatches the fetched word.
#[test]
fn next_advances_ip_sets_w_and_dispatches() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let dup_xt = find_word(&mut vm, "DUP").expect("DUP should be installed");
    let thread_start = vm
        .allot(CELL_SIZE)
        .expect("allocating one threaded-code cell should succeed");
    vm.write_address(thread_start, dup_xt)
        .expect("writing the DUP execution token into threaded code should succeed");
    vm.push_data(FIRST_VALUE)
        .expect("pushing the source value for DUP should succeed");
    vm.ip = thread_start;

    assert_eq!(
        vm.next()
            .expect("NEXT should execute one threaded-code cell"),
        Control::Continue,
        "NEXT should normally continue threaded execution"
    );
    assert_eq!(
        vm.ip,
        thread_start + CELL_SIZE as Address,
        "NEXT should advance the instruction pointer by one cell"
    );
    assert_eq!(
        vm.w, dup_xt,
        "NEXT should load the fetched execution token into the work register"
    );
    assert_eq!(
        vm.pop_data()
            .expect("DUP dispatched through NEXT should leave a copied top cell"),
        FIRST_VALUE,
        "NEXT should dispatch the fetched word after loading W"
    );
    assert_eq!(
        vm.pop_data()
            .expect("the original DUP source value should remain on the data stack"),
        FIRST_VALUE,
        "DUP dispatched through NEXT should preserve the original top cell"
    );
}

/// Verifies `BRANCH` skips the fallthrough path and transfers to the inline target.
#[test]
fn executes_branch_by_jumping_to_the_inline_target() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let branch_xt = find_word(&mut vm, "BRANCH").expect("BRANCH should be installed");
    let lit_xt = find_word(&mut vm, "LIT").expect("LIT should be installed");
    let dosemi_xt = find_word(&mut vm, "DOSEMI").expect("DOSEMI should be installed");
    let colon_xt = vm
        .install_colon_word(
            "BRTEST",
            &[
                Cell::from(branch_xt),
                0,
                Cell::from(lit_xt),
                FIRST_VALUE,
                Cell::from(lit_xt),
                SECOND_VALUE,
                Cell::from(dosemi_xt),
            ],
            0,
        )
        .expect("branch test colon word installation should succeed");
    let entry = vm
        .dictionary_entry(colon_xt)
        .expect("branch test dictionary entry should be readable");
    let branch_target = entry.pfa + (CELL_SIZE as Address * 4);
    vm.write_address(entry.pfa + CELL_SIZE as Address, branch_target)
        .expect("branch target patch should succeed");

    assert_eq!(
        vm.run_word(colon_xt)
            .expect("running the branch test word should succeed"),
        Control::Continue,
        "BRANCH test word should complete normally"
    );
    assert_eq!(
        vm.pop_data()
            .expect("branch target should leave one literal on the data stack"),
        SECOND_VALUE,
        "BRANCH should skip the fallthrough literal and execute only the target path"
    );
    assert_eq!(
        vm.pop_data(),
        Err(VmError::StackUnderflow(rforth::vm::StackKind::Data)),
        "BRANCH should skip the fallthrough path entirely"
    );
}

/// Verifies `0BRANCH` takes the inline target when the popped flag is zero.
#[test]
fn executes_zero_branch_taken_path_on_zero_flag() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let lit_xt = find_word(&mut vm, "LIT").expect("LIT should be installed");
    let zero_branch_xt = find_word(&mut vm, "0BRANCH").expect("0BRANCH should be installed");
    let dosemi_xt = find_word(&mut vm, "DOSEMI").expect("DOSEMI should be installed");
    let colon_xt = vm
        .install_colon_word(
            "ZBTAKE",
            &[
                Cell::from(lit_xt),
                0,
                Cell::from(zero_branch_xt),
                0,
                Cell::from(lit_xt),
                FIRST_VALUE,
                Cell::from(lit_xt),
                SECOND_VALUE,
                Cell::from(dosemi_xt),
            ],
            0,
        )
        .expect("zero-branch taken test colon word installation should succeed");
    let entry = vm
        .dictionary_entry(colon_xt)
        .expect("zero-branch taken dictionary entry should be readable");
    let branch_target = entry.pfa + (CELL_SIZE as Address * 6);
    vm.write_address(entry.pfa + (CELL_SIZE as Address * 3), branch_target)
        .expect("zero-branch taken target patch should succeed");

    assert_eq!(
        vm.run_word(colon_xt)
            .expect("running the zero-branch taken test word should succeed"),
        Control::Continue,
        "0BRANCH taken-path test word should complete normally"
    );
    assert_eq!(
        vm.pop_data()
            .expect("taken zero-branch path should leave one literal on the data stack"),
        SECOND_VALUE,
        "0BRANCH should jump to the target when the flag is zero"
    );
    assert_eq!(
        vm.pop_data(),
        Err(VmError::StackUnderflow(rforth::vm::StackKind::Data)),
        "0BRANCH taken path should skip the fallthrough literal entirely"
    );
}

/// Verifies `0BRANCH` falls through when the popped flag is nonzero.
#[test]
fn executes_zero_branch_fallthrough_on_nonzero_flag() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let lit_xt = find_word(&mut vm, "LIT").expect("LIT should be installed");
    let zero_branch_xt = find_word(&mut vm, "0BRANCH").expect("0BRANCH should be installed");
    let dosemi_xt = find_word(&mut vm, "DOSEMI").expect("DOSEMI should be installed");
    let colon_xt = vm
        .install_colon_word(
            "ZBSKIP",
            &[
                Cell::from(lit_xt),
                1,
                Cell::from(zero_branch_xt),
                0,
                Cell::from(lit_xt),
                FIRST_VALUE,
                Cell::from(dosemi_xt),
                Cell::from(lit_xt),
                SECOND_VALUE,
                Cell::from(dosemi_xt),
            ],
            0,
        )
        .expect("zero-branch fallthrough test colon word installation should succeed");
    let entry = vm
        .dictionary_entry(colon_xt)
        .expect("zero-branch fallthrough dictionary entry should be readable");
    let branch_target = entry.pfa + (CELL_SIZE as Address * 7);
    vm.write_address(entry.pfa + (CELL_SIZE as Address * 3), branch_target)
        .expect("zero-branch fallthrough target patch should succeed");

    assert_eq!(
        vm.run_word(colon_xt)
            .expect("running the zero-branch fallthrough test word should succeed"),
        Control::Continue,
        "0BRANCH fallthrough-path test word should complete normally"
    );
    assert_eq!(
        vm.pop_data()
            .expect("fallthrough zero-branch path should leave one literal on the data stack"),
        FIRST_VALUE,
        "0BRANCH should ignore the branch target when the flag is nonzero"
    );
    assert_eq!(
        vm.pop_data(),
        Err(VmError::StackUnderflow(rforth::vm::StackKind::Data)),
        "0BRANCH fallthrough path should return before the target path executes"
    );
}

/// Verifies `DUP`, `DROP`, and `SWAP` execute as dictionary-resident primitives.
#[test]
fn executes_stage_zero_stack_words() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let dup_xt = find_word(&mut vm, "DUP").expect("DUP should be installed");
    let drop_xt = find_word(&mut vm, "DROP").expect("DROP should be installed");
    let swap_xt = find_word(&mut vm, "SWAP").expect("SWAP should be installed");

    vm.push_data(FIRST_VALUE)
        .expect("pushing the first stack value should succeed");
    assert_eq!(
        vm.run_word(dup_xt).expect("running DUP should succeed"),
        Control::Continue,
        "DUP should complete with a continue control result"
    );
    assert_eq!(
        vm.pop_data()
            .expect("DUP should leave two cells on the stack"),
        FIRST_VALUE,
        "DUP should copy the original top cell"
    );
    assert_eq!(
        vm.pop_data()
            .expect("DUP should preserve the original cell below the copy"),
        FIRST_VALUE,
        "DUP should preserve the original top cell"
    );

    vm.push_data(FIRST_VALUE)
        .expect("pushing the first SWAP operand should succeed");
    vm.push_data(SECOND_VALUE)
        .expect("pushing the second SWAP operand should succeed");
    assert_eq!(
        vm.run_word(swap_xt).expect("running SWAP should succeed"),
        Control::Continue,
        "SWAP should complete with a continue control result"
    );
    assert_eq!(
        vm.pop_data()
            .expect("SWAP should leave the original lower cell on top"),
        FIRST_VALUE,
        "SWAP should move the lower of the two input cells to the top"
    );
    assert_eq!(
        vm.pop_data()
            .expect("SWAP should leave the original top cell underneath"),
        SECOND_VALUE,
        "SWAP should move the original top cell below the other input cell"
    );

    vm.push_data(FIRST_VALUE)
        .expect("pushing the DROP operand should succeed");
    assert_eq!(
        vm.run_word(drop_xt).expect("running DROP should succeed"),
        Control::Continue,
        "DROP should complete with a continue control result"
    );
    assert_eq!(
        vm.pop_data(),
        Err(VmError::StackUnderflow(rforth::vm::StackKind::Data)),
        "DROP should remove the only data-stack cell"
    );
}

/// Verifies `@`, `!`, `C@`, and `C!` round-trip through VM memory.
#[test]
fn executes_stage_zero_memory_words() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let fetch_xt = find_word(&mut vm, "@").expect("@ should be installed");
    let store_xt = find_word(&mut vm, "!").expect("! should be installed");
    let c_fetch_xt = find_word(&mut vm, "C@").expect("C@ should be installed");
    let c_store_xt = find_word(&mut vm, "C!").expect("C! should be installed");

    vm.push_data(FIRST_VALUE)
        .expect("pushing the cell store value should succeed");
    vm.push_data(Cell::from(TEST_MEMORY_ADDR))
        .expect("pushing the cell store address should succeed");
    assert_eq!(
        vm.run_word(store_xt).expect("running ! should succeed"),
        Control::Continue,
        "! should complete with a continue control result"
    );
    vm.push_data(Cell::from(TEST_MEMORY_ADDR))
        .expect("pushing the cell fetch address should succeed");
    assert_eq!(
        vm.run_word(fetch_xt).expect("running @ should succeed"),
        Control::Continue,
        "@ should complete with a continue control result"
    );
    assert_eq!(
        vm.pop_data().expect("@ should leave one fetched cell"),
        FIRST_VALUE,
        "@ should read back the value stored by !"
    );

    vm.push_data(Cell::from(b'Q'))
        .expect("pushing the byte store value should succeed");
    vm.push_data(Cell::from(TEST_MEMORY_ADDR))
        .expect("pushing the byte store address should succeed");
    assert_eq!(
        vm.run_word(c_store_xt).expect("running C! should succeed"),
        Control::Continue,
        "C! should complete with a continue control result"
    );
    vm.push_data(Cell::from(TEST_MEMORY_ADDR))
        .expect("pushing the byte fetch address should succeed");
    assert_eq!(
        vm.run_word(c_fetch_xt).expect("running C@ should succeed"),
        Control::Continue,
        "C@ should complete with a continue control result"
    );
    assert_eq!(
        vm.pop_data().expect("C@ should leave one fetched byte"),
        Cell::from(b'Q'),
        "C@ should read back the byte stored by C!"
    );
}

/// Verifies `KEY` and `EMIT` route through the configured host-backed I/O model.
#[test]
fn executes_stage_zero_io_words() {
    let io = ScriptedIo::new(b"K");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let key_xt = find_word(&mut vm, "KEY").expect("KEY should be installed");
    let emit_xt = find_word(&mut vm, "EMIT").expect("EMIT should be installed");

    assert_eq!(
        vm.run_word(key_xt).expect("running KEY should succeed"),
        Control::Continue,
        "KEY should complete with a continue control result"
    );
    assert_eq!(
        vm.pop_data()
            .expect("KEY should leave one input character on the stack"),
        Cell::from(b'K'),
        "KEY should push the input character as a cell"
    );

    vm.push_data(OUTPUT_BYTE)
        .expect("pushing the EMIT output byte should succeed");
    assert_eq!(
        vm.run_word(emit_xt).expect("running EMIT should succeed"),
        Control::Continue,
        "EMIT should complete with a continue control result"
    );
    assert_eq!(
        vm.io.output.as_slice(),
        &[OUTPUT_BYTE as u8],
        "EMIT should forward the low byte to the host output backend"
    );
}

/// Verifies `QUIT` restores the minimal outer-interpreter state.
#[test]
fn executes_quit_as_outer_interpreter_reset() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let quit_xt = find_word(&mut vm, "QUIT").expect("QUIT should be installed");
    vm.push_return(FIRST_VALUE)
        .expect("pushing a return-stack marker should succeed");
    vm.state = InterpreterState::Compiling;
    vm.load_tib(b"ABC")
        .expect("loading a test terminal input buffer should succeed");
    vm.input_pos = 2;
    vm.ip = TEST_MEMORY_ADDR;
    vm.w = TEST_MEMORY_ADDR;

    assert_eq!(
        vm.run_word(quit_xt).expect("running QUIT should succeed"),
        Control::Quit,
        "QUIT should return an outer-interpreter reset control result"
    );

    assert_eq!(
        vm.rp,
        rforth::vm::RETURN_STACK_BASE,
        "QUIT should reset the return stack to its base"
    );
    assert_eq!(
        vm.state,
        InterpreterState::Interpreting,
        "QUIT should restore interpreting mode"
    );
    assert_eq!(
        vm.tib_len, 0,
        "QUIT should clear the terminal input buffer length"
    );
    assert_eq!(
        vm.input_pos, 0,
        "QUIT should reset the parse offset to the start of input"
    );
    assert_eq!(
        vm.ip, NO_ADDRESS,
        "QUIT should leave no active threaded-code instruction pointer"
    );
    assert_eq!(vm.w, NO_ADDRESS, "QUIT should leave no active current word");
}

/// Verifies `BYE` requests interpreter exit from the outer loop.
#[test]
fn executes_bye_as_process_exit_request() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    install_stage_zero(&mut vm).expect("stage-zero installation should succeed");

    let bye_xt = find_word(&mut vm, "BYE").expect("BYE should be installed");
    vm.ip = TEST_MEMORY_ADDR;
    vm.w = TEST_MEMORY_ADDR;

    assert_eq!(
        vm.run_word(bye_xt).expect("running BYE should succeed"),
        Control::Bye,
        "BYE should return an interpreter-exit control result"
    );
    assert_eq!(
        vm.ip, NO_ADDRESS,
        "BYE should leave no active threaded-code instruction pointer"
    );
    assert_eq!(vm.w, NO_ADDRESS, "BYE should leave no active current word");
}

/// Find the most recent dictionary word whose decoded name matches `target_name`.
fn find_word<I: ForthIo>(vm: &mut ForthVm<I>, target_name: &str) -> Option<Address> {
    let mut cursor = vm.latest;

    while cursor != NO_ADDRESS {
        if word_name(vm, cursor).ok()?.as_str() == target_name {
            return Some(cursor);
        }

        let entry = vm.dictionary_entry(cursor).ok()?;
        cursor = vm.read_address(entry.link_addr).ok()?;
    }

    None
}

/// Decode the stored dictionary name bytes for one execution token into a host string.
fn word_name<I: ForthIo>(vm: &mut ForthVm<I>, xt: Address) -> Result<String, VmError> {
    let entry = vm.dictionary_entry(xt)?;
    let mut name = String::new();

    for offset in 0..usize::from(entry.name_len) {
        let addr = xt
            .checked_add(DICTIONARY_NAME_BYTES_OFFSET as Address)
            .and_then(|start| start.checked_add(offset as Address))
            .ok_or(VmError::InvalidAddress)?;
        let byte = vm.read_memory_word(addr)?;
        name.push(char::from(byte));
    }

    Ok(name)
}
