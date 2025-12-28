//! ARM64-specific benchmarking utilities
//!
//! Uses ARM Generic Timer counters for high-precision timing

use core::arch::asm;

/// Read the virtual counter (CNTVCT_EL0) for high-precision timing
///
/// This is the ARM64 equivalent of x86's rdtsc instruction.
/// The virtual counter increments at a fixed frequency (usually 1 GHz on QEMU).
#[inline]
pub fn read_counter() -> u64 {
    let count: u64;
    unsafe {
        asm!(
            "mrs {0}, cntvct_el0",
            out(reg) count,
            options(nomem, nostack, preserves_flags)
        );
    }
    count
}

/// Read the counter frequency from CNTFRQ_EL0
///
/// Returns the frequency in Hz (e.g., 1000000000 for 1 GHz)
#[inline]
pub fn read_counter_frequency() -> u64 {
    let freq: u64;
    unsafe {
        asm!(
            "mrs {0}, cntfrq_el0",
            out(reg) freq,
            options(nomem, nostack, preserves_flags)
        );
    }
    freq
}

/// Convert counter ticks to microseconds
///
/// Uses the actual hardware counter frequency instead of assuming a fixed CPU speed
pub fn ticks_to_us(ticks: u64) -> u64 {
    let freq = read_counter_frequency();
    // ticks / (freq / 1_000_000) = ticks * 1_000_000 / freq
    (ticks * 1_000_000) / freq
}

/// Convert counter ticks to nanoseconds
///
/// Uses the actual hardware counter frequency instead of assuming a fixed CPU speed
pub fn ticks_to_ns(ticks: u64) -> u64 {
    let freq = read_counter_frequency();
    // ticks / (freq / 1_000_000_000) = ticks * 1_000_000_000 / freq
    (ticks * 1_000_000_000) / freq
}

/// Get counter frequency in human-readable format
pub fn get_counter_info() -> (u64, &'static str) {
    let freq = read_counter_frequency();
    if freq >= 1_000_000_000 {
        (freq / 1_000_000_000, "GHz")
    } else if freq >= 1_000_000 {
        (freq / 1_000_000, "MHz")
    } else if freq >= 1_000 {
        (freq / 1_000, "KHz")
    } else {
        (freq, "Hz")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_reads() {
        let c1 = read_counter();
        let c2 = read_counter();
        assert!(c2 > c1, "Counter should be monotonically increasing");
    }

    #[test]
    fn test_frequency_read() {
        let freq = read_counter_frequency();
        assert!(freq > 0, "Frequency must be non-zero");
        // QEMU typically uses 1 GHz (62.5 MHz on real hardware like RPi)
        assert!(freq >= 1_000_000, "Frequency should be at least 1 MHz");
    }

    #[test]
    fn test_time_conversion() {
        let freq = read_counter_frequency();

        // Test: 1 second worth of ticks should convert to 1_000_000 microseconds
        let ticks = freq; // 1 second
        let us = ticks_to_us(ticks);
        assert_eq!(us, 1_000_000, "1 second should be 1000000 microseconds");

        // Test: 1 millisecond worth of ticks
        let ticks_ms = freq / 1000;
        let us_ms = ticks_to_us(ticks_ms);
        assert_eq!(us_ms, 1_000, "1 millisecond should be 1000 microseconds");
    }
}
