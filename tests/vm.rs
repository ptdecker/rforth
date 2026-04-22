//! VM behavior tests across the supported I/O feature combinations.
//!
//! These tests verify the flat memory layout, type-derived sizing, stack-in-memory behavior,
//! terminal input buffer handling, and the selected direct/UART MMIO or port I/O dispatch model.

use rforth::io::ForthIo;
use rforth::vm::*;

/// Input byte used by memory-mapped I/O tests.
#[cfg(not(feature = "vm-port-io"))]
const INPUT_BYTE_A: u8 = b'a';

/// Input byte used by port I/O tests.
#[cfg(feature = "vm-port-io")]
const INPUT_BYTE_B: u8 = b'b';

/// Output byte written through VM-backed I/O tests.
const OUTPUT_BYTE_Z: Cell = b'z' as Cell;

/// First sample cell used for stack and memory round-trip assertions.
const FIRST_CELL: Cell = 0x1122_3344_5566_7788;

/// Second sample cell used to verify stack ordering.
const SECOND_CELL: Cell = -42;

/// Aligned address used for normal cell memory tests.
const TEST_ADDR: Address = 0x0100;

/// Unaligned address used to verify cell alignment checks.
const UNALIGNED_TEST_ADDR: Address = TEST_ADDR + 1;

/// Sample terminal input buffer contents.
const TIB_SAMPLE: &[MemoryWord] = b"abc";

/// Compile-time sanity checks for the VM memory map.
const _: () = {
    assert!(
        DICTIONARY_START < TIB_START,
        "dictionary must start below the terminal input buffer"
    );
    assert!(
        TIB_END <= STACK_LIMIT,
        "terminal input buffer must not overlap the shared stack arena"
    );
    assert!(
        STACK_LIMIT == RETURN_STACK_BASE,
        "return stack should start at the low end of the shared stack arena"
    );
    assert!(
        RETURN_STACK_BASE < DATA_STACK_BASE,
        "return and data stacks must start at opposite ends of the arena"
    );
    assert!(
        DATA_STACK_BASE == STACK_BASE,
        "data stack should start at the high end of the shared stack arena"
    );
    assert!(
        DATA_STACK_BASE < MMIO_BASE,
        "shared stack arena must stay below the MMIO window"
    );
};

/// Scripted host I/O backend used by VM tests.
///
/// `key` consumes bytes from `input`, and `emit` records bytes in `output`, allowing tests to
/// verify that VM I/O dispatch reaches the [`ForthIo`] boundary.
struct ScriptedIo<'a> {
    /// Input bytes returned by [`ForthIo::key`].
    input: &'a [u8],
    /// Current read offset into `input`.
    input_pos: usize,
    /// Bytes captured from [`ForthIo::emit`].
    output: Vec<u8>,
}

impl<'a> ScriptedIo<'a> {
    /// Construct a scripted I/O backend with empty captured output.
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            input_pos: 0,
            output: Vec::new(),
        }
    }
}

impl ForthIo for ScriptedIo<'_> {
    /// Capture one output byte.
    fn emit(&mut self, c: u8) {
        self.output.push(c);
    }

    /// Return the next scripted input byte.
    fn key(&mut self) -> u8 {
        let c = self.input[self.input_pos];
        self.input_pos += 1;
        c
    }
}

/// Verifies memory and cell sizing are derived from the VM type aliases.
#[test]
fn exposes_type_driven_memory_constants() {
    assert_eq!(
        MEMORY_SIZE,
        Address::MAX as usize + 1,
        "memory size must be derived directly from the address type"
    );
    assert_eq!(
        CELL_SIZE,
        size_of::<Cell>(),
        "cell size constant must match the Cell type width"
    );
    assert_eq!(
        CELL_ALIGN, CELL_SIZE,
        "cell alignment should track the cell size for this VM profile"
    );
}

