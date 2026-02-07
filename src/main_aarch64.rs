//! JerichoOS - ARM64 Kernel Entry Point
//!
//! ARM64 kernel with WASM runtime and MQTT demo integration

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

// Configuration: Set to true to run context switch benchmark, false for demo
const RUN_BENCHMARK: bool = false;

use core::panic::PanicInfo;
use core::arch::asm;
use linked_list_allocator::LockedHeap;

// Architecture-specific code
#[path = "arch/aarch64/mod.rs"]
mod arch;

// Serial output macros (using ARM UART)
#[macro_export]
macro_rules! serial_print {
    ($msg:expr) => {
        $crate::uart_puts($msg)
    };
    // Accept format args for compatibility with x86-64, but since formatting
    // isn't implemented yet, just print the literal value when format is "{}"
    ("{}", $val:expr) => {
        $crate::uart_puts($val)
    };
    ($fmt:expr, $($arg:tt)*) => {{
        // For other format strings, just print the format string itself
        // TODO: Implement proper formatting when core::fmt works
        $crate::uart_puts($fmt)
    }};
}

#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::uart_puts("\n")
    };
    ($msg:expr) => {{
        $crate::uart_puts($msg);
        $crate::uart_puts("\n");
    }};
    // Accept format args for compatibility with x86-64
    ("{}", $val:expr) => {{
        $crate::uart_puts($val);
        $crate::uart_puts("\n");
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        // For other format strings, just print the format string itself
        // TODO: Implement proper formatting when core::fmt works
        $crate::uart_puts($fmt);
        $crate::uart_puts("\n");
    }};
}

// Re-export architecture-specific types at crate root for compatibility
mod task {
    pub use crate::arch::task::TaskContext;
    pub use crate::arch::scheduler::TaskState;

    /// Task ID (compatible with x86 version)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct TaskId(u64);

    impl TaskId {
        pub fn new(id: u64) -> Self {
            TaskId(id)
        }

        pub fn value(&self) -> u64 {
            self.0
        }
    }

    /// Task priority (for compatibility)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Priority {
        Low = 0,
        Normal = 1,
        High = 2,
        Realtime = 3,
    }
}

mod scheduler {
    pub use crate::arch::scheduler::*;
}

// Architecture-independent modules (shared with x86-64)
mod capability;
mod syscall;
mod wasm_runtime;
mod demos;
mod benchmark;

// Global allocator (required for alloc crate)
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Static heap memory (4 MB for WASM linear memory - 3 modules with instance reuse)
const HEAP_SIZE: usize = 4 * 1024 * 1024;
#[repr(align(4096))]
struct HeapMemory([u8; HEAP_SIZE]);
static mut HEAP_MEMORY: HeapMemory = HeapMemory([0; HEAP_SIZE]);

/// Initialize the heap allocator
fn init_heap() {
    unsafe {
        let heap_start = HEAP_MEMORY.0.as_ptr() as usize;
        ALLOCATOR.lock().init(heap_start as *mut u8, HEAP_SIZE);
    }
    uart_puts("[HEAP] Initialized 4 MB heap\n");
}

/// Allocation error handler
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    uart_puts("\n[PANIC] Allocation error: size=");
    uart_puts_hex(layout.size() as u64);
    uart_puts(" align=");
    uart_puts_hex(layout.align() as u64);
    uart_puts("\n");
    hlt()
}

/// PL011 UART base address (QEMU virt machine)
const UART_BASE: usize = 0x09000000;
const UART_DR: usize = UART_BASE + 0x00;
const UART_FR: usize = UART_BASE + 0x18;
const UART_FR_TXFF: u32 = 1 << 5;

/// Write a byte to UART
fn uart_putc(c: u8) {
    unsafe {
        while (core::ptr::read_volatile(UART_FR as *const u32) & UART_FR_TXFF) != 0 {
            core::hint::spin_loop();
        }
        core::ptr::write_volatile(UART_DR as *mut u32, c as u32);
    }
}

/// Write a string to UART
fn uart_puts(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            uart_putc(b'\r');
        }
        uart_putc(byte);
    }
}

/// Halt the CPU
fn hlt() -> ! {
    loop {
        unsafe {
            asm!("wfe");
        }
    }
}

