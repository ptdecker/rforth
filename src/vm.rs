//! Forth virtual machine state and memory access.
//!
//! The VM models a small flat machine: dictionary space, terminal input buffer, data stack, return
//! stack, and optional I/O regions all live in one virtual address space.

use core::mem::size_of;

use crate::io::ForthIo;

/// Smallest addressable unit in VM memory.
///
/// The initial machine profile is byte-addressed, but VM code should use this alias instead of
/// spelling the concrete type directly.
pub type MemoryWord = u8;

/// Forth data cell stored on stacks and read/written by cell memory operations.
///
/// Primitive word execution will use this type for stack values.
pub type Cell = i64;

/// VM-visible memory address.
///
/// The address type determines the size of the flat virtual address space through [`MEMORY_SIZE`].
pub type Address = u16;

/// Number of addressable [`MemoryWord`] slots in virtual memory.
pub const MEMORY_SIZE: usize = Address::MAX as usize + 1;

/// Number of memory words occupied by one [`Cell`].
pub const CELL_SIZE: usize = size_of::<Cell>();

/// Required alignment, in memory words, for cell accesses.
pub const CELL_ALIGN: usize = CELL_SIZE;

/// First address available to the dictionary.
pub const DICTIONARY_START: Address = 0x0000;

/// Start the address of the terminal input buffer.
pub const TIB_START: Address = 0xE000;

/// Size of the terminal input buffer in memory words.
pub const TIB_SIZE: usize = 0x0100;

/// First address after the terminal input buffer.
pub const TIB_END: Address = TIB_START + TIB_SIZE as Address;

/// Lowest address in the shared stack arena.
pub const STACK_LIMIT: Address = TIB_END;

/// Initial return stack pointer; the return stack grows upward from here.
pub const RETURN_STACK_BASE: Address = STACK_LIMIT;

/// Highest address in the shared stack arena.
pub const STACK_BASE: Address = 0xFE00;

/// Initial data stack pointer; the data stack grows downward from here.
pub const DATA_STACK_BASE: Address = STACK_BASE;

/// Start the address of the memory-mapped I/O window.
pub const MMIO_BASE: Address = 0xFF00;

/// Size of the memory-mapped I/O window in memory words.
pub const MMIO_SIZE: usize = 0x0100;

/// First address after the memory-mapped I/O window.
pub const MMIO_END: usize = MMIO_BASE as usize + MMIO_SIZE;

/// Status value returned when input is considered ready.
pub const KEY_READY_TRUE: Cell = 1;

/// Value returned for reads from unmapped I/O locations.
pub const UNKNOWN_IO_VALUE: Cell = 0;

/// Direct MMIO address that reads one byte from [`ForthIo::key`].
pub const DIRECT_MMIO_KEY_ADDR: Address = MMIO_BASE;

/// Direct MMIO address that writes one byte to [`ForthIo::emit`].
pub const DIRECT_MMIO_EMIT_ADDR: Address = MMIO_BASE + 1;

/// Direct MMIO address that reports whether a key is ready.
pub const DIRECT_MMIO_KEY_READY_ADDR: Address = MMIO_BASE + 2;

/// Direct port that reads one byte from [`ForthIo::key`].
pub const DIRECT_KEY_PORT: Address = 0x0000;

/// Direct port that writes one byte to [`ForthIo::emit`].
pub const DIRECT_EMIT_PORT: Address = 0x0001;

/// Direct port that reports whether a key is ready.
pub const DIRECT_KEY_READY_PORT: Address = 0x0002;

/// Base address of the UART register block in memory-mapped I/O mode.
pub const UART_MMIO_BASE: Address = MMIO_BASE;

/// Base port of the UART register block in port I/O mode.
pub const UART_PORT_BASE: Address = 0x03F8;

/// UART offset for the receive buffer register on read and transmit holding register on write.
pub const UART_RBR_THR_OFFSET: Address = 0x0000;

/// UART offset for the line status register.
pub const UART_LSR_OFFSET: Address = 0x0005;

/// UART MMIO address for RBR reads and THR writes.
pub const UART_RBR_THR_ADDR: Address = UART_MMIO_BASE + UART_RBR_THR_OFFSET;

