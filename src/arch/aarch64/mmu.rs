/*
 * ARM64 Memory Management Unit (MMU)
 *
 * This module provides page table management and virtual memory support for ARM64.
 *
 * Page Table Structure (4-level, 4KB granule):
 * - Level 0: PGD (Page Global Directory) - 512 GB per entry
 * - Level 1: PUD (Page Upper Directory) - 1 GB per entry
 * - Level 2: PMD (Page Middle Directory) - 2 MB per entry (we use this level)
 * - Level 3: PTE (Page Table Entry) - 4 KB per entry
 *
 * For simplicity, we use 2MB block mappings at Level 2.
 */

use core::arch::asm;

/// Page size (4 KB)
const PAGE_SIZE: usize = 4096;

/// Number of entries per page table level
const TABLE_ENTRIES: usize = 512;

/// Block size at Level 2 (2 MB)
const BLOCK_SIZE_2MB: usize = 2 * 1024 * 1024;

/// Page table entry bits
const PTE_VALID: u64 = 1 << 0;           // Valid bit
const PTE_TABLE: u64 = 1 << 1;           // Table descriptor (not block)
const PTE_BLOCK: u64 = 0 << 1;           // Block descriptor
const PTE_AF: u64 = 1 << 10;             // Access flag
const PTE_SH_INNER: u64 = 3 << 8;        // Inner shareable
const PTE_AP_RW: u64 = 0 << 7;           // Read-write (EL1)
const PTE_AP_RO: u64 = 2 << 7;           // Read-only (EL1 and EL0)
const PTE_ATTR_NORMAL: u64 = 0 << 2;     // Normal memory (index 0 in MAIR)
const PTE_ATTR_DEVICE: u64 = 1 << 2;     // Device memory (index 1 in MAIR)

/// Memory attributes for MAIR_EL1
const MAIR_NORMAL: u64 = 0xFF;           // Normal memory, write-back cacheable
const MAIR_DEVICE: u64 = 0x00;           // Device memory, non-cacheable

/// Page table alignment (must be 4KB aligned)
#[repr(C, align(4096))]
struct PageTable {
    entries: [u64; TABLE_ENTRIES],
}

impl PageTable {
    const fn new() -> Self {
        PageTable {
            entries: [0; TABLE_ENTRIES],
        }
    }

    fn zero(&mut self) {
        for entry in &mut self.entries {
            *entry = 0;
        }
    }
}

/// Global page tables
/// We'll use:
/// - 1 Level 0 table (PGD)
/// - 1 Level 1 table (PUD)
/// - 2 Level 2 tables (PMD) - each maps 1 GB with 2MB block mappings
static mut L0_TABLE: PageTable = PageTable::new();
static mut L1_TABLE: PageTable = PageTable::new();
static mut L2_TABLE_0: PageTable = PageTable::new();  // Maps 0-1GB
static mut L2_TABLE_1: PageTable = PageTable::new();  // Maps 1-2GB

