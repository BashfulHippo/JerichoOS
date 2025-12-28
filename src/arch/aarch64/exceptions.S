/*
 * ARM64 Exception Vector Table
 *
 * ARM64 has 16 exception vectors, each 128 bytes (0x80) apart:
 * - 4 exception levels (current EL SP0, current EL SPx, lower EL 64-bit, lower EL 32-bit)
 * - 4 exception types per level (Synchronous, IRQ, FIQ, SError)
 *
 * We're running at EL1, so we care about:
 * - Current EL with SPx (kernel exceptions)
 * - Lower EL (userspace exceptions - for future)
 */

.section .text.exceptions

.macro EXCEPTION_ENTRY label
.align 7  // Align to 128 bytes (2^7)
b \label
.endm

// Exception vector table - must be 2KB aligned
.align 11  // 2^11 = 2048 bytes = 2KB
.global exception_vector_table
exception_vector_table:

    // Current EL with SP0 (not used in our kernel)
    EXCEPTION_ENTRY el1_sp0_sync        // Synchronous
    EXCEPTION_ENTRY el1_sp0_irq         // IRQ
    EXCEPTION_ENTRY el1_sp0_fiq         // FIQ
    EXCEPTION_ENTRY el1_sp0_serror      // SError

    // Current EL with SPx (kernel mode - most common)
    EXCEPTION_ENTRY el1_spx_sync        // Synchronous
    EXCEPTION_ENTRY el1_spx_irq         // IRQ
    EXCEPTION_ENTRY el1_spx_fiq         // FIQ
    EXCEPTION_ENTRY el1_spx_serror      // SError

    // Lower EL using AArch64 (userspace - future)
    EXCEPTION_ENTRY el0_aarch64_sync    // Synchronous
    EXCEPTION_ENTRY el0_aarch64_irq     // IRQ
    EXCEPTION_ENTRY el0_aarch64_fiq     // FIQ
    EXCEPTION_ENTRY el0_aarch64_serror  // SError

    // Lower EL using AArch32 (not supported)
    EXCEPTION_ENTRY el0_aarch32_sync    // Synchronous
    EXCEPTION_ENTRY el0_aarch32_irq     // IRQ
    EXCEPTION_ENTRY el0_aarch32_fiq     // FIQ
    EXCEPTION_ENTRY el0_aarch32_serror  // SError

//
// Exception Handlers
//