/// UART MMIO address for the line status register.
pub const UART_LSR_ADDR: Address = UART_MMIO_BASE + UART_LSR_OFFSET;

/// UART port for RBR reads and THR writes.
pub const UART_RBR_THR_PORT: Address = UART_PORT_BASE + UART_RBR_THR_OFFSET;

/// UART port for the line status register.
pub const UART_LSR_PORT: Address = UART_PORT_BASE + UART_LSR_OFFSET;

/// UART line status bit indicating receive data is ready.
pub const UART_LSR_DATA_READY: Cell = 0x01;

/// UART line status bit indicating the transmit buffer register is empty.
pub const UART_LSR_THR_EMPTY: Cell = 0x20;

/// Initial UART line status value exposed by the host-backed VM.
pub const UART_LSR_READY: Cell = UART_LSR_DATA_READY | UART_LSR_THR_EMPTY;

const MEMORY_WORD_ZERO: MemoryWord = 0;

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
    /// The parameter/data stack.
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
    /// Dictionary allocation would collide with the terminal input buffer.
    DictionaryOverflow,
}

/// Forth virtual machine state.
///
/// The VM owns one flat memory array. Dictionary bytes, the terminal input buffer, the data stack,
/// the return stack, and memory-mapped I/O all use addresses within that array's address space.
/// Stack values are stored as little-endian [`Cell`] values in VM memory.
pub struct ForthVm<I: ForthIo> {
    /// Flat virtual memory backing store.
    memory: [MemoryWord; MEMORY_SIZE],
    /// Dictionary pointer: the next free address for compiled data.
    pub here: Address,
    /// Instruction pointer for future primitive/threaded execution.
    pub ip: Address,
    /// Data stack pointer; the data stack grows downward from [`DATA_STACK_BASE`].
    pub sp: Address,
    /// Return stack pointer; the return stack grows upward from [`RETURN_STACK_BASE`].
    pub rp: Address,
    /// Current interpreter mode.
    pub state: InterpreterState,
    /// Start the address of the terminal input buffer.
    pub tib_start: Address,
    /// Number of bytes currently loaded into the terminal input buffer.
    pub tib_len: usize,
    /// Parse offset into the terminal input buffer, equivalent to Forth `>IN`.
    pub input_pos: usize,
    /// Host I/O backend used by MMIO or port dispatch.
    pub io: I,
}

impl<I: ForthIo> ForthVm<I> {
    /// Construct a VM with zeroed memory and the default address layout.
    ///
    /// The VM starts in interpreting mode with empty stacks, an empty terminal input buffer, and
    /// dictionary allocation beginning at [`DICTIONARY_START`].
    pub const fn new(io: I) -> Self {
        Self {
            memory: [MEMORY_WORD_ZERO; MEMORY_SIZE],
            here: DICTIONARY_START,
            ip: DICTIONARY_START,
            sp: DATA_STACK_BASE,
            rp: RETURN_STACK_BASE,
            state: InterpreterState::Interpreting,
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

    /// Read one [`MemoryWord`] from virtual memory.
    ///
    /// In memory-mapped I/O builds, reads from the MMIO window are dispatched to the configured
    /// direct or UART device model instead of returning the backing array contents.
    pub fn read_memory_word(&mut self, address: Address) -> Result<MemoryWord, VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_mmio_address(address) {
            return Ok(self.read_mmio(address) as MemoryWord);
        }

        Ok(self.memory[address_index(address)])
    }

    /// Write one [`MemoryWord`] to virtual memory.
    ///
    /// In memory-mapped I/O builds, writes to the MMIO window are dispatched to the configured
    /// direct or UART device model instead of mutating the backing array.
    pub fn write_memory_word(
        &mut self,
        address: Address,
        value: MemoryWord,
    ) -> Result<(), VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_mmio_address(address) {
            self.write_mmio(address, Cell::from(value));
            return Ok(());
        }

        self.memory[address_index(address)] = value;
        Ok(())
    }

    /// Read one little-endian [`Cell`] from virtual memory.
    ///
    /// Non-I/O cell reads must be aligned to [`CELL_ALIGN`]. In memory-mapped I/O builds, cell
    /// reads from the MMIO window are dispatched as I/O reads and may target byte-wide device
    /// registers at unaligned addresses.
    pub fn read_cell(&mut self, address: Address) -> Result<Cell, VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_mmio_address(address) {
            return Ok(self.read_mmio(address));
        }