/// Verifies a newly constructed VM starts with the expected pointers and empty input state.
#[test]
fn initializes_vm_layout() {
    let io = ScriptedIo::new(b"");
    let vm = ForthVm::new(io);

    assert_eq!(
        vm.here, DICTIONARY_START,
        "dictionary pointer should start at DICTIONARY_START"
    );
    assert_eq!(
        vm.ip, DICTIONARY_START,
        "instruction pointer should start at DICTIONARY_START"
    );
    assert_eq!(
        vm.sp, DATA_STACK_BASE,
        "data stack pointer should start at DATA_STACK_BASE"
    );
    assert_eq!(
        vm.rp, RETURN_STACK_BASE,
        "return stack pointer should start at RETURN_STACK_BASE"
    );
    assert_eq!(
        vm.state,
        InterpreterState::Interpreting,
        "VM should boot in interpreting mode"
    );
    assert_eq!(
        vm.tib_start, TIB_START,
        "VM should expose the configured terminal input buffer start"
    );
    assert_eq!(vm.tib_len, 0, "terminal input buffer should boot empty");
    assert_eq!(
        vm.input_pos, 0,
        "parse offset should boot at the start of TIB"
    );
    assert_eq!(
        vm.memory().len(),
        MEMORY_SIZE,
        "backing memory length should match MEMORY_SIZE"
    );
}

/// Verifies the named memory regions are ordered and sized as expected.
#[test]
fn layout_constants_do_not_overlap() {
    assert_eq!(
        TIB_END as usize,
        TIB_START as usize + TIB_SIZE,
        "TIB_END should be computed from TIB_START plus TIB_SIZE"
    );
    assert_eq!(
        STACK_LIMIT, TIB_END,
        "shared stack arena should begin immediately after the TIB"
    );
    assert_eq!(
        RETURN_STACK_BASE, STACK_LIMIT,
        "return stack should grow upward from the arena low address"
    );
    assert_eq!(
        DATA_STACK_BASE, STACK_BASE,
        "data stack should grow downward from the arena high address"
    );
    assert_eq!(
        MMIO_END,
        MMIO_BASE as usize + MMIO_SIZE,
        "MMIO_END should be computed from MMIO_BASE plus MMIO_SIZE"
    );
    assert_eq!(
        MMIO_END, MEMORY_SIZE,
        "MMIO window should occupy the top of addressable memory"
    );
}

/// Verifies cell writes are stored in VM memory and read back unchanged.
#[test]
fn cell_access_round_trips_through_memory() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    vm.write_cell(TEST_ADDR, FIRST_CELL).unwrap();

    assert_eq!(
        vm.read_cell(TEST_ADDR).unwrap(),
        FIRST_CELL,
        "cell read should return the exact value previously written"
    );
}

/// Verifies cell reads and writes reject unaligned addresses.
#[test]
fn cell_access_rejects_unaligned_addresses() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.write_cell(UNALIGNED_TEST_ADDR, FIRST_CELL),
        Err(VmError::UnalignedCell),
        "cell writes at unaligned addresses should return UnalignedCell"
    );
    assert_eq!(
        vm.read_cell(UNALIGNED_TEST_ADDR),
        Err(VmError::UnalignedCell),
        "cell reads at unaligned addresses should return UnalignedCell"
    );
}

/// Verifies the data stack stores cells in VM memory and preserves LIFO order.
#[test]
fn data_stack_push_pop_uses_memory() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    vm.push_data(FIRST_CELL).unwrap();
    assert_eq!(
        vm.sp,
        DATA_STACK_BASE - CELL_SIZE as Address,
        "data stack push should move sp downward by one cell"
    );
    assert_eq!(
        vm.read_cell(vm.sp).unwrap(),
        FIRST_CELL,
        "data stack push should store the cell at the new sp address"
    );

    vm.push_data(SECOND_CELL).unwrap();
    assert_eq!(
        vm.pop_data().unwrap(),
        SECOND_CELL,
        "first data stack pop should return the most recently pushed cell"
    );
    assert_eq!(
        vm.pop_data().unwrap(),
        FIRST_CELL,
        "second data stack pop should return the older cell"
    );
    assert_eq!(
        vm.sp, DATA_STACK_BASE,
        "data stack pointer should return to base after popping all cells"
    );
}

