//! PL011 UART Driver for ARM
//!
//! Provides serial output for QEMU virt machine

use core::fmt;
use core::ptr::{read_volatile, write_volatile};

/// PL011 UART base address (QEMU virt machine)
const UART_BASE: usize = 0x09000000;

/// UART registers
const UART_DR: usize = UART_BASE + 0x00;      // Data Register
const UART_FR: usize = UART_BASE + 0x18;      // Flag Register

/// Flag register bits
const UART_FR_TXFF: u32 = 1 << 5;  // Transmit FIFO full

/// PL011 UART driver
pub struct Uart {
    base: usize,
}

impl Uart {
    /// Create a new UART instance
    pub const fn new() -> Self {
        Uart { base: UART_BASE }
    }

    /// Initialize the UART
    ///
    /// For QEMU, the UART is already initialized by firmware
    pub fn init(&self) {
        // QEMU's UART is pre-configured, nothing to do
    }

    /// Write a byte to the UART
    fn write_byte(&self, byte: u8) {
        unsafe {
            // Wait while transmit FIFO is full
            while (read_volatile(UART_FR as *const u32) & UART_FR_TXFF) != 0 {
                core::hint::spin_loop();
            }

            // Write byte to data register
            write_volatile(UART_DR as *mut u32, byte as u32);
        }
    }

    /// Write a string to the UART
    fn write_string(&self, s: &str) {
        for byte in s.bytes() {
            // Convert \n to \r\n for proper line endings
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Global UART instance
pub static UART: spin::Mutex<Uart> = spin::Mutex::new(Uart::new());

/// Initialize UART
pub fn init() {
    UART.lock().init();
}

/// Write string to UART
pub fn write_str(s: &str) {
    UART.lock().write_string(s);
}

/// Print macro for ARM
#[macro_export]
macro_rules! uart_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::arch::aarch64::uart::UART.lock(), $($arg)*);
    }};
}

/// Println macro for ARM
#[macro_export]
macro_rules! uart_println {
    () => ($crate::uart_print!("\n"));
    ($($arg:tt)*) => ($crate::uart_print!("{}\n", format_args!($($arg)*)));
}
