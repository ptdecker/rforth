//! Forth virtual machine state and memory access
//!
//! The VM models a small flat machine: dictionary space, terminal input buffer, data stack, return
//! stack, dictionary-resident words, and optional input/output regions all live in one virtual
//! address space.

use core::mem::size_of;

use crate::{
    io::{ForthIo, InputEvent},
    words::{Control, Primitive},
};

/// Smallest addressable unit in VM memory
///
/// The initial machine profile is byte-addressed, but VM code should use this alias instead of
/// spelling the concrete type directly.
pub type MemoryWord = u8;

/// Forth data cell stored on stacks, threaded code, and dictionary fields.
pub type Cell = i64;

/// VM-visible memory address
///
/// The address type determines the size of the flat virtual address space through [`MEMORY_SIZE`].
pub type Address = u16;

/// Number of addressable [`MemoryWord`] slots in virtual memory
///
/// Deriving this from [`Address`] keeps the memory size and visible address width from drifting
/// apart.
pub const MEMORY_SIZE: usize = Address::MAX as usize + 1;

/// Number of memory words occupied by one [`Cell`]
///
/// The VM uses this to lay out stacks, threaded code, and dictionary fields consistently.
pub const CELL_SIZE: usize = size_of::<Cell>();

/// Required alignment, in memory words, for cell accesses
///
/// Cell-aligned access keeps the VM's memory rules explicit and lets misaligned word usage fail
/// deterministically.
pub const CELL_ALIGN: usize = CELL_SIZE;

/// Sentinel address used when no valid dictionary link or instruction pointer is available
///
/// A dedicated invalid value lets the VM represent "no active word" and "no prior link" without
/// needing optional address storage in the core machine state.
pub const NO_ADDRESS: Address = 0xFFFF;

/// First address reserved for task-local user variables.
///
/// The initial VM has one task, but the user area keeps interpreter state such as `BASE` separate
/// from dictionary allocation so later multitasking support can give each task its own copy.
pub const USER_AREA_START: Address = 0x0000;

/// Number of task-local user-variable cells reserved at the bottom of VM memory.
pub const USER_AREA_CELLS: usize = 16;

/// Size of the reserved user-variable area in memory words.
pub const USER_AREA_SIZE: usize = USER_AREA_CELLS * CELL_SIZE;

/// Address of the cell containing the current number-conversion radix.
pub const BASE_ADDRESS: Address = USER_AREA_START;

/// Default number-conversion radix stored in [`BASE_ADDRESS`].
pub const DEFAULT_BASE: Cell = 10;

/// First address available to the dictionary
///
/// The dictionary starts immediately after the user-variable area and grows upward through the free
/// space below the input buffer and stack arena.
pub const DICTIONARY_START: Address = USER_AREA_START + USER_AREA_SIZE as Address;

/// Start the address of the terminal input buffer
///
/// The source buffer lives in VM memory because it is part of the interpreter state, not host-only
/// scaffolding.
pub const TIB_START: Address = 0xE000;

/// Size of the terminal input buffer in memory words.
pub const TIB_SIZE: usize = 0x0100;

/// First address after the terminal input buffer.
pub const TIB_END: Address = TIB_START + TIB_SIZE as Address;

/// Initial return stack pointer; the return stack grows upward from here
///
/// This places the return stack at the low end of the shared stack arena so it can grow toward the
/// downward-growing data stack.
pub const RETURN_STACK_BASE: Address = TIB_END;

/// Initial data stack pointer; the data stack grows downward from here.
///
/// This is the empty-stack pointer, so the first pushed cell lands immediately below the reserved
/// input/output region.
pub const DATA_STACK_BASE: Address = IO_REGION_BASE;

/// Start with the reserved input/output region at the top of VM memory
///
/// This reservation is part of the machine layout regardless of whether the active device model is
/// direct memory-mapped I/O, UART over memory-mapped I/O, or a separate port address space.
pub const IO_REGION_BASE: Address = 0xFF00;

/// Size of the reserved input/output region in memory words.
pub const IO_REGION_SIZE: usize = 0x0100;

/// First address after the reserved input/output region.
pub const IO_REGION_END: usize = IO_REGION_BASE as usize + IO_REGION_SIZE;

/// Generic true value used by simple device-status constants in this VM profile
///
/// This is intentionally separated from Forth boolean semantics, which use all-bits-set rather than
/// `0x01`.
pub const TRUE: Cell = 0x01;

/// Status value returned by direct non-UART key-ready probes
///
/// Direct MMIO and direct port I/O expose a single ready/not-ready value. UART-backed builds use
/// line-status register bits instead, so this constant is not part of the UART configuration.
#[cfg(not(feature = "vm-uart"))]
pub const KEY_READY: Cell = TRUE;

/// Value returned for reads from unmapped input/output locations
///
/// Returning a stable zero keeps unknown device addresses harmless in this early VM profile.
pub const UNKNOWN_IO_VALUE: Cell = 0x00;

/// Direct memory-mapped input/output address that reads one byte from [`ForthIo::key`].
#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
pub const DIRECT_MMIO_KEY_ADDR: Address = IO_REGION_BASE;

/// Direct memory-mapped input/output address that writes one byte to [`ForthIo::emit`].
#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
pub const DIRECT_MMIO_EMIT_ADDR: Address = IO_REGION_BASE + 1;