/// Verifies data stack boundary checks report underflow and collision overflow.
#[test]
fn data_stack_detects_underflow_and_overflow() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.pop_data(),
        Err(VmError::StackUnderflow(StackKind::Data)),
        "popping an empty data stack should report data stack underflow"
    );

    while vm.sp > vm.rp {
        vm.push_data(FIRST_CELL).unwrap();
    }

    assert_eq!(
        vm.push_data(FIRST_CELL),
        Err(VmError::StackOverflow(StackKind::Data)),
        "pushing until data and return stacks collide should report data stack overflow"
    );
}

/// Verifies the return stack stores cells in VM memory and preserves LIFO order.
#[test]
fn return_stack_push_pop_uses_memory() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    vm.push_return(FIRST_CELL).unwrap();
    assert_eq!(
        vm.rp,
        RETURN_STACK_BASE + CELL_SIZE as Address,
        "return stack push should move rp upward by one cell"
    );
    assert_eq!(
        vm.read_cell(RETURN_STACK_BASE).unwrap(),
        FIRST_CELL,
        "return stack push should store the cell at the previous rp address"
    );

    vm.push_return(SECOND_CELL).unwrap();
    assert_eq!(
        vm.pop_return().unwrap(),
        SECOND_CELL,
        "first return stack pop should return the most recently pushed cell"
    );
    assert_eq!(
        vm.pop_return().unwrap(),
        FIRST_CELL,
        "second return stack pop should return the older cell"
    );
    assert_eq!(
        vm.rp, RETURN_STACK_BASE,
        "return stack pointer should return to base after popping all cells"
    );
}

/// Verifies return stack boundary checks report underflow and collision overflow.
#[test]
fn return_stack_detects_underflow_and_overflow() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.pop_return(),
        Err(VmError::StackUnderflow(StackKind::Return)),
        "popping an empty return stack should report return stack underflow"
    );

    while vm.rp < vm.sp {
        vm.push_return(FIRST_CELL).unwrap();
    }

    assert_eq!(
        vm.push_return(FIRST_CELL),
        Err(VmError::StackOverflow(StackKind::Return)),
        "pushing until return and data stacks collide should report return stack overflow"
    );
}

/// Verifies terminal input buffer loading writes VM memory and resets parse state.
#[test]
fn tib_load_and_reset_update_vm_memory_and_offsets() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    vm.load_tib(TIB_SAMPLE).unwrap();

    let start = address_index(TIB_START);
    assert_eq!(
        &vm.memory()[start..start + TIB_SAMPLE.len()],
        TIB_SAMPLE,
        "load_tib should copy input bytes into VM memory at TIB_START"
    );
    assert_eq!(
        vm.tib_len,
        TIB_SAMPLE.len(),
        "load_tib should record the active TIB length"
    );
    assert_eq!(vm.input_pos, 0, "load_tib should reset the parse offset");

    vm.input_pos = TIB_SAMPLE.len();
    vm.reset_tib();

    assert_eq!(
        vm.tib_len, 0,
        "reset_tib should clear the active TIB length"
    );
    assert_eq!(vm.input_pos, 0, "reset_tib should clear the parse offset");
}

/// Verifies oversized input is rejected instead of overflowing the terminal input buffer.
#[test]
fn tib_load_rejects_oversized_input() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);
    let input = [MemoryWord::default(); TIB_SIZE + 1];

    assert_eq!(
        vm.load_tib(&input),
        Err(VmError::TibOverflow),
        "load_tib should reject input longer than TIB_SIZE"
    );
}

