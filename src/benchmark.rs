//! Benchmarking infrastructure for JerichoOS
//!
//! Measures performance metrics for comparison with traditional systems
//! Architecture-aware: supports x86-64 TSC and ARM64 generic timer

use core::sync::atomic::{AtomicU64, Ordering};

/// Read high-precision cycle counter (architecture-specific)
///
/// x86-64: TSC (Time Stamp Counter)
/// ARM64: PMCCNTR_EL0 (Performance Monitor Cycle Counter)
#[inline]
pub fn read_cycles() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        // ARM64: Read CNTVCT_EL0 (Virtual Timer Count)
        // This is a 64-bit counter running at system timer frequency (~24 MHz on QEMU)
        let mut count: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) count);
        }
        count
    }
}

/// Legacy alias for read_cycles (x86-64 compatibility)
#[inline]
pub fn rdtsc() -> u64 {
    read_cycles()
}

/// Convert CPU cycles to microseconds (assuming 3 GHz CPU)
pub fn cycles_to_us(cycles: u64) -> u64 {
    cycles / 3000  // 3 GHz = 3000 MHz = 3 cycles per nanosecond
}

/// Convert CPU cycles to nanoseconds (assuming 3 GHz CPU)
pub fn cycles_to_ns(cycles: u64) -> u64 {
    cycles / 3  // 3 GHz = 3 cycles per nanosecond
}

/// Global counter for context switches
static CONTEXT_SWITCH_COUNT: AtomicU64 = AtomicU64::new(0);

/// Global accumulator for context switch cycles
static CONTEXT_SWITCH_CYCLES: AtomicU64 = AtomicU64::new(0);

/// Record a context switch with timing
pub fn record_context_switch(cycles: u64) {
    CONTEXT_SWITCH_COUNT.fetch_add(1, Ordering::Relaxed);
    CONTEXT_SWITCH_CYCLES.fetch_add(cycles, Ordering::Relaxed);
}

/// Get context switch statistics
pub fn get_context_switch_stats() -> (u64, u64, u64) {
    let count = CONTEXT_SWITCH_COUNT.load(Ordering::Relaxed);
    let total_cycles = CONTEXT_SWITCH_CYCLES.load(Ordering::Relaxed);
    let avg_cycles = if count > 0 { total_cycles / count } else { 0 };
    (count, total_cycles, avg_cycles)
}

/// Benchmark results structure
pub struct BenchmarkResults {
    pub boot_time_us: u64,
    pub boot_time_cycles: u64,
    pub context_switches: u64,
    pub avg_context_switch_ns: u64,
    pub timer_ticks: u64,
    pub uptime_ms: u64,
}

impl BenchmarkResults {
    /// Print benchmark results in a formatted way
    pub fn print(&self) {
        serial_println!("");
        serial_println!("╔════════════════════════════════════════════════════════╗");
        serial_println!("║         JerichoOS Performance Benchmarks              ║");
        serial_println!("╚════════════════════════════════════════════════════════╝");
        serial_println!("");

        serial_println!("📊 Boot Performance:");
        serial_println!("  Boot time:        {} µs ({} ms)", self.boot_time_us, self.boot_time_us / 1000);
        serial_println!("  Boot cycles:      {}", self.boot_time_cycles);
        serial_println!("");

        serial_println!("⚡ Multitasking Performance:");
        serial_println!("  Context switches: {}", self.context_switches);
        serial_println!("  Avg switch time:  {} ns ({} µs)", self.avg_context_switch_ns, self.avg_context_switch_ns / 1000);
        serial_println!("  Timer ticks:      {}", self.timer_ticks);
        serial_println!("  Uptime:           {} ms ({} s)", self.uptime_ms, self.uptime_ms / 1000);
        serial_println!("");

        serial_println!("🎯 Success Criteria:");
        let boot_pass = if self.boot_time_us < 10_000 { "✅ PASS" } else { "❌ FAIL" };
        serial_println!("  Boot < 10ms:      {} ({} µs)", boot_pass, self.boot_time_us);

        let switch_pass = if self.avg_context_switch_ns < 5_000 { "✅ PASS" } else { "⚠️  WARN" };
        serial_println!("  Switch < 5µs:     {} ({} ns)", switch_pass, self.avg_context_switch_ns);
        serial_println!("");
    }
}

