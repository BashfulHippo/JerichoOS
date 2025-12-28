/*
 * JerichoOS AArch64 Boot Stub
 *
 * Entry point for ARM64 kernel
 */

.section .text.boot
.global _start

/*
 * Kernel entry point
 *
 * On entry:
 *   X0 = DTB (Device Tree Blob) pointer
 *   CPU in EL1 (kernel mode)
 *   MMU disabled
 *   Caches disabled
 */
_start:
    // Save DTB pointer
    mov x19, x0

    // Check current exception level
    mrs x0, CurrentEL
    and x0, x0, #(3 << 2)
    cmp x0, #(1 << 2)  // EL1?
    b.ne .    // Hang if not EL1

    // Set up stack pointer
    adrp x0, stack_top
    add x0, x0, :lo12:stack_top
    mov sp, x0

    // Clear BSS segment
    adrp x0, __bss_start
    add x0, x0, :lo12:__bss_start
    adrp x1, __bss_end
    add x1, x1, :lo12:__bss_end
clear_bss:
    cmp x0, x1
    b.ge bss_cleared
    str xzr, [x0], #8
    b clear_bss

bss_cleared:
    // Restore DTB pointer to x0 for kernel_main
    mov x0, x19

    // Jump to Rust kernel
    bl kernel_main

    // Hang if kernel returns
hang:
    wfe
    b hang

.section .bss
.align 16
stack_bottom:
    .space 16384  // 16KB stack
stack_top:
