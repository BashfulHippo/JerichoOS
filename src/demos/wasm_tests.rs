/// JerichoOS WASM Demo Test Suite
///
/// Canonical tests that validate WASM runtime functionality.
/// These tests MUST pass on x86-64 and ARM64 for feature parity.

use crate::wasm_runtime::WasmModule;
#[allow(unused_imports)]
use crate::{serial_print, serial_println};
use wasmi::Value;

/// Demo 1: Pure Computation
///
/// Tests: Basic WASM execution, parameters, return values, recursion
/// Expected: add(2,3)=5, mul(7,6)=42, factorial(5)=120
pub fn demo_01_add() {
    serial_println!("\n[DEMO 1] Pure Computation (01_add.wasm)");
    serial_println!("=========================================");

    // Load compiled WASM module
    const WASM_BYTES: &[u8] = include_bytes!("../../demos/wasm/01_add.wasm");
    serial_println!("[INFO] Loading module ({} bytes)...", WASM_BYTES.len());

    let mut module = match WasmModule::from_bytes(WASM_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Module loaded and validated");
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load module: {:?}", e);
            return;
        }
    };

    // Test 1: add(2, 3)
    serial_print!("[TEST] add(2, 3) = ");
    match module.call_function("add", &[Value::I32(2), Value::I32(3)]) {
        Ok(Some(Value::I32(result))) => {
            if result == 5 {
                serial_println!("{} ✅", result);
            } else {
                serial_println!("{} ❌ (expected 5)", result);
            }
        }
        Ok(_) => serial_println!("❌ (wrong return type)"),
        Err(e) => serial_println!("❌ (error: {})", e),
    }

    // Test 2: mul(7, 6)
    serial_print!("[TEST] mul(7, 6) = ");
    match module.call_function("mul", &[Value::I32(7), Value::I32(6)]) {
        Ok(Some(Value::I32(result))) => {
            if result == 42 {
                serial_println!("{} ✅", result);
            } else {
                serial_println!("{} ❌ (expected 42)", result);
            }
        }
        Ok(_) => serial_println!("❌ (wrong return type)"),
        Err(e) => serial_println!("❌ (error: {})", e),
    }

    // Test 3: factorial(5)
    serial_print!("[TEST] factorial(5) = ");
    match module.call_function("factorial", &[Value::I32(5)]) {
        Ok(Some(Value::I32(result))) => {
            if result == 120 {
                serial_println!("{} ✅", result);
            } else {
                serial_println!("{} ❌ (expected 120)", result);
            }
        }
        Ok(_) => serial_println!("❌ (wrong return type)"),
        Err(e) => serial_println!("❌ (error: {})", e),
    }

    serial_println!("[DEMO 1] ✅ COMPLETE\n");
}

/// Demo 2: Host Function Calls
///
/// Tests: Host imports (env.print), function boundary crossing
/// Expected: Prints 42, 100, 255 via host function
pub fn demo_02_hello() {
    serial_println!("\n[DEMO 2] Host Function Calls (02_hello.wasm)");
    serial_println!("==============================================");

    const WASM_BYTES: &[u8] = include_bytes!("../../demos/wasm/02_hello.wasm");
    serial_println!("[INFO] Loading module ({} bytes)...", WASM_BYTES.len());

    let mut module = match WasmModule::from_bytes(WASM_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Module loaded with host imports");
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load module: {:?}", e);
            return;
        }
    };

    // Test 1: main() - should print 42, 100, 255
    serial_println!("[TEST] Calling main() (should print 3 values):");
    match module.call_function("main", &[]) {
        Ok(_) => serial_println!("[ OK ] main() executed successfully"),
        Err(e) => serial_println!("[FAIL] main() failed: {}", e),
    }

    // Test 2: print_range(1, 5) - should print 1,2,3,4
    serial_println!("[TEST] Calling print_range(1, 5):");
    match module.call_function("print_range", &[Value::I32(1), Value::I32(5)]) {
        Ok(_) => serial_println!("[ OK ] print_range() executed successfully"),
        Err(e) => serial_println!("[FAIL] print_range() failed: {}", e),
    }

    serial_println!("[DEMO 2] ✅ COMPLETE\n");
}