// WASM execution test disabled - wasm_runtime depends on capability which causes ARM64 crashes
// fn test_wasm_execution() { ... }

// Test task 1 - prints periodically to show it's running
// Task 1 - Simple infinite loop printing "A"
// #[inline(never)] prevents inlining and ensures distinct code
#[inline(never)]
extern "C" fn task1() -> ! {
    loop {
        unsafe {
            let uart = 0x09000000 as *mut u32;
            core::ptr::write_volatile(uart, b'A' as u32);
        }
        // Busy wait
        for _ in 0..50000 {
            unsafe { asm!("nop"); }
        }
    }
}

// Task 2 - Simple infinite loop printing "B"
#[inline(never)]
extern "C" fn task2() -> ! {
    loop {
        unsafe {
            let uart = 0x09000000 as *mut u32;
            core::ptr::write_volatile(uart, b'B' as u32);
        }
        // Busy wait
        for _ in 0..50000 {
            unsafe { asm!("nop"); }
        }
    }
}

// Task 3 - Simple infinite loop printing "C"
#[inline(never)]
extern "C" fn task3() -> ! {
    loop {
        unsafe {
            let uart = 0x09000000 as *mut u32;
            core::ptr::write_volatile(uart, b'C' as u32);
        }
        // Busy wait
        for _ in 0..50000 {
            unsafe { asm!("nop"); }
        }
    }
}

// Global benchmark state
static mut BENCHMARK_START_TIME: u64 = 0;
static mut BENCHMARK_RUNNING: bool = false;
const BENCHMARK_TARGET_SWITCHES: u64 = 1000;

// Benchmark task A - monitors switch count and prints results
// NOTE: Has ARM64 cache coherency issue - atomic counter not visible across interrupt/task contexts
#[inline(never)]
extern "C" fn bench_task_a() -> ! {
    loop {
        // Minimal work - benchmark functionality disabled due to cache coherency issue
        for _ in 0..100 {
            unsafe { asm!("nop"); }
        }
    }
}

// Benchmark task B - just minimal work
#[inline(never)]
extern "C" fn bench_task_b() -> ! {
    loop {
        // Very minimal work - just a single nop
        unsafe { asm!("nop"); }
    }
}

// Helper to print hex
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

