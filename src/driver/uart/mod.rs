//! NS16550A UART driver.

use core::fmt::{Write, Result};

const UART_ADDRESS: usize = 0x10000000;
const LINE_STATUS_REGISTER: usize = 0x5;
const LINE_CONTROL_REGISTER: usize = 0x3;
const FIFO_CONTROL_REGISTER: usize = 0x2;
const INTERRUPT_ENABLE_REGISTER: usize = 0x1;

const LINE_STATUS_DATA_READY: u8 = 0x1;

#[derive(Copy, Clone)]
/// Struct representing a NS16550A UART peripheral
pub struct Uart {
    /// Base address of the peripheral
    address: usize,
}

#[derive(Copy, Clone)]
/// Word length
pub enum WordLength {
    FIVE = 0,
    SIX = 1,
    SEVEN = 2,
    EIGHT = 3,
}

#[derive(Copy, Clone)]
/// Number of stop bits
pub enum StopBits {
    ONE = 0,
    TWO = 1,
}

#[derive(Copy, Clone)]
/// Parity bits
pub enum ParityBit {
    DISABLE = 0,
    ENABLE = 1,
}

#[derive(Copy, Clone)]
/// Parity select
pub enum ParitySelect {
    EVEN = 0,
    ODD = 1,
}

#[derive(Copy, Clone)]
/// Stick parity
pub enum StickParity {
    DISABLE = 0,
    ENABLE = 1,
}

#[derive(Copy, Clone)]
/// Break
pub enum Break {
    DISABLE = 0,
    ENABLE = 1,
}

#[derive(Copy, Clone)]
/// Divisor latch access bit
pub enum DLAB {
    CLEAR = 0,
    SET = 1,
}

#[derive(Copy, Clone)]
/// DMA mode select
pub enum DMAMode {
    MODE0 = 0,
    MODE1 = 1,
}

impl Uart {
    pub const fn new(address: usize) -> Self {
        Uart {
            address
        }
    }

    /// Init the UART peripheral with default parameters:
    /// - `WordLength`: 8bits
    /// - `StopBits`: 1bit
    /// - `ParityBit`: Disable
    /// - `ParitySelect`: Even
    /// - `StickParity`: Disable
    /// - `SetBreak`: Disable
    /// - `DMAMode`: Mode0
    /// - `Divisor`: 592
    /// - `FIFO`: Enable
    /// - `ReceiverInterrupts`: Enable
    pub fn init_default(&self) {
        // If we cared about the divisor, the code below would set the divisor
        // from a global clock rate of 22.729 MHz (22,729,000 cycles per second)
        // to a signaling rate of 2400 (BAUD). We usually have much faster signalling
        // rates nowadays, but this demonstrates what the divisor actually does.
        // The formula given in the NS16500A specification for calculating the divisor
        // is:
        // divisor = ceil( (clock_hz) / (baud_sps x 16) )
        // So, we substitute our values and get:
        // divisor = ceil( 22_729_000 / (2400 x 16) )
        // divisor = ceil( 22_729_000 / 38_400 )
        // divisor = ceil( 591.901 ) = 592

        self.init(
            WordLength::EIGHT,
            StopBits::ONE,
            ParityBit::DISABLE,
            ParitySelect::EVEN,
            StickParity::DISABLE,
            Break::DISABLE,
            DMAMode::MODE0,
            592,
        );
    }

    /// Init UART peripheral with the given parameters.
    pub fn init(
        &self,
        word_length: WordLength,
        stop_bits: StopBits,
        parity_bit: ParityBit,
        parity_select: ParitySelect,
        stick_parity: StickParity,
        break_: Break,
        dma_mode: DMAMode,
        divisor: u16) {
        self.set_lcr(
            word_length,
            stop_bits,
            parity_bit,
            parity_select,
            stick_parity,
            break_,
            DLAB::SET,
        );
        self.set_fcr(dma_mode);
        self.set_ier();

        // Set divisor
        // The divisor register is two bytes (16 bits), so we need to split the value
        // into two bytes: address 0 writes the Least bits and address 1 writes the
        // Most bits.
        let divisor_least: u8 = (divisor & 0xff) as u8;
        let divisor_most: u8 = (divisor >> 8) as u8;
        let ptr = self.address as *mut u8;
        unsafe {
            ptr.write_volatile(divisor_least);
            ptr.add(1).write_volatile(divisor_most);
        }

        // Clear divisor latch accessor bit.
        self.set_lcr(
            word_length,
            stop_bits,
            parity_bit,
            parity_select,
            stick_parity,
            break_,
            DLAB::CLEAR,
        );
    }

    /// Sets the line control register with the given parameters.
    pub fn set_lcr(
        &self,
        word_length: WordLength,
        stop_bits: StopBits,
        parity_bit: ParityBit,
        parity_select: ParitySelect,
        stick_parity: StickParity,
        break_: Break,
        dlab: DLAB) {
        let ptr = (self.address + LINE_CONTROL_REGISTER) as *mut u8;
        unsafe {
            ptr.write_volatile(
                word_length as u8
                    | ((stop_bits as u8) << 2)
                    | ((parity_bit as u8) << 3)
                    | ((parity_select as u8) << 4)
                    | ((stick_parity as u8) << 5)
                    | ((break_ as u8) << 6)
                    | ((dlab as u8) << 7),
            );
        }
    }

    /// Sets the FIFO control register with the given parameter.
    pub fn set_fcr(&self, dma_mode: DMAMode) {
        let ptr = (self.address + FIFO_CONTROL_REGISTER) as *mut u8;
        unsafe {
            // Always enable FIFO(fcr[0])
            ptr.write_volatile(1 | ((dma_mode as u8) << 3));
        }
    }

    /// Sets the interrupt enable register.
    pub fn set_ier(&self) {
        let ptr = (self.address + INTERRUPT_ENABLE_REGISTER) as *mut u8;
        unsafe {
            // Always enable receiver interrupts(ier[0])
            ptr.write_volatile(1);
        }
    }

    /// Check if data ready bit is set.
    pub fn data_ready(&self) -> bool {
        let ptr = (self.address + LINE_STATUS_REGISTER) as *mut u8;
        unsafe {
            (ptr.read_volatile() & LINE_STATUS_DATA_READY) != 0
        }
    }

    /// If data ready is set, returns the value read in the receiver buffer register.
    /// Otherwise returns `None`.
    pub fn get(&self) -> Option<u8> {
        if self.data_ready() {
            let ptr = self.address as *mut u8;
            Some(unsafe { ptr.read_volatile() })
        } else {
            None
        }
    }

    pub fn put(&self, c: u8) {
        let ptr = self.address as *mut u8;
        unsafe { ptr.write_volatile(c); }
    }
}

impl Default for Uart {
    fn default() -> Self {
        Uart::new(UART_ADDRESS)
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> Result {
        s.bytes().for_each(|c| self.put(c));
        Ok(())
    }
}