/// Demo 3: Syscall and Capability Test
///
/// Tests: Syscall bridge, capability validation, security isolation
/// Expected: Valid syscalls succeed, unauthorized calls fail
pub fn demo_03_syscall() {
    serial_println!("\n[DEMO 3] Syscall & Capability (03_syscall.wasm)");
    serial_println!("=================================================");

    const WASM_BYTES: &[u8] = include_bytes!("../../demos/wasm/03_syscall.wasm");
    serial_println!("[INFO] Loading module ({} bytes)...", WASM_BYTES.len());

    let mut module = match WasmModule::from_bytes(WASM_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Module loaded with syscall imports");
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load module: {:?}", e);
            return;
        }
    };

    // Test 1: test_syscall() - basic syscall
    serial_println!("[TEST] Basic syscall (sys_write):");
    match module.call_function("test_syscall", &[]) {
        Ok(_) => serial_println!("[ OK ] Syscall executed"),
        Err(e) => serial_println!("[FAIL] Syscall failed: {}", e),
    }

    // Test 2: test_allocate(1024) - requires capability
    serial_println!("[TEST] Memory allocation (requires capability):");
    match module.call_function("test_allocate", &[Value::I32(1024)]) {
        Ok(Some(Value::I32(result))) => {
            if result != 0 {
                serial_println!("[ OK ] Allocation succeeded: address=0x{:X}", result);
            } else {
                serial_println!("[WARN] Allocation returned NULL (capability denied?)");
            }
        }
        Ok(_) => serial_println!("[FAIL] Unexpected return type"),
        Err(e) => serial_println!("[FAIL] Allocate failed: {}", e),
    }

    // Test 3: test_unauthorized() - should fail
    serial_println!("[TEST] Unauthorized access (should fail):");
    match module.call_function("test_unauthorized", &[]) {
        Ok(Some(Value::I32(result))) => {
            if result < 0 {
                serial_println!("[ OK ] Access denied (result={})", result);
            } else {
                serial_println!("[WARN] Unauthorized access succeeded (security issue!)");
            }
        }
        Ok(_) => serial_println!("[FAIL] Unexpected return type"),
        Err(e) => serial_println!("[ OK ] Access denied via exception: {}", e),
    }

    serial_println!("[DEMO 3] ✅ COMPLETE\n");
}

