/*
 * JerichoOS ARM64 Boot
 * Handles EL3→EL2→EL1 transitions for QEMU virt machine
 */

.section .text.boot
.global _start

_start:
    // Preserve DTB pointer across exception level transitions
    mov x19, x0

    // Detect current exception level
    mrs x0, CurrentEL
    lsr x0, x0, #2
    and x0, x0, #3

    cmp x0, #3
    b.eq from_el3
    cmp x0, #2
    b.eq from_el2
    b at_el1

from_el3:
    // Configure EL2 for AArch64 before dropping
    mov x0, #(1 << 31)
    msr hcr_el2, x0

    // Disable all coprocessor traps
    mov x0, #0
    msr cptr_el3, x0

    // Configure lower levels: non-secure + AArch64
    mov x0, #0x401
    msr scr_el3, x0

    // Jump to EL2 handler
    adr x0, from_el2
    msr elr_el3, x0
    mov x0, #0x3c9                   // EL2h, interrupts masked
    msr spsr_el3, x0
    eret

from_el2:
    // Configure EL1 for AArch64 before dropping
    mov x0, #(1 << 31)
    msr hcr_el2, x0

    // Jump to EL1 handler
    adr x0, at_el1
    msr elr_el2, x0
    mov x0, #0x3c5                   // EL1h, interrupts masked
    msr spsr_el2, x0
    eret

at_el1:
    // Load stack from linker script (64KB)
    adrp x0, __stack_top
    add x0, x0, :lo12:__stack_top
    mov sp, x0

    // Zero BSS section
    adrp x0, __bss_start
    add x0, x0, :lo12:__bss_start
    adrp x1, __bss_end
    add x1, x1, :lo12:__bss_end
clear_bss:
    cmp x0, x1
    b.ge bss_done
    str xzr, [x0], #8
    b clear_bss

bss_done:
    // Restore DTB pointer for kernel
    mov x0, x19

    // Enter Rust kernel
    bl kernel_main

    // Unreachable - kernel_main never returns
1:  wfe
    b 1b
