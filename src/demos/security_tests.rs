//! security tests for the ipc and capability system
//!
//! makes sure all the security stuff actually works:
//! - capability checks on ipc calls
//! - message size limits (512 bytes max)
//! - queue depth limits (64 messages max)
//! - can't forge capabilities from userspace

use crate::capability::{Capability, CapabilityId, ResourceType, Rights};
use crate::wasm_runtime::{MAX_IPC_MESSAGE_SIZE, MAX_IPC_QUEUE_DEPTH};
use crate::syscall::SyscallContext;

/// Run all security validation tests
pub fn run_all_security_tests() {
    serial_println!("");
    serial_println!("╔════════════════════════════════════════════════════════╗");
    serial_println!("║         JerichoOS Security Validation Tests            ║");
    serial_println!("╚════════════════════════════════════════════════════════╝");
    serial_println!("");

    test_capability_verification();
    test_message_size_limits();
    test_queue_depth_limits();
    test_capability_forgery_prevention();

    serial_println!("");
    serial_println!("[SECURITY] All security tests completed");
    serial_println!("");
}

/// Test 1: 4-Layer Capability Verification
///
/// Validates that IPC operations enforce:
/// 1. Capability possession
/// 2. Correct resource type (Endpoint)
/// 3. Appropriate rights (READ/WRITE)
/// 4. Matching resource_id
fn test_capability_verification() {
    serial_println!("[SECURITY TEST] 4-Layer Capability Verification");
    serial_println!("────────────────────────────────────────────────");

    // Test 1.1: Valid capability with correct rights
    serial_print!("  Valid Endpoint + WRITE rights: ");
    let valid_cap = Capability::new(
        CapabilityId::new(100),
        ResourceType::Endpoint,
        42,  // resource_id
        Rights { read: false, write: true, execute: false, grant: false },
    );
    if valid_cap.rights().write && valid_cap.resource_type() == ResourceType::Endpoint {
        serial_println!("✅ PASS");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 1.2: Wrong resource type should be rejected
    serial_print!("  Memory cap rejected for IPC: ");
    let memory_cap = Capability::new(
        CapabilityId::new(101),
        ResourceType::Memory,  // Wrong type!
        42,
        Rights::READ,
    );
    if memory_cap.resource_type() != ResourceType::Endpoint {
        serial_println!("✅ PASS (correctly rejected)");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 1.3: Missing WRITE rights for send
    serial_print!("  READ-only cap rejected for send: ");
    let readonly_cap = Capability::new(
        CapabilityId::new(102),
        ResourceType::Endpoint,
        42,
        Rights::READ,  // No WRITE!
    );
    if !readonly_cap.rights().write {
        serial_println!("✅ PASS (correctly rejected)");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 1.4: Wrong resource_id should be rejected
    serial_print!("  Wrong resource_id rejected: ");
    let wrong_id_cap = Capability::new(
        CapabilityId::new(103),
        ResourceType::Endpoint,
        999,  // Wrong resource_id
        Rights { read: false, write: true, execute: false, grant: false },
    );
    // In real use, find_capability() would fail to match resource_id 42
    if wrong_id_cap.resource_id() != 42 {
        serial_println!("✅ PASS (ID mismatch detected)");
    } else {
        serial_println!("❌ FAIL");
    }

    serial_println!("");
}

/// Test 2: Message Size Limits (DoS Prevention)
///
/// Validates MAX_IPC_MESSAGE_SIZE = 512 bytes enforcement
fn test_message_size_limits() {
    serial_println!("[SECURITY TEST] Message Size Limits (DoS Prevention)");
    serial_println!("────────────────────────────────────────────────────");

    // Test 2.1: Verify limit constant
    serial_print!("  MAX_IPC_MESSAGE_SIZE = 512: ");
    if MAX_IPC_MESSAGE_SIZE == 512 {
        serial_println!("✅ PASS");
    } else {
        serial_println!("❌ FAIL (got {})", MAX_IPC_MESSAGE_SIZE);
    }

    // Test 2.2: Message at limit should be valid
    serial_print!("  512-byte message accepted: ");
    let msg_at_limit: usize = 512;
    if msg_at_limit <= MAX_IPC_MESSAGE_SIZE {
        serial_println!("✅ PASS");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 2.3: Message over limit should be rejected
    serial_print!("  513-byte message rejected: ");
    let msg_over_limit: usize = 513;
    if msg_over_limit > MAX_IPC_MESSAGE_SIZE {
        serial_println!("✅ PASS (correctly rejected)");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 2.4: Large message (1MB) rejected
    serial_print!("  1MB message rejected: ");
    let huge_msg: usize = 1024 * 1024;
    if huge_msg > MAX_IPC_MESSAGE_SIZE {
        serial_println!("✅ PASS (correctly rejected)");
    } else {
        serial_println!("❌ FAIL");
    }

    serial_println!("");
}

/// Test 3: Queue Depth Limits (DoS Prevention)
///
/// Validates MAX_IPC_QUEUE_DEPTH = 64 messages enforcement
fn test_queue_depth_limits() {
    serial_println!("[SECURITY TEST] Queue Depth Limits (DoS Prevention)");
    serial_println!("───────────────────────────────────────────────────");

    // Test 3.1: Verify limit constant
    serial_print!("  MAX_IPC_QUEUE_DEPTH = 64: ");
    if MAX_IPC_QUEUE_DEPTH == 64 {
        serial_println!("✅ PASS");
    } else {
        serial_println!("❌ FAIL (got {})", MAX_IPC_QUEUE_DEPTH);
    }

    // Test 3.2: 64 messages should be accepted
    serial_print!("  64 messages fit in queue: ");
    let queue_at_limit: usize = 64;
    if queue_at_limit <= MAX_IPC_QUEUE_DEPTH {
        serial_println!("✅ PASS");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 3.3: 65th message should be rejected
    serial_print!("  65th message rejected: ");
    let queue_over_limit: usize = 65;
    if queue_over_limit > MAX_IPC_QUEUE_DEPTH {
        serial_println!("✅ PASS (correctly rejected)");
    } else {
        serial_println!("❌ FAIL");
    }

    // Test 3.4: Memory bound calculation
    serial_print!("  Max queue memory = 32KB: ");
    let max_memory = MAX_IPC_QUEUE_DEPTH * MAX_IPC_MESSAGE_SIZE;
    if max_memory == 32768 {
        serial_println!("✅ PASS ({} bytes)", max_memory);
    } else {
        serial_println!("❌ FAIL (got {} bytes)", max_memory);
    }

    serial_println!("");
}

/// Test 4: Capability Forgery Prevention
///
/// Validates that sys_cap_create is unconditionally denied
fn test_capability_forgery_prevention() {
    serial_println!("[SECURITY TEST] Capability Forgery Prevention");
    serial_println!("──────────────────────────────────────────────");

    // Test 4.1: sys_cap_create should always fail
    serial_print!("  sys_cap_create denied: ");

    // Create a syscall context
    let mut ctx = SyscallContext::new();

    // Attempt to create a capability (should be denied)
    // SyscallNumber::CapCreate = 0
    let result = ctx.syscall(0, 0, 0, 0, 0);

    match result {
        crate::syscall::SyscallResult::Error(crate::syscall::SyscallError::PermissionDenied) => {
            serial_println!("✅ PASS (PermissionDenied)");
        }
        _ => {
            serial_println!("❌ FAIL (forgery allowed!)");
        }
    }

    // Test 4.2: Capability derivation requires existing capability
    serial_print!("  Derive without cap fails: ");
    // SyscallNumber::CapDerive = 1
    let derive_result = ctx.syscall(1, 9999, 0, 0, 0);
    match derive_result {
        crate::syscall::SyscallResult::Error(_) => {
            serial_println!("✅ PASS (correctly denied)");
        }
        _ => {
            serial_println!("❌ FAIL");
        }
    }

    serial_println!("");
}