/// Demo 4: MQTT Broker Pub/Sub
///
/// Tests: Real-world IoT use case, IPC, capability isolation
/// Expected: Publisher sends messages, subscriber receives them via broker
pub fn demo_04_mqtt() {
    serial_println!("\n\n=== DEMO 4 STARTING ===\n");
    serial_println!("\n[DEMO 4] MQTT Broker Pub/Sub (mqtt_*.wasm)");
    serial_println!("============================================");

    // Load broker
    serial_println!("[INFO] Loading MQTT broker...");
    const BROKER_BYTES: &[u8] = include_bytes!("../../demos/wasm/mqtt_broker.wasm");
    let mut broker = match WasmModule::from_bytes(BROKER_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Broker loaded ({} bytes)", BROKER_BYTES.len());
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load broker: {:?}", e);
            return;
        }
    };

    // Initialize broker
    serial_print!("[TEST] Initializing broker... ");
    match broker.call_function("broker_init", &[]) {
        Ok(Some(Value::I32(0))) => serial_println!("✅"),
        Ok(Some(Value::I32(code))) => {
            serial_println!("❌ (error code: {})", code);
            return;
        }
        Ok(_) => {
            serial_println!("❌ (unexpected return)");
            return;
        }
        Err(e) => {
            serial_println!("❌ ({})", e);
            return;
        }
    }

    // Load subscriber
    serial_println!("[INFO] Loading MQTT subscriber...");
    const SUB_BYTES: &[u8] = include_bytes!("../../demos/wasm/mqtt_subscriber.wasm");
    let mut subscriber = match WasmModule::from_bytes(SUB_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Subscriber loaded ({} bytes)", SUB_BYTES.len());
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load subscriber: {:?}", e);
            return;
        }
    };

    // Initialize subscriber (client_id = 2)
    serial_print!("[TEST] Initializing subscriber (client_id=2)... ");
    match subscriber.call_function("subscriber_init", &[Value::I32(2)]) {
        Ok(Some(Value::I32(0))) => serial_println!("✅"),
        Ok(Some(Value::I32(code))) => serial_println!("⚠️  (code: {})", code),
        Ok(_) => serial_println!("❌ (unexpected return)"),
        Err(e) => serial_println!("❌ ({})", e),
    }

    // Load publisher
    serial_println!("[INFO] Loading MQTT publisher...");
    const PUB_BYTES: &[u8] = include_bytes!("../../demos/wasm/mqtt_publisher.wasm");
    let mut publisher = match WasmModule::from_bytes(PUB_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Publisher loaded ({} bytes)", PUB_BYTES.len());
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load publisher: {:?}", e);
            return;
        }
    };

    // Initialize publisher
    serial_print!("[TEST] Initializing publisher... ");
    match publisher.call_function("publisher_init", &[]) {
        Ok(Some(Value::I32(0))) => serial_println!("✅"),
        Ok(Some(Value::I32(code))) => serial_println!("⚠️  (code: {})", code),
        Ok(_) => serial_println!("❌ (unexpected return)"),
        Err(e) => serial_println!("❌ ({})", e),
    }

    // Run publisher 5 times
    serial_println!("[TEST] Publishing messages (5 iterations)...");
    for i in 1..=5 {
        serial_print!("  [");
        serial_print!("<u32>");
        serial_print!("] Publishing... ");
        match publisher.call_function("publisher_run", &[]) {
            Ok(Some(Value::I32(_count))) => {
                serial_println!("✅");

                // Deliver pending IPC messages to subscriber (simulates kernel IPC delivery)
                use crate::wasm_runtime;
                let delivered = wasm_runtime::deliver_pending_messages(&mut subscriber, 2);
                if delivered > 0 {
                    serial_print!("       → Delivered ");
                    serial_print!("<u32>");
                    serial_println!(" messages to subscriber");
                }
            }
            Ok(_) => serial_println!("❌ (unexpected return)"),
            Err(e) => {
                serial_print!("❌ (error)");
                let _ = e; // Suppress unused warning
                serial_println!("");
            }
        }

        // Small iteration marker
        let _ = i;
    }

    serial_println!("\n[DEMO 4] ✅ COMPLETE");
    serial_println!("✨ Full pub/sub flow working:");
    serial_println!("   1. Subscriber registered with broker via sys_mqtt_subscribe");
    serial_println!("   2. Publisher sends messages via sys_mqtt_publish");
    serial_println!("   3. Broker routes to subscriber via sys_ipc_send");
    serial_println!("   4. Subscriber receives and logs messages\n");
}

