/*
 * ARM64 Exception Handlers
 *
 * This module provides Rust handlers for ARM64 exceptions and interrupts.
 */

use core::arch::asm;

// Import scheduler function
use super::scheduler::scheduler_switch_task;

// External functions from other modules (defined in gic.rs and timer.rs)
extern "C" {
    fn gic_acknowledge_interrupt() -> u32;
    fn gic_end_of_interrupt(irq_num: u32);
    fn timer_rearm();
}

/// Exception frame saved by the assembly exception handlers
#[repr(C)]
pub struct ExceptionFrame {
    // General purpose registers (x0-x29)
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,
    // Link register and stack pointer
    pub x30_lr: u64,
    pub sp_el0: u64,
    // Exception state
    pub elr_el1: u64,  // Exception link register (PC where exception occurred)
    pub spsr_el1: u64, // Saved processor state register
}

// Counter for timer ticks
static mut TIMER_TICKS: u64 = 0;

// Scheduler enabled flag
static mut SCHEDULER_ENABLED: bool = false;

/// Initialize exception handling
pub fn init() {
    unsafe {
        // Load the exception vector table address into VBAR_EL1
        let vector_table: u64;
        asm!(
            "adrp {0}, exception_vector_table",
            "add {0}, {0}, :lo12:exception_vector_table",
            out(reg) vector_table,
        );

        // Set VBAR_EL1 (Vector Base Address Register)
        asm!(
            "msr vbar_el1, {0}",
            in(reg) vector_table,
        );

        // Enable interrupts (clear I and F bits in DAIF)
        // DAIF: Debug, SError, IRQ, FIQ
        // We'll keep them masked for now until GIC is initialized
        // asm!("msr daifclr, #0b1111");  // Unmask all (commented out until GIC ready)
    }

    uart_puts("[EXCEPTIONS] Vector table initialized at 0x");
    uart_puts_hex(unsafe {
        let addr: u64;
        asm!(
            "adrp {0}, exception_vector_table",
            "add {0}, {0}, :lo12:exception_vector_table",
            out(reg) addr,
        );
        addr
    });
    uart_puts("\n");
}

/// Handle synchronous exceptions
#[no_mangle]
extern "C" fn handle_sync_exception(frame: &ExceptionFrame) {
    uart_puts("\n");
    uart_puts("╔════════════════════════════════════════════════════════╗\n");
    uart_puts("║           SYNCHRONOUS EXCEPTION                       ║\n");
    uart_puts("╚════════════════════════════════════════════════════════╝\n");
    uart_puts("\n");

    // Read ESR_EL1 (Exception Syndrome Register)
    let esr: u64;
    unsafe {
        asm!("mrs {0}, esr_el1", out(reg) esr);
    }

    let ec = (esr >> 26) & 0x3F; // Exception Class
    let iss = esr & 0x1FFFFFF;   // Instruction Specific Syndrome

    // Read FAR_EL1 (Fault Address Register) for data aborts
    let far: u64;
    unsafe {
        asm!("mrs {0}, far_el1", out(reg) far);
    }

    // Read SCTLR_EL1 to check if MMU is enabled
    let sctlr: u64;
    unsafe {
        asm!("mrs {0}, sctlr_el1", out(reg) sctlr);
    }

    // Decode DFSC (Data Fault Status Code) from ISS[5:0]
    let dfsc = iss & 0x3F;

    uart_puts("Exception Class: 0x");
    uart_puts_hex(ec);
    uart_puts("\n");
    uart_puts("ISS: 0x");
    uart_puts_hex(iss);
    uart_puts("\n");
    uart_puts("ELR_EL1 (PC): 0x");
    uart_puts_hex(frame.elr_el1);
    uart_puts("\n");
    uart_puts("FAR_EL1 (Fault Addr): 0x");
    uart_puts_hex(far);
    uart_puts("\n");
    uart_puts("X9 register: 0x");
    uart_puts_hex(frame.x9);
    uart_puts("\n");
    uart_puts("DFSC (Fault Status): 0x");
    uart_puts_hex(dfsc);
    uart_puts("\n");
    uart_puts("SCTLR_EL1 (MMU ctrl): 0x");
    uart_puts_hex(sctlr);
    uart_puts("\n");
    uart_puts("SPSR_EL1: 0x");
    uart_puts_hex(frame.spsr_el1);
    uart_puts("\n");

    // Halt on synchronous exceptions
    uart_puts("\n[EXCEPTION] System halted.\n");
    loop {
        unsafe { asm!("wfe"); }
    }
}

