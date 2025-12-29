//! JerichoOS - Capability-Based Wasm Microkernel
//!
//! Main kernel entry point

#![no_std]  // Don't link Rust standard library
#![no_main] // Don't use standard main entry point
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)] // Required for interrupt handlers
#![feature(alloc_error_handler)] // Required for heap allocation

extern crate alloc;  // Enable heap allocation

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo};
use alloc::{boxed::Box, vec::Vec};

#[macro_use]
mod vga_buffer;
#[macro_use]
mod serial;
mod gdt;
mod interrupts;
mod memory;
mod allocator;
mod capability;
mod syscall;
mod wasm_runtime;
mod task;
mod scheduler;
mod ipc;
mod benchmark;
mod demos;

// Configure bootloader to map physical memory
const BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

/// Global boot cycles for benchmarking
static BOOT_CYCLES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// Enable verbose boot logging (disable for faster boot)
const VERBOSE_BOOT: bool = cfg!(debug_assertions);

/// Kernel entry point called by bootloader
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let _framebuffer = boot_info.framebuffer.as_ref();  // Available for future use

    // Start boot timer
    let boot_start = benchmark::rdtsc();

    // Initialize kernel (always print these - critical for debugging)
    serial_println!("\n[BOOT] JerichoOS v0.1.0 Starting...");
    serial_println!("[BOOT] Kernel entry point reached");
    serial_println!("[BOOT] Capability-based Wasm Microkernel\n");

    // Initialize GDT
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing GDT..."); }
    gdt::init();
    if VERBOSE_BOOT { serial_println!("[ OK ] GDT initialized"); }

    // Initialize IDT
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing IDT..."); }
    interrupts::init();
    if VERBOSE_BOOT { serial_println!("[ OK ] IDT initialized"); }

    // Test interrupts (only in debug builds)
    #[cfg(debug_assertions)]
    {
        serial_println!("[TEST] Testing breakpoint exception...");
        x86_64::instructions::interrupts::int3();
        serial_println!("[ OK ] Breakpoint exception handled successfully");
    }

    // Initialize memory management
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing memory management..."); }
    let phys_mem_offset = boot_info.physical_memory_offset.into_option()
        .expect("Physical memory offset required");
    let phys_mem_offset = x86_64::VirtAddr::new(phys_mem_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_regions)
    };
    if VERBOSE_BOOT { serial_println!("[ OK ] Memory management initialized"); }

    // Initialize heap
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing heap allocator..."); }
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    if VERBOSE_BOOT { serial_println!("[ OK ] Heap allocator initialized ({}KB)", allocator::HEAP_SIZE / 1024); }

    // Test heap allocation (only in debug builds)
    #[cfg(debug_assertions)]
    {
        serial_println!("[TEST] Testing heap allocation...");
        let heap_value = Box::new(42);
        serial_println!("[ OK ] Box allocation: value = {}", heap_value);

        let mut vec = Vec::new();
        for i in 0..10 {
            vec.push(i);
        }
        serial_println!("[ OK ] Vec allocation: {:?}", vec);
    }

    // Initialize capability system
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing capability system..."); }
    capability::init();
    if VERBOSE_BOOT { serial_println!("[ OK ] Capability system initialized"); }

    // Initialize IPC system
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing IPC system..."); }
    ipc::init();
    if VERBOSE_BOOT { serial_println!("[ OK ] IPC system initialized"); }

    // Test capability system (only in debug builds)
    #[cfg(debug_assertions)]
    test_capability_system();

    // Initialize Wasm runtime
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing WebAssembly runtime..."); }
    wasm_runtime::init();
    if VERBOSE_BOOT { serial_println!("[ OK ] WebAssembly runtime initialized"); }

    // Test Wasm execution (only in debug builds)
    #[cfg(debug_assertions)]
    test_wasm_execution();

    // Run demo applications (always print this so we know demos are starting)
    serial_println!("\n[INFO] Starting WASM demo suite...");
    demos::run_demos();
    serial_println!("[INFO] Demo suite complete\n");

    // Run benchmark suite
    serial_println!("[INFO] Starting benchmark suite...");
    benchmark::run_benchmark_suite();
    serial_println!("[INFO] Benchmarks complete\n");

    // Initialize scheduler
    serial_println!("[INFO] All core systems operational");
    if VERBOSE_BOOT {
        serial_println!("[INFO] Security: Capability-based access control");
        serial_println!("[INFO] Runtime: WebAssembly native execution");
        serial_println!("[INFO] Scheduler: Round-robin multitasking");
        serial_println!("[INFO] Platform: x86-64 bare metal");
    }
    serial_println!("[INFO] JerichoOS booted successfully!");

    // Report boot time
    let boot_end = benchmark::rdtsc();
    let boot_cycles = boot_end - boot_start;
    BOOT_CYCLES.store(boot_cycles, core::sync::atomic::Ordering::Relaxed);
    let boot_time_us = benchmark::cycles_to_us(boot_cycles);
    let boot_time_ms = boot_time_us / 1000;
    serial_println!("[PERF] Boot time: {} ms ({} µs, {} cycles)",
        boot_time_ms, boot_time_us, boot_cycles);

    // Initialize timer interrupt for preemptive multitasking
    if VERBOSE_BOOT { serial_println!("[INIT] Enabling timer interrupts (100 Hz)..."); }
    interrupts::init_timer(100);  // 100 Hz = 10ms intervals
    if VERBOSE_BOOT { serial_println!("[ OK ] Timer interrupts enabled"); }

    if VERBOSE_BOOT { serial_println!("[INFO] System running, timer ticking every 10ms..."); }

    // Initialize scheduler
    if VERBOSE_BOOT { serial_println!("[INIT] Initializing task scheduler..."); }
    scheduler::init();
    if VERBOSE_BOOT { serial_println!("[ OK ] Task scheduler initialized"); }

    // Test scheduler (THIS CALL NEVER RETURNS - tasks run forever)
    test_scheduler();

    #[cfg(test)]
    test_main();

    // Main idle loop - interrupts will fire asynchronously
    loop {
        x86_64::instructions::hlt();  // Halt until next interrupt
    }
}