// Macro to save all registers to the stack
.macro SAVE_REGS
    // Make room for exception frame (272 bytes)
    sub sp, sp, #272

    // Save general purpose registers
    stp x0, x1, [sp, #16 * 0]
    stp x2, x3, [sp, #16 * 1]
    stp x4, x5, [sp, #16 * 2]
    stp x6, x7, [sp, #16 * 3]
    stp x8, x9, [sp, #16 * 4]
    stp x10, x11, [sp, #16 * 5]
    stp x12, x13, [sp, #16 * 6]
    stp x14, x15, [sp, #16 * 7]
    stp x16, x17, [sp, #16 * 8]
    stp x18, x19, [sp, #16 * 9]
    stp x20, x21, [sp, #16 * 10]
    stp x22, x23, [sp, #16 * 11]
    stp x24, x25, [sp, #16 * 12]
    stp x26, x27, [sp, #16 * 13]
    stp x28, x29, [sp, #16 * 14]

    // Save LR (x30) and SP
    mrs x0, sp_el0
    stp x30, x0, [sp, #16 * 15]

    // Save exception state registers
    mrs x0, elr_el1      // Exception Link Register (PC)
    mrs x1, spsr_el1     // Saved Processor State Register
    stp x0, x1, [sp, #16 * 16]
.endm

// Macro to restore all registers from the stack
.macro RESTORE_REGS
    // Restore exception state registers
    ldp x0, x1, [sp, #16 * 16]
    msr elr_el1, x0
    msr spsr_el1, x1

    // Restore LR and SP
    ldp x30, x0, [sp, #16 * 15]
    msr sp_el0, x0

    // Restore general purpose registers
    ldp x28, x29, [sp, #16 * 14]
    ldp x26, x27, [sp, #16 * 13]
    ldp x24, x25, [sp, #16 * 12]
    ldp x22, x23, [sp, #16 * 11]
    ldp x20, x21, [sp, #16 * 10]
    ldp x18, x19, [sp, #16 * 9]
    ldp x16, x17, [sp, #16 * 8]
    ldp x14, x15, [sp, #16 * 7]
    ldp x12, x13, [sp, #16 * 6]
    ldp x10, x11, [sp, #16 * 5]
    ldp x8, x9, [sp, #16 * 4]
    ldp x6, x7, [sp, #16 * 3]
    ldp x4, x5, [sp, #16 * 2]
    ldp x2, x3, [sp, #16 * 1]
    ldp x0, x1, [sp, #16 * 0]

    // Restore stack pointer
    add sp, sp, #272
.endm

//
// Current EL with SP0 handlers (should not happen)
//

el1_sp0_sync:
    b .

el1_sp0_irq:
    b .

el1_sp0_fiq:
    b .

el1_sp0_serror:
    b .

//
// Current EL with SPx handlers (kernel mode)
//

el1_spx_sync:
    SAVE_REGS
    mov x0, sp
    bl handle_sync_exception
    RESTORE_REGS
    eret

el1_spx_irq:
    SAVE_REGS
    mov x0, sp
    bl handle_irq
    // handle_irq returns the frame pointer to restore from (x0)
    // This may be different from the original SP if a task switch occurred

    // Switch SP to the returned frame pointer
    mov sp, x0
    RESTORE_REGS
    eret

// Debug helper: print SP value
debug_print_sp:
    stp x29, x30, [sp, #-16]!
    stp x0, x1, [sp, #-16]!

    // Print "[SP="
    mov x0, #0x09000000
    mov w1, #0x5B        // '['
    str w1, [x0]
    mov w1, #0x53        // 'S'
    str w1, [x0]
    mov w1, #0x50        // 'P'
    str w1, [x0]
    mov w1, #0x3D        // '='
    str w1, [x0]

    ldp x0, x1, [sp], #16
    ldp x29, x30, [sp], #16
    ret

// Debug helper: print PC value
debug_print_pc:
    stp x29, x30, [sp, #-16]!
    stp x0, x1, [sp, #-16]!

    // Print " PC="
    mov x0, #0x09000000
    mov w1, #0x20        // ' '
    str w1, [x0]
    mov w1, #0x50        // 'P'
    str w1, [x0]
    mov w1, #0x43        // 'C'
    str w1, [x0]
    mov w1, #0x3D        // '='
    str w1, [x0]
    mov w1, #0x5D        // ']'
    str w1, [x0]
    mov w1, #0x20        // ' '
    str w1, [x0]

    ldp x0, x1, [sp], #16
    ldp x29, x30, [sp], #16
    ret

el1_spx_fiq:
    SAVE_REGS
    mov x0, sp
    bl handle_fiq
    RESTORE_REGS
    eret

el1_spx_serror:
    SAVE_REGS
    mov x0, sp
    bl handle_serror
    RESTORE_REGS
    eret

//
// Lower EL AArch64 handlers (userspace - future)
//

el0_aarch64_sync:
    SAVE_REGS
    mov x0, sp
    bl handle_sync_exception
    RESTORE_REGS
    eret

el0_aarch64_irq:
    SAVE_REGS
    mov x0, sp
    bl handle_irq
    RESTORE_REGS
    eret

el0_aarch64_fiq:
    SAVE_REGS
    mov x0, sp
    bl handle_fiq
    RESTORE_REGS
    eret

el0_aarch64_serror:
    SAVE_REGS
    mov x0, sp
    bl handle_serror
    RESTORE_REGS
    eret

//
// Lower EL AArch32 handlers (not supported)
//

el0_aarch32_sync:
    b .

el0_aarch32_irq:
    b .

el0_aarch32_fiq:
    b .

el0_aarch32_serror:
    b .