/// Verifies dictionary allocation advances `here` and stops before the TIB region.
#[test]
fn allot_advances_dictionary_and_detects_tib_collision() {
    let io = ScriptedIo::new(b"");
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.allot(CELL_SIZE).unwrap(),
        DICTIONARY_START,
        "first allot should return the initial dictionary address"
    );
    assert_eq!(
        vm.here,
        DICTIONARY_START + CELL_SIZE as Address,
        "allot should advance here by the requested byte count"
    );

    assert_eq!(
        vm.allot(TIB_START as usize),
        Err(VmError::DictionaryOverflow),
        "allot should reject dictionary growth into the TIB region"
    );
}

/// Verifies default direct MMIO dispatch for key, emit, and key-ready status.
#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
#[test]
fn direct_mmio_dispatches_key_emit_and_ready() {
    let io = ScriptedIo::new(&[INPUT_BYTE_A]);
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.read_cell(DIRECT_MMIO_KEY_READY_ADDR).unwrap(),
        KEY_READY_TRUE,
        "direct MMIO key-ready address should report ready"
    );
    assert_eq!(
        vm.read_memory_word(DIRECT_MMIO_KEY_ADDR).unwrap(),
        INPUT_BYTE_A,
        "direct MMIO key address should read through ForthIo::key"
    );
    vm.write_cell(DIRECT_MMIO_EMIT_ADDR, OUTPUT_BYTE_Z).unwrap();

    assert_eq!(
        vm.io.output, b"z",
        "direct MMIO emit address should write through ForthIo::emit"
    );
}

/// Verifies direct port I/O dispatch for key, emit, and key-ready status.
#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
#[test]
fn direct_port_io_dispatches_key_emit_and_ready() {
    let io = ScriptedIo::new(&[INPUT_BYTE_B]);
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.port_in(DIRECT_KEY_READY_PORT),
        KEY_READY_TRUE,
        "direct key-ready port should report ready"
    );
    assert_eq!(
        vm.port_in(DIRECT_KEY_PORT),
        Cell::from(INPUT_BYTE_B),
        "direct key port should read through ForthIo::key"
    );
    vm.port_out(DIRECT_EMIT_PORT, OUTPUT_BYTE_Z);

    assert_eq!(
        vm.io.output, b"z",
        "direct emit port should write through ForthIo::emit"
    );
}

/// Verifies UART register dispatch through the memory-mapped I/O window.
#[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
#[test]
fn uart_mmio_dispatches_registers() {
    let io = ScriptedIo::new(&[INPUT_BYTE_A]);
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.read_cell(UART_LSR_ADDR).unwrap(),
        UART_LSR_READY,
        "UART MMIO line status register should report ready bits"
    );
    assert_eq!(
        vm.read_memory_word(UART_RBR_THR_ADDR).unwrap(),
        INPUT_BYTE_A,
        "UART MMIO RBR address should read through ForthIo::key"
    );
    vm.write_memory_word(UART_RBR_THR_ADDR, OUTPUT_BYTE_Z as MemoryWord)
        .unwrap();

    assert_eq!(
        vm.io.output, b"z",
        "UART MMIO THR address should write through ForthIo::emit"
    );
}

/// Verifies UART register dispatch through the port I/O address space.
#[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
#[test]
fn uart_port_io_dispatches_registers() {
    let io = ScriptedIo::new(&[INPUT_BYTE_B]);
    let mut vm = ForthVm::new(io);

    assert_eq!(
        vm.port_in(UART_LSR_PORT),
        UART_LSR_READY,
        "UART port line status register should report ready bits"
    );
    assert_eq!(
        vm.port_in(UART_RBR_THR_PORT),
        Cell::from(INPUT_BYTE_B),
        "UART RBR port should read through ForthIo::key"
    );
    vm.port_out(UART_RBR_THR_PORT, OUTPUT_BYTE_Z);

    assert_eq!(
        vm.io.output, b"z",
        "UART THR port should write through ForthIo::emit"
    );
}