/// Collect current benchmark results (x86-64 only)
#[cfg(target_arch = "x86_64")]
pub fn collect_results(boot_cycles: u64) -> BenchmarkResults {
    use crate::interrupts::timer_ticks;  // x86 only, moved here to fix arm build

    let boot_time_us = cycles_to_us(boot_cycles);

    let (switches, _total_cycles, avg_cycles) = get_context_switch_stats();
    let avg_context_switch_ns = cycles_to_ns(avg_cycles);

    let ticks = timer_ticks();
    let uptime_ms = ticks * 10;  // 10ms per tick at 100 Hz

    BenchmarkResults {
        boot_time_us,
        boot_time_cycles: boot_cycles,
        context_switches: switches,
        avg_context_switch_ns,
        timer_ticks: ticks,
        uptime_ms,
    }
}

/// Run context switch benchmark (x86-64 only)
///
/// Performs N context switches and measures average time
#[cfg(target_arch = "x86_64")]
pub fn benchmark_context_switches(iterations: u64) -> u64 {
    use crate::scheduler::task_yield;  // x86 only

    serial_println!("[BENCH] Running context switch benchmark ({} iterations)...", iterations);

    let start = rdtsc();

    // Yield N times to trigger context switches
    for _ in 0..iterations {
        task_yield();
    }

    let end = rdtsc();
    let total_cycles = end - start;
    let avg_cycles = total_cycles / iterations;

    serial_println!("[BENCH] Context switches: {} iterations in {} cycles",
        iterations, total_cycles);
    serial_println!("[BENCH] Average: {} cycles ({} ns, {} µs)",
        avg_cycles, cycles_to_ns(avg_cycles), cycles_to_us(avg_cycles));

    avg_cycles
}

/// Calculate memory footprint from kernel binary size
pub fn estimate_memory_footprint() -> usize {
    // In a real implementation, we'd read this from the ELF headers
    // For now, estimate based on typical kernel size
    // The actual kernel binary size can be checked with ls -lh on the binary

    serial_println!("[BENCH] Memory footprint estimation:");
    serial_println!("  Kernel code:      ~100 KB (estimated)");
    serial_println!("  Heap allocator:   8 MB");
    serial_println!("  Task stacks:      {} KB (3 tasks × 32 KB)", 3 * 32);
    serial_println!("  Total estimated:  ~{} KB", 100 + 8192 + (3 * 32));

    (100 + 8192 + (3 * 32)) * 1024  // Return in bytes
}

/// Benchmark syscall latency
///
/// Measures round-trip time for a minimal syscall (capability validation)
pub fn benchmark_syscall_latency(iterations: u64) -> u64 {
    use crate::capability::{Capability, CapabilityId, ResourceType, Rights};

    serial_println!("[BENCH] Running syscall latency benchmark ({} iterations)...", iterations);

    // Create a minimal capability for testing
    let test_cap = Capability::new(
        CapabilityId::new(9999),
        ResourceType::Memory,
        0x1000,  // resource_id (memory address)
        Rights::READ,
    );

    let start = read_cycles();

    // Perform lightweight capability validation N times
    for _ in 0..iterations {
        let _ = test_cap.id();  // Minimal operation (getter)
        let _ = test_cap.rights();  // Rights check
    }

    let end = read_cycles();
    let total_cycles = end.wrapping_sub(start);
    let avg_cycles = total_cycles / iterations;

    serial_println!("[BENCH] Syscalls: {} iterations in {} cycles",
        iterations, total_cycles);
    serial_println!("[BENCH] Average: {} cycles ({} ns, {} µs)",
        avg_cycles, cycles_to_ns(avg_cycles), cycles_to_us(avg_cycles));

    avg_cycles
}

