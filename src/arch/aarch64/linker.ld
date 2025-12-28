/*
 * JerichoOS AArch64 Linker Script
 *
 * Memory layout for QEMU virt machine
 */

ENTRY(_start)

/* QEMU virt machine loads kernel at 0x40080000 (not 0x40000000!) */
MEMORY
{
    RAM : ORIGIN = 0x40080000, LENGTH = 128M
}

SECTIONS
{
    . = ORIGIN(RAM);

    /* Boot code - must be first */
    .text.boot : {
        *(.text.boot)
    } > RAM

    /* Code section */
    .text : ALIGN(4K) {
        /* Exception vectors MUST come first (2KB aligned) */
        *(.text.exceptions)
        . = ALIGN(2K);  /* Ensure 2KB padding after vectors */
        /* Rest of code sections */
        *(.text .text.*)
    } > RAM

    /* Read-only data */
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
        /* Add 16KB padding for SIMD over-reads (compiler may use 128-bit loads) */
        . = ALIGN(16K);
    } > RAM

    /* Data section */
    .data : ALIGN(4K) {
        *(.data .data.*)
    } > RAM

    /* BSS section (uninitialized data) */
    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } > RAM

    /* Stack grows downwards from end of BSS */
    .stack : ALIGN(16) {
        . += 64K;  /* 64KB stack */
        __stack_top = .;
    } > RAM

    /* Discard sections we don't need */
    /DISCARD/ : {
        *(.comment)
        *(.gnu*)
        *(.note*)
        *(.eh_frame*)
    }
}