/// Direct memory-mapped input/output address that reports whether a key is ready.
#[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
pub const DIRECT_MMIO_KEY_READY_ADDR: Address = IO_REGION_BASE + 2;

/// Direct input/output port that reads one byte from [`ForthIo::key`].
#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
pub const DIRECT_KEY_PORT: Address = 0x0000;

/// Direct input/output port that writes one byte to [`ForthIo::emit`].
#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
pub const DIRECT_EMIT_PORT: Address = 0x0001;

/// Direct input/output port that reports whether a key is ready.
#[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
pub const DIRECT_KEY_READY_PORT: Address = 0x0002;

/// Base port of the universal asynchronous receiver-transmitter register block in port
/// input/output mode.
#[cfg(feature = "vm-uart")]
pub const UART_PORT_BASE: Address = 0x03F8;

/// Universal asynchronous receiver-transmitter offset for the receive buffer register on read and
/// transmit holding register on a "write".
#[cfg(feature = "vm-uart")]
pub const UART_RBR_THR_OFFSET: Address = 0x0000;

/// Universal asynchronous receiver-transmitter offset for the line status register.
#[cfg(feature = "vm-uart")]
pub const UART_LSR_OFFSET: Address = 0x0005;

/// Universal asynchronous receiver-transmitter memory-mapped input/output address for receive
/// buffer reads and transmit holding register writes.
#[cfg(feature = "vm-uart")]
pub const UART_RBR_THR_ADDR: Address = IO_REGION_BASE + UART_RBR_THR_OFFSET;

/// Universal asynchronous receiver-transmitter memory-mapped input/output address for the line
/// status register.
#[cfg(feature = "vm-uart")]
pub const UART_LSR_ADDR: Address = IO_REGION_BASE + UART_LSR_OFFSET;

/// Universal asynchronous receiver-transmitter port for receive buffer reads and transmit holding
/// register writes.
#[cfg(feature = "vm-uart")]
pub const UART_RBR_THR_PORT: Address = UART_PORT_BASE + UART_RBR_THR_OFFSET;

/// Universal asynchronous receiver-transmitter port for the line status register.
#[cfg(feature = "vm-uart")]
pub const UART_LSR_PORT: Address = UART_PORT_BASE + UART_LSR_OFFSET;

/// Universal asynchronous receiver-transmitter line status bit indicating receive data is ready.
#[cfg(feature = "vm-uart")]
pub const UART_LSR_DATA_READY: Cell = 0x01;

/// Universal asynchronous receiver-transmitter line status bit indicating the transmit buffer
/// register is empty.
#[cfg(feature = "vm-uart")]
pub const UART_LSR_THR_EMPTY: Cell = 0x20;

/// Initial universal asynchronous receiver-transmitter line status value exposed by the host-backed
/// VM.
#[cfg(feature = "vm-uart")]
pub const UART_LSR_READY: Cell = UART_LSR_DATA_READY | UART_LSR_THR_EMPTY;

/// Offset of the dictionary link field from the start of a dictionary entry.
pub const DICTIONARY_LINK_OFFSET: usize = 0;

/// Offset of the dictionary flag byte from the start of a dictionary entry.
pub const DICTIONARY_FLAGS_OFFSET: usize = CELL_SIZE;

/// Offset of the dictionary name-length byte from the start of a dictionary entry.
pub const DICTIONARY_NAME_LENGTH_OFFSET: usize = DICTIONARY_FLAGS_OFFSET + 1;

/// Offset of the first dictionary name byte from the start of a dictionary entry.
pub const DICTIONARY_NAME_BYTES_OFFSET: usize = DICTIONARY_NAME_LENGTH_OFFSET + 1;

/// The flag bit marking a dictionary word as immediate.
pub const WORD_FLAG_IMMEDIATE: MemoryWord = 0x01;

/// The flag bit marking a dictionary word as a primitive handler rather than a colon definition.
pub const WORD_FLAG_PRIMITIVE: MemoryWord = 0x02;

/// Marker stored in a code field address (CFA) for a colon definition
///
/// The code field address is the dictionary cell that tells the inner interpreter how to execute a
/// word. Primitive words store an encoded primitive identifier there; colon definitions store this
/// dedicated marker and dispatch through `DOCOL`.
pub const DOCOL_CODE_FIELD: Cell = -1;

/// Zero value used when initializing or clearing VM byte-addressable memory.
const MEMORY_WORD_ZERO: MemoryWord = 0x00;

/// Forth interpreter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpreterState {
    /// Input tokens are executed immediately.
    Interpreting,
    /// Input tokens are compiled into the dictionary.
    Compiling,
}

/// Stack selector used in stack-related VM errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackKind {
    /// The parameter or data stack.
    Data,
    /// The return stack.
    Return,
}

/// Error returned by checked VM operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    /// Address arithmetic would exceed the virtual address space.
    InvalidAddress,
    /// A cell operation was attempted at an address that is not cell-aligned.
    UnalignedCell,
    /// A stack push would make the opposing stacks collide.
    StackOverflow(StackKind),
    /// A stack pop was attempted while the selected stack was empty.
    StackUnderflow(StackKind),
    /// Input was too large for the terminal input buffer.
    TibOverflow,
    /// Dictionary allocation would collide with the terminal input buffer or exceed dictionary
    /// encoding limits.
    DictionaryOverflow,
    /// A dictionary header was malformed.
    InvalidDictionaryEntry,
    /// An execution token referred to an unknown primitive handler.
    UnknownPrimitive,
    /// The active input source reached end-of-input.
    EndOfInput,
    /// The active input source failed unexpectedly.
    IoError,
    /// A source-level definition or compile request was malformed.
    InvalidSource,
    /// A source-level numeric literal could not be parsed.
    InvalidNumber,
    /// The `BASE` user variable is outside the supported radix range.
    InvalidBase,
    /// A source-level abort or failure word requested termination.
    Abort,
}

/// Metadata returned for a dictionary entry
///
/// Tests and dictionary helpers use this struct so header layout can be inspected without
/// re-decoding the same address math at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryEntry {
    /// Address of the dictionary header start; this is also the execution token (XT) used here. The
    /// execution token is the address used by threaded code to refer to a word.
    pub execution_token: Address,
    /// Address of the link field.
    pub link_address: Address,
    /// Address of the code field address (CFA) that identifies the primitive handler or colon-entry
    /// behavior for the word.
    pub code_field_address: Address,
    /// Address of the parameter field.
    pub parameter_field_address: Address,
    /// Encoded flags byte.
    pub flags: MemoryWord,
    /// Dictionary name length in bytes.
    pub name_len: MemoryWord,
}

/// Forth virtual machine state
///
/// The VM keeps all architecturally visible states in one flat address space, so the implementation
/// behaves like a small real machine instead of a collection of disconnected host-side structures.
/// Dictionary bytes, the terminal input buffer, the data stack, the return stack, dictionary
/// entries, and memory-mapped input/output all use addresses within that array's address space.
/// Stack values are stored as little-endian [`Cell`] values in VM memory.
pub struct ForthVm<I: ForthIo> {
    /// Flat virtual memory backing store.
    memory: [MemoryWord; MEMORY_SIZE],
    /// Compile pointer register. This is the VM state corresponding to Forth `HERE` (P).
    pub compile_pointer: Address,
    /// Instruction pointer register for threaded execution (IP).
    pub instruction_pointer: Address,
    /// Work register holding the current execution token (W, XT).
    pub working_register: Address,
    /// Data stack pointer; the data stack grows downward from [`DATA_STACK_BASE`].
    pub data_stack_pointer: Address,
    /// Return stack pointer; the return stack grows upward from [`RETURN_STACK_BASE`].
    pub return_stack_pointer: Address,
    /// Address of the most recently installed dictionary word, or [`NO_ADDRESS`] when the
    /// dictionary is empty.
    pub latest: Address,
    /// Current interpreter mode.
    pub state: InterpreterState,
    /// Execution token of the definition currently being compiled, or [`NO_ADDRESS`] when no
    /// definition is open.
    pub current_definition: Address,
    /// Start the address of the terminal input buffer.
    pub tib_start: Address,
    /// Number of bytes currently loaded into the terminal input buffer.
    pub tib_len: usize,
    /// Parse offset into the terminal input buffer, equivalent to Forth `>IN`.
    pub input_pos: usize,
    /// Host input/output backend used by memory-mapped input/output or port dispatch.
    pub io: I,
}

impl<I: ForthIo> ForthVm<I> {
    /// Construct a VM with zeroed memory and the default address layout
    ///
    /// The VM starts in interpreting mode with empty stacks, an empty terminal input buffer, and
    /// dictionary allocation beginning at [`DICTIONARY_START`].
    pub fn new(io: I) -> Self {
        let mut memory = [MEMORY_WORD_ZERO; MEMORY_SIZE];
        let base_start = BASE_ADDRESS as usize;
        let base_bytes = DEFAULT_BASE.to_le_bytes();
        // Not const fn: copy_from_slice is not const-stable. Embedded static initialization will
        // need lazy initialization or linker-provided bytes if that target needs a static VM.
        memory[base_start..base_start + CELL_SIZE].copy_from_slice(&base_bytes);

        Self {
            memory,
            compile_pointer: DICTIONARY_START,
            instruction_pointer: NO_ADDRESS,
            working_register: NO_ADDRESS,
            data_stack_pointer: DATA_STACK_BASE,
            return_stack_pointer: RETURN_STACK_BASE,
            latest: NO_ADDRESS,
            state: InterpreterState::Interpreting,
            current_definition: NO_ADDRESS,
            tib_start: TIB_START,
            tib_len: 0,
            input_pos: 0,
            io,
        }
    }

    /// Return the full VM memory image.
    pub fn memory(&self) -> &[MemoryWord; MEMORY_SIZE] {
        &self.memory
    }

    /// Read one [`MemoryWord`] from virtual memory
    ///
    /// In memory-mapped input/output builds, reads from the input/output window are dispatched to
    /// the configured direct or universal asynchronous receiver-transmitter device model instead of
    /// returning the backing array contents.
    pub fn read_memory_word(&mut self, address: Address) -> Result<MemoryWord, VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_io_region_address(address) {
            return self.read_mmio(address).map(|value| value as MemoryWord);
        }