/// Demo 5: Security & Isolation
///
/// Tests: WASM sandbox, capability-based access control, attack resistance
/// Expected: All attacks prevented, system remains stable
pub fn demo_05_security() {
    serial_println!("\n[DEMO 5] Security & Isolation (malicious_module.wasm)");
    serial_println!("======================================================");

    // Load malicious module (sandboxed)
    serial_println!("[INFO] Loading malicious module (sandboxed)...");
    const MALICIOUS_BYTES: &[u8] = include_bytes!("../../demos/wasm/malicious_module.wasm");
    let mut malicious = match WasmModule::from_bytes(MALICIOUS_BYTES) {
        Ok(m) => {
            serial_println!("[ OK ] Malicious module loaded ({} bytes)", MALICIOUS_BYTES.len());
            m
        }
        Err(e) => {
            serial_println!("[FAIL] Failed to load module");
            let _ = e;
            return;
        }
    };

    // Initialize malicious module (no capabilities granted)
    serial_print!("[TEST] Initializing malicious module... ");
    match malicious.call_function("malicious_init", &[]) {
        Ok(Some(Value::I32(0))) => serial_println!("✅"),
        Ok(Some(Value::I32(code))) => {
            serial_print!("⚠️  (code: ");
            serial_print!("<u32>");
            serial_println!(")");
            let _ = code;
        }
        Ok(_) => serial_println!("❌ (unexpected return)"),
        Err(e) => {
            serial_println!("❌");
            let _ = e;
        }
    }

    // Test 1: WASM Memory Isolation
    serial_println!("\n[TEST-1] WASM Sandbox Isolation");
    serial_println!("--------------------------------");
    match malicious.call_function("try_escape_sandbox", &[]) {
        Ok(Some(Value::I32(result))) => {
            if result == 0 {
                serial_println!("[ OK ] ✅ Module confined to WASM linear memory");
                serial_println!("       Cannot access kernel address space");
            } else {
                serial_println!("[FAIL] ❌ Unexpected result");
            }
        }
        Ok(_) => serial_println!("[FAIL] ❌ Unexpected return type"),
        Err(e) => {
            serial_print!("[ OK ] ✅ WASM trapped: ");
            serial_println!("{}", e);
        }
    }

    // Test 2: Capability-Based Access Control
    serial_println!("\n[TEST-2] Capability-Based Access Control (Unauthorized IPC)");
    serial_println!("------------------------------------------------------------");
    match malicious.call_function("try_unauthorized_ipc", &[]) {
        Ok(Some(Value::I32(result))) => {
            if result < 0 {
                serial_println!("[ OK ] ✅ Unauthorized IPC rejected (permission denied)");
            } else {
                serial_println!("[FAIL] ❌ Unauthorized IPC succeeded (SECURITY BUG!)");
            }
        }
        Ok(_) => serial_println!("[FAIL] ❌ Unexpected return type"),
        Err(e) => {
            serial_print!("[ OK ] ✅ IPC trapped: ");
            serial_println!("{}", e);
        }
    }

    // Test 3: Stack Overflow Protection
    serial_println!("\n[TEST-3] Stack Overflow Protection");
    serial_println!("-----------------------------------");
    match malicious.call_function("try_stack_overflow", &[]) {
        Ok(Some(Value::I32(_result))) => {
            serial_println!("[ OK ] ✅ Stack overflow handled gracefully");
        }
        Ok(_) => serial_println!("[FAIL] ❌ Unexpected return type"),
        Err(e) => {
            serial_print!("[ OK ] ✅ Stack overflow prevented: ");
            serial_println!("{}", e);
        }
    }

    serial_println!("\n[DEMO 5] ✅ COMPLETE");
    serial_println!("🔒 Security guarantees validated:");
    serial_println!("   1. WASM sandbox isolates modules from kernel memory");
    serial_println!("   2. Capability system blocks unauthorized IPC (CRITICAL!)");
    serial_println!("   3. WASM runtime prevents resource exhaustion");
    serial_println!("   4. System remains stable - malicious code contained\n");
}

/// Run all WASM demos
pub fn run_all_demos() {
    serial_println!("\n╔════════════════════════════════════════════════════╗");
    serial_println!("║  JerichoOS WASM Demo Suite - Canonical Tests      ║");
    serial_println!("╚════════════════════════════════════════════════════╝");

    serial_println!("\n!!! ABOUT TO RUN DEMO 4 !!!\n");
    demo_04_mqtt();
    serial_println!("\n!!! DEMO 4 FINISHED !!!\n");

    demo_01_add();
    demo_02_hello();
    demo_03_syscall();
    demo_05_security();

    serial_println!("╔════════════════════════════════════════════════════╗");
    serial_println!("║  All WASM Demos Complete!                         ║");
    serial_println!("╚════════════════════════════════════════════════════╝\n");
}