/// Test the capability system
fn test_capability_system() {
    use syscall::{SyscallContext, SyscallResult, encode_rights};
    use capability::Rights;

    serial_println!("[TEST] Testing capability system...");

    // Create a simulated user context
    let mut ctx = SyscallContext::new();

    // Test 1: Create a memory capability with full rights
    serial_println!("[TEST] Creating memory capability with ALL rights...");
    let rights_all = encode_rights(Rights::ALL);
    let result = ctx.syscall(0, 0, 0x1000, rights_all, 0);  // CapCreate, Memory, addr=0x1000

    let cap1_id = match result {
        SyscallResult::Success(id) => {
            serial_println!("[ OK ] Created capability ID: {}", id);
            id
        }
        SyscallResult::Error(e) => {
            serial_println!("[FAIL] Failed to create capability: {:?}", e);
            return;
        }
    };

    // Test 2: Derive a read-only capability from the full-rights one
    serial_println!("[TEST] Deriving read-only capability...");
    let rights_read = encode_rights(Rights::READ);
    let result = ctx.syscall(1, cap1_id, rights_read, 0, 0);  // CapDerive

    let cap2_id = match result {
        SyscallResult::Success(id) => {
            serial_println!("[ OK ] Derived read-only capability ID: {}", id);
            id
        }
        SyscallResult::Error(e) => {
            serial_println!("[FAIL] Failed to derive capability: {:?}", e);
            return;
        }
    };

    // Test 3: Try to derive write-only from read-only (should fail - security!)
    serial_println!("[TEST] Attempting to escalate privileges (should fail)...");
    let rights_write = encode_rights(Rights::READ_WRITE);
    let result = ctx.syscall(1, cap2_id, rights_write, 0, 0);  // CapDerive

    match result {
        SyscallResult::Success(_) => {
            serial_println!("[FAIL] Security breach! Escalated privileges!");
        }
        SyscallResult::Error(_) => {
            serial_println!("[ OK ] Privilege escalation blocked (security works!)");
        }
    };

    // Test 4: Invoke the read-only capability
    serial_println!("[TEST] Invoking read-only capability...");
    let result = ctx.syscall(3, cap2_id, 0, 0, 0);  // CapInvoke

    match result {
        SyscallResult::Success(_) => {
            serial_println!("[ OK ] Successfully invoked capability");
        }
        SyscallResult::Error(e) => {
            serial_println!("[FAIL] Failed to invoke capability: {:?}", e);
        }
    };

    // Test 5: Revoke a capability
    serial_println!("[TEST] Revoking capability {}...", cap2_id);
    let result = ctx.syscall(2, cap2_id, 0, 0, 0);  // CapRevoke

    match result {
        SyscallResult::Success(_) => {
            serial_println!("[ OK ] Capability revoked");
        }
        SyscallResult::Error(e) => {
            serial_println!("[FAIL] Failed to revoke capability: {:?}", e);
        }
    };

    // Test 6: Try to invoke revoked capability (should fail)
    serial_println!("[TEST] Attempting to use revoked capability (should fail)...");
    let result = ctx.syscall(3, cap2_id, 0, 0, 0);  // CapInvoke

    match result {
        SyscallResult::Success(_) => {
            serial_println!("[FAIL] Security breach! Used revoked capability!");
        }
        SyscallResult::Error(_) => {
            serial_println!("[ OK ] Revoked capability rejected (security works!)");
        }
    };

    serial_println!("[TEST] Capability system tests complete");
    serial_println!("[ OK ] All {} capabilities properly managed", ctx.capability_count());
}