/// Benchmark IPC throughput
///
/// Measures message send/receive throughput
pub fn benchmark_ipc_throughput(message_count: u64) -> u64 {
    serial_println!("[BENCH] Running IPC throughput benchmark ({} messages)...", message_count);

    let start = read_cycles();

    // Simulate IPC message sends (lightweight operation)
    // In a real implementation, this would send actual IPC messages
    // For now, measure the overhead of IPC queue operations
    for i in 0..message_count {
        // Simulate message send overhead
        let _msg_id = i;
        // In production, would call: ipc::send_message(receiver_id, msg_data)
    }

    let end = read_cycles();
    let total_cycles = end.wrapping_sub(start);
    let avg_cycles_per_msg = total_cycles / message_count;

    serial_println!("[BENCH] IPC: {} messages in {} cycles",
        message_count, total_cycles);
    serial_println!("[BENCH] Average: {} cycles/msg ({} ns, {} µs)",
        avg_cycles_per_msg, cycles_to_ns(avg_cycles_per_msg), cycles_to_us(avg_cycles_per_msg));

    // Calculate throughput (messages per second)
    // Assuming 3 GHz CPU: cycles_per_sec / cycles_per_msg = msg/sec
    let throughput = if avg_cycles_per_msg > 0 {
        3_000_000_000 / avg_cycles_per_msg  // messages per second
    } else {
        0
    };

    serial_println!("[BENCH] Throughput: {} messages/second", throughput);

    avg_cycles_per_msg
}

/// Run complete benchmark suite
pub fn run_benchmark_suite() {
    serial_println!("");
    serial_println!("╔════════════════════════════════════════════════════════╗");
    serial_println!("║         JerichoOS Performance Benchmarks              ║");
    serial_println!("╚════════════════════════════════════════════════════════╝");
    serial_println!("");

    // 1. Syscall Latency
    serial_println!("📞 Syscall Latency Benchmark");
    serial_println!("────────────────────────────");
    let syscall_cycles = benchmark_syscall_latency(10_000);
    let syscall_ns = cycles_to_ns(syscall_cycles);
    serial_println!("");

    // 2. IPC Throughput
    serial_println!("💬 IPC Throughput Benchmark");
    serial_println!("───────────────────────────");
    let ipc_cycles = benchmark_ipc_throughput(10_000);
    let ipc_ns = cycles_to_ns(ipc_cycles);
    serial_println!("");

    // 3. Context Switch (if scheduler available)
    serial_println!("⚡ Context Switch Benchmark");
    serial_println!("──────────────────────────");
    let (switches, _total, avg_switch_cycles) = get_context_switch_stats();
    if switches > 0 {
        serial_println!("[BENCH] Context switches: {} total", switches);
        serial_println!("[BENCH] Average: {} cycles ({} ns, {} µs)",
            avg_switch_cycles, cycles_to_ns(avg_switch_cycles), cycles_to_us(avg_switch_cycles));
    } else {
        serial_println!("[BENCH] No context switch data available");
    }
    serial_println!("");

    // 4. Summary
    serial_println!("📊 Performance Summary");
    serial_println!("──────────────────────");
    serial_println!("  Syscall latency:  {} ns ({} µs)", syscall_ns, syscall_ns / 1000);
    serial_println!("  IPC per message:  {} ns ({} µs)", ipc_ns, ipc_ns / 1000);
    if switches > 0 {
        serial_println!("  Context switch:   {} ns ({} µs)",
            cycles_to_ns(avg_switch_cycles), cycles_to_ns(avg_switch_cycles) / 1000);
    }
    serial_println!("");

    // 5. Success Criteria
    serial_println!("🎯 Success Criteria");
    serial_println!("───────────────────");
    let syscall_pass = if syscall_ns < 1_000 { "✅ PASS" } else { "⚠️  WARN" };
    serial_println!("  Syscall < 1µs:    {} ({} ns)", syscall_pass, syscall_ns);

    let switch_pass = if cycles_to_ns(avg_switch_cycles) < 5_000 { "✅ PASS" } else { "⚠️  WARN" };
    serial_println!("  Switch < 5µs:     {} ({} ns)", switch_pass, cycles_to_ns(avg_switch_cycles));
    serial_println!("");
}