        Ok(self.memory[address_index(address)])
    }

    /// Write one [`MemoryWord`] to virtual memory
    ///
    /// In memory-mapped input/output builds, writes to the input/output window are dispatched to the
    /// configured direct or universal asynchronous receiver-transmitter device model instead of
    /// mutating the backing array.
    pub fn write_memory_word(
        &mut self,
        address: Address,
        value: MemoryWord,
    ) -> Result<(), VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_io_region_address(address) {
            self.write_mmio(address, Cell::from(value));
            return Ok(());
        }

        self.memory[address_index(address)] = value;
        Ok(())
    }

    /// Read one little-endian [`Cell`] from virtual memory
    ///
    /// Non-input/output cell reads must be aligned to [`CELL_ALIGN`]. In memory-mapped
    /// input/output builds, cell reads from the input/output window are dispatched as input/output
    /// reads and may target byte-wide device registers at unaligned addresses.
    pub fn read_cell(&mut self, address: Address) -> Result<Cell, VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_io_region_address(address) {
            return self.read_mmio(address);
        }

        let start = checked_cell_start(address)?;
        let mut bytes = [0u8; CELL_SIZE];
        bytes.copy_from_slice(&self.memory[start..start + CELL_SIZE]);
        Ok(Cell::from_le_bytes(bytes))
    }

    /// Write one little-endian [`Cell`] to virtual memory
    ///
    /// Non-input/output cell writes must be aligned to [`CELL_ALIGN`]. In memory-mapped
    /// input/output builds, cell writes to the input/output window are dispatched as input/output
    /// writes and may target byte-wide device registers at unaligned addresses.
    pub fn write_cell(&mut self, address: Address, value: Cell) -> Result<(), VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_io_region_address(address) {
            self.write_mmio(address, value);
            return Ok(());
        }

        let start = checked_cell_start(address)?;
        self.memory[start..start + CELL_SIZE].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Read an address stored in one dictionary or threaded-code cell
    ///
    /// This helper converts the cell payload into the VM's address type after the underlying cell
    /// read succeeds.
    pub fn read_address(&mut self, address: Address) -> Result<Address, VmError> {
        self.read_cell(address)?
            .try_into()
            .map_err(|_| VmError::InvalidAddress)
    }

    /// Write an address into one dictionary or threaded-code cell.
    pub fn write_address(&mut self, address: Address, value: Address) -> Result<(), VmError> {
        self.write_cell(address, Cell::from(value))
    }

    /// Return the current number-conversion radix stored in `BASE`.
    pub fn base(&mut self) -> Result<Cell, VmError> {
        self.read_cell(BASE_ADDRESS)
    }

    /// Return the current number-conversion radix after validating the supported range.
    pub fn validated_base(&mut self) -> Result<u32, VmError> {
        let base = self.base()?;
        if !(2..=36).contains(&base) {
            return Err(VmError::InvalidBase);
        }
        Ok(base as u32)
    }

    /// Push a cell onto the data stack.
    pub fn push_data(&mut self, value: Cell) -> Result<(), VmError> {
        self.push_stack(StackKind::Data, value)
    }

    /// Pop a cell from the data stack.
    pub fn pop_data(&mut self) -> Result<Cell, VmError> {
        self.pop_stack(StackKind::Data)
    }

    /// Peek at the top cell on the data stack without changing the stack pointer.
    pub fn peek_data(&mut self) -> Result<Cell, VmError> {
        if self.data_stack_pointer == DATA_STACK_BASE {
            return Err(VmError::StackUnderflow(StackKind::Data));
        }
        self.read_cell(self.data_stack_pointer)
    }

    /// Push a cell onto the return stack.
    pub fn push_return(&mut self, value: Cell) -> Result<(), VmError> {
        self.push_stack(StackKind::Return, value)
    }

    /// Pop a cell from the return stack.
    pub fn pop_return(&mut self) -> Result<Cell, VmError> {
        self.pop_stack(StackKind::Return)
    }

    /// Reset the return stack pointer to its base.
    pub fn reset_return_stack(&mut self) {
        self.return_stack_pointer = RETURN_STACK_BASE;
    }

    /// Reset the outer-interpreter observable state used by non-local control transfers
    ///
    /// This helper does not itself request control transfer; it restores the machine state that
    /// words such as `QUIT` and `ABORT` must leave behind before the host-side runner decides
    /// whether to prompt again or exit.
    pub fn reset_outer_interpreter_state(&mut self) {
        self.reset_return_stack();
        self.reset_tib();
        self.state = InterpreterState::Interpreting;
        self.current_definition = NO_ADDRESS;
        self.input_pos = 0;
        self.instruction_pointer = NO_ADDRESS;
        self.working_register = NO_ADDRESS;
    }

    /// Read one byte from the active input source.
    pub fn read_input_byte(&mut self) -> Result<MemoryWord, VmError> {
        match self.io.read_key() {
            InputEvent::Byte(byte) => Ok(byte),
            InputEvent::Eof => Err(VmError::EndOfInput),
            InputEvent::Error => Err(VmError::IoError),
        }
    }

    /// Push a cell onto the selected stack using the VM's shared stack arena rules.
    fn push_stack(&mut self, stack: StackKind, value: Cell) -> Result<(), VmError> {
        match stack {
            StackKind::Data => {
                let next_sp = self
                    .data_stack_pointer
                    .checked_sub(CELL_SIZE as Address)
                    .ok_or(VmError::StackOverflow(StackKind::Data))?;
                if next_sp < self.return_stack_pointer {
                    return Err(VmError::StackOverflow(StackKind::Data));
                }
                self.data_stack_pointer = next_sp;
                self.write_cell(self.data_stack_pointer, value)
            }
            StackKind::Return => {
                let next_rp = self
                    .return_stack_pointer
                    .checked_add(CELL_SIZE as Address)
                    .ok_or(VmError::StackOverflow(StackKind::Return))?;
                if next_rp > self.data_stack_pointer {
                    return Err(VmError::StackOverflow(StackKind::Return));
                }
                self.write_cell(self.return_stack_pointer, value)?;
                self.return_stack_pointer = next_rp;
                Ok(())
            }
        }
    }

    /// Pop a cell from the selected stack using the VM's shared stack arena rules.
    fn pop_stack(&mut self, stack: StackKind) -> Result<Cell, VmError> {
        match stack {
            StackKind::Data => {
                if self.data_stack_pointer == DATA_STACK_BASE {
                    return Err(VmError::StackUnderflow(StackKind::Data));
                }
                let value = self.read_cell(self.data_stack_pointer)?;
                self.data_stack_pointer += CELL_SIZE as Address;
                Ok(value)
            }
            StackKind::Return => {
                if self.return_stack_pointer == RETURN_STACK_BASE {
                    return Err(VmError::StackUnderflow(StackKind::Return));
                }
                self.return_stack_pointer -= CELL_SIZE as Address;
                self.read_cell(self.return_stack_pointer)
            }
        }
    }

    /// Load bytes into the terminal input buffer.
    pub fn load_tib(&mut self, input: &[MemoryWord]) -> Result<(), VmError> {
        if input.len() > TIB_SIZE {
            return Err(VmError::TibOverflow);
        }

        let start = address_index(self.tib_start);
        self.memory[start..start + input.len()].copy_from_slice(input);
        self.tib_len = input.len();
        self.input_pos = 0;
        Ok(())
    }

    /// Append one byte to the active terminal input buffer line
    ///
    /// The byte is written at the current end of the terminal input buffer. Newline handling is
    /// left to the outer interpreter; callers should only append bytes that belong to the current
    /// line content.
    pub fn append_tib_byte(&mut self, byte: MemoryWord) -> Result<(), VmError> {
        if self.tib_len >= TIB_SIZE {
            return Err(VmError::TibOverflow);
        }

        self.write_memory_word(
            self.tib_start
                .checked_add(self.tib_len as Address)
                .ok_or(VmError::TibOverflow)?,
            byte,
        )?;
        self.tib_len += 1;
        Ok(())
    }

    /// Remove the most recently appended terminal input byte
    ///
    /// Interactive backspace handling only edits the end of the current line, so decrementing the
    /// active length is enough. The old byte may remain in memory past `tib_len`, but it is no
    /// longer part of the current input line.
    pub fn remove_last_tib_byte(&mut self) -> bool {
        if self.tib_len == 0 {
            return false;
        }
        self.tib_len -= 1;
        if self.input_pos > self.tib_len {
            self.input_pos = self.tib_len;
        }
        true
    }

    /// Copy the next whitespace-delimited terminal input word into `scratch`
    ///
    /// Parsing starts at the current `>IN` offset and advances `input_pos` past the returned word.
    /// The returned slice borrows from `scratch`, not from VM memory, so callers may execute words
    /// immediately after this method returns without holding a borrow into the VM.
    pub fn next_tib_word<'a, const N: usize>(
        &mut self,
        scratch: &'a mut [MemoryWord; N],
    ) -> Result<Option<&'a [MemoryWord]>, VmError> {
        let start = address_index(self.tib_start);
        let tib = &self.memory[start..start + self.tib_len];
        let mut pos = self.input_pos;

        while pos < tib.len() && tib[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if pos == tib.len() {
            self.input_pos = pos;
            return Ok(None);
        }

        let token_start = pos;
        while pos < tib.len() && !tib[pos].is_ascii_whitespace() {
            pos += 1;
        }

        let token_len = pos - token_start;
        if token_len > scratch.len() {
            return Err(VmError::TibOverflow);
        }

        scratch[..token_len].copy_from_slice(&tib[token_start..pos]);
        self.input_pos = pos;
        Ok(Some(&scratch[..token_len]))
    }

    /// Mark the terminal input buffer as empty and reset the parse offset.
    pub fn reset_tib(&mut self) {
        self.tib_len = 0;
        self.input_pos = 0;
    }

    /// Reserve dictionary space and return the starting address
    ///
    /// This advances the compiler pointer register `p`.
    pub fn allot(&mut self, bytes: usize) -> Result<Address, VmError> {
        let start = self.compile_pointer;
        let next = address_index(start)
            .checked_add(bytes)
            .ok_or(VmError::DictionaryOverflow)?;
        if next > address_index(TIB_START) {
            return Err(VmError::DictionaryOverflow);
        }
        self.compile_pointer = next.try_into().map_err(|_| VmError::DictionaryOverflow)?;
        Ok(start)
    }

    /// Align the compiler pointer up to the next cell boundary.
    pub fn align_dictionary(&mut self) -> Result<(), VmError> {
        let aligned = align_up(address_index(self.compile_pointer), CELL_ALIGN);
        if aligned > address_index(TIB_START) {
            return Err(VmError::DictionaryOverflow);
        }
        self.compile_pointer = aligned
            .try_into()
            .map_err(|_| VmError::DictionaryOverflow)?;
        Ok(())
    }

    /// Install a primitive word into the dictionary and return its execution token (XT)
    ///
    /// The execution token is the header address that threaded code stores for later dispatch.
    pub fn install_primitive_word(
        &mut self,
        name: &str,
        primitive: Primitive,
        flags: MemoryWord,
    ) -> Result<Address, VmError> {
        let entry = self.begin_dictionary_entry(name, flags | WORD_FLAG_PRIMITIVE)?;
        self.write_cell(entry.code_field_address, primitive.code_field())?;
        self.finish_dictionary_entry(entry.execution_token, entry.parameter_field_address)?;
        Ok(entry.execution_token)
    }

    /// Install a colon word into the dictionary and return its execution token (XT)
    ///
    /// The supplied body is written verbatim into the parameter field as threaded cells.
    pub fn install_colon_word(
        &mut self,
        name: &str,
        body: &[Cell],
        flags: MemoryWord,
    ) -> Result<Address, VmError> {
        let entry = self.begin_dictionary_entry(name, flags)?;
        ensure_dictionary_write_fits(
            address_index(entry.parameter_field_address),
            body.len()
                .checked_mul(CELL_SIZE)
                .ok_or(VmError::DictionaryOverflow)?,
        )?;
        self.write_cell(entry.code_field_address, DOCOL_CODE_FIELD)?;
        let mut cursor = entry.parameter_field_address;
        for cell in body {
            self.write_cell(cursor, *cell)?;
            cursor = cursor
                .checked_add(CELL_SIZE as Address)
                .ok_or(VmError::DictionaryOverflow)?;
        }
        self.finish_dictionary_entry(entry.execution_token, cursor)?;
        Ok(entry.execution_token)
    }

    /// Begin compiling a colon definition, whose runtime code field is `DOCOL`.
    pub fn begin_colon_definition(&mut self, name: &[u8]) -> Result<Address, VmError> {
        if self.current_definition != NO_ADDRESS || self.state == InterpreterState::Compiling {
            return Err(VmError::InvalidSource);
        }
        let entry = self.begin_dictionary_entry(
            core::str::from_utf8(name).map_err(|_| VmError::InvalidSource)?,
            0,
        )?;
        self.write_cell(entry.code_field_address, DOCOL_CODE_FIELD)?;
        self.current_definition = entry.execution_token;
        self.compile_pointer = entry.parameter_field_address;
        self.state = InterpreterState::Compiling;
        Ok(entry.execution_token)
    }

    /// Append one threaded execution token to the open definition.
    pub fn compile_xt(&mut self, xt: Address) -> Result<(), VmError> {
        self.compile_cell(Cell::from(xt))
    }

    /// Append one literal cell to the open definition.
    pub fn compile_cell(&mut self, value: Cell) -> Result<(), VmError> {
        if self.current_definition == NO_ADDRESS {
            return Err(VmError::InvalidSource);
        }

        ensure_dictionary_write_fits(address_index(self.compile_pointer), CELL_SIZE)?;
        self.write_cell(self.compile_pointer, value)?;
        self.compile_pointer = self
            .compile_pointer
            .checked_add(CELL_SIZE as Address)
            .ok_or(VmError::DictionaryOverflow)?;
        Ok(())
    }

    /// Finalize the open colon definition and publish it in the dictionary chain.
    pub fn finish_colon_definition(&mut self) -> Result<Address, VmError> {
        let working_register = self.current_definition;
        if working_register == NO_ADDRESS {
            return Err(VmError::InvalidSource);
        }
        self.finish_dictionary_entry(working_register, self.compile_pointer)?;
        self.current_definition = NO_ADDRESS;
        self.state = InterpreterState::Interpreting;
        Ok(working_register)
    }

    /// Read dictionary metadata for a word given its execution token (XT)
    ///
    /// The execution token is the start address of the dictionary header in this VM.
    pub fn dictionary_entry(
        &mut self,
        working_register: Address,
    ) -> Result<DictionaryEntry, VmError> {
        let flags_address = working_register
            .checked_add(DICTIONARY_FLAGS_OFFSET as Address)
            .ok_or(VmError::InvalidAddress)?;
        let name_length_address = working_register
            .checked_add(DICTIONARY_NAME_LENGTH_OFFSET as Address)
            .ok_or(VmError::InvalidAddress)?;
        let name_length = self.read_memory_word(name_length_address)?;
        let code_field_address_offset = aligned_code_field_offset(name_length as usize)?;
        let code_field_address = working_register
            .checked_add(code_field_address_offset as Address)
            .ok_or(VmError::InvalidAddress)?;
        let parameter_field_address = code_field_address
            .checked_add(CELL_SIZE as Address)
            .ok_or(VmError::InvalidAddress)?;
        Ok(DictionaryEntry {
            execution_token: working_register,
            link_address: working_register
                .checked_add(DICTIONARY_LINK_OFFSET as Address)
                .ok_or(VmError::InvalidAddress)?,
            code_field_address,
            parameter_field_address,
            flags: self.read_memory_word(flags_address)?,
            name_len: name_length,
        })
    }

    /// Find the most recent dictionary word whose name matches the supplied ASCII token.
    pub fn find_word(&mut self, target_name: &[u8]) -> Result<Option<Address>, VmError> {
        let mut cursor = self.latest;

        while cursor != NO_ADDRESS {
            if self.word_name_matches(cursor, target_name)? {
                return Ok(Some(cursor));
            }

            let entry = self.dictionary_entry(cursor)?;
            cursor = self.read_address(entry.link_address)?;
        }

        Ok(None)
    }

    /// Return whether the dictionary word addressed by `xt` is marked immediate.
    pub fn word_is_immediate(&mut self, xt: Address) -> Result<bool, VmError> {
        let entry = self.dictionary_entry(xt)?;
        Ok((entry.flags & WORD_FLAG_IMMEDIATE) != 0)
    }

    /// Execute the next word in the current threaded code stream
    ///
    /// This mirrors the traditional `NEXT` inner-interpreter step without using the `next` method
    /// name reserved by Rust's iterator convention.
    pub fn execute_next_threaded_word(&mut self) -> Result<Control, VmError> {
        let next_xt = self.read_address(self.instruction_pointer)?;
        self.instruction_pointer = self
            .instruction_pointer
            .checked_add(CELL_SIZE as Address)
            .ok_or(VmError::InvalidAddress)?;
        self.working_register = next_xt;
        self.execute_current()
    }

    /// Execute the word referenced by the current work register
    ///
    /// The work register holds the current execution token while the code field determines whether
    /// the word is a primitive or a colon definition.
    pub fn execute_current(&mut self) -> Result<Control, VmError> {
        let entry = self.dictionary_entry(self.working_register)?;
        let code_field = self.read_cell(entry.code_field_address)?;
        if code_field == DOCOL_CODE_FIELD {
            return Primitive::Docol.execute(self);
        }
        let primitive = Primitive::from_code_field(code_field).ok_or(VmError::UnknownPrimitive)?;
        primitive.execute(self)
    }

    /// Execute a word until it completes or requests a non-local outer-interpreter transfer
    ///
    /// A return value of [`Control::Continue`] means the word ran to ordinary completion. Other
    /// control values are propagated to the outer interpreter.
    pub fn run_word(&mut self, xt: Address) -> Result<Control, VmError> {
        self.instruction_pointer = NO_ADDRESS;
        self.working_register = xt;
        let mut control = self.execute_current()?;
        if control != Control::Continue {
            return Ok(control);
        }

        while self.instruction_pointer != NO_ADDRESS {
            control = self.execute_next_threaded_word()?;
            if control != Control::Continue {
                return Ok(control);
            }
        }

        self.working_register = NO_ADDRESS;
        Ok(Control::Continue)
    }

    /// Begin encoding one dictionary header and return the derived entry metadata.
    fn begin_dictionary_entry(
        &mut self,
        name: &str,
        flags: MemoryWord,
    ) -> Result<DictionaryEntry, VmError> {
        if name.len() > MemoryWord::MAX as usize {
            return Err(VmError::DictionaryOverflow);
        }

        let execution_token = self.compile_pointer;
        let code_field_address_offset = aligned_code_field_offset(name.len())?;
        let code_field_address_index = address_index(execution_token)
            .checked_add(code_field_address_offset)
            .ok_or(VmError::DictionaryOverflow)?;
        let parameter_field_address_index = code_field_address_index
            .checked_add(CELL_SIZE)
            .ok_or(VmError::DictionaryOverflow)?;
        ensure_dictionary_write_fits(
            address_index(execution_token),
            parameter_field_address_index - address_index(execution_token),
        )?;
        let link_address = execution_token;
        self.write_address(link_address, self.latest)?;
        self.write_memory_word(
            execution_token
                .checked_add(DICTIONARY_FLAGS_OFFSET as Address)
                .ok_or(VmError::InvalidAddress)?,
            flags,
        )?;
        self.write_memory_word(
            execution_token
                .checked_add(DICTIONARY_NAME_LENGTH_OFFSET as Address)
                .ok_or(VmError::InvalidAddress)?,
            name.len() as MemoryWord,
        )?;

        let name_start = execution_token
            .checked_add(DICTIONARY_NAME_BYTES_OFFSET as Address)
            .ok_or(VmError::InvalidAddress)?;
        for (index, byte) in name.as_bytes().iter().enumerate() {
            let address = name_start
                .checked_add(index as Address)
                .ok_or(VmError::InvalidAddress)?;
            self.write_memory_word(address, *byte)?;
        }

        let code_field_address = execution_token
            .checked_add(code_field_address_offset as Address)
            .ok_or(VmError::InvalidAddress)?;
        let parameter_field_address = code_field_address
            .checked_add(CELL_SIZE as Address)
            .ok_or(VmError::InvalidAddress)?;

        Ok(DictionaryEntry {
            execution_token,
            link_address,
            code_field_address,
            parameter_field_address,
            flags,
            name_len: name.len() as MemoryWord,
        })
    }

    /// Finalize one dictionary entry by advancing the compiler pointer and the latest link.
    fn finish_dictionary_entry(&mut self, xt: Address, next_p: Address) -> Result<(), VmError> {
        if next_p as usize > address_index(TIB_START) {
            return Err(VmError::DictionaryOverflow);
        }
        self.latest = xt;
        self.compile_pointer = next_p;
        Ok(())
    }

    /// Compare a dictionary name to an ASCII token using case-insensitive matching.
    fn word_name_matches(&mut self, xt: Address, target_name: &[u8]) -> Result<bool, VmError> {
        let entry = self.dictionary_entry(xt)?;
        if usize::from(entry.name_len) != target_name.len() {
            return Ok(false);
        }

        for (offset, expected_byte) in target_name.iter().enumerate() {
            let addr = xt
                .checked_add(DICTIONARY_NAME_BYTES_OFFSET as Address)
                .and_then(|start| start.checked_add(offset as Address))
                .ok_or(VmError::InvalidAddress)?;
            let actual_byte = self.read_memory_word(addr)?;
            if !actual_byte.eq_ignore_ascii_case(expected_byte) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Read a cell from an input/output port.
    #[cfg(feature = "vm-port-io")]
    pub fn port_in(&mut self, port: Address) -> Result<Cell, VmError> {
        #[cfg(feature = "vm-uart")]
        {
            self.uart_port_in(port)
        }

        #[cfg(not(feature = "vm-uart"))]
        {
            self.direct_port_in(port)
        }
    }

    /// Write a cell to an input/output port.
    #[cfg(feature = "vm-port-io")]
    pub fn port_out(&mut self, port: Address, value: Cell) {
        #[cfg(feature = "vm-uart")]
        {
            self.uart_port_out(port, value);
        }

        #[cfg(not(feature = "vm-uart"))]
        {
            self.direct_port_out(port, value);
        }
    }

    #[cfg(not(feature = "vm-port-io"))]
    fn read_mmio(&mut self, address: Address) -> Result<Cell, VmError> {
        #[cfg(feature = "vm-uart")]
        {
            self.uart_mmio_read(address)
        }

        #[cfg(not(feature = "vm-uart"))]
        {
            self.direct_mmio_read(address)
        }
    }

    #[cfg(not(feature = "vm-port-io"))]
    fn write_mmio(&mut self, address: Address, value: Cell) {
        #[cfg(feature = "vm-uart")]
        {
            self.uart_mmio_write(address, value);
        }

        #[cfg(not(feature = "vm-uart"))]
        {
            self.direct_mmio_write(address, value);
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
    fn direct_mmio_read(&mut self, address: Address) -> Result<Cell, VmError> {
        match address {
            DIRECT_MMIO_KEY_ADDR => self.read_input_byte().map(Cell::from),
            DIRECT_MMIO_KEY_READY_ADDR => Ok(KEY_READY),
            _ => Ok(UNKNOWN_IO_VALUE),
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
    fn direct_mmio_write(&mut self, address: Address, value: Cell) {
        if address == DIRECT_MMIO_EMIT_ADDR {
            self.io.emit(value as u8);
        }
    }

    #[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
    fn direct_port_in(&mut self, port: Address) -> Result<Cell, VmError> {
        match port {
            DIRECT_KEY_PORT => self.read_input_byte().map(Cell::from),
            DIRECT_KEY_READY_PORT => Ok(KEY_READY),
            _ => Ok(UNKNOWN_IO_VALUE),
        }
    }

    #[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
    fn direct_port_out(&mut self, port: Address, value: Cell) {
        if port == DIRECT_EMIT_PORT {
            self.io.emit(value as u8);
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
    fn uart_mmio_read(&mut self, address: Address) -> Result<Cell, VmError> {
        match address {
            UART_RBR_THR_ADDR => self.read_input_byte().map(Cell::from),
            UART_LSR_ADDR => Ok(UART_LSR_READY),
            _ => Ok(UNKNOWN_IO_VALUE),
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
    fn uart_mmio_write(&mut self, address: Address, value: Cell) {
        if address == UART_RBR_THR_ADDR {
            self.io.emit(value as u8);
        }
    }

    #[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
    fn uart_port_in(&mut self, port: Address) -> Result<Cell, VmError> {
        match port {
            UART_RBR_THR_PORT => self.read_input_byte().map(Cell::from),
            UART_LSR_PORT => Ok(UART_LSR_READY),
            _ => Ok(UNKNOWN_IO_VALUE),
        }
    }

    #[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
    fn uart_port_out(&mut self, port: Address, value: Cell) {
        if port == UART_RBR_THR_PORT {
            self.io.emit(value as u8);
        }
    }
}

/// Convert a VM address into a Rust slice index.
pub const fn address_index(address: Address) -> usize {
    address as usize
}

/// Return whether an address falls inside the reserved top-of-memory input/output region.
pub const fn is_io_region_address(address: Address) -> bool {
    address as usize >= IO_REGION_BASE as usize && (address as usize) < IO_REGION_END
}

/// Return whether an address satisfies the VM cell alignment rule.
pub const fn cell_aligned(address: Address) -> bool {
    (address as usize).is_multiple_of(CELL_ALIGN)
}

/// Round an index up to the next multiple of `alignment`.
pub const fn align_up(index: usize, alignment: usize) -> usize {
    index.next_multiple_of(alignment)
}

/// Return the aligned offset, from the start of a dictionary entry, of the code field address
/// (CFA).
pub fn aligned_code_field_offset(name_len: usize) -> Result<usize, VmError> {
    let offset = DICTIONARY_NAME_BYTES_OFFSET
        .checked_add(name_len)
        .ok_or(VmError::DictionaryOverflow)?;
    Ok(align_up(offset, CELL_ALIGN))
}

/// Reject dictionary writes that would reach into the terminal input buffer region.
fn ensure_dictionary_write_fits(start: usize, bytes: usize) -> Result<(), VmError> {
    let end = start
        .checked_add(bytes)
        .ok_or(VmError::DictionaryOverflow)?;
    if end > address_index(TIB_START) {
        return Err(VmError::DictionaryOverflow);
    }
    Ok(())
}

/// Validate a cell access address and return the backing-memory slice start index.
fn checked_cell_start(address: Address) -> Result<usize, VmError> {
    if !cell_aligned(address) {
        return Err(VmError::UnalignedCell);
    }

    let start = address_index(address);
    if start > MEMORY_SIZE - CELL_SIZE {
        return Err(VmError::InvalidAddress);
    }

    Ok(start)
}