/// Handle IRQ interrupts
/// Returns the frame pointer to use for exception return (may be on different stack)
#[no_mangle]
extern "C" fn handle_irq(frame_ptr: *mut ExceptionFrame) -> *mut ExceptionFrame {
    unsafe {
        // Acknowledge interrupt and get IRQ number
        let irq_num = gic_acknowledge_interrupt();

        TIMER_TICKS += 1;

        // Print tick message every 100 ticks to avoid spam
        if TIMER_TICKS % 100 == 0 {
            uart_puts("[IRQ] Timer tick #");
            uart_puts_hex(TIMER_TICKS);
            uart_puts("\n");
        }

        // Re-arm the timer for next interrupt
        timer_rearm();

        // Signal end of interrupt to GIC
        gic_end_of_interrupt(irq_num);

        // If scheduler is enabled, switch tasks every 10 ticks (100ms)
        if SCHEDULER_ENABLED && TIMER_TICKS % 10 == 0 {
            scheduler_switch_task(frame_ptr)
        } else {
            frame_ptr
        }
    }
}

/// Handle FIQ (Fast Interrupt Request)
#[no_mangle]
extern "C" fn handle_fiq(_frame: &ExceptionFrame) {
    uart_puts("[FIQ] Fast interrupt received\n");
}

/// Handle SError (System Error)
#[no_mangle]
extern "C" fn handle_serror(frame: &ExceptionFrame) {
    uart_puts("\n");
    uart_puts("╔════════════════════════════════════════════════════════╗\n");
    uart_puts("║              SYSTEM ERROR (SError)                    ║\n");
    uart_puts("╚════════════════════════════════════════════════════════╝\n");
    uart_puts("\n");
    uart_puts("ELR_EL1 (PC): 0x");
    uart_puts_hex(frame.elr_el1);
    uart_puts("\n");

    uart_puts("\n[SERROR] System halted.\n");
    loop {
        unsafe { asm!("wfe"); }
    }
}

/// Get current timer tick count
pub fn get_timer_ticks() -> u64 {
    unsafe { TIMER_TICKS }
}

/// Enable the scheduler (task switching on timer interrupts)
pub fn enable_scheduler() {
    unsafe {
        SCHEDULER_ENABLED = true;
    }
    uart_puts("[EXCEPTIONS] Scheduler enabled in IRQ handler\n");
}

// Helper functions for UART output (inline to avoid dependency issues)

const UART_BASE: usize = 0x09000000;
const UART_DR: usize = UART_BASE + 0x00;
const UART_FR: usize = UART_BASE + 0x18;
const UART_FR_TXFF: u32 = 1 << 5;

fn uart_putc(c: u8) {
    unsafe {
        while (core::ptr::read_volatile(UART_FR as *const u32) & UART_FR_TXFF) != 0 {
            core::hint::spin_loop();
        }
        core::ptr::write_volatile(UART_DR as *mut u32, c as u32);
    }
}

fn uart_puts(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            uart_putc(b'\r');
        }
        uart_putc(byte);
    }
}

fn uart_puts_hex(mut val: u64) {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    let mut buf = [0u8; 16];

    for i in 0..16 {
        buf[15 - i] = HEX_CHARS[(val & 0xF) as usize];
        val >>= 4;
    }

    for &b in &buf {
        uart_putc(b);
    }
}
