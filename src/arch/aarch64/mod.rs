//! AArch64 (ARM64) Architecture Support
//!
//! This module provides ARM64-specific implementations

pub mod uart;
pub mod mmu;
pub mod exceptions;
pub mod gic;
pub mod timer;
pub mod task;
pub mod scheduler;
pub mod benchmark;

use core::arch::global_asm;

// Include boot assembly
global_asm!(include_str!("boot.S"));

// Include exception vector table
global_asm!(include_str!("exceptions.S"));

/// Initialize ARM64 architecture
pub fn init() {
    uart::init();

    // Initialize MMU (Memory Management Unit)
    // DISABLED: Hangs after SCTLR_EL1 write (see docs/PATHWAY_D_MMU_FINDINGS.md)
    // Requires deep ARM64 expertise - deferred to v2.0
    // mmu::init();

    // Initialize exception handling
    exceptions::init();

    // Initialize GIC (Generic Interrupt Controller)
    gic::init();

    // Initialize ARM Generic Timer
    timer::init();

    // Enable timer interrupt in GIC
    gic::enable_timer_interrupt();
}

/// Halt the CPU
pub fn hlt() {
    unsafe {
        core::arch::asm!("wfe");  // Wait For Event
    }
}