/// Kernel entry point called from boot.S
///
/// # Arguments
/// * `dtb_ptr` - Pointer to Device Tree Blob
#[no_mangle]
pub extern "C" fn kernel_main(_dtb_ptr: usize) -> ! {
    // Print boot banner
    uart_puts("\n");
    uart_puts("╔════════════════════════════════════════════════════════╗\n");
    uart_puts("║         JerichoOS ARM64 Port - Phase 3               ║\n");
    uart_puts("╚════════════════════════════════════════════════════════╝\n");
    uart_puts("\n");
    uart_puts("[BOOT] JerichoOS v0.1.0 - AArch64\n");
    uart_puts("[INFO] Kernel entry point reached\n");
    uart_puts("[INFO] Architecture: AArch64 (ARM64)\n");
    uart_puts("[INFO] Platform: QEMU virt machine\n");
    uart_puts("\n");

    // Initialize architecture (exceptions, GIC, timer)
    uart_puts("[INIT] Initializing ARM64 architecture...\n");
    arch::init();

    // Initialize heap allocator
    uart_puts("[INIT] Initializing heap allocator...\n");
    init_heap();

    // Test heap allocation
    uart_puts("[TEST] Testing heap allocation...\n");
    {
        use alloc::vec::Vec;
        let mut test_vec = Vec::new();
        for i in 0..10 {
            test_vec.push(i);
        }
        uart_puts("[ OK ] Vec allocation successful: ");
        uart_puts_hex(test_vec.len() as u64);
        uart_puts(" elements\n");
    }

    // Test BTreeMap operations
    uart_puts("[TEST] Testing BTreeMap operations...\n");
    {
        use alloc::collections::BTreeMap;

        uart_puts("[TEST] Creating BTreeMap...\n");
        let mut test_map: BTreeMap<u64, u64> = BTreeMap::new();
        uart_puts("[ OK ] BTreeMap created\n");

        uart_puts("[TEST] Inserting into BTreeMap...\n");
        test_map.insert(1, 100);
        test_map.insert(2, 200);
        uart_puts("[ OK ] BTreeMap insert successful\n");

        uart_puts("[TEST] Reading from BTreeMap...\n");
        if let Some(&val) = test_map.get(&1) {
            uart_puts("[ OK ] BTreeMap get successful, value=");
            uart_puts_hex(val);
            uart_puts("\n");
        }
    }

    // PHASE 3: Test capability with spin::Once + BTreeMap
    // NOTE: Historical SIMD concern resolved - capability init works without NEON disable.
    // Keep this as a regression checkpoint during future ARM bring-up changes.
    uart_puts("[TEST] Phase 3: Testing capability with spin::Once...\n");
    capability::init();
    uart_puts("[ OK ] Capability::init() SUCCESS with spin::Once!\n");

    // Initialize WASM runtime
    uart_puts("[INIT] Initializing WebAssembly runtime...\n");
    wasm_runtime::init();
    uart_puts("[ OK ] WebAssembly runtime initialized\n");

    // Run canonical WASM demo suite
    uart_puts("\n");
    uart_puts("╔════════════════════════════════════════════════════════╗\n");
    uart_puts("║   JerichoOS Canonical WASM Demo Suite (ARM64)         ║\n");
    uart_puts("╚════════════════════════════════════════════════════════╝\n");
    uart_puts("\n");
    demos::run_demos();

    uart_puts("\n");
    uart_puts("✅ ARM64 kernel initialization complete!\n");
    uart_puts("\n");

    // Display benchmark counter information
    uart_puts("[INFO] ARM64 Performance Counter Information:\n");
    let (freq_val, freq_unit) = arch::benchmark::get_counter_info();
    uart_puts("  Counter frequency: ");
    uart_puts_hex(freq_val);
    uart_puts(" ");
    uart_puts(freq_unit);
    uart_puts("\n");
    uart_puts("  Counter resolution: ");
    uart_puts_hex(arch::benchmark::ticks_to_ns(1));
    uart_puts(" ns per tick\n");
    uart_puts("\n");

    // Test benchmark timer
    uart_puts("[TEST] Testing benchmark counter...\n");
    let start = arch::benchmark::read_counter();
    // Perform some work
    for _ in 0..10000 {
        unsafe { asm!("nop"); }
    }
    let end = arch::benchmark::read_counter();
    let elapsed_ticks = end - start;
    uart_puts("  Elapsed ticks: ");
    uart_puts_hex(elapsed_ticks);
    uart_puts("\n  Elapsed time: ");
    uart_puts_hex(arch::benchmark::ticks_to_us(elapsed_ticks));
    uart_puts(" µs\n");
    uart_puts("[ OK ] Benchmark counter working!\n");
    uart_puts("\n");

    // Run benchmark suite (quantitative performance metrics)
    benchmark::run_benchmark_suite();

    // Initialize scheduler
    uart_puts("[INIT] Initializing task scheduler...\n");
    arch::scheduler::init();

    // Conditional: Spawn benchmark or demo tasks
    if RUN_BENCHMARK {
        // Benchmark mode
        uart_puts("\n");
        uart_puts("╔════════════════════════════════════════════════════════╗\n");
        uart_puts("║       ARM64 Context Switch Benchmark                 ║\n");
        uart_puts("╚════════════════════════════════════════════════════════╝\n");
        uart_puts("\n");
        uart_puts("[BENCH] Target: ");
        uart_puts_hex(BENCHMARK_TARGET_SWITCHES);
        uart_puts(" context switches\n");
        uart_puts("[BENCH] Spawning benchmark tasks...\n");

        arch::scheduler::spawn(bench_task_a);
        arch::scheduler::spawn(bench_task_b);
        uart_puts("[BENCH] Spawned 2 minimal benchmark tasks\n");

        // Reset counter and set start time
        arch::scheduler::reset_switch_counter();
        unsafe {
            BENCHMARK_START_TIME = arch::benchmark::read_counter();
            BENCHMARK_RUNNING = true;
        }
        uart_puts("[BENCH] Benchmark initialized\n");
        uart_puts("\n");
    } else {
        // Demo mode
        uart_puts("[INIT] Spawning test tasks...\n");
        uart_puts("[DEBUG] task1 address: 0x");
        uart_puts_hex(task1 as usize as u64);
        uart_puts("\n");
        uart_puts("[DEBUG] task2 address: 0x");
        uart_puts_hex(task2 as usize as u64);
        uart_puts("\n");
        uart_puts("[DEBUG] task3 address: 0x");
        uart_puts_hex(task3 as usize as u64);
        uart_puts("\n");
        arch::scheduler::spawn(task1);
        arch::scheduler::spawn(task2);
        arch::scheduler::spawn(task3);
        uart_puts("[INIT] Spawned 3 tasks\n");
        uart_puts("\n");
    }

    // Enable interrupts
    uart_puts("[INFO] Enabling interrupts...\n");
    unsafe {
        asm!("msr daifclr, #2");  // Clear IRQ mask
    }

    // Enable task switching in IRQ handler
    arch::exceptions::enable_scheduler();

    if RUN_BENCHMARK {
        uart_puts("[BENCH] Starting benchmark...\n");
        uart_puts("[INFO] Measuring ");
        uart_puts_hex(BENCHMARK_TARGET_SWITCHES);
        uart_puts(" switches...\n");
    } else {
        uart_puts("[INFO] Interrupts enabled! Starting scheduler...\n");
        uart_puts("[INFO] Task switching every 100ms (10 timer ticks)\n");
        uart_puts("[INFO] Timer ticks every 10ms (100 Hz)\n");
    }
    uart_puts("\n");

    // Start first task
    if RUN_BENCHMARK {
        uart_puts("[BENCH] Jumping to benchmark task...\n");
    } else {
        uart_puts("[INFO] Starting multitasking...\n");
    }
    uart_puts("\n");

    // Jump to first task manually
    unsafe {
        let scheduler = &mut *(core::ptr::addr_of_mut!(arch::scheduler::SCHEDULER));
        if scheduler.num_tasks() > 0 {
            scheduler.tasks[0].state = arch::scheduler::TaskState::Running;
            let ctx = &scheduler.tasks[0].context;

            // Debug: Print task context before jumping
            uart_puts("[DEBUG] Task 0 context:\n");
            uart_puts("  PC: 0x");
            uart_puts_hex(ctx.pc);
            uart_puts("\n  SP: 0x");
            uart_puts_hex(ctx.sp);
            uart_puts("\n");

            let ctx = ctx as *const arch::task::TaskContext;

            uart_puts("[DEBUG] About to jump to task...\n");

            // SIMPLIFIED TASK LAUNCH FOR DEBUGGING
            // Set SP, restore PSTATE, and jump to PC
            let task_pc = (*ctx).pc;
            let task_sp = (*ctx).sp;
            let task_pstate = (*ctx).pstate;

            uart_puts("[DEBUG] Task PC=0x");
            uart_puts_hex(task_pc);
            uart_puts(" SP=0x");
            uart_puts_hex(task_sp);
            uart_puts(" PSTATE=0x");
            uart_puts_hex(task_pstate);
            uart_puts("\n");

            asm!(
                // Set stack pointer
                "mov sp, {sp}",
                // Set PSTATE via SPSR_EL1 for upcoming exception return
                "msr spsr_el1, {pstate}",
                // Set return address to task PC
                "msr elr_el1, {pc}",
                // Memory barriers to ensure coherency
                "dsb sy",
                "isb",
                // Exception return - restores PSTATE and jumps to task PC
                "eret",
                pc = in(reg) task_pc,
                sp = in(reg) task_sp,
                pstate = in(reg) task_pstate,
                options(noreturn)
            );
        }
    }

    // Should never reach here
    loop {
        unsafe { asm!("wfi"); }
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uart_puts("\n");
    uart_puts("╔════════════════════════════════════════════════════════╗\n");
    uart_puts("║                    KERNEL PANIC                       ║\n");
    uart_puts("╚════════════════════════════════════════════════════════╝\n");
    uart_puts("\n");

    if let Some(location) = info.location() {
        uart_puts("Panic at ");
        uart_puts(location.file());
        uart_puts("\n");
    } else {
        uart_puts("Panic at <unknown location>\n");
    }

    hlt()
}