/// Test WebAssembly execution
fn test_wasm_execution() {
    use wasm_runtime::WasmModule;
    use wasmi::Value;

    serial_println!("[TEST] Testing WebAssembly execution...");

    // Simple Wasm module: (module (func (export "test") (result i32) i32.const 42))
    // This module exports a function "test" that returns 42
    const WASM_TEST: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // Magic number: \0asm
        0x01, 0x00, 0x00, 0x00, // Version: 1
        // Type section
        0x01, // Section ID: Type
        0x05, // Section size: 5 bytes
        0x01, // Number of types: 1
        0x60, // Function type
        0x00, // No parameters
        0x01, 0x7f, // One result: i32
        // Function section
        0x03, // Section ID: Function
        0x02, // Section size: 2 bytes
        0x01, // Number of functions: 1
        0x00, // Function 0 uses type 0
        // Export section
        0x07, // Section ID: Export
        0x08, // Section size: 8 bytes
        0x01, // Number of exports: 1
        0x04, // Export name length: 4
        0x74, 0x65, 0x73, 0x74, // Export name: "test"
        0x00, // Export kind: function
        0x00, // Export function index: 0
        // Code section
        0x0a, // Section ID: Code
        0x06, // Section size: 6 bytes
        0x01, // Number of function bodies: 1
        0x04, // Function body size: 4 bytes
        0x00, // No local variables
        0x41, 0x2a, // i32.const 42
        0x0b, // end
    ];

    serial_println!("[TEST] Loading Wasm module ({} bytes)...", WASM_TEST.len());
    let mut module = match WasmModule::from_bytes(WASM_TEST) {
        Ok(m) => {
            serial_println!("[ OK ] Wasm module loaded and validated");
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load Wasm module: {:?}", e);
            return;
        }
    };

    serial_println!("[TEST] Executing Wasm function 'test'...");
    match module.call_function("test", &[]) {
        Ok(Some(Value::I32(result))) => {
            if result == 42 {
                serial_println!("[ OK ] Wasm function returned: {} (expected 42)", result);
            } else {
                serial_println!("[FAIL] Wasm function returned: {} (expected 42)", result);
            }
        }
        Ok(other) => {
            serial_println!("[FAIL] Unexpected result: {:?}", other);
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to execute Wasm function: {}", e);
        }
    }

    // Test 2: Wasm module calling host functions
    serial_println!("[TEST] Testing Wasm host function calls...");

    // Wasm module that imports and calls host print function:
    // (module
    //   (import "env" "print" (func $print (param i32)))
    //   (func (export "test_host")
    //     i32.const 42
    //     call $print
    //   )
    // )
    const WASM_HOST_TEST: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // Magic: \0asm
        0x01, 0x00, 0x00, 0x00, // Version: 1
        // Type section
        0x01, // Section ID: Type
        0x08, // Section size: 8 bytes
        0x02, // Number of types: 2
        0x60, 0x01, 0x7f, 0x00, // Type 0: (param i32) -> ()
        0x60, 0x00, 0x00,       // Type 1: () -> ()
        // Import section
        0x02, // Section ID: Import
        0x0d, // Section size: 13 bytes (was 0x0c, incorrect)
        0x01, // Number of imports: 1
        0x03, 0x65, 0x6e, 0x76, // Import module name: length 3, "env"
        0x05, 0x70, 0x72, 0x69, 0x6e, 0x74, // Import field name: length 5, "print"
        0x00, 0x00, // Import kind: function (0x00), type index 0
        // Function section
        0x03, // Section ID: Function
        0x02, // Section size: 2 bytes
        0x01, // Number of functions: 1
        0x01, // Function 0 uses type 1
        // Export section
        0x07, // Section ID: Export
        0x0d, // Section size: 13 bytes
        0x01, // Number of exports: 1
        0x09, // Export name length: 9
        0x74, 0x65, 0x73, 0x74, 0x5f, 0x68, 0x6f, 0x73, 0x74, // "test_host"
        0x00, 0x01, // Export kind: function, function index 1
        // Code section
        0x0a, // Section ID: Code
        0x08, // Section size: 8 bytes (1 count + 1 body_size + 6 body)
        0x01, // Number of function bodies: 1
        0x06, // Function body size: 6 bytes
        0x00, // No locals (1 byte)
        0x41, 0x2a, // i32.const 42 (2 bytes)
        0x10, 0x00, // call function 0 (2 bytes)
        0x0b,       // end (1 byte)
    ];

    serial_println!("[TEST] Loading Wasm module with host imports ({} bytes)...", WASM_HOST_TEST.len());
    let mut host_module = match WasmModule::from_bytes(WASM_HOST_TEST) {
        Ok(m) => {
            serial_println!("[ OK ] Wasm module with imports loaded");
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load host module: {:?}", e);
            return;
        }
    };

    serial_println!("[TEST] Calling Wasm function that invokes host print...");
    match host_module.call_function("test_host", &[]) {
        Ok(_) => {
            serial_println!("[ OK ] Host function call succeeded");
        }
        Err(e) => {
            serial_println!("[WARN] Host function call: {}", e);
        }
    }

    serial_println!("[TEST] WebAssembly execution tests complete");
}