        let start = checked_cell_start(address)?;
        let mut bytes = [0u8; CELL_SIZE];
        bytes.copy_from_slice(&self.memory[start..start + CELL_SIZE]);
        Ok(Cell::from_le_bytes(bytes))
    }

    /// Write one little-endian [`Cell`] to virtual memory.
    ///
    /// Non-I/O cell writes must be aligned to [`CELL_ALIGN`]. In memory-mapped I/O builds, cell
    /// writes to the MMIO window are dispatched as I/O writes and may target byte-wide device
    /// registers at unaligned addresses.
    pub fn write_cell(&mut self, address: Address, value: Cell) -> Result<(), VmError> {
        #[cfg(not(feature = "vm-port-io"))]
        if is_mmio_address(address) {
            self.write_mmio(address, value);
            return Ok(());
        }

        let start = checked_cell_start(address)?;
        self.memory[start..start + CELL_SIZE].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Push a cell onto the data stack.
    ///
    /// The value is written into VM memory and `sp` is moved downward by [`CELL_SIZE`].
    pub fn push_data(&mut self, value: Cell) -> Result<(), VmError> {
        self.push_stack(StackKind::Data, value)
    }

    /// Pop a cell from the data stack.
    ///
    /// The value is read from VM memory, and `sp` is moved upward by [`CELL_SIZE`].
    pub fn pop_data(&mut self) -> Result<Cell, VmError> {
        self.pop_stack(StackKind::Data)
    }

    /// Push a cell onto the return stack.
    ///
    /// The value is written into VM memory, and `rp` is moved upward by [`CELL_SIZE`].
    pub fn push_return(&mut self, value: Cell) -> Result<(), VmError> {
        self.push_stack(StackKind::Return, value)
    }

    /// Pop a cell from the return stack.
    ///
    /// `rp` is moved downward by [`CELL_SIZE`], and then the value is read from VM memory.
    pub fn pop_return(&mut self) -> Result<Cell, VmError> {
        self.pop_stack(StackKind::Return)
    }

    /// Push a cell onto the selected stack using the VM's shared stack arena rules.
    fn push_stack(&mut self, stack: StackKind, value: Cell) -> Result<(), VmError> {
        match stack {
            StackKind::Data => {
                let next_sp = self
                    .sp
                    .checked_sub(CELL_SIZE as Address)
                    .ok_or(VmError::StackOverflow(StackKind::Data))?;
                if next_sp < self.rp {
                    return Err(VmError::StackOverflow(StackKind::Data));
                }
                self.sp = next_sp;
                self.write_cell(self.sp, value)
            }
            StackKind::Return => {
                let write_addr = self.rp;
                let next_rp = self
                    .rp
                    .checked_add(CELL_SIZE as Address)
                    .ok_or(VmError::StackOverflow(StackKind::Return))?;
                if next_rp > self.sp {
                    return Err(VmError::StackOverflow(StackKind::Return));
                }
                self.write_cell(write_addr, value)?;
                self.rp = next_rp;
                Ok(())
            }
        }
    }

    /// Pop a cell from the selected stack using the VM's shared stack arena rules.
    fn pop_stack(&mut self, stack: StackKind) -> Result<Cell, VmError> {
        match stack {
            StackKind::Data => {
                if self.sp == DATA_STACK_BASE {
                    return Err(VmError::StackUnderflow(StackKind::Data));
                }
                let value = self.read_cell(self.sp)?;
                self.sp += CELL_SIZE as Address;
                Ok(value)
            }
            StackKind::Return => {
                if self.rp == RETURN_STACK_BASE {
                    return Err(VmError::StackUnderflow(StackKind::Return));
                }
                self.rp -= CELL_SIZE as Address;
                self.read_cell(self.rp)
            }
        }
    }

    /// Load bytes into the terminal input buffer.
    ///
    /// Existing TIB bytes beyond the new input length are left untouched; `tib_len` and
    /// `input_pos` define the active input range.
    pub fn load_tib(&mut self, input: &[MemoryWord]) -> Result<(), VmError> {
        if input.len() > TIB_SIZE {
            return Err(VmError::TibOverflow);
        }

        let start = address_index(self.tib_start);
        let end = start + input.len();
        self.memory[start..end].copy_from_slice(input);
        self.tib_len = input.len();
        self.input_pos = 0;
        Ok(())
    }

    /// Mark the terminal input buffer as empty and reset the parse offset.
    pub fn reset_tib(&mut self) {
        self.tib_len = 0;
        self.input_pos = 0;
    }

    /// Reserve dictionary space and return the starting address.
    ///
    /// This only advances `here`; callers are responsible for writing compiled bytes into the
    /// returned range.
    pub fn allot(&mut self, bytes: usize) -> Result<Address, VmError> {
        let start = self.here;
        let next = address_index(start)
            .checked_add(bytes)
            .ok_or(VmError::DictionaryOverflow)?;
        if next > address_index(TIB_START) {
            return Err(VmError::DictionaryOverflow);
        }
        self.here = next.try_into().map_err(|_| VmError::DictionaryOverflow)?;
        Ok(start)
    }

    #[cfg(feature = "vm-port-io")]
    /// Read a cell from an I/O port.
    ///
    /// This method exists only when the `vm-port-io` feature is enabled. With `vm-uart`, ports are
    /// interpreted as UART registers; otherwise they use the direct port model.
    pub fn port_in(&mut self, port: Address) -> Cell {
        #[cfg(feature = "vm-uart")]
        {
            self.uart_port_in(port)
        }

        #[cfg(not(feature = "vm-uart"))]
        {
            self.direct_port_in(port)
        }
    }

    #[cfg(feature = "vm-port-io")]
    /// Write a cell to an I/O port.
    ///
    /// This method exists only when the `vm-port-io` feature is enabled. With `vm-uart`, ports are
    /// interpreted as UART registers; otherwise they use the direct port model.
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
    fn read_mmio(&mut self, address: Address) -> Cell {
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
    fn direct_mmio_read(&mut self, address: Address) -> Cell {
        match address {
            DIRECT_MMIO_KEY_ADDR => Cell::from(self.io.key()),
            DIRECT_MMIO_KEY_READY_ADDR => KEY_READY_TRUE,
            _ => UNKNOWN_IO_VALUE,
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), not(feature = "vm-uart")))]
    fn direct_mmio_write(&mut self, address: Address, value: Cell) {
        if address == DIRECT_MMIO_EMIT_ADDR {
            self.io.emit(value as u8);
        }
    }

    #[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
    fn direct_port_in(&mut self, port: Address) -> Cell {
        match port {
            DIRECT_KEY_PORT => Cell::from(self.io.key()),
            DIRECT_KEY_READY_PORT => KEY_READY_TRUE,
            _ => UNKNOWN_IO_VALUE,
        }
    }

    #[cfg(all(feature = "vm-port-io", not(feature = "vm-uart")))]
    fn direct_port_out(&mut self, port: Address, value: Cell) {
        if port == DIRECT_EMIT_PORT {
            self.io.emit(value as u8);
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
    fn uart_mmio_read(&mut self, address: Address) -> Cell {
        match address {
            UART_RBR_THR_ADDR => Cell::from(self.io.key()),
            UART_LSR_ADDR => UART_LSR_READY,
            _ => UNKNOWN_IO_VALUE,
        }
    }

    #[cfg(all(not(feature = "vm-port-io"), feature = "vm-uart"))]
    fn uart_mmio_write(&mut self, address: Address, value: Cell) {
        if address == UART_RBR_THR_ADDR {
            self.io.emit(value as u8);
        }
    }

    #[cfg(all(feature = "vm-port-io", feature = "vm-uart"))]
    fn uart_port_in(&mut self, port: Address) -> Cell {
        match port {
            UART_RBR_THR_PORT => Cell::from(self.io.key()),
            UART_LSR_PORT => UART_LSR_READY,
            _ => UNKNOWN_IO_VALUE,
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

/// Return whether an address falls inside the memory-mapped I/O window.
pub const fn is_mmio_address(address: Address) -> bool {
    address as usize >= MMIO_BASE as usize && (address as usize) < MMIO_END
}

/// Return whether an address satisfies the VM cell alignment rule.
pub const fn cell_aligned(address: Address) -> bool {
    (address as usize).is_multiple_of(CELL_ALIGN)
}

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