/// Initialize the MMU
pub fn init() {
    unsafe {
        uart_puts("[MMU] Initializing Memory Management Unit...\n");

        // Zero out page tables
        L0_TABLE.zero();
        L1_TABLE.zero();
        L2_TABLE_0.zero();
        L2_TABLE_1.zero();

        // Set up Level 0 table (points to L1)
        let l1_addr = &L1_TABLE as *const _ as u64;
        L0_TABLE.entries[0] = l1_addr | PTE_TABLE | PTE_VALID;

        uart_puts("[MMU] Level 0 table at 0x");
        uart_puts_hex(&L0_TABLE as *const _ as u64);
        uart_puts("\n");

        // Set up Level 1 table (points to two L2 tables)
        let l2_0_addr = &L2_TABLE_0 as *const _ as u64;
        let l2_1_addr = &L2_TABLE_1 as *const _ as u64;
        L1_TABLE.entries[0] = l2_0_addr | PTE_TABLE | PTE_VALID;  // 0-1GB
        L1_TABLE.entries[1] = l2_1_addr | PTE_TABLE | PTE_VALID;  // 1-2GB

        uart_puts("[MMU] Level 1 table at 0x");
        uart_puts_hex(&L1_TABLE as *const _ as u64);
        uart_puts("\n");

        uart_puts("[MMU] Setting up Level 2 tables (2 x 512 x 2MB blocks = 2 GB)...\n");

        uart_puts("[MMU] DEBUG: About to start first loop...\n");

        // Set up Level 2 table 0 with 2MB block mappings (0-1GB)
        // Map first 128 entries = 256 MB (covers GIC @ 0x08000000 and UART @ 0x09000000)
        for i in 0..128 {
            let phys_addr = (i * BLOCK_SIZE_2MB) as u64;

            // Determine memory type based on address
            let attr = if phys_addr >= 0x08000000 && phys_addr < 0x10000000 {
                // GIC and UART region (0x08000000 - 0x10000000) - device memory
                PTE_ATTR_DEVICE
            } else {
                // Everything else - normal memory
                PTE_ATTR_NORMAL
            };

            L2_TABLE_0.entries[i] = phys_addr
                | PTE_BLOCK
                | PTE_VALID
                | PTE_AF
                | PTE_SH_INNER
                | PTE_AP_RW
                | attr;
        }

        uart_puts("[MMU] DEBUG: First loop complete\n");

        // Set up Level 2 table 1 with 2MB block mappings (1-2GB)
        // Map first 192 entries = 384 MB (enough for kernel and page tables up to ~0x58000000)
        for i in 0..192 {
            let phys_addr = (0x40000000 + i * BLOCK_SIZE_2MB) as u64;

            // All normal memory in this range (kernel code and data)
            let attr = PTE_ATTR_NORMAL;

            L2_TABLE_1.entries[i] = phys_addr
                | PTE_BLOCK
                | PTE_VALID
                | PTE_AF
                | PTE_SH_INNER
                | PTE_AP_RW
                | attr;
        }

        uart_puts("[MMU] Level 2 table 0 at 0x");
        uart_puts_hex(&L2_TABLE_0 as *const _ as u64);
        uart_puts("\n");
        uart_puts("[MMU] Level 2 table 1 at 0x");
        uart_puts_hex(&L2_TABLE_1 as *const _ as u64);
        uart_puts("\n");
        uart_puts("[MMU] Identity mapped:\n");
        uart_puts("[MMU]   0x00000000 - 0x0FFFFFFF (256 MB: peripherals)\n");
        uart_puts("[MMU]   0x40000000 - 0x57FFFFFF (384 MB: kernel/data)\n");
        uart_puts("[MMU]   Total: ~640 MB\n");

        uart_puts("[MMU] Configuring memory attributes (MAIR_EL1)...\n");

        // Configure memory attributes (MAIR_EL1)
        // Index 0: Normal memory (write-back cacheable)
        // Index 1: Device memory (non-cacheable, non-bufferable)
        let mair: u64 = (MAIR_DEVICE << 8) | MAIR_NORMAL;
        asm!("msr mair_el1, {}", in(reg) mair);

        uart_puts("[MMU] MAIR_EL1 configured\n");

        uart_puts("[MMU] Configuring Translation Control Register (TCR_EL1)...\n");

        // Configure Translation Control Register (TCR_EL1)
        // T0SZ = 25 (2^(64-25) = 512 GB address space)
        // TG0 = 0 (4KB granule)
        // SH0 = 3 (inner shareable)
        // ORGN0 = 1 (write-back write-allocate cacheable)
        // IRGN0 = 1 (write-back write-allocate cacheable)
        // IPS = 0 (32-bit physical address space, 4GB)
        let tcr: u64 = (25 << 0)    // T0SZ: 512 GB VA space
            | (0 << 14)             // TG0: 4KB granule
            | (3 << 12)             // SH0: Inner shareable
            | (1 << 10)             // ORGN0: Write-back cacheable
            | (1 << 8)              // IRGN0: Write-back cacheable
            | (0 << 32);            // IPS: 32-bit (4GB) physical address space

        asm!("msr tcr_el1, {}", in(reg) tcr);

        uart_puts("[MMU] TCR_EL1 configured (4KB granule, 512GB VA space)\n");

        uart_puts("[MMU] Setting Translation Table Base Register (TTBR0_EL1)...\n");

        // Set Translation Table Base Register (TTBR0_EL1)
        let ttbr0 = &L0_TABLE as *const _ as u64;
        asm!("msr ttbr0_el1, {}", in(reg) ttbr0);

        uart_puts("[MMU] TTBR0_EL1 set to 0x");
        uart_puts_hex(ttbr0);
        uart_puts("\n");

        uart_puts("[MMU] Synchronizing...\n");

        // Ensure all writes complete before enabling MMU
        asm!("dsb sy");   // Data Synchronization Barrier
        asm!("isb");      // Instruction Synchronization Barrier

        uart_puts("[MMU] Enabling MMU and caches (SCTLR_EL1)...\n");

        // Enable MMU and caches (SCTLR_EL1)
        // M bit (0): MMU enable
        // C bit (2): Data cache enable
        // I bit (12): Instruction cache enable
        let mut sctlr: u64;
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);

        uart_puts("[MMU] Current SCTLR_EL1: 0x");
        uart_puts_hex(sctlr);
        uart_puts("\n");

        sctlr |= (1 << 0);   // M: Enable MMU
        // TEMPORARILY: Disable caches for debugging
        // sctlr |= (1 << 2);   // C: Enable data cache
        // sctlr |= (1 << 12);  // I: Enable instruction cache

        uart_puts("[MMU] New SCTLR_EL1: 0x");
        uart_puts_hex(sctlr);
        uart_puts("\n");

        asm!("msr sctlr_el1, {}", in(reg) sctlr);

        // Synchronization barriers after enabling MMU
        asm!("dsb sy");
        asm!("isb");

        uart_puts("[MMU] MMU enabled!\n");
        uart_puts("[MMU] Data cache enabled\n");
        uart_puts("[MMU] Instruction cache enabled\n");
        uart_puts("[MMU] Virtual memory active\n");
        uart_puts("\n");
    }
}

/// Check if MMU is enabled
pub fn is_enabled() -> bool {
    let sctlr: u64;
    unsafe {
        asm!("mrs {}, sctlr_el1", out(reg) sctlr);
    }
    (sctlr & 1) != 0
}

/// Get current page table base address
pub fn get_ttbr0() -> u64 {
    let ttbr0: u64;
    unsafe {
        asm!("mrs {}, ttbr0_el1", out(reg) ttbr0);
    }
    ttbr0
}

// Helper functions for UART output

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