/// Test task 1 - prints message and yields
fn task1_main() -> ! {
    for i in 0..5 {
        serial_println!("[TASK1] Iteration {}", i);
        scheduler::task_yield();  // Yield to other tasks
    }
    serial_println!("[TASK1] Completed");
    loop {
        scheduler::task_yield();  // Keep yielding when done
    }
}

/// Test task 2 - prints message and yields
fn task2_main() -> ! {
    for i in 0..5 {
        serial_println!("[TASK2] Iteration {}", i);
        scheduler::task_yield();  // Yield to other tasks
    }
    serial_println!("[TASK2] Completed");
    loop {
        scheduler::task_yield();  // Keep yielding when done
    }
}

/// Test task 3 - prints message and yields
fn task3_main() -> ! {
    for i in 0..5 {
        serial_println!("[TASK3] Iteration {}", i);
        scheduler::task_yield();  // Yield to other tasks
    }
    serial_println!("[TASK3] Completed");
    loop {
        scheduler::task_yield();  // Keep yielding when done
    }
}

/// Test IPC sender task - sends messages to receiver
fn ipc_sender_main() -> ! {
    use alloc::vec;
    use capability::CapabilityId;

    // Give receiver time to set up
    for _ in 0..3 {
        scheduler::task_yield();
    }

    serial_println!("[IPC_SENDER] Starting message transmission");

    // Get current task ID for sending
    let sender_id = scheduler::SCHEDULER.lock()
        .as_ref()
        .unwrap()
        .current_task()
        .unwrap();

    // Use capability ID 100 for IPC endpoint
    let endpoint_cap = CapabilityId::new(100);

    // Send 3 messages
    for i in 0..3 {
        let message_data = vec![b'A' + i, b'0' + i, 0];  // Simple message
        serial_println!("[IPC_SENDER] Sending message {}", i);

        match ipc::send_message(sender_id, endpoint_cap, message_data) {
            Ok(()) => serial_println!("[IPC_SENDER] Message {} sent successfully", i),
            Err(e) => serial_println!("[IPC_SENDER] Failed to send message {}: {:?}", i, e),
        }

        scheduler::task_yield();
    }

    serial_println!("[IPC_SENDER] All messages sent, going idle");

    loop {
        scheduler::task_yield();
    }
}

/// Test IPC receiver task - receives messages from sender
fn ipc_receiver_main() -> ! {
    use capability::CapabilityId;

    serial_println!("[IPC_RECEIVER] Starting, creating endpoint");

    // Get current task ID
    let receiver_id = scheduler::SCHEDULER.lock()
        .as_ref()
        .unwrap()
        .current_task()
        .unwrap();

    // Create IPC endpoint with capability ID 100
    let endpoint_cap = CapabilityId::new(100);
    match ipc::create_endpoint(endpoint_cap) {
        Ok(_) => serial_println!("[IPC_RECEIVER] Endpoint created successfully"),
        Err(e) => {
            serial_println!("[IPC_RECEIVER] Failed to create endpoint: {:?}", e);
            loop { scheduler::task_yield(); }
        }
    }

    serial_println!("[IPC_RECEIVER] Waiting for messages...");

    // Receive 3 messages
    let mut received = 0;
    while received < 3 {
        serial_println!("[IPC_RECEIVER] Attempting to receive message {}...", received);

        match ipc::try_receive_message(receiver_id, endpoint_cap) {
            Ok(Some(msg)) => {
                serial_println!("[IPC_RECEIVER] Received message from task {}: {:?}",
                    msg.sender.value(), msg.data);
                received += 1;
            }
            Ok(None) => {
                // No message yet
                scheduler::task_yield();
            }
            Err(e) => {
                serial_println!("[IPC_RECEIVER] Error receiving: {:?}", e);
                scheduler::task_yield();
            }
        }
    }

    serial_println!("[IPC_RECEIVER] All messages received, going idle");

    loop {
        scheduler::task_yield();
    }
}

/// Benchmark task - measures context switch performance
fn benchmark_task() -> ! {
    // Wait for other tasks to start
    for _ in 0..2 {
        scheduler::task_yield();
    }

    serial_println!("[BENCH] Starting context switch benchmark...");

    // Perform 10 measured context switches (quick test)
    let iterations = 10;
    let start = benchmark::rdtsc();

    for _ in 0..iterations {
        scheduler::task_yield();
    }

    let end = benchmark::rdtsc();
    let total_cycles = end - start;
    let avg_cycles = total_cycles / iterations;

    serial_println!("[BENCH] Context switch benchmark complete:");
    serial_println!("[BENCH]   {} iterations in {} cycles", iterations, total_cycles);
    serial_println!("[BENCH]   Average: {} cycles ({} ns)",
        avg_cycles, benchmark::cycles_to_ns(avg_cycles));

    // Record for final results
    benchmark::record_context_switch(avg_cycles);

    // Wait a bit for IPC tasks to finish
    for _ in 0..5 {
        scheduler::task_yield();
    }

    // Collect and print final benchmark results
    serial_println!("");
    serial_println!("[BENCH] Collecting final benchmark results...");

    // Get boot cycles from global variable
    let boot_cycles = BOOT_CYCLES.load(core::sync::atomic::Ordering::Relaxed);
    let results = benchmark::collect_results(boot_cycles);
    results.print();

    // Also print memory footprint
    benchmark::estimate_memory_footprint();

    serial_println!("");
    serial_println!("[BENCH] Benchmark complete - system continues running");

    // Continue yielding
    loop {
        scheduler::task_yield();
    }
}

/// Test the task scheduler
fn test_scheduler() {
    use task::{Task, Priority, TaskContext};

    serial_println!("[TEST] Testing multitasking with IPC...");

    // Create tasks: IPC test + benchmark task
    serial_println!("[TEST] Creating tasks (IPC + benchmarks)...");
    let receiver = Task::new("ipc_receiver", ipc_receiver_main, Priority::Normal);
    let sender = Task::new("ipc_sender", ipc_sender_main, Priority::Normal);
    let bencher = Task::new("benchmark", benchmark_task, Priority::Normal);
    let task3 = Task::new("task3", task3_main, Priority::Normal);

    {
        let mut sched = scheduler::SCHEDULER.lock();
        let sched = sched.as_mut().expect("Scheduler not initialized");

        let id_receiver = sched.add_task(receiver);
        let id_sender = sched.add_task(sender);
        let id_bench = sched.add_task(bencher);
        let id3 = sched.add_task(task3);

        serial_println!("[ OK ] Created 4 tasks: {}, {}, {}, {}",
            id_receiver.value(), id_sender.value(), id_bench.value(), id3.value());

        // Schedule first task
        serial_println!("[TEST] Starting multitasking with IPC...");
        sched.schedule();
    }

    serial_println!("[TEST] About to switch to first task...");

    // Create a dummy kernel context to save (we won't return here)
    let mut kernel_context = TaskContext::new();

    // Get first task context
    let first_task_context = {
        let sched = scheduler::SCHEDULER.lock();
        let sched = sched.as_ref().unwrap();
        let current_id = sched.current_task().unwrap();
        let task = sched.get_task(current_id).unwrap();
        *task.context()  // Use accessor method and dereference to copy
    };

    serial_println!("[TEST] Jumping to task 1...");

    // Switch to first task (THIS WON'T RETURN - tasks will run forever)
    unsafe {
        scheduler::switch_context(&mut kernel_context, &first_task_context);
    }

    // Should never reach here
    unreachable!("Returned from task execution");
}

/// Panic handler - called on kernel panic
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Only use serial output - VGA buffer may not be mapped yet
    serial_println!("[PANIC] {}", info);

    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

